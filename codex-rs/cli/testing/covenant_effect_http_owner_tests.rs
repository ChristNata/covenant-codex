//! Unregistered draft. These future tests use owned loopback sockets, not a CLI.
//! Synthetic ExitStatus exercises fixture settlement input; it proves no child run.
use super::FixtureFailure;
use super::HttpPeer;
use super::HttpRequest;
use super::PeerProtocol;
use pretty_assertions::assert_eq;
use std::io;
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time::Instant;

fn protocol() -> PeerProtocol {
    PeerProtocol::Mcp {
        target: "/mcp/nonce".to_owned(),
    }
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(/*secs*/ 2)
}

fn socket_closed(result: io::Result<usize>) -> bool {
    match result {
        Ok(size) => size == 0,
        Err(error) => matches!(
            error.kind(),
            io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::BrokenPipe
                | io::ErrorKind::NotConnected
        ),
    }
}

#[tokio::test]
async fn owner_accepts_clean_absence_only_after_successful_settlement() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let report = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline())
        .await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (0, Vec::new(), Ok(()))
    );
}

#[tokio::test]
async fn owner_counts_queued_connection_before_parsing_at_child_settlement() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    // Establish the connection and its incomplete bytes before the owner task exists.
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    client
        .write_all(b"POST /mcp/nonce HTTP/1.1\r\nHost: local\r\nContent-Length: 8\r\n\r\n{")
        .await
        .unwrap();
    client.shutdown().await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let report = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline())
        .await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (1, Vec::new(), Err(FixtureFailure::Incomplete))
    );
}

#[tokio::test]
async fn owner_preserves_queued_complete_request_and_responds_before_join() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    client
        .write_all(b"GET /mcp/nonce HTTP/1.1\r\nHost: local\r\n\r\n")
        .await
        .unwrap();
    client.shutdown().await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let report = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline())
        .await;
    let mut reply = Vec::new();
    tokio::time::timeout_at(
        deadline(),
        client.take(/*limit*/ 1025).read_to_end(&mut reply),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        reply,
        b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (
            1,
            vec![HttpRequest {
                method: "GET".to_owned(),
                target: "/mcp/nonce".to_owned(),
                headers: vec![("host".to_owned(), "local".to_owned())],
                body: Vec::new(),
            }],
            Ok(())
        )
    );
}

#[tokio::test]
async fn owner_does_not_call_open_partial_connection_a_natural_drain() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    client.write_all(b"POST").await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let report = peer
        .finish_after_child(
            ExitStatus::from_raw(/*raw*/ 0),
            Instant::now() + Duration::from_millis(/*millis*/ 250),
        )
        .await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (1, Vec::new(), Err(FixtureFailure::Deadline))
    );
    // The retained client cannot create EOF itself: the peer must close its owned task/socket.
    let mut bytes = [0; 1];
    let read = tokio::time::timeout_at(deadline(), client.read(&mut bytes))
        .await
        .unwrap();
    assert!(socket_closed(read), "peer socket still active");
}

#[tokio::test]
async fn owner_refuses_connection_overflow_without_dropping_observed_count() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let mut clients = Vec::new();
    for _ in 0..5 {
        clients.push(
            TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap(),
        );
    }
    let peer = HttpPeer::start(listener, protocol());
    let report = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline())
        .await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (5, Vec::new(), Err(FixtureFailure::Limit))
    );
    for mut client in clients {
        let mut bytes = [0; 1];
        let read = tokio::time::timeout_at(deadline(), client.read(&mut bytes))
            .await
            .unwrap();
        assert!(
            socket_closed(read),
            "overflow left an owned connection alive"
        );
    }
}

#[tokio::test]
async fn owner_refuses_cancelled_or_failed_child_even_when_no_attempt_was_seen() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let failed = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 1), deadline())
        .await;
    assert_eq!(
        (failed.accepted, failed.requests, failed.terminal),
        (0, Vec::new(), Err(FixtureFailure::Child))
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let cancelled = peer.abort(deadline()).await;
    assert_eq!(
        (cancelled.accepted, cancelled.requests, cancelled.terminal),
        (0, Vec::new(), Err(FixtureFailure::Cancelled))
    );
}

#[tokio::test]
async fn owner_explicit_abort_closes_a_retained_live_client() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    client
        .write_all(b"GET /mcp/nonce HTTP/1.1\r\nHost: local\r\n\r\n")
        .await
        .unwrap();
    let peer = HttpPeer::start(listener, protocol());
    // A complete wire response proves an accepted task ran. Keep our write side open,
    // so the owner must cancel its retained reader rather than only drop a listener.
    let expected_reply =
        b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let mut reply = vec![0; expected_reply.len()];
    tokio::time::timeout_at(deadline(), client.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reply, expected_reply);
    let report = peer.abort(deadline()).await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (
            1,
            vec![HttpRequest {
                method: "GET".to_owned(),
                target: "/mcp/nonce".to_owned(),
                headers: vec![("host".to_owned(), "local".to_owned())],
                body: Vec::new(),
            }],
            Err(FixtureFailure::Cancelled)
        )
    );
    let mut bytes = [0; 1];
    let read = tokio::time::timeout_at(deadline(), client.read(&mut bytes))
        .await
        .unwrap();
    assert!(
        socket_closed(read),
        "cancelled peer retained a live connection"
    );
}

#[tokio::test]
async fn owner_observes_extra_bytes_sent_after_the_complete_response() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = HttpPeer::start(listener, protocol());
    let mut client = TcpStream::connect(address).await.unwrap();
    client
        .write_all(b"GET /mcp/nonce HTTP/1.1\r\nHost: local\r\n\r\n")
        .await
        .unwrap();
    // A successful peer half-closes response output while retaining its request reader.
    let mut reply = Vec::new();
    tokio::time::timeout_at(
        deadline(),
        (&mut client).take(/*limit*/ 1025).read_to_end(&mut reply),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        reply,
        b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    client.write_all(b"G").await.unwrap();
    client.shutdown().await.unwrap();
    let report = peer
        .finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline())
        .await;
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (
            1,
            vec![HttpRequest {
                method: "GET".to_owned(),
                target: "/mcp/nonce".to_owned(),
                headers: vec![("host".to_owned(), "local".to_owned())],
                body: Vec::new(),
            }],
            Err(FixtureFailure::Framing)
        )
    );
}

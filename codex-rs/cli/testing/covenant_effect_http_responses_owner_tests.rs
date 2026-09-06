//! Ignored test-first draft: real sockets, no actual CLI or child process.
//! The deferred Responses variant is unavailable; no compiled RED is claimed.
use super::FixtureFailure;
use super::HttpPeer;
use super::HttpRequest;
use super::PeerProtocol;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::io;
use std::net::SocketAddr;
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time::Instant;

fn inputs(address: SocketAddr) -> (PeerProtocol, HttpRequest, String) {
    let target = format!("/owned/{}/v1/responses", address.port());
    let api_key = format!("synthetic-socket-key-{}", address.port());
    let prompt = format!("Complete the owned socket exchange on {address}.");
    let marker = format!("OWNED_SOCKET_COMPLETE_{}", address.port());
    let body = serde_json::to_vec(&json!({
        "model":"gpt-5.5", "stream":true,
        "input":[{"type":"message","role":"user","content":[
            {"type":"input_text","text":prompt}
        ]}],
        "tools":[{"type":"function","name":"ordinary_socket_tool",
            "description":"observed-body-".repeat(/*n*/ 3000),
            "parameters":{"type":"object"}}]
    }))
    .unwrap();
    // Real transport must choose the Responses allowance, not the MCP 32768-byte limit.
    assert!(body.len() > 32768 && body.len() < 262144);
    let request = HttpRequest {
        method: "POST".to_owned(),
        target: target.clone(),
        headers: vec![
            ("host".to_owned(), address.to_string()),
            ("authorization".to_owned(), format!("Bearer {api_key}")),
            ("content-type".to_owned(), "application/json".to_owned()),
            ("content-length".to_owned(), body.len().to_string()),
        ],
        body,
    };
    let protocol = PeerProtocol::Responses {
        target,
        api_key,
        prompt,
        marker: marker.clone(),
    };
    (protocol, request, marker)
}

async fn round_trip(
    address: SocketAddr,
    request: &HttpRequest,
    deadline: Instant,
) -> io::Result<(TcpStream, Vec<u8>)> {
    tokio::time::timeout_at(deadline, async {
        let mut client = TcpStream::connect(address).await?;
        let mut wire = format!("{} {} HTTP/1.1\r\n", request.method, request.target);
        for (name, value) in &request.headers {
            wire.push_str(&format!("{name}: {value}\r\n"));
        }
        wire.push_str("\r\n");
        client.write_all(wire.as_bytes()).await?;
        client.write_all(&request.body).await?;
        let mut reply = Vec::new();
        // Reading response EOF leaves the retained client's write side open.
        (&mut client)
            .take(/*limit*/ 270337)
            .read_to_end(&mut reply)
            .await?;
        if reply.len() > 270336 {
            return Err(io::Error::other("owned response byte cap exceeded"));
        }
        Ok((client, reply))
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "owned exchange deadline"))?
}

fn assert_completion(reply: &[u8], marker: &str) {
    let split = reply
        .windows(/*size*/ 4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    assert!(split + 4 <= 8192);
    let (head, body) = reply.split_at(split + 4);
    assert!(body.len() <= 262144);
    let length = body.len();
    assert_eq!(head, format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\nContent-Type: text/event-stream\r\n\r\n"
    ).as_bytes());
    let events: Vec<Value> = std::str::from_utf8(body)
        .unwrap()
        .split("\n\n")
        .filter(|part| !part.is_empty())
        .map(|part| serde_json::from_str(part.strip_prefix("data: ").unwrap()).unwrap())
        .collect();
    assert_eq!(
        events,
        vec![
            json!({"type":"response.created","response":{"id":"resp_effects"}}),
            json!({"type":"response.output_item.done","item":{
                "type":"message","role":"assistant","id":"msg_effects",
                "content":[{"type":"output_text","text":marker}]
            }}),
            json!({"type":"response.completed","response":{"id":"resp_effects","usage":{
                "input_tokens":11,"output_tokens":7,"total_tokens":18
            }}}),
        ]
    );
}

#[tokio::test]
async fn responses_owner_completes_upgrade_fallback_and_naturally_joins() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (protocol, post, marker) = inputs(address);
    let mut upgrade = post.clone();
    upgrade.method = "GET".to_owned();
    upgrade.body.clear();
    for (name, value) in &mut upgrade.headers {
        if name == "content-length" {
            *value = "0".to_owned();
        }
    }
    upgrade.headers.extend([
        ("connection".to_owned(), "Upgrade".to_owned()),
        ("upgrade".to_owned(), "websocket".to_owned()),
    ]);
    let peer = HttpPeer::start(listener, protocol);
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 5);
    let traffic: io::Result<_> = async {
        let (mut first, upgrade_reply) = round_trip(address, &upgrade, deadline).await?;
        first.shutdown().await?;
        let (mut second, completion) = round_trip(address, &post, deadline).await?;
        second.shutdown().await?;
        Ok((upgrade_reply, completion))
    }
    .await;
    // Finish/abort is awaited even when connect/write/read/half-close failed.
    let report = if traffic.is_ok() {
        peer.finish_after_child(
            ExitStatus::from_raw(/*raw*/ 0),
            Instant::now() + Duration::from_secs(/*secs*/ 5),
        )
        .await
    } else {
        peer.abort(Instant::now() + Duration::from_secs(/*secs*/ 5))
            .await
    };
    let (upgrade_reply, completion) = traffic.unwrap();
    assert_eq!(
        upgrade_reply,
        b"HTTP/1.1 426 Upgrade Required\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    assert_completion(&completion, &marker);
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (2, vec![upgrade, post], Ok(()))
    );
}

#[tokio::test]
async fn responses_owner_keeps_finish_owned_until_late_bytes_are_rejected() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (protocol, post, marker) = inputs(address);
    let peer = HttpPeer::start(listener, protocol);
    let traffic = round_trip(
        address,
        &post,
        Instant::now() + Duration::from_secs(/*secs*/ 5),
    )
    .await;
    let (mut client, completion) = match traffic {
        Ok(value) => value,
        Err(_) => {
            peer.abort(Instant::now() + Duration::from_secs(/*secs*/ 5))
                .await;
            panic!("owned Responses wire control failed");
        }
    };
    // The complete response forces an accepted task; our write side has not sent EOF.
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 5);
    let finish = peer.finish_after_child(ExitStatus::from_raw(/*raw*/ 0), deadline);
    tokio::pin!(finish);
    let first_poll = tokio::time::timeout(Duration::from_millis(/*millis*/ 50), &mut finish).await;
    let (premature, report, late_write) = match first_poll {
        Ok(report) => (true, report, Ok(())),
        Err(_) => {
            // This is the same retained finish future after its borrowed wait expired.
            let written = tokio::time::timeout_at(deadline, async {
                client.write_all(b"G").await?;
                client.shutdown().await
            })
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "owned late-byte deadline"))
            .and_then(|value| value);
            let report = finish.await;
            (false, report, written)
        }
    };
    drop(client);
    assert_completion(&completion, &marker);
    assert!(
        !premature,
        "semantic reply was mistaken for settled request readers"
    );
    late_write.unwrap();
    assert_eq!(
        (report.accepted, report.requests, report.terminal),
        (1, vec![post], Err(FixtureFailure::Framing))
    );
    // Receipt of EOF was only response half-close. Source review must also prove
    // the supervisor's retained task joins before returning this failure report.
}

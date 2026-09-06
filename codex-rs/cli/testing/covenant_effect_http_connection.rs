use super::FixtureFailure;
use super::HttpLimits;
use super::HttpReply;
use super::HttpRequest;
use super::framing::HttpDecoder;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub(super) struct RequestWork {
    pub(super) request: HttpRequest,
    pub(super) reply: oneshot::Sender<HttpReply>,
}

pub(super) async fn serve_connection(
    mut stream: TcpStream,
    limits: HttpLimits,
    requests: mpsc::Sender<RequestWork>,
) -> Result<(), FixtureFailure> {
    let mut decoder = HttpDecoder::new(limits);
    let mut bytes = [0; 4096];
    loop {
        let received = stream
            .read(&mut bytes)
            .await
            .map_err(|_| FixtureFailure::Transport)?;
        if received == 0 {
            return decoder.finish_eof();
        }
        if let Some(request) = decoder.feed(&bytes[..received])? {
            let (reply, response) = oneshot::channel();
            requests
                .send(RequestWork { request, reply })
                .await
                .map_err(|_| FixtureFailure::Transport)?;
            let response = response.await.map_err(|_| FixtureFailure::Transport)?;
            if response.body.len() > limits.body {
                return Err(FixtureFailure::Limit);
            }
            let reason = match response.status {
                200 => "OK",
                202 => "Accepted",
                405 => "Method Not Allowed",
                426 => "Upgrade Required",
                _ => return Err(FixtureFailure::Protocol),
            };
            let status = response.status;
            let length = response.body.len();
            let mut header = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {length}\r\nConnection: close\r\n"
            );
            match response.content_type {
                Some(content_type @ ("application/json" | "text/event-stream")) => {
                    header.push_str(&format!("Content-Type: {content_type}\r\n"));
                }
                None => {}
                Some(_) => return Err(FixtureFailure::Protocol),
            }
            header.push_str("\r\n");
            if header.len() > limits.headers {
                return Err(FixtureFailure::Limit);
            }
            stream
                .write_all(header.as_bytes())
                .await
                .map_err(|_| FixtureFailure::Transport)?;
            stream
                .write_all(&response.body)
                .await
                .map_err(|_| FixtureFailure::Transport)?;
            stream
                .shutdown()
                .await
                .map_err(|_| FixtureFailure::Transport)?;
            // Response EOF is only a half-close. Keep this same decoder/reader to real EOF.
        }
    }
}

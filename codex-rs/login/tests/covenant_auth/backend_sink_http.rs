//! Bounded local HTTP authority for Covenant auth tests.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);
const HTTP_LIMIT: usize = 32 * 1024;

pub(super) struct Step {
    method: &'static str,
    path: &'static str,
    status: u16,
    body: Vec<u8>,
    required: Option<String>,
}

impl Step {
    pub(super) fn json(
        method: &'static str,
        path: &'static str,
        status: u16,
        body: serde_json::Value,
        required: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            method,
            path,
            status,
            body: serde_json::to_vec(&body)?,
            required,
        })
    }
}

pub(super) struct HttpFixture {
    result: mpsc::Receiver<Result<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl HttpFixture {
    pub(super) fn start(
        steps: Vec<Step>,
        configure: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        configure(&format!("http://{address}"))?;
        let (sender, result) = mpsc::channel();
        let thread = thread::spawn(move || {
            let outcome = serve_steps(listener, steps);
            let _ = sender.send(outcome);
        });
        Ok(Self {
            result,
            thread: Some(thread),
        })
    }

    pub(super) fn finish(mut self) -> Result<()> {
        self.result.recv_timeout(DEADLINE)??;
        let thread = self.thread.take().context("HTTP fixture thread missing")?;
        thread
            .join()
            .map_err(|_| anyhow::anyhow!("HTTP fixture panicked"))?;
        Ok(())
    }
}

fn serve_steps(listener: TcpListener, steps: Vec<Step>) -> Result<()> {
    for step in steps {
        let deadline = Instant::now() + DEADLINE;
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    ensure!(Instant::now() < deadline, "HTTP fixture timed out");
                    thread::sleep(Duration::from_millis(/*millis*/ 5));
                }
                Err(error) => return Err(error.into()),
            }
        };
        stream.set_nonblocking(false)?;
        let request = read_http_request(&mut stream)?;
        let request_line = request.lines().next().context("request line missing")?;
        ensure!(
            request_line.starts_with(&format!("{} {} ", step.method, step.path)),
            "unexpected fixture route"
        );
        if let Some(required) = step.required {
            ensure!(
                request.contains(&required),
                "fixture request omitted sentinel"
            );
        }
        let reason = if step.status == 200 {
            "OK"
        } else {
            "Internal Server Error"
        };
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            step.status,
            reason,
            step.body.len()
        )?;
        stream.write_all(&step.body)?;
        stream.flush()?;
    }
    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> Result<String> {
    stream.set_read_timeout(Some(DEADLINE))?;
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        ensure!(bytes.len() < HTTP_LIMIT, "oversized fixture header");
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    let header = String::from_utf8(bytes)?;
    let length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    ensure!(length <= HTTP_LIMIT, "oversized fixture body");
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    Ok(format!("{header}{}", String::from_utf8(body)?))
}

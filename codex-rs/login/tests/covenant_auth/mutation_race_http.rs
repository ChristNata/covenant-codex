//! Bounded loopback authority for mutation-race tests.

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub(super) enum EndpointPlan {
    RefreshSuccess { current: String, next: AuthDotJson },
}

#[derive(Default)]
struct EndpointState {
    released: bool,
    stop: bool,
}

pub(super) struct Endpoint {
    address: SocketAddr,
    state: Arc<(Mutex<EndpointState>, Condvar)>,
    accepted: mpsc::Receiver<u32>,
    server: Option<thread::JoinHandle<()>>,
}

impl Endpoint {
    pub(super) fn start(plan: EndpointPlan, hold_first: bool) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let state = Arc::new((Mutex::new(EndpointState::default()), Condvar::new()));
        let (sender, accepted) = mpsc::channel();
        let shared = Arc::clone(&state);
        let server = thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            if !lock_state(&shared).stop {
                let _ = serve(
                    stream, &plan, /*ordinal*/ 1, hold_first, &shared, &sender,
                );
            }
        });
        Ok(Self {
            address,
            state,
            accepted,
            server: Some(server),
        })
    }

    pub(super) fn refresh_url(&self) -> String {
        format!("http://{}/oauth/token", self.address)
    }

    pub(super) fn wait_accepted(&self, ordinal: u32) -> Result<()> {
        ensure!(
            self.accepted
                .recv_timeout(super::mutation_race_fixture::DEADLINE)?
                == ordinal,
            "endpoint accepted unexpected request"
        );
        Ok(())
    }

    pub(super) fn release(&self) {
        lock_state(&self.state).released = true;
        self.state.1.notify_all();
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        lock_state(&self.state).stop = true;
        self.state.1.notify_all();
        let _ = TcpStream::connect(self.address);
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn read_request(mut stream: &TcpStream) -> Result<(String, serde_json::Value)> {
    stream.set_read_timeout(Some(Duration::from_secs(/*secs*/ 10)))?;
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        ensure!(header.len() < 16 * 1024, "oversized fixture header");
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let header = String::from_utf8(header)?;
    let route = header
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("POST "))
        .and_then(|line| line.strip_suffix(" HTTP/1.1"))
        .context("unexpected fixture request line")?
        .to_string();
    let lengths: Vec<_> = header
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then_some(value.trim())
        })
        .collect();
    ensure!(lengths.len() == 1, "missing fixture content length");
    let length: usize = lengths[0].parse()?;
    ensure!(length <= 16 * 1024, "oversized fixture body");
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    Ok((route, serde_json::from_slice(&body)?))
}

fn serve(
    mut stream: TcpStream,
    plan: &EndpointPlan,
    ordinal: u32,
    hold_first: bool,
    shared: &(Mutex<EndpointState>, Condvar),
    sender: &mpsc::Sender<u32>,
) -> Result<()> {
    let (route, body) = read_request(&stream)?;
    let response = match plan {
        EndpointPlan::RefreshSuccess { current, next } => {
            ensure!(
                ordinal == 1 && route == "/oauth/token",
                "unexpected refresh route"
            );
            ensure!(
                body.get("refresh_token")
                    .and_then(serde_json::Value::as_str)
                    == Some(current.as_str())
                    && body.get("grant_type").and_then(serde_json::Value::as_str)
                        == Some("refresh_token"),
                "unexpected refresh request"
            );
            let tokens = next.tokens.as_ref().context("missing fixture tokens")?;
            serde_json::json!({"id_token": tokens.id_token.raw_jwt,
                "access_token": tokens.access_token, "refresh_token": tokens.refresh_token})
        }
    };
    {
        let mut state = lock_state(shared);
        sender.send(ordinal)?;
        while hold_first && ordinal == 1 && !state.released && !state.stop {
            let (next, timeout) = shared
                .1
                .wait_timeout(state, super::mutation_race_fixture::DEADLINE)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next;
            ensure!(!timeout.timed_out(), "fixture endpoint release timed out");
        }
        ensure!(!state.stop, "fixture endpoint stopped");
    }
    let bytes = serde_json::to_vec(&response)?;
    stream.set_write_timeout(Some(Duration::from_secs(/*secs*/ 10)))?;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

fn lock_state(shared: &(Mutex<EndpointState>, Condvar)) -> MutexGuard<'_, EndpointState> {
    shared
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

use super::rotation_support::document;
use anyhow::Result;
use anyhow::ensure;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Snapshot {
    pub requests: u32,
    pub reuses: u32,
    pub generation: u32,
    pub acknowledged: u32,
    pub malformed: bool,
}

#[derive(Default)]
struct State {
    snapshot: Snapshot,
    held: u32,
    released: u32,
    stop: bool,
}

pub(super) struct Authority {
    address: SocketAddr,
    shared: Arc<(Mutex<State>, Condvar)>,
    accepted: mpsc::Receiver<u32>,
    server: Option<thread::JoinHandle<()>>,
}

impl Authority {
    pub fn start(nonce: String) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let (sender, accepted) = mpsc::channel();
        let state = shared.clone();
        let server = thread::spawn(move || {
            let mut workers = Vec::new();
            for stream in listener.incoming() {
                if state.0.lock().unwrap().stop {
                    break;
                }
                let Ok(stream) = stream else {
                    state.0.lock().unwrap().snapshot.malformed = true;
                    break;
                };
                let state = state.clone();
                let sender = sender.clone();
                let nonce = nonce.clone();
                workers.push(thread::spawn(move || {
                    if serve(stream, &nonce, &state, &sender).is_err() {
                        state.0.lock().unwrap().snapshot.malformed = true;
                    }
                }));
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Ok(Self {
            address,
            shared,
            accepted,
            server: Some(server),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}/oauth/token", self.address)
    }

    pub fn arm(&self, generation: u32) {
        self.shared.0.lock().unwrap().held = generation;
    }

    pub fn wait_accepted(&self, generation: u32) -> Result<()> {
        ensure!(
            self.accepted
                .recv_timeout(Duration::from_secs(/*secs*/ 30))?
                == generation,
            "authority accepted an unexpected generation"
        );
        Ok(())
    }

    pub fn release(&self, generation: u32) {
        self.shared.0.lock().unwrap().released = generation;
        self.shared.1.notify_all();
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared.0.lock().unwrap().snapshot
    }

    pub fn wait_acknowledged(&self, generation: u32) -> Result<()> {
        let state = self.shared.0.lock().unwrap();
        let (state, _) = self
            .shared
            .1
            .wait_timeout_while(state, Duration::from_secs(/*secs*/ 30), |state| {
                state.snapshot.acknowledged < generation && !state.snapshot.malformed
            })
            .unwrap();
        ensure!(
            state.snapshot.acknowledged >= generation && !state.snapshot.malformed,
            "authority did not finish acknowledging its response"
        );
        Ok(())
    }
}

impl Drop for Authority {
    fn drop(&mut self) {
        self.shared.0.lock().unwrap().stop = true;
        self.shared.1.notify_all();
        let _ = TcpStream::connect(self.address);
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    nonce: &str,
    shared: &(Mutex<State>, Condvar),
    sender: &mpsc::Sender<u32>,
) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(/*secs*/ 10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(/*secs*/ 10)))?;
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        ensure!(header.len() < 16 * 1024, "oversized fixture request header");
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let header = String::from_utf8(header)?;
    ensure!(
        header.starts_with("POST /oauth/token HTTP/1.1\r\n"),
        "unexpected fixture HTTP route"
    );
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
    ensure!(length <= 16 * 1024, "oversized fixture request body");
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    let request: serde_json::Value = serde_json::from_slice(&body)?;
    let mut state = shared.0.lock().unwrap();
    state.snapshot.requests += 1;
    let current = document(nonce, state.snapshot.generation)?.tokens.unwrap();
    let valid = request
        .get("refresh_token")
        .and_then(serde_json::Value::as_str)
        == Some(current.refresh_token.as_str());
    ensure!(
        request
            .get("grant_type")
            .and_then(serde_json::Value::as_str)
            == Some("refresh_token")
            && request.get("client_id").and_then(serde_json::Value::as_str)
                == Some(codex_login::CLIENT_ID),
        "unexpected fixture OAuth request shape"
    );
    let (status, response, generation) = if valid {
        state.snapshot.generation += 1;
        let generation = state.snapshot.generation;
        sender.send(generation)?;
        while state.held == generation && state.released < generation && !state.stop {
            let (next, timeout) = shared
                .1
                .wait_timeout(state, Duration::from_secs(/*secs*/ 30))
                .unwrap();
            state = next;
            ensure!(!timeout.timed_out(), "authority response barrier timed out");
        }
        ensure!(!state.stop, "fixture authority stopped");
        let tokens = document(nonce, generation)?.tokens.unwrap();
        (
            "200 OK",
            serde_json::json!({"id_token": tokens.id_token.raw_jwt,
            "access_token": tokens.access_token, "refresh_token": tokens.refresh_token}),
            Some(generation),
        )
    } else {
        state.snapshot.reuses += 1;
        (
            "401 Unauthorized",
            serde_json::json!({"error": {"code": "refresh_token_reused"}}),
            None,
        )
    };
    drop(state);
    let body = serde_json::to_vec(&response)?;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()?;
    if let Some(generation) = generation {
        let mut state = shared.0.lock().unwrap();
        state.snapshot.acknowledged = state.snapshot.acknowledged.max(generation);
        shared.1.notify_all();
    }
    Ok(())
}

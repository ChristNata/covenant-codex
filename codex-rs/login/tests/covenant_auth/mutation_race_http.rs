//! Bounded loopback authority for mutation-race tests.

use super::mutation_race_fixture::FailureKind;
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
use std::time::Instant;

#[derive(Clone)]
pub(super) enum EndpointPlan {
    RefreshSuccess {
        current: String,
        next: Box<AuthDotJson>,
    },
    RefreshFailure {
        current: String,
        kind: FailureKind,
    },
    Revoke {
        token: String,
        succeeds: bool,
    },
    AgentIdentity,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct EndpointSnapshot {
    pub(super) requests: u32,
    pub(super) acknowledged: u32,
    pub(super) malformed: bool,
}

#[derive(Default)]
struct EndpointState {
    snapshot: EndpointSnapshot,
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
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let state = Arc::new((Mutex::new(EndpointState::default()), Condvar::new()));
        let (sender, accepted) = mpsc::channel();
        let shared = Arc::clone(&state);
        let server = thread::spawn(move || {
            let count = match &plan {
                EndpointPlan::RefreshFailure { .. } | EndpointPlan::AgentIdentity => 2,
                EndpointPlan::RefreshSuccess { .. } | EndpointPlan::Revoke { .. } => 1,
            };
            let mut ordinal = 0;
            loop {
                let stopped_at_start = lock_state(&shared).stop;
                match listener.accept() {
                    Ok((stream, _)) => {
                        ordinal += 1;
                        {
                            let mut state = lock_state(&shared);
                            state.snapshot.requests += 1;
                            shared.1.notify_all();
                        }
                        if stream.set_nonblocking(false).is_err() || ordinal > count {
                            mark_malformed(&shared);
                            continue;
                        }
                        if serve(stream, &plan, ordinal, hold_first, &shared, &sender).is_err() {
                            mark_malformed(&shared);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped_at_start {
                            break;
                        }
                        thread::sleep(Duration::from_millis(/*millis*/ 5));
                    }
                    Err(_) => {
                        mark_malformed(&shared);
                        break;
                    }
                }
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

    pub(super) fn revoke_url(&self) -> String {
        format!("http://{}/oauth/revoke", self.address)
    }

    pub(super) fn base_url(&self) -> String {
        format!("http://{}", self.address)
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

    pub(super) fn wait_complete(&self, ordinal: u32) -> Result<EndpointSnapshot> {
        let deadline = Instant::now() + super::mutation_race_fixture::DEADLINE;
        let mut state = lock_state(&self.state);
        loop {
            if state.snapshot.acknowledged >= ordinal || state.snapshot.malformed {
                return Ok(state.snapshot);
            }
            let now = Instant::now();
            ensure!(now < deadline, "endpoint completion timed out");
            let (next, _) = self
                .state
                .1
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next;
        }
    }

    pub(super) fn finish(mut self) -> Result<EndpointSnapshot> {
        self.stop_and_join()
    }

    fn stop_and_join(&mut self) -> Result<EndpointSnapshot> {
        {
            let mut state = lock_state(&self.state);
            state.stop = true;
            self.state.1.notify_all();
        }
        let deadline = Instant::now() + super::mutation_race_fixture::DEADLINE;
        while self
            .server
            .as_ref()
            .is_some_and(|server| !server.is_finished())
        {
            ensure!(
                Instant::now() < deadline,
                "fixture endpoint shutdown timed out"
            );
            thread::sleep(Duration::from_millis(/*millis*/ 5));
        }
        if let Some(server) = self.server.take() {
            server
                .join()
                .map_err(|_| anyhow::anyhow!("fixture endpoint thread panicked"))?;
        }
        Ok(lock_state(&self.state).snapshot)
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
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
    let (status, response) = match plan {
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
            (
                "200 OK",
                serde_json::json!({"id_token": tokens.id_token.raw_jwt,
                    "access_token": tokens.access_token, "refresh_token": tokens.refresh_token}),
            )
        }
        EndpointPlan::RefreshFailure { current, kind } => {
            ensure!(route == "/oauth/token", "unexpected refresh route");
            ensure!(
                body.get("refresh_token")
                    .and_then(serde_json::Value::as_str)
                    == Some(current.as_str())
                    && body.get("grant_type").and_then(serde_json::Value::as_str)
                        == Some("refresh_token"),
                "unexpected refresh request"
            );
            match kind {
                FailureKind::Transient => (
                    "503 Service Unavailable",
                    serde_json::json!({"error": {"code": "temporarily_unavailable"}}),
                ),
                FailureKind::Permanent => (
                    "401 Unauthorized",
                    serde_json::json!({"error": {"code": "refresh_token_reused"}}),
                ),
            }
        }
        EndpointPlan::Revoke { token, succeeds } => {
            ensure!(
                ordinal == 1 && route == "/oauth/revoke",
                "unexpected revoke route"
            );
            ensure!(
                body.get("token").and_then(serde_json::Value::as_str) == Some(token.as_str())
                    && body
                        .get("token_type_hint")
                        .and_then(serde_json::Value::as_str)
                        == Some("refresh_token"),
                "unexpected revoke snapshot"
            );
            if *succeeds {
                ("200 OK", serde_json::json!({}))
            } else {
                (
                    "503 Service Unavailable",
                    serde_json::json!({"error": "temporarily_unavailable"}),
                )
            }
        }
        EndpointPlan::AgentIdentity => {
            if ordinal == 1 {
                ensure!(route == "/v1/agent/register", "unexpected agent route");
                (
                    "200 OK",
                    serde_json::json!({"agent_runtime_id": "covenant-agent-runtime"}),
                )
            } else {
                ensure!(
                    route == "/v1/agent/covenant-agent-runtime/task/register",
                    "unexpected agent task route"
                );
                ("200 OK", serde_json::json!({"task_id": "covenant-task"}))
            }
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
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    lock_state(shared).snapshot.acknowledged += 1;
    shared.1.notify_all();
    Ok(())
}

fn mark_malformed(shared: &(Mutex<EndpointState>, Condvar)) {
    lock_state(shared).snapshot.malformed = true;
    shared.1.notify_all();
}

fn lock_state(shared: &(Mutex<EndpointState>, Condvar)) -> MutexGuard<'_, EndpointState> {
    shared
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

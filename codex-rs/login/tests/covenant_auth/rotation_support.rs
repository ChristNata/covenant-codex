use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR;
use codex_login::load_auth_dot_json;
use codex_login::test_support::transport_default_auth_route_config;
use serde::Deserialize;
use serde::Serialize;
use std::future::Future;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const CHILD: &str = "COVENANT_AUTH_ROTATION_CHILD";
const PREFIX: &str = "COVENANT_ROTATION ";
const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(super) enum Method {
    Guarded,
    Authority,
    Probe,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Fixture {
    pub root: PathBuf,
    pub initial: AuthDotJson,
    pub expected: AuthDotJson,
    pub method: Method,
    pub started_at: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Report {
    refresh_succeeded: bool,
    cache_latest: bool,
    whole_document_latest: bool,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.refresh_succeeded && self.cache_latest && self.whole_document_latest
    }
}

#[derive(Deserialize, Serialize)]
enum Event {
    Ready,
    Entered,
    Done(Report),
    Closed,
}

enum Barrier {
    Ready,
    Entered,
}

pub(super) fn unix_seconds() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

pub(super) fn document(nonce: &str, generation: u32) -> Result<AuthDotJson> {
    // Valid synthetic JWT payloads: {} and {"exp":4102444800}; no signing authority.
    Ok(serde_json::from_value(serde_json::json!({
        "auth_mode": "chatgpt", "OPENAI_API_KEY": format!("{nonce}-preserved"),
        "tokens": {"id_token": format!("e30.e30.{nonce}-id-{generation}"),
            "access_token": format!("e30.eyJleHAiOjQxMDI0NDQ4MDB9.{nonce}-access-{generation}"),
            "refresh_token": format!("{nonce}-refresh-{generation}"),
            "account_id": "covenant-synthetic-account"},
        "last_refresh": "2099-01-01T00:00:00Z"
    }))?)
}

pub(super) fn whole_document_matches(
    observed: &AuthDotJson,
    expected: &AuthDotJson,
    started: u64,
) -> bool {
    let Some(refreshed) = observed.last_refresh else {
        return false;
    };
    let Ok(finished) = unix_seconds() else {
        return false;
    };
    let Ok(timestamp) = u64::try_from(refreshed.timestamp()) else {
        return false;
    };
    if timestamp < started || timestamp > finished {
        return false;
    }
    let mut expected = expected.clone();
    expected.last_refresh = Some(refreshed);
    observed == &expected
}

pub(super) fn is_child(test_name: &str) -> bool {
    std::env::var(CHILD).ok().as_deref() == Some(test_name)
}

fn emit(event: Event) -> Result<()> {
    println!("{PREFIX}{}", serde_json::to_string(&event)?);
    std::io::stdout().flush()?;
    Ok(())
}

pub(super) fn run_child() -> Result<()> {
    let mut input = BufReader::new(std::io::stdin());
    let mut line = String::new();
    input.read_line(&mut line)?;
    let fixture: Fixture = serde_json::from_str(&line)
        .map_err(|_| anyhow::anyhow!("invalid synthetic fixture input"))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let home = fixture.root.join("mutable");
            let manager = AuthManager::shared(
                home.clone(),
                /*enable_codex_api_key_env*/ false,
                AuthCredentialsStoreMode::File,
                /*forced_chatgpt_workspace_id*/ None,
                /*chatgpt_base_url*/ None,
                AuthKeyringBackendKind::Direct,
                transport_default_auth_route_config(),
            )
            .await;
            let initial = manager
                .auth_cached()
                .context("public manager did not initialize")?;
            ensure!(
                initial.is_chatgpt_auth()
                    && initial.get_token_data().ok().as_ref() == fixture.initial.tokens.as_ref(),
                "public manager did not cache the starting native generation"
            );
            emit(Event::Ready)?;
            line.clear();
            input.read_line(&mut line)?;
            ensure!(line == "go\n", "missing fixture release barrier");
            let refresh_succeeded = match fixture.method {
                Method::Probe => true,
                Method::Guarded | Method::Authority => {
                    let mut future = Box::pin(async {
                        match fixture.method {
                            Method::Guarded => manager.refresh_token().await,
                            Method::Authority => manager.refresh_token_from_authority().await,
                            Method::Probe => unreachable!(),
                        }
                    });
                    let mut entered = false;
                    std::future::poll_fn(|context| {
                        let result = future.as_mut().poll(context);
                        if !entered {
                            entered = true;
                            // Fixed protocol output only; never format a token or refresh error.
                            emit(Event::Entered).expect("fixture first-poll report failed");
                        }
                        result
                    })
                    .await
                    .is_ok()
                }
            };
            let auth = manager.auth().await;
            let cache_latest = auth.as_ref().is_some_and(|auth| {
                auth.is_chatgpt_auth()
                    && auth.get_token_data().ok().as_ref() == fixture.expected.tokens.as_ref()
            });
            let loaded = load_auth_dot_json(
                &home,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            )
            .ok()
            .flatten();
            let whole_document_latest =
                loaded
                    .as_ref()
                    .is_some_and(|observed| match fixture.method {
                        Method::Probe => observed == &fixture.expected,
                        Method::Guarded | Method::Authority => {
                            whole_document_matches(observed, &fixture.expected, fixture.started_at)
                        }
                    });
            emit(Event::Done(Report {
                refresh_succeeded,
                cache_latest,
                whole_document_latest,
            }))
        })
}

pub(super) struct Processes {
    children: Vec<Child>,
    events: mpsc::Receiver<(usize, Event)>,
    pending: Vec<(usize, Report)>,
}

impl Processes {
    pub fn spawn(
        test_name: &str,
        fixture: &Fixture,
        methods: &[Method],
        endpoint: &str,
    ) -> Result<Self> {
        let (sender, events) = mpsc::channel();
        let mut processes = Self {
            children: Vec::new(),
            events,
            pending: Vec::new(),
        };
        for (index, method) in methods.iter().enumerate() {
            let mut command = Command::new(std::env::current_exe()?);
            command
                .args(["--exact", test_name, "--nocapture"])
                .current_dir(&fixture.root)
                .env_clear()
                .env(CHILD, test_name)
                .env("CODEX_HOME", fixture.root.join("mutable"))
                .env("CODEX_AUTH_HOME", fixture.root.join("auth"))
                .env(REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR, endpoint)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            for name in [
                "TEMP",
                "TMP",
                "USERPROFILE",
                "HOME",
                "LOCALAPPDATA",
                "APPDATA",
            ] {
                command.env(name, &fixture.root);
            }
            for name in ["SystemRoot", "WINDIR"] {
                if let Some(value) = std::env::var_os(name) {
                    command.env(name, value);
                }
            }
            let mut child = command.spawn()?;
            let stdout = child.stdout.take().context("child stdout unavailable")?;
            let sender = sender.clone();
            thread::spawn(move || {
                for line in BufReader::new(stdout.take(/*limit*/ 131_072)).lines() {
                    let Ok(line) = line else {
                        break;
                    };
                    if let Some((_, payload)) = line.split_once(PREFIX)
                        && let Ok(event) = serde_json::from_str(payload)
                        && sender.send((index, event)).is_err()
                    {
                        return;
                    }
                }
                let _ = sender.send((index, Event::Closed));
            });
            processes.children.push(child);
            let child = processes
                .children
                .last_mut()
                .context("child ownership unavailable")?;
            let fixture = Fixture {
                method: *method,
                ..fixture.clone()
            };
            let encoded = serde_json::to_vec(&fixture)?;
            let input = child.stdin.as_mut().context("child stdin unavailable")?;
            input.write_all(&encoded)?;
            input.write_all(b"\n")?;
            input.flush()?;
        }
        Ok(processes)
    }

    fn barrier(&mut self, barrier: Barrier) -> Result<()> {
        let mut seen = vec![false; self.children.len()];
        while seen.iter().any(|ready| !ready) {
            let (index, event) = self
                .events
                .recv_timeout(DEADLINE)
                .context("child barrier timed out")?;
            match (event, &barrier) {
                (Event::Ready, Barrier::Ready) | (Event::Entered, Barrier::Entered) => {
                    ensure!(!seen[index], "duplicate child barrier");
                    seen[index] = true;
                }
                (Event::Done(report), Barrier::Entered) if seen[index] => {
                    ensure!(
                        !self.pending.iter().any(|(prior, _)| *prior == index),
                        "duplicate early result"
                    );
                    self.pending.push((index, report));
                }
                (Event::Closed, Barrier::Entered)
                    if self.pending.iter().any(|(prior, _)| *prior == index) => {}
                (
                    Event::Ready | Event::Entered | Event::Done(_) | Event::Closed,
                    Barrier::Ready | Barrier::Entered,
                ) => {
                    anyhow::bail!("child failed before its required barrier");
                }
            }
        }
        Ok(())
    }

    pub fn ready_and_release(&mut self) -> Result<()> {
        self.barrier(Barrier::Ready)?;
        for child in &mut self.children {
            child
                .stdin
                .as_mut()
                .context("child release pipe unavailable")?
                .write_all(b"go\n")?;
        }
        Ok(())
    }

    pub fn wait_entered(&mut self) -> Result<()> {
        self.barrier(Barrier::Entered)
    }

    pub fn finish(&mut self) -> Result<Vec<Report>> {
        let mut reports = Vec::new();
        let mut seen = vec![false; self.children.len()];
        for (index, report) in std::mem::take(&mut self.pending) {
            seen[index] = true;
            reports.push(report);
        }
        while reports.len() < self.children.len() {
            let (index, event) = self
                .events
                .recv_timeout(DEADLINE)
                .context("child result timed out")?;
            match event {
                Event::Done(report) => {
                    ensure!(!seen[index], "duplicate child result");
                    seen[index] = true;
                    reports.push(report);
                }
                Event::Closed if seen[index] => {}
                Event::Closed | Event::Ready | Event::Entered => {
                    anyhow::bail!("child failed before reporting its result")
                }
            }
        }
        let deadline = Instant::now() + DEADLINE;
        for child in &mut self.children {
            loop {
                if let Some(status) = child.try_wait()? {
                    ensure!(status.success(), "child process failed");
                    break;
                }
                ensure!(Instant::now() < deadline, "child process did not settle");
                thread::sleep(Duration::from_millis(/*millis*/ 5));
            }
        }
        Ok(reports)
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        // Own only these synthetic child processes, never a Cargo/Rust build process.
        for child in &mut self.children {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

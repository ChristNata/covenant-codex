use super::rotation_support;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::load_auth_dot_json;
use codex_login::login_with_api_key;
use codex_login::save_auth;
use codex_login::test_support::transport_default_auth_route_config;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const PREFIX: &str = "COVENANT_PERSISTENCE ";
const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(super) enum Case {
    Replace,
    ReadOnly,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(super) enum Operation {
    Refresh,
    Probe,
    Recovery,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Fixture {
    pub root: PathBuf,
    pub initial: AuthDotJson,
    pub expected: AuthDotJson,
    pub case: Case,
    pub operation: Operation,
    pub started_at: u64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(super) struct Report {
    pub operation_succeeded: bool,
    pub cache_expected: bool,
    pub document_expected: bool,
    pub fault_probe: Option<bool>,
}

#[derive(Deserialize, Serialize)]
enum Event {
    Ready,
    Done(Report),
    Closed,
}

fn emit(event: Event) -> Result<()> {
    println!("{PREFIX}{}", serde_json::to_string(&event)?);
    std::io::stdout().flush()?;
    Ok(())
}

async fn manager(home: &Path) -> Arc<AuthManager> {
    AuthManager::shared(
        home.to_path_buf(),
        /*enable_codex_api_key_env*/ false,
        AuthCredentialsStoreMode::File,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        AuthKeyringBackendKind::Direct,
        transport_default_auth_route_config(),
    )
    .await
}

fn cached_matches(manager: &AuthManager, expected: &AuthDotJson) -> bool {
    manager.auth_cached().is_some_and(|auth| {
        if expected.tokens.is_some() {
            auth.is_chatgpt_auth()
                && auth.get_token_data().ok().as_ref() == expected.tokens.as_ref()
        } else {
            auth.api_key() == expected.openai_api_key.as_deref()
        }
    })
}

pub(super) fn run_child() -> Result<()> {
    let mut input = BufReader::new(std::io::stdin());
    let mut line = String::new();
    input.read_line(&mut line)?;
    let fixture: Fixture = serde_json::from_str(&line)
        .map_err(|_| anyhow::anyhow!("invalid persistence fixture input"))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let home = fixture.root.join("mutable");
            let mut auth_manager = manager(&home).await;
            ensure!(
                cached_matches(&auth_manager, &fixture.initial),
                "initial public cache mismatch"
            );
            let fault_probe = if matches!(
                (fixture.case, fixture.operation),
                (Case::ReadOnly, Operation::Refresh)
            ) {
                let path = fixture.root.join("auth/auth.json");
                let before = fs::read(&path)?;
                let mut alternate = fixture.initial.clone();
                alternate.openai_api_key =
                    Some("distinct synthetic preflight document".to_string());
                let refused = save_auth(
                    &home,
                    &alternate,
                    AuthCredentialsStoreMode::File,
                    AuthKeyringBackendKind::Direct,
                )
                .err()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
                let intact = fs::read(&path)? == before;
                ensure!(
                    refused && intact,
                    "readonly public save fault was not established"
                );
                Some(true)
            } else {
                None
            };
            emit(Event::Ready)?;
            line.clear();
            input.read_line(&mut line)?;
            ensure!(line == "go\n", "missing persistence fixture release");
            let operation_succeeded = match fixture.operation {
                Operation::Refresh => match fixture.case {
                    Case::Replace => auth_manager.refresh_token().await.is_ok(),
                    Case::ReadOnly => auth_manager.refresh_token_from_authority().await.is_ok(),
                },
                Operation::Probe => true,
                Operation::Recovery => {
                    let key = fixture
                        .expected
                        .openai_api_key
                        .as_deref()
                        .context("missing synthetic recovery key")?;
                    let saved = login_with_api_key(
                        &home,
                        key,
                        AuthCredentialsStoreMode::File,
                        AuthKeyringBackendKind::Direct,
                    )
                    .is_ok();
                    auth_manager = manager(&home).await;
                    saved
                }
            };
            let observed = load_auth_dot_json(
                &home,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            )
            .ok()
            .flatten();
            let document_expected =
                observed
                    .as_ref()
                    .is_some_and(|observed| match (fixture.case, fixture.operation) {
                        (Case::Replace, Operation::Refresh) => {
                            rotation_support::whole_document_matches(
                                observed,
                                &fixture.expected,
                                fixture.started_at,
                            )
                        }
                        (Case::ReadOnly, Operation::Refresh)
                        | (_, Operation::Probe | Operation::Recovery) => {
                            observed == &fixture.expected
                        }
                    });
            emit(Event::Done(Report {
                operation_succeeded,
                cache_expected: cached_matches(&auth_manager, &fixture.expected),
                document_expected,
                fault_probe,
            }))
        })
}

pub(super) struct Process {
    child: Child,
    events: mpsc::Receiver<Event>,
}

impl Process {
    pub fn spawn(test_name: &str, fixture: &Fixture, endpoint: &str) -> Result<Self> {
        let mut child =
            rotation_support::child_command(test_name, &fixture.root, endpoint)?.spawn()?;
        let stdout = child
            .stdout
            .take()
            .context("persistence child stdout unavailable")?;
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout.take(/*limit*/ 131_072)).lines() {
                let Ok(line) = line else { break };
                if let Some((_, payload)) = line.split_once(PREFIX)
                    && let Ok(event) = serde_json::from_str(payload)
                    && sender.send(event).is_err()
                {
                    return;
                }
            }
            let _ = sender.send(Event::Closed);
        });
        let mut process = Self { child, events };
        let input = process
            .child
            .stdin
            .as_mut()
            .context("persistence child stdin unavailable")?;
        input.write_all(&serde_json::to_vec(fixture)?)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(process)
    }

    pub fn ready_and_release(&mut self) -> Result<()> {
        ensure!(
            matches!(self.events.recv_timeout(DEADLINE)?, Event::Ready),
            "persistence child failed before Ready"
        );
        self.child
            .stdin
            .as_mut()
            .context("persistence release pipe unavailable")?
            .write_all(b"go\n")?;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<Report> {
        let Event::Done(report) = self.events.recv_timeout(DEADLINE)? else {
            anyhow::bail!("persistence child failed before reporting");
        };
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.child.try_wait()? {
                ensure!(status.success(), "persistence child failed");
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "persistence child did not settle"
            );
            thread::sleep(Duration::from_millis(/*millis*/ 5));
        }
        Ok(report)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // This owns only one synthetic fixture child, never a Cargo/Rust process.
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

pub(super) struct ReadOnlyFixture {
    path: PathBuf,
    original: fs::Permissions,
}

impl ReadOnlyFixture {
    pub fn protect(path: PathBuf) -> Result<Self> {
        let original = fs::metadata(&path)?.permissions();
        let fixture = Self { path, original };
        let mut protected = fixture.original.clone();
        protected.set_readonly(/*readonly*/ true);
        fs::set_permissions(&fixture.path, protected)?;
        Ok(fixture)
    }

    pub fn restore(&self) -> std::io::Result<()> {
        fs::set_permissions(&self.path, self.original.clone())
    }
}

impl Drop for ReadOnlyFixture {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

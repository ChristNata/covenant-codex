use std::path::Path;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use codex_login::REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR;
use serde_json::Value;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::Request;
use wiremock::ResponseTemplate;
use wiremock::matchers::path_regex;

const REQUEST: Duration = Duration::from_secs(/*secs*/ 15);
const SETTLE: Duration = Duration::from_secs(/*secs*/ 5);
const STREAM_LIMIT: usize = 16 * 1024;

pub(super) fn prepare_isolated_root(root: &Path) -> Result<()> {
    let state = root.join("state");
    let auth = root.join("auth");
    std::fs::create_dir_all(&state)?;
    std::fs::create_dir_all(&auth)?;
    let distinct = state.canonicalize()? != auth.canonicalize()?;
    ensure!(distinct, "state and auth homes must be distinct");
    Ok(())
}

pub(super) fn spawn_logout(root: &Path, url: Option<&str>) -> Result<LogoutChild> {
    let state = root.join("state");
    let auth = root.join("auth");
    let mut command = Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    command
        .env_clear()
        .current_dir(&state)
        .env("CODEX_HOME", &state)
        .env("CODEX_AUTH_HOME", &auth)
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .arg("logout")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for name in "TEMP TMP USERPROFILE HOME LOCALAPPDATA APPDATA".split(' ') {
        command.env(name, &state);
    }
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    if let Some(url) = url {
        command.env(REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR, url);
    }
    let canary = state.join("auth.json").try_exists();
    let canary_absent = !canary.context("failed to inspect pre-spawn state auth canary")?;
    ensure!(canary_absent, "state auth canary exists before spawn");
    LogoutChild::spawn(command)
}
type LogoutOutput = (ExitStatus, Vec<u8>, Vec<u8>);
#[derive(Debug, PartialEq, Eq)]
pub(super) struct LogoutObservation {
    pub(super) status_code: Option<i32>,
    pub(super) stdout_empty: bool,
    pub(super) stderr_lines: Vec<String>,
    pub(super) auth_file_matches: bool,
    pub(super) state_auth_absent: bool,
    pub(super) credential_leak: bool,
}
impl LogoutObservation {
    pub(super) fn from_output(
        output: LogoutOutput,
        root: &Path,
        expected_auth: Option<&[u8]>,
        needles: &[&str],
    ) -> Result<Self> {
        let (status, stdout, stderr) = output;
        let stdout = String::from_utf8(stdout).map_err(|_| anyhow!("invalid stdout UTF-8"))?;
        let stderr = String::from_utf8(stderr).map_err(|_| anyhow!("invalid stderr UTF-8"))?;
        let stderr_lines = stderr.lines().map(str::to_string);
        let credential_leak = needles
            .iter()
            .any(|needle| stdout.contains(needle) || stderr.contains(needle));
        let auth_file_matches = match std::fs::read(root.join("auth/auth.json")) {
            Ok(actual) => expected_auth.is_some_and(|expected| actual == expected),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => expected_auth.is_none(),
            Err(error) => return Err(error).context("failed to inspect logout auth file"),
        };
        let canary = root.join("state/auth.json").try_exists();
        let state_auth_absent = !canary.context("failed to inspect post-run state auth canary")?;
        Ok(Self {
            status_code: status.code(),
            stdout_empty: stdout.is_empty(),
            stderr_lines: stderr_lines.collect(),
            auth_file_matches,
            state_auth_absent,
            credential_leak,
        })
    }
}
type Reader = JoinHandle<Result<Vec<u8>>>;
pub(super) struct LogoutChild(Child, (Reader, Reader));
impl LogoutChild {
    fn spawn(mut command: Command) -> Result<Self> {
        let mut child = command.spawn().context("failed to spawn logout child")?;
        let stdout = child.stdout.take().context("stdout unavailable")?;
        let stderr = child.stderr.take().context("stderr unavailable")?;
        let stdout = tokio::spawn(read_capped(stdout));
        let stderr = tokio::spawn(read_capped(stderr));
        Ok(Self(child, (stdout, stderr)))
    }
    pub(super) async fn wait(self) -> Result<LogoutOutput> {
        let Self(mut child, readers) = self;
        let failure = match timeout(REQUEST, child.wait()).await {
            Ok(Ok(status)) => {
                let (stdout, stderr) = finish_readers(readers).await?;
                return Ok((status, stdout, stderr));
            }
            Ok(Err(error)) => anyhow!("failed to wait for logout child: {error}"),
            Err(_) => anyhow!("logout child exceeded its exit deadline"),
        };
        let kill_error = child.start_kill().err();
        let (reap, readers) = tokio::join!(timeout(SETTLE, child.wait()), finish_readers(readers));
        ensure!(kill_error.is_none(), "kill failed after: {failure:#}");
        reap.context("logout child reap timed out")??;
        readers?;
        Err(failure)
    }
}
async fn settle_reader(mut reader: Reader) -> Result<Vec<u8>> {
    match timeout(SETTLE, &mut reader).await {
        Ok(result) => result.context("logout stream reader panicked")?,
        Err(_) => {
            reader.abort();
            let cancelled = timeout(SETTLE, &mut reader).await.is_ok();
            ensure!(cancelled, "reader cancel timed out");
            Err(anyhow!("logout stream reader did not finish"))
        }
    }
}
async fn finish_readers((stdout, stderr): (Reader, Reader)) -> Result<(Vec<u8>, Vec<u8>)> {
    let (stdout, stderr) = tokio::join!(settle_reader(stdout), settle_reader(stderr));
    Ok((stdout?, stderr?))
}

async fn read_capped(input: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut input = input.take((STREAM_LIMIT + 1) as u64);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).await?;
    let within_limit = bytes.len() <= STREAM_LIMIT;
    ensure!(within_limit, "logout child stream exceeded limit");
    Ok(bytes)
}

pub(super) struct HeldRevoke {
    _server: MockServer,
    sync_failed: Arc<AtomicBool>,
    accepted: oneshot::Receiver<bool>,
    release: Option<mpsc::SyncSender<()>>,
}

impl HeldRevoke {
    pub(super) async fn start(expected_body: Value) -> Self {
        let server = MockServer::start().await;
        let sync_failed = Arc::new(AtomicBool::new(/*v*/ false));
        let (accepted_tx, accepted) = oneshot::channel();
        let (release_tx, release_rx) = mpsc::sync_channel(/*bound*/ 1);
        let gate = Mutex::new((Some(accepted_tx), Some(release_rx)));
        let expected = expected_body.clone();
        let failed = Arc::clone(&sync_failed);
        Mock::given(path_regex(".*"))
            .respond_with(move |request: &Request| {
                let body_matches = request.body_json::<Value>().ok().as_ref() == Some(&expected);
                let method_path = request.method == "POST" && request.url.path() == "/oauth/revoke";
                let accepted = method_path && body_matches;
                let (accepted_tx, release_rx) = gate
                    .lock()
                    .map(|mut gate| (gate.0.take(), gate.1.take()))
                    .unwrap_or_default();
                let sent = accepted_tx.is_some_and(|sender| sender.send(accepted).is_ok());
                let released = release_rx.is_some_and(|r| r.recv_timeout(REQUEST).is_ok());
                let released = sent && accepted && released;
                failed.fetch_or(!released, Ordering::SeqCst);
                ResponseTemplate::new(if released { 200 } else { 500 })
            })
            .mount(&server)
            .await;
        Self {
            _server: server,
            sync_failed,
            accepted,
            release: Some(release_tx),
        }
    }

    pub(super) fn url(&self) -> String {
        format!("{}/oauth/revoke", self._server.uri())
    }

    pub(super) async fn wait_accepted(&mut self) -> Result<()> {
        let accepted = timeout(REQUEST, &mut self.accepted).await;
        let accepted = accepted.context("accept timed out")??;
        ensure!(accepted, "unexpected revoke request");
        Ok(())
    }

    pub(super) fn release(&mut self) -> Result<()> {
        let release = self.release.take().context("revoke release missing")?;
        release.try_send(()).context("revoke release failed")
    }

    pub(super) fn verify(self) -> Result<()> {
        let succeeded = !self.sync_failed.load(Ordering::SeqCst);
        ensure!(succeeded, "revoke synchronization failed");
        Ok(())
    }
}

impl Drop for HeldRevoke {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.try_send(());
        }
    }
}

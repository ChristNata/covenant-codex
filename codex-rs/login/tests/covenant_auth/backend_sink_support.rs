//! Bounded child harness shared by the Wave-1 backend tests.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const CHILD: &str = "COVENANT_AUTH_BACKEND_SINK_CHILD";
const COMPLETE: &str = "COVENANT_AUTH_BACKEND_SINK_COMPLETE";
const INPUT_LIMIT: u64 = 131_072;
const OUTPUT_LIMIT: u64 = 131_072;
const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum BackendCase {
    File,
    Direct,
    Secrets,
    AutoDirect,
    AutoDirectFallback,
    AutoSecrets,
    AutoSecretsFallback,
}

impl BackendCase {
    pub(super) fn mode(self) -> AuthCredentialsStoreMode {
        match self {
            Self::File => AuthCredentialsStoreMode::File,
            Self::Direct | Self::Secrets => AuthCredentialsStoreMode::Keyring,
            Self::AutoDirect
            | Self::AutoDirectFallback
            | Self::AutoSecrets
            | Self::AutoSecretsFallback => AuthCredentialsStoreMode::Auto,
        }
    }

    pub(super) fn keyring_kind(self) -> AuthKeyringBackendKind {
        match self {
            Self::Secrets | Self::AutoSecrets | Self::AutoSecretsFallback => {
                AuthKeyringBackendKind::Secrets
            }
            Self::File | Self::Direct | Self::AutoDirect | Self::AutoDirectFallback => {
                AuthKeyringBackendKind::Direct
            }
        }
    }

    pub(super) fn accepts_keyring(self) -> bool {
        !matches!(self, Self::AutoDirectFallback | Self::AutoSecretsFallback)
    }

    pub(super) fn allows_auth_file(self) -> bool {
        matches!(
            self,
            Self::File | Self::AutoDirectFallback | Self::AutoSecretsFallback
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum Scenario {
    Refresh(BackendCase),
    EphemeralDirectLogout,
    EphemeralFreshProbe,
    PersistentManagerLogout,
    EphemeralManagerLogout,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Fixture {
    pub(super) root: PathBuf,
    pub(super) api_key: String,
    pub(super) access_seed: String,
    pub(super) refresh_seed: String,
    pub(super) id_payload: String,
    pub(super) scenario: Scenario,
}

pub(super) fn run(test_name: &str, scenario: Scenario) -> Result<()> {
    run_all(test_name, &[scenario])
}

pub(super) fn run_all(test_name: &str, scenarios: &[Scenario]) -> Result<()> {
    if std::env::var(CHILD).ok().as_deref() == Some(test_name) {
        return run_child();
    }
    for scenario in scenarios {
        run_parent(test_name, *scenario)?;
    }
    Ok(())
}

fn run_parent(test_name: &str, scenario: Scenario) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    let sibling_canary = root.join("unrelated-sibling.canary");
    fs::write(&sibling_canary, b"fixed-unrelated-sibling")?;
    let mut fixture = Fixture {
        root,
        api_key: random_sentinel("api"),
        access_seed: random_sentinel("access"),
        refresh_seed: random_sentinel("refresh"),
        id_payload: format!("{}@example.invalid", random_sentinel("id")),
        scenario,
    };
    spawn_fixture(test_name, &fixture)?;
    if matches!(scenario, Scenario::EphemeralDirectLogout) {
        fixture.scenario = Scenario::EphemeralFreshProbe;
        spawn_fixture(test_name, &fixture)?;
    }
    assert_eq!(fs::read(sibling_canary)?, b"fixed-unrelated-sibling");
    Ok(())
}

fn spawn_fixture(test_name: &str, fixture: &Fixture) -> Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test_name, "--nocapture"])
        .current_dir(&fixture.root)
        .env_clear()
        .env(CHILD, test_name)
        .env("CODEX_HOME", fixture.root.join("mutable"))
        .env("CODEX_AUTH_HOME", fixture.root.join("auth"))
        .env("TEMP", &fixture.root)
        .env("TMP", &fixture.root)
        .env("USERPROFILE", &fixture.root)
        .env("HOME", &fixture.root)
        .env("LOCALAPPDATA", &fixture.root)
        .env("APPDATA", &fixture.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = OwnedChild(command.spawn()?);
    let bytes = serde_json::to_vec(fixture)?;
    ensure!(
        bytes.len() < INPUT_LIMIT as usize,
        "fixture input too large"
    );
    child
        .0
        .stdin
        .take()
        .context("fixture child stdin unavailable")?
        .write_all(&bytes)?;
    let output = wait_output(&mut child)?;
    let stdout = redact(&output.stdout, fixture);
    let stderr = redact(&output.stderr, fixture);
    ensure!(
        output.status.success(),
        "backend child failed for {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        fixture.scenario
    );
    ensure!(
        output
            .stdout
            .windows(COMPLETE.len())
            .any(|window| window == COMPLETE.as_bytes()),
        "backend child did not complete"
    );
    Ok(())
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn wait_output(child: &mut OwnedChild) -> Result<std::process::Output> {
    let stdout = child.0.stdout.take().context("child stdout unavailable")?;
    let stderr = child.0.stderr.take().context("child stderr unavailable")?;
    let out = thread::spawn(move || read_bounded(stdout));
    let err = thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        ensure!(Instant::now() < deadline, "backend child timed out");
        thread::sleep(Duration::from_millis(/*millis*/ 5));
    };
    Ok(std::process::Output {
        status,
        stdout: out
            .join()
            .map_err(|_| anyhow::anyhow!("stdout reader failed"))??,
        stderr: err
            .join()
            .map_err(|_| anyhow::anyhow!("stderr reader failed"))??,
    })
}

fn read_bounded(input: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= OUTPUT_LIMIT as usize,
        "child stream exceeded limit"
    );
    Ok(bytes)
}

fn run_child() -> Result<()> {
    let fixture: Fixture = serde_json::from_reader(std::io::stdin().take(INPUT_LIMIT))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(super::backend_refresh_support::exercise(&fixture))?;
    println!("{COMPLETE}");
    std::io::stdout().flush()?;
    Ok(())
}

pub(super) fn document(fixture: &Fixture, generation: u32) -> Result<AuthDotJson> {
    let payload = serde_json::to_vec(&json!({
        "email": fixture.id_payload,
    }))?;
    let payload = base64_url(&payload).trim_end_matches('=').to_string();
    Ok(serde_json::from_value(json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": fixture.api_key,
        "tokens": {
            "id_token": format!("e30.{payload}.fixture-id-{generation}"),
            "access_token": format!(
                "e30.eyJleHAiOjQxMDI0NDQ4MDB9.{}-{generation}",
                fixture.access_seed
            ),
            "refresh_token": format!("{}-{generation}", fixture.refresh_seed),
            "account_id": "covenant-synthetic-account",
        },
        "last_refresh": "2099-01-01T00:00:00Z",
    }))?)
}

pub(super) fn read_document(home: &Path) -> Result<AuthDotJson> {
    Ok(serde_json::from_slice(&fs::read(home.join("auth.json"))?)?)
}

fn random_sentinel(field: &str) -> String {
    format!("covenant-{field}-{:032x}", rand::random::<u128>())
}

fn redact(bytes: &[u8], fixture: &Fixture) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    for value in [
        &fixture.api_key,
        &fixture.access_seed,
        &fixture.refresh_seed,
        &fixture.id_payload,
    ] {
        text = text.replace(value, "[synthetic secret]");
    }
    text
}

fn base64_url(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::new();
    for chunk in input.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        encoded.push(TABLE[((bits >> 18) & 63) as usize] as char);
        encoded.push(TABLE[((bits >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

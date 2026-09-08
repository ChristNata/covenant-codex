//! Public fail-closed coverage for CODEX_AUTH_HOME initialization and aliases.

#![cfg(windows)]

use super::auth_document;
use super::probe;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::load_auth_dot_json;
use codex_login::login_with_api_key;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const CHILD: &str = "COVENANT_AUTH_HOME_SELECTION_CHILD";
const COMPLETE: &[u8] = b"COVENANT_AUTH_HOME_SELECTION_COMPLETE";

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Operation {
    LoginAndProbe,
    Probe,
    Reject,
}

#[derive(Deserialize, Serialize)]
struct Fixture {
    root: PathBuf,
    override_path: PathBuf,
    selected_home: Option<PathBuf>,
    api_key: String,
    operation: Operation,
    expected_error: Option<String>,
}

#[test]
fn covenant_auth_new_override_root_is_created_without_mutable_fallback() -> Result<()> {
    let test_name =
        "home_selection_tests::covenant_auth_new_override_root_is_created_without_mutable_fallback";
    if is_child(test_name) {
        return run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    let mutable_home = root.join("mutable");
    let auth_home = root.join("new/auth");
    fs::create_dir(&mutable_home)?;
    let decoy = auth_document("covenant-synthetic-mutable-decoy");
    let decoy_bytes = serde_json::to_vec(&decoy)?;
    fs::write(mutable_home.join("auth.json"), &decoy_bytes)?;
    let fixture = fixture(
        &root,
        auth_home.clone(),
        Some(auth_home.clone()),
        Operation::LoginAndProbe,
    );

    spawn(test_name, &fixture)?;

    ensure!(
        read_document(&auth_home)? == auth_document(&fixture.api_key),
        "selected auth document did not match the generated fixture"
    );
    assert_eq!(fs::read(mutable_home.join("auth.json"))?, decoy_bytes);
    Ok(())
}

#[test]
fn covenant_auth_alias_and_canonical_override_share_one_store() -> Result<()> {
    let test_name =
        "home_selection_tests::covenant_auth_alias_and_canonical_override_share_one_store";
    if is_child(test_name) {
        return run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("mutable"))?;
    fs::create_dir(root.join("alias-hop"))?;
    fs::create_dir(root.join("auth"))?;
    let auth_home = root.join("auth").canonicalize()?;
    let alias = root.join("alias-hop").join("..").join("auth");
    let login = fixture(
        &root,
        alias,
        Some(auth_home.clone()),
        Operation::LoginAndProbe,
    );

    spawn(test_name, &login)?;
    let probe_fixture = Fixture {
        operation: Operation::Probe,
        override_path: auth_home.clone(),
        ..login
    };
    spawn(test_name, &probe_fixture)?;

    ensure!(
        read_document(&auth_home)? == auth_document(&probe_fixture.api_key),
        "aliased auth document did not match the generated fixture"
    );
    assert!(!root.join("mutable/auth.json").exists());
    Ok(())
}

#[test]
fn covenant_auth_empty_override_fails_closed() -> Result<()> {
    reject_case(
        "home_selection_tests::covenant_auth_empty_override_fails_closed",
        PathBuf::new(),
        "CODEX_AUTH_HOME must be a nonempty absolute path",
    )
}

#[test]
fn covenant_auth_relative_override_fails_closed() -> Result<()> {
    reject_case(
        "home_selection_tests::covenant_auth_relative_override_fails_closed",
        PathBuf::from("relative-auth"),
        "CODEX_AUTH_HOME must be a nonempty absolute path",
    )
}

#[test]
fn covenant_auth_nondirectory_override_fails_closed() -> Result<()> {
    let test_name = "home_selection_tests::covenant_auth_nondirectory_override_fails_closed";
    if is_child(test_name) {
        return run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("mutable"))?;
    let override_path = root.join("auth-not-directory");
    let original = b"not a directory".to_vec();
    fs::write(&override_path, &original)?;
    reject_in_root(
        test_name,
        root,
        override_path.clone(),
        "unable to initialize CODEX_AUTH_HOME",
    )?;
    ensure!(
        fs::metadata(&override_path)?.is_file(),
        "non-directory override changed file shape"
    );
    ensure!(
        fs::read(&override_path)? == original,
        "non-directory override bytes changed"
    );
    Ok(())
}

fn reject_case(test_name: &str, override_path: PathBuf, expected_error: &str) -> Result<()> {
    if is_child(test_name) {
        return run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("mutable"))?;
    reject_in_root(test_name, root, override_path, expected_error)
}

fn reject_in_root(
    test_name: &str,
    root: PathBuf,
    override_path: PathBuf,
    expected_error: &str,
) -> Result<()> {
    let mutable_auth = root.join("mutable/auth.json");
    let decoy = auth_document("covenant-synthetic-mutable-decoy");
    let decoy_bytes = serde_json::to_vec(&decoy)?;
    fs::write(&mutable_auth, &decoy_bytes)?;
    let mut value = fixture(&root, override_path, None, Operation::Reject);
    value.expected_error = Some(expected_error.to_string());

    spawn(test_name, &value)?;

    assert_eq!(fs::read(&mutable_auth)?, decoy_bytes);
    assert_eq!(read_document(root.join("mutable"))?, decoy);
    Ok(())
}

fn fixture(
    root: &Path,
    override_path: PathBuf,
    selected_home: Option<PathBuf>,
    operation: Operation,
) -> Fixture {
    Fixture {
        root: root.to_path_buf(),
        override_path,
        selected_home,
        api_key: format!("covenant-synthetic-{:032x}", rand::random::<u128>()),
        operation,
        expected_error: None,
    }
}

fn is_child(test_name: &str) -> bool {
    std::env::var(CHILD).ok().as_deref() == Some(test_name)
}

fn spawn(test_name: &str, fixture: &Fixture) -> Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test_name, "--nocapture"])
        .current_dir(&fixture.root)
        .env_clear()
        .env(CHILD, test_name)
        .env("CODEX_HOME", fixture.root.join("mutable"))
        .env("CODEX_AUTH_HOME", &fixture.override_path)
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
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .context("home-selection child stdin is unavailable")?
        .write_all(&serde_json::to_vec(fixture)?)?;
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 30);
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let _ = child.wait();
            anyhow::bail!("home-selection child exceeded its 30-second deadline");
        }
        thread::sleep(Duration::from_millis(/*millis*/ 10));
    }
    let output = child.wait_with_output()?;
    ensure!(
        !contains(&output.stdout, fixture.api_key.as_bytes())
            && !contains(&output.stderr, fixture.api_key.as_bytes()),
        "synthetic auth sentinel escaped the selected store"
    );
    ensure!(
        output.status.success(),
        "home-selection child failed with status {}",
        output.status
    );
    ensure!(
        contains(&output.stdout, COMPLETE),
        "home-selection child did not reach final assertions"
    );
    Ok(())
}

fn run_child() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let fixture: Fixture = serde_json::from_str(&input)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(exercise(&fixture))?;
    std::io::stdout().write_all(COMPLETE)?;
    Ok(())
}

async fn exercise(fixture: &Fixture) -> Result<()> {
    let mutable_home = fixture.root.join("mutable");
    match fixture.operation {
        Operation::LoginAndProbe => {
            login_with_api_key(
                &mutable_home,
                &fixture.api_key,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            )?;
            let selected_home = fixture
                .selected_home
                .as_deref()
                .context("selected home missing from positive fixture")?;
            let expected = auth_document(&fixture.api_key);
            assert_eq!(read_document(selected_home)?, expected);
            probe(&mutable_home, &expected).await?;
        }
        Operation::Probe => {
            let expected = auth_document(&fixture.api_key);
            probe(&mutable_home, &expected).await?;
        }
        Operation::Reject => {
            let expected_error = fixture
                .expected_error
                .as_deref()
                .context("expected error missing from rejection fixture")?;
            let Err(write_error) = login_with_api_key(
                &mutable_home,
                &fixture.api_key,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            ) else {
                anyhow::bail!("invalid CODEX_AUTH_HOME unexpectedly accepted a write");
            };
            let Err(read_error) = load_auth_dot_json(
                &mutable_home,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            ) else {
                anyhow::bail!("invalid CODEX_AUTH_HOME unexpectedly fell back on read");
            };
            assert_eq!(write_error.to_string(), expected_error);
            assert_eq!(read_error.to_string(), expected_error);
            assert_eq!(write_error.kind(), read_error.kind());
        }
    }
    Ok(())
}

fn read_document(home: impl AsRef<Path>) -> Result<AuthDotJson> {
    Ok(serde_json::from_slice(&fs::read(
        home.as_ref().join("auth.json"),
    )?)?)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

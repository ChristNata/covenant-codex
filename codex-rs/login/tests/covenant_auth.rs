//! Windows-only Covenant auth-home acceptance through public codex-login APIs.
//! Every auth operation runs in a child with synthetic homes and a cleared env.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::load_auth_dot_json;
use codex_login::login_with_api_key;
use codex_login::logout;
use codex_login::test_support::transport_default_auth_route_config;
use codex_protocol::auth::AuthMode;
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

const CHILD_TEST: &str = "COVENANT_AUTH_ROUTING_CHILD_TEST";

#[path = "covenant_auth/auto_persistence_support.rs"]
mod auto_persistence_support;
#[path = "covenant_auth/auto_persistence_tests.rs"]
mod auto_persistence_tests;
#[path = "covenant_auth/backend_refresh_support.rs"]
mod backend_refresh_support;
#[path = "covenant_auth/backend_route_support.rs"]
mod backend_route_support;
#[path = "covenant_auth/backend_sink_audit.rs"]
mod backend_sink_audit;
#[path = "covenant_auth/backend_sink_http.rs"]
mod backend_sink_http;
#[path = "covenant_auth/backend_sink_keyring.rs"]
mod backend_sink_keyring;
#[path = "covenant_auth/backend_sink_namespace.rs"]
mod backend_sink_namespace;
#[path = "covenant_auth/backend_sink_support.rs"]
mod backend_sink_support;
#[path = "covenant_auth/backend_sink_tests.rs"]
mod backend_sink_tests;
#[path = "covenant_auth/bounded_child.rs"]
mod bounded_child;
#[path = "covenant_auth/cancellation_support.rs"]
mod cancellation_support;
#[path = "covenant_auth/cancellation_tests.rs"]
mod cancellation_tests;
#[path = "covenant_auth/home_selection_tests.rs"]
mod home_selection_tests;
#[path = "covenant_auth/mutation_race_fixture.rs"]
mod mutation_race_fixture;
#[path = "covenant_auth/mutation_race_http.rs"]
mod mutation_race_http;
#[path = "covenant_auth/mutation_race_linearization.rs"]
mod mutation_race_linearization;
#[path = "covenant_auth/mutation_race_owner_loss.rs"]
mod mutation_race_owner_loss;
#[path = "covenant_auth/mutation_race_process.rs"]
mod mutation_race_process;
#[path = "covenant_auth/mutation_race_recovery.rs"]
mod mutation_race_recovery;
#[path = "covenant_auth/mutation_race_tests.rs"]
mod mutation_race_tests;
#[path = "covenant_auth/persistence_support.rs"]
mod persistence_support;
#[path = "covenant_auth/persistence_tests.rs"]
mod persistence_tests;
#[path = "covenant_auth/rotation_authority.rs"]
mod rotation_authority;
#[path = "covenant_auth/rotation_support.rs"]
mod rotation_support;
#[path = "covenant_auth/rotation_tests.rs"]
mod rotation_tests;

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Operation {
    Login,
    Probe,
    Logout,
    ManagerLogout,
    CallerHomeFallback,
}

#[derive(Deserialize, Serialize)]
struct Fixture {
    root: PathBuf,
    api_key: String,
    operation: Operation,
}

enum Scenario {
    RedirectedLogin,
    RedirectedProbe,
    RedirectedLogout,
    RedirectedManagerLogout,
    MutableHomeDeletion,
    UnsetFallback,
}

#[test]
fn covenant_auth_login_writes_only_to_auth_home() -> Result<()> {
    run_scenario(
        "covenant_auth_login_writes_only_to_auth_home",
        Scenario::RedirectedLogin,
    )
}

#[test]
fn covenant_auth_probe_prefers_auth_home_over_mutable_home() -> Result<()> {
    run_scenario(
        "covenant_auth_probe_prefers_auth_home_over_mutable_home",
        Scenario::RedirectedProbe,
    )
}

#[test]
fn covenant_auth_logout_removes_only_redirected_credentials() -> Result<()> {
    run_scenario(
        "covenant_auth_logout_removes_only_redirected_credentials",
        Scenario::RedirectedLogout,
    )
}

#[test]
fn covenant_auth_manager_logout_clears_redirected_store_and_cache() -> Result<()> {
    run_scenario(
        "covenant_auth_manager_logout_clears_redirected_store_and_cache",
        Scenario::RedirectedManagerLogout,
    )
}

#[test]
fn covenant_auth_fresh_probe_survives_mutable_home_deletion() -> Result<()> {
    run_scenario(
        "covenant_auth_fresh_probe_survives_mutable_home_deletion",
        Scenario::MutableHomeDeletion,
    )
}

#[test]
fn covenant_auth_unset_override_preserves_each_caller_home() -> Result<()> {
    run_scenario(
        "covenant_auth_unset_override_preserves_each_caller_home",
        Scenario::UnsetFallback,
    )
}

fn run_scenario(test_name: &str, scenario: Scenario) -> Result<()> {
    if std::env::var(CHILD_TEST).ok().as_deref() == Some(test_name) {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        let fixture: Fixture = serde_json::from_str(&input)?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(exercise_public_auth(&fixture))?;
        println!("COVENANT_AUTH_FIXTURE_COMPLETE");
        return Ok(());
    }

    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    let mutable_home = root.join("mutable");
    let auth_home = root.join("auth");
    fs::create_dir(&mutable_home)?;
    fs::create_dir(&auth_home)?;
    let mut fixture = Fixture {
        root,
        api_key: format!("covenant-synthetic-{:032x}", rand::random::<u128>()),
        operation: Operation::Login,
    };

    match scenario {
        Scenario::RedirectedLogin => spawn_fixture(test_name, &fixture),
        Scenario::RedirectedProbe
        | Scenario::RedirectedLogout
        | Scenario::RedirectedManagerLogout => {
            // Seed only disposable fixture files; public reads/deletes must choose
            // the redirected credential even when mutable home has a decoy.
            fs::write(
                auth_home.join("auth.json"),
                serde_json::to_vec(&auth_document(&fixture.api_key))?,
            )?;
            let decoy = auth_document("covenant-synthetic-decoy");
            let decoy_bytes = serde_json::to_vec(&decoy)?;
            fs::write(mutable_home.join("auth.json"), &decoy_bytes)?;
            fixture.operation = match scenario {
                Scenario::RedirectedProbe => Operation::Probe,
                Scenario::RedirectedLogout => Operation::Logout,
                Scenario::RedirectedManagerLogout => Operation::ManagerLogout,
                Scenario::RedirectedLogin
                | Scenario::MutableHomeDeletion
                | Scenario::UnsetFallback => unreachable!(),
            };
            spawn_fixture(test_name, &fixture)?;
            assert_eq!(fs::read(mutable_home.join("auth.json"))?, decoy_bytes);
            Ok(())
        }
        Scenario::MutableHomeDeletion => {
            spawn_fixture(test_name, &fixture)?;
            // These exact sibling directories were created in this TempDir above.
            assert!(mutable_home.is_absolute());
            assert_eq!(mutable_home.parent(), Some(fixture.root.as_path()));
            fs::remove_dir_all(&mutable_home)?;
            fixture.operation = Operation::Probe;
            spawn_fixture(test_name, &fixture)?;
            assert!(
                !mutable_home.exists(),
                "auth probing recreated mutable home"
            );
            Ok(())
        }
        Scenario::UnsetFallback => {
            fixture.operation = Operation::CallerHomeFallback;
            spawn_fixture(test_name, &fixture)
        }
    }
}

fn spawn_fixture(test_name: &str, fixture: &Fixture) -> Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test_name, "--nocapture"])
        .current_dir(&fixture.root)
        .env_clear()
        .env(CHILD_TEST, test_name)
        .env("CODEX_HOME", fixture.root.join("mutable"))
        .env("TEMP", &fixture.root)
        .env("TMP", &fixture.root)
        .env("USERPROFILE", &fixture.root)
        .env("HOME", &fixture.root)
        .env("LOCALAPPDATA", &fixture.root)
        .env("APPDATA", &fixture.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Keep only Windows runtime locations; never inherit auth selectors/secrets.
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    if !matches!(fixture.operation, Operation::CallerHomeFallback) {
        command.env("CODEX_AUTH_HOME", fixture.root.join("auth"));
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .context("fixture child stdin is unavailable")?
        .write_all(&serde_json::to_vec(fixture)?)?;
    let output = child.wait_with_output()?;
    let stdout =
        String::from_utf8_lossy(&output.stdout).replace(&fixture.api_key, "[synthetic key]");
    let stderr =
        String::from_utf8_lossy(&output.stderr).replace(&fixture.api_key, "[synthetic key]");
    assert!(
        output.status.success(),
        "public auth fixture {test_name} failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("COVENANT_AUTH_FIXTURE_COMPLETE"),
        "public auth fixture did not execute its assertions"
    );
    Ok(())
}

async fn exercise_public_auth(fixture: &Fixture) -> Result<()> {
    let mutable_home = fixture.root.join("mutable");
    let auth_home = fixture.root.join("auth");
    assert_eq!(
        std::env::var_os("CODEX_HOME").map(PathBuf::from),
        Some(mutable_home.clone())
    );
    let expected = auth_document(&fixture.api_key);
    match fixture.operation {
        Operation::Login => {
            login_with_api_key(
                &mutable_home,
                &fixture.api_key,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            )?;
            assert!(
                auth_home.join("auth.json").is_file(),
                "login did not write auth.json under CODEX_AUTH_HOME"
            );
            assert_eq!(fs::read_dir(&mutable_home)?.count(), 0);
            assert_eq!(
                serde_json::from_slice::<AuthDotJson>(&fs::read(auth_home.join("auth.json"))?)?,
                expected
            );
            probe(&mutable_home, &expected).await?;
        }
        Operation::Probe => {
            probe(&mutable_home, &expected).await?;
        }
        Operation::Logout => {
            assert!(logout(
                &mutable_home,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            )?);
            assert!(!auth_home.join("auth.json").exists());
            assert_eq!(
                load_auth_dot_json(
                    &mutable_home,
                    AuthCredentialsStoreMode::File,
                    AuthKeyringBackendKind::Direct,
                )?,
                None
            );
        }
        Operation::ManagerLogout => {
            let manager = probe(&mutable_home, &expected).await?;
            assert!(manager.logout().await?);
            assert!(manager.auth().await.is_none());
            assert!(!auth_home.join("auth.json").exists());
        }
        Operation::CallerHomeFallback => {
            assert!(std::env::var_os("CODEX_AUTH_HOME").is_none());
            let second_home = fixture.root.join("second-mutable");
            let second_key = format!("{}-second", fixture.api_key);
            for (home, key) in [
                (mutable_home.as_path(), fixture.api_key.as_str()),
                (second_home.as_path(), second_key.as_str()),
            ] {
                login_with_api_key(
                    home,
                    key,
                    AuthCredentialsStoreMode::File,
                    AuthKeyringBackendKind::Direct,
                )?;
                assert!(home.join("auth.json").is_file());
                probe(home, &auth_document(key)).await?;
            }
            probe(&mutable_home, &expected).await?;
            assert_eq!(fs::read_dir(auth_home)?.count(), 0);
        }
    }
    Ok(())
}

async fn probe(home: &Path, expected: &AuthDotJson) -> Result<std::sync::Arc<AuthManager>> {
    assert_eq!(
        load_auth_dot_json(
            home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        Some(expected.clone())
    );
    let manager = AuthManager::shared(
        home.to_path_buf(),
        /*enable_codex_api_key_env*/ false,
        AuthCredentialsStoreMode::File,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        AuthKeyringBackendKind::Direct,
        transport_default_auth_route_config(),
    )
    .await;
    let auth = manager
        .auth()
        .await
        .context("public auth probe found no login")?;
    assert_eq!(auth.api_key(), expected.openai_api_key.as_deref());
    Ok(manager)
}

fn auth_document(api_key: &str) -> AuthDotJson {
    AuthDotJson {
        auth_mode: Some(AuthMode::ApiKey),
        openai_api_key: Some(api_key.to_string()),
        tokens: None,
        last_refresh: None,
        agent_identity: None,
        personal_access_token: None,
        bedrock_api_key: None,
        bedrock_access_keys: None,
    }
}

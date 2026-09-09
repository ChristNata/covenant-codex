//! File refresh and logout behavior for the first Wave-1 checkpoint.

#![cfg(windows)]

use super::auth_document;
use super::backend_sink_http::ExpectedBody;
use super::backend_sink_http::HttpFixture;
use super::backend_sink_http::Step;
use super::backend_sink_keyring::MemoryStore;
use super::backend_sink_support::BackendCase;
use super::backend_sink_support::Fixture;
use super::backend_sink_support::Scenario;
use super::backend_sink_support::document;
use super::backend_sink_support::read_document;
use super::rotation_support;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::load_auth_dot_json;
use codex_login::logout;
use codex_login::save_auth;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

pub(super) async fn exercise(fixture: &Fixture, keyring: &Arc<Mutex<MemoryStore>>) -> Result<()> {
    match fixture.scenario {
        Scenario::Refresh(case) => refresh(fixture, case, keyring).await,
        Scenario::EphemeralDirectLogout => ephemeral_direct_logout(fixture),
        Scenario::EphemeralFreshProbe => ephemeral_fresh_probe(fixture),
        Scenario::PersistentManagerLogout => persistent_manager_logout(fixture).await,
        Scenario::EphemeralManagerLogout => ephemeral_manager_logout(fixture).await,
        Scenario::ApiKeyLoginProbeLogout
        | Scenario::AccessTokenLoginProbe
        | Scenario::BrowserCallbackProbe
        | Scenario::DeviceCodeProbe
        | Scenario::RevokeSuccess
        | Scenario::RevokeFailure => anyhow::bail!("public route sent to backend refresh fixture"),
    }
}

async fn refresh(
    fixture: &Fixture,
    case: BackendCase,
    keyring: &Arc<Mutex<MemoryStore>>,
) -> Result<()> {
    let home = fixture.root.join("mutable");
    let selected = fixture.root.join("auth");
    let decoy = auth_document("covenant-decoy");
    fs::write(selected.join("auth.json"), serde_json::to_vec(&decoy)?)?;
    let initial = document(fixture, /*generation*/ 0)?;
    let next = document(fixture, /*generation*/ 1)?;
    let refresh_token = initial
        .tokens
        .as_ref()
        .context("initial refresh token missing")?
        .refresh_token
        .clone();
    let authority = HttpFixture::start(
        vec![Step::json(
            "POST",
            "/oauth/token",
            /*status*/ 200,
            token_body(&next)?,
            ExpectedBody::Json(json!({
                "client_id": codex_login::CLIENT_ID,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            })),
        )?],
        |url| {
            // SAFETY: the isolated child uses a current-thread runtime, and
            // the HTTP fixture has bound its listener but starts no thread
            // until this callback returns.
            unsafe {
                std::env::set_var(
                    codex_login::REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR,
                    format!("{url}/oauth/token"),
                );
            }
            Ok(())
        },
    )?;
    save_auth(&home, &initial, case.mode(), case.keyring_kind())?;
    let retained = if case.allows_auth_file() {
        let path = selected.join("auth.json");
        Some((fs::File::open(&path)?, fs::read(path)?))
    } else {
        None
    };
    let started_at = rotation_support::unix_seconds()?;
    let manager = manager(&home, case.mode(), case.keyring_kind()).await;
    let refresh_result = manager.refresh_token_from_authority().await;
    let authority_result = authority.finish();
    authority_result.context("refresh authority fixture failed")?;
    refresh_result?;

    let mut expected = initial;
    expected.tokens = next.tokens;
    let loaded = load_auth_dot_json(&home, case.mode(), case.keyring_kind())?
        .context("refreshed document missing")?;
    ensure!(
        rotation_support::whole_document_matches(&loaded, &expected, started_at),
        "refreshed whole document mismatch"
    );
    let cached = manager.auth().await.context("refreshed cache missing")?;
    let expected_tokens = expected
        .tokens
        .clone()
        .context("expected refreshed tokens missing")?;
    assert_eq!(cached.get_token_data()?, expected_tokens);
    assert_eq!(
        selected.join("auth.json").is_file(),
        case.allows_auth_file()
    );
    ensure!(
        !home.join("auth.json").exists(),
        "refresh wrote mutable home"
    );
    let alternate_caller = fixture.root.join("alternate-caller");
    fs::create_dir(&alternate_caller)?;
    let routed = load_auth_dot_json(&alternate_caller, case.mode(), case.keyring_kind())?;
    assert_eq!(routed, Some(loaded));
    ensure!(
        !alternate_caller.join("auth.json").exists(),
        "refresh wrote alternate caller home"
    );
    if let Some((mut reader, prior)) = retained {
        let mut retained_bytes = Vec::new();
        reader.read_to_end(&mut retained_bytes)?;
        assert_eq!(retained_bytes, prior);
    }
    let operation_count = keyring
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .operation_count();
    ensure!(
        operation_count > 0 || matches!(case, BackendCase::File),
        "keyring route was not exercised"
    );
    Ok(())
}

fn ephemeral_direct_logout(fixture: &Fixture) -> Result<()> {
    let home = fixture.root.join("mutable");
    let selected = fixture.root.join("auth");
    let persistent = auth_document("persistent-canary");
    fs::write(selected.join("auth.json"), serde_json::to_vec(&persistent)?)?;
    let ephemeral = auth_document(&fixture.api_key);
    save_auth(
        &home,
        &ephemeral,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )?;
    ensure!(
        logout(
            &home,
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::Direct,
        )?,
        "ephemeral logout removed nothing"
    );
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
    assert_eq!(read_document(&selected)?, persistent);
    save_auth(
        &home,
        &ephemeral,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )?;
    Ok(())
}

fn ephemeral_fresh_probe(fixture: &Fixture) -> Result<()> {
    assert_eq!(
        load_auth_dot_json(
            &fixture.root.join("mutable"),
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
    Ok(())
}

async fn persistent_manager_logout(fixture: &Fixture) -> Result<()> {
    let home = fixture.root.join("mutable");
    let persistent = auth_document(&fixture.api_key);
    save_auth(
        &home,
        &persistent,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?;
    save_auth(
        &home,
        &persistent,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )?;
    let manager = manager(
        &home,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )
    .await;
    ensure!(manager.logout().await?, "manager logout removed nothing");
    ensure!(
        manager.auth().await.is_none(),
        "manager cache survived logout"
    );
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
    Ok(())
}

async fn ephemeral_manager_logout(fixture: &Fixture) -> Result<()> {
    let home = fixture.root.join("mutable");
    let persistent = auth_document("persistent-canary");
    let ephemeral = auth_document(&fixture.api_key);
    save_auth(
        &home,
        &persistent,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?;
    save_auth(
        &home,
        &ephemeral,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )?;
    let manager = manager(
        &home,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )
    .await;
    ensure!(manager.logout().await?, "ephemeral manager removed nothing");
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        Some(persistent)
    );
    Ok(())
}

async fn manager(
    home: &Path,
    mode: AuthCredentialsStoreMode,
    keyring_kind: AuthKeyringBackendKind,
) -> std::sync::Arc<AuthManager> {
    AuthManager::shared(
        home.to_path_buf(),
        /*enable_codex_api_key_env*/ false,
        mode,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        keyring_kind,
        codex_login::test_support::transport_default_auth_route_config(),
    )
    .await
}

fn token_body(document: &AuthDotJson) -> Result<serde_json::Value> {
    let tokens = document.tokens.as_ref().context("token document missing")?;
    Ok(json!({
        "id_token": tokens.id_token.raw_jwt,
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
    }))
}

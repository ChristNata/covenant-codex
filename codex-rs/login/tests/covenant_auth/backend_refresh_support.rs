//! File refresh and logout behavior for the first Wave-1 checkpoint.

#![cfg(windows)]

use super::auth_document;
use super::backend_sink_http::HttpFixture;
use super::backend_sink_http::Step;
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

pub(super) async fn exercise(fixture: &Fixture) -> Result<()> {
    match fixture.scenario {
        Scenario::FileRefresh => refresh_file(fixture).await,
        Scenario::EphemeralDirectLogout => ephemeral_direct_logout(fixture),
        Scenario::EphemeralFreshProbe => ephemeral_fresh_probe(fixture),
        Scenario::PersistentManagerLogout => persistent_manager_logout(fixture).await,
        Scenario::EphemeralManagerLogout => ephemeral_manager_logout(fixture).await,
    }
}

async fn refresh_file(fixture: &Fixture) -> Result<()> {
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
    let authority = HttpFixture::start(vec![Step::json(
        "POST",
        "/oauth/token",
        /*status*/ 200,
        token_body(&next)?,
        Some(refresh_token),
    )?])?;
    // SAFETY: this isolated child uses a current-thread runtime. Its fixture
    // server does not inspect or modify the process environment.
    unsafe {
        std::env::set_var(
            codex_login::REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR,
            format!("{}/oauth/token", authority.url()),
        );
    }
    save_auth(
        &home,
        &initial,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?;
    let path = selected.join("auth.json");
    let mut retained_reader = fs::File::open(&path)?;
    let prior = fs::read(&path)?;
    let started_at = rotation_support::unix_seconds()?;
    let manager = manager(&home, AuthCredentialsStoreMode::File).await;
    manager.refresh_token_from_authority().await?;
    authority.finish()?;

    let mut expected = initial;
    expected.tokens = next.tokens;
    let loaded = load_auth_dot_json(
        &home,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?
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
    ensure!(
        !home.join("auth.json").exists(),
        "refresh wrote mutable home"
    );
    let mut retained_bytes = Vec::new();
    retained_reader.read_to_end(&mut retained_bytes)?;
    assert_eq!(retained_bytes, prior);
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
    let manager = manager(&home, AuthCredentialsStoreMode::File).await;
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
    let manager = manager(&home, AuthCredentialsStoreMode::Ephemeral).await;
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

async fn manager(home: &Path, mode: AuthCredentialsStoreMode) -> std::sync::Arc<AuthManager> {
    AuthManager::shared(
        home.to_path_buf(),
        /*enable_codex_api_key_env*/ false,
        mode,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        AuthKeyringBackendKind::Direct,
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

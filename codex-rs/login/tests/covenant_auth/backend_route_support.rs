//! Public login, completion, and revoke routes for Wave 1.

#![cfg(windows)]

use super::auth_document;
use super::backend_sink_http::ExpectedBody;
use super::backend_sink_http::HttpFixture;
use super::backend_sink_http::PkceExpectation;
use super::backend_sink_http::Step;
use super::backend_sink_http::assert_rejection_probes;
use super::backend_sink_http::parse_form;
use super::backend_sink_http::raw_get;
use super::backend_sink_support::Fixture;
use super::backend_sink_support::Scenario;
use super::backend_sink_support::document;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::LoginSuccessPage;
use codex_login::ServerOptions;
use codex_login::load_auth_dot_json;
use codex_login::login_with_access_token;
use codex_login::login_with_api_key;
use codex_login::logout;
use codex_login::logout_with_revoke;
use codex_login::run_device_code_login;
use codex_login::run_login_server;
use codex_login::save_auth;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;

pub(super) async fn exercise(fixture: &Fixture) -> Result<()> {
    match fixture.scenario {
        Scenario::ApiKeyLoginProbeLogout => api_key_route(fixture).await,
        Scenario::AccessTokenLoginProbe => access_token_route(fixture).await,
        Scenario::BrowserCallbackProbe => browser_route(fixture).await,
        Scenario::DeviceCodeProbe => device_route(fixture).await,
        Scenario::RevokeSuccess => revoke_route(fixture, RevokeOutcome::Success).await,
        Scenario::RevokeFailure => revoke_route(fixture, RevokeOutcome::Failure).await,
        Scenario::Refresh(_)
        | Scenario::EphemeralDirectLogout
        | Scenario::EphemeralFreshProbe
        | Scenario::PersistentManagerLogout
        | Scenario::EphemeralManagerLogout => anyhow::bail!("backend case sent to route fixture"),
    }
}

async fn api_key_route(fixture: &Fixture) -> Result<()> {
    let home = fixture.root.join("mutable");
    let expected = auth_document(&fixture.api_key);
    let api_key = expected
        .openai_api_key
        .as_deref()
        .context("synthetic API key missing")?;
    login_with_api_key(
        &home,
        api_key,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?;
    probe(&home, &expected).await?;
    ensure!(
        logout(
            &home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        "API-key logout removed nothing"
    );
    Ok(())
}

async fn access_token_route(fixture: &Fixture) -> Result<()> {
    let home = fixture.root.join("mutable");
    let token = fixture.personal_access_token.clone();
    let response = || {
        Step::json(
            "GET",
            "/v1/user-auth-credential/whoami",
            /*status*/ 200,
            json!({
                "email": "fixture@example.com",
                "chatgpt_user_id": "fixture-user",
                "chatgpt_account_id": "fixture-account",
                "chatgpt_plan_type": "business",
                "chatgpt_account_is_fedramp": false,
            }),
            ExpectedBody::Empty,
        )
        .map(|step| step.bearer(format!("Bearer {token}")))
    };
    let authority = HttpFixture::start(vec![response()?, response()?], |url| {
        // SAFETY: the isolated child uses a current-thread runtime, and the
        // HTTP fixture starts no thread until this callback returns.
        unsafe { std::env::set_var("CODEX_AUTHAPI_BASE_URL", url) };
        Ok(())
    })?;
    login_with_access_token(
        &home,
        &token,
        AuthCredentialsStoreMode::File,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        AuthKeyringBackendKind::Direct,
        &route(),
    )
    .await?;
    let expected = AuthDotJson {
        auth_mode: None,
        openai_api_key: None,
        tokens: None,
        last_refresh: None,
        agent_identity: None,
        personal_access_token: Some(token),
        bedrock_api_key: None,
        bedrock_access_keys: None,
    };
    probe(&home, &expected).await?;
    authority.finish()?;
    Ok(())
}

async fn browser_route(fixture: &Fixture) -> Result<()> {
    assert_rejection_probes()?;
    let expected = document(fixture, /*generation*/ 0)?;
    let id_token = &expected
        .tokens
        .as_ref()
        .context("browser token document missing")?
        .id_token
        .raw_jwt;
    let (pkce_sender, pkce_binding) = mpsc::sync_channel(/*bound*/ 1);
    let authority = HttpFixture::start(
        vec![
            Step::json(
                "POST",
                "/oauth/token",
                /*status*/ 200,
                token_body(&expected)?,
                ExpectedBody::PkceForm {
                    expected: form_fields(&[
                        ("grant_type", "authorization_code"),
                        ("code", "fixture-code"),
                        ("client_id", codex_login::CLIENT_ID),
                    ]),
                    binding: pkce_binding,
                },
            )?,
            Step::json(
                "POST",
                "/oauth/token",
                /*status*/ 500,
                json!({"message": "fixed fixture response"}),
                form(&[
                    (
                        "grant_type",
                        "urn:ietf:params:oauth:grant-type:token-exchange",
                    ),
                    ("client_id", codex_login::CLIENT_ID),
                    ("requested_token", "openai-api-key"),
                    ("subject_token", id_token),
                    (
                        "subject_token_type",
                        "urn:ietf:params:oauth:token-type:id_token",
                    ),
                ]),
            )?,
        ],
        |_| Ok(()),
    )?;
    let login = run_login_server(server_options(fixture, authority.url().to_string()))?;
    let callback_port = login.actual_port;
    let redirect_uri = format!("http://localhost:{callback_port}/auth/callback");
    let query = login
        .auth_url
        .split_once('?')
        .context("authorization query missing")?
        .1;
    let mut fields = parse_form(query.as_bytes())?;
    assert_eq!(fields.remove("redirect_uri"), Some(redirect_uri.clone()));
    assert_eq!(fields.remove("state").as_deref(), Some("fixed-state"));
    assert_eq!(
        fields.remove("code_challenge_method").as_deref(),
        Some("S256")
    );
    let code_challenge = fields
        .remove("code_challenge")
        .context("challenge missing")?;
    pkce_sender
        .try_send(PkceExpectation {
            redirect_uri,
            code_challenge,
        })
        .context("PKCE binding duplicate or late")?;
    drop(pkce_sender);
    let callback = tokio::task::spawn_blocking(move || follow_local_success(callback_port));
    login.block_until_done().await?;
    callback.await??;
    authority.finish()?;
    probe_persisted_tokens(&fixture.root.join("mutable"), &expected).await
}

fn follow_local_success(port: u16) -> Result<()> {
    let callback = raw_get(port, "/auth/callback?code=fixture-code&state=fixed-state")?;
    ensure!(
        callback.starts_with("HTTP/1.1 302"),
        "callback did not redirect"
    );
    let location = callback
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("location")
                .then_some(value.trim())
        })
        .context("callback location missing")?;
    let authority = format!("localhost:{port}");
    let (_, path) = location
        .split_once(&authority)
        .context("callback location was not local")?;
    let success = raw_get(port, path)?;
    ensure!(
        success.starts_with("HTTP/1.1 200"),
        "success page did not complete"
    );
    Ok(())
}

async fn device_route(fixture: &Fixture) -> Result<()> {
    let expected = document(fixture, /*generation*/ 0)?;
    let authority = HttpFixture::start_dynamic(
        |issuer| {
            Ok(vec![
                Step::json(
                    "POST",
                    "/api/accounts/deviceauth/usercode",
                    /*status*/ 200,
                    json!({
                        "device_auth_id": "fixture-device",
                        "user_code": "FIXTURE",
                        "interval": "0",
                    }),
                    ExpectedBody::Json(json!({"client_id": codex_login::CLIENT_ID})),
                )?,
                Step::json(
                    "POST",
                    "/api/accounts/deviceauth/token",
                    /*status*/ 200,
                    json!({
                        "authorization_code": "fixture-code",
                        "code_challenge": "fixture-challenge",
                        "code_verifier": "fixture-verifier",
                    }),
                    ExpectedBody::Json(json!({
                        "device_auth_id": "fixture-device",
                        "user_code": "FIXTURE",
                    })),
                )?,
                Step::json(
                    "POST",
                    "/oauth/token",
                    /*status*/ 200,
                    token_body(&expected)?,
                    form(&[
                        ("grant_type", "authorization_code"),
                        ("code", "fixture-code"),
                        ("redirect_uri", &format!("{issuer}/deviceauth/callback")),
                        ("client_id", codex_login::CLIENT_ID),
                        ("code_verifier", "fixture-verifier"),
                    ]),
                )?,
            ])
        },
        |_| Ok(()),
    )?;
    let issuer = authority.url().to_string();
    run_device_code_login(server_options(fixture, issuer)).await?;
    authority.finish()?;
    probe_persisted_tokens(&fixture.root.join("mutable"), &expected).await
}

#[derive(Clone, Copy)]
enum RevokeOutcome {
    Success,
    Failure,
}

async fn revoke_route(fixture: &Fixture, outcome: RevokeOutcome) -> Result<()> {
    let home = fixture.root.join("mutable");
    let expected = document(fixture, /*generation*/ 0)?;
    save_auth(
        &home,
        &expected,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?;
    let refresh = expected
        .tokens
        .as_ref()
        .context("revoke refresh token missing")?
        .refresh_token
        .clone();
    let status = match outcome {
        RevokeOutcome::Success => 200,
        RevokeOutcome::Failure => 500,
    };
    let authority = HttpFixture::start(
        vec![Step::json(
            "POST",
            "/oauth/revoke",
            status,
            json!({"message": "fixed fixture response"}),
            ExpectedBody::Json(json!({
                "token": refresh,
                "token_type_hint": "refresh_token",
                "client_id": codex_login::CLIENT_ID,
            })),
        )?],
        |url| {
            // SAFETY: the isolated child uses a current-thread runtime, and the
            // HTTP fixture starts no thread until this callback returns.
            unsafe {
                std::env::set_var(
                    codex_login::REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR,
                    format!("{url}/oauth/revoke"),
                );
            }
            Ok(())
        },
    )?;
    ensure!(
        logout_with_revoke(
            &home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
            &route(),
        )
        .await?,
        "revoke logout removed nothing"
    );
    authority.finish()?;
    assert_eq!(
        load_auth_dot_json(
            &home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        None
    );
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
        route(),
    )
    .await
}

async fn probe(home: &Path, expected: &AuthDotJson) -> Result<()> {
    ensure!(
        !home.join("auth.json").exists(),
        "public route wrote caller home"
    );
    assert_eq!(
        load_auth_dot_json(
            home,
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        )?,
        Some(expected.clone())
    );
    ensure!(
        manager(home).await.auth().await.is_some(),
        "public manager probe found no auth"
    );
    Ok(())
}

async fn probe_persisted_tokens(home: &Path, issued: &AuthDotJson) -> Result<()> {
    let observed = load_auth_dot_json(
        home,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )?
    .context("persisted route document missing")?;
    let expected = AuthDotJson {
        auth_mode: issued.auth_mode,
        openai_api_key: None,
        tokens: issued.tokens.clone(),
        last_refresh: observed.last_refresh,
        agent_identity: None,
        personal_access_token: None,
        bedrock_api_key: None,
        bedrock_access_keys: None,
    };
    assert_eq!(observed, expected);
    probe(home, &expected).await
}

fn route() -> codex_login::AuthRouteConfig {
    codex_login::test_support::transport_default_auth_route_config()
}

fn server_options(fixture: &Fixture, issuer: String) -> ServerOptions {
    ServerOptions {
        codex_home: fixture.root.join("mutable"),
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some("fixed-state".to_string()),
        forced_chatgpt_workspace_id: None,
        codex_streamlined_login: false,
        login_success_page: LoginSuccessPage::default(),
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        auth_keyring_backend_kind: AuthKeyringBackendKind::Direct,
        auth_route_config: route(),
    }
}

fn form(fields: &[(&str, &str)]) -> ExpectedBody {
    ExpectedBody::Form(form_fields(fields))
}

fn form_fields(fields: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    fields
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn token_body(document: &AuthDotJson) -> Result<serde_json::Value> {
    let tokens = document.tokens.as_ref().context("token document missing")?;
    Ok(json!({
        "id_token": tokens.id_token.raw_jwt,
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
    }))
}

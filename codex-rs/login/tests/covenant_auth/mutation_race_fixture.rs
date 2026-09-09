//! Shared synthetic state and child protocol for mutation-race tests.

use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::RefreshTokenError;
use codex_login::load_auth_dot_json;
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
use std::sync::Arc;
use std::time::Duration;

pub(super) const CHILD: &str = "COVENANT_AUTH_MUTATION_RACE_CHILD";
pub(super) const AGENT_ENDPOINT: &str = "CODEX_AGENT_IDENTITY_AUTHAPI_BASE_URL";
pub(super) const PREFIX: &str = "COVENANT_MUTATION_RACE ";
pub(super) const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);
pub(super) const MAX_CHILD_INPUT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Outcome {
    Success,
    Busy,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Report {
    pub outcome: Outcome,
    pub whole_document: bool,
    pub token_or_api_key_cache: bool,
}

impl Report {
    pub(super) fn passed(&self, outcome: Outcome) -> bool {
        self.outcome == outcome && self.whole_document && self.token_or_api_key_cache
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) enum Operation {
    Refresh {
        expected: AuthDotJson,
    },
    Save {
        document: AuthDotJson,
        prior: Box<AuthDotJson>,
    },
    Login {
        api_key: String,
        expected: AuthDotJson,
        prior: Box<AuthDotJson>,
    },
    AgentMetadata {
        expected_base: AuthDotJson,
        prior: Box<AuthDotJson>,
    },
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Fixture {
    pub root: PathBuf,
    pub operation: Operation,
    pub refresh_endpoint: String,
    pub agent_endpoint: String,
}

#[derive(Deserialize, Serialize)]
pub(super) enum Event {
    Ready,
    Entered,
    Done(Report),
    OutputLimitExceeded,
    Closed,
}

pub(super) struct PreparedRoot {
    _temporary: tempfile::TempDir,
    pub root: PathBuf,
    pub nonce: String,
    pub initial: AuthDotJson,
}

impl PreparedRoot {
    pub(super) fn new() -> Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        let nonce = format!("covenant-mutation-{:032x}", rand::random::<u128>());
        let initial = document(&nonce, /*generation*/ 0)?;
        prepare_root(&root, &initial)?;
        Ok(Self {
            _temporary: temporary,
            root,
            nonce,
            initial,
        })
    }
}

pub(super) fn fixture(root: &Path, operation: Operation) -> Fixture {
    Fixture {
        root: root.to_path_buf(),
        operation,
        refresh_endpoint: "http://127.0.0.1:9".to_string(),
        agent_endpoint: "http://127.0.0.1:9".to_string(),
    }
}

pub(super) fn is_child(test_name: &str) -> bool {
    std::env::var(CHILD).ok().as_deref() == Some(test_name)
}

pub(super) fn document(nonce: &str, generation: u32) -> Result<AuthDotJson> {
    const CLAIMS: &str = "eyJlbWFpbCI6ImZpeHR1cmVAZXhhbXBsZS5pbnZhbGlkIiwiaHR0cHM6Ly9hcGkub3BlbmFpLmNvbS9hdXRoIjp7ImNoYXRncHRfYWNjb3VudF9pZCI6ImNvdmVuYW50LXN5bnRoZXRpYy1hY2NvdW50IiwiY2hhdGdwdF91c2VyX2lkIjoiY292ZW5hbnQtc3ludGhldGljLXVzZXIiLCJjaGF0Z3B0X3BsYW5fdHlwZSI6InBybyJ9LCJleHAiOjQxMDI0NDQ4MDB9";
    Ok(serde_json::from_value(serde_json::json!({
        "auth_mode": "chatgpt", "OPENAI_API_KEY": format!("{nonce}-preserved"),
        "tokens": {"id_token": format!("e30.{CLAIMS}.{nonce}-id-{generation}"),
            "access_token": format!("e30.eyJleHAiOjQxMDI0NDQ4MDB9.{nonce}-access-{generation}"),
            "refresh_token": format!("{nonce}-refresh-{generation}"),
            "account_id": "covenant-synthetic-account"},
        "last_refresh": "2099-01-01T00:00:00Z"
    }))?)
}

pub(super) fn api_key_document(api_key: &str) -> Result<AuthDotJson> {
    Ok(serde_json::from_value(serde_json::json!({
        "auth_mode": "apikey", "OPENAI_API_KEY": api_key
    }))?)
}

fn prepare_root(root: &Path, initial: &AuthDotJson) -> Result<()> {
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    fs::write(root.join("auth/auth.json"), serde_json::to_vec(initial)?)?;
    Ok(())
}

pub(super) fn stored_bytes(root: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(root.join("auth/auth.json"))?)
}

pub(super) fn stored(root: &Path) -> Result<AuthDotJson> {
    serde_json::from_slice(&stored_bytes(root)?)
        .map_err(|_| anyhow::anyhow!("stored fixture document is invalid"))
}

pub(super) fn assert_exact(root: &Path, expected: &AuthDotJson) -> Result<()> {
    ensure!(
        stored_bytes(root)? == serde_json::to_vec_pretty(expected)?,
        "stored credential bytes differ from the exact winner"
    );
    Ok(())
}

pub(super) fn load(root: &Path) -> Option<AuthDotJson> {
    load_auth_dot_json(
        &root.join("mutable"),
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )
    .ok()
    .flatten()
}

pub(super) fn cache_token_or_key_matches(manager: &AuthManager, expected: &AuthDotJson) -> bool {
    let Some(auth) = manager.auth_cached() else {
        return false;
    };
    match (&expected.tokens, expected.openai_api_key.as_deref()) {
        (Some(tokens), _) => auth.get_token_data().ok().as_ref() == Some(tokens),
        (None, Some(api_key)) => auth.api_key() == Some(api_key),
        (None, None) => false,
    }
}

pub(super) async fn manager(root: &Path) -> Arc<AuthManager> {
    AuthManager::shared(
        root.join("mutable"),
        /*enable_codex_api_key_env*/ false,
        AuthCredentialsStoreMode::File,
        /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None,
        AuthKeyringBackendKind::Direct,
        transport_default_auth_route_config(),
    )
    .await
}

pub(super) fn read_fixture() -> Result<(Fixture, BufReader<std::io::Stdin>)> {
    let mut input = BufReader::new(std::io::stdin());
    let mut line = String::new();
    input
        .by_ref()
        .take((MAX_CHILD_INPUT_BYTES + 1) as u64)
        .read_line(&mut line)?;
    ensure!(
        line.ends_with('\n') && line.len() <= MAX_CHILD_INPUT_BYTES + 1,
        "invalid bounded child fixture line"
    );
    let fixture = serde_json::from_str(&line)
        .map_err(|_| anyhow::anyhow!("invalid bounded child fixture"))?;
    Ok((fixture, input))
}

pub(super) fn read_release(input: &mut impl BufRead) -> Result<()> {
    let mut release = [0; 3];
    input.read_exact(&mut release)?;
    ensure!(&release == b"go\n", "missing bounded child release");
    Ok(())
}

pub(super) fn emit(event: Event) -> Result<()> {
    println!("{PREFIX}{}", serde_json::to_string(&event)?);
    std::io::stdout().flush()?;
    Ok(())
}

pub(super) fn classify_refresh(result: &Result<(), RefreshTokenError>) -> Outcome {
    match result {
        Ok(()) => Outcome::Success,
        Err(_) => Outcome::Error,
    }
}

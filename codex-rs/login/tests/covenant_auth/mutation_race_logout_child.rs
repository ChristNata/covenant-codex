//! Child-only execution for Covenant logout state-boundary regressions.

use super::mutation_race_fixture as fixture;
use anyhow::Context;
use anyhow::Result;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::CodexAuth;
use codex_login::ExternalAuth;
use codex_login::ExternalAuthFuture;
use codex_login::ExternalAuthRefreshContext;
use codex_login::load_auth_dot_json;
use codex_login::logout_with_revoke;
use codex_login::save_auth;
use fixture::Event;
use fixture::LogoutOperation;
use fixture::Operation;
use fixture::Outcome;
use fixture::RevokeEntrypoint;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
struct StaticExternalAuth(CodexAuth);

impl ExternalAuth for StaticExternalAuth {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.0.clone()) })
    }

    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}

pub(super) fn run_child() -> Result<()> {
    let (child_fixture, mut input) = fixture::read_fixture()?;
    let persistent = match child_fixture.operation {
        Operation::Revoke { expected } => *expected,
        Operation::Refresh { .. }
        | Operation::Save { .. }
        | Operation::Login { .. }
        | Operation::AgentMetadata { .. }
        | Operation::FailureRecovery { .. }
        | Operation::CachedGenerationRecovery { .. } => {
            anyhow::bail!("non-revoke operation reached logout child")
        }
    };
    let operation = child_fixture
        .logout_operation
        .context("logout child operation missing")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    match operation {
        LogoutOperation::Malformed { entrypoint, decoy } => {
            run_malformed(child_fixture.root, entrypoint, *decoy, runtime, &mut input)
        }
        LogoutOperation::External { effective } => run_external(
            child_fixture.root,
            persistent,
            *effective,
            runtime,
            &mut input,
        ),
    }
}

fn run_malformed(
    root: PathBuf,
    entrypoint: RevokeEntrypoint,
    decoy: AuthDotJson,
    runtime: tokio::runtime::Runtime,
    input: &mut impl BufRead,
) -> Result<()> {
    fixture::emit(Event::Ready)?;
    fixture::read_release(input)?;
    fixture::emit(Event::Entered)?;
    let (result, manager_clean) = match entrypoint {
        RevokeEntrypoint::Public => (
            runtime.block_on(logout_with_revoke(
                &root.join("mutable"),
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
                &codex_login::test_support::transport_default_auth_route_config(),
            )),
            true,
        ),
        RevokeEntrypoint::Manager => {
            let manager = runtime.block_on(fixture::manager(&root));
            let result = runtime.block_on(manager.logout_with_revoke());
            let clean = !manager.has_external_auth()
                && manager.auth_cached().is_none()
                && runtime.block_on(manager.auth()).is_none();
            (result, clean)
        }
    };
    let selected = root.join("auth/auth.json");
    let mutable = fs::read(root.join("mutable/auth.json")).ok();
    fixture::emit(Event::Done(fixture::Report {
        outcome: outcome(result),
        whole_document: !selected.exists()
            && mutable.as_deref() == Some(serde_json::to_vec_pretty(&decoy)?.as_slice()),
        token_or_api_key_cache: manager_clean,
    }))
}

fn run_external(
    root: PathBuf,
    persistent: AuthDotJson,
    effective: AuthDotJson,
    runtime: tokio::runtime::Runtime,
    input: &mut impl BufRead,
) -> Result<()> {
    let manager = runtime.block_on(fixture::manager(&root));
    let initially_stored = fixture::stored_bytes(&root)? == serde_json::to_vec(&persistent)?;
    let initially_cached = fixture::cache_token_or_key_matches(&manager, &persistent);
    save_auth(
        &root.join("mutable"),
        &effective,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::Direct,
    )?;
    let ephemeral_loaded =
        load(&root, AuthCredentialsStoreMode::Ephemeral)? == Some(effective.clone());
    let effective_auth = runtime
        .block_on(CodexAuth::from_auth_storage(
            &root.join("mutable"),
            AuthCredentialsStoreMode::Ephemeral,
            /*chatgpt_base_url*/ None,
            AuthKeyringBackendKind::Direct,
            &codex_login::test_support::transport_default_auth_route_config(),
        ))?
        .context("effective external auth did not load")?;
    runtime
        .block_on(manager.set_external_auth(Arc::new(StaticExternalAuth(effective_auth))))
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let prepared = initially_stored
        && initially_cached
        && ephemeral_loaded
        && manager.has_external_auth()
        && fixture::cache_token_or_key_matches(&manager, &effective)
        && load(&root, AuthCredentialsStoreMode::File)? == Some(persistent);
    fixture::emit(Event::Ready)?;
    fixture::read_release(input)?;
    fixture::emit(Event::Entered)?;
    let result = runtime.block_on(manager.logout_with_revoke());
    let stores_clear = load(&root, AuthCredentialsStoreMode::File)?.is_none()
        && load(&root, AuthCredentialsStoreMode::Ephemeral)?.is_none();
    let manager_clear = !manager.has_external_auth()
        && manager.auth_cached().is_none()
        && runtime.block_on(manager.auth()).is_none();
    fixture::emit(Event::Done(fixture::Report {
        outcome: outcome(result),
        whole_document: prepared && stores_clear,
        token_or_api_key_cache: manager_clear,
    }))
}

fn load(root: &std::path::Path, mode: AuthCredentialsStoreMode) -> Result<Option<AuthDotJson>> {
    Ok(load_auth_dot_json(
        &root.join("mutable"),
        mode,
        AuthKeyringBackendKind::Direct,
    )?)
}

fn outcome(result: std::io::Result<bool>) -> Outcome {
    if matches!(result, Ok(true)) {
        Outcome::Success
    } else {
        Outcome::Error
    }
}

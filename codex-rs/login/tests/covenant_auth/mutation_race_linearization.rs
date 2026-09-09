//! Parent and child logic for refresh mutation linearization cases.

use super::mutation_race_fixture as fixture;
use super::mutation_race_http::Endpoint;
use super::mutation_race_http::EndpointPlan;
use super::mutation_race_process::Process;
use super::mutation_race_process::run_writer;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthKeyringBackendKind;
use codex_login::save_auth;
use fixture::Event;
use fixture::Operation;
use fixture::Outcome;

pub(super) fn refresh_vs_save(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let winner = fixture::document(&prepared.nonce, /*generation*/ 2)?;
    writer_race(
        test_name,
        &prepared,
        Operation::Save {
            document: winner.clone(),
            prior: Box::new(prepared.initial.clone()),
        },
        winner,
    )
}

fn writer_race(
    test_name: &str,
    prepared: &fixture::PreparedRoot,
    writer: Operation,
    winner: codex_login::AuthDotJson,
) -> Result<()> {
    let baseline = fixture::stored_bytes(&prepared.root)?;
    let refreshed = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let current = match &prepared.initial.tokens {
        Some(tokens) => tokens.refresh_token.clone(),
        None => anyhow::bail!("initial fixture is missing tokens"),
    };
    let authority = Endpoint::start(
        EndpointPlan::RefreshSuccess {
            current,
            next: refreshed.clone(),
        },
        /*hold_first*/ true,
    )?;
    let mut refresh_fixture = fixture::fixture(
        &prepared.root,
        Operation::Refresh {
            expected: refreshed,
        },
    );
    refresh_fixture.refresh_endpoint = authority.refresh_url();
    let mut refresh = Process::spawn(test_name, &refresh_fixture)?;
    refresh.release()?;
    refresh.wait_entered()?;
    authority.wait_accepted(/*ordinal*/ 1)?;
    run_writer(
        test_name,
        fixture::fixture(&prepared.root, writer.clone()),
        Outcome::Busy,
    )?;
    ensure!(
        fixture::stored_bytes(&prepared.root)? == baseline,
        "busy writer mutated bytes"
    );
    authority.release();
    ensure!(
        refresh.wait_done()?.passed(Outcome::Success),
        "refresh did not commit one complete generation"
    );
    refresh.finish()?;
    run_writer(
        test_name,
        fixture::fixture(&prepared.root, writer),
        Outcome::Success,
    )?;
    fixture::assert_exact(&prepared.root, &winner)
}

fn run_child() -> Result<()> {
    let (fixture, mut input) = fixture::read_fixture()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let manager = runtime.block_on(super::mutation_race_fixture::manager(&fixture.root));
    fixture::emit(Event::Ready)?;
    super::mutation_race_fixture::read_release(&mut input)?;
    fixture::emit(Event::Entered)?;
    let report = match fixture.operation {
        Operation::Refresh { mut expected } => {
            let result = runtime.block_on(manager.refresh_token());
            let observed = super::mutation_race_fixture::load(&fixture.root);
            if let Some(last_refresh) = observed.as_ref().and_then(|value| value.last_refresh) {
                expected.last_refresh = Some(last_refresh);
            }
            runtime.block_on(manager.reload());
            state_report(
                super::mutation_race_fixture::classify_refresh(&result),
                &fixture.root,
                &manager,
                &expected,
            )?
        }
        Operation::Save { document, prior } => {
            let result = save_auth(
                &fixture.root.join("mutable"),
                &document,
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::Direct,
            );
            io_report(result, &fixture.root, &manager, &runtime, &prior, &document)?
        }
    };
    fixture::emit(Event::Done(report))
}

fn io_report(
    result: std::io::Result<()>,
    root: &std::path::Path,
    manager: &codex_login::AuthManager,
    runtime: &tokio::runtime::Runtime,
    prior: &codex_login::AuthDotJson,
    expected: &codex_login::AuthDotJson,
) -> Result<fixture::Report> {
    let (outcome, expected) = match result {
        Ok(()) => {
            runtime.block_on(manager.reload());
            (Outcome::Success, expected)
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => (Outcome::Busy, prior),
        Err(_) => return Ok(failed_report()),
    };
    state_report(outcome, root, manager, expected)
}

fn state_report(
    outcome: Outcome,
    root: &std::path::Path,
    manager: &codex_login::AuthManager,
    expected: &codex_login::AuthDotJson,
) -> Result<fixture::Report> {
    let bytes = match outcome {
        Outcome::Success => serde_json::to_vec_pretty(expected)?,
        Outcome::Busy => serde_json::to_vec(expected)?,
        Outcome::Error => return Ok(failed_report()),
    };
    Ok(fixture::Report {
        outcome,
        whole_document: fixture::stored_bytes(root)? == bytes,
        token_or_api_key_cache: super::mutation_race_fixture::cache_tokens_match(manager, expected),
    })
}

fn failed_report() -> fixture::Report {
    fixture::Report {
        outcome: Outcome::Error,
        whole_document: false,
        token_or_api_key_cache: false,
    }
}

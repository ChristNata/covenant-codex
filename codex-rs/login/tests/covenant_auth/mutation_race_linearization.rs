//! Parent and child logic for refresh mutation linearization cases.

use super::mutation_race_fixture as fixture;
use super::mutation_race_http::Endpoint;
use super::mutation_race_http::EndpointPlan;
use super::mutation_race_process::Process;
use super::mutation_race_process::run_writer;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::protocol::SessionSource;
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

pub(super) fn refresh_vs_login(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let api_key = format!("{}-explicit-login", prepared.nonce);
    let winner = fixture::api_key_document(&api_key)?;
    writer_race(
        test_name,
        &prepared,
        Operation::Login {
            api_key,
            expected: winner.clone(),
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
    let (authority, refresh) = start_refresh(test_name, prepared, refreshed)?;
    run_writer(
        test_name,
        fixture::fixture(&prepared.root, writer.clone()),
        Outcome::Busy,
    )?;
    ensure!(
        fixture::stored_bytes(&prepared.root)? == baseline,
        "busy writer mutated bytes"
    );
    finish_refresh(&authority, refresh)?;
    run_writer(
        test_name,
        fixture::fixture(&prepared.root, writer),
        Outcome::Success,
    )?;
    fixture::assert_exact(&prepared.root, &winner)
}

pub(super) fn refresh_vs_agent_metadata(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let baseline = fixture::stored_bytes(&prepared.root)?;
    let refreshed = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let (authority, refresh) = start_refresh(test_name, &prepared, refreshed.clone())?;
    let agent_endpoint = Endpoint::start(EndpointPlan::AgentIdentity, /*hold_first*/ true)?;
    let mut metadata_fixture = fixture::fixture(
        &prepared.root,
        Operation::AgentMetadata {
            expected_base: refreshed.clone(),
            prior: Box::new(prepared.initial.clone()),
        },
    );
    metadata_fixture.agent_endpoint = agent_endpoint.base_url();
    let mut metadata = Process::spawn(test_name, &metadata_fixture)?;
    metadata.release()?;
    metadata.wait_entered()?;
    agent_endpoint.wait_accepted(/*ordinal*/ 1)?;
    agent_endpoint.release();
    agent_endpoint.wait_accepted(/*ordinal*/ 2)?;
    ensure!(
        metadata.wait_done()?.passed(Outcome::Busy),
        "metadata mutation did not yield to the refresh owner"
    );
    metadata.finish()?;
    ensure!(
        fixture::stored_bytes(&prepared.root)? == baseline,
        "busy metadata mutation changed stored bytes"
    );
    finish_refresh(&authority, refresh)?;
    let committed = fixture::stored(&prepared.root)?;
    let mut retry_fixture = fixture::fixture(
        &prepared.root,
        Operation::AgentMetadata {
            expected_base: committed.clone(),
            prior: Box::new(committed),
        },
    );
    let retry_endpoint = Endpoint::start(EndpointPlan::AgentIdentity, /*hold_first*/ false)?;
    retry_fixture.agent_endpoint = retry_endpoint.base_url();
    let mut retry = Process::spawn(test_name, &retry_fixture)?;
    retry.release()?;
    retry.wait_entered()?;
    retry_endpoint.wait_accepted(/*ordinal*/ 1)?;
    retry_endpoint.wait_accepted(/*ordinal*/ 2)?;
    ensure!(
        retry.wait_done()?.passed(Outcome::Success),
        "metadata retry did not merge into the winning generation"
    );
    retry.finish()?;
    let final_document = fixture::stored(&prepared.root)?;
    ensure!(
        final_document.tokens == refreshed.tokens && final_document.agent_identity.is_some(),
        "final metadata document lost the refreshed generation"
    );
    Ok(())
}

fn start_refresh(
    test_name: &str,
    prepared: &fixture::PreparedRoot,
    refreshed: codex_login::AuthDotJson,
) -> Result<(Endpoint, Process)> {
    let current = match &prepared.initial.tokens {
        Some(tokens) => tokens.refresh_token.clone(),
        None => anyhow::bail!("initial fixture is missing tokens"),
    };
    let authority = Endpoint::start(
        EndpointPlan::RefreshSuccess {
            current,
            next: Box::new(refreshed.clone()),
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
    Ok((authority, refresh))
}

fn finish_refresh(authority: &Endpoint, refresh: Process) -> Result<()> {
    authority.release();
    ensure!(
        refresh.wait_done()?.passed(Outcome::Success),
        "refresh did not commit one complete generation"
    );
    refresh.finish()
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
            fixture::state_report(
                super::mutation_race_fixture::classify_refresh(&result),
                &fixture.root,
                &manager,
                &expected,
            )?
        }
        Operation::Save { document, prior } => {
            fixture::save_report(&fixture.root, &manager, &runtime, &document, &prior)?
        }
        Operation::Login {
            api_key,
            expected,
            prior,
        } => fixture::login_report(
            &fixture.root,
            &manager,
            &runtime,
            &api_key,
            &expected,
            &prior,
        )?,
        Operation::AgentMetadata {
            mut expected_base,
            prior,
        } => {
            let result = runtime.block_on(manager.agent_identity_auth(
                codex_login::AgentIdentityAuthPolicy::ChatGptAuth,
                SessionSource::Cli,
            ));
            match result {
                Ok(Some(agent)) => {
                    expected_base.agent_identity = Some(
                        codex_login::auth::AgentIdentityStorage::Record(agent.record().clone()),
                    );
                    fixture::state_report(
                        Outcome::Success,
                        &fixture.root,
                        &manager,
                        &expected_base,
                    )?
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    fixture::state_report(Outcome::Busy, &fixture.root, &manager, &prior)?
                }
                Ok(None) | Err(_) => fixture::Report::failed(),
            }
        }
        Operation::FailureRecovery { .. } | Operation::CachedGenerationRecovery { .. } => {
            anyhow::bail!("recovery operation reached linearization child")
        }
    };
    fixture::emit(Event::Done(report))
}

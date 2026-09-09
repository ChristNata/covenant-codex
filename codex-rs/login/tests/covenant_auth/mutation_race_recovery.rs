//! W1.3a recovery cases for auth mutation races.
//! Cases 04-06 cover cached-generation and explicit-login recovery.

use super::mutation_race_fixture as fixture;
use super::mutation_race_http::Endpoint;
use super::mutation_race_http::EndpointPlan;
use super::mutation_race_http::EndpointSnapshot;
use super::mutation_race_process::Process;
use super::mutation_race_process::run_writer;
use anyhow::Result;
use anyhow::ensure;
use fixture::Event;
use fixture::FailureKind;
use fixture::Operation;
use fixture::Outcome;

pub(super) fn cached_permanent_failure(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let successor = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let endpoint = Endpoint::start(
        EndpointPlan::RefreshFailure {
            current: refresh_token(&prepared.initial)?,
            kind: FailureKind::Permanent,
        },
        /*hold_first*/ false,
    )?;
    let mut recovery_fixture = fixture::fixture(
        &prepared.root,
        Operation::CachedGenerationRecovery {
            prior: Box::new(prepared.initial.clone()),
            successor: Box::new(successor.clone()),
        },
    );
    recovery_fixture.refresh_endpoint = endpoint.refresh_url();
    let mut process = Process::spawn(test_name, &recovery_fixture)?;
    process.release()?;
    process.wait_entered()?;
    endpoint.wait_accepted(/*ordinal*/ 1)?;
    ensure!(
        process.wait_phase()?.passed(Outcome::Permanent),
        "permanent failure did not preserve generation N"
    );
    run_writer(
        test_name,
        fixture::fixture(
            &prepared.root,
            Operation::Save {
                document: successor.clone(),
                prior: Box::new(prepared.initial.clone()),
            },
        ),
        Outcome::Success,
    )?;
    process.release()?;
    ensure!(
        process.wait_done()?.passed(Outcome::Success),
        "cached failure masked externally committed generation N+1"
    );
    process.finish()?;
    assert_one_request(&endpoint)?;
    fixture::assert_exact(&prepared.root, &successor)
}

pub(super) fn failure_then_login(test_name: &str, kind: FailureKind) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let api_key = format!("{}-recovery-login", prepared.nonce);
    let recovery = fixture::api_key_document(&api_key)?;
    let endpoint = Endpoint::start(
        EndpointPlan::RefreshFailure {
            current: refresh_token(&prepared.initial)?,
            kind,
        },
        /*hold_first*/ false,
    )?;
    let mut recovery_fixture = fixture::fixture(
        &prepared.root,
        Operation::FailureRecovery {
            kind,
            prior: Box::new(prepared.initial.clone()),
            recovery: Box::new(recovery.clone()),
        },
    );
    recovery_fixture.refresh_endpoint = endpoint.refresh_url();
    let mut process = Process::spawn(test_name, &recovery_fixture)?;
    process.release()?;
    process.wait_entered()?;
    endpoint.wait_accepted(/*ordinal*/ 1)?;
    ensure!(
        process.wait_phase()?.passed(failure_outcome(kind)),
        "refresh failure did not preserve the prior complete document"
    );
    run_writer(
        test_name,
        fixture::fixture(
            &prepared.root,
            Operation::Login {
                api_key,
                expected: recovery.clone(),
                prior: Box::new(prepared.initial.clone()),
            },
        ),
        Outcome::Success,
    )?;
    process.release()?;
    ensure!(
        process.wait_done()?.passed(Outcome::Success),
        "explicit login did not recover the existing manager"
    );
    process.finish()?;
    assert_one_request(&endpoint)?;
    fixture::assert_exact(&prepared.root, &recovery)
}

pub(super) fn revoke_with_newer_login(test_name: &str, succeeds: bool) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let api_key = format!("{}-newer-login", prepared.nonce);
    let winner = fixture::api_key_document(&api_key)?;
    let endpoint = Endpoint::start(
        EndpointPlan::Revoke {
            token: refresh_token(&prepared.initial)?,
            succeeds,
        },
        /*hold_first*/ true,
    )?;
    let mut revoke_fixture = fixture::fixture(
        &prepared.root,
        Operation::Revoke {
            expected: Box::new(winner.clone()),
        },
    );
    revoke_fixture.revoke_endpoint = endpoint.revoke_url();
    let mut revoke = Process::spawn(test_name, &revoke_fixture)?;
    revoke.release()?;
    revoke.wait_entered()?;
    endpoint.wait_accepted(/*ordinal*/ 1)?;
    run_writer(
        test_name,
        fixture::fixture(
            &prepared.root,
            Operation::Login {
                api_key,
                expected: winner.clone(),
                prior: Box::new(prepared.initial.clone()),
            },
        ),
        Outcome::Success,
    )?;
    endpoint.release();
    ensure!(
        revoke.wait_done()?.passed(Outcome::Success),
        "revoke operation did not preserve the newer login"
    );
    revoke.finish()?;
    assert_one_request(&endpoint)?;
    fixture::assert_exact(&prepared.root, &winner)
}

fn run_child() -> Result<()> {
    let (fixture, mut input) = fixture::read_fixture()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let manager = runtime.block_on(super::mutation_race_fixture::manager(&fixture.root));
    super::mutation_race_fixture::emit(Event::Ready)?;
    super::mutation_race_fixture::read_release(&mut input)?;
    super::mutation_race_fixture::emit(Event::Entered)?;
    match fixture.operation {
        Operation::FailureRecovery {
            kind,
            prior,
            recovery,
        } => {
            let result = runtime.block_on(manager.refresh_token());
            let outcome = super::mutation_race_fixture::classify_refresh(&result);
            super::mutation_race_fixture::emit(Event::Phase(
                super::mutation_race_fixture::state_report(
                    outcome,
                    &fixture.root,
                    &manager,
                    &prior,
                )?,
            ))?;
            ensure!(outcome == failure_outcome(kind), "unexpected refresh class");
            super::mutation_race_fixture::read_release(&mut input)?;
            runtime.block_on(manager.reload());
            let recovered = runtime.block_on(manager.refresh_token());
            super::mutation_race_fixture::emit(Event::Done(
                super::mutation_race_fixture::state_report(
                    super::mutation_race_fixture::classify_refresh(&recovered),
                    &fixture.root,
                    &manager,
                    &recovery,
                )?,
            ))
        }
        Operation::CachedGenerationRecovery { prior, successor } => {
            let first = runtime.block_on(manager.refresh_token());
            let outcome = super::mutation_race_fixture::classify_refresh(&first);
            super::mutation_race_fixture::emit(Event::Phase(
                super::mutation_race_fixture::state_report(
                    outcome,
                    &fixture.root,
                    &manager,
                    &prior,
                )?,
            ))?;
            ensure!(
                outcome == Outcome::Permanent,
                "unexpected first refresh class"
            );
            super::mutation_race_fixture::read_release(&mut input)?;
            let second = runtime.block_on(manager.refresh_token());
            super::mutation_race_fixture::emit(Event::Done(
                super::mutation_race_fixture::state_report(
                    super::mutation_race_fixture::classify_refresh(&second),
                    &fixture.root,
                    &manager,
                    &successor,
                )?,
            ))
        }
        Operation::Revoke { expected } => {
            let result = runtime.block_on(manager.logout_with_revoke());
            let report = match result {
                Ok(false) => {
                    runtime.block_on(manager.reload());
                    super::mutation_race_fixture::state_report(
                        Outcome::Success,
                        &fixture.root,
                        &manager,
                        &expected,
                    )?
                }
                Ok(true) | Err(_) => super::mutation_race_fixture::Report::failed(),
            };
            super::mutation_race_fixture::emit(Event::Done(report))
        }
        Operation::Save { document, prior } => super::mutation_race_fixture::emit(Event::Done(
            super::mutation_race_fixture::save_report(
                &fixture.root,
                &manager,
                &runtime,
                &document,
                &prior,
            )?,
        )),
        Operation::Login {
            api_key,
            expected,
            prior,
        } => super::mutation_race_fixture::emit(Event::Done(
            super::mutation_race_fixture::login_report(
                &fixture.root,
                &manager,
                &runtime,
                &api_key,
                &expected,
                &prior,
            )?,
        )),
        Operation::Refresh { .. } | Operation::AgentMetadata { .. } => {
            anyhow::bail!("linearization operation reached recovery child")
        }
    }
}

fn refresh_token(document: &codex_login::AuthDotJson) -> Result<String> {
    match &document.tokens {
        Some(tokens) => Ok(tokens.refresh_token.clone()),
        None => anyhow::bail!("fixture document is missing a refresh token"),
    }
}

fn failure_outcome(kind: FailureKind) -> Outcome {
    match kind {
        FailureKind::Transient => Outcome::Transient,
        FailureKind::Permanent => Outcome::Permanent,
    }
}

fn assert_one_request(endpoint: &Endpoint) -> Result<()> {
    ensure!(
        endpoint.wait_complete(/*ordinal*/ 1)?
            == EndpointSnapshot {
                requests: 1,
                acknowledged: 1,
                malformed: false,
            },
        "authority observed an unexpected request sequence"
    );
    Ok(())
}

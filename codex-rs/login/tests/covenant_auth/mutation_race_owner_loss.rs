//! Owner-loss boundaries for native auth refresh transactions.

use super::mutation_race_fixture as fixture;
use super::mutation_race_http::Endpoint;
use super::mutation_race_http::EndpointPlan;
use super::mutation_race_http::EndpointSnapshot;
use super::mutation_race_process::Process;
use super::mutation_race_process::run_writer;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use codex_login::AuthManager;
use fixture::Event;
use fixture::Operation;
use fixture::Outcome;
use fixture::PostCompletion;
use std::future::Future;
use std::task::Poll;

pub(super) fn before_authority(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let baseline = fixture::stored_bytes(&prepared.root)?;
    let lock = fixture::hold_auth_lock(&prepared.root)?;
    let next = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let endpoint = Endpoint::start(
        EndpointPlan::RefreshSuccess {
            current: refresh_token(&prepared.initial)?,
            next: Box::new(next.clone()),
        },
        /*hold_first*/ true,
    )?;
    let mut refresh_fixture =
        fixture::fixture(&prepared.root, Operation::Refresh { expected: next });
    refresh_fixture.refresh_endpoint = endpoint.refresh_url();
    let mut refresh = Process::spawn(test_name, &refresh_fixture)?;
    refresh.release()?;
    refresh.wait_entered()?;
    refresh.kill()?;
    ensure!(
        endpoint.finish()?
            == EndpointSnapshot {
                requests: 0,
                acknowledged: 0,
                malformed: false,
            },
        "authority was reached before transaction ownership"
    );
    ensure!(
        fixture::stored_bytes(&prepared.root)? == baseline,
        "owner loss before authority changed credential bytes"
    );
    drop(lock);
    recover_exact(test_name, &prepared.root, &prepared.nonce)
}

pub(super) fn after_acceptance(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let baseline = fixture::stored_bytes(&prepared.root)?;
    let next = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let endpoint = Endpoint::start(
        EndpointPlan::RefreshSuccess {
            current: refresh_token(&prepared.initial)?,
            next: Box::new(next.clone()),
        },
        /*hold_first*/ true,
    )?;
    let mut refresh_fixture =
        fixture::fixture(&prepared.root, Operation::Refresh { expected: next });
    refresh_fixture.refresh_endpoint = endpoint.refresh_url();
    let mut refresh = Process::spawn(test_name, &refresh_fixture)?;
    refresh.release()?;
    refresh.wait_entered()?;
    endpoint.wait_accepted(/*ordinal*/ 1)?;
    let accepted = EndpointSnapshot {
        requests: 1,
        acknowledged: 0,
        malformed: false,
    };
    ensure!(
        endpoint.wait_complete(/*ordinal*/ 0)? == accepted,
        "authority response was not held before owner loss"
    );
    refresh.kill()?;
    ensure!(
        endpoint.wait_complete(/*ordinal*/ 0)? == accepted,
        "authority state changed before held response release"
    );
    endpoint.release();
    let _ = endpoint.finish()?;
    ensure!(
        fixture::stored_bytes(&prepared.root)? == baseline,
        "owner loss after acceptance changed credential bytes"
    );
    recover_exact(test_name, &prepared.root, &prepared.nonce)
}

pub(super) fn after_replacement(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let next = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let endpoint = Endpoint::start(
        EndpointPlan::RefreshSuccess {
            current: refresh_token(&prepared.initial)?,
            next: Box::new(next.clone()),
        },
        /*hold_first*/ false,
    )?;
    let mut refresh_fixture = fixture::fixture(
        &prepared.root,
        Operation::Refresh {
            expected: next.clone(),
        },
    );
    refresh_fixture.refresh_endpoint = endpoint.refresh_url();
    refresh_fixture.post_completion = PostCompletion::Park;
    let mut refresh = Process::spawn(test_name, &refresh_fixture)?;
    refresh.release()?;
    refresh.wait_entered()?;
    endpoint.wait_accepted(/*ordinal*/ 1)?;
    ensure!(
        refresh.wait_done()?.passed(Outcome::Success),
        "refresh owner did not report an exact complete replacement"
    );
    ensure!(
        endpoint.finish()?
            == EndpointSnapshot {
                requests: 1,
                acknowledged: 1,
                malformed: false,
            },
        "authority did not complete exactly one refresh response"
    );
    let winner_bytes = fixture::stored_bytes(&prepared.root)?;
    let winner = fixture::stored(&prepared.root)?;
    ensure!(
        winner.tokens == next.tokens,
        "stored winner does not contain generation N+1 tokens"
    );
    refresh.kill()?;
    ensure!(
        fixture::stored_bytes(&prepared.root)? == winner_bytes,
        "owner loss after replacement changed winner bytes"
    );
    recover_exact(test_name, &prepared.root, &prepared.nonce)
}

fn recover_exact(test_name: &str, root: &std::path::Path, nonce: &str) -> Result<()> {
    let prior = Box::new(fixture::stored(root)?);
    let api_key = format!("{nonce}-owner-loss-recovery");
    let expected = fixture::api_key_document(&api_key)?;
    run_writer(
        test_name,
        fixture::fixture(
            root,
            Operation::Login {
                api_key,
                expected: expected.clone(),
                prior,
            },
        ),
        Outcome::Success,
    )?;
    fixture::assert_exact(root, &expected)
}

fn run_child() -> Result<()> {
    let (fixture, mut input) = fixture::read_fixture()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let manager = runtime.block_on(super::mutation_race_fixture::manager(&fixture.root));
    super::mutation_race_fixture::emit(Event::Ready)?;
    super::mutation_race_fixture::read_release(&mut input)?;
    match fixture.operation {
        Operation::Refresh { mut expected } => {
            let mut refresh = std::pin::pin!(manager.refresh_token());
            {
                let _runtime_guard = runtime.enter();
                let mut context = std::task::Context::from_waker(std::task::Waker::noop());
                ensure!(
                    matches!(refresh.as_mut().poll(&mut context), Poll::Pending),
                    "refresh completed during its positive entry poll"
                );
            }
            super::mutation_race_fixture::emit(Event::Entered)?;
            let result = runtime.block_on(refresh);
            let report = refresh_report(&fixture.root, &manager, &runtime, &mut expected, &result)?;
            super::mutation_race_fixture::emit(Event::Done(report))?;
            match fixture.post_completion {
                PostCompletion::Exit => Ok(()),
                PostCompletion::Park => super::mutation_race_fixture::read_release(&mut input),
            }
        }
        Operation::Login {
            api_key,
            expected,
            prior,
        } => {
            super::mutation_race_fixture::emit(Event::Entered)?;
            super::mutation_race_fixture::emit(Event::Done(
                super::mutation_race_fixture::login_report(
                    &fixture.root,
                    &manager,
                    &runtime,
                    &api_key,
                    &expected,
                    &prior,
                )?,
            ))
        }
        Operation::Save { .. }
        | Operation::AgentMetadata { .. }
        | Operation::FailureRecovery { .. }
        | Operation::CachedGenerationRecovery { .. }
        | Operation::Revoke { .. } => {
            anyhow::bail!("non-owner-loss operation reached owner-loss child")
        }
    }
}

fn refresh_report(
    root: &std::path::Path,
    manager: &AuthManager,
    runtime: &tokio::runtime::Runtime,
    expected: &mut AuthDotJson,
    result: &std::result::Result<(), codex_login::RefreshTokenError>,
) -> Result<fixture::Report> {
    expected.last_refresh = fixture::stored(root)?.last_refresh;
    runtime.block_on(manager.reload());
    fixture::state_report(fixture::classify_refresh(result), root, manager, expected)
}

fn refresh_token(document: &AuthDotJson) -> Result<String> {
    match &document.tokens {
        Some(tokens) => Ok(tokens.refresh_token.clone()),
        None => anyhow::bail!("fixture document is missing a refresh token"),
    }
}

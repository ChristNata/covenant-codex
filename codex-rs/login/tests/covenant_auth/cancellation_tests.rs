//! Caller-task cancellation with the native runtime still alive for settlement.

use super::cancellation_support::CancelledCaller;
use super::rotation_authority::Authority;
use super::rotation_support as support;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use std::fs;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use support::Fixture;
use support::Method;
use support::Processes;

const PROBE_ENTRY: &str =
    "rotation_tests::covenant_auth_rotation_two_guarded_callers_adopt_one_generation";

enum CancellationPoint {
    Waiting,
    Consumed,
}

#[test]
fn covenant_auth_cancellation_waiter_does_not_refresh() -> Result<()> {
    run("waiter_does_not_refresh", CancellationPoint::Waiting)
}

#[test]
fn covenant_auth_cancellation_owner_commits_after_caller_abort() -> Result<()> {
    run(
        "owner_commits_after_caller_abort",
        CancellationPoint::Consumed,
    )
}

fn run(suffix: &str, point: CancellationPoint) -> Result<()> {
    let test_name = format!("cancellation_tests::covenant_auth_cancellation_{suffix}");
    if support::is_child(&test_name) {
        return super::cancellation_support::run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    let nonce = format!("covenant-synthetic-{:032x}", rand::random::<u128>());
    let authority = Authority::start(nonce.clone())?;
    let initial = support::document(&nonce, /*generation*/ 0)?;
    let mut expected = initial.clone();
    expected.tokens = support::document(&nonce, /*generation*/ 1)?.tokens;
    let auth_file = root.join("auth/auth.json");
    fs::write(&auth_file, serde_json::to_vec(&initial)?)?;
    let fixture = Fixture {
        root: root.clone(),
        initial,
        expected: expected.clone(),
        method: match point {
            CancellationPoint::Waiting => Method::Authority,
            CancellationPoint::Consumed => Method::Guarded,
        },
        started_at: support::unix_seconds()?,
    };
    authority.arm(/*generation*/ 1);
    let mut owner = match point {
        CancellationPoint::Waiting => {
            let mut owner =
                Processes::spawn(PROBE_ENTRY, &fixture, &[Method::Guarded], &authority.url())?;
            owner.ready_and_release()?;
            owner.wait_entered()?;
            authority.wait_accepted(/*generation*/ 1)?;
            Some(owner)
        }
        CancellationPoint::Consumed => None,
    };
    let mut caller = CancelledCaller::spawn(&test_name, &fixture, &authority.url())?;
    caller.start()?;
    if matches!(point, CancellationPoint::Consumed) {
        authority.wait_accepted(/*generation*/ 1)?;
    }
    caller.cancel()?;
    caller.ping()?;
    let before_release = authority.snapshot();
    ensure!(
        before_release.requests == 1
            && before_release.generation == 1
            && before_release.acknowledged == 0
            && before_release.reuses == 0
            && !before_release.malformed,
        "cancellation was not established before the consumed response release"
    );
    authority.release(/*generation*/ 1);
    let owner_passed = if let Some(owner) = owner.as_mut() {
        owner.finish()?.iter().all(support::Report::passed)
    } else {
        true
    };
    // Positive observation of the complete committed document; deadline expiry never passes.
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 10);
    let document_latest = loop {
        let loaded = fs::read(&auth_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AuthDotJson>(&bytes).ok());
        if loaded.as_ref().is_some_and(|document| {
            support::whole_document_matches(document, &expected, fixture.started_at)
        }) {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(/*millis*/ 10));
    };
    caller.ping()?;
    let observed: AuthDotJson = serde_json::from_slice(&fs::read(&auth_file)?)
        .map_err(|_| anyhow::anyhow!("stored cancellation fixture is invalid"))?;
    // Starting-generation assertion remains truthful even on the stale baseline failure path.
    let mut probe_expected = expected;
    probe_expected.last_refresh = observed.last_refresh;
    let probe = Fixture {
        initial: observed,
        expected: probe_expected,
        method: Method::Probe,
        ..fixture
    };
    let state_before_probe = authority.snapshot();
    let mut fresh = Processes::spawn(PROBE_ENTRY, &probe, &[Method::Probe], &authority.url())?;
    fresh.ready_and_release()?;
    let probe_latest = fresh.finish()?.iter().all(support::Report::passed);
    caller.ping()?;
    let state = authority.snapshot();
    let probe_without_http = state == state_before_probe;
    let mutable_home_clean = !root.join("mutable/auth.json").exists();
    // Fixed booleans/counts only: never print refresh errors, credential values or raw streams.
    eprintln!(
        "cancellation {suffix}: cancelled=true, runtime_alive=true, owner_passed={owner_passed}, \
        document_latest={document_latest}, probe_latest={probe_latest}, \
        probe_without_http={probe_without_http}, mutable_home_clean={mutable_home_clean}, authority={state:?}"
    );
    caller.finish()?;
    ensure!(
        owner_passed
            && document_latest
            && probe_latest
            && probe_without_http
            && mutable_home_clean
            && state.requests == 1
            && state.reuses == 0
            && state.generation == 1
            && state.acknowledged == 1
            && !state.malformed,
        "cancelled native refresh did not settle with one committed generation"
    );
    Ok(())
}

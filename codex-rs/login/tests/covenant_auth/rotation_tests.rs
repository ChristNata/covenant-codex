//! Public native refresh races against a local single-use rotating authority.

use super::rotation_authority::Authority;
use super::rotation_support as support;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use std::fs;
use support::Fixture;
use support::Method;
use support::Processes;

#[test]
fn covenant_auth_rotation_two_guarded_callers_adopt_one_generation() -> Result<()> {
    run(
        "two_guarded_callers_adopt_one_generation",
        &[Method::Guarded; 2],
        /*rounds*/ 1,
    )
}

#[test]
fn covenant_auth_rotation_two_authority_callers_adopt_one_generation() -> Result<()> {
    run(
        "two_authority_callers_adopt_one_generation",
        &[Method::Authority; 2],
        /*rounds*/ 1,
    )
}

#[test]
fn covenant_auth_rotation_five_mixed_callers_continue_from_committed_generation() -> Result<()> {
    run(
        "five_mixed_callers_continue_from_committed_generation",
        &[
            Method::Guarded,
            Method::Authority,
            Method::Guarded,
            Method::Authority,
            Method::Guarded,
        ],
        /*rounds*/ 2,
    )
}

fn run(suffix: &str, methods: &[Method], rounds: u32) -> Result<()> {
    let test_name = format!("rotation_tests::covenant_auth_rotation_{suffix}");
    if support::is_child(&test_name) {
        return support::run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    let nonce = format!("covenant-synthetic-{:032x}", rand::random::<u128>());
    let authority = Authority::start(nonce.clone())?;
    let mut current = support::document(&nonce, /*generation*/ 0)?;
    fs::write(root.join("auth/auth.json"), serde_json::to_vec(&current)?)?;
    for round in 0..rounds {
        authority.arm(round + 1);
        let started_at = support::unix_seconds()?;
        let mut expected = current.clone();
        expected.tokens = support::document(&nonce, round + 1)?.tokens;
        let fixture = Fixture {
            root: root.clone(),
            initial: current.clone(),
            expected: expected.clone(),
            method: Method::Guarded,
            started_at,
        };
        let mut children = Processes::spawn(&test_name, &fixture, methods, &authority.url())?;
        children.ready_and_release()?;
        children.wait_entered()?;
        authority.wait_accepted(round + 1)?;
        authority.release(round + 1);
        let reports = children.finish()?;
        authority.wait_acknowledged(round + 1)?;
        let state = authority.snapshot();
        // Only counts and booleans are formatted; raw child streams and tokens are discarded.
        eprintln!(
            "rotation round {}: authority={state:?}, callers={reports:?}",
            round + 1
        );
        ensure!(
            reports.iter().all(support::Report::passed),
            "a public refresh caller failed or retained stale credentials"
        );
        ensure!(
            state.requests == round + 1
                && state.reuses == 0
                && state.generation == round + 1
                && state.acknowledged == round + 1
                && !state.malformed,
            "authority observed duplicate refreshes, token reuse, or malformed requests"
        );
        let observed: AuthDotJson = serde_json::from_slice(&fs::read(root.join("auth/auth.json"))?)
            .map_err(|_| anyhow::anyhow!("stored fixture document is invalid"))?;
        ensure!(
            support::whole_document_matches(&observed, &expected, started_at),
            "final complete credential document differs from the winning generation"
        );
        ensure!(
            !root.join("mutable/auth.json").exists(),
            "refresh wrote mutable-home credentials"
        );
        current = observed;
        let probe = Fixture {
            initial: current.clone(),
            expected: current.clone(),
            method: Method::Probe,
            ..fixture
        };
        let mut fresh = Processes::spawn(&test_name, &probe, &[Method::Probe], &authority.url())?;
        fresh.ready_and_release()?;
        ensure!(
            fresh.finish()?.iter().all(support::Report::passed),
            "fresh public auth probe failed"
        );
        ensure!(
            authority.snapshot() == state,
            "fresh auth probe unexpectedly contacted refresh authority"
        );
    }
    Ok(())
}

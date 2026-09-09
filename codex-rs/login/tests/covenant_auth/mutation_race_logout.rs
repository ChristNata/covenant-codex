//! Logout state-boundary regressions for Covenant-managed auth.

use super::mutation_race_fixture as fixture;
use super::mutation_race_http::Endpoint;
use super::mutation_race_http::EndpointPlan;
use super::mutation_race_http::EndpointSnapshot;
use super::mutation_race_process::Process;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use fixture::LogoutOperation;
use fixture::Operation;
use fixture::Outcome;
use fixture::RevokeEntrypoint;
use std::fs;

struct MalformedObservation {
    prepared: fixture::PreparedRoot,
    decoy_bytes: Vec<u8>,
    sentinel: String,
    report: fixture::Report,
    endpoint: EndpointSnapshot,
}

pub(super) fn remove_malformed_selected_file(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return super::mutation_race_logout_child::run_child();
    }
    let public = observe_malformed(test_name, RevokeEntrypoint::Public)?;
    let manager = observe_malformed(test_name, RevokeEntrypoint::Manager)?;

    assert_malformed_removed(public)?;
    assert_malformed_removed(manager)
}

pub(super) fn revoke_effective_external(test_name: &str) -> Result<()> {
    if fixture::is_child(test_name) {
        return super::mutation_race_logout_child::run_child();
    }
    let prepared = fixture::PreparedRoot::new()?;
    let effective = fixture::document(&prepared.nonce, /*generation*/ 1)?;
    let endpoint = Endpoint::start(
        EndpointPlan::Revoke {
            token: refresh_token(&effective)?.to_string(),
            succeeds: true,
        },
        /*hold_first*/ false,
    )?;
    let mut child_fixture = fixture::fixture(
        &prepared.root,
        Operation::Revoke {
            expected: Box::new(prepared.initial.clone()),
        },
    );
    child_fixture.logout_operation = Some(LogoutOperation::External {
        effective: Box::new(effective),
    });
    child_fixture.revoke_endpoint = endpoint.revoke_url();
    let mut process = Process::spawn(test_name, &child_fixture)?;
    process.release()?;
    process.wait_entered()?;
    let report = process.wait_done()?;
    process.finish()?;
    let snapshot = endpoint.finish()?;

    ensure!(
        report.passed(Outcome::Success),
        "manager did not revoke effective external auth and clear both stores"
    );
    ensure!(
        snapshot
            == EndpointSnapshot {
                requests: 1,
                acknowledged: 1,
                malformed: false,
            },
        "authority did not observe exactly the effective external refresh token"
    );
    Ok(())
}

fn observe_malformed(
    test_name: &str,
    entrypoint: RevokeEntrypoint,
) -> Result<MalformedObservation> {
    let prepared = fixture::PreparedRoot::new()?;
    let sentinel = format!("{}-malformed-refresh", prepared.nonce);
    fs::write(
        prepared.root.join("auth/auth.json"),
        serde_json::to_vec(&serde_json::json!({
            "auth_mode": "covenant-invalid",
            "OPENAI_API_KEY": sentinel,
        }))?,
    )?;
    let decoy = fixture::api_key_document(&format!("{}-mutable-decoy", prepared.nonce))?;
    let decoy_bytes = serde_json::to_vec_pretty(&decoy)?;
    fs::write(prepared.root.join("mutable/auth.json"), &decoy_bytes)?;
    let endpoint = Endpoint::start(
        EndpointPlan::Revoke {
            token: sentinel.clone(),
            succeeds: true,
        },
        /*hold_first*/ false,
    )?;
    let mut child_fixture = fixture::fixture(
        &prepared.root,
        Operation::Revoke {
            expected: Box::new(prepared.initial.clone()),
        },
    );
    child_fixture.logout_operation = Some(LogoutOperation::Malformed {
        entrypoint,
        decoy: Box::new(decoy),
    });
    child_fixture.revoke_endpoint = endpoint.revoke_url();
    let mut process = Process::spawn(test_name, &child_fixture)?;
    process.release()?;
    process.wait_entered()?;
    let report = process.wait_done()?;
    process.finish()?;
    let endpoint = endpoint.finish()?;
    Ok(MalformedObservation {
        prepared,
        decoy_bytes,
        sentinel,
        report,
        endpoint,
    })
}

fn assert_malformed_removed(observation: MalformedObservation) -> Result<()> {
    let selected = observation.prepared.root.join("auth/auth.json");
    let mutable = observation.prepared.root.join("mutable/auth.json");
    let selected_bytes = fs::read(&selected).ok();
    let mutable_bytes = fs::read(&mutable)?;
    let report_bytes = serde_json::to_vec(&observation.report)?;
    let retained_sentinel = selected_bytes
        .iter()
        .chain([&mutable_bytes, &report_bytes])
        .any(|bytes| contains(bytes, observation.sentinel.as_bytes()));
    ensure!(
        observation.report.passed(Outcome::Success),
        "malformed selected File auth was not removed through the public entrypoint"
    );
    ensure!(
        observation.endpoint == EndpointSnapshot::default(),
        "malformed auth caused an authority request"
    );
    ensure!(!selected.exists(), "malformed selected auth remained");
    ensure!(
        mutable_bytes == observation.decoy_bytes,
        "mutable-home decoy changed"
    );
    ensure!(
        !retained_sentinel,
        "malformed sentinel remained in retained state"
    );
    Ok(())
}

fn refresh_token(document: &AuthDotJson) -> Result<&str> {
    document
        .tokens
        .as_ref()
        .map(|tokens| tokens.refresh_token.as_str())
        .context("fixture document is missing a refresh token")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

//! Direct entrypoints for native-auth mutation linearization tests.

use super::mutation_race_fixture::FailureKind;
use super::mutation_race_linearization as linearization;
use super::mutation_race_recovery as recovery;
use anyhow::Result;

#[test]
fn covenant_auth_refresh_vs_save_linearizes_complete_documents() -> Result<()> {
    linearization::refresh_vs_save(
        "mutation_race_tests::covenant_auth_refresh_vs_save_linearizes_complete_documents",
    )
}

#[test]
fn covenant_auth_refresh_vs_login_linearizes_complete_documents() -> Result<()> {
    linearization::refresh_vs_login(
        "mutation_race_tests::covenant_auth_refresh_vs_login_linearizes_complete_documents",
    )
}

#[test]
fn covenant_auth_refresh_vs_agent_metadata_linearizes_complete_documents() -> Result<()> {
    linearization::refresh_vs_agent_metadata(
        "mutation_race_tests::covenant_auth_refresh_vs_agent_metadata_linearizes_complete_documents",
    )
}

#[test]
fn covenant_auth_cached_permanent_failure_recovers_after_n_plus_one() -> Result<()> {
    recovery::cached_permanent_failure(
        "mutation_race_tests::covenant_auth_cached_permanent_failure_recovers_after_n_plus_one",
    )
}

#[test]
fn covenant_auth_transient_failure_preserves_prior_and_explicit_login_recovers() -> Result<()> {
    recovery::failure_then_login(
        "mutation_race_tests::covenant_auth_transient_failure_preserves_prior_and_explicit_login_recovers",
        FailureKind::Transient,
    )
}

#[test]
fn covenant_auth_permanent_failure_preserves_prior_and_explicit_login_recovers() -> Result<()> {
    recovery::failure_then_login(
        "mutation_race_tests::covenant_auth_permanent_failure_preserves_prior_and_explicit_login_recovers",
        FailureKind::Permanent,
    )
}

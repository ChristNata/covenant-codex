//! Direct entrypoints for native-auth mutation linearization tests.

use super::mutation_race_fixture::FailureKind;
use super::mutation_race_linearization as linearization;
use super::mutation_race_logout as logout;
use super::mutation_race_owner_loss as owner_loss;
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

#[test]
fn covenant_auth_revoke_success_preserves_newer_login() -> Result<()> {
    recovery::revoke_with_newer_login(
        "mutation_race_tests::covenant_auth_revoke_success_preserves_newer_login",
        /*succeeds*/ true,
    )
}

#[test]
fn covenant_auth_revoke_failure_preserves_newer_login() -> Result<()> {
    recovery::revoke_with_newer_login(
        "mutation_race_tests::covenant_auth_revoke_failure_preserves_newer_login",
        /*succeeds*/ false,
    )
}

#[test]
fn covenant_auth_public_and_manager_revoke_remove_malformed_selected_file() -> Result<()> {
    logout::remove_malformed_selected_file(
        "mutation_race_tests::covenant_auth_public_and_manager_revoke_remove_malformed_selected_file",
    )
}

#[test]
fn covenant_auth_manager_revoke_uses_effective_external_and_removes_managed_fallback() -> Result<()>
{
    logout::revoke_effective_external(
        "mutation_race_tests::covenant_auth_manager_revoke_uses_effective_external_and_removes_managed_fallback",
    )
}

#[test]
fn covenant_auth_owner_loss_before_authority_preserves_prior_and_recovers() -> Result<()> {
    owner_loss::before_authority(
        "mutation_race_tests::covenant_auth_owner_loss_before_authority_preserves_prior_and_recovers",
    )
}

#[test]
fn covenant_auth_owner_loss_after_acceptance_preserves_prior_and_recovers() -> Result<()> {
    owner_loss::after_acceptance(
        "mutation_race_tests::covenant_auth_owner_loss_after_acceptance_preserves_prior_and_recovers",
    )
}

#[test]
fn covenant_auth_owner_loss_after_replacement_preserves_winner_and_recovers() -> Result<()> {
    owner_loss::after_replacement(
        "mutation_race_tests::covenant_auth_owner_loss_after_replacement_preserves_winner_and_recovers",
    )
}

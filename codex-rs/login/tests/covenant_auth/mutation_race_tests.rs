//! Direct entrypoints for native-auth mutation linearization tests.

use super::mutation_race_linearization as linearization;
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

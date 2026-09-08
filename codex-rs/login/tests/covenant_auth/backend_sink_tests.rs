//! Wave-1 public File refresh and logout acceptance.

#![cfg(windows)]

use super::backend_sink_support as support;
use anyhow::Result;
use support::Scenario;

#[test]
fn covenant_auth_file_refresh_regression() -> Result<()> {
    support::run(
        "backend_sink_tests::covenant_auth_file_refresh_regression",
        Scenario::FileRefresh,
    )
}

#[test]
fn covenant_auth_ephemeral_direct_logout_is_process_local() -> Result<()> {
    support::run(
        "backend_sink_tests::covenant_auth_ephemeral_direct_logout_is_process_local",
        Scenario::EphemeralDirectLogout,
    )
}

#[test]
fn covenant_auth_persistent_manager_logout_clears_managed_and_ephemeral() -> Result<()> {
    support::run(
        "backend_sink_tests::covenant_auth_persistent_manager_logout_clears_managed_and_ephemeral",
        Scenario::PersistentManagerLogout,
    )
}

#[test]
fn covenant_auth_ephemeral_manager_logout_does_not_guess_persistent_stores() -> Result<()> {
    support::run(
        "backend_sink_tests::covenant_auth_ephemeral_manager_logout_does_not_guess_persistent_stores",
        Scenario::EphemeralManagerLogout,
    )
}

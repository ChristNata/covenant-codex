//! Wave-1 public File refresh and logout acceptance.

#![cfg(windows)]

use super::backend_sink_support as support;
use anyhow::Result;
use support::BackendCase;
use support::Scenario;

#[test]
fn covenant_auth_file_refresh_regression() -> Result<()> {
    support::run(
        "backend_sink_tests::covenant_auth_file_refresh_regression",
        Scenario::Refresh(BackendCase::File),
    )
}

#[test]
fn covenant_auth_direct_and_secrets_refresh_selected_keyring() -> Result<()> {
    let test_name = "backend_sink_tests::covenant_auth_direct_and_secrets_refresh_selected_keyring";
    support::run_all(
        test_name,
        &[
            Scenario::Refresh(BackendCase::Direct),
            Scenario::Refresh(BackendCase::Secrets),
        ],
    )
}

#[test]
fn covenant_auth_auto_refresh_prefers_each_available_keyring() -> Result<()> {
    let test_name = "backend_sink_tests::covenant_auth_auto_refresh_prefers_each_available_keyring";
    support::run_all(
        test_name,
        &[
            Scenario::Refresh(BackendCase::AutoDirect),
            Scenario::Refresh(BackendCase::AutoSecrets),
        ],
    )
}

#[test]
fn covenant_auth_auto_refresh_falls_back_atomically_for_each_rejected_keyring() -> Result<()> {
    let test_name = "backend_sink_tests::covenant_auth_auto_refresh_falls_back_atomically_for_each_rejected_keyring";
    support::run_all(
        test_name,
        &[
            Scenario::Refresh(BackendCase::AutoDirectFallback),
            Scenario::Refresh(BackendCase::AutoSecretsFallback),
        ],
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

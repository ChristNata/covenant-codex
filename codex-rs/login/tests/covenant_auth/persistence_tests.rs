use super::persistence_support as support;
use super::rotation_authority::Authority;
use super::rotation_authority::Snapshot;
use super::rotation_support;
use anyhow::Result;
use codex_login::AuthDotJson;
use pretty_assertions::assert_eq;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use support::Case;
use support::Fixture;
use support::Operation;
use support::Process;
use support::ReadOnlyFixture;
use support::Report;

#[test]
fn covenant_auth_persistence_refresh_replaces_retained_reader_snapshot() -> Result<()> {
    run("refresh_replaces_retained_reader_snapshot", Case::Replace)
}

#[test]
fn covenant_auth_persistence_readonly_failure_preserves_then_recovers() -> Result<()> {
    run("readonly_failure_preserves_then_recovers", Case::ReadOnly)
}

fn directories_clean(root: &Path) -> Result<bool> {
    let entries: BTreeSet<_> = fs::read_dir(root.join("auth"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<_>>()?;
    Ok(entries
        == BTreeSet::from([
            OsString::from("auth.json"),
            OsString::from(".auth.lock"),
            OsString::from("unrelated.bin"),
        ])
        && fs::read_dir(root.join("mutable"))?.count() == 0)
}

fn run(suffix: &str, case: Case) -> Result<()> {
    let test_name = format!("persistence_tests::covenant_auth_persistence_{suffix}");
    if rotation_support::is_child(&test_name) {
        return support::run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    let nonce = format!("covenant-synthetic-{:032x}", rand::random::<u128>());
    let initial = rotation_support::document(&nonce, /*generation*/ 0)?;
    let auth_file = root.join("auth/auth.json");
    let prior = serde_json::to_vec(&initial)?;
    fs::write(&auth_file, &prior)?;
    let sibling = root.join("auth/unrelated.bin");
    fs::write(&sibling, b"unrelated fixture canary")?;
    // Ordinary sharing permits mutation and rename: retained bytes distinguish in-place writes.
    let mut retained = File::open(&auth_file)?;
    let protected = match case {
        Case::Replace => None,
        Case::ReadOnly => Some(ReadOnlyFixture::protect(auth_file.clone())?),
    };
    let authority = Authority::start(nonce.clone())?;
    authority.arm(/*generation*/ 1);
    let expected = match case {
        Case::Replace => rotation_support::document(&nonce, /*generation*/ 1)?,
        Case::ReadOnly => initial.clone(),
    };
    let fixture = Fixture {
        root: root.clone(),
        initial,
        expected,
        case,
        operation: Operation::Refresh,
        started_at: rotation_support::unix_seconds()?,
    };
    let mut child = Process::spawn(&test_name, &fixture, &authority.url())?;
    child.ready_and_release()?;
    authority.wait_accepted(/*generation*/ 1)?;
    authority.release(/*generation*/ 1);
    let refresh = child.finish()?;
    authority.wait_acknowledged(/*generation*/ 1)?;
    retained.rewind()?;
    let mut retained_bytes = Vec::new();
    retained.read_to_end(&mut retained_bytes)?;
    let retained_prior = retained_bytes == prior;
    let current_bytes = fs::read(&auth_file)?;
    let observed: AuthDotJson = serde_json::from_slice(&current_bytes)
        .map_err(|_| anyhow::anyhow!("stored persistence fixture is invalid"))?;
    let complete_document = match case {
        Case::Replace => rotation_support::whole_document_matches(
            &observed,
            &fixture.expected,
            fixture.started_at,
        ),
        Case::ReadOnly => current_bytes == prior && observed == fixture.initial,
    };
    let clean_before_recovery = directories_clean(&root)?;
    let sibling_before = fs::read(&sibling)? == b"unrelated fixture canary";
    let mut probe_expected = fixture.expected.clone();
    if matches!(case, Case::Replace) {
        probe_expected.last_refresh = observed.last_refresh;
    }
    let probe_fixture = Fixture {
        initial: observed.clone(),
        expected: probe_expected,
        operation: Operation::Probe,
        ..fixture.clone()
    };
    let mut fresh = Process::spawn(&test_name, &probe_fixture, &authority.url())?;
    fresh.ready_and_release()?;
    let probe = fresh.finish()?;
    let recovery = if let Some(protected) = &protected {
        protected.restore()?;
        let recovery_fixture = Fixture {
            initial: observed,
            expected: super::auth_document(&format!("{nonce}-explicit-new-login")),
            operation: Operation::Recovery,
            ..fixture
        };
        let mut recovery = Process::spawn(&test_name, &recovery_fixture, &authority.url())?;
        recovery.ready_and_release()?;
        Some(recovery.finish()?)
    } else {
        None
    };
    let state = authority.snapshot();
    let clean_after_recovery = directories_clean(&root)?;
    let sibling_after = fs::read(sibling)? == b"unrelated fixture canary";
    eprintln!(
        "persistence {suffix}: refresh={refresh:?}, retained_prior={retained_prior}, complete_document={complete_document}, probe={probe:?}, recovery={recovery:?}, clean_before={clean_before_recovery}, clean_after={clean_after_recovery}, authority={state:?}"
    );
    let success = || Report {
        operation_succeeded: true,
        cache_expected: true,
        document_expected: true,
        fault_probe: None,
    };
    let (expected_refresh, expected_recovery) = match case {
        Case::Replace => (success(), None),
        Case::ReadOnly => (
            Report {
                operation_succeeded: false,
                fault_probe: Some(true),
                ..success()
            },
            Some(success()),
        ),
    };
    assert_eq!(
        (
            refresh,
            retained_prior,
            complete_document,
            probe,
            recovery,
            clean_before_recovery,
            clean_after_recovery,
            sibling_before,
            sibling_after,
            state
        ),
        (
            expected_refresh,
            true,
            true,
            success(),
            expected_recovery,
            true,
            true,
            true,
            true,
            Snapshot {
                requests: 1,
                reuses: 0,
                generation: 1,
                acknowledged: 1,
                malformed: false
            }
        )
    );
    Ok(())
}

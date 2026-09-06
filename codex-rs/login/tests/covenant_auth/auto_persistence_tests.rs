//! Auto fallback replacement and keyring preference through public auth APIs.

#![cfg(windows)]

use super::auto_persistence_support as support;
use super::persistence_support::ReadOnlyFixture;
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
use support::Fixture;
use support::KeyringMode;
use support::Report;
use support::SaveOutcome;

#[test]
fn covenant_auth_auto_fallback_replaces_retained_reader() -> Result<()> {
    run("fallback_replaces_retained_reader", Case::Fallback)
}

#[test]
fn covenant_auth_auto_success_prefers_keyring_and_removes_fallback() -> Result<()> {
    run(
        "success_prefers_keyring_and_removes_fallback",
        Case::Keyring,
    )
}

#[test]
fn covenant_auth_auto_readonly_fallback_preserves_then_recovers() -> Result<()> {
    run("readonly_fallback_preserves_then_recovers", Case::ReadOnly)
}

enum Case {
    Fallback,
    Keyring,
    ReadOnly,
}

#[derive(Debug, PartialEq)]
struct Observation {
    report: Report,
    retained: Vec<u8>,
    current: Option<Vec<u8>>,
    auth_entries: BTreeSet<OsString>,
    mutable_entries: BTreeSet<OsString>,
    auth_canary: Vec<u8>,
    mutable_canary: Vec<u8>,
}

fn observe(root: &Path, retained: &mut File, report: Report) -> Result<Observation> {
    retained.rewind()?;
    let mut prior = Vec::new();
    retained.read_to_end(&mut prior)?;
    let current = match fs::read(root.join("auth/auth.json")) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let entries = |path| {
        fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<BTreeSet<_>>>()
    };
    Ok(Observation {
        report,
        retained: prior,
        current,
        auth_entries: entries(root.join("auth"))?,
        mutable_entries: entries(root.join("mutable"))?,
        auth_canary: fs::read(root.join("auth/unrelated.bin"))?,
        mutable_canary: fs::read(root.join("mutable/unrelated.bin"))?,
    })
}

fn run(suffix: &str, case: Case) -> Result<()> {
    let test_name = format!("auto_persistence_tests::covenant_auth_auto_{suffix}");
    if support::is_child(&test_name) {
        return support::run_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    fs::create_dir(root.join("auth"))?;
    fs::create_dir(root.join("mutable"))?;
    let nonce = format!("covenant-synthetic-{:032x}", rand::random::<u128>());
    let initial = rotation_support::document(&nonce, /*generation*/ 0)?;
    let expected = rotation_support::document(&nonce, /*generation*/ 1)?;
    let prior = serde_json::to_vec(&initial)?;
    let auth_file = root.join("auth/auth.json");
    fs::write(&auth_file, &prior)?;
    fs::write(root.join("auth/unrelated.bin"), b"auth-home canary")?;
    fs::write(root.join("mutable/unrelated.bin"), b"mutable-home canary")?;
    // Ordinary Rust sharing permits both writes and rename. This reader must
    // retain the original bytes across a successful fallback replacement.
    let mut retained = File::open(&auth_file)?;
    let protected = match case {
        Case::ReadOnly => Some(ReadOnlyFixture::protect(auth_file)?),
        Case::Fallback | Case::Keyring => None,
    };
    let fixture = Fixture {
        root: root.clone(),
        document: expected.clone(),
        keyring_mode: match case {
            Case::Keyring => KeyringMode::AcceptWrites,
            Case::Fallback | Case::ReadOnly => KeyringMode::RejectWrites,
        },
    };
    let report = support::save_in_child(&test_name, &fixture)?;
    let first = observe(&root, &mut retained, report)?;
    // Read-only restoration is fixture cleanup, never a production retry or bypass.
    let recovery = if let Some(protected) = &protected {
        protected.restore()?;
        let report = support::save_in_child(&test_name, &fixture)?;
        Some(observe(&root, &mut retained, report)?)
    } else {
        None
    };
    let fallback_entries = BTreeSet::from([
        OsString::from(".auth.lock"),
        OsString::from("auth.json"),
        OsString::from("unrelated.bin"),
    ]);
    let expected_observation = |report: Report, current, auth_entries| Observation {
        report,
        retained: prior.clone(),
        current,
        auth_entries,
        mutable_entries: BTreeSet::from([OsString::from("unrelated.bin")]),
        auth_canary: b"auth-home canary".to_vec(),
        mutable_canary: b"mutable-home canary".to_vec(),
    };
    let report = |outcome, loaded: &AuthDotJson, persisted| Report {
        outcome,
        loaded: Some(loaded.clone()),
        attempted: vec![expected.clone()],
        persisted,
    };
    let replaced = expected_observation(
        report(SaveOutcome::Saved, &expected, Vec::new()),
        Some(serde_json::to_vec_pretty(&expected)?),
        fallback_entries.clone(),
    );
    let expected_pair = match case {
        Case::Fallback => (replaced, None),
        Case::Keyring => (
            expected_observation(
                report(SaveOutcome::Saved, &expected, vec![expected.clone()]),
                None,
                BTreeSet::from([
                    OsString::from(".auth.lock"),
                    OsString::from("unrelated.bin"),
                ]),
            ),
            None,
        ),
        Case::ReadOnly => (
            expected_observation(
                report(SaveOutcome::PermissionDenied, &initial, Vec::new()),
                Some(prior.clone()),
                fallback_entries,
            ),
            Some(replaced),
        ),
    };
    assert_eq!((first, recovery), expected_pair);
    Ok(())
}

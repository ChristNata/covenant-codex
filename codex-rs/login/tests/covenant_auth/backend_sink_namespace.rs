//! Namespace selection and filesystem oracles for synthetic keyring tests.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use super::backend_sink_keyring::KeyIdentity;
use super::bounded_child::BoundedChild;
use super::bounded_child::Settlement;
use super::bounded_child::contains;
use super::bounded_child::isolated_test_command;

const NAMESPACE_CHILD: &str = "COVENANT_AUTH_KEYRING_NAMESPACE_CHILD";
pub(super) const REPORT_PREFIX: &str = "COVENANT_AUTH_KEYRING_NAMESPACE_REPORT:";
const CANARY_PATHS: [&str; 5] = [
    "root-canary.bin",
    "mutable/canary.bin",
    "auth/canary.bin",
    "other-auth/canary.bin",
    "alias-hop/canary.bin",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum NamespaceBackend {
    Direct,
    Secrets,
}

impl NamespaceBackend {
    pub(super) fn kind(self) -> AuthKeyringBackendKind {
        match self {
            Self::Direct => AuthKeyringBackendKind::Direct,
            Self::Secrets => AuthKeyringBackendKind::Secrets,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct NamespaceFixture {
    pub(super) root: PathBuf,
    pub(super) override_path: PathBuf,
    pub(super) selected_home: PathBuf,
    pub(super) backend: NamespaceBackend,
    pub(super) auth: AuthDotJson,
    pub(super) other_identity: Option<KeyIdentity>,
    pub(super) decoy: Vec<u8>,
    pub(super) canaries: BTreeMap<PathBuf, Vec<u8>>,
}

pub(super) struct NamespaceAudit {
    pub(super) built: Vec<KeyIdentity>,
    pub(super) values: Vec<(KeyIdentity, Vec<u8>)>,
    pub(super) journal: Vec<NamespaceJournalEntry>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct NamespaceJournalEntry {
    pub(super) operation: &'static str,
    pub(super) identity: KeyIdentity,
}

pub(super) fn run_namespace_test(test_name: &str) -> Result<()> {
    if std::env::var(NAMESPACE_CHILD).ok().as_deref() == Some(test_name) {
        return super::backend_sink_keyring::run_namespace_child();
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    for directory in ["mutable", "alias-hop", "auth", "other-auth"] {
        fs::create_dir(root.join(directory))?;
    }
    let canaries = CANARY_PATHS
        .into_iter()
        .map(|path| (path.into(), randomized("covenant-canary-").into_bytes()))
        .collect::<BTreeMap<_, _>>();
    for (path, bytes) in &canaries {
        fs::write(root.join(path), bytes)?;
    }
    launch_namespace_probes(test_name, root, canaries)
}

fn launch_namespace_probes(
    test_name: &str,
    root: PathBuf,
    canaries: BTreeMap<PathBuf, Vec<u8>>,
) -> Result<()> {
    for backend in [NamespaceBackend::Direct, NamespaceBackend::Secrets] {
        let auth = super::auth_document(&randomized("covenant-synthetic-"));
        let decoy = randomized("covenant-synthetic-decoy-").into_bytes();
        let other_home = root.join("other-auth").canonicalize()?;
        let selected_home = root.join("auth").canonicalize()?;
        let other = run_namespace_probe(
            test_name,
            &NamespaceFixture {
                root: root.clone(),
                override_path: other_home.clone(),
                selected_home: other_home.clone(),
                backend,
                auth: auth.clone(),
                other_identity: None,
                decoy: decoy.clone(),
                canaries: canaries.clone(),
            },
        )?;
        remove_secrets_fixture(&other_home)?;
        let mut fixture = NamespaceFixture {
            root: root.clone(),
            override_path: selected_home.clone(),
            selected_home: selected_home.clone(),
            backend,
            auth,
            other_identity: Some(other.clone()),
            decoy,
            canaries: canaries.clone(),
        };
        let canonical = run_namespace_probe(test_name, &fixture)?;
        remove_secrets_fixture(&selected_home)?;
        fixture.override_path = root.join("alias-hop").join("..").join("auth");
        let alias = run_namespace_probe(test_name, &fixture)?;
        assert_eq!(canonical, alias);
        ensure!(canonical != other, "keyring homes shared an identity");
        assert_identity(backend, &canonical)?;
    }
    Ok(())
}

fn run_namespace_probe(test_name: &str, fixture: &NamespaceFixture) -> Result<KeyIdentity> {
    let mut command = isolated_test_command(
        test_name,
        NAMESPACE_CHILD,
        &fixture.root,
        &fixture.override_path,
    )?;
    let auth_sentinel = fixture
        .auth
        .openai_api_key
        .as_deref()
        .context("API key missing")?;
    let mut child = BoundedChild::capture(command.spawn()?, 131_072, None)?;
    child
        .take_stdin()
        .context("namespace child stdin unavailable")?
        .write_all(&serde_json::to_vec(&fixture)?)?;
    let output = child.wait_for_exit(Duration::from_secs(/*secs*/ 30))?;
    for stream in [&output.stdout, &output.stderr] {
        ensure!(
            !contains(stream, auth_sentinel.as_bytes()) && !contains(stream, &fixture.decoy),
            "sentinel exposed"
        );
    }
    ensure!(
        matches!(&output.settlement, Settlement::Exited(status) if status.success()),
        "namespace child failed: {:?}; stdout={}; stderr={}",
        output.settlement,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let mut reports = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(REPORT_PREFIX));
    let identity = serde_json::from_str(reports.next().context("namespace report missing")?)?;
    ensure!(reports.next().is_none(), "duplicate namespace report");
    Ok(identity)
}

fn remove_secrets_fixture(home: &Path) -> Result<()> {
    let fixture = home.join("secrets/codex_auth.age");
    if fixture.try_exists()? {
        fs::remove_file(fixture)?;
        fs::remove_dir(home.join("secrets"))?;
    }
    Ok(())
}

pub(super) fn assert_namespace_audit(
    fixture: &NamespaceFixture,
    identity: &KeyIdentity,
    audit: NamespaceAudit,
) -> Result<()> {
    let cleanup_identity = audit
        .journal
        .iter()
        .find(|entry| entry.operation == "delete")
        .context("delete journal entry missing")?
        .identity
        .clone();
    assert_identity(NamespaceBackend::Direct, &cleanup_identity)?;
    let (operations, operation_identities): (&[&str], Vec<&KeyIdentity>) = match fixture.backend {
        NamespaceBackend::Direct => (&["set", "get", "delete", "get"], vec![identity; 4]),
        NamespaceBackend::Secrets => {
            ensure!(
                identity.user.strip_prefix("secrets|")
                    == cleanup_identity.user.strip_prefix("cli|"),
                "selected Secrets and Direct identities diverged"
            );
            (
                &["get", "set", "get", "get", "get", "delete", "get"],
                vec![
                    identity,
                    identity,
                    identity,
                    identity,
                    identity,
                    &cleanup_identity,
                    identity,
                ],
            )
        }
    };
    assert_eq!(
        audit.built,
        operation_identities
            .iter()
            .map(|identity| (*identity).clone())
            .collect::<Vec<_>>()
    );
    let mut identities: Vec<_> = audit.values.iter().map(|(key, _)| key.clone()).collect();
    let mut expected_identities = match fixture.backend {
        NamespaceBackend::Direct => Vec::new(),
        NamespaceBackend::Secrets => vec![identity.clone()],
    };
    if let Some(other) = &fixture.other_identity {
        expected_identities.push(other.clone());
        let decoy = audit.values.iter().find(|(key, _)| key == other);
        assert_eq!(decoy.map(|(_, value)| value), Some(&fixture.decoy));
    }
    let auth = &fixture.auth;
    let api_key = auth
        .openai_api_key
        .as_deref()
        .context("selected API key missing")?;
    let serialized = serde_json::to_vec(auth)?;
    ensure!(
        audit
            .values
            .iter()
            .all(
                |(identity, value)| fixture.other_identity.as_ref() == Some(identity)
                    || (!contains(value, api_key.as_bytes()) && !contains(value, &serialized))
            ),
        "selected credential bytes retained"
    );
    identities.sort();
    expected_identities.sort();
    assert_eq!(identities, expected_identities);
    assert_eq!(
        audit.journal,
        operations
            .iter()
            .zip(operation_identities)
            .map(|(operation, identity)| NamespaceJournalEntry {
                operation,
                identity: identity.clone(),
            })
            .collect::<Vec<_>>()
    );
    assert_identity(fixture.backend, identity)
}

pub(super) fn assert_files(fixture: &NamespaceFixture) -> Result<()> {
    for (path, expected) in &fixture.canaries {
        assert_eq!(fs::read(fixture.root.join(path))?, *expected);
    }
    let mut expected = CANARY_PATHS.map(|path| (1, PathBuf::from(path))).to_vec();
    for path in ["alias-hop", "auth", "mutable", "other-auth"] {
        expected.push((2, path.into()));
    }
    let selected = fixture.selected_home.strip_prefix(&fixture.root)?;
    expected.push((1, selected.join(".auth.lock")));
    if matches!(fixture.backend, NamespaceBackend::Secrets) {
        expected.push((2, selected.join("secrets")));
        expected.push((1, selected.join("secrets/codex_auth.age")));
    }
    expected.sort();
    assert_eq!(file_inventory(&fixture.root)?, expected);
    Ok(())
}

fn file_inventory(root: &Path) -> Result<Vec<(usize, PathBuf)>> {
    const ENTRY_LIMIT: usize = 64;
    // Kind: 0 special, 1 file, 2 directory, 3 reparse.
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut entries_seen = 0;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries_seen += 1;
            ensure!(entries_seen <= ENTRY_LIMIT, "inventory overflow");
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            let kind = if std::os::windows::fs::MetadataExt::file_attributes(&metadata) & 0x400 == 0
            {
                usize::from(metadata.is_file()) + 2 * usize::from(metadata.is_dir())
            } else {
                3
            };
            if kind == 2 {
                pending.push(path.clone());
            }
            files.push((kind, path.strip_prefix(root)?.to_path_buf()));
        }
    }
    files.sort();
    Ok(files)
}

fn assert_identity(backend: NamespaceBackend, identity: &KeyIdentity) -> Result<()> {
    let (service, prefix) = match backend {
        NamespaceBackend::Direct => ("Codex Auth", "cli|"),
        NamespaceBackend::Secrets => ("codex", "secrets|"),
    };
    ensure!(
        identity.target.is_none() && identity.service == service,
        "identity mismatch"
    );
    let suffix = identity.user.strip_prefix(prefix).context("user prefix")?;
    ensure!(
        suffix.len() == 16
            && suffix
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "user suffix"
    );
    Ok(())
}

fn randomized(prefix: &str) -> String {
    format!("{prefix}{:032x}", rand::random::<u128>())
}

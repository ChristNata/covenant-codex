//! Process-local keyring fixture for Covenant auth backend tests.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::load_auth_dot_json;
use codex_login::logout;
use codex_login::save_auth;
use keyring::credential::Credential;
use keyring::credential::CredentialApi;
use keyring::credential::CredentialBuilderApi;
use keyring::credential::CredentialPersistence;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Serialize;
use std::any::Any;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

use super::backend_sink_namespace::NamespaceAudit;
use super::backend_sink_namespace::NamespaceFixture;
use super::backend_sink_namespace::NamespaceJournalEntry;

type EntryKey = (Option<String>, String, String);
const JOURNAL_LIMIT: usize = 256;
const VALUE_ENTRY_LIMIT: usize = 16;
const VALUE_LIMIT: usize = 1_048_576;
const VALUE_BYTES_LIMIT: usize = 4_194_304;

#[derive(Default)]
pub(super) struct MemoryStore {
    values: BTreeMap<EntryKey, Vec<u8>>,
    journal: Vec<JournalEntry>,
    built: Vec<KeyIdentity>,
    journal_overflowed: bool,
}

impl MemoryStore {
    pub(super) fn operation_count(&self) -> usize {
        self.journal.len()
    }

    pub(super) fn snapshot(&self) -> KeyringSnapshot {
        KeyringSnapshot {
            values: self.values.values().cloned().collect(),
            journal: self.journal.iter().map(JournalEntry::summary).collect(),
            journal_overflowed: self.journal_overflowed,
        }
    }

    fn seed(&mut self, identity: &KeyIdentity, value: &[u8]) -> Result<()> {
        let key = identity.as_key();
        let prior = self.values.get(&key).map_or(0, Vec::len);
        let total = self
            .values
            .values()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_sub(prior)
            .saturating_add(value.len());
        ensure!(
            value.len() <= VALUE_LIMIT
                && total <= VALUE_BYTES_LIMIT
                && (self.values.contains_key(&key) || self.values.len() < VALUE_ENTRY_LIMIT),
            "namespace seed exceeded fixture value limits"
        );
        self.values.insert(key, value.to_vec());
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(super) struct KeyIdentity {
    pub(super) target: Option<String>,
    pub(super) service: String,
    pub(super) user: String,
}

impl KeyIdentity {
    fn as_key(&self) -> EntryKey {
        (self.target.clone(), self.service.clone(), self.user.clone())
    }
}

#[derive(Clone)]
struct JournalEntry {
    operation: &'static str,
    key: EntryKey,
    bytes: usize,
}

impl JournalEntry {
    fn summary(&self) -> Vec<u8> {
        format!(
            "{}|{:?}|{}|{}|{}",
            self.operation, self.key.0, self.key.1, self.key.2, self.bytes
        )
        .into_bytes()
    }
}

pub(super) struct KeyringSnapshot {
    pub(super) values: Vec<Vec<u8>>,
    pub(super) journal: Vec<Vec<u8>>,
    pub(super) journal_overflowed: bool,
}

struct Builder {
    store: Arc<Mutex<MemoryStore>>,
    accept_writes: bool,
}

impl CredentialBuilderApi for Builder {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        let identity = KeyIdentity {
            target: target.map(str::to_owned),
            service: service.to_owned(),
            user: user.to_owned(),
        };
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        if store.built.len() == JOURNAL_LIMIT {
            return Err(keyring::Error::Invalid(
                "synthetic keyring".into(),
                "fixture builder limit exceeded".into(),
            ));
        }
        store.built.push(identity.clone());
        drop(store);
        Ok(Box::new(MemoryCredential {
            store: Arc::clone(&self.store),
            key: identity.as_key(),
            accept_writes: self.accept_writes,
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

struct MemoryCredential {
    store: Arc<Mutex<MemoryStore>>,
    key: EntryKey,
    accept_writes: bool,
}

impl MemoryCredential {
    fn with_store<T>(
        &self,
        name: &'static str,
        bytes: usize,
        operation: impl FnOnce(&mut MemoryStore) -> keyring::Result<T>,
    ) -> keyring::Result<T> {
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        if store.journal.len() == JOURNAL_LIMIT {
            store.journal_overflowed = true;
            return Err(keyring::Error::Invalid(
                "synthetic keyring".into(),
                "fixture operation limit exceeded".into(),
            ));
        }
        store.journal.push(JournalEntry {
            operation: name,
            key: self.key.clone(),
            bytes,
        });
        operation(&mut store)
    }
}

impl CredentialApi for MemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.with_store("set", secret.len(), |store| {
            if !self.accept_writes {
                return Err(keyring::Error::Invalid(
                    "synthetic keyring".into(),
                    "fixture rejected write".into(),
                ));
            }
            let is_new = !store.values.contains_key(&self.key);
            let prior_bytes = store.values.get(&self.key).map_or(0, Vec::len);
            let total_bytes = store
                .values
                .values()
                .map(Vec::len)
                .sum::<usize>()
                .saturating_sub(prior_bytes)
                .saturating_add(secret.len());
            if secret.len() > VALUE_LIMIT
                || total_bytes > VALUE_BYTES_LIMIT
                || (is_new && store.values.len() == VALUE_ENTRY_LIMIT)
            {
                return Err(keyring::Error::Invalid(
                    "synthetic keyring".into(),
                    "fixture value limit exceeded".into(),
                ));
            }
            store.values.insert(self.key.clone(), secret.to_vec());
            Ok(())
        })
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        self.with_store("get", /*bytes*/ 0, |store| {
            store
                .values
                .get(&self.key)
                .cloned()
                .ok_or(keyring::Error::NoEntry)
        })
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        self.with_store("delete", /*bytes*/ 0, |store| {
            store
                .values
                .remove(&self.key)
                .map(|_| ())
                .ok_or(keyring::Error::NoEntry)
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(super) fn install(accept_writes: bool) -> Arc<Mutex<MemoryStore>> {
    let store = Arc::new(Mutex::new(MemoryStore::default()));
    keyring::set_default_credential_builder(Box::new(Builder {
        store: Arc::clone(&store),
        accept_writes,
    }));
    store
}

pub(super) fn run_namespace_child() -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().take(131_072).read_to_end(&mut input)?;
    ensure!(
        input.len() < 131_072,
        "namespace fixture input exceeded limit"
    );
    let fixture: NamespaceFixture = serde_json::from_slice(&input)?;
    assert_eq!(
        std::env::var_os("CODEX_AUTH_HOME").map(PathBuf::from),
        Some(fixture.override_path.clone())
    );
    let store = install(/*accept_writes*/ true);
    if let Some(identity) = &fixture.other_identity {
        store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .seed(identity, &fixture.decoy)?;
    }
    save_auth(
        &fixture.root.join("mutable"),
        &fixture.auth,
        AuthCredentialsStoreMode::Keyring,
        fixture.backend.kind(),
    )?;
    assert_eq!(
        load_auth_dot_json(
            &fixture.root.join("mutable"),
            AuthCredentialsStoreMode::Keyring,
            fixture.backend.kind(),
        )?,
        Some(fixture.auth.clone())
    );
    ensure!(
        logout(
            &fixture.root.join("mutable"),
            AuthCredentialsStoreMode::Keyring,
            fixture.backend.kind(),
        )?,
        "public keyring logout removed nothing"
    );
    assert_eq!(
        load_auth_dot_json(
            &fixture.root.join("mutable"),
            AuthCredentialsStoreMode::Keyring,
            fixture.backend.kind(),
        )?,
        None
    );
    let store = store.lock().unwrap_or_else(PoisonError::into_inner);
    let identity = store
        .built
        .first()
        .cloned()
        .context("keyring identity missing")?;
    let to_identity = |key: &EntryKey| KeyIdentity {
        target: key.0.clone(),
        service: key.1.clone(),
        user: key.2.clone(),
    };
    let audit = NamespaceAudit {
        built: store.built.clone(),
        values: store
            .values
            .iter()
            .map(|(key, value)| (to_identity(key), value.clone()))
            .collect(),
        journal: store
            .journal
            .iter()
            .map(|entry| NamespaceJournalEntry {
                operation: entry.operation,
                identity: to_identity(&entry.key),
            })
            .collect(),
    };
    drop(store);
    super::backend_sink_namespace::assert_namespace_audit(&fixture, &identity, audit)?;
    super::backend_sink_namespace::assert_files(&fixture)?;
    fs::remove_file(fixture.selected_home.join(".auth.lock"))?;
    println!(
        "{}{}",
        super::backend_sink_namespace::REPORT_PREFIX,
        serde_json::to_string(&identity)?
    );
    Ok(())
}

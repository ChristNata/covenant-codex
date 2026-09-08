//! Process-local keyring fixture for Covenant auth backend tests.

#![cfg(windows)]

use keyring::credential::Credential;
use keyring::credential::CredentialApi;
use keyring::credential::CredentialBuilderApi;
use keyring::credential::CredentialPersistence;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

type EntryKey = (Option<String>, String, String);

#[derive(Default)]
pub(super) struct MemoryStore {
    values: BTreeMap<EntryKey, Vec<u8>>,
    operations: usize,
}

impl MemoryStore {
    pub(super) fn operation_count(&self) -> usize {
        self.operations
    }
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
        Ok(Box::new(MemoryCredential {
            store: Arc::clone(&self.store),
            key: (
                target.map(str::to_owned),
                service.to_owned(),
                user.to_owned(),
            ),
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
    fn with_store<T>(&self, operation: impl FnOnce(&mut MemoryStore) -> T) -> T {
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        store.operations += 1;
        operation(&mut store)
    }
}

impl CredentialApi for MemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.with_store(|store| {
            if !self.accept_writes {
                return Err(keyring::Error::Invalid(
                    "synthetic keyring".into(),
                    "fixture rejected write".into(),
                ));
            }
            store.values.insert(self.key.clone(), secret.to_vec());
            Ok(())
        })
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        self.with_store(|store| {
            store
                .values
                .get(&self.key)
                .cloned()
                .ok_or(keyring::Error::NoEntry)
        })
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        self.with_store(|store| {
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

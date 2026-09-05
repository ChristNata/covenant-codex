//! Ownership of mutations to an opted-in persistent auth store.

use super::storage::AuthDotJson;
use super::storage::AuthStorageBackend;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::TryLockError;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

const LOCK_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 30);
const LOCK_RETRY: Duration = Duration::from_millis(/*millis*/ 10);

#[derive(Debug)]
pub(super) struct CovenantAuthStorage {
    transaction: AuthTransaction,
}

impl CovenantAuthStorage {
    pub(super) fn new(home: PathBuf, backend: Arc<dyn AuthStorageBackend>) -> Self {
        Self {
            transaction: AuthTransaction {
                lock_path: home.join(".auth.lock"),
                backend,
            },
        }
    }
}

impl AuthStorageBackend for CovenantAuthStorage {
    fn load(&self) -> io::Result<Option<AuthDotJson>> {
        self.transaction.backend.load()
    }

    fn save(&self, auth: &AuthDotJson) -> io::Result<()> {
        self.transaction.try_lock()?.save(auth)
    }

    fn delete(&self) -> io::Result<bool> {
        self.transaction.try_lock()?.delete()
    }

    fn covenant_transaction(&self) -> Option<AuthTransaction> {
        Some(self.transaction.clone())
    }
}

/// An owned store reference whose raw mutation methods require its lock guard.
#[derive(Clone, Debug)]
pub(super) struct AuthTransaction {
    lock_path: PathBuf,
    backend: Arc<dyn AuthStorageBackend>,
}

impl AuthTransaction {
    pub(super) fn try_lock(&self) -> io::Result<AuthWriteGuard> {
        // The lock file is permanent: deleting it would let callers own
        // different file identities for the same credential store.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        match file.try_lock() {
            Ok(()) => Ok(AuthWriteGuard {
                _lock: file,
                backend: Arc::clone(&self.backend),
            }),
            Err(TryLockError::WouldBlock) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "auth storage is busy",
            )),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }

    pub(super) async fn lock(&self) -> io::Result<AuthWriteGuard> {
        let deadline = Instant::now() + LOCK_TIMEOUT;
        loop {
            match self.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "timed out waiting for auth storage",
                        ));
                    }
                    tokio::time::sleep(LOCK_RETRY).await;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

/// Keeps the exclusive OS lock until all operations using this backend finish.
/// Its methods delegate directly to the configured backend without relocking.
#[derive(Debug)]
pub(super) struct AuthWriteGuard {
    _lock: File,
    backend: Arc<dyn AuthStorageBackend>,
}

impl AuthStorageBackend for AuthWriteGuard {
    fn load(&self) -> io::Result<Option<AuthDotJson>> {
        self.backend.load()
    }

    fn save(&self, auth: &AuthDotJson) -> io::Result<()> {
        self.backend.save(auth)
    }

    fn delete(&self) -> io::Result<bool> {
        self.backend.delete()
    }
}

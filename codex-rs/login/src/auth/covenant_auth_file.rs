//! Atomic File saves inside the opted-in store's existing transaction ownership.

use super::storage::AuthDotJson;
use super::storage::AuthStorageBackend;
use super::storage::get_auth_file;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct CovenantAuthFile {
    home: PathBuf,
    delegate: Arc<dyn AuthStorageBackend>,
}

impl CovenantAuthFile {
    pub(super) fn new(home: PathBuf, delegate: Arc<dyn AuthStorageBackend>) -> Self {
        Self { home, delegate }
    }
}

impl AuthStorageBackend for CovenantAuthFile {
    fn load(&self) -> io::Result<Option<AuthDotJson>> {
        self.delegate.load()
    }

    fn save(&self, auth: &AuthDotJson) -> io::Result<()> {
        let mut temporary = tempfile::Builder::new()
            .prefix(".auth-")
            .suffix(".tmp")
            .make_in(&self.home, |path| {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(path)
            })
            .map_err(storage_error)?;
        let cleanup_error = |error: io::Error| {
            io::Error::new(error.kind(), "unable to remove temporary auth storage")
        };
        let prepared: io::Result<()> = (|| {
            serde_json::to_writer_pretty(&mut temporary, auth)?;
            temporary.flush()?;
            temporary.as_file().sync_all()
        })();
        if let Err(error) = prepared {
            temporary.close().map_err(cleanup_error)?;
            return Err(storage_error(error));
        }

        // Rust 1.95's Windows rename supports retained readers without setting
        // IGNORE_READONLY. The owned source has ordinary file attributes.
        // Content is synced above; this does not promise crash-durable metadata.
        match std::fs::rename(temporary.path(), get_auth_file(&self.home)) {
            Ok(()) => {
                // Never unlink a reused old temporary name after the commit.
                temporary.disable_cleanup(/*disable_cleanup*/ true);
                Ok(())
            }
            Err(error) => {
                temporary.close().map_err(cleanup_error)?;
                Err(storage_error(error))
            }
        }
    }

    fn delete(&self) -> io::Result<bool> {
        self.delegate.delete()
    }
}

fn storage_error(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "unable to persist auth storage")
}

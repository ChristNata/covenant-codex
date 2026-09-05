//! Windows auth-home selection, frozen at the first storage factory call.

use super::storage::AuthDotJson;
use super::storage::AuthStorageBackend;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

// Freeze only the override: without it, every caller keeps its supplied home.
static AUTH_HOME: OnceLock<Result<Option<PathBuf>, AuthHomeError>> = OnceLock::new();

pub(super) fn resolve_auth_home() -> Result<Option<PathBuf>, AuthHomeError> {
    match AUTH_HOME.get_or_init(|| {
        let Some(path) = parse_override(std::env::var_os("CODEX_AUTH_HOME"))? else {
            return Ok(None);
        };

        // Resolve aliases before any backend derives its namespace. A new root
        // must have the same identity on its first use and subsequent uses.
        std::fs::create_dir_all(&path)
            .and_then(|()| path.canonicalize())
            .map(Some)
            .map_err(|error| AuthHomeError {
                kind: error.kind(),
                message: "unable to initialize CODEX_AUTH_HOME",
            })
    }) {
        Ok(path) => Ok(path.clone()),
        Err(error) => Err(*error),
    }
}

fn parse_override(value: Option<OsString>) -> Result<Option<PathBuf>, AuthHomeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(AuthHomeError {
            kind: io::ErrorKind::InvalidInput,
            message: "CODEX_AUTH_HOME must be a nonempty absolute path",
        });
    }
    Ok(Some(path))
}

/// A failed override stays failed for every operation, without opening a fallback.
#[derive(Clone, Copy, Debug)]
pub(super) struct AuthHomeError {
    kind: io::ErrorKind,
    message: &'static str,
}

impl AuthHomeError {
    fn as_io_error(&self) -> io::Error {
        io::Error::new(self.kind, self.message)
    }
}

impl AuthStorageBackend for AuthHomeError {
    fn load(&self) -> io::Result<Option<AuthDotJson>> {
        Err(self.as_io_error())
    }

    fn save(&self, _auth: &AuthDotJson) -> io::Result<()> {
        Err(self.as_io_error())
    }

    fn delete(&self) -> io::Result<bool> {
        Err(self.as_io_error())
    }
}

//! Immutable startup expectations, validated without filesystem or process I/O.

use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::path::Prefix;

const MAX_PATH_UNITS: usize = 32_766;
const MAX_MARKER_UNITS: usize = 4_096;
const SHA256_TEXT_UNITS: usize = 64;

/// Named startup values captured by the CLI before configuration and model work.
pub struct StartupControls {
    pub decider_path: Option<OsString>,
    pub decider_sha256: Option<OsString>,
    pub child_marker: Option<OsString>,
}

/// Owned startup expectations; these do not prove any native executable identity.
pub struct LaunchContract {
    decider_path: PathBuf,
    expected_sha256: [u8; 32],
    _marker_present: MarkerPresent,
}

// Only the successful presence check survives; the opaque marker text is dropped.
struct MarkerPresent;

/// A content-free refusal of invalid or missing startup controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchContractError;

impl std::fmt::Display for LaunchContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid startup launch contract")
    }
}

impl std::error::Error for LaunchContractError {}

impl LaunchContract {
    /// Validate one owned startup snapshot without reading or retrying environment values.
    pub fn from_startup(controls: StartupControls) -> Result<Self, LaunchContractError> {
        let marker = controls.child_marker.ok_or(LaunchContractError)?;
        bounded_text(&marker, MAX_MARKER_UNITS)?;
        drop(marker);

        let decider_path = controls.decider_path.ok_or(LaunchContractError)?;
        let path_text = bounded_text(&decider_path, MAX_PATH_UNITS)?;
        let path = Path::new(&decider_path);
        let mut components = path.components();
        if !matches!(
            components.next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        ) || !matches!(components.next(), Some(Component::RootDir))
            || path.file_name().is_none()
            || path_text.ends_with(['\\', '/'])
            || components.any(|component| component == Component::ParentDir)
        {
            return Err(LaunchContractError);
        }

        let digest = controls.decider_sha256.ok_or(LaunchContractError)?;
        let digest_text = bounded_text(&digest, SHA256_TEXT_UNITS)?;
        if digest_text.len() != SHA256_TEXT_UNITS {
            return Err(LaunchContractError);
        }
        let mut expected_sha256 = [0; 32];
        for (index, digit) in digest_text.bytes().enumerate() {
            let nibble = match digit {
                b'0'..=b'9' => digit - b'0',
                b'a'..=b'f' => digit - b'a' + 10,
                _ => return Err(LaunchContractError),
            };
            let byte = &mut expected_sha256[index / 2];
            *byte = (*byte << 4) | nibble;
        }

        Ok(Self {
            decider_path: PathBuf::from(decider_path),
            expected_sha256,
            _marker_present: MarkerPresent,
        })
    }

    pub fn decider_path(&self) -> &Path {
        &self.decider_path
    }

    pub fn expected_sha256(&self) -> &[u8; 32] {
        &self.expected_sha256
    }
}

fn bounded_text(value: &OsStr, maximum_units: usize) -> Result<&str, LaunchContractError> {
    let units = value.encode_wide().take(maximum_units + 1).count();
    if units == 0 || units > maximum_units {
        return Err(LaunchContractError);
    }
    let text = value.to_str().ok_or(LaunchContractError)?;
    if text.contains('\0') {
        return Err(LaunchContractError);
    }
    Ok(text)
}

#[cfg(test)]
#[path = "launch_contract_tests.rs"]
mod tests;

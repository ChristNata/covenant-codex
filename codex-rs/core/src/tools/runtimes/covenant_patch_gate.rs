//! Covenant's pre-writer Patch Decide v1 gate.

use codex_apply_patch::Hunk;
use codex_apply_patch::UpdateFileChunk;
use codex_covenant::IdentityKind;
use codex_covenant::PatchHunk;
use codex_covenant::PatchHunkLine;
use codex_covenant::PatchHunkLineKind;
use codex_covenant::PatchOperation;
use codex_covenant::PatchOperationKind;
use codex_covenant::PatchPermissions;
use codex_covenant::PatchRequest;
use codex_covenant::ResolvedIdentity;
use codex_covenant::SidecarClient;
use codex_covenant::SidecarDecision;
use codex_covenant::encode_patch_request;
use codex_utils_path_uri::PathUri;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

const DECIDER_PATH_ENV: &str = "COVENANT_DECIDER_PATH";
const DECIDER_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum PatchGateError {
    #[error("Covenant patch gate refused: {0}")]
    Refused(String),
}

/// Captures native identities, encodes the complete request, and asks the
/// selected real sidecar. Only an exact `ALLOW` response can pass.
pub(crate) fn authorize_patch(
    patch: &str,
    cwd: &PathUri,
    write_roots: &[PathUri],
    sandbox: &str,
    network: bool,
    tty: bool,
) -> Result<(), PatchGateError> {
    let request = prepare_patch(patch, cwd, write_roots, sandbox, network, tty)?;
    let encoded = encode_patch_request(&request, &cwd.to_string())
        .map_err(|error| PatchGateError::Refused(error.to_string()))?;
    let executable = std::env::var_os(DECIDER_PATH_ENV)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| PatchGateError::Refused("COVENANT_DECIDER_PATH is unset".to_string()))?;
    let client = SidecarClient::new(executable, DECIDER_DEADLINE)
        .map_err(|error| PatchGateError::Refused(error.to_string()))?;
    match client
        .decide(&encoded)
        .map_err(|error| PatchGateError::Refused(error.to_string()))?
    {
        SidecarDecision::Allow => Ok(()),
        SidecarDecision::Deny { reason } => Err(PatchGateError::Refused(reason)),
        SidecarDecision::AllowWithContext { .. } => Err(PatchGateError::Refused(
            "sidecar returned ALLOW_WITH_CONTEXT instead of exact ALLOW".to_string(),
        )),
    }
}

fn prepare_patch(
    patch: &str,
    cwd: &PathUri,
    write_roots: &[PathUri],
    sandbox: &str,
    network: bool,
    tty: bool,
) -> Result<PatchRequest, PatchGateError> {
    let parsed = codex_apply_patch::parse_patch(patch)
        .map_err(|_| PatchGateError::Refused("patch parsing failed".to_string()))?;
    if parsed.hunks.is_empty() {
        return Err(PatchGateError::Refused(
            "patch contains no operations".to_string(),
        ));
    }

    let mut paths = Vec::with_capacity(parsed.hunks.len());
    let mut operations = Vec::with_capacity(parsed.hunks.len());
    let mut identities = Vec::with_capacity(parsed.hunks.len());
    for hunk in parsed.hunks {
        let (path, operation, destination, requires_existing) = describe_hunk(&hunk, cwd)?;
        let path_string = path.to_string();
        if !paths.contains(&path_string) {
            paths.push(path_string.clone());
        }
        if let Some(destination) = destination.as_ref() {
            let destination_string = destination.to_string();
            if !paths.contains(&destination_string) {
                paths.push(destination_string);
            }
        }
        let identity_path = if requires_existing {
            path.clone()
        } else {
            path.parent().ok_or_else(|| {
                PatchGateError::Refused("new patch target has no parent".to_string())
            })?
        };
        let identity = resolve_identity(&identity_path, true)?;
        if !identities
            .iter()
            .any(|existing: &ResolvedIdentity| existing.path == identity.path)
        {
            identities.push(identity);
        }
        operations.push(PatchOperation {
            path: path_string,
            op: operation,
            destination: destination.map(|value| value.to_string()),
            hunks: hunk_to_wire(&hunk),
            pre_image_digest: pre_image_digest(&path, requires_existing)?,
        });
    }

    Ok(PatchRequest {
        paths,
        operations,
        permissions: PatchPermissions {
            sandbox: sandbox.to_string(),
            write_roots: write_roots.iter().map(ToString::to_string).collect(),
            network,
            tty,
        },
        resolved_identities: identities,
    })
}

fn describe_hunk(
    hunk: &Hunk,
    cwd: &PathUri,
) -> Result<(PathUri, PatchOperationKind, Option<PathUri>, bool), PatchGateError> {
    match hunk {
        Hunk::AddFile { path, .. } => Ok((
            resolve_path(cwd, path)?,
            PatchOperationKind::Add,
            None,
            false,
        )),
        Hunk::DeleteFile { path } => Ok((
            resolve_path(cwd, path)?,
            PatchOperationKind::Delete,
            None,
            true,
        )),
        Hunk::UpdateFile {
            path, move_path, ..
        } => {
            let source = resolve_path(cwd, path)?;
            Ok((
                source,
                if move_path.is_some() {
                    PatchOperationKind::Move
                } else {
                    PatchOperationKind::Update
                },
                move_path
                    .as_ref()
                    .map(|path| resolve_path(cwd, path))
                    .transpose()?,
                true,
            ))
        }
    }
}

fn resolve_path(cwd: &PathUri, path: &Path) -> Result<PathUri, PatchGateError> {
    cwd.join(&path.to_string_lossy())
        .map_err(|_| PatchGateError::Refused("patch path could not be resolved".to_string()))
}

fn hunk_to_wire(hunk: &Hunk) -> Vec<PatchHunk> {
    match hunk {
        Hunk::AddFile { contents, .. } => vec![PatchHunk {
            old_start: 0,
            old_lines: 0,
            new_start: 0,
            new_lines: contents.lines().count() as u64,
            lines: contents
                .lines()
                .map(|text| PatchHunkLine {
                    kind: PatchHunkLineKind::Add,
                    text: text.to_string(),
                })
                .collect(),
        }],
        Hunk::DeleteFile { .. } => Vec::new(),
        Hunk::UpdateFile { chunks, .. } => chunks.iter().map(chunk_to_wire).collect(),
    }
}

fn chunk_to_wire(chunk: &UpdateFileChunk) -> PatchHunk {
    let mut lines = Vec::new();
    for (index, text) in chunk.old_lines.iter().enumerate() {
        if chunk
            .context_line_indices
            .iter()
            .any(|(old, _)| *old == index)
        {
            lines.push(PatchHunkLine {
                kind: PatchHunkLineKind::Context,
                text: text.clone(),
            });
        } else {
            lines.push(PatchHunkLine {
                kind: PatchHunkLineKind::Remove,
                text: text.clone(),
            });
        }
    }
    for (index, text) in chunk.new_lines.iter().enumerate() {
        if !chunk
            .context_line_indices
            .iter()
            .any(|(_, new)| *new == index)
        {
            lines.push(PatchHunkLine {
                kind: PatchHunkLineKind::Add,
                text: text.clone(),
            });
        }
    }
    PatchHunk {
        old_start: 0,
        old_lines: chunk.old_lines.len() as u64,
        new_start: 0,
        new_lines: chunk.new_lines.len() as u64,
        lines,
    }
}

fn resolve_identity(
    path: &PathUri,
    requires_existing: bool,
) -> Result<ResolvedIdentity, PatchGateError> {
    let native = path.to_path_buf();
    let metadata = match fs::symlink_metadata(&native) {
        Ok(metadata) => metadata,
        Err(error) if !requires_existing && error.kind() == std::io::ErrorKind::NotFound => {
            let parent = native.parent().ok_or_else(|| {
                PatchGateError::Refused("new patch target has no existing parent".to_string())
            })?;
            fs::symlink_metadata(parent).map_err(|_| {
                PatchGateError::Refused("new patch target parent identity unavailable".to_string())
            })?
        }
        Err(_) => {
            return Err(PatchGateError::Refused(
                "patch target identity unavailable".to_string(),
            ));
        }
    };
    let kind = if metadata.file_type().is_symlink() {
        IdentityKind::Symlink
    } else if metadata.is_dir() {
        IdentityKind::Directory
    } else {
        IdentityKind::File
    };
    let target = if matches!(kind, IdentityKind::Symlink) {
        Some(
            fs::read_link(&native)
                .map_err(|_| PatchGateError::Refused("symlink target unavailable".to_string()))?
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    };
    let (volume_serial, file_index) = native_identity(&native, &metadata)?;
    let digest = if matches!(kind, IdentityKind::File) {
        Some(hash_file(&native)?)
    } else {
        None
    };
    Ok(ResolvedIdentity {
        path: path.to_string(),
        kind,
        target,
        ancestor_identities: Vec::new(),
        win32_normalized: path.inferred_native_path_string(),
        volume_serial,
        file_index,
        pre_image_digest: digest,
    })
}

fn pre_image_digest(
    path: &PathUri,
    requires_existing: bool,
) -> Result<Option<String>, PatchGateError> {
    if !requires_existing {
        return Ok(None);
    }
    let native = path.to_path_buf();
    let metadata = fs::symlink_metadata(&native)
        .map_err(|_| PatchGateError::Refused("patch pre-image unavailable".to_string()))?;
    if metadata.is_file() && !metadata.file_type().is_symlink() {
        Ok(Some(hash_file(&native)?))
    } else {
        Ok(None)
    }
}

fn hash_file(path: &Path) -> Result<String, PatchGateError> {
    let bytes = fs::read(path)
        .map_err(|_| PatchGateError::Refused("patch pre-image unreadable".to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn native_identity(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(String, String), PatchGateError> {
    #[cfg(windows)]
    {
        let _ = metadata;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_NORMAL,
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
        };
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        // SAFETY: `wide` is NUL-terminated and remains alive for the call; the
        // returned handle is closed on every path below.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(PatchGateError::Refused(
                "native file identity could not be opened".to_string(),
            ));
        }
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        // SAFETY: `info` points to writable storage and `handle` is valid.
        let result = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
        // SAFETY: `handle` was returned by CreateFileW and is closed exactly once.
        unsafe { CloseHandle(handle) };
        if result == 0 {
            return Err(PatchGateError::Refused(
                "native file identity could not be queried".to_string(),
            ));
        }
        // SAFETY: GetFileInformationByHandle succeeded and initialized `info`.
        let info = unsafe { info.assume_init() };
        let file_index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
        return Ok((
            info.dwVolumeSerialNumber.to_string(),
            file_index.to_string(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Ok((metadata.dev().to_string(), metadata.ino().to_string()));
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        Err(PatchGateError::Refused(
            "native file identity is unsupported on this platform".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_patch;
    use codex_utils_path_uri::PathUri;
    use std::fs;

    #[test]
    fn prepares_add_with_parent_identity_and_typed_operations() {
        let root = tempfile::tempdir().expect("tempdir");
        let cwd = PathUri::from_host_native_path(root.path()).expect("cwd URI");
        let request = prepare_patch(
            "*** Begin Patch\n*** Add File: note.txt\n+hello\n*** End Patch",
            &cwd,
            std::slice::from_ref(&cwd),
            "none",
            false,
            false,
        )
        .expect("request");
        assert_eq!(request.operations.len(), 1);
        assert_eq!(request.operations[0].pre_image_digest, None);
        assert_eq!(request.resolved_identities.len(), 1);
    }

    #[test]
    fn prepares_update_with_preimage_digest() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("note.txt");
        fs::write(&path, "old\n").expect("write fixture");
        let cwd = PathUri::from_host_native_path(root.path()).expect("cwd URI");
        let request = prepare_patch(
            "*** Begin Patch\n*** Update File: note.txt\n@@\n-old\n+new\n*** End Patch",
            &cwd,
            std::slice::from_ref(&cwd),
            "none",
            false,
            false,
        )
        .expect("request");
        assert_eq!(
            request.operations[0]
                .pre_image_digest
                .as_deref()
                .map(str::len),
            Some(64)
        );
        assert_eq!(
            request.resolved_identities[0]
                .pre_image_digest
                .as_deref()
                .map(str::len),
            Some(64)
        );
    }
}

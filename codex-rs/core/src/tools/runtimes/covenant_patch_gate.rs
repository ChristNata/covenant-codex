//! Covenant's pre-writer Patch Decide v1 gate.

use codex_apply_patch::Hunk;
use codex_apply_patch::UpdateFileChunk;
use codex_covenant::AncestorIdentity;
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
pub(crate) fn authorize_patch(patch: &str, cwd: &PathUri) -> Result<(), PatchGateError> {
    let request = prepare_patch(patch, cwd)?;
    let encoded = encode_patch_request(&request, &native_path_string(cwd))
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
        SidecarDecision::Deny {
            reason,
            remediation,
        } => {
            let remediation = remediation
                .map(|remediation| format!(" Remediation: {remediation}"))
                .unwrap_or_default();
            Err(PatchGateError::Refused(format!("{reason}{remediation}")))
        }
        SidecarDecision::AllowWithContext { .. } => Err(PatchGateError::Refused(
            "sidecar returned ALLOW_WITH_CONTEXT instead of exact ALLOW".to_string(),
        )),
    }
}

fn prepare_patch(patch: &str, cwd: &PathUri) -> Result<PatchRequest, PatchGateError> {
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
        let path_string = native_path_string(&path);
        if !paths.contains(&path_string) {
            paths.push(path_string.clone());
        }
        if let Some(destination) = destination.as_ref() {
            let destination_string = native_path_string(destination);
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
        let identity = resolve_identity(&identity_path)?;
        if !identities
            .iter()
            .any(|existing: &ResolvedIdentity| existing.path == identity.path)
        {
            identities.push(identity);
        }
        operations.push(PatchOperation {
            path: path_string,
            op: operation,
            destination: destination.map(|value| native_path_string(&value)),
            hunks: hunk_to_wire(&hunk),
            pre_image_digest: pre_image_digest(&path, requires_existing)?,
        });
    }

    Ok(PatchRequest {
        paths,
        operations,
        permissions: PatchPermissions {
            sandbox: "workspace-write".to_string(),
            write_roots: vec![native_path_string(cwd)],
            network: false,
            tty: false,
        },
        resolved_identities: identities,
    })
}

fn native_path_string(path: &PathUri) -> String {
    path.inferred_native_path_string()
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

fn resolve_identity(path: &PathUri) -> Result<ResolvedIdentity, PatchGateError> {
    let native = path.to_path_buf();
    let identity = native_identity(&native)?;
    let digest = if matches!(identity.kind, IdentityKind::File) {
        Some(hash_file(&native)?)
    } else {
        None
    };
    Ok(ResolvedIdentity {
        path: native_path_string(path),
        kind: identity.kind,
        target: identity.target,
        ancestor_identities: resolve_ancestor_identities(&native)?,
        win32_normalized: identity.win32_normalized,
        volume_serial: identity.volume_serial,
        file_index: identity.file_index,
        pre_image_digest: digest,
    })
}

fn resolve_ancestor_identities(path: &Path) -> Result<Vec<AncestorIdentity>, PatchGateError> {
    path.parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .map(|ancestor| {
            let identity = native_identity(ancestor)?;
            Ok(AncestorIdentity {
                path: ancestor.to_string_lossy().into_owned(),
                kind: identity.kind,
                target: identity.target,
                win32_normalized: identity.win32_normalized,
                volume_serial: identity.volume_serial,
                file_index: identity.file_index,
            })
        })
        .collect()
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

struct NativeIdentity {
    kind: IdentityKind,
    target: Option<String>,
    win32_normalized: String,
    volume_serial: String,
    file_index: String,
}

fn native_identity(path: &Path) -> Result<NativeIdentity, PatchGateError> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::Foundation::GENERIC_READ;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION;
        use windows_sys::Win32::Storage::FileSystem::CreateFileW;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE;
        use windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle;
        use windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING;
        let metadata = fs::symlink_metadata(path).map_err(|_| {
            PatchGateError::Refused("native file identity metadata is unavailable".to_string())
        })?;
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
        let result = (|| {
            let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
            // SAFETY: `info` points to writable storage and `handle` is valid.
            if unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) } == 0 {
                return Err(PatchGateError::Refused(
                    "native file identity could not be queried".to_string(),
                ));
            }
            // SAFETY: GetFileInformationByHandle succeeded and initialized `info`.
            let info = unsafe { info.assume_init() };
            let reparse = metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
            let kind = if reparse && metadata.file_type().is_symlink() {
                IdentityKind::Symlink
            } else if reparse {
                IdentityKind::Junction
            } else if metadata.is_dir() {
                IdentityKind::Directory
            } else {
                IdentityKind::File
            };
            let target = if reparse {
                Some(
                    fs::read_link(path)
                        .map_err(|_| {
                            PatchGateError::Refused(
                                "native link identity target is unavailable".to_string(),
                            )
                        })?
                        .to_string_lossy()
                        .into_owned(),
                )
            } else {
                None
            };
            let file_index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
            Ok(NativeIdentity {
                kind,
                target,
                win32_normalized: final_path_for_handle(handle)?,
                volume_serial: info.dwVolumeSerialNumber.to_string(),
                file_index: file_index.to_string(),
            })
        })();
        // SAFETY: `handle` was returned by CreateFileW and is closed exactly once.
        unsafe { CloseHandle(handle) };
        return result;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path).map_err(|_| {
            PatchGateError::Refused("native file identity metadata is unavailable".to_string())
        })?;
        let kind = if metadata.file_type().is_symlink() {
            IdentityKind::Symlink
        } else if metadata.is_dir() {
            IdentityKind::Directory
        } else {
            IdentityKind::File
        };
        let target = if matches!(kind, IdentityKind::Symlink) {
            Some(
                fs::read_link(path)
                    .map_err(|_| {
                        PatchGateError::Refused(
                            "native link identity target is unavailable".to_string(),
                        )
                    })?
                    .to_string_lossy()
                    .into_owned(),
            )
        } else {
            None
        };
        return Ok(NativeIdentity {
            kind,
            target,
            win32_normalized: path.to_string_lossy().into_owned(),
            volume_serial: metadata.dev().to_string(),
            file_index: metadata.ino().to_string(),
        });
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(PatchGateError::Refused(
            "native file identity is unsupported on this platform".to_string(),
        ))
    }
}

#[cfg(windows)]
fn final_path_for_handle(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> Result<String, PatchGateError> {
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;

    let mut buffer = vec![0_u16; 260];
    loop {
        // SAFETY: `handle` is open and `buffer` is writable for its full length.
        let length = unsafe {
            GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, 0)
        };
        if length == 0 {
            return Err(PatchGateError::Refused(
                "native file identity final path is unavailable".to_string(),
            ));
        }
        if (length as usize) < buffer.len() {
            return Ok(String::from_utf16_lossy(&buffer[..length as usize]));
        }
        buffer.resize(length as usize + 1, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::authorize_patch;
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
        )
        .expect("request");
        assert_eq!(request.operations.len(), 1);
        assert_eq!(request.operations[0].pre_image_digest, None);
        assert_eq!(request.resolved_identities.len(), 1);
        assert!(
            !request.resolved_identities[0]
                .ancestor_identities
                .is_empty()
        );
        assert_eq!(
            request.permissions.write_roots,
            vec![cwd.inferred_native_path_string()]
        );
        assert!(!request.permissions.network);
        assert!(!request.permissions.tty);
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
        assert!(
            !request.resolved_identities[0]
                .ancestor_identities
                .is_empty()
        );
    }

    #[test]
    fn real_decider_allows_all_bounded_operations_when_configured() {
        if std::env::var_os("COVENANT_REAL_DECIDER_TEST").is_none() {
            return;
        }
        let current_dir = std::env::current_dir().expect("current directory");
        let root = tempfile::tempdir_in(current_dir).expect("in-worktree tempdir");
        let cwd = PathUri::from_host_native_path(root.path()).expect("cwd URI");

        authorize_patch(
            "*** Begin Patch\n*** Add File: added.txt\n+hello\n*** End Patch",
            &cwd,
        )
        .expect("real decider should allow a bounded add");

        let path = root.path().join("updated.txt");
        fs::write(&path, "old\n").expect("write update fixture");
        authorize_patch(
            "*** Begin Patch\n*** Update File: updated.txt\n@@\n-old\n+new\n*** End Patch",
            &cwd,
        )
        .expect("real decider should allow a bounded update");

        let path = root.path().join("deleted.txt");
        fs::write(&path, "old\n").expect("write delete fixture");
        authorize_patch(
            "*** Begin Patch\n*** Delete File: deleted.txt\n*** End Patch",
            &cwd,
        )
        .expect("real decider should allow a bounded delete");

        let path = root.path().join("moved.txt");
        fs::write(&path, "old\n").expect("write move fixture");
        authorize_patch(
            "*** Begin Patch\n*** Update File: moved.txt\n*** Move to: destination.txt\n@@\n-old\n+new\n*** End Patch",
            &cwd,
        )
        .expect("real decider should allow a bounded move");
    }
}

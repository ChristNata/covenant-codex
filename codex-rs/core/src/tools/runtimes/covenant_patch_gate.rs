//! Covenant's pre-writer patch gate.
//!
//! The upstream patch parser can identify the requested paths, but it does not
//! resolve the native identities required by Decide v1. This module keeps that
//! distinction explicit: a request is never passed to a sidecar or writer
//! until the complete envelope can be prepared.

use codex_apply_patch::Hunk;
use codex_utils_path_uri::PathUri;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum PatchGateError {
    #[error("Covenant patch envelope is unavailable: {0}")]
    Unsupported(&'static str),
}

/// The bounded information available after parsing, before native identity
/// resolution. It is intentionally not serializable as a Decide v1 request.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParsedPatch {
    pub(crate) paths: Vec<PathUri>,
}

pub(crate) fn prepare_patch(patch: &str, cwd: &PathUri) -> Result<ParsedPatch, PatchGateError> {
    let parsed = codex_apply_patch::parse_patch(patch)
        .map_err(|_| PatchGateError::Unsupported("patch parsing failed"))?;
    let paths = parsed
        .hunks
        .iter()
        .map(|hunk| resolve_path(hunk, cwd))
        .collect::<Result<Vec<_>, _>>()?;
    if paths.is_empty() {
        return Err(PatchGateError::Unsupported("patch contains no operations"));
    }
    Err(PatchGateError::Unsupported(
        "native file identities and pre-image digests are not resolved",
    ))
}

fn resolve_path(hunk: &Hunk, cwd: &PathUri) -> Result<PathUri, PatchGateError> {
    hunk.resolve_path(cwd)
        .map_err(|_| PatchGateError::Unsupported("patch path could not be resolved"))
}

#[cfg(test)]
mod tests {
    use super::PatchGateError;
    use super::prepare_patch;
    use codex_utils_path_uri::PathUri;

    #[test]
    fn refuses_before_writer_without_native_identity_facts() {
        let cwd = PathUri::parse("C:\\work").expect("absolute path");
        let result = prepare_patch(
            "*** Begin Patch\n*** Add File: note.txt\n+hello\n*** End Patch",
            &cwd,
        );
        assert_eq!(
            result,
            Err(PatchGateError::Unsupported(
                "native file identities and pre-image digests are not resolved",
            ))
        );
    }
}

use crate::DecideV1;
use serde::Serialize;

/// A typed Patch Decide v1 request assembled by an execution gate.
#[derive(Clone, Debug, Serialize)]
pub struct PatchRequest {
    /// Absolute paths covered by the patch operations.
    pub paths: Vec<String>,
    /// The frozen patch operations and hunks.
    pub operations: Vec<PatchOperation>,
    /// Permissions in force for the writer.
    pub permissions: PatchPermissions,
    /// Native identities captured before the writer starts.
    pub resolved_identities: Vec<ResolvedIdentity>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchOperation {
    pub path: String,
    pub op: PatchOperationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub hunks: Vec<PatchHunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_image_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchOperationKind {
    Add,
    Update,
    Delete,
    Move,
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchHunk {
    pub old_start: u64,
    pub old_lines: u64,
    pub new_start: u64,
    pub new_lines: u64,
    pub lines: Vec<PatchHunkLine>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchHunkLine {
    pub kind: PatchHunkLineKind,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchHunkLineKind {
    Context,
    Add,
    Remove,
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchPermissions {
    pub sandbox: String,
    pub write_roots: Vec<String>,
    pub network: bool,
    pub tty: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedIdentity {
    pub path: String,
    pub kind: IdentityKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub ancestor_identities: Vec<AncestorIdentity>,
    pub win32_normalized: String,
    pub volume_serial: String,
    pub file_index: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_image_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AncestorIdentity {
    pub path: String,
    pub kind: IdentityKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub win32_normalized: String,
    pub volume_serial: String,
    pub file_index: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentityKind {
    File,
    Directory,
    Symlink,
    Junction,
}

#[derive(Debug)]
pub enum PatchRequestError {
    Serialize(serde_json::Error),
    Invalid,
}

impl std::fmt::Display for PatchRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Serialize(_) => "patch request serialization failed",
            Self::Invalid => "patch request failed Decide v1 validation",
        })
    }
}

impl std::error::Error for PatchRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::Invalid => None,
        }
    }
}

/// Serializes and validates one complete Patch Decide v1 envelope.
pub fn encode_patch_request(
    request: &PatchRequest,
    cwd: &str,
) -> Result<Vec<u8>, PatchRequestError> {
    let path = request.paths.first().ok_or(PatchRequestError::Invalid)?;
    let envelope = serde_json::json!({
        "hook_event_name": "Patch",
        "cwd": cwd,
        "path": path,
        "decide_v1": {
            "version": 1,
            "kind": "patch",
            "patch": request,
        },
    });
    let bytes = serde_json::to_vec(&envelope).map_err(PatchRequestError::Serialize)?;
    let decide =
        serde_json::to_vec(&envelope["decide_v1"]).map_err(PatchRequestError::Serialize)?;
    DecideV1::decode(&decide).map_err(|_| PatchRequestError::Invalid)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_patch_as_hook_event_with_validated_inner_decision() {
        let request = PatchRequest {
            paths: vec![r"C:\work\a.txt".to_string()],
            operations: vec![PatchOperation {
                path: r"C:\work\a.txt".to_string(),
                op: PatchOperationKind::Add,
                destination: None,
                hunks: vec![],
                pre_image_digest: None,
            }],
            permissions: PatchPermissions {
                sandbox: "workspace-write".to_string(),
                write_roots: vec![r"C:\work".to_string()],
                network: false,
                tty: false,
            },
            resolved_identities: vec![ResolvedIdentity {
                path: r"C:\work".to_string(),
                kind: IdentityKind::Directory,
                target: None,
                ancestor_identities: vec![],
                win32_normalized: r"C:\work".to_string(),
                volume_serial: "1".to_string(),
                file_index: "2".to_string(),
                pre_image_digest: None,
            }],
        };
        let bytes = encode_patch_request(&request, r"C:\work").expect("valid patch envelope");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON envelope");
        assert_eq!(value["hook_event_name"], "Patch");
        assert_eq!(value["decide_v1"]["kind"], "patch");
    }
}

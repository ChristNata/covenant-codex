use crate::FunctionCallError;
use crate::ToolName;
use crate::ToolPayload;

/// The two tool identities admitted by the constrained Covenant build.
///
/// This classifies a tool call, not authorization to execute a process or patch.
/// Those effects require their separate operation gates.
pub enum CovenantTool {
    /// Process-effect row; execution still requires the F12 gate.
    ExecCommand,
    /// Filesystem-effect row; mutation still requires the F13 gate.
    ApplyPatch,
}

impl CovenantTool {
    /// Admit only an exact unqualified name and its prescribed wire payload form.
    /// The caller must supply the original namespace without default normalization.
    pub fn admit(name: &ToolName, payload: &ToolPayload) -> Result<Self, FunctionCallError> {
        if name.namespace.is_some() {
            return Err(FunctionCallError::CovenantDenied);
        }
        match payload {
            ToolPayload::Function { .. } if name.name == "exec_command" => Ok(Self::ExecCommand),
            ToolPayload::Custom { .. } if name.name == "apply_patch" => Ok(Self::ApplyPatch),
            ToolPayload::Function { .. }
            | ToolPayload::Custom { .. }
            | ToolPayload::ToolSearch { .. } => Err(FunctionCallError::CovenantDenied),
        }
    }
}

use crate::FunctionCallError;
use crate::TOOL_SEARCH_TOOL_NAME;
use crate::ToolName;
use crate::ToolPayload;
use codex_protocol::models::ResponseItem;

enum WireForm {
    Function,
    Custom,
    ToolSearch,
}

/// Tool identities admitted by the constrained Covenant build.
///
/// This classifies a tool call, not authorization to execute a process or patch.
/// Those effects require their separate operation gates.
pub enum CovenantTool {
    /// Process-effect row; execution still requires the F12 gate.
    ExecCommand,
    /// Filesystem-effect row; mutation still requires the F13 gate.
    ApplyPatch,
    /// Read the namespace-scoped fanin inventory through the managed gateway.
    FaninListTools,
    /// Read one upstream tool schema through the managed gateway.
    FaninGetToolSchema,
    /// Invoke an upstream tool through the managed gateway and its namespace ACL.
    FaninInvokeTool,
}

impl CovenantTool {
    /// Admit only the exact name, namespace, and prescribed wire payload form.
    /// The caller must supply the original namespace without default normalization.
    pub fn admit(name: &ToolName, payload: &ToolPayload) -> Result<Self, FunctionCallError> {
        let form = match payload {
            ToolPayload::Function { .. } => WireForm::Function,
            ToolPayload::Custom { .. } => WireForm::Custom,
            ToolPayload::ToolSearch { .. } => WireForm::ToolSearch,
        };
        Self::admit_identity(name.namespace.as_deref(), &name.name, form)
    }

    /// Classify raw local tool calls before parsing arguments or normalizing names.
    /// `None` denotes a response item outside this local tool-call classifier;
    /// an admitted classification is not authority for native or hosted effects.
    pub fn admit_response_item(item: &ResponseItem) -> Result<Option<Self>, FunctionCallError> {
        let (namespace, name, form) = match item {
            ResponseItem::FunctionCall {
                namespace, name, ..
            } => (namespace.as_deref(), name.as_str(), WireForm::Function),
            ResponseItem::CustomToolCall {
                namespace, name, ..
            } => (namespace.as_deref(), name.as_str(), WireForm::Custom),
            ResponseItem::ToolSearchCall { .. } => {
                (None, TOOL_SEARCH_TOOL_NAME, WireForm::ToolSearch)
            }
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::Message { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => return Ok(None),
        };
        Self::admit_identity(namespace, name, form).map(Some)
    }

    fn admit_identity(
        namespace: Option<&str>,
        name: &str,
        form: WireForm,
    ) -> Result<Self, FunctionCallError> {
        match (namespace, form, name) {
            (None, WireForm::Function, "exec_command") => Ok(Self::ExecCommand),
            (None, WireForm::Custom, "apply_patch") => Ok(Self::ApplyPatch),
            (Some("mcp__fanin"), WireForm::Function, "list_tools") => Ok(Self::FaninListTools),
            (Some("mcp__fanin"), WireForm::Function, "get_tool_schema") => {
                Ok(Self::FaninGetToolSchema)
            }
            (Some("mcp__fanin"), WireForm::Function, "invoke_tool") => Ok(Self::FaninInvokeTool),
            (None | Some(_), WireForm::Function | WireForm::Custom | WireForm::ToolSearch, _) => {
                Err(FunctionCallError::CovenantDenied)
            }
        }
    }
}

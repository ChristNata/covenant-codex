//! Builds the constrained router from the existing guarded execution handlers.
use super::CoreToolPlanContext;
use super::add_shell_tools;
use super::merge_into_namespaces;
use super::tool_environment_mode;
use crate::session::session::Session;
use crate::tools::handlers::ApplyPatchHandler;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use codex_mcp::McpBinding;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::openai_models::ToolMode;
use codex_tools::ToolEnvironmentMode;
use codex_tools::ToolName;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::Arc;

pub(super) fn build_tool_router(
    session: &Session,
    context: &CoreToolPlanContext<'_>,
    mcp: &Arc<McpBinding>,
) -> CodexResult<ToolRouter> {
    let mut registry = ToolRegistry::default();
    add_shell_tools(context, &mut registry);

    let environment_mode = tool_environment_mode(context.environments);
    if environment_mode.has_environment() && context.model_info.apply_patch_tool_type.is_some() {
        let include_environment_id = matches!(environment_mode, ToolEnvironmentMode::Multiple);
        registry.add(ApplyPatchHandler::new(include_environment_id));
    }

    let fanin_catalog = &context.mcp.config().mcp_server_catalog;
    if fanin_catalog
        .server("fanin")
        .is_some_and(|server| server.config().enabled)
    {
        let registered = session.services.mcp_handler_cache.append_mcp_tools(
            mcp,
            &context.turn_context.config,
            /*apps_enabled*/ false,
            fanin_catalog,
            /*search_tool_enabled*/ false,
            &mut registry,
        );
        let expected = ["list_tools", "get_tool_schema", "invoke_tool"]
            .map(|name| ToolName::namespaced("mcp__fanin", name))
            .into_iter()
            .collect::<HashSet<_>>();
        if registered != expected {
            return Err(CodexErrorDetails::Fatal(
                "Covenant fanin gateway did not expose its three bounded meta-tools".to_string(),
            )
            .into());
        }
    }

    let specs = merge_into_namespaces(registry.entries().map(|tool| tool.runtime.spec()).collect());
    Ok(ToolRouter::from_parts(
        registry,
        specs,
        ToolMode::Direct,
        BTreeMap::new(),
        /*tool_namespaces_info*/ None,
        &[],
    ))
}

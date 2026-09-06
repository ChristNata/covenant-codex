//! Builds the constrained router from the existing guarded execution handlers.
use super::CoreToolPlanContext;
use super::add_shell_tools;
use super::tool_environment_mode;
use crate::tools::handlers::ApplyPatchHandler;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use codex_protocol::openai_models::ToolMode;
use codex_tools::ToolEnvironmentMode;
use std::collections::BTreeMap;

pub(super) fn build_tool_router(context: &CoreToolPlanContext<'_>) -> ToolRouter {
    let mut registry = ToolRegistry::default();
    add_shell_tools(context, &mut registry);

    let environment_mode = tool_environment_mode(context.environments);
    if environment_mode.has_environment() && context.model_info.apply_patch_tool_type.is_some() {
        let include_environment_id = matches!(environment_mode, ToolEnvironmentMode::Multiple);
        registry.add(ApplyPatchHandler::new(include_environment_id));
    }

    let specs = registry.entries().map(|tool| tool.runtime.spec()).collect();
    ToolRouter::from_parts(
        registry,
        specs,
        ToolMode::Direct,
        BTreeMap::new(),
        /*tool_namespaces_info*/ None,
        &[],
    )
}

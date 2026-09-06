use std::io;

use codex_config::ConfigRequirements;
use codex_config::Constrained;
use codex_config::ConstrainedWithSource;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::ExperimentalRequestUserInput;
use codex_config::config_toml::OrchestratorFeatureToml;
use codex_config::config_toml::OrchestratorToml;
use codex_config::config_toml::UpdatePlanToolConfig;
use codex_config::types::ToolSuggestConfig;
use codex_protocol::config_types::WebSearchMode;

use super::ConfigOverrides;

pub(super) fn apply_config_profile(
    cfg: &mut ConfigToml,
    overrides: &mut ConfigOverrides,
    requirements: &ConfigRequirements,
) -> io::Result<()> {
    if requirements
        .managed_hooks
        .as_ref()
        .is_some_and(|hooks| hooks.get().handler_count() > 0)
    {
        return Err(refusal());
    }

    cfg.orchestrator = Some(OrchestratorToml {
        skills: Some(OrchestratorFeatureToml {
            enabled: Some(false),
        }),
        mcp: Some(OrchestratorFeatureToml {
            enabled: Some(false),
        }),
    });
    cfg.mcp_servers.clear();
    cfg.web_search = Some(WebSearchMode::Disabled);
    cfg.notify = None;
    cfg.tool_suggest = Some(ToolSuggestConfig::default());
    let tools = cfg.tools.get_or_insert_default();
    tools.web_search = None;
    tools.experimental_request_user_input = Some(ExperimentalRequestUserInput { enabled: false });
    tools.update_plan = Some(UpdatePlanToolConfig { enabled: false });
    overrides.bypass_hook_trust = Some(false);
    Ok(())
}

pub(super) fn freeze_web_search_mode(
    mode: &mut ConstrainedWithSource<WebSearchMode>,
) -> io::Result<()> {
    let original = mode.value.clone();
    let mut frozen = Constrained::normalized(WebSearchMode::Disabled, |_| WebSearchMode::Disabled)
        .map_err(|_| refusal())?;
    frozen
        .add_validator(move |candidate| original.can_set(candidate))
        .map_err(|_| refusal())?;
    mode.value = frozen;
    Ok(())
}

fn refusal() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Covenant profile rejected configuration requirements",
    )
}

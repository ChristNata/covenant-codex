//! Pins the one managed fanin stdio server before constructing final Config.

use codex_config::config_toml::ConfigToml;
use std::io;

#[cfg(windows)]
use codex_config::types::AppToolApproval;
#[cfg(windows)]
use codex_config::types::McpServerConfig;
#[cfg(windows)]
use codex_config::types::McpServerTransportConfig;
#[cfg(windows)]
use codex_covenant::FaninBinding;
#[cfg(windows)]
use codex_covenant::LaunchContract;
#[cfg(windows)]
use codex_covenant::StartupControls;
#[cfg(windows)]
use std::collections::HashMap;

pub(super) fn clamp_fanin(cfg: &mut ConfigToml) -> io::Result<()> {
    #[cfg(windows)]
    {
        let controls = StartupControls {
            decider_path: std::env::var_os("COVENANT_DECIDER_PATH"),
            decider_sha256: std::env::var_os("COVENANT_DECIDER_SHA256"),
            child_marker: std::env::var_os("COVENANT_CHILD_MARKER"),
        };
        let namespace = std::env::var("COVENANT_MCP_NAMESPACE").ok();
        clamp_with_startup(&mut cfg.mcp_servers, controls, namespace.as_deref())
    }
    #[cfg(not(windows))]
    {
        cfg.mcp_servers.clear();
        Ok(())
    }
}

#[cfg(windows)]
fn clamp_with_startup(
    servers: &mut HashMap<String, McpServerConfig>,
    controls: StartupControls,
    namespace: Option<&str>,
) -> io::Result<()> {
    let fanin = servers.remove("fanin");
    servers.clear();
    let Some(mut fanin) = fanin else {
        return Ok(());
    };

    let launch = LaunchContract::from_startup(controls).map_err(|_| refusal())?;
    let namespace = namespace.ok_or_else(refusal)?;
    if !fanin.enabled || !fanin.is_local_environment() {
        return Err(refusal());
    }
    let McpServerTransportConfig::Stdio {
        command,
        args,
        env,
        env_vars,
        cwd,
    } = &mut fanin.transport
    else {
        return Err(refusal());
    };
    if env.is_some() || !env_vars.is_empty() || cwd.is_some() {
        return Err(refusal());
    }
    let binding = FaninBinding::from_managed_launch(&launch, command, args, namespace)
        .map_err(|_| refusal())?;
    args[3] = binding.namespace().to_string();

    fanin.required = true;
    fanin.supports_parallel_tool_calls = false;
    fanin.omit_tools_from = None;
    fanin.disabled_reason = None;
    fanin.default_tools_approval_mode = Some(AppToolApproval::Approve);
    fanin.enabled_tools = Some(vec![
        "list_tools".to_string(),
        "get_tool_schema".to_string(),
        "invoke_tool".to_string(),
    ]);
    fanin.disabled_tools = None;
    fanin.scopes = None;
    fanin.oauth = None;
    fanin.oauth_resource = None;
    fanin.tools.clear();
    servers.insert("fanin".to_string(), fanin);
    Ok(())
}

#[cfg(windows)]
fn refusal() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Covenant rejected fanin startup binding",
    )
}

#[cfg(all(test, windows))]
#[path = "covenant_fanin_tests.rs"]
mod tests;

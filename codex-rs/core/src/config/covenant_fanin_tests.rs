use super::*;
use pretty_assertions::assert_eq;
use std::ffi::OsString;

fn managed_controls() -> StartupControls {
    StartupControls {
        decider_path: Some(OsString::from(r"C:\Covenant\covenant-cli.exe")),
        decider_sha256: Some(OsString::from("0".repeat(64))),
        child_marker: Some(OsString::from("codex-child")),
    }
}

fn managed_fanin() -> McpServerConfig {
    toml::from_str(
        r#"
command = 'C:\Covenant\fanin-mcp.exe'
args = ['--config', 'C:\Covenant\mcp\config.toml', '--namespace', '${COVENANT_MCP_NAMESPACE}']
"#,
    )
    .expect("managed fanin config")
}

#[test]
fn covenant_fanin_profile_keeps_only_validated_gateway_and_meta_tools() -> io::Result<()> {
    let mut servers = HashMap::from([
        ("fanin".to_string(), managed_fanin()),
        (
            "other".to_string(),
            toml::from_str("url = 'http://localhost:1234/mcp'").expect("other server"),
        ),
    ]);
    clamp_with_startup(&mut servers, managed_controls(), Some("p_47de8f8d"))?;

    let mut expected = managed_fanin();
    let McpServerTransportConfig::Stdio { args, .. } = &mut expected.transport else {
        panic!("managed server must be stdio");
    };
    args[3] = "p_47de8f8d".to_string();
    expected.required = true;
    expected.enabled_tools = Some(vec![
        "list_tools".to_string(),
        "get_tool_schema".to_string(),
        "invoke_tool".to_string(),
    ]);
    assert_eq!(servers, HashMap::from([("fanin".to_string(), expected)]));
    Ok(())
}

#[test]
fn covenant_fanin_profile_without_gateway_preserves_two_tool_mode() -> io::Result<()> {
    let mut servers = HashMap::from([(
        "other".to_string(),
        toml::from_str("url = 'http://localhost:1234/mcp'").expect("other server"),
    )]);
    clamp_with_startup(
        &mut servers,
        StartupControls {
            decider_path: None,
            decider_sha256: None,
            child_marker: None,
        },
        None,
    )?;
    assert_eq!(servers, HashMap::new());
    Ok(())
}

#[test]
fn covenant_fanin_profile_refuses_unmanaged_gateway_before_connection() {
    for namespace in ["unknown", "global", "global_explorer"] {
        let mut servers = HashMap::from([("fanin".to_string(), managed_fanin())]);
        let result = clamp_with_startup(&mut servers, managed_controls(), Some(namespace));
        assert_eq!(
            result.map_err(|error| error.kind()),
            Err(io::ErrorKind::InvalidData)
        );
        assert_eq!(servers, HashMap::new());
    }
}

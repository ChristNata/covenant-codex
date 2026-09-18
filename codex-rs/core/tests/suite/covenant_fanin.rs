//! Managed Codex turn → native fanin stdio client → two upstream stdio servers.

use anyhow::Context;
use anyhow::Result;
use codex_config::types::AppToolApproval;
use codex_config::types::McpServerConfig;
use codex_core::config::Constrained;
use codex_login::CodexAuth;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use core_test_support::responses;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_mcp_server;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const NAMESPACE: &str = "p_47de8f8d";
const LIVE_MODEL: &str = "gpt-6-astra";

fn upstream_config(upstream: &Path) -> Result<String> {
    let command = serde_json::to_string(&upstream.to_string_lossy())?;
    Ok(format!(
        r#"
[servers.alpha]
transport = "stdio"
command = {command}
description = "First local test server"
[servers.alpha.env]
MCP_TEST_DYNAMIC_SERVER_METADATA = "1"

[servers.beta]
transport = "stdio"
command = {command}
description = "Second local test server"
[servers.beta.env]
MCP_TEST_DYNAMIC_SERVER_METADATA = "1"

[namespaces.{NAMESPACE}]
servers = ["alpha", "beta"]
[namespaces.{NAMESPACE}.tools]
alpha = ["echo", "sync"]
beta = ["echo"]
"#
    ))
}

fn tool_identities(request: &responses::ResponsesRequest) -> Vec<(String, String, Vec<String>)> {
    let body = request.body_json();
    let mut identities = body["tools"]
        .as_array()
        .expect("model-visible tool array")
        .iter()
        .map(|tool| {
            let names = tool["tools"]
                .as_array()
                .map(|tools| {
                    tools
                        .iter()
                        .map(|inner| {
                            inner["name"]
                                .as_str()
                                .expect("namespace tool name")
                                .to_string()
                        })
                        .collect()
                })
                .unwrap_or_default();
            (
                tool["name"].as_str().expect("tool name").to_string(),
                tool["type"].as_str().expect("tool type").to_string(),
                names,
            )
        })
        .collect::<Vec<_>>();
    identities.sort();
    identities
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn covenant_fanin_turn_exposes_only_gateway_tools_and_calls_two_upstreams() -> Result<()> {
    let Some(source) = std::env::var_os("COVENANT_FANIN_MCP_TEST_EXE") else {
        // The release F31 job always provides the pinned gateway. Ordinary core test jobs do not.
        return Ok(());
    };
    let source = std::path::PathBuf::from(source);
    let version = std::process::Command::new(&source)
        .arg("--version")
        .output()?;
    anyhow::ensure!(
        version.status.success()
            && String::from_utf8_lossy(&version.stdout).trim() == "fanin-mcp 1.2.0",
        "fanin E2E gateway must be pinned at 1.2.0"
    );
    let upstream = codex_utils_cargo_bin::cargo_bin("test_stdio_server")?;
    let managed = TempDir::new()?;
    let managed_mcp = managed.path().join("mcp");
    fs::create_dir_all(&managed_mcp)?;
    let gateway = managed.path().join("fanin-mcp.exe");
    fs::copy(source, &gateway)?;
    let config_path = managed_mcp.join("config.toml");
    fs::write(&config_path, upstream_config(&upstream)?)?;

    let server = responses::start_mock_server().await;
    let calls = [
        ("resp-1", "call-list", "list_tools", "{}"),
        (
            "resp-2",
            "call-schema",
            "get_tool_schema",
            r#"{"name":"alpha__echo"}"#,
        ),
        (
            "resp-3",
            "call-alpha",
            "invoke_tool",
            r#"{"name":"alpha__echo","arguments":{"message":"via alpha"}}"#,
        ),
        (
            "resp-4",
            "call-beta",
            "invoke_tool",
            r#"{"name":"beta__echo","arguments":{"message":"via beta"}}"#,
        ),
        (
            "resp-5",
            "call-non-read-only",
            "invoke_tool",
            r#"{"name":"alpha__sync","arguments":{}}"#,
        ),
        (
            "resp-6",
            "call-denied",
            "invoke_tool",
            r#"{"name":"beta__cwd","arguments":{}}"#,
        ),
    ];
    let mut bodies = calls
        .map(|(response_id, call_id, name, arguments)| {
            responses::sse(vec![
                responses::ev_response_created(response_id),
                responses::ev_function_call_with_namespace(call_id, "mcp__fanin", name, arguments),
                responses::ev_completed(response_id),
            ])
        })
        .to_vec();
    bodies.push(responses::sse(vec![
        responses::ev_response_created("resp-7"),
        responses::ev_assistant_message("msg-7", "fanin done"),
        responses::ev_completed("resp-7"),
    ]));
    let response_mock = responses::mount_sse_sequence(&server, bodies).await;

    let server_config: McpServerConfig = serde_json::from_value(json!({
        "command": gateway,
        "args": ["--config", config_path, "--namespace", NAMESPACE],
        "enabled_tools": ["list_tools", "get_tool_schema", "invoke_tool"],
        "default_tools_approval_mode": "approve",
        "required": true,
        "supports_parallel_tool_calls": false,
    }))?;
    assert_eq!(
        server_config.default_tools_approval_mode,
        Some(AppToolApproval::Approve)
    );
    let mut catalog = codex_models_manager::covenant_model_catalog()?;
    catalog.models[0].slug = LIVE_MODEL.to_owned();
    catalog.models[0].tool_mode = Some(ToolMode::CodeModeOnly);
    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            config.model = Some(LIVE_MODEL.to_owned());
            config.model_catalog = Some(catalog);
            let pinned = std::collections::HashMap::from([("fanin".to_owned(), server_config)]);
            let frozen = pinned.clone();
            config.mcp_servers = Constrained::normalized(pinned, move |_| frozen.clone())
                .expect("sole managed fanin server");
        });
    let test = builder.build_with_auto_env(&server).await?;
    wait_for_mcp_server(&test.codex, "fanin").await?;
    test.submit_turn_with_policies(
        "Use fanin's two upstream echo tools and the non-read-only sync tool, then reply done",
        AskForApproval::Never,
        SandboxPolicy::new_workspace_write_policy(),
    )
    .await?;

    let requests = response_mock.requests();
    assert_eq!(requests.len(), 7);
    let expected_tools = vec![
        ("apply_patch".to_owned(), "custom".to_owned(), vec![]),
        ("exec_command".to_owned(), "function".to_owned(), vec![]),
        (
            "mcp__fanin".to_owned(),
            "namespace".to_owned(),
            vec![
                "get_tool_schema".to_owned(),
                "invoke_tool".to_owned(),
                "list_tools".to_owned(),
            ],
        ),
    ];
    assert_eq!(
        requests.iter().map(tool_identities).collect::<Vec<_>>(),
        vec![expected_tools; 7]
    );
    let output_text = |index: usize, call_id: &str| -> Result<String> {
        let item = requests[index].function_call_output(call_id);
        match &item["output"] {
            Value::String(text) => Ok(text.clone()),
            Value::Array(items) => {
                let text = items
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::ensure!(!text.is_empty(), "MCP call output has no text: {item}");
                Ok(text)
            }
            Value::Object(output) => {
                anyhow::ensure!(
                    output.get("success") != Some(&Value::Bool(false)),
                    "MCP call failed: {item}"
                );
                output
                    .get("content")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .with_context(|| format!("MCP call output has no text: {item}"))
            }
            _ => anyhow::bail!("MCP call output has no text: {item}"),
        }
    };
    let list = output_text(1, "call-list")?;
    let schema = output_text(2, "call-schema")?;
    let alpha = output_text(3, "call-alpha")?;
    let beta = output_text(4, "call-beta")?;
    let non_read_only = output_text(5, "call-non-read-only")?;
    let denied = output_text(6, "call-denied")?;
    anyhow::ensure!(
        list.contains("\"server\":\"alpha\"")
            && list.contains("\"server\":\"beta\"")
            && list.contains("\"tool\":\"echo\"")
            && list.contains("\"tool\":\"sync\""),
        "fanin did not list both upstreams: {list}"
    );
    anyhow::ensure!(
        schema.contains("message"),
        "fanin did not route upstream schema: {schema}"
    );
    anyhow::ensure!(
        alpha.contains("rmcp-test-process-")
            && beta.contains("rmcp-test-process-")
            && alpha != beta,
        "fanin did not invoke two distinct upstream processes"
    );
    anyhow::ensure!(
        non_read_only.contains("\"result\":\"ok\""),
        "fanin did not invoke the allowed non-read-only upstream tool: {non_read_only}"
    );
    anyhow::ensure!(
        denied.contains("namespace_denied") && denied.contains("beta") && denied.contains("cwd"),
        "fanin did not deny a non-allowed upstream tool at invocation: {denied}"
    );
    Ok(())
}

//! Windows ordinary Codex MCP integration test.
use super::HttpRequest;
use super::cli_fixture::API_KEY;
use super::cli_fixture::Fixture;
use super::cli_fixture::MARKER;
use super::cli_fixture::PROMPT;
use anyhow::Result;
use anyhow::anyhow;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

fn dynamic_id<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a str> {
    let value = value
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{label} missing"))?;
    if value.is_empty() || value.len() > 128 {
        return Err(anyhow!("{label} must contain 1..=128 UTF-8 bytes"));
    }
    Ok(value)
}

fn ordinary_tool_identities() -> BTreeSet<(String, Option<String>)> {
    [
        ("function", Some("exec_command")),
        ("function", Some("write_stdin")),
        ("function", Some("request_user_input")),
        ("function", Some("view_image")),
        ("function", Some("list_mcp_resources")),
        ("function", Some("list_mcp_resource_templates")),
        ("function", Some("read_mcp_resource")),
        ("custom", Some("apply_patch")),
        ("tool_search", None),
        ("web_search", None),
    ]
    .into_iter()
    .map(|(wire_type, name)| (wire_type.to_owned(), name.map(str::to_owned)))
    .collect()
}

fn tool_identity(value: &Value) -> Result<(String, Option<String>)> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("ordinary tool is not an object"))?;
    if object.contains_key("namespace") {
        return Err(anyhow!("ordinary tool namespace refused"));
    }
    let wire_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("ordinary tool type missing"))?;
    let name = match wire_type {
        "function" | "custom" => Some(
            object
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty() && name.len() <= 128)
                .ok_or_else(|| anyhow!("named ordinary tool identity missing"))?
                .to_owned(),
        ),
        "tool_search" | "web_search" => {
            if object.contains_key("name") {
                return Err(anyhow!("unnamed ordinary tool contains a name"));
            }
            None
        }
        _ => return Err(anyhow!("unknown ordinary tool type")),
    };
    Ok((wire_type.to_owned(), name))
}

fn request_value(request: &HttpRequest) -> Value {
    json!({
        "method":request.method, "target":request.target,
        "headers":request.headers, "body":request.body
    })
}

#[tokio::test(flavor = "current_thread")]
async fn ordinary_mcp_requests_are_observed() -> Result<()> {
    let case_started = Instant::now();
    let mut fixture = Fixture::mcp_only().await?;
    assert!(
        std::fs::read_dir(fixture.effect_directory())?
            .next()
            .is_none()
    );
    let observed = fixture.run_mcp().await?;
    assert!(
        observed.exit.success(),
        "native failure: {:?}",
        observed.exit
    );
    assert_eq!(observed.binary, codex_utils_cargo_bin::cargo_bin("codex")?);
    assert!(observed.binary.is_absolute());
    assert!(observed.stdout.len() <= 65_536 && observed.stderr.len() <= 65_536);
    assert_eq!(observed.before, observed.after);

    let framed = observed
        .stdout
        .strip_suffix(b"\n")
        .ok_or_else(|| anyhow!("actual JSONL requires one final LF"))?;
    if framed.is_empty()
        || framed.contains(&b'\r')
        || framed.split(|byte| *byte == b'\n').any(<[u8]>::is_empty)
    {
        return Err(anyhow!("actual JSONL framing refused"));
    }
    let events = framed
        .split(|byte| *byte == b'\n')
        .map(serde_json::from_slice::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let thread_id = dynamic_id(
        events.first().and_then(|event| event.get("thread_id")),
        "thread ID",
    )?;
    let message_id = dynamic_id(
        events
            .get(/*index*/ 2)
            .and_then(|event| event.pointer("/item/id")),
        "message ID",
    )?;
    assert_eq!(
        events,
        vec![
            json!({"type":"thread.started","thread_id":thread_id}),
            json!({"type":"turn.started"}),
            json!({"type":"item.completed","item":{
                "id":message_id,"type":"agent_message","text":MARKER
            }}),
            json!({"type":"turn.completed","usage":{
                "input_tokens":11,"cached_input_tokens":0,
                "cache_write_input_tokens":0,"output_tokens":7,
                "reasoning_output_tokens":0
            }}),
        ]
    );

    let expected = fixture.expectation();
    let config: toml::Value = toml::from_str(std::str::from_utf8(&observed.before.config)?)?;
    let project_key = &expected.cwd;
    assert_eq!(
        serde_json::to_value(config)?,
        json!({
            "cli_auth_credentials_store":"file","forced_login_method":"api",
            "projects":{project_key:{"trust_level":"trusted"}},
            "features":{"hooks":true,"mcp_2026_07_28":false},
            "mcp_servers":{"covenant_effect_probe":{
                "url":observed.mcp_url,"enabled":true,"required":true,
                "startup_timeout_sec":10.0,"tool_timeout_sec":10.0
            }}
        })
    );
    assert!(observed.before.hooks.is_none());

    let model = &observed.model;
    assert_eq!(
        observed.model_target,
        format!("/{}/v1/responses", expected.nonce)
    );
    assert_eq!(model.terminal, Ok(()));
    assert_eq!(model.accepted, model.requests.len());
    assert!((2..=4).contains(&model.accepted));
    for request in &model.requests {
        assert_eq!(request.target, observed.model_target);
        assert_eq!(
            request
                .headers
                .iter()
                .filter(|(name, _)| name == "authorization")
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            vec![format!("Bearer {API_KEY}").as_str()]
        );
        assert!(
            request
                .headers
                .iter()
                .all(|(name, _)| { name != "proxy-authorization" && name != "cookie" })
        );
    }
    let (post, upgrades) = model
        .requests
        .split_last()
        .ok_or_else(|| anyhow!("ordinary model request sequence is empty"))?;
    assert!((1..=3).contains(&upgrades.len()));
    for request in upgrades {
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
        assert!(
            request.headers.iter().any(|(name, value)| {
                name == "upgrade" && value.eq_ignore_ascii_case("websocket")
            })
        );
        assert!(request.headers.iter().any(|(name, value)| {
            name == "connection"
                && value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
        }));
    }
    assert_eq!(post.method, "POST");
    let body = serde_json::from_slice::<Value>(&post.body)?;
    assert_eq!(body["model"], json!("gpt-5.5"));
    assert!(body.get("previous_response_id").is_none());
    assert!(matches!(
        body.get("generate"),
        None | Some(Value::Bool(true))
    ));
    assert_eq!(
        body["input"]
            .as_array()
            .ok_or_else(|| anyhow!("model input missing"))?
            .iter()
            .filter(|item| item["type"] == "message" && item["role"] == "user")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|item| item["type"] == "input_text" && item["text"] == PROMPT)
            .count(),
        1
    );
    let tools = body["tools"]
        .as_array()
        .ok_or_else(|| anyhow!("ordinary tool catalog missing"))?;
    let identities = tools
        .iter()
        .map(tool_identity)
        .collect::<Result<BTreeSet<_>>>()?;
    assert_eq!(tools.len(), identities.len());
    assert_eq!(identities, ordinary_tool_identities());

    let target = format!("/mcp/{}", expected.nonce);
    assert!(
        observed.mcp_url.starts_with("http://127.0.0.1:") && observed.mcp_url.ends_with(&target)
    );
    let mcp = &observed.mcp;
    assert_eq!(mcp.terminal, Ok(()));
    assert_eq!(mcp.accepted, mcp.requests.len());
    assert!((3..=4).contains(&mcp.accepted));
    let mut posts = Vec::new();
    let mut auxiliaries = 0;
    for request in &mcp.requests {
        assert_eq!(request.target, target);
        assert!(request.headers.iter().all(|(name, _)| {
            !matches!(
                name.to_ascii_lowercase().as_str(),
                "authorization" | "proxy-authorization" | "cookie" | "mcp-session-id"
            )
        }));
        match request.method.as_str() {
            "POST" => posts.push(serde_json::from_slice::<Value>(&request.body)?),
            "GET" | "DELETE" => {
                auxiliaries += 1;
                assert!(request.body.is_empty());
            }
            _ => return Err(anyhow!("unaccounted MCP method")),
        }
    }
    assert!(auxiliaries <= 1);
    assert_eq!(
        posts
            .iter()
            .map(|body| body["method"].as_str())
            .collect::<Vec<_>>(),
        vec![
            Some("initialize"),
            Some("notifications/initialized"),
            Some("tools/list")
        ]
    );
    let initialize_id = &posts[0]["id"];
    assert!(initialize_id.is_string() || initialize_id.is_i64());
    let capabilities = &posts[0]["params"]["capabilities"];
    let elicitation = capabilities["elicitation"]
        .as_object()
        .ok_or_else(|| anyhow!("actual elicitation capability missing"))?;
    assert!(
        elicitation.iter().all(|(name, value)| {
            matches!(name.as_str(), "form" | "url") && value == &json!({})
        })
    );
    assert_eq!(capabilities, &json!({"elicitation":elicitation}));
    let version = posts[0]["params"]["clientInfo"]["version"]
        .as_str()
        .filter(|version| !version.is_empty() && version.len() <= 128)
        .ok_or_else(|| anyhow!("actual client version missing"))?;
    assert_eq!(
        posts[0],
        json!({"jsonrpc":"2.0","id":initialize_id,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":capabilities,
            "clientInfo":{"name":"codex-mcp-client","title":"Codex","version":version}
        }})
    );
    assert_eq!(
        posts[1],
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    );
    let list_id = &posts[2]["id"];
    assert!(list_id.is_string() || list_id.is_i64());
    assert_eq!(
        posts[2],
        json!({"jsonrpc":"2.0","id":list_id,"method":"tools/list",
            "params":{"_meta":{"progressToken":0}}})
    );

    assert!(observed.marker.is_none() && observed.effects.is_empty());
    assert_eq!(
        observed.hook_attempts.script_before,
        observed.hook_attempts.script_after
    );
    assert!(observed.hook_attempts.before.is_empty() && observed.hook_attempts.after.is_empty());

    let model_value = json!({"kind":"ordinary_http","peer":{
        "accepted":model.accepted,
        "requests":model.requests.iter().map(request_value).collect::<Vec<_>>(),
        "terminal":null
    }});
    let mcp_value = json!({
        "accepted":mcp.accepted,
        "requests":mcp.requests.iter().map(request_value).collect::<Vec<_>>(),
        "terminal":null
    });
    let receipt_root = PathBuf::from(
        std::env::var_os("COVENANT_EFFECT_RECEIPT_DIR")
            .ok_or_else(|| anyhow!("required owned receipt directory missing"))?,
    );
    assert!(receipt_root.is_absolute());
    assert_eq!(observed.receipt.parent(), Some(receipt_root.as_path()));
    assert_eq!(
        observed.receipt.file_name().and_then(|name| name.to_str()),
        Some("ordinary_mcp_requests_are_observed.first.json")
    );
    let mut raw_receipt = Vec::new();
    File::open(&observed.receipt)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut raw_receipt)?;
    assert!(raw_receipt.len() <= 16 * 1024 * 1024);
    assert_eq!(
        serde_json::from_slice::<Value>(&raw_receipt)?,
        json!({
            "format":"covenant-effects-mcp-observation-v1",
            "case":"ordinary_mcp_requests_are_observed","mode":"ordinary","phase":"first",
            "binary":observed.binary.to_str().ok_or_else(|| anyhow!("binary encoding refused"))?,
            "exit_code":observed.exit.code(),"stdout":observed.stdout,"stderr":observed.stderr,
            "before":{"config":observed.before.config,"hooks":observed.before.hooks},
            "after":{"config":observed.after.config,"hooks":observed.after.hooks},
            "marker":observed.marker,"effects":observed.effects,
            "model":model_value,"mcp":mcp_value,"hook_attempts":observed.hook_attempts
        })
    );
    assert!(case_started.elapsed() <= Duration::from_secs(/*secs*/ 70));
    Ok(())
}

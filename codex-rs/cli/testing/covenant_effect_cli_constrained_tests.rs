//! Windows constrained Codex hook and MCP absence integration test.
use super::HttpRequest;
use super::cli_fixture::Fixture;
use super::proxy;
use anyhow::Result;
use anyhow::anyhow;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
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

fn request_value(request: &HttpRequest) -> Value {
    json!({
        "method":request.method, "target":request.target,
        "headers":request.headers, "body":request.body
    })
}

#[tokio::test(flavor = "current_thread")]
async fn covenant_hook_mcp_effects_are_absent() -> Result<()> {
    let case_started = Instant::now();
    let mut fixture = Fixture::hook_and_mcp().await?;
    let effect_path = fixture
        .effect_directory()
        .to_str()
        .ok_or_else(|| anyhow!("owned effect path encoding refused"))?;
    assert!(effect_path.contains('\'') && effect_path.contains('$'));
    assert!(
        std::fs::read_dir(fixture.effect_directory())?
            .next()
            .is_none()
    );
    let observed = fixture.run_constrained().await?;
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
                "id":message_id,"type":"agent_message","text":proxy::MARKER
            }}),
            json!({"type":"turn.completed","usage":{
                "input_tokens":11,"cached_input_tokens":3,
                "cache_write_input_tokens":2,"output_tokens":7,
                "reasoning_output_tokens":5
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
    let hooks: Value = serde_json::from_slice(
        observed
            .before
            .hooks
            .as_deref()
            .ok_or_else(|| anyhow!("active hook declaration missing"))?,
    )?;
    let command = hooks
        .pointer("/hooks/SessionStart/0/hooks/0/command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("hook command missing"))?;
    assert!(!command.is_empty() && command.encode_utf16().count() <= 8192);
    assert_eq!(
        hooks,
        json!({"hooks":{"SessionStart":[{"matcher":"^startup$",
            "hooks":[{"type":"command","command":command,"timeout":10}]}]}})
    );

    let capture = &observed.model;
    assert_eq!(capture.failure, None);
    assert!((1..=4).contains(&capture.connections));
    assert_eq!(
        (
            capture.connects.len(),
            capture.handshakes.len(),
            capture.qualified_eof_connections
        ),
        (
            capture.connections,
            capture.connections,
            capture.connections
        )
    );
    let authorization = format!("Bearer {}", proxy::API_KEY);
    for (sni, headers) in &capture.handshakes {
        assert_eq!(sni, "api.openai.com");
        assert_eq!(
            headers
                .iter()
                .filter(|(name, _)| name == "authorization")
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            vec![authorization.as_str()]
        );
        assert_eq!(
            headers
                .iter()
                .filter(|(name, _)| name == "host")
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            vec!["api.openai.com"]
        );
    }
    let warmups = capture
        .requests
        .iter()
        .filter(|request| request.get("generate") == Some(&Value::Bool(false)))
        .collect::<Vec<_>>();
    let inferences = capture
        .requests
        .iter()
        .filter(|request| request.get("generate") != Some(&Value::Bool(false)))
        .collect::<Vec<_>>();
    assert!(warmups.len() <= 1);
    assert_eq!(inferences.len(), 1);
    assert_eq!(capture.requests.len(), warmups.len() + 1);
    assert_eq!(
        (capture.warmup_completed, capture.inference_completed),
        (!warmups.is_empty(), true)
    );
    assert!(capture.messages >= capture.requests.len() && capture.messages <= 8);
    let first_tools = capture
        .requests
        .first()
        .and_then(|request| request.get("tools"))
        .ok_or_else(|| anyhow!("actual canonical tools missing"))?;
    for request in &capture.requests {
        assert_eq!(
            (&request["type"], &request["model"]),
            (&json!("response.create"), &json!("gpt-5.5"))
        );
        assert!(matches!(
            request.get("generate"),
            None | Some(Value::Bool(_))
        ));
        assert_eq!(&request["tools"], first_tools);
        let tools = request["tools"]
            .as_array()
            .ok_or_else(|| anyhow!("tool array missing"))?;
        let mut identities = tools
            .iter()
            .map(|tool| (tool["name"].as_str(), tool["type"].as_str()))
            .collect::<Vec<_>>();
        identities.sort_unstable();
        assert_eq!(
            identities,
            vec![
                (Some("apply_patch"), Some("custom")),
                (Some("exec_command"), Some("function"))
            ]
        );
        assert!(tools.iter().all(|tool| tool.get("namespace").is_none()));
    }
    let inference = inferences[0];
    let tail = inference["input"]
        .as_array()
        .ok_or_else(|| anyhow!("inference input missing"))?;
    let resolved = match inference.get("previous_response_id") {
        None => tail.clone(),
        Some(Value::String(id)) => {
            assert_eq!(id, "resp_covenant_owned_warmup");
            assert_eq!(warmups.len(), 1);
            assert!(warmups[0].get("previous_response_id").is_none());
            let mut prefix = warmups[0]["input"]
                .as_array()
                .ok_or_else(|| anyhow!("warmup input missing"))?
                .clone();
            prefix.extend(tail.iter().cloned());
            prefix
        }
        Some(_) => return Err(anyhow!("unknown actual prior response")),
    };
    assert_eq!(capture.inference_input, resolved);
    assert_eq!(
        resolved
            .iter()
            .filter(|item| item["type"] == "message" && item["role"] == "user")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|item| item["type"] == "input_text" && item["text"] == proxy::PROMPT)
            .count(),
        1
    );

    assert_eq!(
        (
            observed.mcp.accepted,
            &observed.mcp.requests,
            observed.mcp.terminal
        ),
        (0, &Vec::<HttpRequest>::new(), Ok(()))
    );
    assert!(observed.marker.is_none() && observed.effects.is_empty());
    assert_eq!(
        observed.hook_attempts.script_before,
        observed.hook_attempts.script_after
    );
    assert!(observed.hook_attempts.before.is_empty() && observed.hook_attempts.after.is_empty());

    let model_value = json!({"kind":"canonical_sc5","capture":{
        "connections":capture.connections,"connects":capture.connects,
        "handshakes":capture.handshakes,"requests":capture.requests,
        "inference_input":capture.inference_input,"warmup_completed":capture.warmup_completed,
        "inference_completed":capture.inference_completed,"messages":capture.messages,
        "qualified_eof_connections":capture.qualified_eof_connections,"failure":capture.failure
    }});
    let mcp_value = json!({
        "accepted":observed.mcp.accepted,
        "requests":observed.mcp.requests.iter().map(request_value).collect::<Vec<_>>(),
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
        Some("covenant_hook_mcp_effects_are_absent.first.json")
    );
    let mut raw_receipt = Vec::new();
    File::open(&observed.receipt)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut raw_receipt)?;
    assert!(raw_receipt.len() <= 16 * 1024 * 1024);
    assert_eq!(
        serde_json::from_slice::<Value>(&raw_receipt)?,
        json!({
            "format":"covenant-effects-absence-observation-v1",
            "case":"covenant_hook_mcp_effects_are_absent","mode":"covenant","phase":"first",
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

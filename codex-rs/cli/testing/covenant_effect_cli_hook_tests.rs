//! Windows ordinary Codex hook integration test.
use super::HttpRequest;
use super::cli_fixture::API_KEY;
use super::cli_fixture::Fixture;
use super::cli_fixture::HookAttemptObservation;
use super::cli_fixture::MARKER;
use super::cli_fixture::PROMPT;
use super::cli_fixture::RunLabel;
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

use super::parse_marker;

#[path = "covenant_effect_cli_hook_witness_tests.rs"]
mod witness;

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

#[cfg(not(feature = "covenant"))]
#[tokio::test(flavor = "current_thread")]
async fn ordinary_hook_effect_is_observed() -> Result<()> {
    let case_started = Instant::now();
    let fixture = Fixture::hook_only()?;
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
    let case = "ordinary_hook_effect_is_observed";
    let mode = "ordinary";
    let phase = "first";
    let first_witness_before = witness::capture(&fixture)?;
    witness::assert_initial_state(&fixture, &first_witness_before)?;
    // run returns only after actual child wait and all owned peer cleanup paths.
    let observed = fixture.run(RunLabel::First).await?;
    let first_witness_after = witness::capture(&fixture)?;
    assert_eq!(
        observed.hook_attempts,
        HookAttemptObservation {
            script_before: first_witness_before.script.clone(),
            script_after: first_witness_after.script.clone(),
            before: first_witness_before.attempts.clone(),
            after: first_witness_after.attempts.clone(),
        }
    );
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
    if framed.is_empty() {
        return Err(anyhow!("actual JSONL stream is empty"));
    }
    let lines = framed.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if lines.iter().any(|line| line.is_empty()) {
        return Err(anyhow!("actual JSONL contains an empty record"));
    }
    if lines.iter().any(|line| line.contains(&b'\r')) {
        return Err(anyhow!("actual JSONL contains a non-LF record terminator"));
    }
    let events = lines
        .into_iter()
        .map(serde_json::from_slice::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let bypass_warning = "`--dangerously-bypass-hook-trust` is enabled. \
            Enabled hooks may run without review for this invocation.";
    let mut thread_id = None;
    let mut warning_ids = Vec::new();
    let mut message_id = None;
    let mut turn_started = false;
    let mut turn_completed = false;
    for event in &events {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("actual event type missing"))?;
        match event_type {
            "thread.started" => {
                let id = dynamic_id(event.get("thread_id"), "actual thread ID")?;
                if thread_id.replace(id).is_some() {
                    return Err(anyhow!("duplicate thread.started event"));
                }
                assert_eq!(event, &json!({"type":"thread.started","thread_id":id}));
            }
            "item.completed" => {
                let item = event
                    .get("item")
                    .and_then(Value::as_object)
                    .ok_or_else(|| anyhow!("actual completed item missing"))?;
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("actual completed item type missing"))?;
                if item_type == "error"
                    && item.get("message").and_then(Value::as_str) == Some(bypass_warning)
                {
                    let id = dynamic_id(item.get("id"), "actual warning item ID")?;
                    assert_eq!(
                        event,
                        &json!({"type":"item.completed","item":{
                            "id":id,"type":"error","message":bypass_warning
                        }})
                    );
                    warning_ids.push(id);
                } else if item_type == "agent_message"
                    && item.get("text").and_then(Value::as_str) == Some(MARKER)
                {
                    let id = dynamic_id(item.get("id"), "actual message item ID")?;
                    if message_id.replace(id).is_some() {
                        return Err(anyhow!("duplicate agent message event"));
                    }
                    assert_eq!(
                        event,
                        &json!({"type":"item.completed","item":{
                            "id":id,"type":"agent_message","text":MARKER
                        }})
                    );
                } else {
                    return Err(anyhow!("unrecognized completed item event"));
                }
            }
            "turn.started" => {
                if turn_started {
                    return Err(anyhow!("duplicate turn.started event"));
                }
                turn_started = true;
                assert_eq!(event, &json!({"type":"turn.started"}));
            }
            "turn.completed" => {
                if turn_completed {
                    return Err(anyhow!("duplicate turn.completed event"));
                }
                turn_completed = true;
                assert_eq!(
                    event,
                    &json!({"type":"turn.completed","usage":{
                        "input_tokens":11,"cached_input_tokens":0,
                        "cache_write_input_tokens":0,"output_tokens":7,
                        "reasoning_output_tokens":0
                    }})
                );
            }
            _ => return Err(anyhow!("unrecognized actual event")),
        }
    }
    assert_eq!(events.len(), 6);
    let thread_id = thread_id.ok_or_else(|| anyhow!("actual thread ID missing"))?;
    assert_eq!(warning_ids.len(), 2);
    let message_id = message_id.ok_or_else(|| anyhow!("actual message ID missing"))?;
    if warning_ids[0] == warning_ids[1]
        || warning_ids[0] == message_id
        || warning_ids[1] == message_id
    {
        return Err(anyhow!("actual completed item IDs are not distinct"));
    }
    if !turn_started || !turn_completed {
        return Err(anyhow!("actual turn boundary event missing"));
    }
    let expected = fixture.expectation();
    let config: toml::Value = toml::from_str(std::str::from_utf8(&observed.before.config)?)?;
    let project_key = &expected.cwd;
    assert_eq!(
        serde_json::to_value(config)?,
        json!({
            "cli_auth_credentials_store":"file","forced_login_method":"api",
            "projects":{project_key:{"trust_level":"trusted"}},
            "features":{"hooks":true,"mcp_2026_07_28":false}
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
    assert_eq!(
        events,
        vec![
            json!({"type":"thread.started","thread_id":thread_id}),
            json!({"type":"item.completed","item":{"id":warning_ids[0],
                "type":"error","message":bypass_warning}}),
            json!({"type":"item.completed","item":{"id":warning_ids[1],
                "type":"error","message":bypass_warning}}),
            json!({"type":"turn.started"}),
            json!({"type":"item.completed","item":{"id":message_id,
                "type":"agent_message","text":MARKER}}),
            json!({"type":"turn.completed","usage":{"input_tokens":11,
                "cached_input_tokens":0,"cache_write_input_tokens":0,
                "output_tokens":7,"reasoning_output_tokens":0}}),
        ]
    );
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
                .all(|(name, _)| name != "proxy-authorization" && name != "cookie")
        );
    }
    let (post, upgrade_prefix) = model
        .requests
        .split_last()
        .ok_or_else(|| anyhow!("ordinary model request sequence is empty"))?;
    if !(1..=3).contains(&upgrade_prefix.len()) {
        return Err(anyhow!(
            "ordinary upgrade prefix is not bounded and nonempty"
        ));
    }
    for request in upgrade_prefix {
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "upgrade" && value.eq_ignore_ascii_case("websocket"))
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
    let inputs = body["input"]
        .as_array()
        .ok_or_else(|| anyhow!("model input missing"))?;
    assert_eq!(
        inputs
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
    let mut actual_tool_identities = BTreeSet::new();
    for tool in tools {
        if !actual_tool_identities.insert(tool_identity(tool)?) {
            return Err(anyhow!("duplicate ordinary tool identity"));
        }
    }
    assert_eq!(tools.len(), 7);
    assert_eq!(actual_tool_identities, ordinary_tool_identities());
    assert!(observed.mcp.is_none() && observed.mcp_url.is_none());
    assert_eq!(observed.effects, vec![fixture.marker_name().to_owned()]);
    let marker_bytes = observed
        .marker
        .as_deref()
        .ok_or_else(|| anyhow!("real hook marker missing"))?;
    let marker = parse_marker(marker_bytes, expected)?;
    let pid = marker["pid"]
        .as_u64()
        .filter(|pid| *pid > 0 && *pid <= u32::MAX.into())
        .ok_or_else(|| anyhow!("positive actual hook PID missing"))?;
    assert_eq!(
        marker,
        json!({"nonce":expected.nonce,"pid":pid,"input":{
            "session_id":thread_id,"transcript_path":null,"cwd":expected.cwd,
            "hook_event_name":"SessionStart","model":"gpt-5.5",
            "permission_mode":"bypassPermissions","source":"startup"
        }})
    );
    assert_eq!(first_witness_before.script, first_witness_after.script);
    assert!(first_witness_before.attempts.is_empty());
    assert_eq!(first_witness_after.attempts.len(), 1);
    let first_attempt = first_witness_after
        .attempts
        .first_key_value()
        .ok_or_else(|| anyhow!("first hook attempt missing"))?;
    assert_eq!(first_attempt.1.as_slice(), marker_bytes);
    let request_value = |request: &HttpRequest| {
        json!({
            "method":request.method, "target":request.target,
            "headers":request.headers, "body":request.body
        })
    };
    let model_value = json!({"kind":"ordinary_http","peer":{
        "accepted":model.accepted,
        "requests":model.requests.iter().map(&request_value).collect::<Vec<_>>(),
        "terminal":null
    }});
    let mcp_value = Value::Null;
    let receipt_root = PathBuf::from(
        std::env::var_os("COVENANT_EFFECT_RECEIPT_DIR")
            .ok_or_else(|| anyhow!("required owned receipt directory missing"))?,
    );
    assert!(receipt_root.is_absolute());
    assert_eq!(observed.receipt.parent(), Some(receipt_root.as_path()));
    assert_eq!(
        observed.receipt.file_name().and_then(|name| name.to_str()),
        Some(format!("{case}.{phase}.json").as_str())
    );
    let mut raw_receipt = Vec::new();
    File::open(&observed.receipt)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut raw_receipt)?;
    assert!(raw_receipt.len() <= 16 * 1024 * 1024);
    let archived: Value = serde_json::from_slice(&raw_receipt)?;
    assert_eq!(
        archived,
        json!({
            "format":"covenant-effects-hook-observation-v2", "case":case,
            "mode":mode, "phase":phase,
            "binary":observed.binary.to_str().ok_or_else(|| anyhow!("binary encoding refused"))?,
            "exit_code":observed.exit.code(), "stdout":observed.stdout, "stderr":observed.stderr,
            "before":{"config":observed.before.config,"hooks":observed.before.hooks},
            "after":{"config":observed.after.config,"hooks":observed.after.hooks},
            "marker":observed.marker,"effects":observed.effects,
            "model":model_value,"mcp":mcp_value,
            "hook_attempts":&observed.hook_attempts
        })
    );
    witness::assert_existing_marker_session(
        &fixture,
        &observed,
        thread_id,
        marker_bytes,
        &first_witness_after,
    )
    .await?;
    assert!(case_started.elapsed() <= Duration::from_secs(/*secs*/ 70));
    Ok(())
}

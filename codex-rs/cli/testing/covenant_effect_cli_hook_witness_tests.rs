//! Independent hook-attempt witness and retained-marker assertions.
use super::super::HttpRequest;
use super::super::cli_fixture::API_KEY;
use super::super::cli_fixture::Fixture;
use super::super::cli_fixture::HookAttemptObservation;
use super::super::cli_fixture::MARKER;
use super::super::cli_fixture::PROMPT;
use super::super::cli_fixture::RunLabel;
use super::super::cli_fixture::RunObservation;
use anyhow::Result;
use anyhow::anyhow;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

const SCRIPT_LIMIT: usize = 32 * 1024;
const ATTEMPT_LIMIT: usize = 16 * 1024;
const RECEIPT_LIMIT: usize = 16 * 1024 * 1024;
const SCRIPT_PREIMAGE_BYTES: usize = 3_663;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WitnessState {
    pub(super) script: Vec<u8>,
    pub(super) attempts: BTreeMap<String, Vec<u8>>,
}

fn read_bounded_regular(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(anyhow!("owned witness type/cap refused"));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(anyhow!("owned witness cap exceeded"));
    }
    Ok(bytes)
}

fn is_attempt_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 45
        && bytes.starts_with(b"attempt-")
        && bytes.ends_with(b".json")
        && bytes[8..40]
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_attempts(directory: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let entries = fs::read_dir(directory)?
        .take(/*n*/ 3)
        .collect::<std::io::Result<Vec<_>>>()?;
    if entries.len() > 2 {
        return Err(anyhow!("owned attempt entry cap exceeded"));
    }
    let mut attempts = BTreeMap::new();
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("attempt name encoding refused"))?;
        if !is_attempt_name(&name) || !entry.file_type()?.is_file() {
            return Err(anyhow!("attempt name/type refused"));
        }
        if attempts
            .insert(name, read_bounded_regular(&entry.path(), ATTEMPT_LIMIT)?)
            .is_some()
        {
            return Err(anyhow!("duplicate attempt name refused"));
        }
    }
    Ok(attempts)
}

fn assert_attempt_directory(fixture: &Fixture) -> Result<()> {
    let directory = fixture.hook_attempt_directory();
    assert!(directory.is_absolute() && fs::symlink_metadata(directory)?.file_type().is_dir());
    assert_eq!(directory.parent(), Some(fixture.root.as_path()));
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("attempt directory name encoding refused"))?;
    assert_eq!(name, "hook attempts '$' \u{03bb} with spaces");
    let effects = fixture.effect_directory();
    assert!(!directory.starts_with(effects) && !effects.starts_with(directory));
    Ok(())
}

pub(super) fn capture(fixture: &Fixture) -> Result<WitnessState> {
    assert_attempt_directory(fixture)?;
    let script = fixture.hook_script_path();
    Ok(WitnessState {
        script: read_bounded_regular(script, SCRIPT_LIMIT)?,
        attempts: read_attempts(fixture.hook_attempt_directory())?,
    })
}

fn attempt_insertion(path: &Path) -> Result<Vec<u8>> {
    let value = path
        .to_str()
        .ok_or_else(|| anyhow!("attempt directory encoding refused"))?;
    if value.encode_utf16().count() > 8192 || value.chars().any(char::is_control) {
        return Err(anyhow!("attempt directory literal refused"));
    }
    let directory = value.replace('\'', "''");
    Ok(format!(
        "\n$attemptName = 'attempt-' + [Guid]::NewGuid().ToString('N') + '.json'\n\
$attemptPath = [IO.Path]::Combine('{directory}', $attemptName)\n\
$attemptFile = $null\n\
try {{\n\
    $attemptFile = [IO.File]::Open($attemptPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)\n\
    $attemptFile.Write($bytes, 0, $bytes.Length)\n\
    $attemptFile.Flush($true)\n\
}} finally {{\n\
    if ($null -ne $attemptFile) {{ $attemptFile.Dispose() }}\n\
}}\n"
    )
    .into_bytes())
}

fn unique_offset(haystack: &[u8], needle: &[u8], label: &str) -> Result<usize> {
    let offsets = haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == needle).then_some(offset))
        .collect::<Vec<_>>();
    if offsets.len() != 1 {
        return Err(anyhow!("{label} is not unique"));
    }
    Ok(offsets[0])
}

pub(super) fn assert_initial_state(fixture: &Fixture, state: &WitnessState) -> Result<()> {
    assert_eq!(state.attempts, BTreeMap::new());
    let raw_directory = fixture
        .hook_attempt_directory()
        .to_str()
        .ok_or_else(|| anyhow!("attempt directory encoding refused"))?;
    let escaped_directory = raw_directory.replace('\'', "''");
    assert!(
        escaped_directory.contains("''")
            && escaped_directory.contains('$')
            && escaped_directory.contains(' ')
            && escaped_directory
                .chars()
                .any(|character| !character.is_ascii())
    );
    let insertion = attempt_insertion(fixture.hook_attempt_directory())?;
    let mut preimage = vec![0xef, 0xbb, 0xbf];
    preimage.extend_from_slice(super::super::hook_command::SCRIPT.as_bytes());
    assert_eq!(preimage.len(), SCRIPT_PREIMAGE_BYTES);
    let boundary = b"if ($bytes.Length -gt 16384) { Refuse-HookInput }\n";
    let insertion_offset =
        unique_offset(&preimage, boundary, "script insertion boundary")? + boundary.len();
    let marker_open = b"try { $file = [IO.File]::Open($Marker, [IO.FileMode]::CreateNew";
    let marker_offset = unique_offset(&preimage, marker_open, "marker CreateNew boundary")?;
    assert_eq!(insertion_offset, marker_offset);
    let mut expected = preimage[..insertion_offset].to_vec();
    expected.extend_from_slice(&insertion);
    expected.extend_from_slice(&preimage[insertion_offset..]);
    assert_eq!(state.script, expected);
    let inserted_at = unique_offset(&state.script, &insertion, "hook attempt insertion")?;
    let mut restored = state.script[..inserted_at].to_vec();
    restored.extend_from_slice(&state.script[inserted_at + insertion.len()..]);
    assert_eq!(restored, preimage);
    Ok(())
}

fn assert_transcript(stdout: &[u8]) -> Result<String> {
    let framed = stdout
        .strip_suffix(b"\n")
        .ok_or_else(|| anyhow!("second actual JSONL requires one final LF"))?;
    if framed.is_empty() {
        return Err(anyhow!("second actual JSONL stream is empty"));
    }
    let lines = framed.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if lines.iter().any(|line| line.is_empty()) {
        return Err(anyhow!("second actual JSONL contains an empty record"));
    }
    if lines.iter().any(|line| line.contains(&b'\r')) {
        return Err(anyhow!(
            "second actual JSONL contains a non-LF record terminator"
        ));
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
    for event in &events {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("second actual event type missing"))?;
        match event_type {
            "thread.started" => {
                let id = super::dynamic_id(event.get("thread_id"), "second actual thread ID")?;
                if thread_id.replace(id.to_owned()).is_some() {
                    return Err(anyhow!("duplicate second thread.started event"));
                }
                assert_eq!(event, &json!({"type":"thread.started","thread_id":id}));
            }
            "item.completed" => {
                let item = event
                    .get("item")
                    .and_then(Value::as_object)
                    .ok_or_else(|| anyhow!("second actual completed item missing"))?;
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("second actual completed item type missing"))?;
                if item_type == "error"
                    && item.get("message").and_then(Value::as_str) == Some(bypass_warning)
                {
                    let id = super::dynamic_id(item.get("id"), "second actual warning item ID")?;
                    assert_eq!(
                        event,
                        &json!({"type":"item.completed","item":{
                            "id":id,"type":"error","message":bypass_warning
                        }})
                    );
                    warning_ids.push(id.to_owned());
                } else if item_type == "agent_message"
                    && item.get("text").and_then(Value::as_str) == Some(MARKER)
                {
                    let id = super::dynamic_id(item.get("id"), "second actual message item ID")?;
                    if message_id.replace(id.to_owned()).is_some() {
                        return Err(anyhow!("duplicate second agent message event"));
                    }
                    assert_eq!(
                        event,
                        &json!({"type":"item.completed","item":{
                            "id":id,"type":"agent_message","text":MARKER
                        }})
                    );
                } else {
                    return Err(anyhow!("unrecognized second completed item event"));
                }
            }
            "turn.started" => {
                assert_eq!(event, &json!({"type":"turn.started"}));
            }
            "turn.completed" => {
                assert_eq!(
                    event,
                    &json!({"type":"turn.completed","usage":{
                        "input_tokens":11,"cached_input_tokens":0,
                        "cache_write_input_tokens":0,"output_tokens":7,
                        "reasoning_output_tokens":0
                    }})
                );
            }
            _ => return Err(anyhow!("unrecognized second actual event")),
        }
    }
    assert_eq!(events.len(), 6);
    let thread_id = thread_id.ok_or_else(|| anyhow!("second actual thread ID missing"))?;
    assert_eq!(warning_ids.len(), 2);
    let message_id = message_id.ok_or_else(|| anyhow!("second actual message ID missing"))?;
    if warning_ids[0] == warning_ids[1]
        || warning_ids[0] == message_id
        || warning_ids[1] == message_id
    {
        return Err(anyhow!("second completed item IDs are not distinct"));
    }
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
    Ok(thread_id)
}

fn assert_model(fixture: &Fixture, observed: &RunObservation) -> Result<Value> {
    let model = &observed.model;
    assert_eq!(
        observed.model_target,
        format!("/{}/v1/responses", fixture.expectation().nonce)
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
        .ok_or_else(|| anyhow!("second ordinary model request sequence is empty"))?;
    if !(1..=3).contains(&upgrade_prefix.len()) {
        return Err(anyhow!("second ordinary upgrade prefix is not bounded"));
    }
    for request in upgrade_prefix {
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
    let inputs = body["input"]
        .as_array()
        .ok_or_else(|| anyhow!("second model input missing"))?;
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
        .ok_or_else(|| anyhow!("second ordinary tool catalog missing"))?;
    let mut actual_tool_identities = std::collections::BTreeSet::new();
    for tool in tools {
        if !actual_tool_identities.insert(super::tool_identity(tool)?) {
            return Err(anyhow!("duplicate second ordinary tool identity"));
        }
    }
    assert_eq!(tools.len(), 7);
    assert_eq!(actual_tool_identities, super::ordinary_tool_identities());
    let request_value = |request: &HttpRequest| {
        json!({
            "method":request.method, "target":request.target,
            "headers":request.headers, "body":request.body
        })
    };
    Ok(json!({"kind":"ordinary_http","peer":{
        "accepted":model.accepted,
        "requests":model.requests.iter().map(&request_value).collect::<Vec<_>>(),
        "terminal":null
    }}))
}

pub(super) async fn assert_existing_marker_session(
    fixture: &Fixture,
    first: &RunObservation,
    first_thread: &str,
    first_marker: &[u8],
    first_after: &WitnessState,
) -> Result<()> {
    let second_before = capture(fixture)?;
    assert_eq!(&second_before, first_after);
    let observed = fixture.run(RunLabel::ExistingMarker).await?;
    let second_after = capture(fixture)?;
    assert_eq!(
        observed.hook_attempts,
        HookAttemptObservation {
            script_before: second_before.script.clone(),
            script_after: second_after.script.clone(),
            before: second_before.attempts.clone(),
            after: second_after.attempts.clone(),
        }
    );
    assert!(
        observed.exit.success(),
        "second native failure: {:?}",
        observed.exit
    );
    assert_eq!(observed.binary, first.binary);
    assert!(observed.stdout.len() <= 65_536 && observed.stderr.len() <= 65_536);
    assert_eq!(observed.before, observed.after);
    assert_eq!(observed.before, first.before);
    let second_thread = assert_transcript(&observed.stdout)?;
    assert_ne!(second_thread, first_thread);
    let model_value = assert_model(fixture, &observed)?;
    assert!(observed.mcp.is_none() && observed.mcp_url.is_none());
    assert_eq!(observed.effects, vec![fixture.marker_name().to_owned()]);
    let marker_bytes = observed
        .marker
        .as_deref()
        .ok_or_else(|| anyhow!("retained real hook marker missing"))?;
    assert_eq!(marker_bytes, first_marker);
    assert_eq!(second_after.script, second_before.script);
    assert_eq!(
        second_after.attempts.len(),
        second_before.attempts.len() + 1
    );
    for (name, raw) in &second_before.attempts {
        assert_eq!(second_after.attempts.get(name), Some(raw));
    }
    let new_attempts = second_after
        .attempts
        .iter()
        .filter(|(name, _)| !second_before.attempts.contains_key(*name))
        .collect::<Vec<_>>();
    assert_eq!(new_attempts.len(), 1);
    let (_, new_raw) = new_attempts[0];
    let attempt = super::super::parse_marker(new_raw, fixture.expectation())?;
    let pid = attempt["pid"]
        .as_u64()
        .filter(|pid| *pid > 0 && *pid <= u32::MAX.into())
        .ok_or_else(|| anyhow!("positive second hook attempt PID missing"))?;
    let expected = fixture.expectation();
    assert_eq!(
        attempt,
        json!({"nonce":expected.nonce,"pid":pid,"input":{
            "session_id":second_thread,"transcript_path":null,"cwd":expected.cwd,
            "hook_event_name":"SessionStart","model":"gpt-5.5",
            "permission_mode":"bypassPermissions","source":"startup"
        }})
    );
    let receipt_root = PathBuf::from(
        std::env::var_os("COVENANT_EFFECT_RECEIPT_DIR")
            .ok_or_else(|| anyhow!("required owned receipt directory missing"))?,
    );
    assert_eq!(observed.receipt.parent(), Some(receipt_root.as_path()));
    assert_eq!(
        observed.receipt.file_name().and_then(|name| name.to_str()),
        Some("ordinary_hook_effect_is_observed.existing-marker.json")
    );
    let raw_receipt = read_bounded_regular(&observed.receipt, RECEIPT_LIMIT)?;
    let archived: Value = serde_json::from_slice(&raw_receipt)?;
    assert_eq!(
        archived,
        json!({
            "format":"covenant-effects-hook-observation-v2",
            "case":"ordinary_hook_effect_is_observed","mode":"ordinary",
            "phase":"existing-marker",
            "binary":observed.binary.to_str().ok_or_else(|| anyhow!("binary encoding refused"))?,
            "exit_code":observed.exit.code(),"stdout":observed.stdout,"stderr":observed.stderr,
            "before":{"config":observed.before.config,"hooks":observed.before.hooks},
            "after":{"config":observed.after.config,"hooks":observed.after.hooks},
            "marker":observed.marker,"effects":observed.effects,
            "model":model_value,"mcp":Value::Null,
            "hook_attempts":&observed.hook_attempts
        })
    );
    Ok(())
}

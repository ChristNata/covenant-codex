//! Unregistered draft: marker parsing only; no shell or process execution.
use super::FixtureFailure;
use super::MarkerExpectation;
use super::parse_marker;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

fn expected() -> MarkerExpectation {
    MarkerExpectation {
        nonce: "owned-7d32721b".to_owned(),
        cwd: "C:/owned/quote's $name/work".to_owned(),
        model: "gpt-5.5".to_owned(),
    }
}

fn record() -> Value {
    json!({
        "nonce": "owned-7d32721b", "pid": 4321,
        "input": {
            "session_id": "session-observed-from-stdin",
            "transcript_path": null,
            "cwd": "C:/owned/quote's $name/work",
            "hook_event_name": "SessionStart", "model": "gpt-5.5",
            "permission_mode": "default", "source": "startup"
        }
    })
}

#[test]
fn marker_preserves_the_complete_observed_record() {
    let mut value = record();
    value["input"]["session_id"] = json!("a-different-session");
    value["input"]["transcript_path"] = json!("C:/owned/transcript.jsonl");
    assert_eq!(
        parse_marker(&serde_json::to_vec(&value).unwrap(), &expected()).unwrap(),
        value
    );
}

#[test]
fn marker_refuses_wrong_nonce_event_source_cwd_model_or_process_identity() {
    for (pointer, replacement) in [
        ("/nonce", json!("stale-marker")),
        ("/pid", json!(0)),
        ("/pid", json!(-1)),
        ("/pid", json!(1.5)),
        ("/pid", json!("4321")),
        ("/input/hook_event_name", json!("PreToolUse")),
        ("/input/source", json!("resume")),
        ("/input/cwd", json!("C:/unrelated")),
        ("/input/model", json!("unselected-model")),
        ("/input/session_id", json!("")),
        ("/input/transcript_path", json!({})),
    ] {
        let mut value = record();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_eq!(
            parse_marker(&serde_json::to_vec(&value).unwrap(), &expected()),
            Err(FixtureFailure::Marker),
            "accepted substitution at {pointer}"
        );
    }
}

#[test]
fn marker_refuses_incomplete_extra_or_non_utf8_records() {
    let mut missing = record();
    missing.as_object_mut().unwrap().remove("pid");
    let mut extra = record();
    extra["unexpected"] = json!(true);
    let mut nested = record();
    nested["input"]["unexpected"] = json!(true);
    let mut duplicate = serde_json::to_vec(&record()).unwrap();
    duplicate.extend_from_slice(b"\n{}");
    let serialized = serde_json::to_string(&record()).unwrap();
    let duplicate_key = format!("{{\"nonce\":\"stale\",{}", &serialized[1..]).into_bytes();
    for bytes in [
        serde_json::to_vec(&missing).unwrap(),
        serde_json::to_vec(&extra).unwrap(),
        serde_json::to_vec(&nested).unwrap(),
        duplicate,
        duplicate_key,
        b"{\"pid\":1".to_vec(),
        vec![0xff],
    ] {
        assert_eq!(
            parse_marker(&bytes, &expected()),
            Err(FixtureFailure::Marker)
        );
    }
}

#[test]
fn marker_byte_limit_applies_before_json_parsing() {
    let value = record();
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut bounded = vec![b' '; 16_384 - bytes.len()];
    bounded.extend_from_slice(&bytes);
    assert_eq!(parse_marker(&bounded, &expected()).unwrap(), value);
    bounded.push(b' ');
    assert_eq!(
        parse_marker(&bounded, &expected()),
        Err(FixtureFailure::Limit)
    );
}

#[test]
fn marker_failure_does_not_echo_observed_contents() {
    let mut value = record();
    value["nonce"] = json!("private-observed-canary");
    let error = parse_marker(&serde_json::to_vec(&value).unwrap(), &expected()).unwrap_err();
    assert_eq!(error.to_string(), "owned effect fixture refused");
    assert!(!format!("{error:?}").contains("private-observed-canary"));
}

//! Unregistered draft: supplied request/response behavior, without live transport.
use super::FixtureFailure;
use super::HttpRequest;
use super::ResponsesExchange;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

fn post(target: &str, body: Value) -> HttpRequest {
    HttpRequest {
        method: "POST".to_owned(),
        target: target.to_owned(),
        headers: vec![("content-type".to_owned(), "application/json".to_owned())],
        body: serde_json::to_vec(&body).unwrap(),
    }
}

fn model_request() -> HttpRequest {
    let mut request = post(
        "/v1/responses",
        json!({
            "model":"gpt-5.5",
            "input":[{"type":"message", "role":"user", "content":[
                {"type":"input_text", "text":"owned completion prompt"}
            ]}],
            "tools":[{"type":"function", "name":"ordinary-control-tool", "parameters":{"type":"object"}}]
        }),
    );
    request.headers.push((
        "authorization".to_owned(),
        "Bearer synthetic-api-key".to_owned(),
    ));
    request
}

fn responses() -> ResponsesExchange {
    ResponsesExchange::new(
        "/v1/responses".to_owned(),
        "synthetic-api-key".to_owned(),
        "owned completion prompt".to_owned(),
        "owned completion marker".to_owned(),
    )
}

#[test]
fn ordinary_responses_records_upgrade_and_returns_real_completion_sse() {
    let mut exchange = responses();
    let mut upgrade = model_request();
    upgrade.method = "GET".to_owned();
    upgrade.body.clear();
    upgrade.headers.extend([
        ("connection".to_owned(), "Upgrade".to_owned()),
        ("upgrade".to_owned(), "websocket".to_owned()),
    ]);
    let reply = exchange.respond(upgrade.clone()).unwrap();
    assert_eq!(
        (reply.status, reply.content_type, reply.body),
        (426, None, Vec::new())
    );
    assert!(!exchange.semantic_complete());
    let request = model_request();
    let reply = exchange.respond(request.clone()).unwrap();
    assert_eq!(
        (reply.status, reply.content_type),
        (200, Some("text/event-stream"))
    );
    let text = std::str::from_utf8(&reply.body).unwrap();
    let events: Vec<Value> = text
        .split("\n\n")
        .filter(|part| !part.is_empty())
        .map(|part| serde_json::from_str(part.strip_prefix("data: ").unwrap()).unwrap())
        .collect();
    assert_eq!(
        events,
        vec![
            json!({"type":"response.created", "response":{"id":"resp_effects"}}),
            json!({"type":"response.output_item.done", "item":{
                "type":"message", "role":"assistant", "id":"msg_effects",
                "content":[{"type":"output_text", "text":"owned completion marker"}]
            }}),
            json!({"type":"response.completed", "response":{"id":"resp_effects", "usage":{
                "input_tokens":11, "output_tokens":7, "total_tokens":18
            }}}),
        ]
    );
    assert_eq!(exchange.requests(), &[upgrade, request]);
    assert!(exchange.semantic_complete());
}

#[test]
fn ordinary_responses_cannot_complete_on_wrong_prompt_model_auth_or_endpoint() {
    for change in 0..4 {
        let mut request = model_request();
        match change {
            0 => {
                let mut body: Value = serde_json::from_slice(&request.body).unwrap();
                body["input"] = json!([]);
                request.body = serde_json::to_vec(&body).unwrap();
            }
            1 => {
                let mut body: Value = serde_json::from_slice(&request.body).unwrap();
                body["model"] = json!("other");
                request.body = serde_json::to_vec(&body).unwrap();
            }
            2 => request.headers.retain(|(name, _)| name != "authorization"),
            3 => request.target = "/other".to_owned(),
            _ => unreachable!(),
        }
        let mut exchange = responses();
        assert_eq!(
            exchange.respond(request.clone()),
            Err(FixtureFailure::Protocol)
        );
        assert_eq!(exchange.requests(), &[request]);
        assert!(!exchange.semantic_complete());
    }
}

#[test]
fn ordinary_responses_refuses_duplicate_semantic_completion() {
    let mut exchange = responses();
    let request = model_request();
    exchange.respond(request.clone()).unwrap();
    assert!(exchange.semantic_complete());
    assert_eq!(
        exchange.respond(request.clone()),
        Err(FixtureFailure::Protocol)
    );
    assert_eq!(exchange.requests(), &[request.clone(), request]);
    assert!(!exchange.semantic_complete());
}

#[test]
fn ordinary_responses_counts_upgrade_attempts_toward_its_cap() {
    let mut exchange = responses();
    let mut request = model_request();
    request.method = "GET".to_owned();
    request.body.clear();
    request.headers.extend([
        ("connection".to_owned(), "Upgrade".to_owned()),
        ("upgrade".to_owned(), "websocket".to_owned()),
    ]);
    for _ in 0..4 {
        exchange.respond(request.clone()).unwrap();
    }
    assert_eq!(
        exchange.respond(request.clone()),
        Err(FixtureFailure::Limit)
    );
    assert_eq!(exchange.requests(), vec![request; 4]);
    assert!(!exchange.semantic_complete());
}

//! Unregistered draft: supplied request/response behavior, without live transport.
use super::FixtureFailure;
use super::HttpRequest;
use super::McpExchange;
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

fn initialize(id: Value) -> HttpRequest {
    post(
        "/mcp/nonce",
        json!({
            "jsonrpc":"2.0", "id":id, "method":"initialize",
            "params": {"protocolVersion":"2025-06-18", "capabilities":{},
                "clientInfo":{"name":"actual-client-control", "version":"0.1"}}
        }),
    )
}

fn initialized() -> HttpRequest {
    post(
        "/mcp/nonce",
        json!({"jsonrpc":"2.0", "method":"notifications/initialized"}),
    )
}

fn list_tools() -> HttpRequest {
    post(
        "/mcp/nonce",
        json!({"jsonrpc":"2.0", "id":91, "method":"tools/list", "params":{}}),
    )
}

#[test]
fn mcp_completes_real_legacy_sequence_and_preserves_request_ids_and_inputs() {
    for id in [json!(37), json!("observed-id")] {
        let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
        let first = initialize(id.clone());
        let reply = exchange.respond(first.clone()).unwrap();
        assert_eq!(
            (
                reply.status,
                reply.content_type,
                serde_json::from_slice::<Value>(&reply.body).unwrap()
            ),
            (
                200,
                Some("application/json"),
                json!({"jsonrpc":"2.0", "id":id, "result":{
                    "protocolVersion":"2025-06-18", "capabilities":{"tools":{}},
                    "serverInfo":{"name":"covenant-effects-fixture", "version":"1"}
                }})
            )
        );
        assert!(!exchange.discovery_complete());
        let notification = initialized();
        let reply = exchange.respond(notification.clone()).unwrap();
        assert_eq!(
            (reply.status, reply.content_type, reply.body),
            (202, None, Vec::new())
        );
        assert!(!exchange.discovery_complete());
        let discovery = list_tools();
        let reply = exchange.respond(discovery.clone()).unwrap();
        assert_eq!(
            (
                reply.status,
                reply.content_type,
                serde_json::from_slice::<Value>(&reply.body).unwrap()
            ),
            (
                200,
                Some("application/json"),
                json!({"jsonrpc":"2.0", "id":91, "result":{"tools":[]}})
            )
        );
        assert!(exchange.discovery_complete());
        assert_eq!(exchange.requests(), &[first, notification, discovery]);
    }
}

#[test]
fn mcp_auxiliary_http_requests_are_counted_but_do_not_prove_discovery() {
    let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
    let mut seen = Vec::new();
    for method in ["GET", "DELETE"] {
        let request = HttpRequest {
            method: method.to_owned(),
            target: "/mcp/nonce".to_owned(),
            headers: Vec::new(),
            body: Vec::new(),
        };
        let reply = exchange.respond(request.clone()).unwrap();
        assert_eq!(
            (reply.status, reply.content_type, reply.body),
            (405, None, Vec::new())
        );
        seen.push(request);
    }
    assert_eq!(exchange.requests(), seen);
    assert!(!exchange.discovery_complete());
}

#[test]
fn mcp_rejects_invalid_order_endpoint_ids_or_authentication_and_stays_failed() {
    let mut wrong_path = initialize(json!(1));
    wrong_path.target = "/mcp/unissued".to_owned();
    let mut auth = initialize(json!(1));
    auth.headers.push((
        "authorization".to_owned(),
        "Bearer private-canary".to_owned(),
    ));
    for request in [
        initialized(),
        list_tools(),
        initialize(json!(null)),
        wrong_path,
        auth,
    ] {
        let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
        assert_eq!(
            exchange.respond(request.clone()),
            Err(FixtureFailure::Protocol)
        );
        assert_eq!(exchange.requests(), &[request]);
        assert_eq!(
            exchange.respond(initialize(json!(2))),
            Err(FixtureFailure::Protocol)
        );
        assert!(!exchange.discovery_complete());
    }
}

#[test]
fn mcp_does_not_let_late_invalid_requests_erase_a_completed_exchange() {
    let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
    for request in [initialize(json!(1)), initialized(), list_tools()] {
        exchange.respond(request).unwrap();
    }
    let invalid = post(
        "/mcp/nonce",
        json!({"jsonrpc":"2.0", "id":92, "method":"tools/call"}),
    );
    assert_eq!(
        exchange.respond(invalid.clone()),
        Err(FixtureFailure::Protocol)
    );
    assert_eq!(exchange.requests().last(), Some(&invalid));
    assert!(!exchange.discovery_complete());
}

#[test]
fn mcp_request_cap_includes_auxiliary_requests() {
    let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
    let request = HttpRequest {
        method: "GET".to_owned(),
        target: "/mcp/nonce".to_owned(),
        headers: Vec::new(),
        body: Vec::new(),
    };
    for _ in 0..8 {
        exchange.respond(request.clone()).unwrap();
    }
    assert_eq!(
        exchange.respond(request.clone()),
        Err(FixtureFailure::Limit)
    );
    assert_eq!(exchange.requests(), vec![request; 8]);
    assert!(!exchange.discovery_complete());
}

#[test]
fn mcp_accepts_codex_client_metadata_and_supported_list_params() {
    for elicitation in [json!({}), json!({"form":{}, "url":{}})] {
        for list in [
            json!({"jsonrpc":"2.0", "id":"list-id", "method":"tools/list", "params":{}}),
            json!({"jsonrpc":"2.0", "id":"list-id", "method":"tools/list"}),
            json!({"jsonrpc":"2.0", "id":"list-id", "method":"tools/list",
                "params":{"_meta":{"progressToken":0}}}),
        ] {
            let mut first = initialize(json!(37));
            let mut body: Value = serde_json::from_slice(&first.body).unwrap();
            body["params"]["capabilities"] = json!({"elicitation":elicitation});
            body["params"]["clientInfo"] = json!({
                "name":"codex-mcp-client", "version":"0.153.4", "title":"Codex"
            });
            first.body = serde_json::to_vec(&body).unwrap();
            let notification = initialized();
            let discovery = post("/mcp/nonce", list);
            let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
            exchange.respond(first.clone()).unwrap();
            exchange.respond(notification.clone()).unwrap();
            let reply = exchange.respond(discovery.clone()).unwrap();
            assert_eq!(
                (
                    reply.status,
                    reply.content_type,
                    serde_json::from_slice::<Value>(&reply.body).unwrap()
                ),
                (
                    200,
                    Some("application/json"),
                    json!({"jsonrpc":"2.0", "id":"list-id", "result":{"tools":[]}})
                )
            );
            assert_eq!(exchange.requests(), &[first, notification, discovery]);
            assert!(exchange.discovery_complete());
        }
    }
}

#[test]
fn mcp_rejects_unreviewed_tool_list_metadata() {
    for params in [
        json!({"_meta":{}}),
        json!({"_meta":{"progressToken":1}}),
        json!({"_meta":{"progressToken":"unissued"}}),
        json!({"_meta":{"progressToken":0,"extra":true}}),
        json!({"_meta":{"progressToken":0},"cursor":"unissued"}),
    ] {
        let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
        exchange.respond(initialize(json!(1))).unwrap();
        exchange.respond(initialized()).unwrap();
        let request = post(
            "/mcp/nonce",
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":params}),
        );
        assert_eq!(
            exchange.respond(request.clone()),
            Err(FixtureFailure::Protocol)
        );
        assert_eq!(exchange.requests().last(), Some(&request));
        assert!(!exchange.discovery_complete());
    }
}

#[test]
fn mcp_codex_metadata_does_not_relax_required_protocol_identity_or_method() {
    let mut body: Value = serde_json::from_slice(&initialize(json!(37)).body).unwrap();
    body["params"]["capabilities"] = json!({"elicitation":{"form":{}, "url":{}}});
    body["params"]["clientInfo"] = json!({
        "name":"codex-mcp-client", "version":"0.153.4", "title":"Codex"
    });
    for (pointer, invalid) in [
        ("/jsonrpc", json!("1.0")),
        ("/id", json!(null)),
        ("/id", json!({"unissued":1})),
        ("/method", json!("tools/list")),
        ("/method", json!("tools/call")),
        ("/params/protocolVersion", json!("unreviewed-version")),
    ] {
        let mut changed = body.clone();
        *changed.pointer_mut(pointer).unwrap() = invalid;
        let request = post("/mcp/nonce", changed);
        let mut exchange = McpExchange::new("/mcp/nonce".to_owned());
        assert_eq!(
            exchange.respond(request.clone()),
            Err(FixtureFailure::Protocol)
        );
        assert_eq!(exchange.requests(), &[request]);
        assert_eq!(
            exchange.respond(post("/mcp/nonce", body.clone())),
            Err(FixtureFailure::Protocol)
        );
        assert!(!exchange.discovery_complete());
    }
}

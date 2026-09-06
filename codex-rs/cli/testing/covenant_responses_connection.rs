//! Actual scripted WebSocket exchange; original request, tool and event checks are retained.
use super::API_KEY;
use super::Capture;
use super::INFERENCE_ID;
use super::MARKER;
use super::MESSAGE_LIMIT;
use super::PROMPT;
use super::WARMUP_ID;
use super::captured;
use super::observation::DeliveredKind;
use super::observation::TrafficBudget;
use super::transport::ConnectionObservation;
use super::transport::ObservedStream;
use super::transport::read_header;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use futures::SinkExt;
use futures::StreamExt;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_hdr_async_with_config;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::handshake::server::Response;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

pub(super) async fn connection(
    mut stream: TcpStream,
    acceptor: TlsAcceptor,
    capture: Arc<Mutex<Capture>>,
    budget: Arc<Mutex<TrafficBudget>>,
) -> Result<Arc<ConnectionObservation>> {
    let connect = read_header(&mut stream).await?;
    captured(&capture).connects.push(connect.clone());
    ensure!(
        connect.starts_with(b"CONNECT api.openai.com:443 HTTP/1.1\r\n"),
        "unexpected authority"
    );
    stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    stream.flush().await?;
    let observation = Arc::new(ConnectionObservation::new(budget));
    let raw = ObservedStream::raw_tls(stream, Arc::clone(&observation));
    let mut tls = acceptor.accept(raw).await?;
    let sni = tls
        .get_ref()
        .1
        .server_name()
        .unwrap_or_default()
        .to_string();
    ensure!(sni == "api.openai.com", "unexpected SNI");
    let prefix = read_header(&mut tls).await?;
    let observed = Arc::clone(&capture);
    let callback = move |request: &Request, response: Response| {
        let headers = request
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("<nontext>").to_string(),
                )
            })
            .collect();
        captured(&observed).handshakes.push((sni.clone(), headers));
        let expected_auth = format!("Bearer {API_KEY}");
        let accepted = request.method() == http::Method::GET
            && request
                .uri()
                .path_and_query()
                .is_some_and(|value| value.as_str() == "/v1/responses")
            && request.headers().get_all("host").iter().count() == 1
            && request
                .headers()
                .get("host")
                .is_some_and(|value| value == "api.openai.com")
            && request.headers().get_all("authorization").iter().count() == 1
            && request
                .headers()
                .get("authorization")
                .is_some_and(|value| value == expected_auth.as_str());
        if accepted {
            Ok(response)
        } else {
            captured(&observed)
                .failure
                .get_or_insert("unexpected upgrade or synthetic auth");
            let mut response = http::Response::new(Some("owned request refused".to_string()));
            *response.status_mut() = http::StatusCode::BAD_REQUEST;
            Err(response)
        }
    };
    let config = WebSocketConfig::default()
        .read_buffer_size(8192)
        .max_message_size(Some(MESSAGE_LIMIT))
        .max_frame_size(Some(MESSAGE_LIMIT));
    // Default server extensions remain disabled; no deflate is negotiated.
    let mut socket = accept_hdr_async_with_config(
        ObservedStream::websocket(tls, prefix, Arc::clone(&observation)),
        callback,
        Some(config),
    )
    .await?;
    loop {
        let message = match socket.next().await {
            Some(Ok(message)) => message,
            Some(Err(WebSocketError::Io(error)))
                if error.kind() == std::io::ErrorKind::UnexpectedEof
                    && observation.missing_close_notify_observed() =>
            {
                // Qualification is deferred until the actual child and every task are joined.
                return Ok(observation);
            }
            Some(Err(_)) | None => return Err(anyhow!("owned WebSocket terminal unqualified")),
        };
        let kind = match &message {
            Message::Text(_) => DeliveredKind::Text,
            Message::Binary(_) => DeliveredKind::Binary,
            Message::Ping(_) => DeliveredKind::Ping,
            Message::Pong(_) => DeliveredKind::Pong,
            Message::Close(_) => DeliveredKind::Close,
            Message::Frame(_) => return Err(anyhow!("unexpected WebSocket frame")),
        };
        observation.delivered(kind)?;
        {
            let mut state = captured(&capture);
            state.messages += 1;
            ensure!(state.messages <= 8, "message cap exceeded");
        }
        match message {
            Message::Text(text) => {
                ensure!(text.len() <= MESSAGE_LIMIT, "request cap exceeded");
                let request: Value =
                    serde_json::from_str(&text).map_err(|_| anyhow!("request JSON refused"))?;
                let warmup = observe_request(&capture, request)?;
                let id = if warmup { WARMUP_ID } else { INFERENCE_ID };
                socket
                    .send(Message::Text(
                        json!({"type":"response.created","response":{"id":id}})
                            .to_string()
                            .into(),
                    ))
                    .await?;
                if !warmup {
                    socket
                        .send(Message::Text(
                            json!({"type":"response.output_item.done","item":{
                                "type":"message","role":"assistant","id":"msg_covenant_owned",
                                "content":[{"type":"output_text","text":MARKER}]
                            }})
                            .to_string()
                            .into(),
                        ))
                        .await?;
                }
                // Source event conventions: core/tests/common/responses.rs:737-799.
                let usage = if warmup {
                    json!({"input_tokens":0,"output_tokens":0,"total_tokens":0})
                } else {
                    json!({"input_tokens":11,"input_tokens_details":{"cached_tokens":3,"cache_write_tokens":2},
                        "output_tokens":7,"output_tokens_details":{"reasoning_tokens":5},"total_tokens":18})
                };
                socket
                    .send(Message::Text(
                        json!({"type":"response.completed","response":{"id":id,"usage":usage}})
                            .to_string()
                            .into(),
                    ))
                    .await?;
                let mut state = captured(&capture);
                if warmup {
                    state.warmup_completed = true;
                } else {
                    state.inference_completed = true;
                }
            }
            Message::Ping(value) => socket.send(Message::Pong(value)).await?,
            Message::Pong(_) => {}
            Message::Close(_) => {}
            Message::Binary(_) | Message::Frame(_) => {
                return Err(anyhow!("unexpected WebSocket frame"));
            }
        }
    }
}

fn observe_request(capture: &Mutex<Capture>, request: Value) -> Result<bool> {
    let mut state = captured(capture);
    ensure!(state.requests.len() < 8, "request count cap exceeded");
    state.requests.push(request.clone()); // Retain the original even when validation fails.
    ensure!(
        request["type"] == "response.create" && request["model"] == "gpt-5.5",
        "request identity refused"
    );
    validate_tools(&request["tools"])?;
    if let Some(first) = state.requests.first() {
        ensure!(
            request["tools"] == first["tools"],
            "captured catalogs differ"
        );
    }
    let input = request["input"]
        .as_array()
        .ok_or_else(|| anyhow!("input shape refused"))?;
    let warmup = request.get("generate") == Some(&Value::Bool(false));
    ensure!(
        matches!(request.get("generate"), None | Some(Value::Bool(_))),
        "generate shape refused"
    );
    let resolved = match request.get("previous_response_id") {
        None => input.clone(),
        Some(Value::String(id)) if id == WARMUP_ID && state.warmup_completed && !warmup => {
            let prior = state
                .requests
                .iter()
                .find(|value| value.get("generate") == Some(&Value::Bool(false)))
                .ok_or_else(|| anyhow!("captured warmup missing"))?;
            let mut resolved = prior["input"]
                .as_array()
                .ok_or_else(|| anyhow!("prior input refused"))?
                .clone();
            resolved.extend(input.iter().cloned());
            resolved
        }
        _ => return Err(anyhow!("unknown or uncompleted prior response")),
    };
    if warmup {
        ensure!(
            !state.warmup_completed && state.requests.len() == 1,
            "extra warmup refused"
        );
    } else {
        ensure!(
            state
                .requests
                .iter()
                .filter(|value| value.get("generate") != Some(&Value::Bool(false)))
                .count()
                == 1,
            "extra inference refused"
        );
        let prompts = resolved
            .iter()
            .filter(|item| item["type"] == "message" && item["role"] == "user")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|item| item["type"] == "input_text" && item["text"] == PROMPT)
            .count();
        ensure!(prompts == 1, "actual semantic prompt missing or duplicated");
        state.inference_input = resolved;
    }
    Ok(warmup)
}

fn validate_tools(tools: &Value) -> Result<()> {
    let entries = tools
        .as_array()
        .ok_or_else(|| anyhow!("tool array missing"))?;
    ensure!(entries.len() == 2, "tool count refused");
    let mut shapes = entries
        .iter()
        .map(|tool| (tool["name"].as_str(), tool["type"].as_str()))
        .collect::<Vec<_>>();
    shapes.sort_unstable();
    ensure!(
        shapes
            == [
                (Some("apply_patch"), Some("custom")),
                (Some("exec_command"), Some("function"))
            ],
        "tool identities refused"
    );
    ensure!(
        entries.iter().all(|tool| tool.get("namespace").is_none()),
        "namespaced tool refused"
    );
    let exec = entries
        .iter()
        .find(|tool| tool["name"] == "exec_command")
        .ok_or_else(|| anyhow!("validated exec_command tool missing"))?;
    let patch = entries
        .iter()
        .find(|tool| tool["name"] == "apply_patch")
        .ok_or_else(|| anyhow!("validated apply_patch tool missing"))?;
    ensure!(
        exec.pointer("/parameters/properties/cmd/type") == Some(&json!("string"))
            && exec.pointer("/parameters/required") == Some(&json!(["cmd"]))
            && exec.pointer("/parameters/properties/timeout_ms/type") == Some(&json!("number"))
            && exec.pointer("/parameters/properties/tty").is_none()
            && exec
                .pointer("/parameters/properties/yield_time_ms")
                .is_none()
            && exec
                .pointer("/output_schema/properties/session_id")
                .is_none(),
        "one-shot command schema refused"
    );
    ensure!(
        patch.pointer("/format/type") == Some(&json!("grammar"))
            && patch.pointer("/format/syntax") == Some(&json!("lark"))
            && patch
                .pointer("/format/definition")
                .and_then(Value::as_str)
                .is_some_and(|value| value.contains("*** Begin Patch")),
        "patch grammar refused"
    );
    Ok(())
}

use super::FixtureFailure;
use super::HttpReply;
use super::HttpRequest;
use serde_json::Value;
use serde_json::json;
use std::io::Write;

const MAX_HEADERS: usize = 8192;
const MAX_BODY: usize = 262144;

pub(super) struct ResponsesExchange {
    target: String,
    api_key: String,
    prompt: String,
    marker: String,
    requests: Vec<HttpRequest>,
    completed: bool,
    failure: Option<FixtureFailure>,
}

impl ResponsesExchange {
    pub(super) fn new(target: String, api_key: String, prompt: String, marker: String) -> Self {
        let failure = if target.len() > MAX_HEADERS
            || api_key.len() > MAX_HEADERS
            || prompt.len() > MAX_BODY
            || marker.len() > MAX_BODY
        {
            Some(FixtureFailure::Limit)
        } else if target.is_empty() || api_key.is_empty() || prompt.is_empty() || marker.is_empty()
        {
            Some(FixtureFailure::Protocol)
        } else {
            None
        };
        let (target, api_key, prompt, marker) = if failure.is_some() {
            (String::new(), String::new(), String::new(), String::new())
        } else {
            (target, api_key, prompt, marker)
        };
        Self {
            target,
            api_key,
            prompt,
            marker,
            requests: Vec::new(),
            completed: false,
            failure,
        }
    }

    pub(super) fn respond(&mut self, request: HttpRequest) -> Result<HttpReply, FixtureFailure> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let headers = std::iter::once(request.method.len())
            .chain(std::iter::once(request.target.len()))
            .chain(
                request
                    .headers
                    .iter()
                    .flat_map(|(name, value)| [name.len(), value.len(), 4]),
            )
            .try_fold(/*init*/ 0usize, usize::checked_add);
        if self.requests.len() == 4
            || headers.is_none_or(|size| size > MAX_HEADERS)
            || request.body.len() > MAX_BODY
        {
            self.failure = Some(FixtureFailure::Limit);
            return Err(FixtureFailure::Limit);
        }
        let index = self.requests.len();
        self.requests.push(request);
        let result = self.classify(&self.requests[index]);
        match result {
            Ok(reply) => {
                self.completed |= reply.status == 200;
                Ok(reply)
            }
            Err(error) => {
                self.failure = Some(error);
                Err(error)
            }
        }
    }

    pub(super) fn requests(&self) -> &[HttpRequest] {
        &self.requests
    }

    pub(super) fn semantic_complete(&self) -> bool {
        self.completed && self.failure.is_none()
    }

    fn classify(&self, request: &HttpRequest) -> Result<HttpReply, FixtureFailure> {
        if request.target != self.target
            || header(request, "authorization")?
                != Some(format!("Bearer {}", self.api_key).as_str())
            || header(request, "proxy-authorization")?.is_some()
            || header(request, "cookie")?.is_some()
        {
            return Err(FixtureFailure::Protocol);
        }
        if request.method == "GET" {
            if !request.body.is_empty()
                || !header(request, "upgrade")?
                    .is_some_and(|value| value.eq_ignore_ascii_case("websocket"))
                || !header(request, "connection")?.is_some_and(|value| {
                    value
                        .split(',')
                        .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
                })
            {
                return Err(FixtureFailure::Protocol);
            }
            return Ok(HttpReply {
                status: 426,
                content_type: None,
                body: Vec::new(),
            });
        }
        if request.method != "POST"
            || self.completed
            || header(request, "content-type")?
                .and_then(|value| value.split(';').next())
                .map(str::trim)
                != Some("application/json")
            || header(request, "upgrade")?.is_some()
        {
            return Err(FixtureFailure::Protocol);
        }
        let body: Value =
            serde_json::from_slice(&request.body).map_err(|_| FixtureFailure::Protocol)?;
        if body["model"] != "gpt-5.5"
            || body.get("previous_response_id").is_some()
            || !matches!(body.get("generate"), None | Some(Value::Bool(true)))
            || !matches!(body.get("stream"), None | Some(Value::Bool(true)))
        {
            return Err(FixtureFailure::Protocol);
        }
        let input = body["input"].as_array().ok_or(FixtureFailure::Protocol)?;
        let prompts = input
            .iter()
            .filter(|item| item["type"] == "message" && item["role"] == "user")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|item| item["type"] == "input_text" && item["text"] == self.prompt)
            .count();
        if prompts != 1 {
            return Err(FixtureFailure::Protocol);
        }
        let events = [
            json!({"type":"response.created", "response":{"id":"resp_effects"}}),
            json!({"type":"response.output_item.done", "item":{
                "type":"message", "role":"assistant", "id":"msg_effects",
                "content":[{"type":"output_text", "text":self.marker}]
            }}),
            json!({"type":"response.completed", "response":{"id":"resp_effects", "usage":{
                "input_tokens":11, "output_tokens":7, "total_tokens":18
            }}}),
        ];
        let mut output = SseOutput(Vec::new());
        for event in events {
            output
                .write_all(b"data: ")
                .map_err(|_| FixtureFailure::Limit)?;
            serde_json::to_writer(&mut output, &event).map_err(|_| FixtureFailure::Limit)?;
            output
                .write_all(b"\n\n")
                .map_err(|_| FixtureFailure::Limit)?;
        }
        Ok(HttpReply {
            status: 200,
            content_type: Some("text/event-stream"),
            body: output.0,
        })
    }
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> Result<Option<&'a str>, FixtureFailure> {
    let mut values = request
        .headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str());
    let first = values.next();
    if values.next().is_some() {
        return Err(FixtureFailure::Protocol);
    }
    Ok(first)
}

struct SseOutput(Vec<u8>);

impl Write for SseOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > MAX_BODY)
        {
            return Err(std::io::Error::other("owned fixture output limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

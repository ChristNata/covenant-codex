use super::FixtureFailure;
use super::HttpReply;
use super::HttpRequest;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

const MAX_BYTES: usize = 32768;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Fresh,
    Initialized,
    Ready,
    Complete,
}

pub(super) struct McpExchange {
    target: String,
    phase: Phase,
    requests: Vec<HttpRequest>,
    failure: Option<FixtureFailure>,
}

impl McpExchange {
    pub(super) fn new(target: String) -> Self {
        Self {
            target,
            phase: Phase::Fresh,
            requests: Vec::new(),
            failure: None,
        }
    }

    pub(super) fn respond(&mut self, request: HttpRequest) -> Result<HttpReply, FixtureFailure> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let size = std::iter::once(request.method.len())
            .chain(std::iter::once(request.target.len()))
            .chain(std::iter::once(request.body.len()))
            .chain(
                request
                    .headers
                    .iter()
                    .flat_map(|(name, value)| [name.len(), value.len(), 4]),
            )
            .try_fold(/*init*/ 0usize, usize::checked_add);
        if self.requests.len() == 8 || size.is_none_or(|size| size > MAX_BYTES) {
            self.failure = Some(FixtureFailure::Limit);
            return Err(FixtureFailure::Limit);
        }
        let index = self.requests.len();
        self.requests.push(request);
        let result = classify(&self.requests[index], &self.target, self.phase);
        match result {
            Ok((phase, reply)) => {
                self.phase = phase;
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

    pub(super) fn discovery_complete(&self) -> bool {
        self.phase == Phase::Complete && self.failure.is_none()
    }
}

fn object<'a>(value: &'a Value, keys: &[&str]) -> Result<&'a Map<String, Value>, FixtureFailure> {
    let object = value.as_object().ok_or(FixtureFailure::Protocol)?;
    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(FixtureFailure::Protocol);
    }
    Ok(object)
}

fn empty_params(params: Option<&Value>) -> bool {
    params.is_none_or(|value| value.as_object().is_some_and(Map::is_empty))
}

fn list_params(params: Option<&Value>) -> bool {
    if empty_params(params) {
        return true;
    }
    let Some(params) = params else {
        return false;
    };
    let Ok(params) = object(params, &["_meta"]) else {
        return false;
    };
    let Some(metadata) = params.get("_meta") else {
        return false;
    };
    let Ok(metadata) = object(metadata, &["progressToken"]) else {
        return false;
    };
    metadata.get("progressToken") == Some(&json!(0))
}

fn classify(
    request: &HttpRequest,
    target: &str,
    phase: Phase,
) -> Result<(Phase, HttpReply), FixtureFailure> {
    if request.target != target
        || request.headers.iter().any(|(name, _)| {
            [
                "authorization",
                "proxy-authorization",
                "cookie",
                "mcp-session-id",
            ]
            .iter()
            .any(|blocked| name.eq_ignore_ascii_case(blocked))
        })
    {
        return Err(FixtureFailure::Protocol);
    }
    let versions: Vec<_> = request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("mcp-protocol-version"))
        .collect();
    if versions.len() > 1 || versions.iter().any(|(_, value)| value != "2025-06-18") {
        return Err(FixtureFailure::Protocol);
    }
    if matches!(request.method.as_str(), "GET" | "DELETE") && request.body.is_empty() {
        return Ok((
            phase,
            HttpReply {
                status: 405,
                content_type: None,
                body: Vec::new(),
            },
        ));
    }
    let content_types: Vec<_> = request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .collect();
    if request.method != "POST"
        || content_types.len() != 1
        || content_types[0].1.split(';').next().map(str::trim) != Some("application/json")
    {
        return Err(FixtureFailure::Protocol);
    }
    let body: Value =
        serde_json::from_slice(&request.body).map_err(|_| FixtureFailure::Protocol)?;
    let fields = object(&body, &["jsonrpc", "id", "method", "params"])?;
    if fields.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(FixtureFailure::Protocol);
    }
    let method = fields.get("method").and_then(Value::as_str);
    let params = fields.get("params");
    if phase == Phase::Initialized && method == Some("notifications/initialized") {
        if fields.contains_key("id") || !empty_params(params) {
            return Err(FixtureFailure::Protocol);
        }
        return Ok((
            Phase::Ready,
            HttpReply {
                status: 202,
                content_type: None,
                body: Vec::new(),
            },
        ));
    }
    let id = fields.get("id").ok_or(FixtureFailure::Protocol)?;
    if !id.is_string() && !id.is_i64() {
        return Err(FixtureFailure::Protocol);
    }
    let (next, result) = match (phase, method) {
        (Phase::Fresh, Some("initialize")) => {
            let params = object(
                params.ok_or(FixtureFailure::Protocol)?,
                &["protocolVersion", "capabilities", "clientInfo"],
            )?;
            if params.get("protocolVersion").and_then(Value::as_str) != Some("2025-06-18") {
                return Err(FixtureFailure::Protocol);
            }
            let capabilities = object(
                params.get("capabilities").ok_or(FixtureFailure::Protocol)?,
                &["elicitation"],
            )?;
            if let Some(elicitation) = capabilities.get("elicitation") {
                let modes = object(elicitation, &["form", "url"])?;
                if modes.values().any(|value| !empty_params(Some(value))) {
                    return Err(FixtureFailure::Protocol);
                }
            }
            let client = object(
                params.get("clientInfo").ok_or(FixtureFailure::Protocol)?,
                &["name", "version", "title"],
            )?;
            if ["name", "version"].iter().any(|key| {
                client
                    .get(*key)
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
            }) || client.get("title").is_some_and(|value| !value.is_string())
            {
                return Err(FixtureFailure::Protocol);
            }
            (
                Phase::Initialized,
                json!({
                    "protocolVersion":"2025-06-18", "capabilities":{"tools":{}},
                    "serverInfo":{"name":"covenant-effects-fixture", "version":"1"}
                }),
            )
        }
        (Phase::Ready, Some("tools/list")) if list_params(params) => {
            (Phase::Complete, json!({"tools":[]}))
        }
        _ => return Err(FixtureFailure::Protocol),
    };
    let body = serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":id, "result":result}))
        .map_err(|_| FixtureFailure::Protocol)?;
    if body.len() > MAX_BYTES {
        return Err(FixtureFailure::Limit);
    }
    Ok((
        next,
        HttpReply {
            status: 200,
            content_type: Some("application/json"),
            body,
        },
    ))
}

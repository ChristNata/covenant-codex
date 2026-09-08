//! Private, bounded HTTP framing for the owned effect fixture.

#[path = "covenant_effect_http_framing.rs"]
mod framing;
use framing::HttpDecoder;

#[path = "covenant_effect_http_mcp.rs"]
mod mcp;
use mcp::McpExchange;

#[path = "covenant_effect_http_responses.rs"]
mod responses;
use responses::ResponsesExchange;

#[path = "covenant_effect_http_connection.rs"]
mod connection;

#[path = "covenant_effect_http_peer.rs"]
mod peer;
use peer::HttpPeer;
use peer::PeerProtocol;

#[path = "covenant_effect_marker.rs"]
mod marker;
use marker::MarkerExpectation;
use marker::parse_marker;

#[path = "covenant_effect_cli_receipt.rs"]
mod receipt;

#[path = "covenant_effect_cli_child.rs"]
mod child;

#[path = "covenant_effect_hook_command.rs"]
mod hook_command;

#[cfg(not(feature = "covenant"))]
#[path = "covenant_effect_cli_fixture.rs"]
mod cli_fixture;

#[cfg(not(feature = "covenant"))]
#[path = "covenant_effect_cli_run.rs"]
mod cli_run;

#[cfg(not(feature = "covenant"))]
#[path = "covenant_effect_cli_mcp_run.rs"]
mod cli_mcp_run;

#[cfg(all(test, not(feature = "covenant")))]
#[path = "covenant_effect_cli_hook_tests.rs"]
mod cli_hook_tests;

#[cfg(all(test, not(feature = "covenant")))]
#[path = "covenant_effect_cli_mcp_tests.rs"]
mod cli_mcp_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixtureFailure {
    Marker,
    Framing,
    Limit,
    Incomplete,
    Protocol,
    Transport,
    Child,
    Cancelled,
    Deadline,
}

impl std::fmt::Display for FixtureFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("owned effect fixture refused")
    }
}

impl std::error::Error for FixtureFailure {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HttpLimits {
    headers: usize,
    body: usize,
    total: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HttpRequest {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
struct HttpReply {
    status: u16,
    content_type: Option<&'static str>,
    body: Vec<u8>,
}

#[cfg(test)]
#[path = "covenant_effect_http_responses_tests.rs"]
mod responses_tests;

#[cfg(test)]
#[path = "covenant_effect_http_mcp_tests.rs"]
mod mcp_tests;

#[cfg(test)]
#[path = "covenant_effect_http_framing_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "covenant_effect_http_owner_tests.rs"]
mod owner_tests;

#[cfg(test)]
#[path = "covenant_effect_marker_tests.rs"]
mod marker_tests;

#[cfg(test)]
#[path = "covenant_effect_http_responses_owner_tests.rs"]
mod responses_owner_tests;

#[cfg(test)]
#[path = "covenant_effect_cli_receipt_tests.rs"]
mod receipt_tests;

#[cfg(test)]
#[path = "covenant_effect_cli_child_tests.rs"]
mod child_tests;

#[cfg(test)]
#[path = "covenant_effect_hook_command_tests.rs"]
mod hook_command_tests;

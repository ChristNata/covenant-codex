//! Private, bounded HTTP framing for the owned effect fixture.

#[path = "covenant_effect_http_framing.rs"]
mod framing;
use framing::HttpDecoder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixtureFailure {
    Framing,
    Limit,
    Incomplete,
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

#[derive(Debug, Eq, PartialEq)]
struct HttpRequest {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[cfg(test)]
#[path = "covenant_effect_http_framing_tests.rs"]
mod tests;

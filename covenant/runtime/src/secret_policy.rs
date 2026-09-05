//! Reviewed finite scrub policy; arbitrary provider-name closure belongs to F21.

use super::WindowsEnvironmentError;
use super::compare_names;
use std::cmp::Ordering;

const SCRUB_KEYS: &[&str] = &[
    "COVENANT_DECIDER_PATH",
    "COVENANT_DECIDER_SHA256",
    "COVENANT_CHILD_MARKER",
    "CODEX_AUTH_HOME",
    "OPENAI_API_KEY",
    "CODEX_API_KEY",
    "CODEX_ACCESS_TOKEN",
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "CODEX_CONNECTORS_TOKEN",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_SECRET",
    "AZURE_FEDERATED_TOKEN_FILE",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "CODEX_GITHUB_PERSONAL_ACCESS_TOKEN",
    "CODEX_EXEC_SERVER_NOISE_AUTH_TOKEN",
    "NODE_REPL_AUTH_TOKEN",
    "OPENAI_FEDERATION_RULE_ID",
    "OPENAI_IDENTITY_TOKEN_FILE",
    "OPENAI_WORKLOAD_IDENTITY_CONTEXT",
    "OPENAI_ORGANIZATION",
    "OPENAI_PROJECT",
];

pub(super) fn should_scrub(name: &[u16]) -> Result<bool, WindowsEnvironmentError> {
    let aws_prefix = [
        u16::from(b'A'),
        u16::from(b'W'),
        u16::from(b'S'),
        u16::from(b'_'),
    ];
    if let Some(prefix) = name.get(..aws_prefix.len())
        && compare_names(prefix, &aws_prefix)? == Ordering::Equal
    {
        return Ok(true);
    }
    for key in SCRUB_KEYS {
        let key_units: Vec<u16> = key.encode_utf16().collect();
        if compare_names(name, &key_units)? == Ordering::Equal {
            return Ok(true);
        }
    }
    Ok(false)
}

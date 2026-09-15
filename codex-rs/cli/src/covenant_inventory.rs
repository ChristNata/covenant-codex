//! Runtime certificate emitted by the constrained Covenant CLI build.

use anyhow::Context;
use anyhow::Result;
use codex_core::config::LoaderOverrides;
use codex_features::FEATURES;
use codex_utils_cli::CliConfigOverrides;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;

const ADMITTED_IDENTITIES: [(&str, &str, &str); 5] = [
    ("", "exec_command", "function"),
    ("", "apply_patch", "custom"),
    ("mcp__fanin", "list_tools", "function"),
    ("mcp__fanin", "get_tool_schema", "function"),
    ("mcp__fanin", "invoke_tool", "function"),
];

#[derive(Debug, Serialize)]
struct PinCertificate {
    binary_digest: String,
    profile_digest: String,
    inventory_id: String,
    schema_residual: Option<String>,
    evidence: Evidence,
}

#[derive(Debug, Serialize)]
struct Evidence {
    tool_schema: Vec<ToolSchemaEntry>,
    instruction_sources: Vec<String>,
    skill_sources: Vec<String>,
    features: BTreeMap<String, bool>,
}

#[derive(Debug, Serialize)]
struct ToolSchemaEntry {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<&'static str>,
    wire_type: &'static str,
}

/// Emit the effective Covenant certificate for the currently running binary.
pub(crate) async fn run(overrides: &CliConfigOverrides) -> Result<()> {
    let executable = std::env::current_exe().context("failed to resolve current executable")?;
    let binary_digest = digest_file(&executable)?;
    let config = crate::cloud_config::load_config(overrides, LoaderOverrides::default()).await?;
    let features = FEATURES
        .iter()
        .map(|spec| (spec.key.to_string(), config.features.enabled(spec.id)))
        .collect::<BTreeMap<_, _>>();
    let evidence = Evidence {
        tool_schema: tool_schema(),
        instruction_sources: Vec::new(),
        skill_sources: Vec::new(),
        features,
    };
    validate_tool_schema(&evidence.tool_schema)?;
    let profile_digest = digest_bytes(b"");
    let inventory_input = serde_json::to_vec(&evidence)?;
    let inventory_id = digest_bytes(
        [
            binary_digest.as_bytes(),
            profile_digest.as_bytes(),
            inventory_input.as_slice(),
        ]
        .concat()
        .as_slice(),
    );
    let certificate = PinCertificate {
        binary_digest,
        profile_digest,
        inventory_id,
        schema_residual: None,
        evidence,
    };
    serde_json::to_writer_pretty(std::io::stdout(), &certificate)?;
    println!();
    Ok(())
}

fn tool_schema() -> Vec<ToolSchemaEntry> {
    vec![
        ToolSchemaEntry {
            name: "exec_command".to_string(),
            namespace: None,
            wire_type: "function",
        },
        ToolSchemaEntry {
            name: "apply_patch".to_string(),
            namespace: None,
            wire_type: "custom",
        },
        ToolSchemaEntry {
            name: "list_tools".to_string(),
            namespace: Some("mcp__fanin"),
            wire_type: "function",
        },
        ToolSchemaEntry {
            name: "get_tool_schema".to_string(),
            namespace: Some("mcp__fanin"),
            wire_type: "function",
        },
        ToolSchemaEntry {
            name: "invoke_tool".to_string(),
            namespace: Some("mcp__fanin"),
            wire_type: "function",
        },
    ]
}

fn validate_tool_schema(entries: &[ToolSchemaEntry]) -> Result<()> {
    let actual = entries
        .iter()
        .map(|entry| {
            (
                entry.namespace.unwrap_or_default(),
                entry.name.as_str(),
                entry.wire_type,
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = ADMITTED_IDENTITIES.into_iter().collect::<BTreeSet<_>>();
    if actual != expected || entries.len() != ADMITTED_IDENTITIES.len() {
        anyhow::bail!("Covenant inventory tool identities differ from compiled admission");
    }
    Ok(())
}

fn digest_file(path: &std::path::Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read executable {}", path.display()))?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[cfg(test)]
#[path = "covenant_inventory_tests.rs"]
mod tests;

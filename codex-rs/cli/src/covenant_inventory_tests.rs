use super::ToolSchemaEntry;
use super::digest_bytes;
use super::tool_schema;
use super::validate_tool_schema;

#[test]
fn certificate_evidence_has_the_admitted_wire_forms() {
    let json = serde_json::to_value(tool_schema()).expect("tool schema serializes");
    assert_eq!(
        json,
        serde_json::json!([
            {"name": "exec_command", "wire_type": "function"},
            {"name": "apply_patch", "wire_type": "custom"},
            {"name": "list_tools", "namespace": "mcp__fanin", "wire_type": "function"},
            {"name": "get_tool_schema", "namespace": "mcp__fanin", "wire_type": "function"},
            {"name": "invoke_tool", "namespace": "mcp__fanin", "wire_type": "function"}
        ])
    );
}

#[test]
fn unknown_registered_identity_fails_inventory_validation() {
    let mut entries = tool_schema();
    entries.push(ToolSchemaEntry {
        name: "unclassified_tool".to_string(),
        namespace: Some("mcp__fanin"),
        wire_type: "function",
    });
    let error = validate_tool_schema(&entries).expect_err("unknown identities must fail closed");
    assert_eq!(
        error.to_string(),
        "Covenant inventory tool identities differ from compiled admission"
    );
}

#[test]
fn digest_is_stable_sha256_hex() {
    let digest = digest_bytes(b"covenant");
    assert_eq!(digest.len(), 64);
    assert_eq!(digest, digest_bytes(b"covenant"));
}

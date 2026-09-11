use super::digest_bytes;
use super::tool_schema;
use super::validate_tool_schema;

#[test]
fn certificate_evidence_has_the_two_admitted_wire_forms() {
    let json = serde_json::to_value(tool_schema()).expect("tool schema serializes");
    assert_eq!(
        json,
        serde_json::json!([
            {"name": "exec_command", "wire_type": "function"},
            {"name": "apply_patch", "wire_type": "custom"}
        ])
    );
}

#[test]
fn unknown_registered_identity_fails_inventory_validation() {
    let error = validate_tool_schema(["exec_command", "unclassified_tool"])
        .expect_err("unknown identities must fail closed");
    assert!(error.to_string().contains("unclassified_tool"));
}

#[test]
fn digest_is_stable_sha256_hex() {
    let digest = digest_bytes(b"covenant");
    assert_eq!(digest.len(), 64);
    assert_eq!(digest, digest_bytes(b"covenant"));
}

use codex_covenant::DecideV1;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::sync::LazyLock;

static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema = serde_json::from_str(include_str!("../../schema/decide-v1.json"))
        .expect("canonical schema is JSON");
    jsonschema::draft202012::options()
        .build(&schema)
        .expect("canonical schema compiles as Draft 2020-12")
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Acceptance {
    Allow,
    Deny,
}

fn assert_acceptance(cases: Vec<(String, Value)>, expected: Acceptance) {
    let expected: Vec<_> = cases.iter().map(|(name, _)| (name, expected)).collect();
    let schema: Vec<_> = cases
        .iter()
        .map(|(name, value)| {
            let actual = if SCHEMA.is_valid(value) {
                Acceptance::Allow
            } else {
                Acceptance::Deny
            };
            (name, actual)
        })
        .collect();
    assert_eq!(schema, expected, "test corpus must match the shared schema");
    let decoded: Vec<_> = cases
        .iter()
        .map(|(name, value)| {
            let bytes = serde_json::to_vec(value).expect("fixture encodes");
            let actual = if DecideV1::decode(&bytes).is_ok() {
                Acceptance::Allow
            } else {
                Acceptance::Deny
            };
            (name, actual)
        })
        .collect();
    assert_eq!(decoded, expected, "decoder must enforce the shared schema");
}

fn exec() -> Value {
    json!({
        "version": 1,
        "kind": "exec",
        "exec": {
            "program": "C:\\Windows\\System32\\cmd.exe",
            "argv": ["/d", "/c", "echo", "", "two words", "quote\"text", "世界"],
            "cwd": "C:\\work",
            "env": {"PATH": "C:\\Windows\\System32", "BUILD_LABEL": "café", "EMPTY": ""},
            "sandbox": "workspace-write",
            "network": false,
            "tty": false
        }
    })
}

fn patch(operation: &str, identity: &str) -> Value {
    let mut value = json!({
        "version": 1,
        "kind": "patch",
        "patch": {
            "paths": ["C:\\work\\file.txt"],
            "operations": [{
                "path": "C:\\work\\file.txt", "op": operation,
                "hunks": [{"old_start": 1, "old_lines": 2, "new_start": 1, "new_lines": 2,
                    "lines": [{"kind": "context", "text": "kept"},
                              {"kind": "remove", "text": "old"},
                              {"kind": "add", "text": "new 世界"}]}]
            }],
            "permissions": {"sandbox": "workspace-write", "write_roots": ["C:\\work"],
                            "network": false, "tty": false},
            "resolved_identities": [{
                "path": "C:\\work\\file.txt", "kind": identity,
                "win32_normalized": "\\\\?\\C:\\work\\file.txt",
                "volume_serial": "01234567", "file_index": "100000003",
                "ancestor_identities": [{"path": "C:\\work", "kind": "directory",
                    "win32_normalized": "\\\\?\\C:\\work", "volume_serial": "01234567",
                    "file_index": "100000001"}]
            }]
        }
    });
    if operation != "add" {
        value["patch"]["operations"][0]["pre_image_digest"] = json!("a".repeat(/*n*/ 64));
    }
    if operation == "move" {
        value["patch"]["operations"][0]["destination"] = json!("C:\\work\\renamed.txt");
    }
    if identity == "file" {
        value["patch"]["resolved_identities"][0]["pre_image_digest"] = json!("b".repeat(/*n*/ 64));
    }
    if matches!(identity, "symlink" | "junction") {
        value["patch"]["resolved_identities"][0]["target"] = json!("C:\\work\\target");
    }
    value
}

fn replace(mut value: Value, pointer: &str, replacement: Value) -> Value {
    *value.pointer_mut(pointer).expect("fixture field exists") = replacement;
    value
}

fn remove(mut value: Value, object: &str, field: &str) -> Value {
    value
        .pointer_mut(object)
        .and_then(Value::as_object_mut)
        .expect("fixture object exists")
        .remove(field);
    value
}

#[test]
fn exec_and_patch_round_trips_preserve_complete_payloads() {
    let mut inputs = vec![exec()];
    for operation in ["add", "update", "delete", "move"] {
        for identity in ["file", "directory", "symlink", "junction"] {
            inputs.push(patch(operation, identity));
        }
    }
    for input in inputs {
        assert!(SCHEMA.is_valid(&input));
        let bytes = serde_json::to_vec(&input).expect("fixture encodes");
        let parsed = DecideV1::decode(&bytes).expect("valid request decodes");
        assert_eq!(
            serde_json::to_value(parsed).expect("request encodes"),
            input
        );
    }
}

#[test]
fn schema_permitted_empty_and_optional_fields_remain_allowed() {
    let mut empty_exec = exec();
    empty_exec["exec"]["argv"] = json!([]);
    empty_exec["exec"]["env"] = json!({"": "", "path": "a", "PATH": "b"});
    let mut empty_patch = patch("add", "directory");
    empty_patch["patch"]["operations"][0]["hunks"] = json!([]);
    empty_patch["patch"]["permissions"]["write_roots"] = json!([]);
    empty_patch["patch"]["resolved_identities"][0]["ancestor_identities"] = json!([]);
    let mut extras = empty_patch.clone();
    extras["patch"]["operations"][0]["destination"] = json!("C:\\optional");
    extras["patch"]["operations"][0]["pre_image_digest"] = json!("c".repeat(/*n*/ 64));
    extras["patch"]["resolved_identities"][0]["target"] = json!("C:\\optional");
    extras["patch"]["resolved_identities"][0]["pre_image_digest"] = json!("d".repeat(/*n*/ 64));
    assert_acceptance(
        vec![
            ("empty argv and schema-only env".into(), empty_exec),
            ("empty optional collections".into(), empty_patch),
            ("optional fields outside required cases".into(), extras),
        ],
        Acceptance::Allow,
    );
}

#[test]
fn envelope_requires_version_kind_and_exclusive_matching_payload() {
    let mut both = exec();
    both["patch"] = patch("add", "directory")["patch"].clone();
    let cases = vec![
        ("missing version".into(), remove(exec(), "", "version")),
        ("version two".into(), replace(exec(), "/version", json!(2))),
        ("missing kind".into(), remove(exec(), "", "kind")),
        (
            "unknown kind".into(),
            replace(exec(), "/kind", json!("read")),
        ),
        ("missing payload".into(), remove(exec(), "", "exec")),
        ("both payloads".into(), both),
        (
            "wrong payload".into(),
            replace(exec(), "/kind", json!("patch")),
        ),
    ];
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn unknown_fields_are_rejected_at_every_closed_object_depth() {
    let mut cases = Vec::new();
    for (template, pointers) in [
        (exec(), vec!["", "/exec"]),
        (
            patch("update", "file"),
            vec![
                "/patch",
                "/patch/operations/0",
                "/patch/permissions",
                "/patch/resolved_identities/0",
                "/patch/resolved_identities/0/ancestor_identities/0",
                "/patch/operations/0/hunks/0",
                "/patch/operations/0/hunks/0/lines/0",
            ],
        ),
    ] {
        for pointer in pointers {
            let mut value = template.clone();
            value
                .pointer_mut(pointer)
                .and_then(Value::as_object_mut)
                .expect("fixture object exists")
                .insert("unknown".into(), json!("value"));
            cases.push((pointer.into(), value));
        }
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn required_fields_and_argv_env_types_cannot_be_substituted() {
    let mut cases: Vec<_> = ["program", "argv", "cwd", "env", "sandbox", "network", "tty"]
        .into_iter()
        .map(|field| {
            (
                format!("missing exec {field}"),
                remove(exec(), "/exec", field),
            )
        })
        .collect();
    for (pointer, replacement) in [
        ("/exec/argv", json!("echo hello")),
        ("/exec/argv/0", json!(7)),
        ("/exec/env", json!([])),
        ("/exec/env/PATH", json!(true)),
        ("/exec/network", json!("false")),
        ("/exec/tty", json!(0)),
    ] {
        cases.push((pointer.into(), replace(exec(), pointer, replacement)));
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn patch_operation_and_identity_conditionals_are_enforced_independently() {
    let mut cases = Vec::new();
    for operation in ["update", "delete", "move"] {
        cases.push((
            format!("{operation} without digest"),
            remove(
                patch(operation, "directory"),
                "/patch/operations/0",
                "pre_image_digest",
            ),
        ));
    }
    cases.push((
        "move without destination".into(),
        remove(
            patch("move", "directory"),
            "/patch/operations/0",
            "destination",
        ),
    ));
    cases.push((
        "file without digest".into(),
        remove(
            patch("add", "file"),
            "/patch/resolved_identities/0",
            "pre_image_digest",
        ),
    ));
    for kind in ["symlink", "junction"] {
        cases.push((
            format!("{kind} without target"),
            remove(patch("add", kind), "/patch/resolved_identities/0", "target"),
        ));
        let mut ancestor = patch("add", "directory");
        ancestor["patch"]["resolved_identities"][0]["ancestor_identities"][0]["kind"] = json!(kind);
        cases.push((format!("ancestor {kind} without target"), ancestor));
    }
    for object in [
        "/patch/resolved_identities/0",
        "/patch/resolved_identities/0/ancestor_identities/0",
    ] {
        for field in ["win32_normalized", "volume_serial", "file_index"] {
            cases.push((
                format!("{object} without {field}"),
                remove(patch("add", "directory"), object, field),
            ));
        }
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn optional_fields_reject_explicit_null_and_malformed_present_values() {
    let mut cases = Vec::new();
    for pointer in [
        "/patch/operations/0/destination",
        "/patch/operations/0/pre_image_digest",
        "/patch/resolved_identities/0/target",
        "/patch/resolved_identities/0/pre_image_digest",
    ] {
        let mut value = patch("move", "file");
        value["patch"]["resolved_identities"][0]["target"] = json!("C:\\target");
        cases.push((
            format!("null {pointer}"),
            replace(value, pointer, Value::Null),
        ));
    }
    for pointer in [
        "/patch/operations/0/pre_image_digest",
        "/patch/resolved_identities/0/pre_image_digest",
    ] {
        for digest in [
            "a".repeat(/*n*/ 63),
            "a".repeat(/*n*/ 65),
            "A".repeat(/*n*/ 64),
            "g".repeat(/*n*/ 64),
        ] {
            cases.push((
                format!("bad digest {pointer}: {}", digest.len()),
                replace(patch("update", "file"), pointer, json!(digest)),
            ));
        }
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn minimum_lengths_and_required_collection_sizes_are_enforced() {
    let mut cases = Vec::new();
    for pointer in ["/exec/program", "/exec/cwd", "/exec/sandbox"] {
        cases.push((pointer.into(), replace(exec(), pointer, json!(""))));
    }
    for pointer in [
        "/patch/paths/0",
        "/patch/operations/0/path",
        "/patch/operations/0/destination",
        "/patch/permissions/sandbox",
        "/patch/permissions/write_roots/0",
        "/patch/resolved_identities/0/path",
        "/patch/resolved_identities/0/win32_normalized",
        "/patch/resolved_identities/0/volume_serial",
        "/patch/resolved_identities/0/file_index",
        "/patch/resolved_identities/0/ancestor_identities/0/path",
    ] {
        cases.push((
            pointer.into(),
            replace(patch("move", "file"), pointer, json!("")),
        ));
    }
    for field in ["paths", "operations", "resolved_identities"] {
        let pointer = format!("/patch/{field}");
        cases.push((
            pointer.clone(),
            replace(patch("add", "directory"), &pointer, json!([])),
        ));
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn hunk_counts_use_json_schema_integer_semantics_without_u64_narrowing() {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for field in ["old_start", "old_lines", "new_start", "new_lines"] {
        let pointer = format!("/patch/operations/0/hunks/0/{field}");
        for number in ["0", "1.0", "18446744073709551616"] {
            valid.push((
                format!("{field}={number}"),
                replace(
                    patch("update", "file"),
                    &pointer,
                    serde_json::from_str(number).expect("number fixture"),
                ),
            ));
        }
        for number in [json!(-1), json!(0.5), json!("1"), Value::Null] {
            invalid.push((
                format!("bad {field}={number}"),
                replace(patch("update", "file"), &pointer, number),
            ));
        }
        invalid.push((
            format!("missing {field}"),
            remove(
                patch("update", "file"),
                "/patch/operations/0/hunks/0",
                field,
            ),
        ));
    }
    assert_acceptance(valid, Acceptance::Allow);
    assert_acceptance(invalid, Acceptance::Deny);
}

#[test]
fn duplicate_object_keys_are_rejected_by_transport_policy() {
    // JSON Schema sees already-decoded objects and cannot detect duplicate keys.
    let original = serde_json::to_string(&exec()).expect("fixture encodes");
    let envelope = original.replacen(
        "\"version\":1",
        "\"version\":2,\"version\":1",
        /*count*/ 1,
    );
    let env = original.replacen(
        "\"env\":{",
        "\"env\":{\"BUILD_LABEL\":\"hidden\",",
        /*count*/ 1,
    );
    let actual: Vec<_> = [envelope, env]
        .iter()
        .map(|raw| DecideV1::decode(raw.as_bytes()).is_err())
        .collect();
    assert_eq!(actual, vec![true, true]);
}

#[test]
fn malformed_or_secret_bearing_failures_have_safe_diagnostics() {
    let sentinel = "SENTINEL_ENV_OR_PATCH_SECRET";
    let mut invalid = exec();
    invalid["exec"]["env"]["TOKEN"] = json!(sentinel);
    invalid["kind"] = json!(sentinel);
    let valid_json = serde_json::to_vec(&invalid).expect("fixture encodes");
    let malformed = format!("{{\"kind\":\"{sentinel}\",");
    for bytes in [valid_json, malformed.into_bytes()] {
        let error = match DecideV1::decode(&bytes) {
            Ok(_) => panic!("invalid input must be rejected"),
            Err(error) => error,
        };
        assert!(!format!("{error} {error:?}").contains(sentinel));
    }
}

#[test]
fn string_enums_reject_externally_tagged_object_substitutions() {
    let mut cases = Vec::new();
    for (pointer, variant) in [
        ("/kind", "exec"),
        ("/patch/operations/0/op", "move"),
        ("/patch/operations/0/hunks/0/lines/0/kind", "context"),
        ("/patch/resolved_identities/0/kind", "file"),
        (
            "/patch/resolved_identities/0/ancestor_identities/0/kind",
            "directory",
        ),
    ] {
        let input = if pointer == "/kind" {
            exec()
        } else {
            patch("move", "file")
        };
        cases.push((
            pointer.into(),
            replace(input, pointer, json!({(variant): null})),
        ));
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn schema_objects_reject_positional_array_substitutions() {
    let mut cases = Vec::new();
    let layouts: &[(&str, &[&str])] = &[
        ("", &["version", "kind", "exec"]),
        (
            "/exec",
            &["program", "argv", "cwd", "env", "sandbox", "network", "tty"],
        ),
        ("/exec/env", &[]),
        (
            "/patch",
            &["paths", "operations", "permissions", "resolved_identities"],
        ),
        (
            "/patch/operations/0",
            &["path", "op", "destination", "hunks", "pre_image_digest"],
        ),
        (
            "/patch/operations/0/hunks/0",
            &["old_start", "old_lines", "new_start", "new_lines", "lines"],
        ),
        ("/patch/operations/0/hunks/0/lines/0", &["kind", "text"]),
        (
            "/patch/permissions",
            &["sandbox", "write_roots", "network", "tty"],
        ),
        (
            "/patch/resolved_identities/0",
            &[
                "path",
                "kind",
                "target",
                "ancestor_identities",
                "win32_normalized",
                "volume_serial",
                "file_index",
                "pre_image_digest",
            ],
        ),
        (
            "/patch/resolved_identities/0/ancestor_identities/0",
            &[
                "path",
                "kind",
                "target",
                "win32_normalized",
                "volume_serial",
                "file_index",
            ],
        ),
    ];
    for (pointer, fields) in layouts {
        let mut input = if pointer.is_empty() || pointer.starts_with("/exec") {
            exec()
        } else {
            let mut input = patch("move", "file");
            input["patch"]["resolved_identities"][0]["target"] = json!("C:\\target");
            input["patch"]["resolved_identities"][0]["ancestor_identities"][0]["target"] =
                json!("C:\\parent-target");
            input
        };
        assert!(SCHEMA.is_valid(&input));
        let object = input.pointer(pointer).expect("fixture object exists");
        let sequence = Value::Array(fields.iter().map(|field| object[*field].clone()).collect());
        *input.pointer_mut(pointer).expect("fixture position exists") = sequence;
        cases.push(((*pointer).into(), input));
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn numeric_tokens_reject_raw_serde_private_number_objects() {
    let mut cases = Vec::new();
    for pointer in [
        "/version",
        "/patch/operations/0/hunks/0/old_start",
        "/patch/operations/0/hunks/0/old_lines",
        "/patch/operations/0/hunks/0/new_start",
        "/patch/operations/0/hunks/0/new_lines",
    ] {
        let input = if pointer == "/version" {
            exec()
        } else {
            patch("update", "file")
        };
        // Construct an actual object and serialize directly to wire bytes in
        // assert_acceptance. Reparsing through Value here could interpret this
        // private serde representation as a number and erase the hostile shape.
        let spoof = Value::Object(serde_json::Map::from_iter([(
            "$serde_json::private::Number".into(),
            json!("1"),
        )]));
        cases.push((pointer.into(), replace(input, pointer, spoof)));
    }
    assert_acceptance(cases, Acceptance::Deny);
}

#[test]
fn ordinary_decimal_and_exponent_versions_match_the_schema_oracle() {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for number in ["1.0", "1e0", "1E+0", "10e-1", "0.1e1", "1000e-3"] {
        valid.push((
            number.into(),
            replace(
                exec(),
                "/version",
                serde_json::from_str(number).expect("number fixture"),
            ),
        ));
    }
    for number in ["0", "-0", "-1", "1e1", "1e-1", "1.5", "20e-1"] {
        invalid.push((
            number.into(),
            replace(
                exec(),
                "/version",
                serde_json::from_str(number).expect("number fixture"),
            ),
        ));
    }
    assert_acceptance(valid, Acceptance::Allow);
    assert_acceptance(invalid, Acceptance::Deny);
}

#[test]
fn exact_numeric_boundaries_are_checked_without_float_oracle_rounding() {
    // jsonschema 0.29 uses f64 for numeric const comparison, so it cannot be
    // the oracle for decimal values that round to one or overflow f64. These
    // explicit mathematical cases supplement the shared schema corpus above.
    let cases = [
        (
            "/version",
            "1.0000000000000000000000000000000001",
            Acceptance::Deny,
        ),
        (
            "/version",
            "0.9999999999999999999999999999999999",
            Acceptance::Deny,
        ),
        (
            "/version",
            "10000000000000000000000000000000001e-34",
            Acceptance::Deny,
        ),
        (
            "/version",
            "10000000000000000000000000000000000e-34",
            Acceptance::Allow,
        ),
        (
            "/version",
            "1e999999999999999999999999999999999999999",
            Acceptance::Deny,
        ),
        (
            "/version",
            "1e-999999999999999999999999999999999999999",
            Acceptance::Deny,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "1e9999",
            Acceptance::Allow,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "-0e-9999",
            Acceptance::Allow,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "-1e9999",
            Acceptance::Deny,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "1e-9999",
            Acceptance::Deny,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "9007199254740992.1",
            Acceptance::Deny,
        ),
        (
            "/patch/operations/0/hunks/0/old_start",
            "900719925474099210e-1",
            Acceptance::Allow,
        ),
    ];
    let expected: Vec<_> = cases
        .iter()
        .map(|(path, number, expected)| (*path, *number, *expected))
        .collect();
    let actual: Vec<_> = cases
        .iter()
        .map(|(path, number, _)| {
            let input = if *path == "/version" {
                exec()
            } else {
                patch("update", "file")
            };
            let input = replace(
                input,
                path,
                serde_json::from_str(number).expect("exact number fixture"),
            );
            let bytes = serde_json::to_vec(&input).expect("fixture encodes without rounding");
            let acceptance = if DecideV1::decode(&bytes).is_ok() {
                Acceptance::Allow
            } else {
                Acceptance::Deny
            };
            (*path, *number, acceptance)
        })
        .collect();
    assert_eq!(actual, expected);
}

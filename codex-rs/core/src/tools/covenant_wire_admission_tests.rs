use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[derive(Clone, Copy)]
enum Form {
    Function,
    Custom,
}

fn wire(form: Form, name: &str, namespace: Option<&str>) -> ResponseItem {
    let mut value = json!({"type": "function_call", "name": name,
        "namespace": namespace, "call_id": "wire-call", "arguments": "{opaque input",
        "encrypted_function_args": ["preserved fixture"]});
    if matches!(form, Form::Custom) {
        value = json!({"type": "custom_tool_call", "name": name,
            "namespace": namespace, "call_id": "wire-call", "input": "unchanged patch\n"});
    }
    serde_json::from_value(value).unwrap()
}

fn expected(form: Form, name: &str, namespace: Option<&str>) -> ToolCall {
    let (payload, encrypted_function_args) = match form {
        Form::Function => (
            ToolPayload::Function {
                arguments: "{opaque input".to_string(),
            },
            Some(vec!["preserved fixture".to_string()]),
        ),
        Form::Custom => (
            ToolPayload::Custom {
                input: "unchanged patch\n".to_string(),
            },
            None,
        ),
    };
    ToolCall {
        tool_name: ToolName::new(namespace.map(str::to_string), name),
        call_id: "wire-call".to_string(),
        payload,
        encrypted_function_args,
    }
}

fn search(arguments: serde_json::Value, execution: &str, call_id: Option<&str>) -> ResponseItem {
    ResponseItem::ToolSearchCall {
        id: None,
        status: None,
        call_id: call_id.map(str::to_string),
        execution: execution.to_string(),
        arguments,
        internal_chat_message_metadata_passthrough: None,
    }
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_wire_admission_preserves_unqualified_allowed_calls() {
    for (form, name) in [
        (Form::Function, "exec_command"),
        (Form::Custom, "apply_patch"),
    ] {
        assert_eq!(
            ToolRouter::build_tool_call(wire(form, name, /*namespace*/ None)),
            Ok(Some(expected(form, name, /*namespace*/ None)))
        );
    }
    let message = serde_json::from_value(json!({"type": "message", "role": "assistant",
        "content": [{"type": "output_text", "text": "ordinary message"}]}))
    .unwrap();
    assert_eq!(ToolRouter::build_tool_call(message), Ok(None));
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_wire_admission_denies_namespaces_names_and_wrong_forms() {
    let mut cases = Vec::new();
    for (form, name) in [
        (Form::Function, "exec_command"),
        (Form::Custom, "apply_patch"),
    ] {
        for namespace in ["", "functions", "extension"] {
            cases.push(wire(form, name, Some(namespace)));
        }
        for changed in [
            format!("functions.{name}"),
            format!(" {name}"),
            name.to_uppercase(),
        ] {
            cases.push(wire(form, &changed, /*namespace*/ None));
        }
    }
    for name in [
        "WIRE-CANARY-UNKNOWN",
        "read_file",
        "get_context_remaining",
        "write_stdin",
    ] {
        cases.push(wire(Form::Function, name, /*namespace*/ None));
    }
    cases.push(wire(Form::Function, "apply_patch", /*namespace*/ None));
    cases.push(wire(Form::Custom, "exec_command", /*namespace*/ None));
    let actual: Vec<_> = cases.into_iter().map(ToolRouter::build_tool_call).collect();
    let expected: Vec<_> = (0..actual.len())
        .map(|_| Err(FunctionCallError::CovenantDenied))
        .collect();
    assert_eq!(actual, expected);
    assert!(
        actual
            .iter()
            .all(|result| result.as_ref().is_err_and(|error| {
                let text = error.to_string();
                text == "CovenantDenied { code: unknown_tool }"
                    && text.len() < 128
                    && !text.contains("WIRE-CANARY")
            }))
    );
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_wire_admission_denies_all_search_shapes_before_parse() {
    let mut actual = Vec::new();
    for arguments in [
        json!({"query":"fixture", "limit":1}),
        json!("WIRE-CANARY"),
        json!([]),
    ] {
        for (execution, call_id) in [
            ("client", Some("search-call")),
            ("client", None),
            ("server", Some("search-call")),
        ] {
            actual.push(ToolRouter::build_tool_call(search(
                arguments.clone(),
                execution,
                call_id,
            )));
        }
    }
    let expected: Vec<_> = (0..actual.len())
        .map(|_| Err(FunctionCallError::CovenantDenied))
        .collect();
    assert_eq!(actual, expected);
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_wire_admission_ordinary_mode_preserves_upstream_routing() {
    for namespace in [None, Some(""), Some("functions")] {
        for (form, name) in [
            (Form::Function, "ordinary_read"),
            (Form::Custom, "apply_patch"),
        ] {
            assert_eq!(
                ToolRouter::build_tool_call(wire(form, name, namespace)),
                Ok(Some(expected(form, name, Some("functions"))))
            );
        }
    }
    assert!(matches!(
        ToolRouter::build_tool_call(search(json!([]), "client", Some("search-call"))),
        Err(FunctionCallError::RespondToModel(_))
    ));
    assert_eq!(
        ToolRouter::build_tool_call(search(json!([]), "server", /*call_id*/ None)),
        Ok(None)
    );
}

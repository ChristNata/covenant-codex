use super::covenant_model_catalog;
use super::covenant_selected_model;
use super::decode;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::Value;
use serde_json::json;
use std::io::ErrorKind;

const CATALOG: &[u8] = include_bytes!("../../../covenant/model-catalog.json");
const LIMIT: usize = 256 * 1024;
const CATALOG_REFUSAL: &str = "Covenant model catalog refused";
const SELECTION_REFUSAL: &str = "Covenant model selection refused";

fn fixture() -> Value {
    serde_json::from_slice(CATALOG).expect("frozen owned catalog is JSON")
}

fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("owned fixture serializes")
}

fn typed(bytes: &[u8]) -> ModelsResponse {
    serde_json::from_slice(bytes).expect("owned control uses existing catalog semantics")
}

fn refused(bytes: &[u8]) {
    let error = decode(bytes).expect_err("catalog input must refuse");
    assert_eq!(
        (error.kind(), error.to_string()),
        (ErrorKind::InvalidData, CATALOG_REFUSAL.to_owned())
    );
}

#[test]
fn covenant_catalog_loader_and_decoder_preserve_typed_records() {
    let original = typed(CATALOG);
    assert_eq!(covenant_model_catalog().unwrap(), original);
    assert_eq!(decode(CATALOG).unwrap(), original);

    let mut input = fixture();
    input["models"].as_array_mut().unwrap().reverse();
    input["models"][0]["display_name"] = json!("Owned display-name mutation");
    input["models"][0]["future_metadata"] = json!({
        "left": {"repeated_key": 1},
        "right": {"repeated_key": 2}
    });
    let bytes = encoded(&input);
    let expected = typed(&bytes);
    assert_ne!(expected, original);
    assert_eq!(decode(&bytes).unwrap(), expected);
}

#[test]
fn covenant_catalog_preserves_legacy_instructions_and_metadata_defaults() {
    let legacy = "Owned legacy instruction control. No external input.";
    let mut input = fixture();
    let model = input["models"][0].as_object_mut().unwrap();
    model.remove("model_messages");
    model.remove("default_reasoning_level");
    model.remove("include_apps_usage_instructions");
    model.insert("base_instructions".to_owned(), json!(legacy));
    let bytes = encoded(&input);
    let expected = typed(&bytes);
    assert_eq!(
        (
            expected.models[0].get_model_instructions(/*personality*/ None),
            expected.models[0].default_reasoning_level.clone(),
            expected.models[0].include_apps_usage_instructions,
        ),
        (legacy.to_owned(), None, true)
    );
    assert_eq!(decode(&bytes).unwrap(), expected);

    input["models"][0]["model_messages"] = json!({
        "instructions_template": "Owned canonical template wins."
    });
    let bytes = encoded(&input);
    let expected = typed(&bytes);
    assert_eq!(
        expected.models[0].get_model_instructions(/*personality*/ None),
        "Owned canonical template wins."
    );
    assert_eq!(decode(&bytes).unwrap(), expected);
}

#[test]
fn covenant_catalog_checks_the_original_byte_boundary() {
    let expected = typed(CATALOG);
    let mut bytes = CATALOG.to_vec();
    assert!(bytes.len() < LIMIT);
    bytes.resize(LIMIT - 1, b' ');
    assert_eq!(decode(&bytes).unwrap(), expected);
    bytes.push(b' ');
    assert_eq!(decode(&bytes).unwrap(), expected);
    bytes.push(b' ');
    // This remains a valid complete ModelsResponse; only its original byte count changes.
    assert_eq!(typed(&bytes), expected);
    refused(&bytes);
}

#[test]
fn covenant_catalog_refuses_malformed_json_and_wrong_container_shapes() {
    for bytes in [
        b"".as_slice(),
        b"{",
        b"null",
        b"[]",
        b"true",
        b"4",
        b"\"models\"",
        b"\xff",
        b"{\"models\": [}",
    ] {
        refused(bytes);
    }
    let mut trailing = CATALOG.to_vec();
    trailing.extend_from_slice(b"{} ");
    refused(&trailing);

    let original = fixture();
    for root in [
        json!({}),
        json!({"Models": original["models"]}),
        json!({"models": original["models"], "unexpected": "owned canary"}),
    ] {
        refused(&encoded(&root));
    }
    for collection in [
        Value::Null,
        json!(true),
        json!(4),
        json!("models"),
        json!({}),
    ] {
        refused(&encoded(&json!({"models": collection})));
    }
    for record in [
        Value::Null,
        json!(false),
        json!(1),
        json!("model"),
        json!([]),
    ] {
        let mut input = original.clone();
        input["models"][0] = record;
        refused(&encoded(&input));
    }
    assert_eq!(decode(CATALOG).unwrap(), typed(CATALOG));
}

#[test]
fn covenant_catalog_refuses_duplicate_keys_at_every_object_depth() {
    let original = fixture();
    let models = serde_json::to_string(&original["models"]).unwrap();
    let root_duplicate = format!("{{\"models\":null,\"models\":{models}}}");
    assert!(root_duplicate.len() <= LIMIT);
    refused(root_duplicate.as_bytes());

    let bytes = encoded(&original);
    let text = String::from_utf8(bytes).unwrap();
    let anchor = "\"slug\":\"gpt-5.5\"";
    assert_eq!(text.matches(anchor).count(), 1);
    for replacement in [
        "\"slug\":\"gpt-5.5\",\"slug\":\"gpt-5.5\"",
        "\"slug\":\"gpt-5.5\",\"sl\\u0075g\":\"gpt-5.5\"",
        "\"slug\":\"gpt-5.5\",\"future_metadata\":{\"key\":1,\"key\":2}",
        "\"slug\":\"gpt-5.5\",\"future_metadata\":null,\"future_metadata\":null",
    ] {
        let duplicate = text.replacen(anchor, replacement, 1);
        assert!(duplicate.len() <= LIMIT);
        refused(duplicate.as_bytes());
    }

    let valid = text.replacen(
        anchor,
        "\"slug\":\"gpt-5.5\",\"future_metadata\":{\"left\":{\"key\":1},\"right\":{\"key\":2}}",
        1,
    );
    assert_eq!(decode(valid.as_bytes()).unwrap(), typed(valid.as_bytes()));
}

#[test]
fn covenant_catalog_requires_exactly_four_unique_approved_identities() {
    let original = fixture();
    for models in [
        json!([]),
        json!(&original["models"].as_array().unwrap()[..3]),
        json!([
            original["models"][0],
            original["models"][1],
            original["models"][2],
            original["models"][3],
            original["models"][0]
        ]),
    ] {
        refused(&encoded(&json!({"models": models})));
    }
    let mut duplicate = original.clone();
    duplicate["models"][3] = original["models"][0].clone();
    refused(&encoded(&duplicate));

    for identity in [
        Value::Null,
        json!(true),
        json!(5.5),
        json!(["gpt-5.5"]),
        json!({"slug": "gpt-5.5"}),
        json!(""),
        json!("gpt-6-astra"),
        json!("openai/gpt-5.5"),
        json!("gpt-5.5 "),
        json!("GPT-5.5"),
    ] {
        let mut input = original.clone();
        input["models"][0]["slug"] = identity;
        refused(&encoded(&input));
    }
    let mut missing = original;
    missing["models"][0].as_object_mut().unwrap().remove("slug");
    refused(&encoded(&missing));
}

#[test]
fn covenant_catalog_selection_defaults_and_refuses_unknown_requested_ids() {
    assert_eq!(covenant_selected_model(/*model*/ None).unwrap(), "gpt-5.5");
    for model in ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.2"] {
        assert_eq!(covenant_selected_model(Some(model)).unwrap(), model);
    }
    let long = "owned-secret-model-canary-".repeat(4096);
    for model in [
        "",
        "gpt-6-astra",
        "openai/gpt-5.5",
        "gpt-5.5/extra",
        "gpt-5.5 ",
        " gpt-5.5",
        "GPT-5.5",
        "gpt-5.4-latest",
        "gpt-5.5\0",
        long.as_str(),
    ] {
        let error = covenant_selected_model(Some(model)).expect_err("model must refuse");
        assert_eq!(
            (error.kind(), error.to_string()),
            (ErrorKind::InvalidData, SELECTION_REFUSAL.to_owned())
        );
    }
}

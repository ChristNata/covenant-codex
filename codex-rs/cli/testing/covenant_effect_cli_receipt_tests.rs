//! Ignored receipt-codec tests; APIs are unavailable, not a compiled RED.
use super::receipt::ReceiptError;
use super::receipt::ReceiptName;
use super::receipt::ReceiptRoot;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_json::Value;
use serde_json::json;
use std::cell::Cell;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;

const RAW_LIMIT: usize = 16 * 1024 * 1024;

fn observed(seed: &str) -> Value {
    json!({
        "source":seed, "bytes":[0,255,10,128], "nullable":null,
        "headers":[["x-observed","first"],["x-observed","second"]],
        "nested":{"text":"owned '$' Unicode \u{03bb}","path_data":"../outside.json"}
    })
}

struct RefusingSerialize;

impl Serialize for RefusingSerialize {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("private-serialization-canary"))
    }
}

thread_local! {
    static DECODE_CALLS: Cell<usize> = const { Cell::new(/*value*/ 0) };
}

#[derive(Debug, PartialEq)]
struct Decoded(Value);

impl<'de> Deserialize<'de> for Decoded {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        DECODE_CALLS.with(|calls| calls.set(calls.get() + 1));
        Value::deserialize(deserializer).map(Self)
    }
}

#[test]
fn receipt_round_trips_complete_data_under_each_closed_case_name() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("receipts '$' \u{03bb}");
    fs::create_dir(&root).unwrap();
    let outside = temporary.path().join("outside.json");
    fs::write(&outside, b"owned outside control").unwrap();
    let archive = ReceiptRoot::open(&root).unwrap();
    let names = [
        (
            ReceiptName::OrdinaryHookFirst,
            "ordinary_hook_effect_is_observed.first.json",
        ),
        (
            ReceiptName::OrdinaryHookExistingMarker,
            "ordinary_hook_effect_is_observed.existing-marker.json",
        ),
        (
            ReceiptName::OrdinaryMcpFirst,
            "ordinary_mcp_requests_are_observed.first.json",
        ),
        (
            ReceiptName::CovenantAbsenceFirst,
            "covenant_hook_mcp_effects_are_absent.first.json",
        ),
    ];
    for (name, filename) in names {
        let record = observed(&root.join(filename).to_string_lossy());
        let written = archive.write_new(name, &record).unwrap();
        assert_eq!(written, root.join(filename));
        assert_eq!(written.parent(), Some(root.as_path()));
        // Independently read/decode the actual file; do not trust only the paired codec.
        let mut bytes = Vec::new();
        File::open(&written)
            .unwrap()
            .take((RAW_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= RAW_LIMIT);
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), record);
        assert_eq!(archive.read::<Value>(name).unwrap(), record);
    }
    let mut actual = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    actual.sort();
    let mut expected = names.map(|(_, filename)| filename.to_owned()).to_vec();
    expected.sort();
    assert_eq!(actual, expected);
    assert_eq!(fs::read(&outside).unwrap(), b"owned outside control");
    drop(archive);
    temporary.close().unwrap();
}

#[test]
fn receipt_duplicate_is_refused_before_serialization_without_replacing_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = ReceiptRoot::open(temporary.path()).unwrap();
    let name = ReceiptName::OrdinaryHookFirst;
    let record = observed(&temporary.path().to_string_lossy());
    let path = archive.write_new(name, &record).unwrap();
    let original = fs::read(&path).unwrap();
    assert_eq!(
        archive.write_new(name, &observed("replacement")),
        Err(ReceiptError::Exists)
    );
    // A serializer that refuses on a fresh name must not run for an occupied name.
    assert_eq!(
        archive.write_new(name, &RefusingSerialize),
        Err(ReceiptError::Exists)
    );
    let refused = archive
        .write_new(ReceiptName::OrdinaryMcpFirst, &RefusingSerialize)
        .unwrap_err();
    assert_eq!(refused, ReceiptError::Encode);
    assert_eq!(
        format!("{refused:?} {refused}"),
        "Encode owned receipt refused"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(archive.read::<Value>(name).unwrap(), record);
    // The new failed artifact is retained as failure evidence, never returned as success.
    let partial = temporary
        .path()
        .join("ordinary_mcp_requests_are_observed.first.json");
    assert!(partial.is_file());
    assert!(fs::metadata(&partial).unwrap().len() <= RAW_LIMIT as u64);
    assert_eq!(
        archive.read::<Value>(ReceiptName::OrdinaryMcpFirst),
        Err(ReceiptError::Decode)
    );
    drop(archive);
    temporary.close().unwrap();
}

#[test]
fn receipt_writer_accepts_exact_raw_limit_and_preserves_a_failed_overflow_artifact() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = ReceiptRoot::open(temporary.path()).unwrap();
    let overhead = serde_json::to_vec(&json!({"payload":""})).unwrap().len();
    let mut payload = "x".repeat(RAW_LIMIT - overhead);
    let exact = json!({"payload":payload});
    let path = archive
        .write_new(ReceiptName::OrdinaryHookFirst, &exact)
        .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), RAW_LIMIT as u64);
    let mut bytes = Vec::new();
    File::open(&path)
        .unwrap()
        .take((RAW_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes.len(), RAW_LIMIT);
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), exact);
    drop(bytes);
    drop(exact);
    payload.push('x');
    assert_eq!(
        archive.write_new(
            ReceiptName::OrdinaryHookExistingMarker,
            &json!({"payload":payload})
        ),
        Err(ReceiptError::Limit)
    );
    let failed = temporary
        .path()
        .join("ordinary_hook_effect_is_observed.existing-marker.json");
    assert!(failed.is_file());
    assert!(fs::metadata(&failed).unwrap().len() <= RAW_LIMIT as u64);
    let failed_bytes = fs::read(&failed).unwrap();
    assert_eq!(
        archive.write_new(
            ReceiptName::OrdinaryHookExistingMarker,
            &observed("cannot silently replace a partial artifact")
        ),
        Err(ReceiptError::Exists)
    );
    assert_eq!(fs::read(&failed).unwrap(), failed_bytes);
    assert_eq!(fs::metadata(&path).unwrap().len(), RAW_LIMIT as u64);
    drop(archive);
    temporary.close().unwrap();
}

#[test]
fn receipt_reader_checks_raw_limit_before_invoking_the_decoder() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = ReceiptRoot::open(temporary.path()).unwrap();
    let name = ReceiptName::CovenantAbsenceFirst;
    let record = observed(&temporary.path().to_string_lossy());
    let path = archive.write_new(name, &record).unwrap();
    let mut raw = fs::read(&path).unwrap();
    raw.resize(RAW_LIMIT, b' ');
    fs::write(&path, &raw).unwrap();
    DECODE_CALLS.with(|calls| calls.set(/*val*/ 0));
    assert_eq!(archive.read::<Decoded>(name).unwrap(), Decoded(record));
    assert_eq!(DECODE_CALLS.with(Cell::get), 1);
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(b" ").unwrap();
    drop(file);
    assert_eq!(archive.read::<Decoded>(name), Err(ReceiptError::Limit));
    assert_eq!(DECODE_CALLS.with(Cell::get), 1);
    assert_eq!(fs::metadata(&path).unwrap().len(), (RAW_LIMIT + 1) as u64);
    drop(archive);
    temporary.close().unwrap();
}

#[test]
fn receipt_root_refuses_missing_relative_and_non_directory_inputs_without_creation() {
    let temporary = tempfile::tempdir().unwrap();
    let missing = temporary.path().join("missing-root");
    let plain_file = temporary.path().join("owned-file-root");
    fs::write(&plain_file, b"root control bytes").unwrap();
    for input in [
        missing.as_path(),
        plain_file.as_path(),
        Path::new("relative-receipt-root"),
    ] {
        // Validation alone is exercised for invalid roots; no writer gets such a root.
        assert_eq!(ReceiptRoot::open(input).err(), Some(ReceiptError::Root));
    }
    assert!(!missing.exists());
    assert_eq!(fs::read(&plain_file).unwrap(), b"root control bytes");
    let root = temporary.path().join("fresh-root");
    fs::create_dir(&root).unwrap();
    let archive = ReceiptRoot::open(&root).unwrap();
    assert_eq!(
        archive.read::<Value>(ReceiptName::OrdinaryHookFirst),
        Err(ReceiptError::Io)
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    drop(archive);
    temporary.close().unwrap();
}

#[test]
fn receipt_reader_refuses_malformed_or_extra_data_without_echo_or_mutation() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = ReceiptRoot::open(temporary.path()).unwrap();
    let name = ReceiptName::OrdinaryMcpFirst;
    let path = archive
        .write_new(name, &observed("first valid record"))
        .unwrap();
    for raw in [
        Vec::new(),
        b"{\"private-observed-canary\":".to_vec(),
        vec![0xff],
        b"{}\n{}".to_vec(),
    ] {
        fs::write(&path, &raw).unwrap();
        let error = archive.read::<Value>(name).unwrap_err();
        assert_eq!(error, ReceiptError::Decode);
        assert_eq!(format!("{error:?} {error}"), "Decode owned receipt refused");
        assert_eq!(fs::read(&path).unwrap(), raw);
    }
    let record = observed("successful recovery after replacing owned malformed test data");
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(archive.read::<Value>(name).unwrap(), record);
    drop(archive);
    temporary.close().unwrap();
}

use super::FrozenWindowsEnvironment;
use pretty_assertions::assert_eq;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use windows_sys::Win32::Globalization::CSTR_EQUAL;
use windows_sys::Win32::Globalization::CSTR_GREATER_THAN;
use windows_sys::Win32::Globalization::CSTR_LESS_THAN;
use windows_sys::Win32::Globalization::CompareStringOrdinal;

fn pairs(entries: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    entries
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect()
}

fn ordinal(left: &str, right: &str) -> Ordering {
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    // SAFETY: both owned UTF-16 slices remain live for their exact explicit lengths.
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len().try_into().unwrap(),
            right.as_ptr(),
            right.len().try_into().unwrap(),
            /*bignorecase*/ 1,
        )
    };
    match result {
        CSTR_LESS_THAN => Ordering::Less,
        CSTR_EQUAL => Ordering::Equal,
        CSTR_GREATER_THAN => Ordering::Greater,
        _ => panic!("the independent Windows ordinal oracle failed"),
    }
}

fn assert_environment(frozen: &FrozenWindowsEnvironment, expected: BTreeMap<String, String>) {
    let mut ordered: Vec<_> = expected.iter().collect();
    ordered.sort_by(|(left, _), (right, _)| ordinal(left, right));
    let mut expected_native = Vec::new();
    for (name, value) in ordered {
        expected_native.extend(format!("{name}={value}").encode_utf16());
        expected_native.push(0);
    }
    if expected_native.is_empty() {
        expected_native.push(0);
    }
    expected_native.push(0);
    assert_eq!(
        (frozen.as_json(), frozen.as_native_block()),
        (&expected, expected_native.as_slice()),
    );

    // Decode the native representation independently, including pseudo names.
    let block = frozen.as_native_block();
    assert!(block.ends_with(&[0, 0]));
    let mut decoded = BTreeMap::new();
    if block.len() > 2 {
        for entry in block[..block.len() - 2].split(|unit| *unit == 0) {
            let text = String::from_utf16(entry).unwrap();
            let offset = usize::from(text.starts_with('='));
            let separator = text[offset..].find('=').unwrap() + offset;
            let name = text[..separator].to_string();
            let value = text[separator + 1..].to_string();
            assert!(decoded.insert(name, value).is_none());
        }
    }
    assert_eq!(decoded, expected);
}

fn refused(input: Vec<(OsString, OsString)>) -> bool {
    match FrozenWindowsEnvironment::from_final_pairs(input) {
        Ok(_) => false,
        Err(error) => {
            let display = error.to_string();
            let debug = format!("{error:?}");
            assert!(display.len() <= 96 && debug.len() <= 96);
            assert!(!display.contains("SECRET_CANARY") && !debug.contains("SECRET_CANARY"));
            true
        }
    }
}

#[test]
fn empty_environment_is_explicit_and_does_not_inherit() {
    let frozen = FrozenWindowsEnvironment::from_final_pairs(Vec::new()).unwrap();
    assert_environment(&frozen, BTreeMap::new());
}

#[test]
fn owned_lossless_pairs_have_one_sorted_json_and_native_content() {
    let mut input = pairs(&[
        ("zeta", "line one\nline two=value"),
        ("Éclair", "😀"),
        ("alpha", ""),
        ("Ωmega", "preserve spelling"),
        ("emoji😀", "e\u{301}"),
    ]);
    let expected = input
        .iter()
        .map(|(name, value)| {
            (
                name.to_str().unwrap().to_string(),
                value.to_str().unwrap().to_string(),
            )
        })
        .collect();
    let frozen = FrozenWindowsEnvironment::from_final_pairs(input.clone()).unwrap();
    input.reverse();
    let reordered = FrozenWindowsEnvironment::from_final_pairs(input.clone()).unwrap();
    input[0] = ("changed".into(), "different".into());
    input.clear();
    assert_environment(&frozen, expected);
    assert_eq!(
        (reordered.as_json(), reordered.as_native_block()),
        (frozen.as_json(), frozen.as_native_block()),
    );
}

#[test]
fn duplicate_names_follow_actual_windows_ordinal_equality_before_scrub() {
    let aliases = [
        ("same", "same"),
        ("Path", "PATH"),
        ("ÉCLAIR", "éclair"),
        ("ΩMEGA", "ωmega"),
        ("OPENAI_API_KEY", "openai_api_key"),
        ("=C:", "=c:"),
    ];
    let mut actual = Vec::new();
    for (left, right) in aliases {
        assert_eq!(ordinal(left, right), Ordering::Equal);
        let value = if left.starts_with('=') {
            "C:\\work"
        } else {
            "SECRET_CANARY"
        };
        actual.push(refused(pairs(&[(left, value), (right, value)])));
    }
    assert_eq!(actual, vec![true; aliases.len()]);
}

#[test]
fn unicode_distinctions_are_not_normalized_or_expanded() {
    let names = ["Straße", "STRASSE", "é", "e\u{301}"];
    for (index, left) in names.iter().enumerate() {
        for right in names.iter().skip(index + 1) {
            assert_ne!(ordinal(left, right), Ordering::Equal);
        }
    }
    let input: Vec<_> = names
        .iter()
        .map(|name| (OsString::from(name), OsString::from("keep")))
        .collect();
    let expected = names
        .into_iter()
        .map(|name| (name.to_string(), "keep".to_string()))
        .collect();
    assert_environment(
        &FrozenWindowsEnvironment::from_final_pairs(input).unwrap(),
        expected,
    );
}

#[test]
fn malformed_names_values_and_utf16_refuse_without_secret_output() {
    let mut cases = vec![
        pairs(&[("", "SECRET_CANARY")]),
        pairs(&[("bad=name", "SECRET_CANARY")]),
        pairs(&[("SECRET_CANARY\0name", "value")]),
        pairs(&[("normal", "SECRET_CANARY\0value")]),
        pairs(&[("OPENAI_API_KEY", "SECRET_CANARY\0value")]),
    ];
    let invalid = OsString::from_wide(&[0xd800]);
    cases.push(vec![(invalid.clone(), "SECRET_CANARY".into())]);
    cases.push(vec![("SECRET_CANARY".into(), invalid.clone())]);
    cases.push(vec![("OPENAI_API_KEY".into(), invalid)]);
    let count = cases.len();
    assert_eq!(
        cases.into_iter().map(refused).collect::<Vec<_>>(),
        vec![true; count]
    );
}

#[test]
fn matching_drive_pseudo_entries_preserve_exact_native_spelling() {
    let input = pairs(&[
        ("=c:", "C:\\"),
        ("=D:", "d:/work/../kept"),
        ("=E:", "\\\\?\\E:\\deep"),
        ("NORMAL", "x=y"),
    ]);
    let expected = input
        .iter()
        .map(|(name, value)| {
            (
                name.to_str().unwrap().to_string(),
                value.to_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_environment(
        &FrozenWindowsEnvironment::from_final_pairs(input).unwrap(),
        expected,
    );
}

#[test]
fn malformed_or_mismatched_drive_pseudo_entries_refuse() {
    let entries = [
        ("=", "SECRET_CANARY"),
        ("=CD:", "C:\\work"),
        ("=1:", "C:\\work"),
        ("=C", "C:\\work"),
        ("=C:=tail", "C:\\work"),
        ("=C:", "D:\\work"),
        ("=C:", "C:relative"),
        ("=C:", "\\rooted"),
        ("=C:", "\\\\server\\share\\work"),
        ("=C:", "\\\\.\\C:\\work"),
        ("=C:", ""),
    ];
    assert_eq!(entries.map(|entry| refused(pairs(&[entry]))), [true; 11]);
}

#[test]
fn reviewed_secret_canaries_and_aws_prefix_are_scrubbed_from_both_outputs() {
    let secret_names = [
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
        "AWS_FUTURE_SECRET",
        "AWS_",
    ];
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let challenge_name = format!("KEEP_CHALLENGE_{nonce:x}");
    let challenge_value = format!("value-{nonce:x}");
    let challenge = pairs(&[
        (&challenge_name, &challenge_value),
        ("AWS", "keep"),
        ("XA WS_", "keep"),
        ("XAWS_TOKEN", "keep"),
        ("OPENAI_API_KEY_SUFFIX", "keep"),
        ("PATH", "C:\\tools"),
    ]);
    let expected: BTreeMap<_, _> = challenge
        .iter()
        .map(|(name, value)| {
            (
                name.to_str().unwrap().to_string(),
                value.to_str().unwrap().to_string(),
            )
        })
        .collect();
    for lower in [false, true] {
        let mut input = challenge.clone();
        input.extend(secret_names.iter().map(|name| {
            let name = if lower {
                name.to_lowercase()
            } else {
                name.to_string()
            };
            (name.into(), "SECRET_CANARY".into())
        }));
        let frozen = FrozenWindowsEnvironment::from_final_pairs(input).unwrap();
        assert_environment(&frozen, expected.clone());
    }
}

#[test]
fn name_and_value_bounds_count_utf16_units_before_scrub() {
    let valid = vec![(
        OsString::from("😀".repeat(512)),
        OsString::from("😀".repeat(16383)),
    )];
    let expected = BTreeMap::from([("😀".repeat(512), "😀".repeat(16383))]);
    assert_environment(
        &FrozenWindowsEnvironment::from_final_pairs(valid).unwrap(),
        expected,
    );
    let cases = vec![
        vec![(OsString::from(format!("{}a", "😀".repeat(512))), "v".into())],
        vec![(
            "a".into(),
            OsString::from(format!("{}a", "😀".repeat(16383))),
        )],
        vec![("OPENAI_API_KEY".into(), OsString::from("s".repeat(32767)))],
    ];
    assert_eq!(
        cases.into_iter().map(refused).collect::<Vec<_>>(),
        vec![true; 3]
    );
}

#[test]
fn aggregate_environment_bound_includes_all_raw_terminators_before_scrub() {
    let make = |second: usize| {
        vec![
            (OsString::from("a"), OsString::from("x".repeat(32766))),
            (OsString::from("b"), OsString::from("y".repeat(second))),
        ]
    };
    let expected = BTreeMap::from([
        ("a".to_string(), "x".repeat(32766)),
        ("b".to_string(), "y".repeat(32763)),
    ]);
    let frozen = FrozenWindowsEnvironment::from_final_pairs(make(32763)).unwrap();
    assert_environment(&frozen, expected);
    assert_eq!(frozen.as_native_block().len(), 65536);
    assert!(refused(make(32764)));
    let scrubbed = vec![
        (
            OsString::from("OPENAI_API_KEY"),
            OsString::from("x".repeat(32766)),
        ),
        (
            OsString::from("CODEX_API_KEY"),
            OsString::from("y".repeat(32766)),
        ),
    ];
    assert!(refused(scrubbed));
}

#[test]
fn entry_limit_refuses_without_consuming_an_unbounded_iterator() {
    let input: Vec<_> = (0..2048)
        .map(|index| (OsString::from(format!("K{index:04}")), OsString::from("")))
        .collect();
    let expected = (0..2048)
        .map(|index| (format!("K{index:04}"), String::new()))
        .collect();
    assert_environment(
        &FrozenWindowsEnvironment::from_final_pairs(input).unwrap(),
        expected,
    );
    let mut consumed = 0;
    let result = FrozenWindowsEnvironment::from_final_pairs(std::iter::from_fn(|| {
        consumed += 1;
        assert!(
            consumed <= 2049,
            "constructor over-consumed an oversized iterator"
        );
        Some((
            OsString::from(format!("K{consumed:04}")),
            OsString::from(""),
        ))
    }));
    assert_eq!((result.is_err(), consumed), (true, 2049));
}

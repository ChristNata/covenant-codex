use crate::DecideV1;
use crate::ExecEvent;
use crate::ExecEventError;
use crate::FinalExecInput;
use crate::FrozenWindowsEnvironment;
use crate::NetworkAccess;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;

fn input() -> FinalExecInput {
    FinalExecInput {
        program: PathBuf::from(r"C:\runner.exe"),
        argument_tail: Vec::new(),
        cwd: PathBuf::from(r"C:\"),
        environment: FrozenWindowsEnvironment::from_final_pairs([]).expect("valid empty pairs"),
        sandbox: "restricted".to_owned(),
        network: NetworkAccess::Denied,
    }
}

#[test]
fn complete_event_binds_one_program_to_argv_and_native_facts() {
    let program = r"C:\Program Files\runner😀.exe";
    let tail = [
        "",
        "two words",
        "a\"b",
        "tail\\",
        "line\nfeed",
        r"D:\other.exe",
        "🐙",
    ];
    let challenge = format!("envelope-fixture-{}", std::process::id());
    for network in [NetworkAccess::Denied, NetworkAccess::Allowed] {
        let environment = FrozenWindowsEnvironment::from_final_pairs([
            (OsString::from("PATH"), OsString::from(r"C:\Windows")),
            (OsString::from("=D:"), OsString::from(r"d:\work")),
            (
                OsString::from("COVENANT_RUN_CHALLENGE"),
                OsString::from(&challenge),
            ),
            (
                OsString::from("OPENAI_API_KEY"),
                OsString::from("synthetic-secret-canary"),
            ),
        ])
        .expect("valid final pairs");
        let expected_env = json!({
            "=D:": r"d:\work", "COVENANT_RUN_CHALLENGE": challenge, "PATH": r"C:\Windows"
        });
        let argv: Vec<String> = std::iter::once(program)
            .chain(tail)
            .map(str::to_owned)
            .collect();
        let event = ExecEvent::from_final(FinalExecInput {
            program: PathBuf::from(program),
            argument_tail: tail.map(OsString::from).to_vec(),
            cwd: PathBuf::from(r"D:\"),
            environment,
            sandbox: "restricted token".to_owned(),
            network,
        })
        .expect("valid complete final input");
        let expected = json!({
            "hook_event_name": "Exec", "cwd": r"D:\",
            "decide_v1": {"version": 1, "kind": "exec", "exec": {
                "program": program, "argv": argv, "cwd": r"D:\", "env": expected_env,
                "sandbox": "restricted token", "network": network == NetworkAccess::Allowed,
                "tty": false
            }}
        });
        let actual: Value =
            serde_json::from_slice(event.as_stdin()).expect("one UTF-8 JSON object");
        assert_eq!(actual, expected);
        let compact = serde_json::to_vec(&actual).expect("serialize expected framing");
        assert_eq!(event.as_stdin().len(), compact.len());
        assert_eq!(event.as_stdin().last(), Some(&b'}'));
        assert!(!event.as_stdin().contains(&0), "stdin has no NUL framing");
        let inner = DecideV1::decode(&serde_json::to_vec(&actual["decide_v1"]).unwrap())
            .expect("inner event satisfies the existing schema decoder");
        assert_eq!(serde_json::to_value(inner).unwrap(), expected["decide_v1"]);
        assert_eq!(event.program(), Path::new(program));
        assert_eq!(event.argv(), argv);
        assert_eq!(event.cwd(), Path::new(r"D:\"));
        assert_eq!(
            serde_json::to_value(event.environment().as_json()).unwrap(),
            expected_env
        );
        let expected_native: Vec<u16> =
            format!("=D:=d:\\work\0COVENANT_RUN_CHALLENGE={challenge}\0PATH=C:\\Windows\0\0")
                .encode_utf16()
                .collect();
        assert_eq!(event.environment().as_native_block(), expected_native);
        assert_eq!(event.sandbox(), "restricted token");
        assert_eq!(event.network(), network);
    }
}

#[test]
fn final_event_owns_inputs_and_has_no_later_source_overlay() {
    let mut program = PathBuf::from(r"C:\owned.exe");
    let mut cwd = PathBuf::from(r"D:\work");
    let mut arguments = vec![
        OsString::from("original"),
        OsString::from(r"D:\alternate.exe"),
    ];
    let mut pairs = vec![(OsString::from("VISIBLE"), OsString::from("original"))];
    let mut sandbox = "owned sandbox".to_owned();
    let event = ExecEvent::from_final(FinalExecInput {
        program: program.clone(),
        argument_tail: arguments.clone(),
        cwd: cwd.clone(),
        environment: FrozenWindowsEnvironment::from_final_pairs(pairs.clone()).unwrap(),
        sandbox: sandbox.clone(),
        network: NetworkAccess::Allowed,
    })
    .expect("valid owned input");
    let before = event.as_stdin().to_vec();
    program.clear();
    cwd.push("later");
    arguments.clear();
    pairs[0].1 = OsString::from("later");
    sandbox.clear();
    drop((program, cwd, arguments, pairs, sandbox));
    assert_eq!(event.as_stdin(), before);
    assert_eq!(event.program(), Path::new(r"C:\owned.exe"));
    assert_eq!(
        event.argv(),
        [r"C:\owned.exe", "original", r"D:\alternate.exe"]
    );
    assert_eq!(event.cwd(), Path::new(r"D:\work"));
    assert_eq!(
        event.environment().as_json(),
        &std::collections::BTreeMap::from([("VISIBLE".to_owned(), "original".to_owned())])
    );
    assert_eq!(event.sandbox(), "owned sandbox");
    assert_eq!(event.network(), NetworkAccess::Allowed);
}

#[test]
fn native_path_forms_and_parent_components_are_validated_without_rewriting() {
    for (program, cwd) in [
        (r"C:\work\.\runner.exe", r"C:\work\.\nested"),
        (r"\\?\C:\runner.exe", r"\\?\C:\"),
    ] {
        let mut candidate = input();
        candidate.program = PathBuf::from(program);
        candidate.cwd = PathBuf::from(cwd);
        let event = ExecEvent::from_final(candidate).expect("supported lexical paths");
        assert_eq!(event.program().as_os_str(), OsString::from(program));
        assert_eq!(event.cwd().as_os_str(), OsString::from(cwd));
        assert_eq!(event.argv()[0], program);
    }
    let invalid = [
        "",
        "runner.exe",
        r"C:runner.exe",
        r"\runner.exe",
        r"\\server\share\runner.exe",
        r"\\?\UNC\server\share\runner.exe",
        r"\\.\C:\runner.exe",
        r"\\?\GLOBALROOT\Device\HarddiskVolume1\runner.exe",
        r"C:\..\runner.exe",
        r"C:\work\..\runner.exe",
        r"\\?\C:\work\..\runner.exe",
    ];
    let results: Vec<_> = invalid
        .iter()
        .map(|path| {
            let mut program = input();
            program.program = PathBuf::from(path);
            let mut cwd = input();
            cwd.cwd = PathBuf::from(path);
            (
                ExecEvent::from_final(program).is_err(),
                ExecEvent::from_final(cwd).is_err(),
            )
        })
        .collect();
    let program_only: Vec<_> = [r"C:\", r"C:\directory\", "C:/directory/"]
        .into_iter()
        .map(|path| {
            let mut candidate = input();
            candidate.program = PathBuf::from(path);
            ExecEvent::from_final(candidate).is_err()
        })
        .collect();
    assert_eq!(
        (results, program_only),
        (vec![(true, true); invalid.len()], vec![true; 3])
    );
}

#[test]
fn invalid_unicode_and_nul_are_refused_with_content_free_errors() {
    let malformed = OsString::from_wide(&[0xd800]);
    let canary = format!("synthetic-private-input-{}", std::process::id());
    let mut cases = Vec::new();
    for bad in [malformed, OsString::from(format!("{canary}\0tail"))] {
        let mut program = input();
        let mut path = OsString::from(r"C:\");
        path.push(&bad);
        program.program = PathBuf::from(path.clone());
        cases.push(program);
        let mut cwd = input();
        cwd.cwd = PathBuf::from(path);
        cases.push(cwd);
        let mut argument = input();
        argument.argument_tail.push(bad);
        cases.push(argument);
    }
    let mut sandbox = input();
    sandbox.sandbox = format!("{canary}\0label");
    cases.push(sandbox);
    let errors: Vec<Option<ExecEventError>> = cases
        .into_iter()
        .map(|candidate| ExecEvent::from_final(candidate).err())
        .collect();
    for error in errors.iter().flatten() {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(
            display == "invalid final exec event",
            "fixed content-free display"
        );
        assert!(debug == "ExecEventError", "fixed content-free debug");
        assert!(display.len() <= 96 && debug.len() <= 96);
        assert!(!display.contains(&canary) && !debug.contains(&canary));
    }
    assert_eq!(
        errors.iter().map(Option::is_some).collect::<Vec<_>>(),
        vec![true; errors.len()]
    );
}

#[test]
fn complete_argv_count_and_utf16_budget_include_generated_program_and_terminators() {
    let mut count_at_limit = input();
    count_at_limit.argument_tail = vec![OsString::new(); 1023];
    let event = ExecEvent::from_final(count_at_limit).expect("1024 complete arguments fit");
    assert_eq!(event.argv().len(), 1024);
    let mut count_over_limit = input();
    count_over_limit.argument_tail = vec![OsString::new(); 1024];

    let tail = format!("{}x", "😀".repeat(16375));
    let complete_units =
        r"C:\runner.exe".encode_utf16().count() + 1 + tail.encode_utf16().count() + 1;
    assert_eq!(complete_units, 32766);
    let mut at_limit = input();
    at_limit.argument_tail.push(OsString::from(&tail));
    let event = ExecEvent::from_final(at_limit).expect("exact UTF-16 budget fits");
    assert!(
        event.argv()[1] == tail,
        "supplementary characters remain lossless"
    );
    let mut over_limit = input();
    over_limit
        .argument_tail
        .push(OsString::from(format!("{tail}x")));
    assert_eq!(
        (
            ExecEvent::from_final(count_over_limit).is_err(),
            ExecEvent::from_final(over_limit).is_err()
        ),
        (true, true)
    );
}

#[test]
fn path_and_sandbox_caps_are_lossless_and_respect_coupled_argv_budget() {
    let program = format!("C:\\{}", "p".repeat(32762));
    let cwd = format!("C:\\{}", "c".repeat(32763));
    let sandbox = "é".repeat(64);
    let mut at_limit = input();
    at_limit.program = PathBuf::from(&program);
    at_limit.cwd = PathBuf::from(&cwd);
    at_limit.sandbox = sandbox.clone();
    let event = ExecEvent::from_final(at_limit).expect("coupled exact boundaries fit");
    assert!(event.program().as_os_str() == OsString::from(&program));
    assert!(event.cwd().as_os_str() == OsString::from(&cwd));
    assert!(event.sandbox() == sandbox);
    let mut program_over = input();
    program_over.program = PathBuf::from(format!("{program}p"));
    let mut cwd_over = input();
    cwd_over.cwd = PathBuf::from(format!("{cwd}c"));
    let sandbox_refusals: Vec<_> = [String::new(), format!("{sandbox}x")]
        .into_iter()
        .map(|invalid_sandbox| {
            let mut candidate = input();
            candidate.sandbox = invalid_sandbox;
            ExecEvent::from_final(candidate).is_err()
        })
        .collect();
    assert_eq!(
        (
            ExecEvent::from_final(program_over).is_err(),
            ExecEvent::from_final(cwd_over).is_err(),
            sandbox_refusals
        ),
        (true, true, vec![true; 2])
    );
}

fn escaped_input(padding: String) -> FinalExecInput {
    let mut candidate = input();
    candidate.program = PathBuf::from(format!("C:\\{}", "\u{1}".repeat(32762)));
    candidate.cwd = PathBuf::from(format!("C:\\{}", "\u{1}".repeat(32763)));
    candidate.environment = FrozenWindowsEnvironment::from_final_pairs([
        (OsString::from("a"), OsString::from("\u{1}".repeat(32766))),
        (OsString::from("b"), OsString::from(padding)),
    ])
    .expect("bounded raw pairs remain valid before event escaping");
    candidate
}

fn reference_wire(input: &FinalExecInput) -> Vec<u8> {
    // Independent contract oracle for aggregate encoded length; not a production helper.
    let program = input.program.to_str().expect("fixture uses valid Unicode");
    serde_json::to_vec(&json!({
        "hook_event_name": "Exec", "cwd": input.cwd.to_str().unwrap(),
        "decide_v1": {"version": 1, "kind": "exec", "exec": {
            "program": program, "argv": [program], "cwd": input.cwd.to_str().unwrap(),
            "env": input.environment.as_json(), "sandbox": input.sandbox,
            "network": false, "tty": false
        }}
    }))
    .unwrap()
}

#[test]
fn final_utf8_event_cap_counts_escaping_and_duplicated_paths() {
    let cap = 1_048_576;
    let base_length = reference_wire(&escaped_input(String::new())).len();
    assert!(
        base_length < cap,
        "fixture leaves room for a bounded padding value"
    );
    let gap = cap - base_length;
    let padding = format!("{}{}", "\u{1}".repeat(gap / 6), "x".repeat(gap % 6));
    let at_limit = escaped_input(padding.clone());
    let expected = reference_wire(&at_limit);
    assert_eq!(expected.len(), cap);
    let event = ExecEvent::from_final(at_limit).expect("exact encoded cap fits");
    assert_eq!(event.as_stdin().len(), cap);
    let actual: Value = serde_json::from_slice(event.as_stdin()).expect("valid capped JSON");
    let expected_value: Value = serde_json::from_slice(&expected).unwrap();
    assert!(
        actual == expected_value,
        "complete escaped event is retained without truncation"
    );
    let over_limit = escaped_input(format!("{padding}x"));
    assert_eq!(reference_wire(&over_limit).len(), cap + 1);
    assert!(
        ExecEvent::from_final(over_limit).is_err(),
        "one encoded byte over the cap is refused"
    );
}

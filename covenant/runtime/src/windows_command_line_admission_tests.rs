use crate::ExecEvent;
use crate::ExecEventError;
use crate::FinalExecInput;
use crate::FrozenWindowsEnvironment;
use crate::NetworkAccess;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

fn input(program: &str, tail: &str) -> FinalExecInput {
    FinalExecInput {
        program: PathBuf::from(program),
        argument_tail: vec![OsString::from(tail)],
        cwd: PathBuf::from(r"D:\work"),
        environment: FrozenWindowsEnvironment::from_final_pairs([]).unwrap(),
        sandbox: "restricted".to_owned(),
        network: NetworkAccess::Denied,
    }
}

fn retained(event: &ExecEvent, program: &str, tail: &str) -> bool {
    let expected = json!({
        "hook_event_name": "Exec", "cwd": r"D:\work",
        "decide_v1": {"version": 1, "kind": "exec", "exec": {
            "program": program, "argv": [program, tail], "cwd": r"D:\work",
            "env": {}, "sandbox": "restricted", "network": false, "tty": false
        }}
    });
    serde_json::from_slice::<Value>(event.as_stdin()).unwrap() == expected
        && event.argv() == [program, tail]
        && event.program() == Path::new(program)
        && event.environment().as_native_block() == [0, 0]
}

fn safe_refusal(result: Result<ExecEvent, ExecEventError>) -> bool {
    result.err().is_some_and(|error| {
        let display = error.to_string();
        let debug = format!("{error:?}");
        error == ExecEventError
            && display == "invalid final exec event"
            && debug == "ExecEventError"
            && display.len() < 96
            && debug.len() < 96
            && !display.contains("PRIVATE")
            && !debug.contains("PRIVATE")
    })
}

#[test]
fn constructor_rejects_embedded_program_quote() {
    let program = r"C:\r.exe";
    let quoted_tail = "PRIVATE\"argument";
    let plain = ExecEvent::from_final(input(program, "ordinary")).unwrap();
    let quoted = ExecEvent::from_final(input(program, quoted_tail)).unwrap();
    assert_eq!(
        [
            retained(&plain, program, "ordinary"),
            retained(&quoted, program, quoted_tail)
        ],
        [true; 2]
    );
    let invalid = [r#"C:\PRIVATE"runner.exe"#, r#"\\?\C:\PRIVATE"runner.exe"#];
    let mut refusals = Vec::new();
    for candidate in invalid {
        let path = Path::new(candidate);
        assert!(path.is_absolute() && path.file_name().is_some());
        assert!(candidate.encode_utf16().count() + 2 < 32766);
        refusals.push(safe_refusal(ExecEvent::from_final(input(candidate, ""))));
    }
    assert_eq!(refusals, vec![true; 2]);
}

#[test]
fn constructor_rejects_native_quote_expansion_over_cap() {
    let program = r"C:\r.exe";
    let tail = format!("{}x", "\"".repeat(16377));
    let over = format!("{tail}x");
    let program_units = program.encode_utf16().count();
    let raw_units = program_units + 1 + tail.encode_utf16().count() + 1;
    let encoded_units = program_units + 1 + 2 + 2 * 16377 + 1 + 1;
    assert_eq!((program_units, raw_units, encoded_units), (8, 16388, 32767));
    assert_eq!(program_units + 1 + over.encode_utf16().count() + 1, 16389);
    let accepted = ExecEvent::from_final(input(program, &tail)).unwrap();
    assert!(
        retained(&accepted, program, &tail),
        "whole positive facts retained"
    );
    assert_eq!(
        safe_refusal(ExecEvent::from_final(input(program, &over))),
        true
    );
}

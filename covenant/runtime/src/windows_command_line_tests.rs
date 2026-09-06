use crate::ExecEvent;
use crate::FinalExecInput;
use crate::FrozenWindowsEnvironment;
use crate::NetworkAccess;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;

fn input(program: &str, tail: &[&str]) -> FinalExecInput {
    FinalExecInput {
        program: PathBuf::from(program),
        argument_tail: tail.iter().copied().map(OsString::from).collect(),
        cwd: PathBuf::from(r"D:\work"),
        environment: FrozenWindowsEnvironment::from_final_pairs([(
            OsString::from("VISIBLE"),
            OsString::from("original"),
        )])
        .unwrap(),
        sandbox: "restricted".to_owned(),
        network: NetworkAccess::Denied,
    }
}

fn bound_facts(event: &ExecEvent, argv: &[String]) -> bool {
    let expected = json!({
        "hook_event_name": "Exec", "cwd": r"D:\work",
        "decide_v1": {"version": 1, "kind": "exec", "exec": {
            "program": argv[0], "argv": argv, "cwd": r"D:\work",
            "env": {"VISIBLE": "original"}, "sandbox": "restricted",
            "network": false, "tty": false
        }}
    });
    let environment: Vec<u16> = "VISIBLE=original\0\0".encode_utf16().collect();
    serde_json::from_slice::<Value>(event.as_stdin()).unwrap() == expected
        && event.argv() == argv
        && event.program() == Path::new(&argv[0])
        && event.cwd() == Path::new(r"D:\work")
        && event.environment().as_native_block() == environment
        && event.sandbox() == "restricted"
        && event.network() == NetworkAccess::Denied
}

struct ParsedArguments(*mut *mut u16);

impl Drop for ParsedArguments {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this is the single allocation returned by Shell32, still owned here.
            unsafe {
                LocalFree(self.0.cast());
            }
        }
    }
}

fn shell32_argv(units: &[u16]) -> Vec<String> {
    assert!(!units.is_empty() && units.len() <= 32767);
    assert_eq!(units.last(), Some(&0));
    assert!(!units[..units.len() - 1].contains(&0));
    let mut count = 0;
    // SAFETY: the checked NUL-terminated slice stays live throughout this call.
    let pointer = unsafe { CommandLineToArgvW(units.as_ptr(), &mut count) };
    assert!(!pointer.is_null(), "Shell32 parser allocation failed");
    let mut allocation = ParsedArguments(pointer);
    assert!((1..=1024).contains(&count), "bounded reported argc");
    let mut result = Vec::with_capacity(usize::try_from(count).unwrap());
    let mut total = 0_usize;
    for index in 0..usize::try_from(count).unwrap() {
        // SAFETY: Shell32 reports this many pointer entries in its owned allocation.
        let argument = unsafe { pointer.add(index).read() };
        assert!(!argument.is_null(), "reported argument pointer exists");
        let mut value = Vec::new();
        let mut terminated = false;
        for offset in 0..units.len() {
            // SAFETY: each reported argument is NUL-terminated; stop at that terminator.
            let unit = unsafe { argument.add(offset).read() };
            if unit == 0 {
                terminated = true;
                break;
            }
            total = total
                .checked_add(1)
                .filter(|value| *value <= units.len())
                .unwrap();
            value.push(unit);
        }
        assert!(terminated, "argument terminates within the command bound");
        result.push(String::from_utf16(&value).expect("lossless synthetic Unicode"));
    }
    // SAFETY: copying is complete and this guard still owns the one Shell32 allocation.
    let released = unsafe { LocalFree(allocation.0.cast()) };
    if released.is_null() {
        allocation.0 = std::ptr::null_mut();
    }
    assert!(released.is_null(), "Shell32 allocation release succeeded");
    result
}

#[test]
fn native_command_line_round_trips_complete_bound_facts() {
    let tail = [
        "plain",
        "",
        "two words",
        "a\tb",
        "line\r\nfeed",
        "😀🐙",
        "a\"b",
        "\"\"",
        r#"a\"b"#,
        r#"a\\"b"#,
        r#"a\\\"b"#,
        r"tail\",
        r"space tail\\",
        "%NAME%^*$(never)&|",
        r"D:\other.exe",
    ];
    for program in [
        r"C:\Program Files\runner😀.exe",
        r"\\?\C:\Program Files\runner😀.exe",
    ] {
        let expected: Vec<String> = std::iter::once(program)
            .chain(tail)
            .map(str::to_owned)
            .collect();
        let event = ExecEvent::from_final(input(program, &tail)).unwrap();
        assert_eq!(shell32_argv(event.native_command_line()), expected);
        assert!(
            bound_facts(&event, &expected),
            "complete native and G4 facts agree"
        );
    }
}

#[test]
fn native_command_line_matches_independent_canonical_vectors() {
    let program_cases = [
        (r"C:\r.exe", r"C:\r.exe"),
        (r"C:\Program Files\r.exe", r#""C:\Program Files\r.exe""#),
        ("C:\\tab\tname.exe", "\"C:\\tab\tname.exe\""),
        ("C:\\line\r\nname.exe", "\"C:\\line\r\nname.exe\""),
        (r"\\?\C:\r.exe", r"\\?\C:\r.exe"),
        (
            r"\\?\C:\Program Files\r.exe",
            r#""\\?\C:\Program Files\r.exe""#,
        ),
    ];
    for (program, command) in program_cases {
        let event = ExecEvent::from_final(input(program, &[])).unwrap();
        let expected: Vec<u16> = command.encode_utf16().chain([0]).collect();
        assert_eq!(event.native_command_line(), expected);
        assert_eq!(shell32_argv(&expected), [program]);
        assert!(bound_facts(&event, &[program.to_owned()]));
    }
    // Independently written tail units: quotes=34, backslashes=92. No encoder oracle.
    let cases: &[(&str, &[u16])] = &[
        ("", &[34, 34]),
        ("plain", &[112, 108, 97, 105, 110]),
        (r"x\", &[120, 92]),
        (r"x \", &[34, 120, 32, 92, 92, 34]),
        (r"x \\", &[34, 120, 32, 92, 92, 92, 92, 34]),
        ("a\"b", &[34, 97, 92, 34, 98, 34]),
        (r#"a\"b"#, &[34, 97, 92, 92, 92, 34, 98, 34]),
        (r#"a\\"b"#, &[34, 97, 92, 92, 92, 92, 92, 34, 98, 34]),
        (
            r#"a\\\"b"#,
            &[34, 97, 92, 92, 92, 92, 92, 92, 92, 34, 98, 34],
        ),
        ("\"\"", &[34, 92, 34, 92, 34, 34]),
    ];
    for (tail, encoded) in cases {
        let program = r"C:\r.exe";
        let event = ExecEvent::from_final(input(program, &[*tail])).unwrap();
        let expected: Vec<u16> = program
            .encode_utf16()
            .chain([32])
            .chain(encoded.iter().copied())
            .chain([0])
            .collect();
        assert_eq!(event.native_command_line(), expected);
        assert_eq!(shell32_argv(&expected), [program, *tail]);
        assert!(bound_facts(
            &event,
            &[program.to_owned(), (*tail).to_owned()]
        ));
    }
}

#[test]
fn native_command_line_is_retained_immutable_owned_data() {
    let mut program = String::from(r"C:\Program Files\owned.exe");
    let mut arguments = vec![OsString::from("a\"b"), OsString::from("plain")];
    let mut cwd = PathBuf::from(r"D:\work");
    let mut pairs = vec![(OsString::from("VISIBLE"), OsString::from("original"))];
    let mut sandbox = String::from("restricted");
    let event = ExecEvent::from_final(FinalExecInput {
        program: PathBuf::from(&program),
        argument_tail: arguments.clone(),
        cwd: cwd.clone(),
        environment: FrozenWindowsEnvironment::from_final_pairs(pairs.clone()).unwrap(),
        sandbox: sandbox.clone(),
        network: NetworkAccess::Denied,
    })
    .unwrap();
    let pointer = event.native_command_line().as_ptr();
    let before = event.as_stdin().to_vec();
    let mut copied = event.native_command_line().to_vec();
    copied[0] = 0;
    program.clear();
    arguments.clear();
    cwd.clear();
    pairs.clear();
    sandbox.clear();
    drop((program, arguments, cwd, pairs, sandbox));
    let expected: Vec<u16> =
        r#""C:\Program Files\owned.exe" "a\"b" plain"#.encode_utf16().chain([0]).collect();
    let argv = [r"C:\Program Files\owned.exe", "a\"b", "plain"].map(str::to_owned);
    assert_eq!(event.native_command_line(), expected);
    assert!(
        event.native_command_line().as_ptr() == pointer,
        "same retained allocation"
    );
    assert_ne!(copied, expected);
    assert_eq!(shell32_argv(event.native_command_line()), argv);
    assert_eq!(event.as_stdin(), before);
    assert!(bound_facts(&event, &argv));
}

#[test]
fn native_command_line_exact_limit_and_empty_argument_expansion() {
    let program = r"C:\r.exe";
    let tail = format!("{}x", "\"".repeat(16377));
    let expected: Vec<u16> = format!("C:\\r.exe \"{}x\"\0", "\\\"".repeat(16377))
        .encode_utf16()
        .collect();
    assert_eq!(
        (tail.encode_utf16().count() + 10, expected.len()),
        (16388, 32767)
    );
    let exact = ExecEvent::from_final(input(program, &[&tail])).unwrap();
    let argv = vec![program.to_owned(), tail.clone()];
    assert!(
        exact.native_command_line() == expected,
        "entire exact-cap vector agrees"
    );
    assert!(
        shell32_argv(exact.native_command_line()) == argv,
        "entire cap argv round trips"
    );
    assert!(bound_facts(&exact, &argv));
    let over = format!("{tail}x");
    assert_eq!(over.encode_utf16().count() + 10, 16389);
    assert!(ExecEvent::from_final(input(program, &[&over])).is_err());

    let plain = "x".repeat(32756);
    let long = ExecEvent::from_final(input(program, &[&plain])).unwrap();
    let expected: Vec<u16> = format!("C:\\r.exe {plain}\0").encode_utf16().collect();
    let argv = vec![program.to_owned(), plain];
    assert_eq!(expected.len(), 32766);
    assert!(
        long.native_command_line() == expected,
        "entire unquoted vector agrees"
    );
    assert!(
        shell32_argv(long.native_command_line()) == argv,
        "entire unquoted argv agrees"
    );
    assert!(bound_facts(&long, &argv));

    let tails = vec![""; 1023];
    let empty = ExecEvent::from_final(input(program, &tails)).unwrap();
    let expected: Vec<u16> = program
        .encode_utf16()
        .chain([32, 34, 34].repeat(1023))
        .chain([0])
        .collect();
    let argv: Vec<String> = std::iter::once(program)
        .chain(tails)
        .map(str::to_owned)
        .collect();
    assert_eq!((expected.len(), argv.len()), (3078, 1024));
    assert!(
        empty.native_command_line() == expected,
        "all empty argument quotes and separators agree"
    );
    assert!(
        shell32_argv(empty.native_command_line()) == argv,
        "all empty argv entries agree"
    );
    assert!(bound_facts(&empty, &argv));
}

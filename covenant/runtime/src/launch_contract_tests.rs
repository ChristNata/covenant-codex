use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;

use pretty_assertions::assert_eq;

use crate::LaunchContract;
use crate::StartupControls;

fn controls() -> StartupControls {
    StartupControls {
        decider_path: Some(OsString::from(r"C:\STARTUP-CANARY\covenant-cli.exe")),
        decider_sha256: Some(OsString::from("0".repeat(/*n*/ 64))),
        child_marker: Some(OsString::from("STARTUP-CANARY-OPAQUE-MARKER")),
    }
}

fn assert_refused(input: StartupControls) {
    let Err(error) = LaunchContract::from_startup(input) else {
        panic!("invalid startup controls were accepted");
    };
    let rendered = format!("{error:?}: {error}");
    assert!(rendered.len() <= 256, "startup error was not bounded");
    assert!(!rendered.contains("STARTUP-CANARY"));
    assert!(!rendered.contains("OPAQUE-MARKER"));
}

fn long_path(units: usize) -> OsString {
    let mut value = String::from("C:\\");
    while units - value.len() > 200 {
        value.push_str(&"a".repeat(/*n*/ 198));
        value.push('\\');
    }
    value.push_str(&"z".repeat(units - value.len() - 4));
    value.push_str(".exe");
    assert_eq!(value.encode_utf16().count(), units);
    OsString::from(value)
}

#[test]
fn owned_startup_values_remain_frozen_when_original_inputs_change() {
    let mut source_path = OsString::from(r"C:\STARTUP-CANARY\covenant-cli.exe");
    let mut source_digest = OsString::from("0".repeat(/*n*/ 64));
    let mut source_marker = OsString::from("STARTUP-CANARY-OPAQUE-MARKER");
    let contract = LaunchContract::from_startup(StartupControls {
        decider_path: Some(source_path.clone()),
        decider_sha256: Some(source_digest.clone()),
        child_marker: Some(source_marker.clone()),
    })
    .unwrap_or_else(|_| panic!("valid startup controls refused"));
    source_path.clear();
    source_digest.clear();
    source_marker.clear();
    assert_eq!(
        (contract.decider_path(), *contract.expected_sha256()),
        (Path::new(r"C:\STARTUP-CANARY\covenant-cli.exe"), [0; 32]),
    );
}

#[test]
fn digest_decoding_preserves_every_expected_byte() {
    let expected = std::array::from_fn::<_, 32, _>(|index| (index * 7) as u8);
    let mut input = controls();
    input.decider_sha256 = Some(OsString::from(
        expected
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    ));
    let contract = LaunchContract::from_startup(input)
        .unwrap_or_else(|_| panic!("valid nonzero digest refused"));
    assert_eq!(*contract.expected_sha256(), expected);
}

#[test]
fn each_missing_control_refuses_without_exposing_values() {
    let mut missing_path = controls();
    missing_path.decider_path = None;
    assert_refused(missing_path);
    let mut missing_digest = controls();
    missing_digest.decider_sha256 = None;
    assert_refused(missing_digest);
    let mut missing_marker = controls();
    missing_marker.child_marker = None;
    assert_refused(missing_marker);
}

#[test]
fn malformed_digest_text_refuses_without_trimming_or_normalizing() {
    for digest in [
        String::new(),
        "0".repeat(/*n*/ 63),
        "0".repeat(/*n*/ 65),
        "A".repeat(/*n*/ 64),
        "g".repeat(/*n*/ 64),
        format!("{}\n", "0".repeat(/*n*/ 64)),
        format!(" {}", "0".repeat(/*n*/ 64)),
        format!("{}\0", "0".repeat(/*n*/ 63)),
        "STARTUP-CANARY".repeat(/*n*/ 100),
    ] {
        let mut input = controls();
        input.decider_sha256 = Some(OsString::from(digest));
        assert_refused(input);
    }
}

#[test]
fn only_absolute_local_disk_paths_with_a_filename_are_accepted() {
    for valid in [
        r"C:\STARTUP-CANARY\covenant-cli.exe",
        r"\\?\C:\STARTUP-CANARY\covenant-cli.exe",
        r"D:\folder with spaces\政策🦀.exe",
    ] {
        let mut input = controls();
        input.decider_path = Some(OsString::from(valid));
        let contract = LaunchContract::from_startup(input)
            .unwrap_or_else(|_| panic!("valid local path refused"));
        assert_eq!(contract.decider_path(), Path::new(valid));
    }
    for invalid in [
        "",
        "covenant-cli.exe",
        r"C:covenant-cli.exe",
        r"\STARTUP-CANARY\covenant-cli.exe",
        r"C:\",
        r"C:\STARTUP-CANARY\",
        r"C:\STARTUP-CANARY\..\covenant-cli.exe",
        r"\\?\C:\STARTUP-CANARY\..\covenant-cli.exe",
        r"\\server\share\covenant-cli.exe",
        r"\\?\UNC\server\share\covenant-cli.exe",
        r"\\.\C:\STARTUP-CANARY\covenant-cli.exe",
        r"\\?\GLOBALROOT\Device\HarddiskVolume1\covenant-cli.exe",
        "C:\\STARTUP-CANARY\\bad\0.exe",
    ] {
        let mut input = controls();
        input.decider_path = Some(OsString::from(invalid));
        assert_refused(input);
    }
}

#[test]
fn empty_nul_and_invalid_utf16_markers_refuse() {
    for marker in [
        OsString::new(),
        OsString::from("STARTUP-CANARY\0OPAQUE-MARKER"),
        OsString::from_wide(&[0xd800]),
        OsString::from_wide(&[0xdc00]),
    ] {
        let mut input = controls();
        input.child_marker = Some(marker);
        assert_refused(input);
    }
}

#[test]
fn invalid_utf16_in_an_otherwise_absolute_path_or_digest_refuses() {
    let mut wide_path = r"C:\STARTUP-CANARY\bad".encode_utf16().collect::<Vec<_>>();
    wide_path.push(/*value*/ 0xd800);
    wide_path.extend(".exe".encode_utf16());
    let mut input = controls();
    input.decider_path = Some(OsString::from_wide(&wide_path));
    assert_refused(input);
    let mut wide_digest = vec![u16::from(b'0'); 64];
    wide_digest[31] = 0xd800;
    let mut input = controls();
    input.decider_sha256 = Some(OsString::from_wide(&wide_digest));
    assert_refused(input);
}

#[test]
fn startup_size_boundaries_count_utf16_units() {
    let accepted_path = long_path(/*units*/ 32766);
    let mut input = controls();
    input.decider_path = Some(accepted_path.clone());
    input.child_marker = Some(OsString::from("🦀".repeat(/*n*/ 2048)));
    let contract = LaunchContract::from_startup(input)
        .unwrap_or_else(|_| panic!("exact-boundary controls refused"));
    assert_eq!(contract.decider_path(), PathBuf::from(accepted_path));

    let mut path_overflow = controls();
    path_overflow.decider_path = Some(long_path(/*units*/ 32767));
    assert_refused(path_overflow);
    let mut marker_overflow = controls();
    marker_overflow.child_marker = Some(OsString::from(format!("{}x", "🦀".repeat(/*n*/ 2048))));
    assert_refused(marker_overflow);
}

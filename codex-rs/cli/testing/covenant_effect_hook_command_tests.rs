//! Windows P1 generated hook-command integration test.
#![cfg(windows)]
use super::MarkerExpectation;
use super::child::ChildInput;
use super::child::ChildReport;
use super::child::OwnedChild;
use super::hook_command::HookCommand;
use super::parse_marker;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::Instant;

struct HookFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    powershell: PathBuf,
    system_root: PathBuf,
    expected: MarkerExpectation,
}

impl HookFixture {
    fn new() -> Result<Self> {
        let system_root = PathBuf::from(
            std::env::var_os("SystemRoot")
                .ok_or_else(|| anyhow!("SystemRoot prerequisite missing"))?,
        );
        ensure!(system_root.is_absolute(), "absolute SystemRoot required");
        let powershell = system_root.join("System32/WindowsPowerShell/v1.0/powershell.exe");
        ensure!(
            powershell.is_file(),
            "Windows PowerShell prerequisite missing"
        );
        let temporary = tempfile::tempdir()?;
        let raw = temporary.path().join("hook '$' \u{03bb} with spaces");
        fs::create_dir(&raw)?;
        let root = codex_utils_absolute_path::canonicalize_existing_preserving_symlinks(&raw)?;
        for name in ["work space", "home", "path", "temp"] {
            fs::create_dir(root.join(name))?;
        }
        let cwd = root
            .join("work space")
            .into_os_string()
            .into_string()
            .map_err(|_| anyhow!("fixture cwd must be Unicode"))?;
        let expected = MarkerExpectation {
            nonce: "owned nonce '$' \u{03bb} with spaces".to_string(),
            cwd,
            model: "gpt-5.5".to_string(),
        };
        Ok(Self {
            _temporary: temporary,
            root,
            powershell,
            system_root,
            expected,
        })
    }

    fn prepare(&self, label: &str) -> Result<(HookCommand, PathBuf)> {
        let directory = self.root.join(label);
        fs::create_dir(&directory)?;
        fs::create_dir(directory.join("effects"))?;
        let marker = directory.join("effects/marker '$' \u{03bb}.json");
        let hook = HookCommand::prepare(
            &directory.join("writer '$' \u{03bb} script.ps1"),
            &marker,
            &self.expected,
        )?;
        Ok((hook, marker))
    }

    async fn run(
        &self,
        hook: &HookCommand,
        input: Vec<u8>,
        deadline: Instant,
    ) -> Result<ChildReport> {
        let mut command = Command::new(&self.powershell);
        // The exact string later stored in hooks.json runs in this same PowerShell process.
        command
            .env_clear()
            .current_dir(&self.expected.cwd)
            .env("SystemRoot", &self.system_root)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"))
            .env("PATH", self.root.join("path"))
            .env("TEMP", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
            .arg(hook.command_line());
        let child = OwnedChild::spawn(command, ChildInput::Bytes(input))?;
        let (_cancel, cancellation) = oneshot::channel();
        Ok(child.finish(deadline, cancellation).await)
    }
}

fn read_marker(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    ensure!(file.metadata()?.len() <= 16_384, "oversized marker");
    let mut bytes = Vec::new();
    file.take(/*limit*/ 16_385).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 16_384, "marker grew beyond cap");
    Ok(bytes)
}

fn effect_names(marker: &Path) -> Result<Vec<std::ffi::OsString>> {
    let directory = marker
        .parent()
        .ok_or_else(|| anyhow!("marker parent required"))?;
    let mut names = fs::read_dir(directory)?
        .take(/*n*/ 3)
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<Vec<_>>>()?;
    names.sort();
    Ok(names)
}

#[tokio::test(flavor = "current_thread")]
async fn hook_command_creates_exact_marker_and_refuses_reuse_or_invalid_input() -> Result<()> {
    let fixture = HookFixture::new()?;
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 45);
    let first_input = json!({
        "session_id":"synthetic thread '$' \u{03bb} \"quoted\" / \u{1f642}", "transcript_path":null,
        "cwd":fixture.expected.cwd, "hook_event_name":"SessionStart",
        "model":fixture.expected.model, "permission_mode":"bypassPermissions", "source":"startup"
    });
    let mut exact_input = serde_json::to_string(&first_input)?;
    let encoded_session = serde_json::to_string(&first_input["session_id"])?;
    assert_eq!(exact_input.matches(&encoded_session).count(), 1);
    exact_input = exact_input.replacen(
        &encoded_session,
        r#""synthetic thread '$' \u03bb \"quoted\" \/ \ud83d\ude42""#,
        /*count*/ 1,
    );
    for spelling in [r#"\""#, r#"\/"#, r#"\ud83d\ude42"#] {
        assert!(
            exact_input.contains(spelling),
            "missing JSON wire spelling {spelling}"
        );
    }
    assert_eq!(serde_json::from_str::<Value>(&exact_input)?, first_input);
    let units = exact_input.encode_utf16().count();
    ensure!(units < 8192, "fixture input itself exceeds bound");
    exact_input.push_str(&" ".repeat(8192 - units));
    let (first_hook, first_marker) = fixture.prepare("first attempt")?;
    let report = fixture
        .run(&first_hook, exact_input.as_bytes().to_vec(), deadline)
        .await?;
    assert_eq!(
        (
            report.exit.and_then(|exit| exit.code()),
            report.terminal,
            report.cleanup,
            report.stdout,
            report.stderr
        ),
        (Some(0), Ok(()), Ok(()), Vec::new(), Vec::new())
    );
    let original = read_marker(&first_marker)?;
    assert!(!original.starts_with(&[0xef, 0xbb, 0xbf]));
    std::str::from_utf8(&original)?;
    let independently_decoded: Value = serde_json::from_slice(&original)?;
    let parsed = parse_marker(&original, &fixture.expected)?;
    let expected_record =
        json!({"nonce":fixture.expected.nonce,"pid":report.pid,"input":first_input});
    assert_eq!(
        (parsed, independently_decoded),
        (expected_record.clone(), expected_record)
    );
    assert_eq!(
        effect_names(&first_marker)?,
        vec![
            first_marker
                .file_name()
                .ok_or_else(|| anyhow!("marker filename required"))?
                .to_os_string()
        ]
    );

    // Reuse the identical prepared command and destination with distinct complete input.
    let mut second_input = first_input.clone();
    second_input["session_id"] = json!("second synthetic thread");
    second_input["transcript_path"] = json!(fixture.root.join("transcript '$' \u{03bb}.jsonl"));
    let report = fixture
        .run(&first_hook, serde_json::to_vec(&second_input)?, deadline)
        .await?;
    assert_eq!(
        (
            report.exit.and_then(|exit| exit.code()),
            report.terminal,
            report.cleanup,
            report.stdout,
            report.stderr
        ),
        (
            Some(73),
            Ok(()),
            Ok(()),
            Vec::new(),
            b"covenant-hook-refused-v1\n".to_vec()
        )
    );
    assert_eq!(read_marker(&first_marker)?, original);
    assert_eq!(
        effect_names(&first_marker)?,
        vec![
            first_marker
                .file_name()
                .ok_or_else(|| anyhow!("marker filename required"))?
                .to_os_string()
        ]
    );

    // A fresh destination proves that nullable transcript fields are preserved as strings too.
    let (string_hook, string_marker) = fixture.prepare("string transcript")?;
    let report = fixture
        .run(&string_hook, serde_json::to_vec(&second_input)?, deadline)
        .await?;
    assert_eq!(
        (
            report.exit.and_then(|exit| exit.code()),
            report.terminal,
            report.cleanup,
            report.stdout,
            report.stderr
        ),
        (Some(0), Ok(()), Ok(()), Vec::new(), Vec::new())
    );
    let actual = read_marker(&string_marker)?;
    assert!(!actual.starts_with(&[0xef, 0xbb, 0xbf]));
    assert_eq!(
        parse_marker(&actual, &fixture.expected)?,
        json!({
            "nonce":fixture.expected.nonce,"pid":report.pid,"input":second_input
        })
    );

    let mut invalid = vec![
        b"{\"session_id\":".to_vec(),
        format!("{exact_input} ").into_bytes(),
        serde_json::to_vec(&json!([first_input.clone()]))?,
    ];
    for field in ["session_id", "transcript_path"] {
        let mut input = first_input.clone();
        input
            .as_object_mut()
            .ok_or_else(|| anyhow!("input object required"))?
            .remove(field);
        invalid.push(serde_json::to_vec(&input)?);
    }
    for (field, wrong) in [
        ("hook_event_name", "SessionEnd"),
        ("source", "resume"),
        ("cwd", "wrong cwd"),
        ("model", "wrong model"),
    ] {
        let mut input = first_input.clone();
        input[field] = json!(wrong);
        invalid.push(serde_json::to_vec(&input)?);
    }
    for (index, input) in invalid.into_iter().enumerate() {
        let (hook, marker) = fixture.prepare(&format!("refused attempt {index}"))?;
        let report = fixture.run(&hook, input, deadline).await?;
        assert_eq!(
            (
                report.exit.and_then(|exit| exit.code()),
                report.terminal,
                report.cleanup,
                report.stdout,
                report.stderr
            ),
            (
                Some(73),
                Ok(()),
                Ok(()),
                Vec::new(),
                b"covenant-hook-refused-v1\n".to_vec()
            ),
            "invalid row {index}"
        );
        assert_eq!(effect_names(&marker)?, Vec::<std::ffi::OsString>::new());
    }
    Ok(())
}

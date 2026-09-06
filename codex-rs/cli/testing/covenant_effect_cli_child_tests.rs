//! Ignored P0 draft; missing private child APIs are discovery, not a compiled RED.
#![cfg(windows)]
use super::child::ChildFailure;
use super::child::ChildInput;
use super::child::ChildStream;
use super::child::OwnedChild;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::Instant;

const LIMIT: usize = 65_536;
const SCRIPT: &str = r#"param([string]$Mode, [string]$Argument, [int]$ExitCode, [string]$Ready)
$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false, $true)
[Console]::InputEncoding = $utf8
[Console]::OutputEncoding = $utf8
$out = [Console]::OpenStandardOutput()
$err = [Console]::OpenStandardError()
if ($Mode -eq 'record' -or $Mode -eq 'echo') {
    $inputStream = [Console]::OpenStandardInput()
    $buffer = New-Object byte[] 65537
    $filled = 0
    do {
        $count = $inputStream.Read($buffer, $filled, $buffer.Length - $filled)
        $filled += $count
        if ($filled -gt 65536) { exit 97 }
    } while ($count -gt 0)
    if ($Mode -eq 'record') {
        $record = [ordered]@{argument=$Argument; cwd=[Environment]::CurrentDirectory;
            input=$utf8.GetString($buffer,0,$filled); pid=$PID;
            home=$env:HOME; path=$env:PATH; major=$PSVersionTable.PSVersion.Major}
        $bytes = $utf8.GetBytes(($record | ConvertTo-Json -Compress))
        $out.Write($bytes, 0, $bytes.Length)
        $bytes = $utf8.GetBytes(('stderr:' + $Argument))
        $err.Write($bytes, 0, $bytes.Length)
    } else { $out.Write($buffer, 0, $filled) }
    $out.Flush(); $err.Flush(); exit $ExitCode
}
if ($Mode -eq 'overflow-out' -or $Mode -eq 'overflow-err') {
    $bytes = [Text.Encoding]::ASCII.GetBytes(('x' * 65537))
    if ($Mode -eq 'overflow-out') { $out.Write($bytes,0,$bytes.Length); $out.Flush() }
    else { $err.Write($bytes,0,$bytes.Length); $err.Flush() }
} elseif ($Mode -eq 'hold') {
    [IO.File]::WriteAllText($Ready, $PID.ToString(), $utf8)
} else { exit 98 }
[Threading.Thread]::Sleep(30000)
exit 91
"#;

struct ProcessFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    powershell: PathBuf,
    system_root: PathBuf,
}

impl ProcessFixture {
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
        let raw = temporary.path().join("child '$' \u{03bb}");
        fs::create_dir(&raw)?;
        let root = codex_utils_absolute_path::canonicalize_existing_preserving_symlinks(&raw)?;
        for name in ["work", "home", "path", "temp"] {
            fs::create_dir(root.join(name))?;
        }
        let mut script = vec![0xef, 0xbb, 0xbf];
        script.extend_from_slice(SCRIPT.as_bytes());
        fs::write(root.join("controlled.ps1"), script)?;
        Ok(Self {
            _temporary: temporary,
            root,
            powershell,
            system_root,
        })
    }

    fn command(&self, mode: &str, argument: &str, exit: i32, ready: &str) -> Command {
        let mut command = Command::new(&self.powershell);
        command
            .env_clear()
            .current_dir(self.root.join("work"))
            .env("SystemRoot", &self.system_root)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"))
            .env("PATH", self.root.join("path"))
            .env("TEMP", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(self.root.join("controlled.ps1"))
            .arg("-Mode")
            .arg(mode)
            .arg("-Argument")
            .arg(argument)
            .arg("-ExitCode")
            .arg(exit.to_string())
            .arg("-Ready")
            .arg(self.root.join(ready));
        command
    }
}

#[tokio::test(flavor = "current_thread")]
async fn child_preserves_full_utf8_io_cwd_arguments_and_native_status() -> Result<()> {
    let fixture = ProcessFixture::new()?;
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 30);
    let argument = "one argument '$' \u{03bb} with spaces";
    let input = "actual stdin '$' \u{03bb}\nsecond line\r\n";
    for code in [0, 23] {
        let command = fixture.command("record", argument, code, "unused-ready");
        let child = OwnedChild::spawn(command, ChildInput::Bytes(input.as_bytes().to_vec()))?;
        let (_cancel, cancellation) = oneshot::channel();
        let report = child.finish(deadline, cancellation).await;
        // All checks occur after the actual owner returns, including nonzero status.
        let stdout: Value = serde_json::from_slice(&report.stdout)?;
        assert!(report.pid > 0);
        assert_eq!(
            (
                report.exit.and_then(|exit| exit.code()),
                stdout,
                report.stderr,
                report.terminal,
                report.cleanup
            ),
            (
                Some(code),
                json!({
                    "argument":argument, "cwd":fixture.root.join("work"), "input":input,
                    "pid":report.pid, "home":fixture.root.join("home"),
                    "path":fixture.root.join("path"), "major":5
                }),
                format!("stderr:{argument}").into_bytes(),
                Ok(()),
                Ok(())
            )
        );
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn child_enforces_input_and_both_output_caps_with_real_waited_controls() -> Result<()> {
    let fixture = ProcessFixture::new()?;
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 30);
    let bytes = vec![b'x'; LIMIT];
    let command = fixture.command("echo", "exact cap", /*exit*/ 0, "unused-ready");
    let child = OwnedChild::spawn(command, ChildInput::Bytes(bytes.clone()))?;
    let (_cancel, cancellation) = oneshot::channel();
    let exact = child.finish(deadline, cancellation).await;
    assert_eq!(
        (
            exact.exit.and_then(|exit| exit.code()),
            exact.stdout,
            exact.stderr,
            exact.terminal,
            exact.cleanup
        ),
        (Some(0), bytes, Vec::new(), Ok(()), Ok(()))
    );

    let command = fixture.command("hold", "must not spawn", /*exit*/ 0, "oversize-ready");
    let refused = match OwnedChild::spawn(command, ChildInput::Bytes(vec![b'x'; LIMIT + 1])) {
        Err(error) => error,
        Ok(child) => {
            let (cancel, cancellation) = oneshot::channel();
            let _ = cancel.send(());
            let report = child.finish(deadline, cancellation).await;
            panic!("oversize input admitted; cleanup result: {report:?}");
        }
    };
    assert_eq!(refused, ChildFailure::InputLimit);
    assert!(!fixture.root.join("oversize-ready").exists());
    for (mode, stream) in [
        ("overflow-out", ChildStream::Stdout),
        ("overflow-err", ChildStream::Stderr),
    ] {
        let command = fixture.command(mode, "output cap", /*exit*/ 0, "unused-ready");
        let child = OwnedChild::spawn(command, ChildInput::Null)?;
        let (_cancel, cancellation) = oneshot::channel();
        let report = child.finish(deadline, cancellation).await;
        let (stdout, stderr) = match stream {
            ChildStream::Stdout => (vec![b'x'; LIMIT], Vec::new()),
            ChildStream::Stderr => (Vec::new(), vec![b'x'; LIMIT]),
        };
        assert!(report.exit.is_some());
        assert_ne!(report.exit.and_then(|exit| exit.code()), Some(91));
        assert_eq!(
            (
                report.stdout,
                report.stderr,
                report.terminal,
                report.cleanup
            ),
            (
                stdout,
                stderr,
                Err(ChildFailure::OutputLimit(stream)),
                Ok(())
            )
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Stop {
    Deadline,
    CallerCancellation,
}

#[tokio::test(flavor = "current_thread")]
async fn child_deadline_and_caller_cancellation_retain_the_same_settlement_owner() -> Result<()> {
    let fixture = ProcessFixture::new()?;
    let case_deadline = Instant::now() + Duration::from_secs(/*secs*/ 30);
    for stop in [Stop::Deadline, Stop::CallerCancellation] {
        let (ready_name, expected, deadline) = match stop {
            Stop::Deadline => (
                "deadline-ready",
                ChildFailure::Deadline,
                Instant::now() + Duration::from_secs(/*secs*/ 10),
            ),
            Stop::CallerCancellation => ("cancel-ready", ChildFailure::Cancelled, case_deadline),
        };
        let ready_path = fixture.root.join(ready_name);
        let command = fixture.command("hold", "owned wait", /*exit*/ 0, ready_name);
        let child = OwnedChild::spawn(command, ChildInput::Null)?;
        let pid = child.id();
        let (cancel, cancellation) = oneshot::channel();
        let finish = child.finish(deadline, cancellation);
        tokio::pin!(finish);
        let ready = async {
            loop {
                if let Ok(mut file) = File::open(&ready_path) {
                    let mut bytes = [0; 16];
                    if let Ok(count) = file.read(&mut bytes)
                        && &bytes[..count] == pid.to_string().as_bytes()
                    {
                        break true;
                    }
                }
                tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
            }
        };
        let mut early = None;
        let seen = tokio::select! {
            report = &mut finish => { early = Some(report); false }
            result = tokio::time::timeout(Duration::from_secs(/*secs*/ 8), ready) => result.unwrap_or(/*default*/ false),
        };
        let mut pending_after_ready = false;
        if seen && matches!(stop, Stop::CallerCancellation) {
            // Only a borrowed wait is cancelled; the future still owns the actual Child.
            match tokio::time::timeout(Duration::from_millis(/*millis*/ 50), &mut finish).await {
                Ok(report) => early = Some(report),
                Err(_) => pending_after_ready = true,
            }
        }
        if !seen || matches!(stop, Stop::CallerCancellation) {
            let _ = cancel.send(());
        }
        let report = match early {
            Some(report) => report,
            None => finish.await,
        };
        // A missing ready record still awaits cleanup before any failure assertion.
        assert!(seen, "controlled PowerShell never reached its live wait");
        if matches!(stop, Stop::CallerCancellation) {
            assert!(pending_after_ready);
        }
        if matches!(stop, Stop::Deadline) {
            assert!(Instant::now() >= deadline);
        }
        assert!(report.exit.is_some());
        assert_ne!(report.exit.and_then(|exit| exit.code()), Some(91));
        assert_eq!(
            (
                report.pid,
                report.stdout,
                report.stderr,
                report.terminal,
                report.cleanup
            ),
            (pid, Vec::new(), Vec::new(), Err(expected), Ok(()))
        );
    }
    Ok(())
}

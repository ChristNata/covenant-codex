//! Private Codex fixture for checkpoint A's first ordinary session.
use super::MarkerExpectation;
use super::hook_command::HookCommand;
use super::peer::PeerReport;
use super::receipt::ReceiptRoot;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::time::Instant;

// Exact synthetic data from covenant_responses_proxy.rs; no proxy runtime is loaded.
pub(super) const API_KEY: &str = "covenant-sc5-synthetic-noncredential";
pub(super) const PROMPT: &str = "Return the owned SC5 completion marker.";
pub(super) const MARKER: &str = "COVENANT_SC5_TEXT_COMPLETE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RunLabel {
    First,
    ExistingMarker,
}

#[derive(Debug, PartialEq, Serialize)]
pub(super) struct DeclarationBytes {
    pub(super) config: Vec<u8>,
    pub(super) hooks: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq, Serialize)]
pub(super) struct HookAttemptObservation {
    pub(super) script_before: Vec<u8>,
    pub(super) script_after: Vec<u8>,
    pub(super) before: BTreeMap<String, Vec<u8>>,
    pub(super) after: BTreeMap<String, Vec<u8>>,
}

pub(super) struct RunObservation {
    pub(super) binary: PathBuf,
    pub(super) exit: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) before: DeclarationBytes,
    pub(super) after: DeclarationBytes,
    pub(super) model: PeerReport,
    pub(super) mcp: Option<PeerReport>,
    pub(super) model_target: String,
    pub(super) mcp_url: Option<String>,
    pub(super) marker: Option<Vec<u8>>,
    pub(super) effects: Vec<String>,
    pub(super) hook_attempts: HookAttemptObservation,
    pub(super) receipt: PathBuf,
}

pub(super) struct Fixture {
    _temporary: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) work: PathBuf,
    pub(super) system_root: PathBuf,
    pub(super) expected: MarkerExpectation,
    pub(super) receipts: ReceiptRoot,
    pub(super) deadline: Instant,
    effects: PathBuf,
    hook_script: PathBuf,
    hook_attempts: PathBuf,
}

impl Fixture {
    pub(super) fn hook_only() -> Result<Self> {
        let deadline = Instant::now() + Duration::from_secs(/*secs*/ 60);
        let receipt_path = PathBuf::from(
            std::env::var_os("COVENANT_EFFECT_RECEIPT_DIR")
                .ok_or_else(|| anyhow!("required owned receipt directory missing"))?,
        );
        let receipts = ReceiptRoot::open(&receipt_path)?;
        let system_root = PathBuf::from(
            std::env::var_os("SystemRoot")
                .ok_or_else(|| anyhow!("SystemRoot prerequisite missing"))?,
        );
        ensure!(
            system_root.is_absolute() && system_root.is_dir(),
            "absolute SystemRoot required"
        );
        let temporary = tempfile::Builder::new().prefix("covenant-h-").tempdir()?;
        let root =
            codex_utils_absolute_path::canonicalize_existing_preserving_symlinks(temporary.path())?;
        let nonce = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("owned nonce encoding refused"))?
            .to_owned();
        ensure!(
            !nonce.is_empty()
                && nonce.len() <= 128
                && nonce
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "owned nonce representation refused"
        );
        for name in [
            "work", "home", "auth", "profile", "local", "roaming", "temp", "path",
        ] {
            fs::create_dir(root.join(name))?;
        }
        let effects = root.join("effects '$' \u{03bb} with spaces");
        fs::create_dir(&effects)?;
        let hook_attempts = root.join("hook attempts '$' \u{03bb} with spaces");
        fs::create_dir(&hook_attempts)?;
        let attempt_name = hook_attempts
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("attempt directory name encoding refused"))?;
        ensure!(
            hook_attempts.is_absolute()
                && hook_attempts.parent() == Some(root.as_path())
                && fs::symlink_metadata(&hook_attempts)?.file_type().is_dir()
                && attempt_name == "hook attempts '$' \u{03bb} with spaces"
                && !hook_attempts.starts_with(&effects)
                && !effects.starts_with(&hook_attempts),
            "owned attempt directory representation refused"
        );
        let work = codex_utils_absolute_path::canonicalize_existing_preserving_symlinks(
            &root.join("work"),
        )?;
        let expected = MarkerExpectation {
            nonce,
            cwd: work
                .to_str()
                .ok_or_else(|| anyhow!("owned cwd encoding refused"))?
                .to_owned(),
            model: "gpt-5.5".to_owned(),
        };
        let hook_script = root.join("writer '$' \u{03bb} with spaces.ps1");
        let hook = HookCommand::prepare(&hook_script, &effects.join("marker.json"), &expected)?;
        let script_metadata = fs::symlink_metadata(&hook_script)?;
        ensure!(
            script_metadata.file_type().is_file() && script_metadata.len() <= 32 * 1024,
            "owned hook script type/cap refused"
        );
        let mut original = Vec::new();
        File::open(&hook_script)?
            .take(/*limit*/ 32 * 1024 + 1)
            .read_to_end(&mut original)?;
        ensure!(
            original.len() <= 32 * 1024,
            "owned hook script cap exceeded"
        );
        let mut expected_script = vec![0xef, 0xbb, 0xbf];
        expected_script.extend_from_slice(super::hook_command::SCRIPT.as_bytes());
        ensure!(
            original == expected_script,
            "owned hook script preimage changed"
        );
        let boundary = b"if ($bytes.Length -gt 16384) { Refuse-HookInput }\n";
        let offsets = original
            .windows(boundary.len())
            .enumerate()
            .filter_map(|(offset, bytes)| (bytes == boundary).then_some(offset))
            .collect::<Vec<_>>();
        ensure!(offsets.len() == 1, "owned hook script boundary changed");
        let insertion_offset = offsets[0] + boundary.len();
        ensure!(
            original[insertion_offset..]
                .starts_with(b"try { $file = [IO.File]::Open($Marker, [IO.FileMode]::CreateNew"),
            "owned marker CreateNew boundary changed"
        );
        let attempts = hook_attempts
            .to_str()
            .ok_or_else(|| anyhow!("attempt directory encoding refused"))?;
        ensure!(
            attempts.encode_utf16().count() <= 8192 && !attempts.chars().any(char::is_control),
            "attempt directory literal refused"
        );
        let directory = attempts.replace('\'', "''");
        let insertion = format!(
            "\n$attemptName = 'attempt-' + [Guid]::NewGuid().ToString('N') + '.json'\n\
$attemptPath = [IO.Path]::Combine('{directory}', $attemptName)\n\
$attemptFile = $null\n\
try {{\n\
    $attemptFile = [IO.File]::Open($attemptPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)\n\
    $attemptFile.Write($bytes, 0, $bytes.Length)\n\
    $attemptFile.Flush($true)\n\
}} finally {{\n\
    if ($null -ne $attemptFile) {{ $attemptFile.Dispose() }}\n\
}}\n"
        );
        let mut script = original[..insertion_offset].to_vec();
        script.extend_from_slice(insertion.as_bytes());
        script.extend_from_slice(&original[insertion_offset..]);
        ensure!(script.len() <= 32 * 1024, "owned hook script cap exceeded");
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&hook_script)?;
        file.write_all(&script)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        let project_key = &expected.cwd;
        let config = toml::Value::try_from(json!({
            "cli_auth_credentials_store":"file", "forced_login_method":"api",
            "projects":{project_key:{"trust_level":"trusted"}},
            "features":{"hooks":true,"mcp_2026_07_28":false}
        }))?;
        let config = toml::to_string(&config)?;
        let hooks = serde_json::to_vec(&json!({"hooks":{"SessionStart":[{
            "matcher":"^startup$", "hooks":[{
                "type":"command", "command":hook.command_line(), "timeout":10
            }]
        }]}}))?;
        ensure!(
            config.len() <= 65_536 && hooks.len() <= 65_536,
            "owned declarations too large"
        );
        fs::write(root.join("home/config.toml"), config)?;
        fs::write(root.join("home/hooks.json"), hooks)?;
        Ok(Self {
            _temporary: temporary,
            root,
            work,
            system_root,
            expected,
            receipts,
            deadline,
            effects,
            hook_script,
            hook_attempts,
        })
    }

    pub(super) fn expectation(&self) -> &MarkerExpectation {
        &self.expected
    }

    pub(super) fn effect_directory(&self) -> &Path {
        &self.effects
    }

    pub(super) fn hook_script_path(&self) -> &Path {
        &self.hook_script
    }

    pub(super) fn hook_attempt_directory(&self) -> &Path {
        &self.hook_attempts
    }

    pub(super) fn marker_name(&self) -> &str {
        "marker.json"
    }
}

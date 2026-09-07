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
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::time::Instant;

// Exact synthetic data from covenant_responses_proxy.rs; no proxy runtime is loaded.
pub(super) const API_KEY: &str = "covenant-sc5-synthetic-noncredential";
pub(super) const PROMPT: &str = "Return the owned SC5 completion marker.";
pub(super) const MARKER: &str = "COVENANT_SC5_TEXT_COMPLETE";

#[derive(Debug, PartialEq, Serialize)]
pub(super) struct DeclarationBytes {
    pub(super) config: Vec<u8>,
    pub(super) hooks: Option<Vec<u8>>,
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
        let hook = HookCommand::prepare(
            &root.join("writer '$' \u{03bb} with spaces.ps1"),
            &effects.join("marker.json"),
            &expected,
        )?;
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
        })
    }

    pub(super) fn expectation(&self) -> &MarkerExpectation {
        &self.expected
    }

    pub(super) fn effect_directory(&self) -> &Path {
        &self.effects
    }

    pub(super) fn marker_name(&self) -> &str {
        "marker.json"
    }
}

//! Actual ordinary MCP child, two retained peers, and one closed receipt.
use super::child::ChildInput;
use super::child::OwnedChild;
use super::cli_fixture::API_KEY;
use super::cli_fixture::DeclarationBytes;
use super::cli_fixture::Fixture;
use super::cli_fixture::HookAttemptObservation;
use super::cli_fixture::MARKER;
use super::cli_fixture::PROMPT;
use super::peer::HttpPeer;
use super::peer::PeerProtocol;
use super::peer::PeerReport;
use super::receipt::ReceiptName;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::Instant;

#[derive(Serialize)]
struct ArchivedRequest<'a> {
    method: &'a str,
    target: &'a str,
    headers: &'a [(String, String)],
    body: &'a [u8],
}

#[derive(Serialize)]
struct ArchivedPeer<'a> {
    accepted: usize,
    requests: Vec<ArchivedRequest<'a>>,
    terminal: Option<()>,
}

#[derive(Serialize)]
struct ArchivedModel<'a> {
    kind: &'static str,
    peer: ArchivedPeer<'a>,
}

#[derive(Serialize)]
struct Archive<'a> {
    format: &'static str,
    case: &'static str,
    mode: &'static str,
    phase: &'static str,
    binary: &'a str,
    exit_code: Option<i32>,
    stdout: &'a [u8],
    stderr: &'a [u8],
    before: &'a DeclarationBytes,
    after: &'a DeclarationBytes,
    marker: &'a Option<Vec<u8>>,
    effects: &'a [String],
    model: ArchivedModel<'a>,
    mcp: ArchivedPeer<'a>,
    hook_attempts: &'a HookAttemptObservation,
}

pub(super) struct McpRunObservation {
    pub(super) binary: PathBuf,
    pub(super) exit: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) before: DeclarationBytes,
    pub(super) after: DeclarationBytes,
    pub(super) model: PeerReport,
    pub(super) mcp: PeerReport,
    pub(super) model_target: String,
    pub(super) mcp_url: String,
    pub(super) marker: Option<Vec<u8>>,
    pub(super) effects: Vec<String>,
    pub(super) hook_attempts: HookAttemptObservation,
    pub(super) receipt: PathBuf,
}

impl Fixture {
    pub(super) async fn run_mcp(&mut self) -> Result<McpRunObservation> {
        let script_before = read_bounded(self.hook_script_path(), /*limit*/ 32 * 1024)?;
        let attempts_before = read_attempts(self.hook_attempt_directory())?;
        let before = DeclarationBytes {
            config: read_bounded(&self.root.join("home/config.toml"), /*limit*/ 65_536)?,
            hooks: optional_bounded(&self.root.join("home/hooks.json"), /*limit*/ 65_536)?,
        };
        let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
        ensure!(
            binary.is_absolute() && binary.is_file(),
            "actual binary prerequisite missing"
        );
        let model_listener =
            tokio::time::timeout_at(self.deadline, TcpListener::bind("127.0.0.1:0")).await??;
        let model_address = model_listener.local_addr()?;
        let mcp_listener = self
            .mcp_listener
            .take()
            .ok_or_else(|| anyhow!("owned MCP listener missing"))?;
        let mcp_url = self
            .mcp_url
            .clone()
            .ok_or_else(|| anyhow!("owned MCP URL missing"))?;
        let model_target = format!("/{}/v1/responses", self.expected.nonce);
        let base = format!("http://{model_address}/{}/v1", self.expected.nonce);
        let override_value = toml::Value::String(base);
        let override_argument = format!("openai_base_url={override_value}");
        let mcp_target = format!("/mcp/{}", self.expected.nonce);
        ensure!(
            Instant::now() < self.deadline,
            "owned case deadline expired before peer start"
        );
        let model_peer = HttpPeer::start(
            model_listener,
            PeerProtocol::Responses {
                target: model_target.clone(),
                api_key: API_KEY.to_owned(),
                prompt: PROMPT.to_owned(),
                marker: MARKER.to_owned(),
            },
        );
        let mcp_peer = HttpPeer::start(mcp_listener, PeerProtocol::Mcp { target: mcp_target });
        let mut command = Command::new(&binary);
        command
            .env_clear()
            .current_dir(&self.work)
            .env("SystemRoot", &self.system_root)
            .env("CODEX_HOME", self.root.join("home"))
            .env("CODEX_AUTH_HOME", self.root.join("auth"))
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("profile"))
            .env("LOCALAPPDATA", self.root.join("local"))
            .env("APPDATA", self.root.join("roaming"))
            .env("TEMP", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .env("PATH", self.root.join("path"))
            .env("CODEX_API_KEY", API_KEY)
            .args([
                "-c",
                &override_argument,
                "exec",
                "--json",
                "--ephemeral",
                "--skip-git-repo-check",
                "--color",
                "never",
                "--model",
                "gpt-5.5",
                PROMPT,
            ]);
        let child = match OwnedChild::spawn(command, ChildInput::Null) {
            Ok(child) => child,
            Err(error) => {
                let (model, mcp) = tokio::join!(
                    model_peer.abort(self.deadline),
                    mcp_peer.abort(self.deadline)
                );
                return Err(anyhow!(
                    "actual CLI spawn failed: {error:?}; model: {:?}; MCP: {:?}",
                    model.terminal,
                    mcp.terminal
                ));
            }
        };
        let (cancellation, cancelled) = oneshot::channel();
        let report = child.finish(self.deadline, cancelled).await;
        drop(cancellation);
        let drain_deadline = (Instant::now() + Duration::from_secs(/*secs*/ 5))
            .min(self.deadline + Duration::from_secs(/*secs*/ 5));
        let (exit, model, mcp) = match report.exit {
            Some(exit) if exit.success() && report.terminal.is_ok() && report.cleanup.is_ok() => {
                let (model, mcp) = tokio::join!(
                    model_peer.finish_after_child(exit, drain_deadline),
                    mcp_peer.finish_after_child(exit, drain_deadline)
                );
                (exit, model, mcp)
            }
            Some(_) | None => {
                let (model, mcp) = tokio::join!(
                    model_peer.abort(drain_deadline),
                    mcp_peer.abort(drain_deadline)
                );
                return Err(anyhow!(
                    "actual CLI failed: exit {:?}, terminal {:?}, cleanup {:?}; model {:?}; MCP {:?}",
                    report.exit,
                    report.terminal,
                    report.cleanup,
                    model.terminal,
                    mcp.terminal
                ));
            }
        };
        ensure!(
            model.terminal.is_ok() && mcp.terminal.is_ok(),
            "ordinary peer failed: model {:?}; MCP {:?}",
            model.terminal,
            mcp.terminal
        );
        let after = DeclarationBytes {
            config: read_bounded(&self.root.join("home/config.toml"), /*limit*/ 65_536)?,
            hooks: optional_bounded(&self.root.join("home/hooks.json"), /*limit*/ 65_536)?,
        };
        let marker = optional_bounded(
            &self.effect_directory().join(self.marker_name()),
            /*limit*/ 16_384,
        )?;
        let mut effects = fs::read_dir(self.effect_directory())?
            .take(/*n*/ 4)
            .map(|entry| {
                entry?
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow!("effect name encoding refused"))
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(effects.len() <= 3, "owned effect entry cap exceeded");
        effects.sort();
        let hook_attempts = HookAttemptObservation {
            script_before,
            script_after: read_bounded(self.hook_script_path(), /*limit*/ 32 * 1024)?,
            before: attempts_before,
            after: read_attempts(self.hook_attempt_directory())?,
        };
        let archive = Archive {
            format: "covenant-effects-mcp-observation-v1",
            case: "ordinary_mcp_requests_are_observed",
            mode: "ordinary",
            phase: "first",
            binary: binary
                .to_str()
                .ok_or_else(|| anyhow!("binary encoding refused"))?,
            exit_code: exit.code(),
            stdout: &report.stdout,
            stderr: &report.stderr,
            before: &before,
            after: &after,
            marker: &marker,
            effects: &effects,
            model: ArchivedModel {
                kind: "ordinary_http",
                peer: archived_peer(&model),
            },
            mcp: archived_peer(&mcp),
            hook_attempts: &hook_attempts,
        };
        let receipt = self
            .receipts
            .write_new(ReceiptName::OrdinaryMcpFirst, &archive)?;
        Ok(McpRunObservation {
            binary,
            exit,
            stdout: report.stdout,
            stderr: report.stderr,
            before,
            after,
            model,
            mcp,
            model_target,
            mcp_url,
            marker,
            effects,
            hook_attempts,
            receipt,
        })
    }
}

fn archived_peer(report: &PeerReport) -> ArchivedPeer<'_> {
    ArchivedPeer {
        accepted: report.accepted,
        requests: report
            .requests
            .iter()
            .map(|request| ArchivedRequest {
                method: &request.method,
                target: &request.target,
                headers: &request.headers,
                body: &request.body,
            })
            .collect(),
        terminal: None,
    }
}

fn read_bounded(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(std::io::Error::other("owned file cap/type refused"));
    }
    let mut file = File::open(path)?;
    let mut bytes = vec![0; limit + 1];
    let mut filled = 0;
    loop {
        let count = match file.read(&mut bytes[filled..]) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            break;
        }
        filled += count;
        if filled > limit {
            return Err(std::io::Error::other("owned file cap exceeded"));
        }
    }
    bytes.truncate(filled);
    Ok(bytes)
}

fn optional_bounded(path: &Path, limit: usize) -> std::io::Result<Option<Vec<u8>>> {
    match read_bounded(path, limit) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_attempt_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 45
        && bytes.starts_with(b"attempt-")
        && bytes.ends_with(b".json")
        && bytes[8..40]
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_attempts(directory: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    ensure!(
        fs::symlink_metadata(directory)?.file_type().is_dir(),
        "owned attempt directory type refused"
    );
    let entries = fs::read_dir(directory)?
        .take(/*n*/ 3)
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(entries.len() <= 2, "owned attempt entry cap exceeded");
    let mut attempts = BTreeMap::new();
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("attempt name encoding refused"))?;
        ensure!(
            is_attempt_name(&name) && entry.file_type()?.is_file(),
            "attempt name/type refused"
        );
        ensure!(
            attempts
                .insert(name, read_bounded(&entry.path(), /*limit*/ 16 * 1024)?)
                .is_none(),
            "duplicate attempt name refused"
        );
    }
    Ok(attempts)
}

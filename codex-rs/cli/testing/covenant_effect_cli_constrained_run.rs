//! Actual constrained child with settled canonical proxy and MCP absence peer.
use super::child::ChildInput;
use super::child::OwnedChild;
use super::cli_fixture::DeclarationBytes;
use super::cli_fixture::Fixture;
use super::cli_fixture::HookAttemptObservation;
use super::peer::HttpPeer;
use super::peer::PeerProtocol;
use super::peer::PeerReport;
use super::proxy;
use super::proxy::Capture;
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
struct ArchivedCapture<'a> {
    connections: usize,
    connects: &'a [Vec<u8>],
    handshakes: &'a [(String, Vec<(String, String)>)],
    requests: &'a [serde_json::Value],
    inference_input: &'a [serde_json::Value],
    warmup_completed: bool,
    inference_completed: bool,
    messages: usize,
    qualified_eof_connections: usize,
    failure: Option<&'static str>,
}

#[derive(Serialize)]
struct ArchivedModel<'a> {
    kind: &'static str,
    capture: ArchivedCapture<'a>,
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

pub(super) struct ConstrainedRunObservation {
    pub(super) binary: PathBuf,
    pub(super) exit: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) before: DeclarationBytes,
    pub(super) after: DeclarationBytes,
    pub(super) model: Capture,
    pub(super) mcp: PeerReport,
    pub(super) mcp_url: String,
    pub(super) marker: Option<Vec<u8>>,
    pub(super) effects: Vec<String>,
    pub(super) hook_attempts: HookAttemptObservation,
    pub(super) receipt: PathBuf,
}

impl Fixture {
    pub(super) async fn run_constrained(&mut self) -> Result<ConstrainedRunObservation> {
        let script_before = read_bounded(self.hook_script_path(), /*limit*/ 32 * 1024)?;
        let attempts_before = read_attempts(self.hook_attempt_directory())?;
        let before = declarations(self)?;
        let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
        ensure!(
            binary.is_absolute() && binary.is_file(),
            "actual binary prerequisite missing"
        );
        let mcp_listener = self
            .mcp_listener
            .take()
            .ok_or_else(|| anyhow!("owned MCP listener missing"))?;
        let mcp_url = self
            .mcp_url
            .clone()
            .ok_or_else(|| anyhow!("owned MCP URL missing"))?;
        ensure!(
            Instant::now() < self.deadline,
            "owned case deadline expired before peer start"
        );
        let ca_path = self.root.join("owned-ca.pem");
        let model_peer = tokio::time::timeout_at(self.deadline, proxy::Proxy::start(&ca_path))
            .await
            .map_err(|_| anyhow!("owned proxy preparation deadline exceeded"))??;
        let mcp_peer = HttpPeer::start(
            mcp_listener,
            PeerProtocol::Mcp {
                target: format!("/mcp/{}", self.expected.nonce),
            },
        );
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
            .env("CODEX_API_KEY", proxy::API_KEY)
            .env("HTTPS_PROXY", &model_peer.address)
            .env("CODEX_CA_CERTIFICATE", &ca_path)
            .args([
                "exec",
                "--json",
                "--ephemeral",
                "--skip-git-repo-check",
                "--color",
                "never",
                "--model",
                "gpt-5.5",
                proxy::PROMPT,
            ]);
        let child = match OwnedChild::spawn(command, ChildInput::Null) {
            Ok(child) => child,
            Err(error) => {
                let (model, mcp) = tokio::join!(model_peer.abort(), mcp_peer.abort(self.deadline));
                return Err(anyhow!(
                    "actual CLI spawn failed: {error:?}; model: {}; MCP: {:?}",
                    model.is_ok(),
                    mcp.terminal
                ));
            }
        };
        let (cancellation, cancelled) = oneshot::channel();
        let report = child.finish(self.deadline, cancelled).await;
        drop(cancellation);
        let drain_deadline = (Instant::now() + Duration::from_secs(/*secs*/ 5))
            .min(self.deadline + Duration::from_secs(/*secs*/ 5));
        let successful = report.exit.is_some_and(|exit| exit.success())
            && report.terminal.is_ok()
            && report.cleanup.is_ok();
        let (model, mcp) = match report.exit.filter(|_| successful) {
            Some(exit) => tokio::join!(
                model_peer.finish_after_child(exit),
                mcp_peer.finish_after_child(exit, drain_deadline)
            ),
            None => tokio::join!(model_peer.abort(), mcp_peer.abort(drain_deadline)),
        };
        let model = model?;
        if !successful {
            return Err(anyhow!(
                "actual CLI failed: exit {:?}, terminal {:?}, cleanup {:?}; model {:?}; MCP {:?}",
                report.exit,
                report.terminal,
                report.cleanup,
                model.failure,
                mcp.terminal
            ));
        }
        ensure!(
            model.failure.is_none() && mcp.terminal.is_ok(),
            "constrained peer failed"
        );
        let exit = report
            .exit
            .ok_or_else(|| anyhow!("actual CLI exit missing"))?;
        let after = declarations(self)?;
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
            format: "covenant-effects-absence-observation-v1",
            case: "covenant_hook_mcp_effects_are_absent",
            mode: "covenant",
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
                kind: "canonical_sc5",
                capture: archived_capture(&model),
            },
            mcp: archived_peer(&mcp),
            hook_attempts: &hook_attempts,
        };
        let receipt = self
            .receipts
            .write_new(ReceiptName::CovenantAbsenceFirst, &archive)?;
        Ok(ConstrainedRunObservation {
            binary,
            exit,
            stdout: report.stdout,
            stderr: report.stderr,
            before,
            after,
            model,
            mcp,
            mcp_url,
            marker,
            effects,
            hook_attempts,
            receipt,
        })
    }
}

fn declarations(fixture: &Fixture) -> Result<DeclarationBytes> {
    Ok(DeclarationBytes {
        config: read_bounded(
            &fixture.root.join("home/config.toml"),
            /*limit*/ 65_536,
        )?,
        hooks: optional_bounded(&fixture.root.join("home/hooks.json"), /*limit*/ 65_536)?,
    })
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

fn archived_capture(capture: &Capture) -> ArchivedCapture<'_> {
    ArchivedCapture {
        connections: capture.connections,
        connects: &capture.connects,
        handshakes: &capture.handshakes,
        requests: &capture.requests,
        inference_input: &capture.inference_input,
        warmup_completed: capture.warmup_completed,
        inference_completed: capture.inference_completed,
        messages: capture.messages,
        qualified_eof_connections: capture.qualified_eof_connections,
        failure: capture.failure,
    }
}

fn read_bounded(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(std::io::Error::other("owned file cap/type refused"));
    }
    let mut bytes = Vec::with_capacity((metadata.len() as usize).min(limit));
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::other("owned file cap exceeded"));
    }
    Ok(bytes)
}

fn optional_bounded(path: &Path, limit: usize) -> std::io::Result<Option<Vec<u8>>> {
    match read_bounded(path, limit) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
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
        let bytes = name.as_bytes();
        ensure!(
            bytes.len() == 45
                && bytes.starts_with(b"attempt-")
                && bytes.ends_with(b".json")
                && bytes[8..40]
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
                && entry.file_type()?.is_file(),
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

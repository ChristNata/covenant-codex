//! Actual ordinary child, retained peer settlement, and complete owned receipts.
use super::HttpRequest;
use super::child::ChildInput;
use super::child::OwnedChild;
use super::cli_fixture::API_KEY;
use super::cli_fixture::DeclarationBytes;
use super::cli_fixture::Fixture;
use super::cli_fixture::HookAttemptObservation;
use super::cli_fixture::MARKER;
use super::cli_fixture::PROMPT;
use super::cli_fixture::RunLabel;
use super::cli_fixture::RunObservation;
use super::peer::HttpPeer;
use super::peer::PeerProtocol;
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
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::Instant;

// Borrow the bounded captured bytes instead of constructing per-byte JSON Values.
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
    mcp: Option<()>,
    hook_attempts: &'a HookAttemptObservation,
}

impl Fixture {
    pub(super) async fn run(&self, label: RunLabel) -> Result<RunObservation> {
        let script_before = read_bounded(self.hook_script_path(), /*limit*/ 32 * 1024)?;
        let attempts_before = read_attempts(self.hook_attempt_directory())?;
        let (phase, name) = match label {
            RunLabel::First => ("first", ReceiptName::OrdinaryHookFirst),
            RunLabel::ExistingMarker => {
                ("existing-marker", ReceiptName::OrdinaryHookExistingMarker)
            }
        };
        let before = DeclarationBytes {
            config: read_bounded(&self.root.join("home/config.toml"), /*limit*/ 65_536)?,
            hooks: Some(read_bounded(
                &self.root.join("home/hooks.json"),
                /*limit*/ 65_536,
            )?),
        };
        // The external runner must bind this to the inspected non-libtest artifact.
        let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
        ensure!(
            binary.is_absolute() && binary.is_file(),
            "actual binary prerequisite missing"
        );
        let listener =
            tokio::time::timeout_at(self.deadline, TcpListener::bind("127.0.0.1:0")).await??;
        let address = listener.local_addr()?;
        let nonce = &self.expected.nonce;
        let model_target = format!("/{nonce}/v1/responses");
        let base = format!("http://{address}/{nonce}/v1");
        let override_value = toml::Value::String(base);
        let override_argument = format!("openai_base_url={override_value}");
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
                "--enable",
                "hooks",
                "-c",
                &override_argument,
                "exec",
                "--dangerously-bypass-hook-trust",
                "--json",
                "--ephemeral",
                "--skip-git-repo-check",
                "--color",
                "never",
                "--model",
                "gpt-5.5",
                PROMPT,
            ]);
        ensure!(
            Instant::now() < self.deadline,
            "owned case deadline expired before spawn"
        );
        let peer = HttpPeer::start(
            listener,
            PeerProtocol::Responses {
                target: model_target.clone(),
                api_key: API_KEY.to_owned(),
                prompt: PROMPT.to_owned(),
                marker: MARKER.to_owned(),
            },
        );
        // After start, all failure exits first await the retained peer owner.
        let child = match OwnedChild::spawn(command, ChildInput::Null) {
            Ok(child) => child,
            Err(error) => {
                let aborted = peer.abort(self.deadline).await;
                return Err(anyhow!(
                    "actual CLI spawn failed: {error:?}; peer: {:?}",
                    aborted.terminal
                ));
            }
        };
        let (cancellation, cancelled) = oneshot::channel();
        // Keep the sender and this same finish future alive through direct-child wait and all I/O joins.
        let report = child.finish(self.deadline, cancelled).await;
        drop(cancellation);
        let drain_deadline = (Instant::now() + Duration::from_secs(/*secs*/ 5))
            .min(self.deadline + Duration::from_secs(/*secs*/ 5));
        let (exit, model) = match report.exit {
            Some(exit) if exit.success() && report.terminal.is_ok() && report.cleanup.is_ok() => {
                (exit, peer.finish_after_child(exit, drain_deadline).await)
            }
            Some(_) | None => {
                let aborted = peer.abort(drain_deadline).await;
                return Err(anyhow!(
                    "actual CLI failed: exit {:?}, terminal {:?}, cleanup {:?}; peer {:?}",
                    report.exit,
                    report.terminal,
                    report.cleanup,
                    aborted.terminal
                ));
            }
        };
        // All live owners have now settled. No Drop, EOF shortcut or timeout supplies a report.
        ensure!(
            model.terminal.is_ok(),
            "ordinary model peer failed: {:?}",
            model.terminal
        );
        let after = DeclarationBytes {
            config: read_bounded(&self.root.join("home/config.toml"), /*limit*/ 65_536)?,
            hooks: Some(read_bounded(
                &self.root.join("home/hooks.json"),
                /*limit*/ 65_536,
            )?),
        };
        let marker = match read_bounded(
            &self.effect_directory().join(self.marker_name()),
            /*limit*/ 16_384,
        ) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
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
        let requests = model
            .requests
            .iter()
            .map(|request: &HttpRequest| ArchivedRequest {
                method: &request.method,
                target: &request.target,
                headers: &request.headers,
                body: &request.body,
            })
            .collect::<Vec<_>>();
        let archive = Archive {
            format: "covenant-effects-hook-observation-v2",
            case: "ordinary_hook_effect_is_observed",
            mode: "ordinary",
            phase,
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
                peer: ArchivedPeer {
                    // Null encodes the actual Ok terminal checked above; failed peers never reach this writer.
                    accepted: model.accepted,
                    requests,
                    terminal: None,
                },
            },
            mcp: None,
            hook_attempts: &hook_attempts,
        };
        let receipt = self.receipts.write_new(name, &archive)?;
        Ok(RunObservation {
            binary,
            exit,
            stdout: report.stdout,
            stderr: report.stderr,
            before,
            after,
            model,
            mcp: None,
            model_target,
            mcp_url: None,
            marker,
            effects,
            hook_attempts,
            receipt,
        })
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
    drop(file);
    bytes.truncate(filled);
    Ok(bytes)
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

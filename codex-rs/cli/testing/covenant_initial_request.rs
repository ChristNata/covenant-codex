//! Actual Cargo-built CLI initial request; no release-byte or native-gate claim.
#![cfg(all(windows, feature = "covenant"))]

#[path = "covenant_responses_proxy.rs"]
mod proxy;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const OUTPUT_LIMIT: usize = 64 * 1024;
const CASE_DEADLINE: Duration = Duration::from_secs(60);
const CLEANUP_BUDGET: Duration = Duration::from_secs(5);

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        for name in [
            "work", "home", "auth", "profile", "local", "roaming", "temp", "path",
        ] {
            fs::create_dir(root.join(name))?;
        }
        let project = root.join("work");
        let project_key = project
            .to_str()
            .ok_or_else(|| anyhow!("owned path encoding refused"))?;
        let config = toml::Value::try_from(json!({
            "cli_auth_credentials_store": "file",
            "forced_login_method": "api",
            "projects": {project_key: {"trust_level":"trusted"}}
        }))?;
        fs::write(root.join("home/config.toml"), toml::to_string(&config)?)?;
        Ok(Self {
            _temporary: temporary,
            root,
        })
    }

    async fn run(&self) -> Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>, proxy::Capture)> {
        let deadline = tokio::time::Instant::now() + CASE_DEADLINE;
        // Author runner must first bind this to the inspected non-libtest artifact.
        let executable = codex_utils_cargo_bin::cargo_bin("codex")?;
        ensure!(
            executable.is_absolute(),
            "actual binary path must be absolute"
        );
        let ca_path = self.root.join("owned-ca.pem");
        let peer = tokio::time::timeout_at(deadline, proxy::Proxy::start(&ca_path))
            .await
            .map_err(|_| anyhow!("owned listener preparation deadline exceeded"))??;
        let mut command = Command::new(executable);
        command
            .env_clear()
            .current_dir(self.root.join("work"))
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
            .env("HTTPS_PROXY", &peer.address)
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
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                let settled = peer.abort().await;
                settled?;
                return Err(anyhow!("owned actual CLI spawn failed"));
            }
        };
        // No question-mark or assertion bypasses cleanup after successful spawn.
        let result = match (child.stdout.take(), child.stderr.take()) {
            (Some(stdout), Some(stderr)) => tokio::time::timeout_at(deadline, async {
                let exit = async {
                    child
                        .wait()
                        .await
                        .map_err(|_| anyhow!("owned child wait failed"))
                };
                tokio::try_join!(exit, read_output(stdout), read_output(stderr))
            })
            .await
            .map_err(|_| anyhow!("initial request deadline exceeded"))
            .and_then(|value| value),
            _ => Err(anyhow!("owned child pipe missing")),
        };
        // Reader futures own their pipes and have settled or been dropped above; none are spawned.
        let cleanup_started = Instant::now();
        let child_cleanup = async {
            if child.id().is_some() {
                let termination = child.start_kill();
                // Attempt the owned wait even if the termination call itself fails.
                tokio::time::timeout(CLEANUP_BUDGET, child.wait())
                    .await
                    .map_err(|_| anyhow!("owned child cleanup deadline exceeded"))?
                    .map_err(|_| anyhow!("owned child cleanup wait failed"))?;
                termination.map_err(|_| anyhow!("owned child termination failed"))?;
            }
            Ok::<(), anyhow::Error>(())
        };
        let proxy_cleanup = async {
            match &result {
                Ok((exit, _, _)) => peer.finish_after_child(*exit).await,
                Err(_) => peer.abort().await,
            }
        };
        let (child_settled, capture) = tokio::join!(child_cleanup, proxy_cleanup);
        // Both cleanup branches ran even when child IO, timeout, transport, or either cleanup failed.
        child_settled?;
        let capture = capture?;
        ensure!(
            cleanup_started.elapsed() <= CLEANUP_BUDGET,
            "owned cleanup budget exceeded"
        );
        let (exit, stdout, stderr) = result?;
        Ok((exit, stdout, stderr, capture))
    }
}

async fn read_output(mut input: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut block = [0; 4096];
    loop {
        // One sentinel byte distinguishes exact-cap EOF from overflow before another allocation.
        let allowed = (OUTPUT_LIMIT + 1 - output.len()).min(block.len());
        let count = input
            .read(&mut block[..allowed])
            .await
            .map_err(|_| anyhow!("owned output read failed"))?;
        if count == 0 {
            return Ok(output);
        }
        ensure!(
            output.len() + count <= OUTPUT_LIMIT,
            "owned output cap exceeded"
        );
        output.extend_from_slice(&block[..count]);
    }
}

#[derive(Debug, PartialEq)]
struct Outcome {
    native_success: bool,
    proxy_clean: bool,
    attempts_bounded_and_captured: bool,
    authenticated_upgrades: bool,
    warmup_count: usize,
    inference_count: usize,
    completed_counts_agree: bool,
    actual_jsonl_matches: bool,
    resolved_input_observed: bool,
    actual_catalogs_match: bool,
}

#[tokio::test(flavor = "current_thread")]
async fn covenant_initial_request_captures_canonical_wss_catalog_and_jsonl_completion() -> Result<()>
{
    let fixture = Fixture::new()?;
    let (exit, stdout, _stderr, capture) = fixture.run().await?;
    // All assertions and observation formatting happen after child/server settlement.
    let events = stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice::<Value>(line).map_err(|_| anyhow!("owned JSONL refused"))
        })
        .collect::<Result<Vec<_>>>()?;
    let thread_id = events
        .first()
        .and_then(|value| value.get("thread_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("actual thread id missing"))?;
    let message_id = events
        .get(2)
        .and_then(|value| value.pointer("/item/id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("actual message id missing"))?;
    let expected_events = vec![
        json!({"type":"thread.started","thread_id":thread_id}),
        json!({"type":"turn.started"}),
        json!({"type":"item.completed","item":{"id":message_id,"type":"agent_message","text":proxy::MARKER}}),
        json!({"type":"turn.completed","usage":{"input_tokens":11,"cached_input_tokens":3,
            "cache_write_input_tokens":2,"output_tokens":7,"reasoning_output_tokens":5}}),
    ];
    let warmups = capture
        .requests
        .iter()
        .filter(|request| request.get("generate") == Some(&Value::Bool(false)))
        .count();
    let inferences = capture.requests.len() - warmups;
    let first_tools = capture
        .requests
        .first()
        .and_then(|request| request.get("tools"));
    let actual_catalogs_match = first_tools.is_some_and(|first| {
        capture
            .requests
            .iter()
            .all(|request| request.get("tools") == Some(first))
    });
    let expected_auth = format!("Bearer {}", proxy::API_KEY);
    let authenticated_upgrades = !capture.handshakes.is_empty()
        && capture.handshakes.iter().all(|(sni, headers)| {
            sni == "api.openai.com"
                && headers
                    .iter()
                    .filter(|(name, value)| name == "authorization" && value == &expected_auth)
                    .count()
                    == 1
                && headers
                    .iter()
                    .filter(|(name, value)| name == "host" && value == "api.openai.com")
                    .count()
                    == 1
        });
    let observed = Outcome {
        native_success: exit.success(),
        proxy_clean: capture.failure.is_none(),
        attempts_bounded_and_captured: (1..=4).contains(&capture.connections)
            && capture.connects.len() == capture.connections
            && capture.handshakes.len() == capture.connections,
        authenticated_upgrades,
        warmup_count: warmups,
        inference_count: inferences,
        completed_counts_agree: warmups <= 1
            && capture.warmup_completed == (warmups == 1)
            && capture.inference_completed,
        actual_jsonl_matches: events == expected_events,
        resolved_input_observed: !capture.inference_input.is_empty(),
        actual_catalogs_match,
    };
    assert_eq!(
        observed,
        Outcome {
            native_success: true,
            proxy_clean: true,
            attempts_bounded_and_captured: true,
            authenticated_upgrades: true,
            warmup_count: usize::from(capture.warmup_completed),
            inference_count: 1,
            completed_counts_agree: true,
            actual_jsonl_matches: true,
            resolved_input_observed: true,
            actual_catalogs_match: true,
        }
    );
    // Emit the complete actual canonical catalog only after closure succeeds, without headers/input.
    let catalog =
        serde_json::to_vec(first_tools.ok_or_else(|| anyhow!("captured catalog missing"))?)?;
    ensure!(
        catalog.len() <= OUTPUT_LIMIT,
        "catalog review output cap exceeded"
    );
    println!(
        "COVENANT_SC5_CAPTURED_TOOLS={}",
        String::from_utf8(catalog)?
    );
    println!(
        "COVENANT_SC5_COUNTS=warmup:{warmups},inference:{inferences},connections:{}",
        capture.connections
    );
    Ok(())
}

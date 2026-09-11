//! Process client for the independently supplied Covenant decision sidecar.
//!
//! This module only transports a frozen decision envelope.  It does not grant
//! native execution or filesystem authority; callers must perform their native
//! precommit checks before acting on [`SidecarDecision::Allow`].

use serde::Deserialize;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const CLIENT: &str = "codex";

/// A parsed response from `covenant-cli hook decide --client codex`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidecarDecision {
    /// The sidecar explicitly authorized the frozen request.
    Allow,
    /// The sidecar refused the request.
    Deny { reason: String },
    /// The sidecar authorized the request and returned bounded context.
    AllowWithContext { context: String },
}

/// Errors returned when the sidecar transport or response contract fails.
#[derive(Debug)]
pub enum SidecarError {
    EmptyPath,
    RequestTooLarge,
    Deadline,
    Spawn(std::io::Error),
    Write(std::io::Error),
    Exit,
    ResponseTooLarge,
    Malformed,
}

impl std::fmt::Display for SidecarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyPath => "sidecar executable path is empty",
            Self::RequestTooLarge => "sidecar request exceeds the maximum input size",
            Self::Deadline => "sidecar did not finish before the deadline",
            Self::Spawn(_) => "could not start sidecar",
            Self::Write(_) => "could not write sidecar request",
            Self::Exit => "sidecar exited unsuccessfully",
            Self::ResponseTooLarge => "sidecar response is too large",
            Self::Malformed => "sidecar response is malformed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SidecarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(error) | Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

/// A bounded client for the real, pinned Covenant sidecar executable.
#[derive(Clone, Debug)]
pub struct SidecarClient {
    executable: PathBuf,
    deadline: Duration,
}

impl SidecarClient {
    /// Creates a client.  The executable is not downloaded or substituted.
    pub fn new(executable: impl Into<PathBuf>, deadline: Duration) -> Result<Self, SidecarError> {
        let executable = executable.into();
        if executable.as_os_str().is_empty() {
            return Err(SidecarError::EmptyPath);
        }
        Ok(Self {
            executable,
            deadline,
        })
    }

    /// Returns the exact executable selected for this client.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Sends one frozen envelope to the real sidecar and parses one response.
    ///
    /// Any timeout, non-zero exit, malformed output, or unsupported decision
    /// fails closed as an error.  No fallback policy is evaluated here.
    pub fn decide(&self, request: &[u8]) -> Result<SidecarDecision, SidecarError> {
        if request.len() > MAX_OUTPUT_BYTES {
            return Err(SidecarError::RequestTooLarge);
        }
        let mut child = Command::new(&self.executable)
            .args([OsString::from("hook"), OsString::from("decide")])
            .args([OsString::from("--client"), OsString::from(CLIENT)])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(SidecarError::Spawn)?;
        child
            .stdin
            .take()
            .ok_or_else(|| SidecarError::Write(std::io::Error::other("missing stdin")))?
            .write_all(request)
            .map_err(SidecarError::Write)?;

        let started = Instant::now();
        loop {
            if child.try_wait().map_err(SidecarError::Spawn)?.is_some() {
                break;
            }
            if started.elapsed() >= self.deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SidecarError::Deadline);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let output = child.wait_with_output().map_err(SidecarError::Spawn)?;
        if !output.status.success() {
            return Err(SidecarError::Exit);
        }
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(SidecarError::ResponseTooLarge);
        }
        parse_response(&output.stdout)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDecision {
    decision: WireDecisionKind,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WireDecisionKind {
    Allow,
    Deny,
    AllowWithContext,
}

fn parse_response(bytes: &[u8]) -> Result<SidecarDecision, SidecarError> {
    let response: WireDecision =
        serde_json::from_slice(bytes).map_err(|_| SidecarError::Malformed)?;
    match response.decision {
        WireDecisionKind::Allow if response.reason.is_none() && response.context.is_none() => {
            Ok(SidecarDecision::Allow)
        }
        WireDecisionKind::Deny if response.context.is_none() => Ok(SidecarDecision::Deny {
            reason: response
                .reason
                .filter(|reason| !reason.is_empty())
                .ok_or(SidecarError::Malformed)?,
        }),
        WireDecisionKind::AllowWithContext if response.reason.is_none() => {
            Ok(SidecarDecision::AllowWithContext {
                context: response
                    .context
                    .filter(|context| !context.is_empty())
                    .ok_or(SidecarError::Malformed)?,
            })
        }
        _ => Err(SidecarError::Malformed),
    }
}

#[cfg(test)]
#[path = "sidecar_tests.rs"]
mod tests;

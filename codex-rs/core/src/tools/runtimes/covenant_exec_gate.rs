//! Covenant's pre-spawn exec gate.
//!
//! The request is assembled only after unified exec has finished its command
//! and environment transforms. The exact bytes sent to the sidecar therefore
//! describe the launch that will be handed to the process manager.

use codex_covenant::ExecEvent;
use codex_covenant::FinalExecInput;
use codex_covenant::FrozenWindowsEnvironment;
use codex_covenant::NetworkAccess;
use codex_covenant::SidecarClient;
use codex_covenant::SidecarDecision;
use codex_utils_path_uri::PathUri;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

const DECIDER_PATH_ENV: &str = "COVENANT_DECIDER_PATH";
const DECISION_DEADLINE: Duration = Duration::from_secs(2);

/// Build and submit the immutable exec envelope before a process is spawned.
pub(crate) fn authorize_exec(
    command: &[String],
    cwd: &PathUri,
    env: &HashMap<String, String>,
    sandbox: &str,
    network_allowed: bool,
) -> Result<(), String> {
    let request = build_request(command, cwd, env, sandbox, network_allowed)?;
    let decider_path = std::env::var_os(DECIDER_PATH_ENV)
        .ok_or_else(|| format!("{DECIDER_PATH_ENV} is required for Covenant exec authorization"))?;
    let client = SidecarClient::new(decider_path, DECISION_DEADLINE)
        .map_err(|error| format!("failed to configure Covenant exec sidecar: {error}"))?;
    match client
        .decide(&request)
        .map_err(|error| format!("Covenant exec sidecar failed: {error}"))?
    {
        SidecarDecision::Allow => Ok(()),
        SidecarDecision::Deny { reason } => {
            Err(format!("Covenant exec sidecar denied launch: {reason}"))
        }
        SidecarDecision::AllowWithContext { .. } => Err(
            "Covenant exec sidecar returned ALLOW_WITH_CONTEXT; exact ALLOW is required"
                .to_string(),
        ),
    }
}

fn build_request(
    command: &[String],
    cwd: &PathUri,
    env: &HashMap<String, String>,
    sandbox: &str,
    network_allowed: bool,
) -> Result<Vec<u8>, String> {
    let (program, argument_tail) = command
        .split_first()
        .ok_or_else(|| "Covenant exec authorization requires a command".to_string())?;
    let cwd = cwd
        .to_abs_path()
        .map_err(|error| format!("Covenant exec cwd is invalid: {error}"))?;
    let environment = FrozenWindowsEnvironment::from_final_pairs(
        env.iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value))),
    )
    .map_err(|_| "Covenant exec environment is invalid".to_string())?;
    let event = ExecEvent::from_final(FinalExecInput {
        program: PathBuf::from(program),
        argument_tail: argument_tail.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
        environment,
        sandbox: sandbox.to_string(),
        network: if network_allowed {
            NetworkAccess::Allowed
        } else {
            NetworkAccess::Denied
        },
    })
    .map_err(|_| "Covenant exec request contains invalid native launch facts".to_string())?;
    Ok(event.as_stdin().to_vec())
}

#[cfg(test)]
mod tests {
    use super::build_request;
    use codex_utils_path_uri::PathUri;
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn request_contains_final_exec_facts_and_scrubs_decider_control() {
        let cwd = PathUri::parse("file:///C:/work").expect("absolute Windows cwd");
        let env = HashMap::from([
            ("PATH".to_string(), r"C:\Windows\System32".to_string()),
            (
                "COVENANT_DECIDER_PATH".to_string(),
                r"C:\private\covenant-cli.exe".to_string(),
            ),
        ]);
        let request = build_request(
            &[
                r"C:\Windows\System32\cmd.exe".to_string(),
                "/c".to_string(),
                "echo ok".to_string(),
            ],
            &cwd,
            &env,
            "workspace-write",
            true,
        )
        .expect("valid exec request");
        let value: Value = serde_json::from_slice(&request).expect("valid sidecar envelope");
        assert_eq!(value["decide_v1"]["kind"], "exec");
        assert_eq!(
            value["decide_v1"]["exec"]["program"],
            r"C:\Windows\System32\cmd.exe"
        );
        assert_eq!(value["decide_v1"]["exec"]["network"], true);
        assert!(
            value["decide_v1"]["exec"]["env"]
                .as_object()
                .expect("environment object")
                .get("COVENANT_DECIDER_PATH")
                .is_none()
        );
    }

    #[test]
    fn request_refuses_empty_commands_before_any_sidecar_call() {
        let cwd = PathUri::parse("file:///C:/work").expect("absolute Windows cwd");
        let result = build_request(&[], &cwd, &HashMap::new(), "workspace-write", false);
        assert_eq!(
            result,
            Err("Covenant exec authorization requires a command".to_string())
        );
    }
}

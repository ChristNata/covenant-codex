//! Binds the sole managed MCP client to Covenant's launcher-owned installation.

use crate::LaunchContract;
use std::path::Path;
use std::path::PathBuf;

const NAMESPACE_PLACEHOLDER: &str = "${COVENANT_MCP_NAMESPACE}";

/// Validated fanin startup facts. This is not permission for an upstream effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaninBinding {
    command: PathBuf,
    config: PathBuf,
    namespace: String,
}

/// Content-free refusal of a non-managed fanin launch shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaninBindingError;

impl std::fmt::Display for FaninBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid managed fanin binding")
    }
}

impl std::error::Error for FaninBindingError {}

impl FaninBinding {
    /// Accept only the sibling gateway, generated config and scoped namespace.
    ///
    /// The current Harness template passes a literal namespace placeholder;
    /// the caller must replace that exact argument with `namespace()` before spawn.
    pub fn from_managed_launch(
        contract: &LaunchContract,
        command: &str,
        args: &[String],
        namespace: &str,
    ) -> Result<Self, FaninBindingError> {
        let root = contract.decider_path().parent().ok_or(FaninBindingError)?;
        if contract
            .decider_path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !name.eq_ignore_ascii_case("covenant-cli.exe"))
            || !valid_namespace(namespace)
        {
            return Err(FaninBindingError);
        }

        let command = PathBuf::from(command);
        let config = PathBuf::from(args.get(1).ok_or(FaninBindingError)?);
        if command != root.join("fanin-mcp.exe")
            || config != root.join("mcp").join("config.toml")
            || args.len() != 4
            || args[0] != "--config"
            || args[2] != "--namespace"
            || (args[3] != NAMESPACE_PLACEHOLDER && args[3] != namespace)
            || command.to_str().is_none()
            || config.to_str().is_none()
        {
            return Err(FaninBindingError);
        }

        Ok(Self {
            command,
            config,
            namespace: namespace.to_string(),
        })
    }

    pub fn command(&self) -> &Path {
        &self.command
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

fn valid_namespace(namespace: &str) -> bool {
    let scoped = namespace.strip_suffix("_explorer").unwrap_or(namespace);
    scoped.strip_prefix("p_").is_some_and(|short| {
        short.len() == 8
            && short
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

#[cfg(test)]
#[path = "fanin_binding_tests.rs"]
mod tests;

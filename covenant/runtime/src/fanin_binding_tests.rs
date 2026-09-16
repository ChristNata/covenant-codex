use super::*;
use crate::StartupControls;
use pretty_assertions::assert_eq;
use std::ffi::OsString;

fn managed_contract() -> LaunchContract {
    LaunchContract::from_startup(StartupControls {
        decider_path: Some(OsString::from(r"C:\Covenant\covenant-cli.exe")),
        decider_sha256: Some(OsString::from("0".repeat(64))),
        child_marker: Some(OsString::from("codex-child")),
    })
    .expect("managed contract")
}

fn managed_args(namespace_arg: &str) -> Vec<String> {
    vec![
        "--config".to_string(),
        r"C:\Covenant\mcp\config.toml".to_string(),
        "--namespace".to_string(),
        namespace_arg.to_string(),
    ]
}

#[test]
fn covenant_fanin_binding_accepts_managed_project_and_explorer_namespaces() {
    let contract = managed_contract();
    for namespace in ["p_47de8f8d", "p_47de8f8d_explorer"] {
        assert_eq!(
            FaninBinding::from_managed_launch(
                &contract,
                r"C:\Covenant\fanin-mcp.exe",
                &managed_args(NAMESPACE_PLACEHOLDER),
                namespace,
            ),
            Ok(FaninBinding {
                command: PathBuf::from(r"C:\Covenant\fanin-mcp.exe"),
                config: PathBuf::from(r"C:\Covenant\mcp\config.toml"),
                namespace: namespace.to_string(),
            })
        );
    }

    assert_eq!(
        FaninBinding::from_managed_launch(
            &contract,
            r"C:\Covenant\fanin-mcp.exe",
            &managed_args("p_47de8f8d"),
            "p_47de8f8d",
        ),
        Ok(FaninBinding {
            command: PathBuf::from(r"C:\Covenant\fanin-mcp.exe"),
            config: PathBuf::from(r"C:\Covenant\mcp\config.toml"),
            namespace: "p_47de8f8d".to_string(),
        })
    );
}

#[test]
fn covenant_fanin_binding_refuses_non_managed_paths_flags_and_namespace() {
    let contract = managed_contract();
    let cases = [
        (
            r"C:\Other\fanin-mcp.exe",
            managed_args(NAMESPACE_PLACEHOLDER),
            "p_47de8f8d",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            vec![
                "--config".to_string(),
                r"C:\Other\config.toml".to_string(),
                "--namespace".to_string(),
                NAMESPACE_PLACEHOLDER.to_string(),
            ],
            "p_47de8f8d",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            managed_args("global"),
            "p_47de8f8d",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            managed_args(NAMESPACE_PLACEHOLDER),
            "p_47de8f8d_other",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            managed_args(NAMESPACE_PLACEHOLDER),
            "global",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            managed_args(NAMESPACE_PLACEHOLDER),
            "global_explorer",
        ),
        (
            r"C:\Covenant\fanin-mcp.exe",
            managed_args(NAMESPACE_PLACEHOLDER),
            "p_47de8f8g",
        ),
    ];
    assert_eq!(
        cases.map(|(command, args, namespace)| {
            FaninBinding::from_managed_launch(&contract, command, &args, namespace)
        }),
        std::array::from_fn(|_| Err(FaninBindingError))
    );
}

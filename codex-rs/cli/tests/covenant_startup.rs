//! Windows Cargo-built CLI fixture draft; target registration is pending.
#![cfg(all(windows, feature = "covenant-startup-test-fixture"))]
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use anyhow::Context;
use pretty_assertions::assert_eq;
use serde::Deserialize;

const LAUNCHER_PATH: &str = "covenant-launcher-path";
#[cfg(not(feature = "covenant"))]
const DOTENV_PATH: &str = "covenant-dotenv-path";

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    _held_lock: File,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        for directory in [
            "home/tmp/arg0/stale",
            "home/tmp/arg0/locked",
            "profile",
            "local",
            "roaming",
            "temp",
            "cwd",
        ] {
            fs::create_dir_all(root.join(directory))?;
        }
        for (path, contents) in initial_tree() {
            if !path.ends_with('/') {
                fs::write(root.join("home/tmp/arg0").join(path), contents)?;
            }
        }
        fs::write(
            root.join("home/.env"),
            "COVENANT_STARTUP_CANARY=dotenv-overwrite\nPATH=covenant-dotenv-path\n",
        )?;
        let held_lock = File::options()
            .read(true)
            .write(true)
            .open(root.join("home/tmp/arg0/locked/.lock"))?;
        held_lock.try_lock()?;
        Ok(Self {
            _temporary: temporary,
            root,
            _held_lock: held_lock,
        })
    }

    fn command(&self, name: &str) -> anyhow::Result<assert_cmd::Command> {
        let executable = codex_utils_cargo_bin::cargo_bin(name)?;
        let mut command = assert_cmd::Command::new(executable);
        command
            .env_clear()
            .current_dir(self.root.join("cwd"))
            .env("CODEX_HOME", self.root.join("home"))
            .env("HOME", self.root.join("profile"))
            .env("USERPROFILE", self.root.join("profile"))
            .env("LOCALAPPDATA", self.root.join("local"))
            .env("APPDATA", self.root.join("roaming"))
            .env("TEMP", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .env(
                "SystemRoot",
                std::env::var_os("SystemRoot").context("Windows SystemRoot required")?,
            )
            .env("PATH", LAUNCHER_PATH)
            .env("COVENANT_STARTUP_CANARY", "launcher-kept")
            .timeout(Duration::from_secs(/*secs*/ 15));
        Ok(command)
    }
}

fn initial_tree() -> BTreeMap<String, String> {
    [
        ("locked/", ""),
        ("locked/.lock", ""),
        ("locked/owned.txt", "locked-before"),
        ("stale/", ""),
        ("stale/.lock", ""),
        ("stale/owned.txt", "stale-before"),
    ]
    .map(|(path, content)| (path.to_owned(), content.to_owned()))
    .into()
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Observation {
    canary: Option<String>,
    path_before: String,
    path_after: String,
    executable: PathBuf,
    self_executable: Option<PathBuf>,
    tree: BTreeMap<String, String>,
}

fn observe(fixture: &Fixture) -> anyhow::Result<Observation> {
    // assert_cmd buffers output; the size checks below are not streaming caps.
    let output = fixture
        .command("codex-covenant-startup-observer")?
        .arg(&fixture.root)
        .output()?;
    assert_eq!(
        (
            output.status.code(),
            output.stderr.len(),
            output.stdout.len() <= 16_385
        ),
        (Some(0), 0, true)
    );
    let observed: Observation = serde_json::from_slice(&output.stdout)?;
    let executable =
        codex_utils_cargo_bin::cargo_bin("codex-covenant-startup-observer")?.canonicalize()?;
    assert_eq!(
        (
            observed.executable.canonicalize()?,
            observed
                .self_executable
                .as_ref()
                .context("missing self path")?
                .canonicalize()?
        ),
        (executable.clone(), executable)
    );
    Ok(observed)
}

fn snapshot(
    root: &Path,
    relative: &Path,
    tree: &mut BTreeMap<String, String>,
) -> anyhow::Result<()> {
    anyhow::ensure!(relative.components().count() <= 4, "fixture tree too deep");
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        let name = path
            .to_str()
            .context("non-Unicode fixture path")?
            .replace('\\', "/");
        anyhow::ensure!(tree.len() < 32, "fixture tree too large");
        let kind = entry.file_type()?;
        if kind.is_dir() {
            tree.insert(format!("{name}/"), String::new());
            snapshot(root, &path, tree)?;
        } else {
            anyhow::ensure!(kind.is_file(), "unexpected fixture entry type");
            let file = File::open(entry.path())?;
            let mut bytes = Vec::new();
            // Zero length proves the complete empty byte sequence without a locked ReadFile.
            if file.metadata()?.len() != 0 {
                file.take(/*limit*/ 4097).read_to_end(&mut bytes)?;
            }
            anyhow::ensure!(bytes.len() <= 4096, "fixture file too large");
            tree.insert(name, String::from_utf8(bytes)?);
        }
    }
    Ok(())
}

fn display(output: &Output) -> anyhow::Result<(Option<i32>, bool, bool, bool)> {
    anyhow::ensure!(
        output.stdout.len() <= 16_384 && output.stderr.len() <= 16_384,
        "CLI output exceeds bound"
    );
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    Ok((
        output.status.code(),
        stdout.starts_with("Codex CLI") || stdout.starts_with("codex-cli "),
        stderr.contains("unexpected argument '--covenant-owned-invalid'"),
        stderr.is_empty(),
    ))
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_startup_observer_preserves_launcher_inputs_and_owned_tree() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let observed = observe(&fixture)?;
    let expected = Observation {
        canary: Some("launcher-kept".to_owned()),
        path_before: LAUNCHER_PATH.to_owned(),
        path_after: LAUNCHER_PATH.to_owned(),
        executable: observed.executable.clone(),
        self_executable: Some(observed.executable.clone()),
        tree: initial_tree(),
    };
    assert_eq!(observed, expected);
    Ok(())
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_startup_actual_cli_display_and_refusal_preserve_owned_tree() -> anyhow::Result<()> {
    let mut outcomes = Vec::new();
    for argument in ["--help", "--version", "--covenant-owned-invalid"] {
        let fixture = Fixture::new()?;
        let output = fixture.command("codex")?.arg(argument).output()?;
        let mut tree = BTreeMap::new();
        snapshot(
            &fixture.root.join("home/tmp/arg0"),
            Path::new(""),
            &mut tree,
        )?;
        outcomes.push((display(&output)?, tree == initial_tree()));
    }
    assert_eq!(
        outcomes,
        vec![
            ((Some(0), true, false, true), true),
            ((Some(0), true, false, true), true),
            ((Some(2), false, true, false), true)
        ]
    );
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_startup_ordinary_observer_and_actual_cli_effects_are_active() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut observed = observe(&fixture)?;
    let paths = std::env::split_paths(&observed.path_after).collect::<Vec<_>>();
    assert_eq!(paths.len(), 2);
    let active = paths[0].clone();
    let canonical_parent = active
        .parent()
        .context("missing alias parent")?
        .canonicalize()?;
    assert_eq!(
        canonical_parent,
        fixture.root.join("home/tmp/arg0").canonicalize()?
    );
    let name = active
        .file_name()
        .context("missing alias directory")?
        .to_str()
        .context("non-Unicode alias directory")?;
    assert!(name.starts_with("codex-arg0"));
    let batch_base = canonical_parent.join(name);
    let mut expected_tree = initial_tree();
    expected_tree.retain(|path, _| !path.starts_with("stale/"));
    expected_tree.insert(format!("{name}/"), String::new());
    expected_tree.insert(format!("{name}/.lock"), String::new());
    for alias in ["apply_patch.bat", "applypatch.bat"] {
        let key = format!("{name}/{alias}");
        let content = observed
            .tree
            .get_mut(&key)
            .context("missing generated alias")?;
        let target = content
            .strip_prefix("@echo off\n\"")
            .and_then(|s| s.strip_suffix("\" --codex-run-as-apply-patch %*\n"))
            .context("unexpected complete batch body")?;
        // On this canonical Windows base, PathBuf::join removes relative ..
        // before canonicalize opens the target; the alias guard may have dropped.
        let target = target.strip_prefix("%~dp0").map_or_else(
            || PathBuf::from(target),
            |relative| batch_base.join(relative),
        );
        assert_eq!(target.canonicalize()?, observed.executable.canonicalize()?);
        *content = "@echo off\n\"$SELF\" --codex-run-as-apply-patch %*\n".to_owned();
        expected_tree.insert(
            key,
            "@echo off\n\"$SELF\" --codex-run-as-apply-patch %*\n".to_owned(),
        );
    }
    let expected = Observation {
        canary: Some("dotenv-overwrite".to_owned()),
        path_before: LAUNCHER_PATH.to_owned(),
        path_after: std::env::join_paths([active, PathBuf::from(DOTENV_PATH)])?
            .into_string()
            .unwrap(),
        executable: observed.executable.clone(),
        self_executable: Some(observed.executable.clone()),
        tree: expected_tree,
    };
    assert_eq!(observed, expected);
    let mut outcomes = Vec::new();
    for argument in ["--help", "--version", "--covenant-owned-invalid"] {
        let fixture = Fixture::new()?;
        let output = fixture.command("codex")?.arg(argument).output()?;
        let mut tree = BTreeMap::new();
        snapshot(
            &fixture.root.join("home/tmp/arg0"),
            Path::new(""),
            &mut tree,
        )?;
        let mut known = tree.clone();
        known.retain(|path, _| path.starts_with("locked/") || path.starts_with("stale/"));
        let mut expected = initial_tree();
        expected.retain(|path, _| path.starts_with("locked/"));
        assert_eq!(known, expected);
        outcomes.push((display(&output)?, tree.len() > known.len()));
    }
    assert_eq!(
        outcomes,
        vec![
            ((Some(0), true, false, true), true),
            ((Some(0), true, false, true), true),
            ((Some(2), false, true, false), true)
        ]
    );
    Ok(())
}

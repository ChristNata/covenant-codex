//! Windows Cargo-built CLI direct patch-entry contract.
#![cfg(windows)]

use anyhow::Context;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

#[cfg(feature = "covenant")]
const REFUSAL: &[u8] = b"Covenant patch entrypoint refused\n";

struct Case {
    name: &'static str,
    patch: &'static str,
    #[cfg(not(feature = "covenant"))]
    success_stdout: &'static [u8],
    #[cfg(not(feature = "covenant"))]
    changed_path: &'static str,
    #[cfg(not(feature = "covenant"))]
    changed_contents: &'static [u8],
}

const CASES: [Case; 2] = [
    Case {
        name: "add",
        patch: "*** Begin Patch\n*** Add File: added.txt\n+created by the owned fixture\n*** End Patch",
        #[cfg(not(feature = "covenant"))]
        success_stdout: b"Success. Updated the following files:\nA added.txt\n",
        #[cfg(not(feature = "covenant"))]
        changed_path: "work/added.txt",
        #[cfg(not(feature = "covenant"))]
        changed_contents: b"created by the owned fixture\n",
    },
    Case {
        name: "update",
        patch: "*** Begin Patch\n*** Update File: existing.txt\n@@\n-original content\n+updated by the owned fixture\n*** End Patch",
        #[cfg(not(feature = "covenant"))]
        success_stdout: b"Success. Updated the following files:\nM existing.txt\n",
        #[cfg(not(feature = "covenant"))]
        changed_path: "work/existing.txt",
        #[cfg(not(feature = "covenant"))]
        changed_contents: b"updated by the owned fixture\n",
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Directory,
    File(Vec<u8>),
}

fn initial_tree() -> BTreeMap<String, Entry> {
    let mut tree: BTreeMap<_, _> = [
        "work",
        "home",
        "profile",
        "local",
        "roaming",
        "temp",
        "outside-work",
    ]
    .map(|name| (name.to_owned(), Entry::Directory))
    .into();
    for (path, bytes) in [
        ("work/existing.txt", b"original content\n".as_slice()),
        ("work/sentinel.txt", b"keep the work sentinel\n".as_slice()),
        (
            "outside-work/sentinel.txt",
            b"keep the sibling sentinel\n".as_slice(),
        ),
    ] {
        tree.insert(path.to_owned(), Entry::File(bytes.to_vec()));
    }
    tree
}

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        for (relative, entry) in initial_tree() {
            let path = root.join(relative);
            match entry {
                Entry::Directory => fs::create_dir_all(path)?,
                Entry::File(bytes) => fs::write(path, bytes)?,
            }
        }
        Ok(Self {
            _temporary: temporary,
            root,
        })
    }

    fn run(&self, case: &Case) -> anyhow::Result<Output> {
        let executable = codex_utils_cargo_bin::cargo_bin("codex")?;
        let mut command = assert_cmd::Command::new(executable);
        command
            .env_clear()
            .current_dir(self.root.join("work"))
            .env("CODEX_HOME", self.root.join("home"))
            .env("HOME", self.root.join("profile"))
            .env("USERPROFILE", self.root.join("profile"))
            .env("LOCALAPPDATA", self.root.join("local"))
            .env("APPDATA", self.root.join("roaming"))
            .env("TEMP", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .env("PATH", self.root.join("home"))
            .env(
                "SystemRoot",
                std::env::var_os("SystemRoot").context("Windows SystemRoot required")?,
            )
            .arg("--codex-run-as-apply-patch")
            .arg(case.patch)
            .timeout(Duration::from_secs(/*secs*/ 15));
        // assert_cmd buffers output; these checks are not streaming limits.
        let output = command.output()?;
        anyhow::ensure!(
            output.stdout.len() <= 16_384 && output.stderr.len() <= 16_384,
            "owned patch-entry output exceeded its observation bound"
        );
        Ok(output)
    }
}

fn snapshot(root: &Path) -> anyhow::Result<BTreeMap<String, Entry>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        anyhow::ensure!(relative.components().count() <= 4, "owned tree too deep");
        for entry in fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            let name = path
                .to_str()
                .context("owned tree has a non-Unicode name")?
                .replace('\\', "/");
            anyhow::ensure!(result.len() < 32, "owned tree has too many entries");
            let kind = entry.file_type()?;
            let value = if kind.is_dir() {
                pending.push(path);
                Entry::Directory
            } else {
                anyhow::ensure!(kind.is_file(), "unexpected owned entry kind");
                let mut bytes = Vec::new();
                fs::File::open(entry.path())?
                    .take(/*limit*/ 4097)
                    .read_to_end(&mut bytes)?;
                anyhow::ensure!(bytes.len() <= 4096, "owned file too large");
                Entry::File(bytes)
            };
            anyhow::ensure!(
                result.insert(name, value).is_none(),
                "duplicate owned entry name"
            );
        }
    }
    Ok(result)
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    operation: &'static str,
    native_exit: Option<i32>,
    stdout_matches: bool,
    stderr_matches: bool,
    complete_tree_matches: bool,
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_patch_entry_refuses_valid_add_and_update_before_mutation() -> anyhow::Result<()> {
    let mut outcomes = Vec::new();
    for case in &CASES {
        let fixture = Fixture::new()?;
        let before = snapshot(&fixture.root)?;
        assert_eq!(before, initial_tree());
        let output = fixture.run(case)?;
        let after = snapshot(&fixture.root)?;
        outcomes.push(Outcome {
            operation: case.name,
            native_exit: output.status.code(),
            stdout_matches: output.stdout.is_empty(),
            stderr_matches: output.stderr == REFUSAL,
            complete_tree_matches: after == before,
        });
    }
    let expected: Vec<_> = CASES
        .iter()
        .map(|case| Outcome {
            operation: case.name,
            native_exit: Some(2),
            stdout_matches: true,
            stderr_matches: true,
            complete_tree_matches: true,
        })
        .collect();
    assert_eq!(outcomes, expected);
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_patch_entry_ordinary_performs_both_valid_operations() -> anyhow::Result<()> {
    let mut outcomes = Vec::new();
    for case in &CASES {
        let fixture = Fixture::new()?;
        assert_eq!(snapshot(&fixture.root)?, initial_tree());
        let mut expected_tree = initial_tree();
        expected_tree.insert(
            case.changed_path.to_owned(),
            Entry::File(case.changed_contents.to_vec()),
        );
        let output = fixture.run(case)?;
        let after = snapshot(&fixture.root)?;
        outcomes.push(Outcome {
            operation: case.name,
            native_exit: output.status.code(),
            stdout_matches: output.stdout == case.success_stdout,
            stderr_matches: output.stderr.is_empty(),
            complete_tree_matches: after == expected_tree,
        });
    }
    let expected: Vec<_> = CASES
        .iter()
        .map(|case| Outcome {
            operation: case.name,
            native_exit: Some(0),
            stdout_matches: true,
            stderr_matches: true,
            complete_tree_matches: true,
        })
        .collect();
    assert_eq!(outcomes, expected);
    Ok(())
}

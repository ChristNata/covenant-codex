//! Standalone, explicitly selected test fixture; never a product entrypoint.
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Only reads precede the real owner. No libtest, ctor, thread or runtime.
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("startup observer requires an owned fixture root"))?,
    );
    let executable = std::env::current_exe()?;
    let path_before = std::env::var("PATH")?;
    codex_arg0::arg0_dispatch_or_else(move |paths| async move {
        let mut tree = BTreeMap::new();
        snapshot(&root.join("home/tmp/arg0"), Path::new(""), &mut tree)?;
        let observation = serde_json::json!({
            "canary": std::env::var("COVENANT_STARTUP_CANARY").ok(),
            "path_before": path_before,
            "path_after": std::env::var("PATH")?,
            "executable": executable,
            "self_executable": paths.codex_self_exe,
            "tree": tree,
        });
        let output = serde_json::to_string(&observation)?;
        anyhow::ensure!(output.len() <= 16_384, "startup observation exceeds bound");
        println!("{output}");
        Ok(())
    })
}

fn snapshot(
    root: &Path,
    relative: &Path,
    tree: &mut BTreeMap<String, String>,
) -> anyhow::Result<()> {
    anyhow::ensure!(relative.components().count() <= 4, "fixture tree too deep");
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        let name = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("non-Unicode fixture path"))?
            .replace('\\', "/");
        anyhow::ensure!(tree.len() < 32, "fixture tree too large");
        let kind = entry.file_type()?;
        if kind.is_dir() {
            tree.insert(format!("{name}/"), String::new());
            snapshot(root, &path, tree)?;
        } else {
            anyhow::ensure!(kind.is_file(), "unexpected fixture entry type");
            let file = std::fs::File::open(entry.path())?;
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

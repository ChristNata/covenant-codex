//! Small, dependency-free semantic checks used by the Covenant re-audit job.
//!
//! This is intentionally a source-tree harness rather than a Codex product test. It
//! consumes the F00 inventory, checks that its recorded anchors still exist, and
//! exercises the negative mutation rule used for the golden fixtures.

use std::fs;
use std::path::{Path, PathBuf};

const FAMILIES: &[&str] = &[
    "process",
    "filesystem",
    "network",
    "hosted",
    "mcp",
    "hook",
    "generated_code",
];

fn value(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| {
            let (name, raw) = line.split_once('=')?;
            if name.trim() != key {
                return None;
            }
            Some(toml_string(raw.trim()))
        })
}

fn toml_string(raw: &str) -> String {
    let raw = raw.trim();
    let raw = raw.strip_prefix('"').and_then(|value| value.strip_suffix('"')).unwrap_or(raw);
    raw.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn inventory_anchors(repo: &Path) -> Result<Vec<(String, String)>, String> {
    let inventory = fs::read_to_string(repo.join("covenant/AUTHORITY-INVENTORY.toml"))
        .map_err(|error| format!("read inventory: {error}"))?;
    let mut anchors = Vec::new();
    let mut path = None;
    let mut symbol = None;
    for line in inventory.lines() {
        if let Some((key, raw)) = line.split_once('=') {
            match key.trim() {
                "path" => path = Some(toml_string(raw)),
                "symbol" => symbol = Some(toml_string(raw)),
                _ => {}
            }
        }
        if path.is_some() && symbol.is_some() {
            anchors.push((path.take().unwrap(), symbol.take().unwrap()));
        }
    }
    if anchors.is_empty() {
        return Err("inventory has no source anchors".to_owned());
    }
    Ok(anchors)
}

fn source_check(repo: &Path) -> Result<(), String> {
    let upstream = repo.join("covenant/UPSTREAM.toml");
    let integration = repo.join("covenant/INTEGRATION-FILES.toml");
    let inventory = repo.join("covenant/AUTHORITY-INVENTORY.toml");
    let commit = value(&upstream, "commit").ok_or("UPSTREAM.toml has no commit")?;
    for manifest in [&integration, &inventory] {
        let actual = value(manifest, "commit").ok_or("manifest has no commit")?;
        if actual != commit {
            return Err(format!("commit binding mismatch: {actual} != {commit}"));
        }
    }
    let inventory_text = fs::read_to_string(&inventory)
        .map_err(|error| format!("read inventory: {error}"))?;
    for family in FAMILIES {
        let marker = format!("family = \"{family}\"");
        if !inventory_text.lines().any(|line| line.trim() == marker) {
            return Err(format!("inventory omits sink family: {family}"));
        }
    }
    for (relative, symbol) in inventory_anchors(repo)? {
        let source = fs::read_to_string(repo.join(relative))
            .map_err(|error| format!("read inventory source: {error}"))?;
        if !source.contains(&symbol) {
            return Err(format!("inventory anchor disappeared: {symbol}"));
        }
    }
    Ok(())
}

fn ungated_effects(source: &str) -> Vec<&'static str> {
    let mut effects = Vec::new();
    let mut gated = false;
    for line in source.lines() {
        if line.contains("COVENANT_GATE") || line.contains("covenant_gate") {
            gated = true;
        }
        let family = if line.contains("Command::new") || line.contains("std::process") {
            Some("process")
        } else if line.contains("fs::write") || line.contains("File::create") {
            Some("filesystem")
        } else {
            None
        };
        if let Some(family) = family {
            if !gated {
                effects.push(family);
            }
            gated = false;
        }
        if !line.trim_start().starts_with("//") && !line.trim().is_empty() {
            gated = false;
        }
    }
    effects
}

fn run(repo: &Path) -> Result<(), String> {
    source_check(repo)?;
    let fixture = "// COVENANT_GATE\nlet _ = Command::new(\"allowed\");\nlet _ = Command::new(\"ungated\");\n";
    if ungated_effects(fixture) != ["process"] {
        return Err("negative process mutation was not discriminated".to_owned());
    }
    Ok(())
}

fn main() {
    let repo = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    if let Err(error) = run(&repo) {
        eprintln!("Covenant re-audit failed: {error}");
        std::process::exit(1);
    }
    println!("Covenant re-audit source and mutation checks passed");
}

#[cfg(test)]
mod tests {
    use super::ungated_effects;

    #[test]
    fn allowed_effect_is_followed_by_a_failing_ungated_mutation() {
        let fixture = "// COVENANT_GATE\nCommand::new(\"allowed\");\nCommand::new(\"mutation\");";
        assert_eq!(ungated_effects(fixture), ["process"]);
    }

    #[test]
    fn ordinary_comments_do_not_authorize_an_effect() {
        assert_eq!(ungated_effects("// explanatory\nCommand::new(\"mutation\");"), ["process"]);
    }
}

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "covenant")]
use chrono::Utc;
#[cfg(feature = "covenant")]
use codex_models_manager::cache::ModelsCacheEntry;
use codex_protocol::openai_models::ModelsResponse;
#[cfg(feature = "covenant")]
use pretty_assertions::assert_eq;
use tempfile::TempDir;

#[cfg(feature = "covenant")]
const CACHED_REMOTE_MODEL: &str = "live-catalog-test";

fn codex_command(codex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    cmd.env("CODEX_HOME", codex_home);
    Ok(cmd)
}

fn catalog_from_stdout(stdout: &[u8]) -> Result<ModelsResponse> {
    Ok(serde_json::from_slice(stdout)?)
}

#[cfg(feature = "covenant")]
fn write_remote_models_cache(codex_home: &Path) -> Result<()> {
    let mut remote_model = codex_models_manager::covenant_model_catalog()?
        .models
        .into_iter()
        .next()
        .context("Covenant catalog has no models")?;
    remote_model.slug = CACHED_REMOTE_MODEL.to_owned();
    remote_model.display_name = "Live Catalog Test".to_owned();
    let cache = ModelsCacheEntry {
        fetched_at: Utc::now(),
        etag: Some("live-catalog-test-etag".to_owned()),
        client_version: Some(codex_models_manager::client_version_to_whole()),
        models: vec![remote_model],
    };
    std::fs::write(
        codex_home.join("models_cache.json"),
        serde_json::to_vec(&cache)?,
    )?;
    Ok(())
}

#[test]
fn debug_models_bundled_prints_json() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd.args(["debug", "models", "--bundled"]).output()?;

    assert!(output.status.success());
    let actual = catalog_from_stdout(&output.stdout)?;
    #[cfg(feature = "covenant")]
    assert_eq!(actual, codex_models_manager::covenant_model_catalog()?);
    #[cfg(not(feature = "covenant"))]
    assert!(!actual.models.is_empty());

    Ok(())
}

#[test]
fn debug_models_default_prints_json_without_auth() -> Result<()> {
    let codex_home = TempDir::new()?;
    #[cfg(feature = "covenant")]
    std::fs::write(
        codex_home.path().join("config.toml"),
        "model = \"not-a-covenant-model\"\n",
    )?;
    #[cfg(feature = "covenant")]
    write_remote_models_cache(codex_home.path())?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd.args(["debug", "models"]).output()?;

    assert!(output.status.success());
    let actual = catalog_from_stdout(&output.stdout)?;
    assert!(!actual.models.is_empty());
    #[cfg(feature = "covenant")]
    assert!(
        actual
            .models
            .iter()
            .any(|model| model.slug == CACHED_REMOTE_MODEL)
    );

    Ok(())
}

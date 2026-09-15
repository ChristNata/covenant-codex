//! Binds a headless Covenant run to one current, authenticated OpenAI model catalog.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_core::build_models_manager;
use codex_core::config::Config;
use codex_login::AuthManager;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;

pub(crate) async fn validate(mut config: Config) -> Result<Config> {
    let auth_manager =
        AuthManager::shared_from_config(&config, /*enable_codex_api_key_env*/ true).await?;
    let models_manager = build_models_manager(&config, auth_manager);
    let catalog = models_manager
        .fresh_model_catalog(config.http_client_factory())
        .await
        .context("Covenant live OpenAI model catalog unavailable")?;

    let selected = match config.model.as_deref() {
        Some(model) => catalog
            .models
            .iter()
            .find(|candidate| candidate.slug == model && admissible(candidate))
            .map(|candidate| candidate.slug.clone()),
        None => catalog
            .models
            .iter()
            .filter(|candidate| admissible(candidate))
            .min_by_key(|candidate| candidate.priority)
            .map(|candidate| candidate.slug.clone()),
    };
    let Some(selected) = selected else {
        bail!("Covenant live OpenAI model selection refused");
    };

    config.model = Some(selected);
    // The internal app-server must use exactly the metadata snapshot that admitted the model.
    config.model_catalog = Some(catalog);
    Ok(config)
}

fn admissible(candidate: &ModelInfo) -> bool {
    candidate.visibility == ModelVisibility::List
        && candidate.shell_type == ConfigShellToolType::UnifiedExec
        && candidate.apply_patch_tool_type.is_some()
}

#[cfg(test)]
#[path = "covenant_models_tests.rs"]
mod tests;

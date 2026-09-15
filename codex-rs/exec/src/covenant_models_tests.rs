use super::*;
use codex_core::config::ConfigBuilder;
use codex_core::config::LoaderOverrides;
use codex_login::AuthCredentialsStoreMode;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

async fn config_with_synthetic_chatgpt_auth(
    home: &TempDir,
    server: &MockServer,
    model: Option<&str>,
) -> Result<Config> {
    std::fs::write(
        home.path().join("auth.json"),
        serde_json::to_vec(&json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "id_token": "eyJhbGciOiJub25lIn0.e30.c2ln",
                "access_token": "synthetic-model-catalog-access-token",
                "refresh_token": "synthetic-model-catalog-refresh-token",
                "account_id": "synthetic-model-catalog-account",
            },
            "last_refresh": "2099-01-01T00:00:00Z",
        }))?,
    )?;
    let mut config = ConfigBuilder::default()
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .codex_home(home.path().to_path_buf())
        .fallback_cwd(Some(home.path().to_path_buf()))
        .build()
        .await?;
    config.cli_auth_credentials_store_mode = AuthCredentialsStoreMode::File;
    config.model_provider.base_url = Some(format!("{}/backend-api/codex", server.uri()));
    config.model = model.map(str::to_owned);
    Ok(config)
}

fn live_catalog() -> Result<ModelsResponse> {
    let model: ModelInfo = serde_json::from_value(json!({
        "slug": "gpt-6-astra",
        "display_name": "GPT-6 Astra",
        "description": "Synthetic live catalog model",
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [{"effort": "medium", "description": "Balanced"}],
        "shell_type": "unified_exec",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 1,
        "availability_nux": null,
        "upgrade": null,
        "model_messages": {
            "instructions_template": "Synthetic Astra instructions",
            "instructions_variables": null,
        },
        "support_verbosity": true,
        "default_verbosity": "low",
        "apply_patch_tool_type": "freeform",
        "truncation_policy": {"mode": "tokens", "limit": 10000},
        "experimental_supported_tools": [],
    }))?;
    let mut hidden = model.clone();
    hidden.slug = "gpt-6-hidden".to_owned();
    hidden.visibility = ModelVisibility::Hide;
    hidden.priority = 0;
    Ok(ModelsResponse {
        models: vec![model, hidden],
    })
}

#[tokio::test]
async fn exec_selects_visible_new_model_and_binds_exact_live_metadata() -> Result<()> {
    let home = TempDir::new()?;
    let server = MockServer::start().await;
    let catalog = live_catalog()?;
    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&catalog))
        .expect(1)
        .mount(&server)
        .await;

    let config = config_with_synthetic_chatgpt_auth(&home, &server, Some("gpt-6-astra")).await?;
    let admitted = validate(config).await?;
    assert_eq!(
        (admitted.model, admitted.model_catalog),
        (Some("gpt-6-astra".to_owned()), Some(catalog))
    );
    Ok(())
}

#[tokio::test]
async fn exec_default_uses_visible_remote_priority_not_hidden_or_bundled() -> Result<()> {
    let home = TempDir::new()?;
    let server = MockServer::start().await;
    let catalog = live_catalog()?;
    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&catalog))
        .expect(1)
        .mount(&server)
        .await;

    let config = config_with_synthetic_chatgpt_auth(&home, &server, None).await?;
    let admitted = validate(config).await?;
    assert_eq!(admitted.model, Some("gpt-6-astra".to_owned()));
    assert_eq!(admitted.model_catalog, Some(catalog));
    Ok(())
}

#[tokio::test]
async fn exec_refuses_hidden_or_absent_identity_before_inference() -> Result<()> {
    let home = TempDir::new()?;
    let server = MockServer::start().await;
    let catalog = live_catalog()?;
    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&catalog))
        .expect(2)
        .mount(&server)
        .await;

    for model in ["gpt-6-hidden", "gpt-5.5"] {
        let config = config_with_synthetic_chatgpt_auth(&home, &server, Some(model)).await?;
        let error = validate(config).await.expect_err("model must be refused");
        assert_eq!(
            error.to_string(),
            "Covenant live OpenAI model selection refused"
        );
    }
    Ok(())
}

#[tokio::test]
async fn exec_refuses_list_visible_model_without_the_two_native_tool_forms() -> Result<()> {
    let home = TempDir::new()?;
    let server = MockServer::start().await;
    let mut catalog = live_catalog()?;
    let mut incompatible = catalog.models[0].clone();
    incompatible.slug = "gpt-6-incompatible".to_owned();
    incompatible.shell_type = ConfigShellToolType::Disabled;
    incompatible.apply_patch_tool_type = None;
    catalog.models.push(incompatible);
    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&catalog))
        .expect(1)
        .mount(&server)
        .await;

    let config =
        config_with_synthetic_chatgpt_auth(&home, &server, Some("gpt-6-incompatible")).await?;
    let error = validate(config)
        .await
        .expect_err("unsupported tool forms must refuse");
    assert_eq!(
        error.to_string(),
        "Covenant live OpenAI model selection refused"
    );
    Ok(())
}

#[tokio::test]
async fn exec_does_not_fall_back_to_bundled_catalog_on_remote_error() -> Result<()> {
    let home = TempDir::new()?;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&server)
        .await;

    let config = config_with_synthetic_chatgpt_auth(&home, &server, Some("gpt-5.5")).await?;
    let error = validate(config)
        .await
        .expect_err("remote failure must refuse");
    assert_eq!(
        error.to_string(),
        "Covenant live OpenAI model catalog unavailable"
    );
    Ok(())
}

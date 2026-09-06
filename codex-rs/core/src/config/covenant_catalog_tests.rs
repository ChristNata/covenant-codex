use super::*;
#[cfg(feature = "covenant")]
use codex_login::AuthManager;
#[cfg(feature = "covenant")]
use codex_login::CodexAuth;
#[cfg(feature = "covenant")]
use codex_models_manager::manager::RefreshStrategy;
use pretty_assertions::assert_eq;
use std::fs;

const SECURITY: &str = "allowed_approval_policies = ['on-request']\nallowed_sandbox_modes = ['read-only']\nallowed_login_methods = ['api']\ncli_auth_credentials_store = 'file'\nallow_login_shell = false\n";
#[cfg(feature = "covenant")]
const SELECTION_REFUSAL: &str = "Covenant model selection refused";
#[cfg(feature = "covenant")]
const CATALOG_REFUSAL: &str = "Covenant model catalog refused";

struct Fixture {
    _temporary: tempfile::TempDir,
    root: AbsolutePathBuf,
    home: AbsolutePathBuf,
    project: AbsolutePathBuf,
    selected: AbsolutePathBuf,
    loader: LoaderOverrides,
}

impl Fixture {
    fn new() -> std::io::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = AbsolutePathBuf::try_from(temporary.path().canonicalize()?)?;
        let home = root.join("home");
        let project = root.join("project");
        fs::create_dir_all(&home)?;
        fs::create_dir_all(project.join(".codex"))?;
        fs::write(project.join(".fixture-root"), "")?;
        fs::write(
            project.join(".codex/config.toml"),
            "project_doc_max_bytes = 777",
        )?;
        for name in ["packaged.toml", "managed.toml", "system.toml"] {
            fs::write(root.join(name), "")?;
        }
        fs::write(root.join("requirements.toml"), SECURITY)?;
        let base = ConfigToml {
            sqlite_home: Some(root.join("sqlite")),
            approval_policy: Some(AskForApproval::OnRequest),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            allow_login_shell: Some(false),
            forced_login_method: Some(ForcedLoginMethod::Api),
            cli_auth_credentials_store: Some(AuthCredentialsStoreMode::File),
            project_root_markers: Some(vec![".fixture-root".to_owned()]),
            projects: Some(HashMap::from([(
                project.to_string_lossy().to_string(),
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            ..Default::default()
        };
        // No initial layer supplies a model: the default test must exercise absence.
        fs::write(home.join("config.toml"), toml::to_string(&base).unwrap())?;
        let selected = home.join("worker.config.toml");
        fs::write(&selected, "model_reasoning_effort = 'low'")?;
        let loader = LoaderOverrides {
            packaged_defaults_path: Some(root.join("packaged.toml")),
            managed_config_path: Some(root.join("managed.toml").to_path_buf()),
            system_config_path: Some(root.join("system.toml").to_path_buf()),
            system_requirements_path: Some(root.join("requirements.toml").to_path_buf()),
            user_config_path: Some(selected.clone()),
            user_config_profile: Some("worker".parse().unwrap()),
            macos_managed_config_requirements_base64: Some(String::new()),
            #[cfg(target_os = "macos")]
            managed_preferences_base64: Some(String::new()),
            ..Default::default()
        };
        Ok(Self {
            _temporary: temporary,
            root,
            home,
            project,
            selected,
            loader,
        })
    }

    fn builder(&self) -> ConfigBuilder {
        ConfigBuilder::default()
            .codex_home(self.home.to_path_buf())
            .loader_overrides(self.loader.clone())
            .harness_overrides(ConfigOverrides {
                cwd: Some(self.project.to_path_buf()),
                ..Default::default()
            })
    }

    fn profile(&self, value: &str) -> std::io::Result<()> {
        fs::write(&self.selected, value)
    }

    #[cfg(feature = "covenant")]
    async fn layers(&self, overrides: &[(String, TomlValue)]) -> std::io::Result<ConfigLayerStack> {
        load_config_layers_state(
            LOCAL_FS.as_ref(),
            &self.home,
            Some(self.project.clone()),
            overrides,
            ConfigLoadOptions {
                loader_overrides: self.loader.clone(),
                ..Default::default()
            },
            &codex_config::NoopThreadConfigLoader,
        )
        .await
    }

    fn observe(&self, config: &Config, expected_catalog: Option<&ModelsResponse>) -> Observation {
        let layers = &config.config_layer_stack;
        assert_eq!(
            (
                layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::User { file, profile } if file == &self.selected && profile.as_deref() == Some("worker"))),
                layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::Project { dot_codex_folder } if dot_codex_folder == &self.project.join(".codex"))),
                config.project_doc_max_bytes, &config.cwd, &config.codex_home,
            ),
            (true, true, 777, &self.project, &self.home)
        );
        security(config);
        Observation {
            model: config.model.clone(),
            raw_model: layers
                .effective_config()
                .get("model")
                .and_then(TomlValue::as_str)
                .map(str::to_owned),
            effort: config.model_reasoning_effort.clone(),
            // Evaluate complete typed equality without printing a 156KiB catalog on failure.
            complete_catalog_matches: config.model_catalog.as_ref() == expected_catalog,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Observation {
    model: Option<String>,
    raw_model: Option<String>,
    effort: Option<ReasoningEffort>,
    complete_catalog_matches: bool,
}

fn expected_catalog() -> ModelsResponse {
    let upstream = codex_models_manager::bundled_models_response().unwrap();
    ModelsResponse {
        models: ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.2"]
            .into_iter()
            .map(|id| {
                upstream
                    .models
                    .iter()
                    .find(|model| model.slug == id)
                    .unwrap()
                    .clone()
            })
            .collect(),
    }
}

fn custom_catalog(name: &str) -> ModelsResponse {
    let mut catalog = expected_catalog();
    catalog.models.truncate(1);
    catalog.models[0].slug = name.to_owned();
    catalog.models[0].display_name = format!("Owned {name}");
    catalog
}

fn security(
    config: &Config,
) -> (
    Permissions,
    AuthCredentialsStoreMode,
    Option<ForcedLoginMethod>,
    ManagedAuthPolicy,
) {
    assert_eq!(
        (
            config.permissions.approval_policy.value(),
            config.permissions.permission_profile().clone(),
            config.permissions.allow_login_shell,
            config.cli_auth_credentials_store_mode,
            config.forced_login_method,
            config.managed_auth_policy().allowed_login_methods,
        ),
        (
            AskForApproval::OnRequest,
            PermissionProfile::read_only(),
            false,
            AuthCredentialsStoreMode::File,
            Some(ForcedLoginMethod::Api),
            Some(vec![ForcedLoginMethod::Api])
        )
    );
    (
        config.permissions.clone(),
        config.cli_auth_credentials_store_mode,
        config.forced_login_method,
        config.managed_auth_policy(),
    )
}

#[cfg(feature = "covenant")]
fn exact_refusal(result: std::io::Result<Config>, expected: &str) -> bool {
    result.is_err_and(|error| {
        error.kind() == ErrorKind::InvalidData
            && error.to_string() == expected
            && format!("{error:?}").len() < 160
    })
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_catalog_config_selection_reload_and_manager_preserve_catalog()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let expected = expected_catalog();
    let default = fixture.builder().build().await?;
    fixture.profile("model = 'gpt-5.4'\nmodel_reasoning_effort = 'low'")?;
    let profile = fixture.builder().build().await?;
    let cli_override = vec![(
        "model".to_owned(),
        TomlValue::String("gpt-5.4-mini".to_owned()),
    )];
    let cli = fixture
        .builder()
        .cli_overrides(cli_override.clone())
        .build()
        .await?;
    let harness = fixture
        .builder()
        .cli_overrides(cli_override)
        .harness_overrides(ConfigOverrides {
            cwd: Some(fixture.project.to_path_buf()),
            model: Some("gpt-5.5".to_owned()),
            ..Default::default()
        })
        .build()
        .await?;
    fixture.profile("model = 'gpt-5.2'\nmodel_reasoning_effort = 'high'")?;
    let refreshed = fixture.builder().build().await?;
    let rebuilt = cli.rebuild_preserving_session_layers(&refreshed).await?;
    let configs = [&default, &profile, &cli, &harness, &refreshed, &rebuilt];
    for config in configs {
        assert_eq!(security(config), security(&default));
    }
    let observed = configs.map(|config| fixture.observe(config, Some(&expected)));

    let auth = AuthManager::from_auth_for_testing_with_home(
        CodexAuth::from_api_key("fixture-not-a-real-key"),
        fixture.home.to_path_buf(),
    );
    let manager = crate::thread_manager::build_models_manager(&rebuilt, auth);
    let mut raw_matches = vec![
        manager
            .raw_model_catalog(RefreshStrategy::Offline, rebuilt.http_client_factory())
            .await
            == expected,
    ];
    for strategy in [
        RefreshStrategy::Offline,
        RefreshStrategy::OnlineIfUncached,
        RefreshStrategy::Online,
    ] {
        manager
            .refresh_if_new_etag("owned-etag".to_owned(), rebuilt.http_client_factory())
            .await;
        raw_matches.push(
            manager
                .raw_model_catalog(strategy, rebuilt.http_client_factory())
                .await
                == expected,
        );
    }
    let expected_rows = [
        (Some("gpt-5.5"), None, ReasoningEffort::Low),
        (Some("gpt-5.4"), Some("gpt-5.4"), ReasoningEffort::Low),
        (
            Some("gpt-5.4-mini"),
            Some("gpt-5.4-mini"),
            ReasoningEffort::Low,
        ),
        (Some("gpt-5.5"), Some("gpt-5.4-mini"), ReasoningEffort::Low),
        (Some("gpt-5.2"), Some("gpt-5.2"), ReasoningEffort::High),
        (
            Some("gpt-5.4-mini"),
            Some("gpt-5.4-mini"),
            ReasoningEffort::High,
        ),
    ]
    .map(|(model, raw_model, effort)| Observation {
        model: model.map(str::to_owned),
        raw_model: raw_model.map(str::to_owned),
        effort: Some(effort),
        complete_catalog_matches: true,
    });
    assert_eq!((observed, raw_matches), (expected_rows, vec![true; 4]));
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_catalog_config_refuses_unknown_effective_selections() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let accepted = fixture.builder().build().await?;
    let before = fixture.observe(&accepted, Some(&expected_catalog()));
    let mut refused = Vec::new();
    for model in ["owned-unknown", "openai/gpt-5.5", "GPT-5.5", "gpt-5.5 "] {
        fixture.profile(&format!("model = {}", TomlValue::String(model.to_owned())))?;
        assert_eq!(
            fixture.layers(&[]).await?.effective_config()["model"].as_str(),
            Some(model)
        );
        refused.push(exact_refusal(
            fixture.builder().build().await,
            SELECTION_REFUSAL,
        ));
        fixture.profile("model = 'gpt-5.4'")?;
        refused.push(exact_refusal(
            fixture
                .builder()
                .cli_overrides(vec![(
                    "model".to_owned(),
                    TomlValue::String(model.to_owned()),
                )])
                .build()
                .await,
            SELECTION_REFUSAL,
        ));
        refused.push(exact_refusal(
            fixture
                .builder()
                .harness_overrides(ConfigOverrides {
                    cwd: Some(fixture.project.to_path_buf()),
                    model: Some(model.to_owned()),
                    ..Default::default()
                })
                .build()
                .await,
            SELECTION_REFUSAL,
        ));
    }
    fixture.profile("model = 'owned-shadowed-unknown'\nmodel_reasoning_effort = 'low'")?;
    let cli = fixture
        .builder()
        .cli_overrides(vec![(
            "model".to_owned(),
            TomlValue::String("gpt-5.4-mini".to_owned()),
        )])
        .build()
        .await?;
    let harness = fixture
        .builder()
        .harness_overrides(ConfigOverrides {
            cwd: Some(fixture.project.to_path_buf()),
            model: Some("gpt-5.2".to_owned()),
            ..Default::default()
        })
        .build()
        .await?;
    let expected = expected_catalog();
    let controls = [
        fixture.observe(&cli, Some(&expected)),
        fixture.observe(&harness, Some(&expected)),
    ];
    let expected_controls = [
        ("gpt-5.4-mini", "gpt-5.4-mini"),
        ("gpt-5.2", "owned-shadowed-unknown"),
    ]
    .map(|(model, raw_model)| Observation {
        model: Some(model.to_owned()),
        raw_model: Some(raw_model.to_owned()),
        effort: Some(ReasoningEffort::Low),
        complete_catalog_matches: true,
    });
    assert_eq!(
        [security(&cli), security(&harness)],
        [security(&accepted), security(&accepted)]
    );
    assert_eq!(
        fixture.observe(&accepted, Some(&expected_catalog())),
        before
    );
    assert_eq!((refused, controls), (vec![true; 12], expected_controls));
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_catalog_config_refuses_caller_paths_independent_of_contents()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let valid = fixture.root.join("CATALOG-PRIVATE-valid.json");
    let malformed = fixture.root.join("CATALOG-PRIVATE-malformed.json");
    let missing = fixture.root.join("CATALOG-PRIVATE-missing.json");
    let bytes = serde_json::to_vec(&custom_catalog("owned-custom-model")).unwrap();
    fs::write(&valid, &bytes)?;
    fs::write(&malformed, b"CATALOG-PRIVATE not JSON")?;
    let mut refused = Vec::new();
    for path in [&valid, &malformed, &missing] {
        let value = TomlValue::String(path.to_string_lossy().to_string());
        fixture.profile(&format!("model = 'gpt-5.5'\nmodel_catalog_json = {value}"))?;
        assert_eq!(
            fixture.layers(&[]).await?.effective_config()["model_catalog_json"],
            value
        );
        refused.push(exact_refusal(
            fixture.builder().build().await,
            CATALOG_REFUSAL,
        ));
        fixture.profile("model = 'gpt-5.5'")?;
        let overrides = vec![("model_catalog_json".to_owned(), value.clone())];
        assert_eq!(
            fixture.layers(&overrides).await?.effective_config()["model_catalog_json"],
            value
        );
        refused.push(exact_refusal(
            fixture.builder().cli_overrides(overrides).build().await,
            CATALOG_REFUSAL,
        ));
    }
    assert_eq!(
        (fs::read(&valid)?, fs::read(&malformed)?, missing.exists()),
        (bytes, b"CATALOG-PRIVATE not JSON".to_vec(), false)
    );
    let control = fixture.builder().build().await?;
    let observed_control = fixture.observe(&control, Some(&expected_catalog()));
    let expected_control = Observation {
        model: Some("gpt-5.5".to_owned()),
        raw_model: Some("gpt-5.5".to_owned()),
        effort: None,
        complete_catalog_matches: true,
    };
    // File preservation is not a read-attempt counter; source review verifies pre-reader placement.
    assert_eq!(
        (refused, observed_control),
        (vec![true; 6], expected_control)
    );
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_catalog_config_ordinary_preserves_selection_and_file_reload()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let initial = fixture.builder().build().await?;
    let path = fixture.root.join("ordinary-catalog.json");
    let first = custom_catalog("owned-first-model");
    fs::write(&path, serde_json::to_vec(&first).unwrap())?;
    let catalog_path = TomlValue::String(path.to_string_lossy().to_string());
    fixture.profile(&format!("model = 'owned-first-model'\nmodel_reasoning_effort = 'low'\nmodel_catalog_json = {catalog_path}"))?;
    let selected = fixture.builder().build().await?;
    let second = custom_catalog("owned-second-model");
    fs::write(&path, serde_json::to_vec(&second).unwrap())?;
    fixture.profile(&format!("model = 'owned-second-model'\nmodel_reasoning_effort = 'high'\nmodel_catalog_json = {catalog_path}"))?;
    let refreshed = fixture.builder().build().await?;
    let observed = [
        fixture.observe(&initial, /*expected_catalog*/ None),
        fixture.observe(&selected, Some(&first)),
        fixture.observe(&refreshed, Some(&second)),
    ];
    assert_eq!(
        [security(&selected), security(&refreshed)],
        [security(&initial), security(&initial)]
    );
    let expected = [
        (None, ReasoningEffort::Low),
        (Some("owned-first-model"), ReasoningEffort::Low),
        (Some("owned-second-model"), ReasoningEffort::High),
    ]
    .map(|(model, effort)| Observation {
        model: model.map(str::to_owned),
        raw_model: model.map(str::to_owned),
        effort: Some(effort),
        complete_catalog_matches: true,
    });
    assert_eq!(observed, expected);
    Ok(())
}

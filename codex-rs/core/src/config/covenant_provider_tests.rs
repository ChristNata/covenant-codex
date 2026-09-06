use super::*;
use codex_model_provider_info::WireApi;
use pretty_assertions::assert_eq;
#[cfg(not(feature = "covenant"))]
use pretty_assertions::assert_ne;
use std::fs;

const SECURITY: &str = "allowed_approval_policies = ['on-request']\nallowed_sandbox_modes = ['read-only']\nallowed_login_methods = ['api']\ncli_auth_credentials_store = 'file'\nallow_login_shell = false\n";
const DECLARATIONS: &str = "[model_providers.owned]\nname = 'Owned provider'\nbase_url = 'https://owned.invalid/v1'\n[model_providers.inactive]\nname = 'Ignored shadow'\nbase_url = 'https://ignored.invalid/v1'\nenv_key = 'COVENANT_UNUSED_PROVIDER_CANARY'\n";
#[cfg(feature = "covenant")]
const REFUSAL: &str = "Covenant model provider refused";

struct Fixture {
    _temporary: tempfile::TempDir,
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
        // No initial layer supplies a selected provider.
        fs::write(home.join("config.toml"), toml::to_string(&base).unwrap())?;
        let selected = home.join("worker.config.toml");
        fs::write(&selected, DECLARATIONS)?;
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
        fs::write(&self.selected, format!("{value}\n{DECLARATIONS}"))
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

    fn observe(&self, config: &Config, expected: &ModelProviderInfo) -> Observation {
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
        let raw = layers.effective_config();
        Observation {
            selected: config.model_provider_id.clone(),
            raw_selected: raw
                .get("model_provider")
                .and_then(TomlValue::as_str)
                .map(str::to_owned),
            raw_base_url: raw
                .get("openai_base_url")
                .and_then(TomlValue::as_str)
                .map(str::to_owned),
            // Complete equality is evaluated before retaining the bounded outcome.
            complete_provider_matches: &config.model_provider == expected,
            inert_declarations_retained: raw.get("model_providers").is_some_and(|providers| {
                providers.get("owned").and_then(|p| p.get("name"))
                    == Some(&TomlValue::String("Owned provider".to_owned()))
                    && providers.get("inactive").and_then(|p| p.get("env_key"))
                        == Some(&TomlValue::String(
                            "COVENANT_UNUSED_PROVIDER_CANARY".to_owned(),
                        ))
            }),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Observation {
    selected: String,
    raw_selected: Option<String>,
    raw_base_url: Option<String>,
    complete_provider_matches: bool,
    inert_declarations_retained: bool,
}

fn expected_row(
    selected: &str,
    raw_selected: Option<&str>,
    raw_base_url: Option<&str>,
) -> Observation {
    Observation {
        selected: selected.to_owned(),
        raw_selected: raw_selected.map(str::to_owned),
        raw_base_url: raw_base_url.map(str::to_owned),
        complete_provider_matches: true,
        inert_declarations_retained: true,
    }
}

fn canonical_provider() -> ModelProviderInfo {
    // Independent complete expectation, not the future validator's output.
    ModelProviderInfo {
        name: "OpenAI".to_owned(),
        base_url: None,
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: Some(HashMap::from([(
            "version".to_owned(),
            env!("CARGO_PKG_VERSION").into(),
        )])),
        env_http_headers: Some(HashMap::from([
            (
                "OpenAI-Organization".to_owned(),
                "OPENAI_ORGANIZATION".to_owned(),
            ),
            ("OpenAI-Project".to_owned(), "OPENAI_PROJECT".to_owned()),
        ])),
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: true,
        supports_websockets: true,
        supports_standalone_web_search: true,
    }
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
fn refused(result: std::io::Result<Config>) -> bool {
    result.is_err_and(|error| {
        error.kind() == ErrorKind::InvalidData
            && error.to_string() == REFUSAL
            && format!("{error:?}").len() < 160
    })
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_provider_config_refuses_effective_selection_and_preserves_precedence()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let canonical = canonical_provider();
    let initial = fixture.builder().build().await?;
    let before = fixture.observe(&initial, &canonical);
    let mut refusals = Vec::new();
    for selected in ["owned", "ollama", "amazon-bedrock", "OPENAI", ""] {
        let value = TomlValue::String(selected.to_owned());
        fixture.profile(&format!("model_provider = {value}"))?;
        assert_eq!(
            fixture.layers(&[]).await?.effective_config()["model_provider"],
            value
        );
        refusals.push(refused(fixture.builder().build().await));
        fixture.profile("model_provider = 'openai'")?;
        let overrides = vec![("model_provider".to_owned(), value.clone())];
        assert_eq!(
            fixture.layers(&overrides).await?.effective_config()["model_provider"],
            value
        );
        refusals.push(refused(
            fixture.builder().cli_overrides(overrides).build().await,
        ));
        refusals.push(refused(
            fixture
                .builder()
                .harness_overrides(ConfigOverrides {
                    cwd: Some(fixture.project.to_path_buf()),
                    model_provider: Some(selected.to_owned()),
                    ..Default::default()
                })
                .build()
                .await,
        ));
    }
    fixture.profile("model_provider = 'owned'")?;
    let cli = fixture
        .builder()
        .cli_overrides(vec![(
            "model_provider".to_owned(),
            TomlValue::String("openai".to_owned()),
        )])
        .build()
        .await?;
    let harness = fixture
        .builder()
        .harness_overrides(ConfigOverrides {
            cwd: Some(fixture.project.to_path_buf()),
            model_provider: Some("openai".to_owned()),
            ..Default::default()
        })
        .build()
        .await?;
    fixture.profile("model_provider = 'openai'")?;
    let refreshed = fixture.builder().build().await?;
    let rebuilt = cli.rebuild_preserving_session_layers(&refreshed).await?;
    let configs = [&initial, &cli, &harness, &refreshed, &rebuilt];
    for config in configs {
        assert_eq!(security(config), security(&initial));
    }
    let observations = configs.map(|config| fixture.observe(config, &canonical));
    assert_eq!(fixture.observe(&initial, &canonical), before);
    assert_eq!(
        (refusals, observations),
        (
            vec![true; 15],
            [
                expected_row(
                    "openai", /*raw_selected*/ None, /*raw_base_url*/ None
                ),
                expected_row("openai", Some("openai"), /*raw_base_url*/ None),
                expected_row("openai", Some("owned"), /*raw_base_url*/ None),
                expected_row("openai", Some("openai"), /*raw_base_url*/ None),
                expected_row("openai", Some("openai"), /*raw_base_url*/ None),
            ]
        )
    );
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_provider_config_refuses_effective_base_url_and_allows_empty_override()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let canonical = canonical_provider();
    let initial = fixture.builder().build().await?;
    let mut refusals = Vec::new();
    for url in [
        "https://owned.invalid/v1",
        "http://127.0.0.1:9/v1",
        "https://api.openai.com/v1",
        " ",
    ] {
        let value = TomlValue::String(url.to_owned());
        fixture.profile(&format!(
            "model_provider = 'openai'\nopenai_base_url = {value}"
        ))?;
        assert_eq!(
            fixture.layers(&[]).await?.effective_config()["openai_base_url"],
            value
        );
        refusals.push(refused(fixture.builder().build().await));
        fixture.profile("model_provider = 'openai'")?;
        let overrides = vec![("openai_base_url".to_owned(), value.clone())];
        assert_eq!(
            fixture.layers(&overrides).await?.effective_config()["openai_base_url"],
            value
        );
        refusals.push(refused(
            fixture.builder().cli_overrides(overrides).build().await,
        ));
    }
    fixture
        .profile("model_provider = 'openai'\nopenai_base_url = 'https://shadowed.invalid/v1'")?;
    let shadowed = fixture
        .builder()
        .cli_overrides(vec![(
            "openai_base_url".to_owned(),
            TomlValue::String(String::new()),
        )])
        .build()
        .await?;
    fixture.profile("model_provider = 'openai'\nopenai_base_url = ''")?;
    let empty = fixture.builder().build().await?;
    let observations = [&shadowed, &empty].map(|config| {
        assert_eq!(security(config), security(&initial));
        fixture.observe(config, &canonical)
    });
    // Config construction does not measure command execution, file reads or network attempts.
    assert_eq!(
        (refusals, observations),
        (
            vec![true; 8],
            [
                expected_row("openai", Some("openai"), Some("")),
                expected_row("openai", Some("openai"), Some("")),
            ]
        )
    );
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_provider_config_ordinary_preserves_custom_and_url_selection()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let canonical = canonical_provider();
    let initial = fixture.builder().build().await?;
    fixture.profile("model_provider = 'owned'")?;
    let custom = fixture.builder().build().await?;
    let expected_custom = ModelProviderInfo {
        name: "Owned provider".to_owned(),
        base_url: Some("https://owned.invalid/v1".to_owned()),
        ..Default::default()
    };
    assert_ne!(custom.model_provider, canonical);
    let overridden = fixture
        .builder()
        .harness_overrides(ConfigOverrides {
            cwd: Some(fixture.project.to_path_buf()),
            model_provider: Some("openai".to_owned()),
            ..Default::default()
        })
        .build()
        .await?;
    fixture.profile("model_provider = 'openai'\nopenai_base_url = 'https://owned.invalid/v1'")?;
    let url = fixture.builder().build().await?;
    let mut expected_url = canonical.clone();
    expected_url.base_url = Some("https://owned.invalid/v1".to_owned());
    assert_ne!(url.model_provider, canonical);
    for config in [&custom, &overridden, &url] {
        assert_eq!(security(config), security(&initial));
    }
    assert_eq!(
        [
            fixture.observe(&initial, &canonical),
            fixture.observe(&custom, &expected_custom),
            fixture.observe(&overridden, &canonical),
            fixture.observe(&url, &expected_url),
        ],
        [
            expected_row(
                "openai", /*raw_selected*/ None, /*raw_base_url*/ None
            ),
            expected_row("owned", Some("owned"), /*raw_base_url*/ None),
            expected_row("openai", Some("owned"), /*raw_base_url*/ None),
            expected_row("openai", Some("openai"), Some("https://owned.invalid/v1")),
        ]
    );
    // The inherited deserializer still refuses reserved built-in declarations.
    fixture.profile("[model_providers.openai]\nname = 'Reserved provider'")?;
    let error = fixture
        .builder()
        .build()
        .await
        .err()
        .expect("reserved built-in provider must remain refused");
    assert_eq!(
        (
            error.kind(),
            error
                .to_string()
                .contains("model_providers contains reserved built-in provider IDs: `openai`.")
        ),
        (std::io::ErrorKind::InvalidData, true)
    );
    Ok(())
}

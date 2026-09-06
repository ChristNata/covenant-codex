use super::*;
use codex_config::RequirementSource;
use pretty_assertions::assert_eq;
use std::fs;

const CANARY: &str = "CONFIG-HOOK-PRIVATE";
const SECURITY: &str = r#"
allowed_approval_policies = ["on-request"]
allowed_sandbox_modes = ["read-only"]
allowed_login_methods = ["api"]
cli_auth_credentials_store = "file"
allow_login_shell = false
"#;
const HOSTILE: &str = r#"
web_search = "live"
notify = ["fixture-notifier", "never-executed"]
[orchestrator.skills]
enabled = true
[orchestrator.mcp]
enabled = true
[mcp_servers.fixture]
command = "fixture-mcp-never-executed"
args = ["--synthetic"]
[tools.experimental_request_user_input]
enabled = true
[tools.update_plan]
enabled = true
[tools.web_search]
context_size = "high"
allowed_domains = ["fixture.invalid"]
location = { country = "ID", city = "Jakarta" }
[tool_suggest]
discoverables = [{ type = "connector", id = "fixture_connector" }]
disabled_tools = [{ type = "plugin", id = "fixture_plugin@fixture" }]
"#;
const HANDLERS: &[&str] = &[
    "type = 'command'\ncommand = 'CONFIG-HOOK-PRIVATE never-executed'",
    "type = 'mcp_tool'\nserver = 'fixture'\ntool = 'CONFIG-HOOK-PRIVATE'\ninput = { payload = 'CONFIG-HOOK-PRIVATE' }",
];

struct Fixture {
    _temporary: tempfile::TempDir,
    root: AbsolutePathBuf,
    home: AbsolutePathBuf,
    project: AbsolutePathBuf,
    selected: AbsolutePathBuf,
    requirements: AbsolutePathBuf,
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
        fs::create_dir_all(root.join(CANARY))?;
        for file in ["packaged.toml", "managed.toml", "system.toml"] {
            fs::write(root.join(file), "")?;
        }
        let requirements = root.join(format!("{CANARY}-requirements.toml"));
        fs::write(&requirements, SECURITY)?;
        fs::write(project.join(".fixture-root"), "")?;
        fs::write(
            project.join(".codex/config.toml"),
            "project_doc_max_bytes = 777",
        )?;
        let base = ConfigToml {
            model: Some("gpt-5.5".to_string()),
            sqlite_home: Some(root.join("sqlite")),
            approval_policy: Some(AskForApproval::OnRequest),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            allow_login_shell: Some(false),
            forced_login_method: Some(ForcedLoginMethod::Api),
            cli_auth_credentials_store: Some(AuthCredentialsStoreMode::File),
            project_root_markers: Some(vec![".fixture-root".to_string()]),
            projects: Some(HashMap::from([(
                project.to_string_lossy().to_string(),
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            ..Default::default()
        };
        fs::write(home.join("config.toml"), toml::to_string(&base).unwrap())?;
        let selected = home.join("worker.config.toml");
        fs::write(&selected, "model_reasoning_effort = 'low'")?;
        let loader = LoaderOverrides {
            packaged_defaults_path: Some(root.join("packaged.toml")),
            managed_config_path: Some(root.join("managed.toml").to_path_buf()),
            system_config_path: Some(root.join("system.toml").to_path_buf()),
            system_requirements_path: Some(requirements.to_path_buf()),
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
            requirements,
            loader,
        })
    }

    fn builder(&self) -> ConfigBuilder {
        ConfigBuilder::default()
            .codex_home(self.home.to_path_buf())
            .loader_overrides(self.loader.clone())
            .harness_overrides(ConfigOverrides {
                cwd: Some(self.project.to_path_buf()),
                bypass_hook_trust: Some(true),
                ..Default::default()
            })
    }

    fn profile(&self, content: &str) -> std::io::Result<()> {
        fs::write(&self.selected, content)
    }

    async fn required_hook(&self, handler: &str) -> std::io::Result<()> {
        let directory = TomlValue::String(self.root.join(CANARY).to_string_lossy().to_string());
        let document = format!(
            "{SECURITY}\n[hooks]\nmanaged_dir = {directory}\nwindows_managed_dir = {directory}\n[[hooks.SessionStart]]\n[[hooks.SessionStart.hooks]]\n{handler}\n"
        );
        fs::write(&self.requirements, &document)?;
        let expected: ConfigRequirementsToml = toml::from_str(&document).unwrap();
        let layers = load_config_layers_state(
            LOCAL_FS.as_ref(),
            &self.home,
            Some(self.project.clone()),
            &[],
            ConfigLoadOptions {
                loader_overrides: self.loader.clone(),
                ..Default::default()
            },
            &codex_config::NoopThreadConfigLoader,
        )
        .await?;
        let hooks = layers
            .requirements()
            .managed_hooks
            .as_ref()
            .expect("fixture hook requirement loaded");
        assert_eq!(
            (
                hooks.get(),
                hooks.get().handler_count(),
                hooks.source.as_ref()
            ),
            (
                expected.hooks.as_ref().unwrap(),
                1,
                Some(&RequirementSource::SystemRequirementsToml {
                    file: self.requirements.clone()
                })
            )
        );
        Ok(())
    }

    fn provenance(&self, config: &Config) -> (bool, bool, usize) {
        let layers = &config.config_layer_stack;
        (
            layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::User { file, profile } if file == &self.selected && profile.as_deref() == Some("worker"))),
            layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::Project { dot_codex_folder } if dot_codex_folder == &self.project.join(".codex"))),
            config.project_doc_max_bytes,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Channels {
    orchestrator_skills: bool,
    orchestrator_mcp: bool,
    mcp: HashMap<String, McpServerConfig>,
    web: WebSearchMode,
    web_config: Option<WebSearchConfig>,
    request_user_input: bool,
    update_plan: bool,
    notify: Option<Vec<String>>,
    bypass_hook_trust: bool,
    bypass_warning: bool,
    tool_suggest: ToolSuggestConfig,
    layer_tool_suggest: ToolSuggestConfig,
}

fn observe(config: &Config) -> Channels {
    Channels {
        orchestrator_skills: config.orchestrator_skills_enabled,
        orchestrator_mcp: config.orchestrator_mcp_enabled,
        mcp: config.mcp_servers.get().clone(),
        web: config.web_search_mode.value(),
        web_config: config.web_search_config.clone(),
        request_user_input: config.experimental_request_user_input_enabled,
        update_plan: config.update_plan_enabled,
        notify: config.notify.clone(),
        bypass_hook_trust: config.bypass_hook_trust,
        bypass_warning: config
            .startup_warnings
            .iter()
            .any(|warning| warning.contains("--dangerously-bypass-hook-trust")),
        tool_suggest: config.tool_suggest.clone(),
        layer_tool_suggest: resolve_tool_suggest_config_from_layer_stack(
            &config.config_layer_stack,
        ),
    }
}

#[cfg(feature = "covenant")]
fn closed() -> Channels {
    Channels {
        orchestrator_skills: false,
        orchestrator_mcp: false,
        mcp: HashMap::new(),
        web: WebSearchMode::Disabled,
        web_config: None,
        request_user_input: false,
        update_plan: false,
        notify: None,
        bypass_hook_trust: false,
        bypass_warning: false,
        tool_suggest: ToolSuggestConfig::default(),
        layer_tool_suggest: ToolSuggestConfig::default(),
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
            config.managed_auth_policy().allowed_login_methods
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
#[tokio::test]
async fn covenant_config_load_normalizes_nonfeature_channels() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let missing = fixture.builder().build().await?;
    fixture.profile(HOSTILE)?;
    let hostile = fixture.builder().build().await?;
    assert_eq!(
        (fixture.provenance(&missing), fixture.provenance(&hostile)),
        ((true, true, 777), (true, true, 777))
    );
    assert_eq!(security(&missing), security(&hostile));
    assert_eq!([observe(&missing), observe(&hostile)], [closed(), closed()]);
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_config_rebuild_and_mutations_keep_closed_values() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    fixture.profile(&format!("model_reasoning_effort = 'low'\n{HOSTILE}"))?;
    let initial = fixture
        .builder()
        .cli_overrides(vec![(
            "web_search".to_string(),
            TomlValue::String("indexed".to_string()),
        )])
        .build()
        .await?;
    fixture.profile(&format!(
        "model_reasoning_effort = 'high'\n{}",
        HOSTILE.replace("\"live\"", "\"cached\"")
    ))?;
    let refreshed = fixture.builder().build().await?;
    let rebuilt = initial
        .rebuild_preserving_session_layers(&refreshed)
        .await?;
    let raw_web = rebuilt
        .config_layer_stack
        .effective_config()
        .get("web_search")
        .and_then(TomlValue::as_str)
        .map(str::to_string);
    assert_eq!(
        (
            [
                fixture.provenance(&initial),
                fixture.provenance(&refreshed),
                fixture.provenance(&rebuilt)
            ],
            [
                initial.model_reasoning_effort.clone(),
                refreshed.model_reasoning_effort.clone(),
                rebuilt.model_reasoning_effort.clone()
            ],
            raw_web
        ),
        (
            [(true, true, 777); 3],
            [
                Some(ReasoningEffort::Low),
                Some(ReasoningEffort::High),
                Some(ReasoningEffort::High)
            ],
            Some("indexed".to_string())
        )
    );
    assert_eq!(
        [security(&initial), security(&refreshed)],
        [security(&rebuilt), security(&rebuilt)]
    );
    let mut actual = vec![observe(&initial), observe(&refreshed), observe(&rebuilt)];
    let mut cloned = rebuilt.clone();
    let raw: ConfigToml = toml::from_str(HOSTILE).unwrap();
    let mut turn_modes = Vec::new();
    for web in [
        WebSearchMode::Cached,
        WebSearchMode::Indexed,
        WebSearchMode::Live,
    ] {
        cloned.web_search_mode.set(web).unwrap();
        cloned.mcp_servers.set(raw.mcp_servers.clone()).unwrap();
        actual.push(observe(&cloned));
        for permissions in [PermissionProfile::read_only(), PermissionProfile::Disabled] {
            for external_web_access in [false, true] {
                turn_modes.push(resolve_web_search_mode_for_turn(
                    &cloned.web_search_mode,
                    &permissions,
                    ProviderCapabilities {
                        external_web_access,
                        ..ProviderCapabilities::default()
                    },
                ));
            }
        }
    }
    assert_eq!(security(&cloned), security(&rebuilt));
    actual.push(observe(&rebuilt));
    assert_eq!(
        (actual, turn_modes),
        (vec![closed(); 7], vec![WebSearchMode::Disabled; 12])
    );
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_config_mandatory_hooks_refuse_safely() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    fixture.profile(HOSTILE)?;
    let mut refusals = Vec::new();
    for handler in HANDLERS {
        fixture.required_hook(handler).await?;
        refusals.push(fixture.builder().build().await.is_err_and(|error| {
            let text = error.to_string();
            let debug = format!("{error:?}");
            error.kind() == ErrorKind::InvalidData
                && text == "Covenant profile rejected configuration requirements"
                && text.len() < 96
                && debug.len() < 160
                && !text.contains(CANARY)
                && !debug.contains(CANARY)
        }));
    }
    fs::write(
        &fixture.requirements,
        format!(
            "{SECURITY}\nallow_managed_hooks_only = true\nallowed_web_search_modes = ['cached']\n[mcp_servers.fixture.identity]\ncommand = 'fixture-mcp-never-executed'\n"
        ),
    )?;
    let agreeing = fixture.builder().build().await?;
    assert_eq!(fixture.provenance(&agreeing), (true, true, 777));
    let requirements = agreeing.config_layer_stack.requirements();
    assert_eq!(
        (
            requirements
                .allow_managed_hooks_only
                .as_ref()
                .map(|value| value.value),
            requirements
                .mcp_servers
                .as_ref()
                .map(|value| value.value.len()),
            requirements.managed_hooks.is_none()
        ),
        (Some(true), Some(1), true)
    );
    security(&agreeing);
    assert_eq!((refusals, observe(&agreeing)), (vec![true; 2], closed()));
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_config_ordinary_preserves_existing_channels() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    fixture.profile(HOSTILE)?;
    let mut config = fixture.builder().build().await?;
    assert_eq!(fixture.provenance(&config), (true, true, 777));
    let raw: ConfigToml = toml::from_str(HOSTILE).unwrap();
    let suggestion = raw.tool_suggest.unwrap();
    let expected = Channels {
        orchestrator_skills: true, orchestrator_mcp: true, mcp: raw.mcp_servers.clone(),
        web: WebSearchMode::Live,
        web_config: Some(serde_json::from_value(serde_json::json!({"filters":{"allowed_domains":["fixture.invalid"]},"user_location":{"type":"approximate","country":"ID","city":"Jakarta"},"search_context_size":"high"})).unwrap()),
        request_user_input: true, update_plan: true,
        notify: Some(vec!["fixture-notifier".to_string(), "never-executed".to_string()]),
        bypass_hook_trust: true, bypass_warning: true,
        tool_suggest: suggestion.clone(), layer_tool_suggest: suggestion,
    };
    let before = observe(&config);
    let initial_security = security(&config);
    config.web_search_mode.set(WebSearchMode::Indexed).unwrap();
    config.mcp_servers.set(raw.mcp_servers).unwrap();
    let after = observe(&config);
    let mut expected_after = expected.clone();
    expected_after.web = WebSearchMode::Indexed;
    let mut hooks = Vec::new();
    for handler in HANDLERS {
        fixture.required_hook(handler).await?;
        let accepted = fixture.builder().build().await?;
        assert_eq!(security(&accepted), initial_security);
        hooks.push(observe(&accepted));
    }
    assert_eq!(
        (before, after, hooks),
        (expected.clone(), expected_after, vec![expected; 2])
    );
    Ok(())
}

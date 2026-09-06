use super::*;
use crate::config::ConfigBuilder;
use crate::config::ConfigOverrides;
use codex_config::Constrained;
use codex_config::LoaderOverrides;
use codex_config::config_toml::ConfigToml;
use codex_config::types::AuthCredentialsStoreMode;
use codex_core_plugins::PluginLoadOutcome;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::McpServerContributor;
use codex_login::AuthManager;
use codex_login::AuthManagerConfig;
use codex_mcp::McpServerConflict;
use codex_mcp::McpServerSource;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::config_types::SandboxMode;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::io::Read;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use tracing::Subscriber;
use tracing::instrument::WithSubscriber;
use tracing::span::Attributes;
use tracing::span::Id;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;

const MARKER: &[u8] = b"owned-mcp-input";
const CONTRIBUTOR: &str = "covenant_projection_fixture";
const SECURITY: &str = "allowed_approval_policies = ['on-request']\nallowed_sandbox_modes = ['read-only']\nallowed_login_methods = ['api']\ncli_auth_credentials_store = 'file'\nallow_login_shell = false\n";

#[derive(Clone, Copy)]
enum Scope {
    Global = 0,
    Step = 1,
}

#[derive(Default)]
struct Counts([[AtomicU8; 3]; 2]);

impl Counts {
    fn add(&self, scope: Scope, operation: usize) {
        self.0[scope as usize][operation]
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                Some(value.saturating_add(1))
            })
            .expect("bounded counter update");
    }
    fn snapshot(&self) -> [[u8; 3]; 2] {
        std::array::from_fn(|scope| {
            std::array::from_fn(|operation| self.0[scope][operation].load(Ordering::SeqCst))
        })
    }
}

struct PluginEntryLayer {
    counts: Arc<Counts>,
    scope: Scope,
}

impl<S: Subscriber> Layer<S> for PluginEntryLayer {
    fn on_new_span(&self, attributes: &Attributes<'_>, _id: &Id, _context: Context<'_, S>) {
        let metadata = attributes.metadata();
        if metadata.name() == "plugins_for_config"
            && metadata.target() == "codex_core_plugins::manager"
        {
            self.counts.add(self.scope, /*operation*/ 2);
        }
    }
}

#[derive(Clone)]
struct Seed(&'static str);

struct Contributor {
    marker: AbsolutePathBuf,
    counts: Arc<Counts>,
}

impl McpServerContributor<Config> for Contributor {
    fn id(&self) -> &'static str {
        CONTRIBUTOR
    }

    fn contribute<'a>(
        &'a self,
        context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        let scope = if context.thread_init().is_some() {
            Scope::Step
        } else {
            Scope::Global
        };
        self.counts.add(scope, /*operation*/ 0);
        Box::pin(async move {
            let facts = (
                context
                    .thread_init()
                    .and_then(|init| init.get::<Seed>())
                    .map(|seed| seed.0),
                context.thread_store().map(ExtensionData::level_id),
                context.session_source().cloned(),
                context.originator(),
                context.ready_selected_capability_roots().map(<[_]>::len),
                context.executor_capability_discovery().is_some(),
            );
            let expected = match scope {
                Scope::Global => (None, None, None, None, None, false),
                Scope::Step => (
                    Some("seed"),
                    Some("fixture-thread"),
                    Some(SessionSource::Cli),
                    Some("covenant-test"),
                    Some(0),
                    false,
                ),
            };
            assert_eq!(facts, expected);
            let mut bytes = Vec::with_capacity(MARKER.len() + 1);
            fs::File::open(&self.marker)
                .unwrap()
                .take((MARKER.len() + 1) as u64)
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes, MARKER);
            self.counts.add(scope, /*operation*/ 1);
            vec![
                McpServerContribution::Set {
                    name: "callback".to_owned(),
                    config: Box::new(server("callback")),
                },
                McpServerContribution::SelectedPlugin {
                    name: "selected".to_owned(),
                    plugin_id: "fixture@local".to_owned(),
                    plugin_display_name: "Fixture".to_owned(),
                    selection_order: 0,
                    config: Box::new(server("selected")),
                },
                McpServerContribution::SelectedPluginPackage {
                    selected_root_id: "fixture-root".to_owned(),
                    plugin_id: "fixture@local".to_owned(),
                    plugin_display_name: "Fixture".to_owned(),
                    connector_ids: vec!["fixture-connector".to_owned()],
                },
            ]
        })
    }
}

fn server(name: &str) -> McpServerConfig {
    serde_json::from_value(serde_json::json!({"url": format!("https://fixture.invalid/{name}")}))
        .unwrap()
}

struct Fixture {
    _temporary: tempfile::TempDir,
    root: AbsolutePathBuf,
    home: AbsolutePathBuf,
    cwd: AbsolutePathBuf,
    marker: AbsolutePathBuf,
    loader: LoaderOverrides,
}

impl Fixture {
    fn new() -> std::io::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = AbsolutePathBuf::try_from(temporary.path().canonicalize()?)?;
        let home = root.join("home");
        let cwd = root.join("workspace");
        fs::create_dir_all(&home)?;
        fs::create_dir_all(&cwd)?;
        for name in ["packaged.toml", "managed.toml", "system.toml"] {
            fs::write(root.join(name), "")?;
        }
        fs::write(root.join("requirements.toml"), SECURITY)?;
        let marker = root.join("marker");
        fs::write(&marker, MARKER)?;
        let cfg = ConfigToml {
            model: Some("gpt-5.5".to_owned()),
            sqlite_home: Some(root.join("sqlite")),
            project_doc_max_bytes: Some(739),
            approval_policy: Some(AskForApproval::OnRequest),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            allow_login_shell: Some(false),
            forced_login_method: Some(ForcedLoginMethod::Api),
            cli_auth_credentials_store: Some(AuthCredentialsStoreMode::File),
            mcp_servers: HashMap::from([("initial".to_owned(), server("initial"))]),
            ..Default::default()
        };
        fs::write(home.join("config.toml"), toml::to_string(&cfg).unwrap())?;
        let loader = LoaderOverrides {
            packaged_defaults_path: Some(root.join("packaged.toml")),
            managed_config_path: Some(root.join("managed.toml").to_path_buf()),
            system_config_path: Some(root.join("system.toml").to_path_buf()),
            system_requirements_path: Some(root.join("requirements.toml").to_path_buf()),
            macos_managed_config_requirements_base64: Some(String::new()),
            #[cfg(target_os = "macos")]
            managed_preferences_base64: Some(String::new()),
            ..Default::default()
        };
        Ok(Self {
            _temporary: temporary,
            root,
            home,
            cwd,
            marker,
            loader,
        })
    }

    async fn observe(&self, input: Input) -> std::io::Result<Observation> {
        let mut config = ConfigBuilder::default()
            .codex_home(self.home.to_path_buf())
            .loader_overrides(self.loader.clone())
            .harness_overrides(ConfigOverrides {
                cwd: Some(self.cwd.to_path_buf()),
                ..Default::default()
            })
            .build()
            .await?;
        for feature in [Feature::Plugins, Feature::RemotePlugin, Feature::Apps] {
            config.features.disable(feature).unwrap();
        }
        assert_eq!(
            (
                config.project_doc_max_bytes,
                config.cwd.clone(),
                config.codex_home.clone()
            ),
            (739, self.cwd.clone(), self.home.clone())
        );
        let raw = config.config_layer_stack.get_active_user_layer().unwrap();
        assert_eq!(
            raw.config["mcp_servers"]["initial"]["url"].as_str(),
            Some("https://fixture.invalid/initial")
        );
        let security_before = security(&config);
        if let Input::Replaced = input {
            let replacement = HashMap::from([("replacement".to_owned(), server("replacement"))]);
            config.mcp_servers = Constrained::allow_any(replacement.clone());
            assert_eq!(config.mcp_servers.get(), &replacement);
        }
        let baseline = config
            .to_mcp_config_with_loaded_plugins(&PluginLoadOutcome::default(), std::iter::empty());
        let counts = Arc::new(Counts::default());
        let mut extensions = ExtensionRegistryBuilder::new();
        extensions.mcp_server_contributor(Arc::new(Contributor {
            marker: self.marker.clone(),
            counts: counts.clone(),
        }));
        let auth = AuthManager::from_auth_for_testing_with_home(
            CodexAuth::from_api_key("fixture-not-a-real-key"),
            self.home.to_path_buf(),
        );
        let plugins = Arc::new(crate::plugins::plugins_manager_for_config(&config, auth));
        let manager = McpManager::new_with_extensions(
            plugins,
            Arc::new(extensions.build()),
            ConnectorRuntimeManager::default(),
        );
        let global = manager
            .runtime_config(&config)
            .with_subscriber(tracing_subscriber::registry().with(PluginEntryLayer {
                counts: counts.clone(),
                scope: Scope::Global,
            }))
            .await;
        let mut init = ExtensionDataInit::new();
        init.insert(Seed("seed"));
        let store = ExtensionData::new_with_init("fixture-thread", init.clone());
        let step = manager
            .runtime_config_for_step(
                &config,
                &init,
                &store,
                McpThreadIdentity {
                    session_source: &SessionSource::Cli,
                    originator: "covenant-test",
                    environments: McpEnvironmentScope::Initial(&[]),
                },
                &[],
                /*executor_capability_discovery*/ None,
            )
            .with_subscriber(tracing_subscriber::registry().with(PluginEntryLayer {
                counts: counts.clone(),
                scope: Scope::Step,
            }))
            .await;
        assert_eq!(
            [metadata(&global), metadata(&step.config)],
            [metadata(&baseline), metadata(&baseline)]
        );
        assert_eq!(security(&config), security_before);
        assert!(config.codex_home.as_path().starts_with(self.root.as_path()));
        Ok(Observation {
            global: catalog(&global),
            step: catalog(&step.config),
            plugins_available: step.plugins_available,
            selected: step
                .selected_plugins
                .plugins
                .into_iter()
                .map(|plugin| (plugin.selected_root_id, plugin.plugin_id))
                .collect(),
            disabled_roots: step.selected_plugins.disabled_plugin_roots,
            counts: counts.snapshot(),
        })
    }
}

#[derive(Clone, Copy)]
enum Input {
    Configured,
    Replaced,
}

fn security(config: &Config) -> impl Debug + PartialEq + use<> {
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

fn metadata(config: &McpConfig) -> impl Debug + PartialEq + use<> {
    (
        (
            config.chatgpt_base_url.clone(),
            config.apps_mcp_product_sku.clone(),
            config.codex_home.clone(),
            config.mcp_oauth_credentials_store_mode,
            config.oauth_refresh_mode,
            config.auth_keyring_backend_kind,
            config.mcp_oauth_callback_port,
            config.mcp_oauth_callback_url.clone(),
            config.optional_mcp_startup_grace,
            config.skill_mcp_dependency_install_enabled,
        ),
        (
            config.approval_policy.clone(),
            [
                AskForApproval::OnRequest,
                AskForApproval::Never,
                AskForApproval::UnlessTrusted,
            ]
            .map(|policy| config.approval_policy.can_set(&policy).is_ok()),
            config.permission_profile.clone(),
            config.approvals_reviewer,
            config.environment_cwds.clone(),
            config.server_permission_profiles.clone(),
            config.codex_linux_sandbox_exe.clone(),
            config.use_legacy_landlock,
        ),
        (
            config.apps_enabled,
            config.prefix_mcp_tool_names,
            config.non_prefixed_mcp_tool_servers.clone(),
            config.protocol_mode,
            serde_json::to_value(&config.client_elicitation_capability).unwrap(),
        ),
        config.config_layer_stack.clone(),
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Catalog {
    servers: BTreeMap<String, (McpServerConfig, McpServerSource)>,
    conflicts: Vec<McpServerConflict>,
    connectors: ConnectorSnapshot,
}

fn catalog(config: &McpConfig) -> Catalog {
    let resolved = &config.mcp_server_catalog;
    Catalog {
        servers: resolved
            .configured_servers()
            .into_iter()
            .map(|(name, server)| {
                let source = resolved.server(&name).unwrap().source().clone();
                (name, (server, source))
            })
            .collect(),
        conflicts: resolved.conflicts().to_vec(),
        connectors: config.connector_snapshot.clone(),
    }
}

#[derive(Debug, Default, PartialEq)]
struct Observation {
    global: Catalog,
    step: Catalog,
    plugins_available: bool,
    selected: Vec<(String, String)>,
    disabled_roots: Vec<String>,
    counts: [[u8; 3]; 2],
}

#[cfg(not(feature = "covenant"))]
fn ordinary(input: Input) -> Observation {
    let name = match input {
        Input::Configured => "initial",
        Input::Replaced => "replacement",
    };
    let servers = BTreeMap::from([
        (name.to_owned(), (server(name), McpServerSource::Config)),
        (
            "callback".to_owned(),
            (
                server("callback"),
                McpServerSource::Extension {
                    id: CONTRIBUTOR.to_owned(),
                    host_owned_apps: false,
                },
            ),
        ),
        (
            "selected".to_owned(),
            (
                server("selected"),
                McpServerSource::SelectedPlugin(McpPluginAttribution::new(
                    "fixture@local".to_owned(),
                    "Fixture".to_owned(),
                )),
            ),
        ),
    ]);
    let expected = Catalog {
        servers,
        ..Default::default()
    };
    Observation {
        global: expected.clone(),
        step: expected,
        disabled_roots: vec!["fixture-root".to_owned()],
        counts: [[1; 3]; 2],
        ..Default::default()
    }
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_mcp_projection_skips_global_and_step_effects() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(
        fixture.observe(Input::Configured).await?,
        Observation::default()
    );
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_mcp_projection_replaced_wrapper_cannot_restore_effects() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(
        fixture.observe(Input::Replaced).await?,
        Observation::default()
    );
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_mcp_projection_ordinary_runs_both_scopes_and_preserves_sources()
-> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let actual = [
        fixture.observe(Input::Configured).await?,
        fixture.observe(Input::Replaced).await?,
    ];
    assert_eq!(
        actual,
        [ordinary(Input::Configured), ordinary(Input::Replaced)]
    );
    Ok(())
}

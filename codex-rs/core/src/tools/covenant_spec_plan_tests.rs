use super::build_tool_router;
use crate::config::Config;
use crate::config::ConfigBuilder;
use crate::config::ConfigOverrides;
use crate::config::LoaderOverrides;
use crate::responses_metadata::TurnToolNamespacesInfo;
use crate::session::step_context::StepContext;
use crate::session::tests::make_session_and_context_with_auth_and_config_and_rx;
use crate::thread_manager::build_models_manager;
use crate::tools::registry::ToolExposure;
use codex_config::ConfigLayerSource;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::ProjectConfig;
use codex_config::types::AuthCredentialsStoreMode;
use codex_login::AuthManager;
use codex_login::AuthManagerConfig;
use codex_login::CodexAuth;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::config_types::SandboxMode;
use codex_protocol::config_types::TrustLevel;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::protocol::AskForApproval;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::create_tools_json_for_responses_api;
use codex_tools::create_tools_raw_json_for_responses_api;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use serde_json::Value;
#[cfg(feature = "covenant")]
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

const MODELS: [&str; 4] = ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.2"];
const EXTRA_FLAGS: [&str; 3] = ["deferred_executor", "token_budget", "current_time_reminder"];
const SECURITY: &str = "allowed_approval_policies = ['on-request']\nallowed_sandbox_modes = ['read-only']\nallowed_login_methods = ['api']\ncli_auth_credentials_store = 'file'\nallow_login_shell = false\n";

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
        fs::write(home.join("config.toml"), toml::to_string(&base).unwrap())?;
        let selected = home.join("worker.config.toml");
        fs::write(&selected, "")?;
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

    async fn observe(
        &self,
        model: &str,
        flags: &[&str],
        overrides: Vec<(String, toml::Value)>,
    ) -> Observation {
        let mut profile = toml::toml! {
            web_search = "disabled"
            update_plan_enabled = false
            experimental_request_user_input_enabled = false
            [agents]
            enabled = false
            [features]
            deferred_executor = false
            token_budget = false
            current_time_reminder = false
        };
        profile.insert("model".to_owned(), toml::Value::String(model.to_owned()));
        for flag in flags {
            profile["features"][*flag] = toml::Value::Boolean(true);
        }
        #[cfg(not(feature = "covenant"))]
        {
            // Ordinary Config needs a caller catalog to exercise the same pinned metadata.
            let catalog_path = self.root.join("ordinary-catalog.json");
            fs::write(
                &catalog_path,
                serde_json::to_vec(&codex_models_manager::covenant_model_catalog().unwrap())
                    .unwrap(),
            )
            .unwrap();
            profile.insert(
                "model_catalog_json".to_owned(),
                toml::Value::String(catalog_path.to_string_lossy().to_string()),
            );
        }
        fs::write(&self.selected, toml::to_string(&profile).unwrap()).unwrap();
        let requested_overrides = overrides.clone();
        let config = ConfigBuilder::default()
            .codex_home(self.home.to_path_buf())
            .loader_overrides(self.loader.clone())
            .cli_overrides(overrides)
            .harness_overrides(ConfigOverrides {
                cwd: Some(self.project.to_path_buf()),
                ..Default::default()
            })
            .build()
            .await
            .expect("owned tool Config should load");
        let layers = &config.config_layer_stack;
        let effective = layers.effective_config();
        assert_eq!(
            (
                layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::User { file, profile } if file == &self.selected && profile.as_deref() == Some("worker"))),
                layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::Project { dot_codex_folder } if dot_codex_folder == &self.project.join(".codex"))),
                &config.cwd, &config.codex_home, config.project_doc_max_bytes, config.model.as_deref(),
            ),
            (true, true, &self.project, &self.home, 777, Some(model))
        );
        for flag in flags {
            assert_eq!(
                effective.get("features").and_then(|v| v.get(*flag)),
                Some(&toml::Value::Boolean(true))
            );
        }
        for (key, expected) in requested_overrides {
            let actual = key
                .split('.')
                .try_fold(&effective, |value, part| value.get(part));
            assert_eq!(actual, Some(&expected));
        }
        assert_security(&config);
        assert!(config.codex_home.starts_with(&self.root));
        assert!(
            config.model_catalog.is_some(),
            "fixture requires the actual static catalog branch"
        );
        let auth = AuthManager::from_auth_for_testing_with_home(
            CodexAuth::from_api_key("synthetic-tool-fixture"),
            self.home.to_path_buf(),
        );
        let manager = build_models_manager(&config, auth);
        let expected_model = manager
            .get_model_info(model, &config.to_models_manager_config())
            .await;
        assert_eq!(
            (
                expected_model.slug.as_str(),
                expected_model.use_responses_lite,
                expected_model.used_fallback_model_metadata
            ),
            (model, false, false)
        );
        let expected_permissions = config.permissions.clone();
        let (session, turn, _events) = make_session_and_context_with_auth_and_config_and_rx(
            CodexAuth::from_api_key("synthetic-tool-fixture"),
            Vec::new(),
            |initial| *initial = config,
        )
        .await;
        assert_security(&turn.config);
        assert_eq!(
            (
                turn.config.permissions == expected_permissions,
                turn.model_info().as_ref() == &expected_model
            ),
            (true, true)
        );
        let step = StepContext::for_test(Arc::clone(&turn));
        assert_eq!(
            (
                step.environments.environments.len(),
                step.environments.turn_environments().count(),
                step.environments
                    .turn_environments()
                    .all(|env| !env.environment.is_remote()),
                step.mcp.has_servers()
            ),
            (1, 1, true, false)
        );
        let router = build_tool_router(
            &session,
            &turn,
            step.settings.model_info.as_ref(),
            step.settings.model_info.model_messages.as_ref(),
            &step.environments,
            &step.mcp,
            /*apps_enabled*/ false,
            &turn.extension_data,
            /*tool_suggest_candidates*/ None,
        )
        .expect("actual tool factory should construct");
        let specs = router.model_visible_specs().to_vec();
        let wire = create_tools_json_for_responses_api(&specs).unwrap();
        let raw = create_tools_raw_json_for_responses_api(&specs).unwrap();
        assert!(
            raw.get().len() < 128 * 1024,
            "fixture tool observation exceeded its output bound"
        );
        assert_eq!(
            serde_json::from_str::<Value>(raw.get()).unwrap() == Value::Array(wire.clone()),
            true
        );
        let registered = router.registered_tool_names_for_test();
        let exposures = registered
            .iter()
            .map(|name| (name.clone(), router.tool_exposure_for_test(name)))
            .collect();
        Observation {
            registered,
            exposures,
            specs,
            wire,
            mode: router.tool_mode(),
            code_mode: router.code_mode_tool_names().clone(),
            namespaces: router.tool_namespaces_info().cloned(),
            deferred: router.deferred_tool_namespaces(),
            worker: router.requires_code_mode_worker(),
            terminal: router.has_terminal_controls(),
            children: router.can_manage_children(),
        }
    }
}

fn assert_security(config: &Config) {
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
}

#[derive(Clone, Debug, PartialEq)]
struct Observation {
    registered: Vec<ToolName>,
    exposures: Vec<(ToolName, Option<ToolExposure>)>,
    specs: Vec<ToolSpec>,
    wire: Vec<Value>,
    mode: ToolMode,
    code_mode: BTreeMap<String, ToolName>,
    namespaces: Option<TurnToolNamespacesInfo>,
    deferred: BTreeMap<String, String>,
    worker: bool,
    terminal: bool,
    children: bool,
}

fn wire_shapes(tools: &[Value]) -> Vec<(&str, &str)> {
    let mut shapes: Vec<_> = tools
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().expect("actual tool name"),
                tool["type"].as_str().expect("actual wire form"),
            )
        })
        .collect();
    shapes.sort_unstable();
    shapes
}

#[cfg(feature = "covenant")]
fn assert_two_tools(observed: &Observation) {
    // Registry storage canonicalizes keys; advertised wire names must remain unqualified.
    let names = vec![
        ToolName::namespaced("functions", "apply_patch"),
        ToolName::namespaced("functions", "exec_command"),
    ];
    let shapes = wire_shapes(&observed.wire);
    assert_eq!(
        (
            &observed.registered,
            shapes,
            observed.mode,
            observed.code_mode.is_empty(),
            observed.namespaces.is_none(),
            observed.deferred.is_empty(),
            observed.worker,
            observed.terminal,
            observed.children
        ),
        (
            &names,
            vec![("apply_patch", "custom"), ("exec_command", "function")],
            ToolMode::Direct,
            true,
            true,
            true,
            false,
            false,
            false
        )
    );
    assert_eq!(
        observed
            .exposures
            .iter()
            .all(|(_, exposure)| exposure.is_some_and(|value| value.is_direct())),
        true
    );
    let exec = observed
        .wire
        .iter()
        .find(|tool| tool["name"] == "exec_command")
        .unwrap();
    let patch = observed
        .wire
        .iter()
        .find(|tool| tool["name"] == "apply_patch")
        .unwrap();
    assert_eq!(
        (
            exec.pointer("/parameters/properties/cmd/type"),
            exec.pointer("/parameters/required"),
            patch.pointer("/format/type"),
            patch.pointer("/format/syntax")
        ),
        (
            Some(&json!("string")),
            Some(&json!(["cmd"])),
            Some(&json!("grammar")),
            Some(&json!("lark"))
        )
    );
    assert_eq!(
        patch
            .pointer("/format/definition")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("*** Begin Patch")),
        true
    );
    assert_eq!(
        (
            exec.pointer("/parameters/properties/timeout_ms/type"),
            exec.pointer("/parameters/properties/tty"),
            exec.pointer("/parameters/properties/yield_time_ms"),
            exec.pointer("/output_schema/properties/session_id")
        ),
        (Some(&json!("number")), None, None, None)
    );
    for spec in &observed.specs {
        let payload = match spec {
            ToolSpec::Function(_) => codex_tools::ToolPayload::Function {
                arguments: r#"{"cmd":"echo fixture"}"#.to_owned(),
            },
            ToolSpec::Freeform(_) => codex_tools::ToolPayload::Custom {
                input: "*** Begin Patch\n*** End Patch".to_owned(),
            },
            ToolSpec::Namespace(_) | ToolSpec::ToolSearch { .. } | ToolSpec::WebSearch { .. } => {
                panic!("unexpected wire form at closed factory")
            }
        };
        assert_eq!(
            codex_tools::CovenantTool::admit(&ToolName::plain(spec.name()), &payload).is_ok(),
            true
        );
    }
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_spec_plan_all_catalog_models_advertise_the_two_real_tools() {
    let fixture = Fixture::new().unwrap();
    let mut reference = None;
    let mut rows = Vec::new();
    for model in MODELS {
        let observed = fixture.observe(model, &[], Vec::new()).await;
        assert_two_tools(&observed);
        let expected = reference.get_or_insert_with(|| observed.clone());
        rows.push((model, observed == *expected));
    }
    assert_eq!(rows, MODELS.map(|model| (model, true)).to_vec());
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_spec_plan_hostile_effective_flags_cannot_expand_the_factory() {
    let fixture = Fixture::new().unwrap();
    let mut rows = Vec::new();
    for model in MODELS {
        let clean = fixture.observe(model, &[], Vec::new()).await;
        assert_two_tools(&clean);
        for flag in EXTRA_FLAGS {
            let observed = fixture.observe(model, &[flag], Vec::new()).await;
            rows.push((model, flag, observed == clean));
        }
        let combined = fixture.observe(model, &EXTRA_FLAGS, Vec::new()).await;
        rows.push((model, "combined", combined == clean));
        let attempted = ["unified_exec", "code_mode", "view_image"]
            .map(|flag| (format!("features.{flag}"), toml::Value::Boolean(true)));
        let mut overrides = attempted.to_vec();
        overrides.push((
            "web_search".to_owned(),
            toml::Value::String("live".to_owned()),
        ));
        let observed = fixture.observe(model, &EXTRA_FLAGS, overrides).await;
        rows.push((model, "pinned-and-hosted-attempts", observed == clean));
    }
    let expected: Vec<_> = MODELS
        .into_iter()
        .flat_map(|model| {
            [
                "deferred_executor",
                "token_budget",
                "current_time_reminder",
                "combined",
                "pinned-and-hosted-attempts",
            ]
            .map(|flag| (model, flag, true))
        })
        .collect();
    assert_eq!(rows, expected);
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_spec_plan_ordinary_flags_add_tools_and_retain_the_original_specs() {
    let fixture = Fixture::new().unwrap();
    let baseline = fixture.observe(MODELS[0], &[], Vec::new()).await;
    assert_eq!(
        (
            baseline
                .registered
                .contains(&ToolName::namespaced("functions", "exec_command")),
            baseline
                .registered
                .contains(&ToolName::namespaced("functions", "write_stdin")),
            baseline.terminal
        ),
        (true, true, true)
    );
    let mut changed = fixture.observe(MODELS[0], &EXTRA_FLAGS, Vec::new()).await;
    let added = vec![
        ToolName::namespaced("clock", "curr_time"),
        ToolName::namespaced("functions", "get_context_remaining"),
        ToolName::namespaced("functions", "new_context"),
        ToolName::namespaced("functions", "wait_for_environment"),
    ];
    assert_eq!(
        changed
            .registered
            .iter()
            .filter(|name| !baseline.registered.contains(name))
            .cloned()
            .collect::<Vec<_>>(),
        added
    );
    let roots = [
        "get_context_remaining",
        "new_context",
        "wait_for_environment",
        "clock",
    ];
    let added_shapes: Vec<_> = wire_shapes(&changed.wire)
        .into_iter()
        .filter(|(name, _)| roots.contains(name))
        .collect();
    assert_eq!(
        added_shapes,
        vec![
            ("clock", "namespace"),
            ("get_context_remaining", "function"),
            ("new_context", "function"),
            ("wait_for_environment", "function")
        ]
    );
    let clock = changed
        .wire
        .iter()
        .find(|tool| tool["name"] == "clock")
        .unwrap();
    assert_eq!(
        wire_shapes(clock["tools"].as_array().unwrap()),
        vec![("curr_time", "function")]
    );
    changed.registered.retain(|name| !added.contains(name));
    changed.exposures.retain(|(name, _)| !added.contains(name));
    changed.specs.retain(|spec| !roots.contains(&spec.name()));
    changed
        .wire
        .retain(|tool| !roots.contains(&tool["name"].as_str().unwrap_or_default()));
    assert_eq!(
        changed == baseline,
        true,
        "complete original registry/spec observation must survive the ordinary feature additions"
    );
}

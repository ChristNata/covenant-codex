use super::*;
use codex_config::RequirementSource;
use pretty_assertions::assert_eq;
#[cfg(feature = "covenant")]
use std::fs;

// Independent contract rows: never use the production policy as the test oracle.
const DISABLED: &[Feature] = &[
    Feature::UnifiedExec,
    Feature::ShellZshFork,
    Feature::UnifiedExecZshFork,
    Feature::ShellSnapshot,
    Feature::ShellSnapshotV2,
    Feature::CodeMode,
    Feature::CodeModeHost,
    Feature::CodeModePrewarm,
    Feature::CodeModeInterrupt,
    Feature::CodeModeOnly,
    Feature::Collab,
    Feature::MultiAgentV2,
    Feature::Goals,
    Feature::MemoryTool,
    Feature::ExternalAgentMemoryImport,
    Feature::Chronicle,
    Feature::Apps,
    Feature::EnableMcpApps,
    Feature::Mcp20260728,
    Feature::McpOAuthRefreshCoordination,
    Feature::DeferredToolWorldState,
    Feature::NonPrefixedMcpToolNames,
    Feature::ToolSuggest,
    Feature::RecommendedPlugins,
    Feature::Plugins,
    Feature::ExecutorCapabilityDiscovery,
    Feature::RemotePlugin,
    Feature::PluginSharing,
    Feature::SkillMcpDependencyInstall,
    Feature::SkillSearch,
    Feature::WorkspaceDependencies,
    Feature::ViewImage,
    Feature::SleepTool,
    Feature::RequestPermissionsTool,
    Feature::WebSearchRequest,
    Feature::WebSearchCached,
    Feature::StandaloneWebSearch,
    Feature::ImageGeneration,
    Feature::Artifact,
    Feature::DefaultModeRequestUserInput,
    Feature::InAppBrowser,
    Feature::BrowserUse,
    Feature::BrowserUseFullCdpAccess,
    Feature::BrowserUseExternal,
    Feature::ComputerUse,
    Feature::RealtimeConversation,
    Feature::CodexHooks,
    Feature::GuardianApproval,
    Feature::GuardianReuseParentCompaction,
    Feature::GuardianEnhancedNodeReplTranscripts,
    Feature::GuardianNodeReplTranscriptImages,
    Feature::GuardianV2,
    Feature::GuardianExt,
    Feature::RespectSystemProxy,
    Feature::StepModelSwitching,
];
#[cfg(feature = "covenant")]
const REFUSAL: &str = "Covenant profile rejected feature requirements";
const CANARY: &str = "PROFILE-PRIVATE-CANARY";

fn hostile() -> Features {
    let mut features = Features::default();
    for feature in DISABLED {
        features.enable(*feature);
    }
    features
}

#[cfg(feature = "covenant")]
fn observation(features: &ManagedFeatures) -> (bool, Vec<bool>) {
    (
        features.enabled(Feature::ShellTool),
        DISABLED.iter().map(|f| features.enabled(*f)).collect(),
    )
}

#[cfg(feature = "covenant")]
fn closed() -> (bool, Vec<bool>) {
    (true, vec![false; DISABLED.len()])
}

fn requirements(entries: &[(&str, bool)]) -> Sourced<FeatureRequirementsToml> {
    Sourced::new(
        FeatureRequirementsToml {
            entries: entries
                .iter()
                .map(|(key, value)| (key.to_string(), *value))
                .collect(),
        },
        RequirementSource::EnterpriseManaged {
            id: CANARY.to_string(),
            name: CANARY.to_string(),
        },
    )
}

#[cfg(feature = "covenant")]
fn refused<T>(result: std::io::Result<T>) -> bool {
    result.is_err_and(|error| {
        let text = error.to_string();
        let debug = format!("{error:?}");
        error.kind() == ErrorKind::InvalidData
            && text == REFUSAL
            && text.len() < 96
            && debug.len() < 160
            && !text.contains(CANARY)
            && !debug.contains(CANARY)
    })
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_profile_constructors_and_mutations_keep_compiled_pins() -> std::io::Result<()> {
    let mut warnings = Vec::new();
    let mut actual = Vec::new();
    for (constructor, mut features) in [
        ("default", ManagedFeatures::default()),
        (
            "configured",
            ManagedFeatures::from_configured(hostile(), /*feature_requirements*/ None)?,
        ),
        (
            "warnings",
            ManagedFeatures::from_configured_with_warnings(
                hostile(),
                /*feature_requirements*/ None,
                &mut warnings,
            )?,
        ),
        ("conversion", ManagedFeatures::from(hostile())),
    ] {
        actual.push((
            constructor,
            "initial",
            None,
            observation(&features) == closed(),
        ));
        features.enable(Feature::ExecutedToolCallMetadata).unwrap();
        assert!(features.enabled(Feature::ExecutedToolCallMetadata));
        let requested = hostile();
        assert!(features.can_set(&requested).is_ok());
        features.set(requested).unwrap();
        assert!(!features.enabled(Feature::ExecutedToolCallMetadata));
        actual.push((constructor, "set", None, observation(&features) == closed()));
        for feature in DISABLED {
            features.enable(*feature).unwrap();
            actual.push((
                constructor,
                "enable",
                Some(*feature),
                observation(&features) == closed(),
            ));
            features.set_enabled(*feature, /*enabled*/ true).unwrap();
            actual.push((
                constructor,
                "set_enabled",
                Some(*feature),
                observation(&features) == closed(),
            ));
        }
        features.disable(Feature::ShellTool).unwrap();
        actual.push((
            constructor,
            "disable_shell",
            Some(Feature::ShellTool),
            observation(&features) == closed(),
        ));
        features
            .set_enabled(Feature::ShellTool, /*enabled*/ false)
            .unwrap();
        actual.push((
            constructor,
            "set_shell",
            Some(Feature::ShellTool),
            observation(&features) == closed(),
        ));
        let mut cloned = features.clone();
        cloned.enable(Feature::ExecutedToolCallMetadata).unwrap();
        cloned.enable(Feature::CodeModeOnly).unwrap();
        assert_eq!(
            (
                features.enabled(Feature::ExecutedToolCallMetadata),
                cloned.enabled(Feature::ExecutedToolCallMetadata)
            ),
            (false, true)
        );
        actual.push((
            constructor,
            "clone",
            Some(Feature::CodeModeOnly),
            observation(&cloned) == closed(),
        ));
    }
    let expected: Vec<_> = actual
        .iter()
        .map(|(constructor, operation, feature, _)| (*constructor, *operation, *feature, true))
        .collect();
    assert_eq!(actual.len(), 4 * (2 * DISABLED.len() + 5));
    assert_eq!(actual, expected);
    Ok(())
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_profile_source_layers_and_aliases_are_clamped() -> std::io::Result<()> {
    let base: FeaturesToml =
        toml::from_str("shell_tool = true\ncode_mode_only = false\nplugins = false").unwrap();
    let profile: FeaturesToml = toml::from_str("shell_tool = false\ncode_mode_only = true\nplugins = true\nconnectors = true\ncollab = true\nimagegenext = true\ncodex_hooks = true").unwrap();
    let raw = Features::from_sources(
        FeatureConfigSource {
            features: Some(&base),
            experimental_use_unified_exec_tool: Some(false),
        },
        FeatureConfigSource {
            features: Some(&profile),
            experimental_use_unified_exec_tool: Some(true),
        },
        FeatureOverrides {
            web_search_request: Some(true),
        },
    );
    assert!(
        raw.enabled(Feature::CodeMode)
            && raw.enabled(Feature::Apps)
            && raw.enabled(Feature::WebSearchRequest)
    );
    let resolved = ManagedFeatures::from_configured(raw, /*feature_requirements*/ None)?;
    assert_eq!(observation(&resolved), closed());
    Ok(())
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_profile_requirements_refuse_conflicts_without_echoing() -> std::io::Result<()> {
    let mut cases: Vec<_> = DISABLED
        .iter()
        .map(|feature| requirements(&[(feature.key(), true)]))
        .collect();
    cases.extend([
        requirements(&[("shell_tool", false)]),
        requirements(&[(CANARY, true)]),
        requirements(&[("multi_agent", false), ("collab", true)]),
        requirements(&[("auto_review", true)]),
        requirements(&[
            (Feature::ExecPermissionApprovals.key(), true),
            ("request_permissions", false),
        ]),
    ]);
    let mut actual = Vec::new();
    for required in cases {
        let mut warnings = Vec::new();
        actual.push((
            refused(ManagedFeatures::from_configured(
                Features::default(),
                Some(required.clone()),
            )),
            refused(ManagedFeatures::from_configured_with_warnings(
                Features::default(),
                Some(required.clone()),
                &mut warnings,
            )),
            refused(validate_feature_requirements_for_config_toml(
                &ConfigToml::default(),
                Some(&required),
            )),
            warnings.is_empty(),
        ));
    }
    let agreeing = requirements(&[
        ("shell_tool", true),
        ("unified_exec", false),
        (Feature::ExecutedToolCallMetadata.key(), true),
    ]);
    let mut compatible = ManagedFeatures::from_configured(hostile(), Some(agreeing.clone()))?;
    compatible
        .disable(Feature::ExecutedToolCallMetadata)
        .unwrap();
    assert!(compatible.enabled(Feature::ExecutedToolCallMetadata));
    assert!(
        validate_feature_requirements_for_config_toml(&ConfigToml::default(), Some(&agreeing))
            .is_ok()
    );
    assert_eq!(
        (actual, observation(&compatible)),
        (vec![(true, true, true, true); DISABLED.len() + 5], closed())
    );
    Ok(())
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_profile_real_loader_profile_and_rebuild_keep_pins() -> std::io::Result<()> {
    let temporary = tempfile::tempdir()?;
    let root = AbsolutePathBuf::try_from(temporary.path().canonicalize()?)?;
    let home = root.join("home");
    let project = root.join("project");
    let dot_codex = project.join(".codex");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&dot_codex)?;
    for file in [
        "packaged.toml",
        "managed.toml",
        "system.toml",
        "requirements.toml",
    ] {
        fs::write(root.join(file), "")?;
    }
    fs::write(project.join(".fixture-root"), "")?;
    let user = ConfigToml {
        model: Some("gpt-5.5".to_string()),
        sqlite_home: Some(root.join("sqlite")),
        project_root_markers: Some(vec![".fixture-root".to_string()]),
        projects: Some(HashMap::from([(
            project.to_string_lossy().to_string(),
            ProjectConfig {
                trust_level: Some(TrustLevel::Trusted),
            },
        )])),
        features: Some(toml::from_str("unified_exec = true\ncode_mode_only = true").unwrap()),
        ..Default::default()
    };
    fs::write(home.join("config.toml"), toml::to_string(&user).unwrap())?;
    fs::write(
        dot_codex.join("config.toml"),
        "project_doc_max_bytes = 1234\n[features]\nconnectors = true\ncodex_hooks = true\n",
    )?;
    let selected = home.join("work.config.toml");
    fs::write(
        &selected,
        "model_reasoning_effort = \"low\"\n[features]\nshell_tool = false\nplugins = true\n",
    )?;
    let loader = LoaderOverrides {
        packaged_defaults_path: Some(root.join("packaged.toml")),
        managed_config_path: Some(root.join("managed.toml").to_path_buf()),
        system_config_path: Some(root.join("system.toml").to_path_buf()),
        system_requirements_path: Some(root.join("requirements.toml").to_path_buf()),
        user_config_path: Some(selected.clone()),
        user_config_profile: Some("work".parse().unwrap()),
        macos_managed_config_requirements_base64: Some(String::new()),
        #[cfg(target_os = "macos")]
        managed_preferences_base64: Some(String::new()),
        ..Default::default()
    };
    let builder = ConfigBuilder::default()
        .codex_home(home.to_path_buf())
        .loader_overrides(loader)
        .harness_overrides(ConfigOverrides {
            cwd: Some(project.to_path_buf()),
            ..Default::default()
        });
    let initial = builder
        .clone()
        .cli_overrides(vec![(
            "features.plugins".to_string(),
            TomlValue::Boolean(true),
        )])
        .build()
        .await?;
    fs::write(
        &selected,
        "model_reasoning_effort = \"high\"\n[features]\nshell_tool = false\nplugins = false\n",
    )?;
    let refreshed = builder.build().await?;
    let rebuilt = initial
        .rebuild_preserving_session_layers(&refreshed)
        .await?;
    let provenance: Vec<_> = [&initial, &refreshed, &rebuilt].iter().map(|config| {
        let layers = &config.config_layer_stack;
        (layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::User { file, profile } if file == &selected && profile.as_deref() == Some("work"))),
         layers.layers_low_to_high().any(|layer| matches!(&layer.name, ConfigLayerSource::Project { dot_codex_folder } if dot_codex_folder == &dot_codex)), config.project_doc_max_bytes)
    }).collect();
    let session_override_preserved = rebuilt
        .config_layer_stack
        .effective_config()
        .get("features")
        .and_then(|features| features.get("plugins"))
        .and_then(TomlValue::as_bool);
    assert_eq!(
        (
            provenance,
            initial.model_reasoning_effort.clone(),
            refreshed.model_reasoning_effort.clone(),
            rebuilt.model_reasoning_effort.clone(),
            session_override_preserved
        ),
        (
            vec![(true, true, 1234); 3],
            Some(ReasoningEffort::Low),
            Some(ReasoningEffort::High),
            Some(ReasoningEffort::High),
            Some(true)
        )
    );
    let mut observations = Vec::new();
    for mut config in [initial, refreshed, rebuilt] {
        observations.push(observation(&config.features));
        config.features.set(hostile()).unwrap();
        observations.push(observation(&config.features));
    }
    assert_eq!(observations, vec![closed(); observations.len()]);
    Ok(())
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_profile_bootstrap_route_clamps_before_final_config() -> std::io::Result<()> {
    let configured = ConfigToml {
        features: Some(toml::from_str("respect_system_proxy = true").unwrap()),
        ..Default::default()
    };
    let agreeing = requirements(&[("respect_system_proxy", false)]);
    let mut actual = Vec::new();
    for required in [None, Some(&agreeing)] {
        actual.push((
            resolve_bootstrap_respect_system_proxy(&configured, required)?,
            resolve_bootstrap_auth_route_config(&configured, required)?
                .http_client_factory()
                .outbound_proxy_policy(),
        ));
    }
    let conflict = requirements(&[("respect_system_proxy", true)]);
    assert_eq!(
        (
            actual,
            refused(resolve_bootstrap_respect_system_proxy(
                &configured,
                Some(&conflict)
            )),
            refused(resolve_bootstrap_auth_route_config(
                &configured,
                Some(&conflict)
            ))
        ),
        (
            vec![(false, OutboundProxyPolicy::ReqwestDefault); 2],
            true,
            true
        )
    );
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_profile_ordinary_mode_preserves_existing_policy() -> std::io::Result<()> {
    let raw = Features::from_sources(
        FeatureConfigSource::default(),
        FeatureConfigSource::default(),
        FeatureOverrides::default(),
    );
    let mut managed = ManagedFeatures::from_configured(raw, /*feature_requirements*/ None)?;
    for feature in [
        Feature::Plugins,
        Feature::CodeModeOnly,
        Feature::RespectSystemProxy,
    ] {
        managed.enable(feature).unwrap();
    }
    let enabled: Vec<_> = [
        Feature::UnifiedExec,
        Feature::Plugins,
        Feature::CodeModeOnly,
        Feature::CodeMode,
        Feature::RespectSystemProxy,
    ]
    .iter()
    .map(|feature| managed.enabled(*feature))
    .collect();
    let required = requirements(&[("unified_exec", false), ("shell_tool", false)]);
    let mut pinned = ManagedFeatures::from_configured(hostile(), Some(required))?;
    pinned.enable(Feature::UnifiedExec).unwrap();
    pinned.enable(Feature::ShellTool).unwrap();
    assert_eq!(
        (
            enabled,
            pinned.enabled(Feature::UnifiedExec),
            pinned.enabled(Feature::ShellTool)
        ),
        (vec![true; 5], false, false)
    );
    Ok(())
}

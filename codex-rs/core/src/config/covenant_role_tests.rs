use super::*;
use codex_config::ConfigLayerEntry;
use codex_config::config_toml::AgentRoleToml;
use codex_config::config_toml::AgentsToml;
use codex_exec_server::CopyOptions;
use codex_exec_server::CreateDirectoryOptions;
use codex_exec_server::ExecutorFileSystemFuture;
use codex_exec_server::FileMetadata;
use codex_exec_server::FileSystemReadStream;
use codex_exec_server::FileSystemSandboxContext;
use codex_exec_server::GetMetadataOptions;
use codex_exec_server::ReadDirectoryEntry;
use codex_exec_server::RemoveOptions;
use codex_exec_server::WalkOptions;
use codex_exec_server::WalkOutcome;
use codex_exec_server::WriteFileOptions;
use pretty_assertions::assert_eq;
use std::fs;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
enum Operation {
    Canonicalize,
    Read,
    Stream,
    Text,
    Write,
    Create,
    Metadata,
    Directory,
    Walk,
    Remove,
    Copy,
}

struct CountingRoleFileSystem {
    watched: [PathUri; 3],
    counters: [[AtomicU8; 11]; 3],
}

impl CountingRoleFileSystem {
    fn record(&self, path: &PathUri, operation: Operation) {
        for (index, watched) in self.watched.iter().enumerate() {
            if path == watched {
                let _ = self.counters[index][operation as usize].fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |value| Some(value.saturating_add(1)),
                );
            }
        }
    }

    fn snapshot(&self) -> [[u8; 11]; 3] {
        std::array::from_fn(|path| {
            std::array::from_fn(|operation| self.counters[path][operation].load(Ordering::Relaxed))
        })
    }
}

impl ExecutorFileSystem for CountingRoleFileSystem {
    fn canonicalize<'a>(
        &'a self,
        path: &'a PathUri,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, PathUri> {
        self.record(path, Operation::Canonicalize);
        LOCAL_FS.canonicalize(path, sandbox)
    }

    fn read_file<'a>(
        &'a self,
        path: &'a PathUri,
        options: ReadFileOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, Vec<u8>> {
        self.record(path, Operation::Read);
        LOCAL_FS.read_file(path, options, sandbox)
    }

    fn read_file_stream<'a>(
        &'a self,
        path: &'a PathUri,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, FileSystemReadStream> {
        self.record(path, Operation::Stream);
        LOCAL_FS.read_file_stream(path, sandbox)
    }

    fn read_file_text<'a>(
        &'a self,
        path: &'a PathUri,
        options: ReadFileOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, String> {
        self.record(path, Operation::Text);
        LOCAL_FS.read_file_text(path, options, sandbox)
    }

    fn write_file<'a>(
        &'a self,
        path: &'a PathUri,
        contents: Vec<u8>,
        options: WriteFileOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, ()> {
        self.record(path, Operation::Write);
        LOCAL_FS.write_file(path, contents, options, sandbox)
    }

    fn create_directory<'a>(
        &'a self,
        path: &'a PathUri,
        options: CreateDirectoryOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, ()> {
        self.record(path, Operation::Create);
        LOCAL_FS.create_directory(path, options, sandbox)
    }

    fn get_metadata<'a>(
        &'a self,
        path: &'a PathUri,
        options: GetMetadataOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, FileMetadata> {
        self.record(path, Operation::Metadata);
        LOCAL_FS.get_metadata(path, options, sandbox)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a PathUri,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, Vec<ReadDirectoryEntry>> {
        self.record(path, Operation::Directory);
        LOCAL_FS.read_directory(path, sandbox)
    }

    fn walk<'a>(
        &'a self,
        path: &'a PathUri,
        options: WalkOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, WalkOutcome> {
        self.record(path, Operation::Walk);
        LOCAL_FS.walk(path, options, sandbox)
    }

    fn remove<'a>(
        &'a self,
        path: &'a PathUri,
        options: RemoveOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, ()> {
        self.record(path, Operation::Remove);
        LOCAL_FS.remove(path, options, sandbox)
    }

    fn copy<'a>(
        &'a self,
        source: &'a PathUri,
        destination: &'a PathUri,
        options: CopyOptions,
        sandbox: Option<&'a FileSystemSandboxContext>,
    ) -> ExecutorFileSystemFuture<'a, ()> {
        self.record(source, Operation::Copy);
        self.record(destination, Operation::Copy);
        LOCAL_FS.copy(source, destination, options, sandbox)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layers {
    User,
    None,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Agents {
    Missing,
    Enabled,
    Disabled,
}

struct Fixture {
    _temporary: tempfile::TempDir,
    root: AbsolutePathBuf,
    home: AbsolutePathBuf,
    cwd: AbsolutePathBuf,
    declared: AbsolutePathBuf,
    directory: AbsolutePathBuf,
    discovered: AbsolutePathBuf,
}

impl Fixture {
    fn new() -> std::io::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = AbsolutePathBuf::try_from(temporary.path().canonicalize()?)?;
        let home = root.join("home");
        let cwd = root.join("workspace");
        let directory = home.join("agents");
        fs::create_dir_all(&directory)?;
        fs::create_dir_all(&cwd)?;
        let declared = home.join("declared.toml");
        let discovered = directory.join("discovered.toml");
        fs::write(
            &declared,
            "name = 'declared_file'\ndescription = 'Declared file description'\nnickname_candidates = ['Delta']\ndeveloper_instructions = 'Declared fixture instructions'\nmodel = 'gpt-5.5'\n",
        )?;
        fs::write(
            &discovered,
            "name = 'discovered_file'\ndescription = 'Discovered file description'\nnickname_candidates = ['Sierra']\ndeveloper_instructions = 'Discovered fixture instructions'\nmodel = 'gpt-5.5'\n",
        )?;
        Ok(Self {
            _temporary: temporary,
            root,
            home,
            cwd,
            declared,
            directory,
            discovered,
        })
    }

    async fn load(
        &self,
        layers: Layers,
        agents: Agents,
    ) -> std::io::Result<(Config, RoleObservation)> {
        let fs = CountingRoleFileSystem {
            watched: [&self.declared, &self.directory, &self.discovered]
                .map(PathUri::from_abs_path),
            counters: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(0))),
        };
        let cfg = ConfigToml {
            model: Some("gpt-5.5".to_string()),
            sqlite_home: Some(self.root.join("sqlite")),
            project_doc_max_bytes: Some(731),
            approval_policy: Some(AskForApproval::OnRequest),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            allow_login_shell: Some(false),
            cli_auth_credentials_store: Some(AuthCredentialsStoreMode::File),
            forced_login_method: Some(ForcedLoginMethod::Api),
            agents: Some(AgentsToml {
                enabled: match agents {
                    Agents::Missing => None,
                    Agents::Enabled => Some(true),
                    Agents::Disabled => Some(false),
                },
                roles: BTreeMap::from([(
                    "declared_hint".to_string(),
                    AgentRoleToml {
                        description: Some("Declaration fallback".to_string()),
                        config_file: Some(self.declared.clone()),
                        nickname_candidates: Some(vec!["Fallback".to_string()]),
                    },
                )]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let raw = TomlValue::try_from(&cfg).unwrap();
        let file = self.home.join("config.toml");
        fs::write(&file, toml::to_string(&cfg).unwrap())?;
        let entries = match layers {
            Layers::User => vec![ConfigLayerEntry::new(
                ConfigLayerSource::User {
                    file: file.clone(),
                    profile: None,
                },
                raw.clone(),
            )],
            Layers::None => Vec::new(),
        };
        let stack = ConfigLayerStack::new(
            entries,
            ConfigRequirements::default(),
            ConfigRequirementsToml::default(),
        )?;
        let config = Config::load_config_with_layer_stack(
            &fs,
            cfg,
            ConfigOverrides {
                cwd: Some(self.cwd.to_path_buf()),
                ..Default::default()
            },
            self.home.clone(),
            stack,
        )
        .await?;
        let retained: Vec<_> = config
            .config_layer_stack
            .layers_low_to_high()
            .map(|layer| (layer.name.clone(), layer.config.clone()))
            .collect();
        let expected = match layers {
            Layers::User => vec![(
                ConfigLayerSource::User {
                    file,
                    profile: None,
                },
                raw,
            )],
            Layers::None => Vec::new(),
        };
        assert_eq!((config.project_doc_max_bytes, retained), (731, expected));
        security(&config);
        let observed = RoleObservation {
            enabled: config.agents_enabled,
            roles: config.agent_roles.clone(),
            calls: fs.snapshot(),
        };
        Ok((config, observed))
    }

    #[cfg(not(feature = "covenant"))]
    fn ordinary_roles(&self, layers: Layers) -> BTreeMap<String, AgentRoleConfig> {
        let mut roles = BTreeMap::from([(
            "declared_file".to_string(),
            AgentRoleConfig {
                description: Some("Declared file description".to_string()),
                config_file: Some(self.declared.to_path_buf()),
                nickname_candidates: Some(vec!["Delta".to_string()]),
            },
        )]);
        if layers == Layers::User {
            roles.insert(
                "discovered_file".to_string(),
                AgentRoleConfig {
                    description: Some("Discovered file description".to_string()),
                    config_file: Some(self.discovered.to_path_buf()),
                    nickname_candidates: Some(vec!["Sierra".to_string()]),
                },
            );
        }
        roles
    }
}

fn security(config: &Config) -> Permissions {
    assert_eq!(
        (
            config.permissions.approval_policy.value(),
            config.permissions.permission_profile().clone(),
            config.permissions.allow_login_shell,
            config.cli_auth_credentials_store_mode,
            config.forced_login_method
        ),
        (
            AskForApproval::OnRequest,
            PermissionProfile::read_only(),
            false,
            AuthCredentialsStoreMode::File,
            Some(ForcedLoginMethod::Api)
        )
    );
    config.permissions.clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleObservation {
    enabled: bool,
    roles: BTreeMap<String, AgentRoleConfig>,
    calls: [[u8; 11]; 3],
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_roles_materialization_skips_owned_role_io() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let mut actual = Vec::new();
    for layers in [Layers::User, Layers::None] {
        for agents in [Agents::Missing, Agents::Enabled, Agents::Disabled] {
            let (_, observed) = fixture.load(layers, agents).await?;
            actual.push((layers, agents, observed));
        }
    }
    let expected: Vec<_> = actual
        .iter()
        .map(|(layers, agents, _)| {
            (
                *layers,
                *agents,
                RoleObservation {
                    enabled: false,
                    roles: BTreeMap::new(),
                    calls: [[0; 11]; 3],
                },
            )
        })
        .collect();
    assert_eq!(actual.len(), 6);
    assert_eq!(actual, expected);
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_roles_ordinary_preserves_loading() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for layers in [Layers::User, Layers::None] {
        for agents in [Agents::Missing, Agents::Enabled, Agents::Disabled] {
            let (_, observed) = fixture.load(layers, agents).await?;
            let mut calls = [[0; 11]; 3];
            calls[0][Operation::Metadata as usize] = 1;
            calls[0][Operation::Text as usize] = 1;
            if layers == Layers::User {
                calls[1][Operation::Directory as usize] = 1;
                calls[2][Operation::Text as usize] = 1;
            }
            actual.push((layers, agents, observed));
            expected.push((
                layers,
                agents,
                RoleObservation {
                    enabled: agents != Agents::Disabled,
                    roles: fixture.ordinary_roles(layers),
                    calls,
                },
            ));
        }
    }
    assert_eq!(actual.len(), 6);
    assert_eq!(actual, expected);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Versions {
    override_value: Option<MultiAgentVersion>,
    features: MultiAgentVersion,
    model: [MultiAgentVersion; 4],
}

fn versions(config: &Config) -> Versions {
    Versions {
        override_value: config.multi_agent_version_override(),
        features: config.multi_agent_version_from_features(),
        model: [
            None,
            Some(MultiAgentVersion::Disabled),
            Some(MultiAgentVersion::V1),
            Some(MultiAgentVersion::V2),
        ]
        .map(|metadata| config.multi_agent_version_for_model(metadata)),
    }
}

fn disabled_versions() -> Versions {
    Versions {
        override_value: Some(MultiAgentVersion::Disabled),
        features: MultiAgentVersion::Disabled,
        model: [MultiAgentVersion::Disabled; 4],
    }
}

#[cfg(feature = "covenant")]
#[tokio::test]
async fn covenant_roles_metadata_stays_disabled_after_mutation() -> std::io::Result<()> {
    let fixture = Fixture::new()?;
    let (mut config, _) = fixture.load(Layers::User, Agents::Enabled).await?;
    let permissions = security(&config);
    let mut actual = vec![("initial", versions(&config))];
    for enabled in [true, false] {
        config.agents_enabled = enabled;
        for (name, feature, requested) in [
            ("collab_on", Feature::Collab, true),
            ("v2_on", Feature::MultiAgentV2, true),
            ("v2_off", Feature::MultiAgentV2, false),
            ("collab_off", Feature::Collab, false),
        ] {
            config.features.set_enabled(feature, requested).unwrap();
            actual.push((name, versions(&config)));
        }
    }
    let mut cloned = config.clone();
    cloned.agents_enabled = true;
    cloned.features.enable(Feature::MultiAgentV2).unwrap();
    actual.push(("clone", versions(&cloned)));
    actual.push(("original", versions(&config)));
    assert_eq!(
        [security(&config), security(&cloned)],
        [permissions.clone(), permissions]
    );
    let expected: Vec<_> = actual
        .iter()
        .map(|(name, _)| (*name, disabled_versions()))
        .collect();
    assert_eq!(actual.len(), 11);
    assert_eq!(actual, expected);
    Ok(())
}

#[cfg(not(feature = "covenant"))]
#[tokio::test]
async fn covenant_roles_metadata_ordinary_preserves_precedence() -> std::io::Result<()> {
    use MultiAgentVersion::{Disabled, V1, V2};
    let fixture = Fixture::new()?;
    let (mut config, _) = fixture.load(Layers::User, Agents::Enabled).await?;
    let permissions = security(&config);
    config.features.disable(Feature::Collab).unwrap();
    config.features.disable(Feature::MultiAgentV2).unwrap();
    let mut actual = vec![("enabled", versions(&config))];
    config.agents_enabled = false;
    actual.push(("disabled", versions(&config)));
    config.features.enable(Feature::MultiAgentV2).unwrap();
    actual.push(("v2", versions(&config)));
    config.features.disable(Feature::MultiAgentV2).unwrap();
    config.features.enable(Feature::Collab).unwrap();
    config.agents_enabled = true;
    actual.push(("v1", versions(&config)));
    assert_eq!(security(&config), permissions);
    assert_eq!(
        actual,
        vec![
            (
                "enabled",
                Versions {
                    override_value: None,
                    features: Disabled,
                    model: [Disabled, Disabled, V1, V2]
                }
            ),
            ("disabled", disabled_versions()),
            (
                "v2",
                Versions {
                    override_value: Some(V2),
                    features: V2,
                    model: [V2; 4]
                }
            ),
            (
                "v1",
                Versions {
                    override_value: None,
                    features: V1,
                    model: [V1, Disabled, V1, V2]
                }
            ),
        ]
    );
    Ok(())
}

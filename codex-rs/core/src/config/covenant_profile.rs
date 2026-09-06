use std::collections::BTreeMap;
use std::io;

use codex_config::FeatureRequirementsToml;
use codex_features::Feature;

use super::managed_features::feature_for_requirement_key;

pub(super) const FEATURE_PINS: &[(Feature, bool)] = &[
    (Feature::ShellTool, true),
    (Feature::UnifiedExec, false),
    (Feature::ShellZshFork, false),
    (Feature::UnifiedExecZshFork, false),
    (Feature::ShellSnapshot, false),
    (Feature::ShellSnapshotV2, false),
    (Feature::CodeMode, false),
    (Feature::CodeModeHost, false),
    (Feature::CodeModePrewarm, false),
    (Feature::CodeModeInterrupt, false),
    (Feature::CodeModeOnly, false),
    (Feature::Collab, false),
    (Feature::MultiAgentV2, false),
    (Feature::Goals, false),
    (Feature::MemoryTool, false),
    (Feature::ExternalAgentMemoryImport, false),
    (Feature::Chronicle, false),
    (Feature::Apps, false),
    (Feature::EnableMcpApps, false),
    (Feature::Mcp20260728, false),
    (Feature::McpOAuthRefreshCoordination, false),
    (Feature::DeferredToolWorldState, false),
    (Feature::NonPrefixedMcpToolNames, false),
    (Feature::ToolSuggest, false),
    (Feature::RecommendedPlugins, false),
    (Feature::Plugins, false),
    (Feature::ExecutorCapabilityDiscovery, false),
    (Feature::RemotePlugin, false),
    (Feature::PluginSharing, false),
    (Feature::SkillMcpDependencyInstall, false),
    (Feature::SkillSearch, false),
    (Feature::WorkspaceDependencies, false),
    (Feature::ViewImage, false),
    (Feature::SleepTool, false),
    (Feature::RequestPermissionsTool, false),
    (Feature::WebSearchRequest, false),
    (Feature::WebSearchCached, false),
    (Feature::StandaloneWebSearch, false),
    (Feature::ImageGeneration, false),
    (Feature::Artifact, false),
    (Feature::DefaultModeRequestUserInput, false),
    (Feature::InAppBrowser, false),
    (Feature::BrowserUse, false),
    (Feature::BrowserUseFullCdpAccess, false),
    (Feature::BrowserUseExternal, false),
    (Feature::ComputerUse, false),
    (Feature::RealtimeConversation, false),
    (Feature::CodexHooks, false),
    (Feature::GuardianApproval, false),
    (Feature::GuardianReuseParentCompaction, false),
    (Feature::GuardianEnhancedNodeReplTranscripts, false),
    (Feature::GuardianNodeReplTranscriptImages, false),
    (Feature::GuardianV2, false),
    (Feature::GuardianExt, false),
    (Feature::RespectSystemProxy, false),
    (Feature::StepModelSwitching, false),
];

pub(super) fn validate_feature_requirements(
    requirements: &FeatureRequirementsToml,
) -> io::Result<()> {
    let refusal = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Covenant profile rejected feature requirements",
        )
    };
    let mut seen = BTreeMap::new();
    for (key, enabled) in &requirements.entries {
        let feature = feature_for_requirement_key(key).ok_or_else(refusal)?;
        if seen
            .insert(feature, *enabled)
            .is_some_and(|previous| previous != *enabled)
            || FEATURE_PINS
                .iter()
                .any(|(pinned, required)| *pinned == feature && required != enabled)
        {
            return Err(refusal());
        }
    }
    Ok(())
}

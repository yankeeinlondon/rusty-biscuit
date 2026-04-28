use std::path::PathBuf;

use crate::provider::Provider;
/// Canonical capability model for a supported agentic CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCapabilities {
    pub meta: AgentMeta,
    pub docs: AgentDocs,
    pub config: ConfigCapabilities,

    pub runtime: RuntimeCapabilities,
    pub skills: SkillsCapabilities,
    pub slash_commands: SlashCommandCapabilities,
    pub subagents: SubagentCapabilities,
    pub scripts: ScriptCapabilities,

    pub confidence: ConfidenceProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMeta {
    pub id: Provider,
    pub display_name: &'static str,
    pub binary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDocs {
    pub homepage: Option<&'static str>,
    pub docs: Option<&'static str>,
    pub skills_docs: Option<&'static str>,
    pub slash_docs: Option<&'static str>,
    pub subagents_docs: Option<&'static str>,
    pub scripts_docs: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCapabilities {
    pub user_files: Vec<PathBuf>,
    pub project_files: Vec<PathBuf>,
    pub local_files: Vec<PathBuf>,
    pub format: Option<ConfigFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Jsonc,
    Toml,
    Yaml,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub model: ModelCapabilities,
    pub non_interactive: NonInteractiveCapabilities,
    pub system_prompt: SystemPromptCapabilities,
    pub permissions: PermissionCapabilities,
    pub reasoning: ReasoningCapabilities,
    pub logging: LoggingCapabilities,
    pub billing: BillingCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCapabilities {
    pub cli_flags: Vec<&'static str>,
    pub session_switch_commands: Vec<&'static str>,
    pub aliases: Vec<&'static str>,
    pub precedence_order: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonInteractiveCapabilities {
    pub supported: bool,
    pub entrypoints: Vec<&'static str>,
    pub stdin_supported: bool,
    pub output_formats: Vec<&'static str>,
    pub structured_output_supported: bool,
    pub resume_supported: bool,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptCapabilities {
    pub supplement_sources: Vec<&'static str>,
    pub full_replacement_supported: bool,
    pub replacement_mechanisms: Vec<&'static str>,
    pub memory_files: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionCapabilities {
    pub modes: Vec<&'static str>,
    pub yolo_equivalent: Option<&'static str>,
    pub sandbox_modes: Vec<&'static str>,
    pub tool_allowlist_controls: Vec<&'static str>,
    pub tool_denylist_controls: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningCapabilities {
    pub style: ReasoningStyle,
    pub levels_or_controls: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningStyle {
    NamedLevels,
    NumericBudget,
    BinaryToggle,
    ProviderSpecific,
    NotDocumented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingCapabilities {
    pub session_locations: Vec<&'static str>,
    pub log_locations: Vec<&'static str>,
    pub debug_controls: Vec<&'static str>,
    pub telemetry_controls: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingCapabilities {
    pub models: Vec<BillingModel>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingModel {
    Subscription,
    PerToken,
    PrepaidCredits,
    ProviderOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsCapabilities {
    pub status: CapabilityStatus,
    pub activation: ActivationStyle,
    pub user_consent_required: bool,
    pub paths: PathDiscovery,
    pub reads_claude_dirs: bool,
    pub reads_agents_dirs: bool,
    pub frontmatter: FrontmatterContract,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandCapabilities {
    pub status: CapabilityStatus,
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_format: CommandFormat,
    pub paths: PathDiscovery,
    pub supports_subdirectory_namespacing: bool,
    pub supports_hot_reload: bool,
    pub reads_claude_dirs: bool,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentCapabilities {
    pub status: CapabilityStatus,
    pub definition_format: AgentDefinitionFormat,
    pub paths: PathDiscovery,
    pub enablement_controls: Vec<&'static str>,
    pub invocation_style: InvocationStyle,
    pub context_isolation: bool,
    pub parallel_supported: bool,
    pub nesting_supported: Option<bool>,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCapabilities {
    pub status: CapabilityStatus,
    pub dedicated_script_dirs: Vec<PathBuf>,
    pub tool_dirs: Vec<PathBuf>,
    pub plugin_dirs: Vec<PathBuf>,
    pub skill_local_scripts_supported: bool,
    pub hook_or_notify_mechanisms: Vec<&'static str>,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    Supported,
    Partial,
    Deprecated,
    Experimental,
    NotSupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationStyle {
    AutoMatch,
    ToolInvocation,
    ExplicitCommand,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationStyle {
    ToolDelegation,
    OrchestratorMode,
    AutomaticSpawn,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentDefinitionFormat {
    MarkdownFrontmatter,
    Yaml,
    Toml,
    ModesYaml,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFormat {
    Markdown,
    Toml,
    JsonConfig,
    RecipeYaml,
    Mixed,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterContract {
    pub required_fields: Vec<&'static str>,
    pub optional_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathDiscovery {
    pub user_paths: Vec<PathBuf>,
    pub project_paths: Vec<PathBuf>,
    pub admin_paths: Vec<PathBuf>,
    pub extension_paths: Vec<PathBuf>,
    pub precedence_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidenceProfile {
    pub overall: Confidence,
    pub by_area: Vec<AreaConfidence>,
    pub gaps: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaConfidence {
    pub area: &'static str,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Shared trait implemented by every supported agent capability descriptor.
pub trait Agent: Send + Sync + std::fmt::Debug {
    fn id(&self) -> Provider;
    fn capabilities(&self) -> &AgentCapabilities;

    fn meta(&self) -> &AgentMeta {
        &self.capabilities().meta
    }

    fn supports_skills(&self) -> bool {
        matches!(
            self.capabilities().skills.status,
            CapabilityStatus::Supported
                | CapabilityStatus::Experimental
                | CapabilityStatus::Partial
        )
    }

    fn supports_custom_slash_commands(&self) -> bool {
        self.capabilities().slash_commands.custom_supported
    }

    fn supports_subagents(&self) -> bool {
        matches!(
            self.capabilities().subagents.status,
            CapabilityStatus::Supported
                | CapabilityStatus::Experimental
                | CapabilityStatus::Partial
        )
    }

    fn skill_paths(&self) -> &PathDiscovery {
        &self.capabilities().skills.paths
    }

    fn slash_paths(&self) -> &PathDiscovery {
        &self.capabilities().slash_commands.paths
    }

    fn subagent_paths(&self) -> &PathDiscovery {
        &self.capabilities().subagents.paths
    }

    fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        let caps = self.capabilities();

        if caps.meta.binary.is_empty() {
            issues.push("binary is empty".to_string());
        }

        if caps.skills.status == CapabilityStatus::Supported
            && caps.skills.paths.user_paths.is_empty()
            && caps.skills.paths.project_paths.is_empty()
        {
            issues.push("skills marked supported but no discovery paths configured".to_string());
        }

        if caps.subagents.status == CapabilityStatus::Supported
            && caps.subagents.definition_format == AgentDefinitionFormat::Unknown
        {
            issues.push("subagents supported but definition format unknown".to_string());
        }

        issues
    }
}

pub(crate) fn path(path: &str) -> PathBuf {
    PathBuf::from(path)
}

pub(crate) fn paths(
    user: &[&str],
    project: &[&str],
    admin: &[&str],
    extension: &[&str],
    precedence: &[&'static str],
) -> PathDiscovery {
    PathDiscovery {
        user_paths: user.iter().map(|p| path(p)).collect(),
        project_paths: project.iter().map(|p| path(p)).collect(),
        admin_paths: admin.iter().map(|p| path(p)).collect(),
        extension_paths: extension.iter().map(|p| path(p)).collect(),
        precedence_rules: precedence.to_vec(),
    }
}

pub(crate) fn frontmatter(
    required: &[&'static str],
    optional: &[&'static str],
) -> FrontmatterContract {
    FrontmatterContract {
        required_fields: required.to_vec(),
        optional_fields: optional.to_vec(),
    }
}

pub(crate) fn area_confidence(area: &'static str, confidence: Confidence) -> AreaConfidence {
    AreaConfidence { area, confidence }
}

pub(crate) fn path_vec(paths: &[&str]) -> Vec<PathBuf> {
    paths.iter().map(|p| path(p)).collect()
}

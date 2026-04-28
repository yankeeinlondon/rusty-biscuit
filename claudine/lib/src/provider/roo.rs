//! Roo Code provider definition.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
use super::identity::Provider;
use crate::agents::{
    ActivationStyle, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, Confidence,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, InvocationStyle, LoggingCapabilities,
    ModelCapabilities, NonInteractiveCapabilities, PermissionCapabilities, ReasoningCapabilities,
    ReasoningStyle, RuntimeCapabilities, ScriptCapabilities, SkillsCapabilities,
    SlashCommandCapabilities, SubagentCapabilities, SystemPromptCapabilities,
};
use crate::agents::model::{area_confidence, frontmatter, path_vec, paths};
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport,
    SkillFrontmatter,
};

#[derive(Debug)]
pub(super) struct RooProvider;

pub(super) static ROO_PROVIDER: RooProvider = RooProvider;

impl ProviderBehavior for RooProvider {}
impl McpBehavior for RooProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for RooProvider {}
impl ConfiguratorBehavior for RooProvider {}

static ROO_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_roo_agent_capabilities);

static ROO_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_roo_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &ROO_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &ROO_RESOURCE_SUPPORT
}

pub(super) static ROO_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::RooCode,
    display_name: "Roo Code",
    slug: "roo_code",
    binary: "roo",
    agent_offset: ".roo",
    cli_aliases: &["roo", "roocode", "roo_code", "roo-code"],
    docs_url: "https://github.com/RooVetGit/Roo-Code",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Roo,
    supports_skills: true,
    behavior: &ROO_PROVIDER,
    mcp: &ROO_PROVIDER,
    adapter: &ROO_PROVIDER,
    configurator: &ROO_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
};

const ROO_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint", "mode"],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["slug", "name", "roleDefinition", "groups"],
    &["description", "whenToUse", "customInstructions"],
    "claudine/docs/cross-referencing/roo-code.md",
);

fn build_roo_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::RooCode,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".roo/skills", ".roo/skills")
            .with_note("Roo skills are compatible with markdown skill bundles")
            .with_properties(ROO_SKILL_SCHEMA),
        commands: ResourceSupport::full(ResourceFormat::Markdown, ".roo/commands", ".roo/commands")
            .with_properties(ROO_COMMAND_SCHEMA),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".roomodes",
            ".roo/custom_modes.yaml",
        )
        .with_note("Mode definitions via .roomodes/custom_modes.yaml")
        .with_properties(ROO_AGENT_SCHEMA),
        scripts: ResourceSupport::full(ResourceFormat::Executable, ".roo/scripts", ".roo/scripts"),
        skill_frontmatter: SkillFrontmatter::standard().with_allowed_tools(),
    }
}

fn build_roo_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::RooCode,
            display_name: "Roo Code",
            binary: "roo",
        },
        docs: AgentDocs {
            homepage: Some("https://roocode.com/"),
            docs: Some("https://docs.roocode.com/"),
            skills_docs: Some("https://docs.roocode.com/"),
            slash_docs: Some("https://docs.roocode.com/"),
            subagents_docs: Some("https://docs.roocode.com/"),
            scripts_docs: Some("https://docs.roocode.com/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.roo/custom_modes.yaml", "~/.roo/custom_modes.json"]),
            project_files: path_vec(&[
                ".roomodes",
                ".roo/custom_modes.yaml",
                ".roo/custom_modes.json",
            ]),
            local_files: vec![],
            format: Some(ConfigFormat::Mixed),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["--provider", "--model", "--reasoning-effort", "--api-key"],
                session_switch_commands: vec![],
                aliases: vec![],
                precedence_order: vec![
                    "CLI flags",
                    "provider-specific environment variables",
                    "roo auth stored credentials",
                    "roo defaults",
                ],
                notes: vec![
                    "VS Code extension supports profile-based model settings",
                    "CLI does not read a dedicated default model config file",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "roo --print <prompt>",
                    "roo --print --prompt-file <file>",
                    "roo --print --stdin-prompt-stream",
                    "roo --oneshot <prompt>",
                    "roo --ephemeral",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: false,
                limitations: vec![
                    "CLI defaults to auto-approve unless --require-approval is provided",
                    "prompt is required for print mode",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "rules directories",
                    ".roorules/.roorules-{mode}",
                    "AGENTS.md/AGENT.md",
                    ".rooignore",
                ],
                full_replacement_supported: true,
                replacement_mechanisms: vec![".roo/system-prompt-{mode-slug}"],
                memory_files: vec!["AGENTS.md", "AGENT.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec![
                    "auto-approve (default)",
                    "manual approval (--require-approval)",
                ],
                yolo_equivalent: Some("default auto-approve behavior"),
                sandbox_modes: vec![],
                tool_allowlist_controls: vec!["VS Code auto-approve categories"],
                tool_denylist_controls: vec!["VS Code category toggles"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NamedLevels,
                levels_or_controls: vec![
                    "unspecified",
                    "disabled",
                    "none",
                    "minimal",
                    "low",
                    "medium",
                    "high",
                    "xhigh",
                ],
                notes: vec!["Configured through --reasoning-effort on CLI"],
            },
            logging: LoggingCapabilities {
                session_locations: vec![
                    "VS Code globalStorage/rooveterinaryinc.roo-cline/",
                    "custom roo-cline storage path",
                ],
                log_locations: vec!["@roo-code/core debug-log output"],
                debug_controls: vec!["--debug"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::PrepaidCredits, BillingModel::PerToken],
                notes: vec![
                    "Roo Cloud uses prepaid credits",
                    "BYOK providers bill per token",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &["~/.roo/skills", "~/.roo/skills-<modeSlug>"],
                &[".roo/skills", ".roo/skills-<modeSlug>"],
                &[],
                &[],
                &["project mode-specific > project generic > user mode-specific > user generic"],
            ),
            reads_claude_dirs: false,
            reads_agents_dirs: false,
            frontmatter: frontmatter(&["name", "description"], &[]),
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec!["Symlinks are supported for skill directories"],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Markdown,
            paths: paths(
                &["~/.roo/commands"],
                &[".roo/commands"],
                &[],
                &[],
                &["project commands override user commands"],
            ),
            supports_subdirectory_namespacing: false,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec![
                "Command frontmatter includes description, argument-hint, and mode",
                "Programmatic run_slash_command tool is experimental",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Partial,
            definition_format: AgentDefinitionFormat::ModesYaml,
            paths: paths(
                &["~/.roo/custom_modes.yaml", "~/.roo/custom_modes.json"],
                &[
                    ".roomodes",
                    ".roo/custom_modes.yaml",
                    ".roo/custom_modes.json",
                ],
                &[],
                &[],
                &["mode files define orchestrated agent behavior instead of agent directories"],
            ),
            enablement_controls: vec!["new_task tool", "mode selection"],
            invocation_style: InvocationStyle::OrchestratorMode,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: None,
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec![
                "No directory-based .agents definition model",
                "Boomerang task orchestration is mode-centric",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: path_vec(&[".roo/tools", "~/.roo/tools"]),
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["MCP tools", "run_slash_command", "custom tools"],
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec!["Custom tools are the primary script-like extension surface"],
        },
        confidence: ConfidenceProfile {
            overall: Confidence::High,
            by_area: vec![
                area_confidence("runtime", Confidence::High),
                area_confidence("skills", Confidence::High),
                area_confidence("slash_commands", Confidence::High),
                area_confidence("subagents", Confidence::High),
                area_confidence("scripts", Confidence::High),
            ],
            gaps: vec![],
        },
    }
}

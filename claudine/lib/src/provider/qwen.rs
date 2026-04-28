//! Qwen Code provider definition.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use crate::events::{AgenticEvent, EventSupportLevel};
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
pub(super) struct QwenProvider;

pub(super) static QWEN_PROVIDER: QwenProvider = QwenProvider;

impl ProviderBehavior for QwenProvider {}
impl McpBehavior for QwenProvider {}
impl AdapterBehavior for QwenProvider {}
impl ConfiguratorBehavior for QwenProvider {}

static QWEN_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_qwen_agent_capabilities);

static QWEN_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_qwen_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &QWEN_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &QWEN_RESOURCE_SUPPORT
}

pub(super) static QWEN_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::QwenCode,
    display_name: "Qwen Code",
    slug: "qwen_code",
    binary: "qwen",
    agent_offset: ".qwen",
    cli_aliases: &["qwen", "qwencode", "qwen_code", "qwen-code"],
    docs_url: "https://qwenlm.github.io/qwen-code-docs/",
    usage_dashboard_url: Some("https://bailian.console.aliyun.com/"),
    sniff_binding: AiCli::QwenCli,
    supports_skills: true,
    event_mapping: &QWEN_EVENT_MAPPING,
    behavior: &QWEN_PROVIDER,
    mcp: &QWEN_PROVIDER,
    adapter: &QWEN_PROVIDER,
    configurator: &QWEN_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
};

pub(super) static QWEN_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::NonHook,
            native_name: "CanUseTool",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::NonHook,
            native_name: "result",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::NonHook,
            native_name: "result",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeModel,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::NonHook,
            native_name: "assistant",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::NonHook,
            native_name: "system",
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const QWEN_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/qwen-cli.md",
);
const QWEN_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description"],
    "claudine/docs/cross-referencing/qwen-cli.md",
);
const QWEN_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["tools", "color"],
    "claudine/docs/cross-referencing/qwen-cli.md",
);

fn build_qwen_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::QwenCode,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".qwen/skills", ".qwen/skills")
            .with_note("Skills support is experimental")
            .with_properties(QWEN_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".qwen/commands",
            ".qwen/commands",
        )
        .with_note("Markdown custom commands; TOML remains deprecated fallback")
        .with_properties(QWEN_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".qwen/agents", ".qwen/agents")
            .with_properties(QWEN_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".qwen/scripts",
            ".qwen/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::standard().with_allowed_tools(),
    }
}

fn build_qwen_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::QwenCode,
            display_name: "Qwen Code CLI",
            binary: "qwen",
        },
        docs: AgentDocs {
            homepage: Some("https://qwen.ai/"),
            docs: Some("https://qwenlm.github.io/qwen-code-docs/"),
            skills_docs: Some("https://qwenlm.github.io/qwen-code-docs/en/users/skills"),
            slash_docs: Some("https://qwenlm.github.io/qwen-code-docs/en/users/commands"),
            subagents_docs: Some("https://qwenlm.github.io/qwen-code-docs/en/users/agents"),
            scripts_docs: Some("https://qwenlm.github.io/qwen-code-docs/en/users/skills"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.qwen/settings.json"]),
            project_files: path_vec(&[".qwen/settings.json"]),
            local_files: vec![],
            format: Some(ConfigFormat::Json),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["-m", "--model", "--auth-type"],
                session_switch_commands: vec!["/model"],
                aliases: vec!["qwen3-coder-plus"],
                precedence_order: vec![
                    "--model",
                    "OPENAI_MODEL",
                    "model.name in settings.json",
                    "auth default model",
                ],
                notes: vec![
                    "Any OpenAI-compatible provider can be configured in modelProviders",
                    "Auth type controls default provider protocol",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "qwen <prompt>",
                    "qwen -p <prompt>",
                    "qwen -i <prompt>",
                    "stdin piping",
                    "qwen --continue",
                    "qwen --resume <id>",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec![
                    "qwen-oauth is not suitable for headless CI environments",
                    "-p is deprecated in favor of positional prompts",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "QWEN.md hierarchy",
                    "@path markdown imports",
                    "/memory refresh",
                ],
                full_replacement_supported: false,
                replacement_mechanisms: vec![],
                memory_files: vec!["~/.qwen/QWEN.md", "QWEN.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec!["plan", "default", "auto-edit", "yolo"],
                yolo_equivalent: Some("--yolo"),
                sandbox_modes: vec!["--sandbox"],
                tool_allowlist_controls: vec!["tools.allowed", "--allowed-tools", "tools.core"],
                tool_denylist_controls: vec!["tools.exclude", "--exclude-tools"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NumericBudget,
                levels_or_controls: vec![
                    "enable_thinking",
                    "thinking_budget",
                    "/think",
                    "/no_think",
                ],
                notes: vec!["No dedicated CLI reasoning-effort flag is documented"],
            },
            logging: LoggingCapabilities {
                session_locations: vec!["~/.qwen/projects/<sanitized-cwd>/chats/"],
                log_locations: vec!["logs/openai"],
                debug_controls: vec!["--debug"],
                telemetry_controls: vec![
                    "telemetry.* in settings.json",
                    "privacy.usageStatisticsEnabled",
                ],
            },
            billing: BillingCapabilities {
                models: vec![
                    BillingModel::PrepaidCredits,
                    BillingModel::Subscription,
                    BillingModel::PerToken,
                ],
                notes: vec![
                    "Qwen OAuth free quotas exist",
                    "Bailian coding plan is subscription-style",
                    "Third-party providers are billed per token",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &["~/.qwen/skills"],
                &[".qwen/skills"],
                &[],
                &["<extension>/skills"],
                &["project > user > extension"],
            ),
            reads_claude_dirs: false,
            reads_agents_dirs: false,
            frontmatter: frontmatter(&["name", "description"], &[]),
            docs_url: Some("https://qwenlm.github.io/qwen-code-docs/"),
            notes: vec![
                "Skills started as an experimental feature and are now documented",
                "Explicit invocation is available through /skills <name>",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Mixed,
            paths: paths(
                &["~/.qwen/commands"],
                &[".qwen/commands"],
                &[],
                &[],
                &["project command files override user command files"],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: true,
            reads_claude_dirs: false,
            docs_url: Some("https://qwenlm.github.io/qwen-code-docs/"),
            notes: vec!["Markdown command files are preferred; TOML command files are deprecated"],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::MarkdownFrontmatter,
            paths: paths(
                &["~/.qwen/agents"],
                &[".qwen/agents"],
                &[],
                &["<extension>/agents"],
                &["project > user > extension"],
            ),
            enablement_controls: vec!["/agents create", "/agents manage"],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://qwenlm.github.io/qwen-code-docs/"),
            notes: vec![
                "Subagent definitions use markdown with YAML frontmatter",
                "Nested delegation support is not documented",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: vec![],
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["MCP servers", "skill-local scripts"],
            docs_url: Some("https://qwenlm.github.io/qwen-code-docs/"),
            notes: vec!["Scripts are expected under <skill>/scripts"],
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

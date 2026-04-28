//! Claude Code provider definition.

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

/// Zero-sized provider behavior implementor used as the trait-object value
/// for all four behavior trait fields on `CLAUDE_INFO`.
#[derive(Debug)]
pub(super) struct ClaudeProvider;

pub(super) static CLAUDE_PROVIDER: ClaudeProvider = ClaudeProvider;

impl ProviderBehavior for ClaudeProvider {}
impl McpBehavior for ClaudeProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for ClaudeProvider {}
impl ConfiguratorBehavior for ClaudeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }
}

static CLAUDE_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_claude_agent_capabilities);

static CLAUDE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_claude_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &CLAUDE_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &CLAUDE_RESOURCE_SUPPORT
}

pub(super) static CLAUDE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Claude,
    display_name: "Claude",
    slug: "claude",
    binary: "claude",
    agent_offset: ".claude",
    cli_aliases: &["claude"],
    docs_url: "https://docs.anthropic.com/en/docs/claude-code",
    usage_dashboard_url: Some("https://console.anthropic.com/settings/billing"),
    sniff_binding: AiCli::Claude,
    supports_skills: true,
    event_mapping: &CLAUDE_EVENT_MAPPING,
    behavior: &CLAUDE_PROVIDER,
    mcp: &CLAUDE_PROVIDER,
    adapter: &CLAUDE_PROVIDER,
    configurator: &CLAUDE_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
};

pub(super) static CLAUDE_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::Hook,
            native_name: "SessionStart",
            parse_aliases: &["SessionStart"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::Hook,
            native_name: "SessionEnd",
            parse_aliases: &["SessionEnd"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::Hook,
            native_name: "UserPromptSubmit",
            parse_aliases: &["UserPromptSubmit"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::Hook,
            native_name: "PreToolUse",
            parse_aliases: &["PreToolUse"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Hook,
            native_name: "PostToolUse",
            parse_aliases: &["PostToolUse"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::Hook,
            native_name: "PostToolUseFailure",
            parse_aliases: &["PostToolUseFailure"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::Hook,
            native_name: "PermissionRequest",
            parse_aliases: &["PermissionRequest"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::Hook,
            native_name: "PreToolUse",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook,
            native_name: "Stop",
            parse_aliases: &["Stop", "TeammateIdle", "TaskCompleted"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::Hook,
            native_name: "SubagentStart",
            parse_aliases: &["SubagentStart"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::Hook,
            native_name: "SubagentStop",
            parse_aliases: &["SubagentStop"],
            registration_target: true,
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
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::Hook,
            native_name: "PreCompact",
            parse_aliases: &["PreCompact"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::Hook,
            native_name: "Notification",
            parse_aliases: &["Notification"],
            registration_target: true,
        },
    ],
};

const CLAUDE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "argument-hint",
        "disable-model-invocation",
        "user-invocable",
        "allowed-tools",
        "model",
        "context",
        "agent",
        "hooks",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);
const CLAUDE_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "argument-hint",
        "disable-model-invocation",
        "user-invocable",
        "allowed-tools",
        "model",
        "context",
        "agent",
        "hooks",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);
const CLAUDE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[
        "tools",
        "disallowedTools",
        "model",
        "permissionMode",
        "maxTurns",
        "skills",
        "mcpServers",
        "hooks",
        "memory",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);

fn build_claude_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Claude,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".claude/skills", ".claude/skills")
            .with_properties(CLAUDE_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".claude/commands",
            ".claude/commands",
        )
        .with_properties(CLAUDE_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".claude/agents", ".claude/agents")
            .with_properties(CLAUDE_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts are stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::full(),
    }
}

fn build_claude_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::Claude,
            display_name: "Claude Code",
            binary: "claude",
        },
        docs: AgentDocs {
            homepage: Some("https://claude.ai/code"),
            docs: Some("https://code.claude.com/docs/en"),
            skills_docs: Some("https://code.claude.com/docs/en/skills"),
            slash_docs: Some("https://code.claude.com/docs/en/slash-commands"),
            subagents_docs: Some("https://code.claude.com/docs/en/sub-agents"),
            scripts_docs: Some("https://code.claude.com/docs/en/hooks"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.claude/settings.json", "~/.claude.json"]),
            project_files: path_vec(&[".claude/settings.json", ".mcp.json"]),
            local_files: path_vec(&[".claude/settings.local.json"]),
            format: Some(ConfigFormat::Json),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["--model"],
                session_switch_commands: vec!["/model"],
                aliases: vec![
                    "default",
                    "sonnet",
                    "opus",
                    "haiku",
                    "sonnet[1m]",
                    "opusplan",
                ],
                precedence_order: vec![
                    "/model in session",
                    "--model",
                    "ANTHROPIC_MODEL",
                    "model in settings.json",
                ],
                notes: vec![
                    "availableModels can restrict interactive selection",
                    "subagent model can be overridden with CLAUDE_CODE_SUBAGENT_MODEL",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "claude --print",
                    "claude -c --print",
                    "claude -r <session> --print",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec![
                    "print-mode specific flags require --print",
                    "some full-replacement mechanisms differ between interactive and print modes",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "--append-system-prompt",
                    "--append-system-prompt-file",
                    "CLAUDE.md hierarchy",
                ],
                full_replacement_supported: true,
                replacement_mechanisms: vec!["--system-prompt", "--system-prompt-file"],
                memory_files: vec![
                    "~/.claude/CLAUDE.md",
                    "CLAUDE.md",
                    ".claude/CLAUDE.md",
                    ".claude/CLAUDE.local.md",
                ],
            },
            permissions: PermissionCapabilities {
                modes: vec![
                    "default",
                    "acceptEdits",
                    "plan",
                    "dontAsk",
                    "bypassPermissions",
                ],
                yolo_equivalent: Some("--dangerously-skip-permissions"),
                sandbox_modes: vec![],
                tool_allowlist_controls: vec![
                    "settings.json permissions",
                    "skill frontmatter allowed-tools",
                    "subagent frontmatter tools",
                ],
                tool_denylist_controls: vec!["subagent disallowedTools", "dontAsk mode auto-deny"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NamedLevels,
                levels_or_controls: vec!["low", "medium", "high"],
                notes: vec!["Extended thinking budget is model-dependent"],
            },
            logging: LoggingCapabilities {
                session_locations: vec![
                    "~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl",
                ],
                log_locations: vec!["~/.claude/projects/"],
                debug_controls: vec!["--verbose"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::Subscription, BillingModel::PerToken],
                notes: vec![
                    "Subscription and pay-as-you-go API billing are both supported",
                    "Alternative backends include Bedrock, Vertex, and Foundry",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &["~/.claude/skills"],
                &[".claude/skills"],
                &["managed settings (enterprise skills)"],
                &["<plugin>/skills"],
                &[
                    "enterprise > personal > project for name collisions",
                    "plugin skills are namespaced",
                ],
            ),
            reads_claude_dirs: true,
            reads_agents_dirs: false,
            frontmatter: frontmatter(
                &[],
                &[
                    "name",
                    "description",
                    "argument-hint",
                    "disable-model-invocation",
                    "user-invocable",
                    "allowed-tools",
                    "model",
                    "context",
                    "agent",
                    "hooks",
                ],
            ),
            docs_url: Some("https://code.claude.com/docs/en/skills"),
            notes: vec![
                "Skill frontmatter has recommended fields, not hard-required fields",
                "Skills are the preferred replacement for legacy custom slash commands",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Markdown,
            paths: paths(
                &["~/.claude/commands"],
                &[".claude/commands"],
                &[],
                &[],
                &["legacy command files remain supported for backward compatibility"],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: false,
            reads_claude_dirs: true,
            docs_url: Some("https://code.claude.com/docs/en/slash-commands"),
            notes: vec![
                "Custom command behavior is now primarily modeled through skills",
                "Legacy markdown commands still map to slash command names",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::MarkdownFrontmatter,
            paths: paths(
                &["~/.claude/agents"],
                &[".claude/agents"],
                &[],
                &["<plugin>/agents"],
                &[
                    "--agents session payload has highest precedence",
                    "project agents override user and plugin agents",
                ],
            ),
            enablement_controls: vec!["/agents", "--agents <json>"],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://code.claude.com/docs/en/sub-agents"),
            notes: vec![
                "Task-style subagent delegation is stable",
                "Subagents cannot recursively spawn additional subagents",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: path_vec(&["~/.claude/hooks", ".claude/hooks"]),
            tool_dirs: vec![],
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["settings.json hooks", "skill/agent scoped hooks"],
            docs_url: Some("https://code.claude.com/docs/en/hooks"),
            notes: vec![
                "No single global scripts convention; hook commands are the primary automation surface",
                "Skill-local scripts are commonly referenced from SKILL.md",
            ],
        },
        confidence: ConfidenceProfile {
            overall: Confidence::Medium,
            by_area: vec![
                area_confidence("runtime", Confidence::Medium),
                area_confidence("skills", Confidence::Medium),
                area_confidence("slash_commands", Confidence::Medium),
                area_confidence("subagents", Confidence::Medium),
                area_confidence("scripts", Confidence::Medium),
            ],
            gaps: vec!["Populate claudine/docs/cross-referencing/claude-code.md"],
        },
    }
}

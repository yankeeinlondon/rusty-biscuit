//! Claude Code provider definition.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use super::known_gap::{KnownGap, KnownGapArea};
use super::output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport,
};
use super::path_template::PathTemplate;
use super::reasoning::ReasoningSupport;
use super::system_prompt::{SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec};
use super::yolo::YoloSupport;
use crate::adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::stream::{ParserConfig, StreamProtocol};
use crate::stream::claude_semantic::ClaudeSemanticStreamParser;
use crate::stream::parser::SemanticStreamParser;
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

impl ProviderBehavior for ClaudeProvider {
    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        _config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        Box::new(ClaudeSemanticStreamParser::new(sink))
    }
}
impl McpBehavior for ClaudeProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Claude
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_claude_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_claude_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".claude.json"),
            Scope::Repo(root) => root.join(".mcp.json"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_json_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_claude_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for ClaudeProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::CLAUDE_ADAPTER
    }
}
impl ConfiguratorBehavior for ClaudeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::ClaudeConfigurator)
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
    stream_protocol: Some(StreamProtocol::StreamJson),
    event_mapping: &CLAUDE_EVENT_MAPPING,
    behavior: &CLAUDE_PROVIDER,
    mcp: &CLAUDE_PROVIDER,
    adapter: &CLAUDE_PROVIDER,
    configurator: &CLAUDE_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: CLAUDE_SESSION_LOG_PATHS,
    session_locations: CLAUDE_SESSION_LOCATIONS,
    config_paths: CLAUDE_CONFIG_PATHS,
    memory_files: CLAUDE_MEMORY_FILES,
    output_formats: CLAUDE_OUTPUT_FORMATS,
    entrypoints: CLAUDE_ENTRYPOINTS,
    system_prompt: &CLAUDE_SYSTEM_PROMPT,
    yolo: YoloSupport::DirectFlag {
        native_flag: "--dangerously-skip-permissions",
    },
    reasoning: ReasoningSupport::NamedLevels {
        flag: "thinking_effort",
        levels: &["low", "medium", "high"],
    },
    known_gaps: CLAUDE_KNOWN_GAPS,
};

const CLAUDE_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl",
)];

const CLAUDE_SESSION_LOCATIONS: &[PathTemplate] =
    &[PathTemplate::Static("~/.claude/projects/")];

const CLAUDE_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.claude/settings.json"),
    PathTemplate::Static("~/.claude.json"),
    PathTemplate::Static(".claude/settings.json"),
    PathTemplate::Static(".mcp.json"),
    PathTemplate::Static(".claude/settings.local.json"),
];

const CLAUDE_MEMORY_FILES: &[PathTemplate] = &[
    PathTemplate::Static("~/.claude/CLAUDE.md"),
    PathTemplate::Static("CLAUDE.md"),
    PathTemplate::Static(".claude/CLAUDE.md"),
    PathTemplate::Static(".claude/CLAUDE.local.md"),
];

const CLAUDE_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
    OutputFormatSupport {
        format: OutputFormat::Text,
        native_name: "text",
        cli_flag: Some("--output-format"),
        stdin_supported: true,
    },
    OutputFormatSupport {
        format: OutputFormat::Json,
        native_name: "json",
        cli_flag: Some("--output-format"),
        stdin_supported: true,
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: Some("--output-format"),
        stdin_supported: true,
    },
];

const CLAUDE_ENTRYPOINTS: &[EntrypointSpec] = &[
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--print"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["-c", "--print"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["-r", "--print"],
        mode: EntrypointMode::NonInteractive,
    },
];

static CLAUDE_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::InlineFlag {
            flag: "--append-system-prompt",
        },
        non_interactive: SystemPromptDelivery::InlineFlag {
            flag: "--append-system-prompt",
        },
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::InlineFlag {
            flag: "--system-prompt",
        },
        non_interactive: SystemPromptDelivery::InlineFlag {
            flag: "--system-prompt",
        },
    },
    memory_files: CLAUDE_MEMORY_FILES,
};

const CLAUDE_KNOWN_GAPS: &[KnownGap] = &[KnownGap {
    area: KnownGapArea::Other,
    note: "Populate claudine/docs/cross-referencing/claude-code.md",
    tracker: Some("claudine/docs/cross-referencing/claude-code.md"),
}];

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

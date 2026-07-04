//! Typed static catalog data for the Claude Code provider.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use crate::events::AgenticEvent;
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport, SkillFrontmatter,
};
use crate::provider::EventSupportLevel;
use crate::provider::OutputFormatSelector;
use crate::provider::ProviderInfo;
use crate::provider::acp::AcpSupport;
use crate::provider::cli_sensitivity::CliSensitiveAxes;
use crate::provider::event_mapping::{EventMapping, EventMappingTable};
use crate::provider::identity::Provider;
use crate::provider::known_gap::{KnownGap, KnownGapArea};
use crate::provider::model_catalog_source::ModelCatalogSource;
use crate::provider::output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport,
};
use crate::provider::path_template::PathTemplate;
use crate::provider::prompt_args::PromptArgConventions;
use crate::provider::reasoning::ReasoningSupport;
use crate::provider::system_prompt::{
    SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use crate::provider::yolo::YoloSupport;
use crate::stream::StreamProtocol;

use super::behavior::CLAUDE_PROVIDER;
use super::legacy::agent_capabilities;

static CLAUDE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_claude_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &CLAUDE_RESOURCE_SUPPORT
}

pub(in crate::provider) static CLAUDE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Claude,
    display_name: "Claude",
    slug: "claude",
    short_name: "claude",
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
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions::positional_only(),
    static_models: CLAUDE_STATIC_MODELS,
    dynamic_source: ModelCatalogSource::Static,
    model_env_vars: &["CLAUDE_MODEL", "ANTHROPIC_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes::ALL,
    repo_home_root_files: &[".claude.json"],
};

/// Static, compiled-in Anthropic model catalog.
///
/// Sourced from the generated enums in `unchained-ai/lib`. The list mirrors
/// the previous body of `model_catalog::provider_sources::anthropic_models`.
const CLAUDE_STATIC_MODELS: &[&str] = &[
    "claude-3-5-haiku-20241022",
    "claude-3-7-sonnet-20250219",
    "claude-3-haiku-20240307",
    "claude-haiku-4-5-20251001",
    "claude-opus-4-1-20250805",
    "claude-opus-4-20250514",
    "claude-opus-4-5-20251101",
    "claude-sonnet-4-20250514",
    "claude-sonnet-4-5-20250929",
];

const CLAUDE_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl",
)];

const CLAUDE_SESSION_LOCATIONS: &[PathTemplate] = &[PathTemplate::Static("~/.claude/projects/")];

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
        selector: OutputFormatSelector::FlagValue {
            flag: "--output-format",
        },
    },
    OutputFormatSupport {
        format: OutputFormat::Json,
        native_name: "json",
        cli_flag: Some("--output-format"),
        stdin_supported: true,
        selector: OutputFormatSelector::FlagValue {
            flag: "--output-format",
        },
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: Some("--output-format"),
        stdin_supported: true,
        selector: OutputFormatSelector::FlagValue {
            flag: "--output-format",
        },
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

pub(in crate::provider) static CLAUDE_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::Hook {
                native_name: "SessionStart",
            },
            parse_aliases: &["SessionStart"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::Hook {
                native_name: "SessionEnd",
            },
            parse_aliases: &["SessionEnd"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::Hook {
                native_name: "UserPromptSubmit",
            },
            parse_aliases: &["UserPromptSubmit"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::Hook {
                native_name: "PreToolUse",
            },
            parse_aliases: &["PreToolUse"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Hook {
                native_name: "PostToolUse",
            },
            parse_aliases: &["PostToolUse"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::Hook {
                native_name: "PostToolUseFailure",
            },
            parse_aliases: &["PostToolUseFailure"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::Hook {
                native_name: "PermissionRequest",
            },
            parse_aliases: &["PermissionRequest"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::Hook {
                native_name: "PreToolUse",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook {
                native_name: "Stop",
            },
            parse_aliases: &["Stop", "TeammateIdle", "TaskCompleted"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::Hook {
                native_name: "SubagentStart",
            },
            parse_aliases: &["SubagentStart"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::Hook {
                native_name: "SubagentStop",
            },
            parse_aliases: &["SubagentStop"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeModel,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::Hook {
                native_name: "PreCompact",
            },
            parse_aliases: &["PreCompact"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::Hook {
                native_name: "Notification",
            },
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

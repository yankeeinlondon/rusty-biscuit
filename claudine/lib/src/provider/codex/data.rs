//! Typed static catalog data for the Codex CLI provider.

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
use crate::provider::known_gap::KnownGap;
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

use super::behavior::CODEX_PROVIDER;
use super::legacy::agent_capabilities;

static CODEX_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_codex_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &CODEX_RESOURCE_SUPPORT
}

pub(in crate::provider) static CODEX_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Codex,
    display_name: "Codex",
    slug: "codex",
    short_name: "codex",
    binary: "codex",
    agent_offset: ".codex",
    cli_aliases: &["codex"],
    docs_url: "https://github.com/openai/codex",
    usage_dashboard_url: Some("https://platform.openai.com/usage"),
    sniff_binding: AiCli::Codex,
    supports_skills: true,
    stream_protocol: Some(StreamProtocol::Jsonl),
    event_mapping: &CODEX_EVENT_MAPPING,
    behavior: &CODEX_PROVIDER,
    mcp: &CODEX_PROVIDER,
    adapter: &CODEX_PROVIDER,
    configurator: &CODEX_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: CODEX_SESSION_LOG_PATHS,
    session_locations: CODEX_SESSION_LOCATIONS,
    config_paths: CODEX_CONFIG_PATHS,
    memory_files: CODEX_MEMORY_FILES,
    output_formats: CODEX_OUTPUT_FORMATS,
    entrypoints: CODEX_ENTRYPOINTS,
    system_prompt: &CODEX_SYSTEM_PROMPT,
    yolo: YoloSupport::DirectFlag {
        native_flag: "--dangerously-bypass-approvals-and-sandbox",
    },
    reasoning: ReasoningSupport::NamedLevels {
        flag: "model_reasoning_effort",
        levels: &["minimal", "low", "medium", "high", "xhigh"],
    },
    known_gaps: CODEX_KNOWN_GAPS,
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions::positional_after("exec"),
    static_models: CODEX_STATIC_MODELS,
    dynamic_source: ModelCatalogSource::Static,
    model_env_vars: &["CODEX_MODEL", "OPENAI_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes {
        read_path: true,
        write_path: true,
        traverse_path: true,
        execute_command: true,
        access_domain: true,
        use_mcp_server: false,
        use_mcp_tool: false,
        spawn_subagent: false,
        switch_mode: false,
        modify_provider_config: true,
    },
    repo_home_root_files: &[],
};

/// Static, compiled-in OpenAI model catalog.
///
/// Sourced from the generated enums in `unchained-ai/lib`. The list mirrors
/// the previous body of `model_catalog::provider_sources::openai_models`.
const CODEX_STATIC_MODELS: &[&str] = &[
    "gpt-3.5-turbo",
    "gpt-5-search-api",
    "gpt-5.1-codex",
    "gpt-5.2",
    "gpt-5.2-chat-latest",
    "o3",
    "o3-mini",
    "o3-mini-2025-01-31",
    "o4-mini",
];

const CODEX_SESSION_LOG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.codex/sessions/YYYY/MM/DD/<session-id>/"),
    PathTemplate::Static("~/.codex/history.jsonl"),
];

const CODEX_SESSION_LOCATIONS: &[PathTemplate] = &[
    PathTemplate::Static("~/.codex/log/codex-tui.log"),
    PathTemplate::Static("~/.codex/shell_snapshots/"),
];

const CODEX_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.codex/config.toml"),
    PathTemplate::Static(".codex/config.toml"),
];

const CODEX_MEMORY_FILES: &[PathTemplate] = &[
    PathTemplate::Static("~/.codex/AGENTS.override.md"),
    PathTemplate::Static("~/.codex/AGENTS.md"),
    PathTemplate::Static("AGENTS.md"),
];

const CODEX_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
    OutputFormatSupport {
        format: OutputFormat::Text,
        native_name: "text",
        cli_flag: None,
        stdin_supported: true,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Json,
        native_name: "jsonl",
        cli_flag: Some("--json"),
        stdin_supported: true,
        selector: OutputFormatSelector::Flag { flag: "--json" },
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "schema-json",
        cli_flag: Some("--output-schema"),
        stdin_supported: true,
        selector: OutputFormatSelector::FlagValue {
            flag: "--output-schema",
        },
    },
];

const CODEX_ENTRYPOINTS: &[EntrypointSpec] = &[
    EntrypointSpec {
        subcommand: Some("exec"),
        required_flags: &[],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: Some("review"),
        required_flags: &[],
        mode: EntrypointMode::Interactive,
    },
    EntrypointSpec {
        subcommand: Some("exec review"),
        required_flags: &[],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: Some("exec resume"),
        required_flags: &[],
        mode: EntrypointMode::NonInteractive,
    },
];

static CODEX_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::ConfigKeyInline {
            flag: "-c",
            key: "developer_instructions",
        },
        non_interactive: SystemPromptDelivery::ConfigKeyInline {
            flag: "-c",
            key: "developer_instructions",
        },
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::ConfigKeyFile {
            flag: "-c",
            key: "model_instructions_file",
        },
        non_interactive: SystemPromptDelivery::ConfigKeyFile {
            flag: "-c",
            key: "model_instructions_file",
        },
    },
    memory_files: CODEX_MEMORY_FILES,
};

const CODEX_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static CODEX_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "thread.started",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "turn.started",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "item.started",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "item.completed",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "error",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "tool/requestUserInput",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook {
                native_name: "turn.completed",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "turn.failed",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeModel,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "agent_message",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::Jsonl,
                native_name: "reasoning",
            },
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const CODEX_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata"],
    "claudine/docs/cross-referencing/codex.md",
);
const CODEX_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint"],
    "claudine/docs/cross-referencing/codex.md",
);

fn build_codex_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Codex,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".codex/skills", ".codex/skills")
            .with_also_reads(vec![".claude/skills", ".agents/skills"])
            .with_properties(CODEX_SKILL_SCHEMA),
        // Codex custom prompts are deprecated but still supported from user scope.
        // Current Codex docs state prompt files live in the local Codex home and
        // are not shared through the repository, so repo_path is intentionally empty.
        // TODO: remove when Codex fully drops prompt files.
        commands: ResourceSupport::custom_format(ResourceFormat::Markdown, "", ".codex/prompts")
            .with_note("Deprecated custom prompts; user scope only; prefer skills")
            .with_properties(CODEX_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".codex/agents", ".codex/agents"),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".codex/scripts",
            ".codex/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

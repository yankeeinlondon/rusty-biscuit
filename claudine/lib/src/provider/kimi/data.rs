//! Typed static catalog data for the Kimi Code provider.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use crate::events::AgenticEvent;
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport, SkillFrontmatter,
};
use crate::provider::EventSupportLevel;
use crate::provider::OutputFormatSelector;
use crate::provider::ProviderInfo;
use crate::provider::acp::{AcpEvent, AcpServerMode, AcpSupport};
use crate::provider::cli_sensitivity::CliSensitiveAxes;
use crate::provider::event_mapping::WireProxyMode;
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
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use crate::provider::yolo::YoloSupport;
use crate::stream::StreamProtocol;

use super::behavior::KIMI_PROVIDER;
use super::legacy::agent_capabilities;

static KIMI_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_kimi_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &KIMI_RESOURCE_SUPPORT
}

pub(in crate::provider) static KIMI_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::KimiCode,
    display_name: "Kimi Code",
    slug: "kimi",
    short_name: "kimi",
    binary: "kimi",
    agent_offset: ".kimi",
    cli_aliases: &["kimi", "kimicode", "kimi_code", "kimi-code"],
    docs_url: "https://moonshotai.github.io/kimi-cli/en/",
    usage_dashboard_url: Some("https://platform.moonshot.cn/console/account"),
    sniff_binding: AiCli::KimiCli,
    supports_skills: false,
    stream_protocol: Some(StreamProtocol::WireJsonRpc),
    event_mapping: &KIMI_EVENT_MAPPING,
    behavior: &KIMI_PROVIDER,
    mcp: &KIMI_PROVIDER,
    adapter: &KIMI_PROVIDER,
    configurator: &KIMI_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: KIMI_SESSION_LOG_PATHS,
    session_locations: KIMI_SESSION_LOCATIONS,
    config_paths: KIMI_CONFIG_PATHS,
    memory_files: KIMI_MEMORY_FILES,
    output_formats: KIMI_OUTPUT_FORMATS,
    entrypoints: KIMI_ENTRYPOINTS,
    system_prompt: &KIMI_SYSTEM_PROMPT,
    yolo: YoloSupport::DirectFlag {
        native_flag: "--yolo",
    },
    reasoning: ReasoningSupport::BinaryToggle {
        flag: "--thinking",
        on: "--thinking",
        off: "--no-thinking",
    },
    known_gaps: KIMI_KNOWN_GAPS,
    acp: AcpSupport {
        server_mode: AcpServerMode::AvailableViaWireProxy,
        client_supported: true,
        events_via_acp: KIMI_ACP_EVENTS,
    },
    prompt_arg_conventions: PromptArgConventions {
        prompt_flags: &["--prompt"],
        entrypoint: None,
    },
    static_models: &[],
    dynamic_source: ModelCatalogSource::None,
    model_env_vars: &["KIMI_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes::NONE,
    repo_home_root_files: &[],
};

const KIMI_ACP_EVENTS: &[AcpEvent] = &[AcpEvent::ApprovalRequest];

const KIMI_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.kimi/sessions/<dir-hash>/<session-id>/context.jsonl",
)];

const KIMI_SESSION_LOCATIONS: &[PathTemplate] = &[];

const KIMI_CONFIG_PATHS: &[PathTemplate] = &[
    // First element is the primary user-level config path consumed by
    // `config::discover_agents_full`. Kimi's CLI auto-migrates legacy
    // `config.json` to `config.toml`, but historical agent discovery
    // reports presence based on `config.json` so the legacy file remains
    // first to preserve detection behavior.
    PathTemplate::Static("~/.kimi/config.json"),
    PathTemplate::Static("~/.kimi/config.toml"),
    PathTemplate::Static("~/.kimi/mcp.json"),
];

const KIMI_MEMORY_FILES: &[PathTemplate] = &[PathTemplate::Static("AGENTS.md")];

const KIMI_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
    OutputFormatSupport {
        format: OutputFormat::Text,
        native_name: "text",
        cli_flag: None,
        stdin_supported: true,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: Some("--wire"),
        stdin_supported: true,
        selector: OutputFormatSelector::TransportFlag { flag: "--wire" },
    },
];

const KIMI_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: None,
    required_flags: &["--wire"],
    mode: EntrypointMode::NonInteractive,
}];

static KIMI_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::KimiAgentFile),
        non_interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::KimiAgentFile),
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::KimiAgentFile),
        non_interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::KimiAgentFile),
    },
    memory_files: KIMI_MEMORY_FILES,
};

const KIMI_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static KIMI_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::NotSupported,
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
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "TurnBegin",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "ToolCall",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "ToolResult",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "ToolResult",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::Acp {
                event: AcpEvent::ApprovalRequest,
                native_name: "ApprovalRequest",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::Acp {
                event: AcpEvent::ApprovalRequest,
                native_name: "ApprovalRequest",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "TurnEnd",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "prompt.status",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "SubagentEvent",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "SubagentEvent",
            },
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
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "ContentPart",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "CompactionBegin",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::WireProxy {
                mode: WireProxyMode::JsonRpc,
                native_name: "StatusUpdate",
            },
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const KIMI_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "license",
        "compatibility",
        "metadata",
        "type",
    ],
    "claudine/docs/cross-referencing/kimi-code.md",
);
const KIMI_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "system_prompt_path", "tools"],
    &["extend", "system_prompt_args", "exclude_tools", "subagents"],
    "claudine/docs/cross-referencing/kimi-code.md",
);

fn build_kimi_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::KimiCode,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".kimi/skills",
            ".config/agents/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills", ".codex/skills"])
        .with_properties(KIMI_SKILL_SCHEMA),
        commands: ResourceSupport::limited().with_note("Built-in slash commands only"),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".kimi/agents",
            ".kimi/agents",
        )
        .with_note("YAML agent files loaded via --agent-file flag")
        .with_properties(KIMI_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

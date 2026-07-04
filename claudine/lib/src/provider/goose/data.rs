//! Typed static catalog data for the Goose provider.

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
use crate::provider::event_mapping::{EventMapping, EventMappingTable};
use crate::provider::identity::Provider;
use crate::provider::known_gap::KnownGap;
use crate::provider::model_catalog_source::ModelCatalogSource;
use crate::provider::output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport,
};
use crate::provider::path_template::PathTemplate;
use crate::provider::prompt_args::PromptArgConventions;
use crate::provider::reasoning::{ReasoningCustomTag, ReasoningSupport};
use crate::provider::system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use crate::provider::yolo::YoloSupport;

use super::behavior::GOOSE_PROVIDER;
use super::legacy::agent_capabilities;

static GOOSE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_goose_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &GOOSE_RESOURCE_SUPPORT
}

pub(in crate::provider) static GOOSE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Goose,
    display_name: "Goose",
    slug: "goose",
    short_name: "goose",
    binary: "goose",
    agent_offset: ".goose",
    cli_aliases: &["goose"],
    docs_url: "https://block.github.io/goose/",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Goose,
    supports_skills: false,
    stream_protocol: None,
    event_mapping: &GOOSE_EVENT_MAPPING,
    behavior: &GOOSE_PROVIDER,
    mcp: &GOOSE_PROVIDER,
    adapter: &GOOSE_PROVIDER,
    configurator: &GOOSE_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: GOOSE_SESSION_LOG_PATHS,
    session_locations: GOOSE_SESSION_LOCATIONS,
    config_paths: GOOSE_CONFIG_PATHS,
    memory_files: GOOSE_MEMORY_FILES,
    output_formats: GOOSE_OUTPUT_FORMATS,
    entrypoints: GOOSE_ENTRYPOINTS,
    system_prompt: &GOOSE_SYSTEM_PROMPT,
    yolo: YoloSupport::EnvVar {
        env_var: "GOOSE_MODE",
        value: "auto",
    },
    reasoning: ReasoningSupport::ProviderSpecific(ReasoningCustomTag::GooseDelegated),
    known_gaps: GOOSE_KNOWN_GAPS,
    acp: AcpSupport {
        server_mode: AcpServerMode::Native,
        client_supported: true,
        events_via_acp: GOOSE_ACP_EVENTS,
    },
    prompt_arg_conventions: PromptArgConventions {
        prompt_flags: &["-t", "--text"],
        entrypoint: Some("run"),
    },
    static_models: &[],
    dynamic_source: ModelCatalogSource::None,
    model_env_vars: &["GOOSE_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes::NONE,
    repo_home_root_files: &[],
};

const GOOSE_ACP_EVENTS: &[AcpEvent] = &[AcpEvent::RequestPermission];

const GOOSE_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.local/share/goose/sessions/sessions.db",
)];

const GOOSE_SESSION_LOCATIONS: &[PathTemplate] = &[
    PathTemplate::Static("~/.local/state/goose/logs/cli/"),
    PathTemplate::Static("~/.local/state/goose/logs/server/"),
    PathTemplate::Static("~/.local/state/goose/logs/llm_request.*.jsonl"),
];

const GOOSE_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.config/goose/config.yaml"),
    PathTemplate::Static("~/.config/goose/permission.yaml"),
];

const GOOSE_MEMORY_FILES: &[PathTemplate] = &[PathTemplate::Static(".goosehints")];

const GOOSE_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
    OutputFormatSupport {
        format: OutputFormat::Text,
        native_name: "text",
        cli_flag: None,
        stdin_supported: true,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Json,
        native_name: "json",
        cli_flag: None,
        stdin_supported: true,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: None,
        stdin_supported: true,
        selector: OutputFormatSelector::Default,
    },
];

const GOOSE_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: Some("run"),
    required_flags: &[],
    mode: EntrypointMode::NonInteractive,
}];

static GOOSE_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::GooseRecipe),
        non_interactive: SystemPromptDelivery::InlineFlag { flag: "--system" },
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Unsupported,
        non_interactive: SystemPromptDelivery::Unsupported,
    },
    memory_files: GOOSE_MEMORY_FILES,
};

const GOOSE_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static GOOSE_EVENT_MAPPING: EventMappingTable = EventMappingTable {
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
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::NotSupported,
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
            support_level: EventSupportLevel::Acp {
                event: AcpEvent::RequestPermission,
                native_name: "request_permission",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Wrapper {
                native_name: "complete",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::Wrapper {
                native_name: "error",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::Wrapper {
                native_name: "subagent_tool_request",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::Wrapper {
                native_name: "tasks_complete",
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
            support_level: EventSupportLevel::Wrapper {
                native_name: "message",
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
            support_level: EventSupportLevel::Wrapper {
                native_name: "notification",
            },
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const GOOSE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata", "allowed-tools"],
    "claudine/docs/cross-referencing/goose.md",
);
const GOOSE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["title", "description"],
    &[
        "instructions",
        "prompt",
        "extensions",
        "parameters",
        "sub_recipes",
    ],
    "claudine/docs/cross-referencing/goose.md",
);

fn build_goose_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Goose,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".goose/skills",
            ".config/goose/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills"])
        .with_properties(GOOSE_SKILL_SCHEMA),
        commands: ResourceSupport::custom_format(ResourceFormat::Mcp, "", "")
            .with_note("MCP-based commands, not file-based"),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".goose/recipes",
            ".config/goose/recipes",
        )
        .with_note("Recipe YAML files with specific schema")
        .with_properties(GOOSE_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".goose/scripts",
            ".config/goose/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended().with_allowed_tools(),
    }
}

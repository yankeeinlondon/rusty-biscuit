//! Typed static catalog data for the Qwen Code provider.

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
use crate::provider::prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
use crate::provider::reasoning::ReasoningSupport;
use crate::provider::system_prompt::{
    SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use crate::provider::yolo::YoloSupport;
use crate::stream::StreamProtocol;

use super::behavior::QWEN_PROVIDER;
use super::legacy::agent_capabilities;

static QWEN_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_qwen_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &QWEN_RESOURCE_SUPPORT
}

pub(in crate::provider) static QWEN_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::QwenCode,
    display_name: "Qwen Code",
    slug: "qwen",
    short_name: "qwen",
    binary: "qwen",
    agent_offset: ".qwen",
    cli_aliases: &["qwen", "qwencode", "qwen_code", "qwen-code"],
    docs_url: "https://qwenlm.github.io/qwen-code-docs/",
    usage_dashboard_url: Some("https://bailian.console.aliyun.com/"),
    sniff_binding: AiCli::QwenCli,
    supports_skills: true,
    stream_protocol: Some(StreamProtocol::StreamJson),
    event_mapping: &QWEN_EVENT_MAPPING,
    behavior: &QWEN_PROVIDER,
    mcp: &QWEN_PROVIDER,
    adapter: &QWEN_PROVIDER,
    configurator: &QWEN_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: QWEN_SESSION_LOG_PATHS,
    session_locations: QWEN_SESSION_LOCATIONS,
    config_paths: QWEN_CONFIG_PATHS,
    memory_files: QWEN_MEMORY_FILES,
    output_formats: QWEN_OUTPUT_FORMATS,
    entrypoints: QWEN_ENTRYPOINTS,
    system_prompt: &QWEN_SYSTEM_PROMPT,
    yolo: YoloSupport::DirectFlag {
        native_flag: "--yolo",
    },
    reasoning: ReasoningSupport::NumericBudget {
        flag: "thinking_budget",
        min: 0,
        max: 32_768,
        default: None,
    },
    known_gaps: QWEN_KNOWN_GAPS,
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions {
        prompt_flags: &["-p", "--prompt"],
        entrypoint: None,
        value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
    },
    static_models: &[],
    dynamic_source: ModelCatalogSource::OpencodeCliQwenFiltered,
    model_env_vars: &["QWEN_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes {
        read_path: false,
        write_path: false,
        traverse_path: false,
        execute_command: true,
        access_domain: false,
        use_mcp_server: true,
        use_mcp_tool: true,
        spawn_subagent: false,
        switch_mode: false,
        modify_provider_config: true,
    },
    repo_home_root_files: &[],
};

const QWEN_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.qwen/projects/<sanitized-cwd>/chats/",
)];

const QWEN_SESSION_LOCATIONS: &[PathTemplate] = &[PathTemplate::Static("logs/openai")];

const QWEN_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.qwen/settings.json"),
    PathTemplate::Static(".qwen/settings.json"),
];

const QWEN_MEMORY_FILES: &[PathTemplate] = &[
    PathTemplate::Static("~/.qwen/QWEN.md"),
    PathTemplate::Static("QWEN.md"),
];

const QWEN_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
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

const QWEN_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: None,
    required_flags: &[],
    mode: EntrypointMode::NonInteractive,
}];

static QWEN_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
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
    memory_files: QWEN_MEMORY_FILES,
};

const QWEN_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static QWEN_EVENT_MAPPING: EventMappingTable = EventMappingTable {
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
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::StreamJson,
                native_name: "CanUseTool",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::StreamJson,
                native_name: "result",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::StreamParse {
                protocol: StreamProtocol::StreamJson,
                native_name: "result",
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
                protocol: StreamProtocol::StreamJson,
                native_name: "assistant",
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
                protocol: StreamProtocol::StreamJson,
                native_name: "system",
            },
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

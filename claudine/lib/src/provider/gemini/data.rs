//! Typed static catalog data for the Gemini CLI provider.

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

use super::behavior::GEMINI_PROVIDER;
use super::legacy::agent_capabilities;

static GEMINI_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_gemini_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &GEMINI_RESOURCE_SUPPORT
}

pub(in crate::provider) static GEMINI_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Gemini,
    display_name: "Gemini",
    slug: "gemini",
    short_name: "gemini",
    binary: "gemini",
    agent_offset: ".gemini",
    cli_aliases: &["gemini"],
    docs_url: "https://github.com/google-gemini/gemini-cli",
    usage_dashboard_url: Some("https://aistudio.google.com/billing"),
    sniff_binding: AiCli::GeminiCli,
    supports_skills: true,
    stream_protocol: Some(StreamProtocol::StreamJson),
    event_mapping: &GEMINI_EVENT_MAPPING,
    behavior: &GEMINI_PROVIDER,
    mcp: &GEMINI_PROVIDER,
    adapter: &GEMINI_PROVIDER,
    configurator: &GEMINI_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: GEMINI_SESSION_LOG_PATHS,
    session_locations: GEMINI_SESSION_LOCATIONS,
    config_paths: GEMINI_CONFIG_PATHS,
    memory_files: GEMINI_MEMORY_FILES,
    output_formats: GEMINI_OUTPUT_FORMATS,
    entrypoints: GEMINI_ENTRYPOINTS,
    system_prompt: &GEMINI_SYSTEM_PROMPT,
    yolo: YoloSupport::DirectFlag {
        native_flag: "--approval-mode=yolo",
    },
    reasoning: ReasoningSupport::NumericBudget {
        flag: "thinkingBudget",
        min: 0,
        max: 32_768,
        default: None,
    },
    known_gaps: GEMINI_KNOWN_GAPS,
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions {
        prompt_flags: &["-p", "--prompt"],
        entrypoint: None,
        value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
    },
    static_models: &[],
    dynamic_source: ModelCatalogSource::None,
    model_env_vars: &["GEMINI_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes {
        read_path: false,
        write_path: false,
        traverse_path: false,
        execute_command: true,
        access_domain: false,
        use_mcp_server: false,
        use_mcp_tool: false,
        spawn_subagent: false,
        switch_mode: false,
        modify_provider_config: true,
    },
    repo_home_root_files: &[],
};

const GEMINI_SESSION_LOG_PATHS: &[PathTemplate] =
    &[PathTemplate::Static("~/.gemini/tmp/<project_hash>/chats/")];

const GEMINI_SESSION_LOCATIONS: &[PathTemplate] = &[];

const GEMINI_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.gemini/settings.json"),
    PathTemplate::Static(".gemini/settings.json"),
];

const GEMINI_MEMORY_FILES: &[PathTemplate] = &[
    PathTemplate::Static("~/.gemini/GEMINI.md"),
    PathTemplate::Static(".gemini/GEMINI.md"),
    PathTemplate::Static("GEMINI.md"),
];

const GEMINI_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
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

const GEMINI_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: None,
    required_flags: &[],
    mode: EntrypointMode::NonInteractive,
}];

static GEMINI_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::EnvVarFile {
            env_var: "GEMINI_SYSTEM_MD",
        },
        non_interactive: SystemPromptDelivery::EnvVarFile {
            env_var: "GEMINI_SYSTEM_MD",
        },
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::EnvVarFile {
            env_var: "GEMINI_SYSTEM_MD",
        },
        non_interactive: SystemPromptDelivery::EnvVarFile {
            env_var: "GEMINI_SYSTEM_MD",
        },
    },
    memory_files: GEMINI_MEMORY_FILES,
};

const GEMINI_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static GEMINI_EVENT_MAPPING: EventMappingTable = EventMappingTable {
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
                native_name: "BeforeAgent",
            },
            parse_aliases: &["BeforeAgent"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::Hook {
                native_name: "BeforeTool",
            },
            parse_aliases: &["BeforeTool"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Hook {
                native_name: "AfterTool",
            },
            parse_aliases: &["AfterTool"],
            registration_target: true,
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
            support_level: EventSupportLevel::NotSupported,
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook {
                native_name: "AfterAgent",
            },
            parse_aliases: &["AfterAgent"],
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
            support_level: EventSupportLevel::Hook {
                native_name: "BeforeModel",
            },
            parse_aliases: &["BeforeModel", "BeforeToolSelection"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::Hook {
                native_name: "AfterModel",
            },
            parse_aliases: &["AfterModel"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::Hook {
                native_name: "PreCompress",
            },
            parse_aliases: &["PreCompress"],
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

const GEMINI_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/gemini-cli.md",
);
const GEMINI_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["prompt"],
    &["description"],
    "claudine/docs/cross-referencing/gemini-cli.md",
);
const GEMINI_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[
        "kind",
        "tools",
        "model",
        "temperature",
        "max_turns",
        "timeout_mins",
    ],
    "claudine/docs/cross-referencing/gemini-cli.md",
);

fn build_gemini_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Gemini,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".gemini/skills", ".gemini/skills")
            .with_note("Also supports .gemini/modules/ as context modules")
            .with_properties(GEMINI_SKILL_SCHEMA),
        commands: ResourceSupport::custom_format(
            ResourceFormat::Toml,
            ".gemini/commands",
            ".gemini/commands",
        )
        .with_note("Uses TOML format with {{args}} placeholder")
        .with_properties(GEMINI_COMMAND_SCHEMA),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Markdown,
            ".gemini/agents",
            ".gemini/agents",
        )
        .with_note("Sub-agent markdown definitions are experimental")
        .with_properties(GEMINI_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::standard(),
    }
}

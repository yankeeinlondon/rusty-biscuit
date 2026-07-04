//! Typed static catalog data for the OpenCode provider.

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

use super::behavior::OPENCODE_PROVIDER;
use super::legacy::agent_capabilities;

static OPENCODE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_opencode_resource_support);

fn resource_support() -> &'static ProviderCapabilities {
    &OPENCODE_RESOURCE_SUPPORT
}

pub(in crate::provider) static OPENCODE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::OpenCode,
    display_name: "OpenCode",
    slug: "opencode",
    short_name: "opencode",
    binary: "opencode",
    agent_offset: ".opencode",
    cli_aliases: &["opencode", "open_code", "open-code"],
    docs_url: "https://github.com/opencode-ai/opencode",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Opencode,
    supports_skills: true,
    stream_protocol: Some(StreamProtocol::Ndjson),
    event_mapping: &OPENCODE_EVENT_MAPPING,
    behavior: &OPENCODE_PROVIDER,
    mcp: &OPENCODE_PROVIDER,
    adapter: &OPENCODE_PROVIDER,
    configurator: &OPENCODE_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: OPENCODE_SESSION_LOG_PATHS,
    session_locations: OPENCODE_SESSION_LOCATIONS,
    config_paths: OPENCODE_CONFIG_PATHS,
    memory_files: OPENCODE_MEMORY_FILES,
    output_formats: OPENCODE_OUTPUT_FORMATS,
    entrypoints: OPENCODE_ENTRYPOINTS,
    system_prompt: &OPENCODE_SYSTEM_PROMPT,
    yolo: YoloSupport::NonInteractiveOnly {
        non_interactive_flag: "--dangerously-skip-permissions",
    },
    reasoning: ReasoningSupport::NotDocumented,
    known_gaps: OPENCODE_KNOWN_GAPS,
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions::positional_after("run"),
    static_models: &[],
    dynamic_source: ModelCatalogSource::OpencodeCli,
    model_env_vars: &["OPENCODE_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes {
        read_path: false,
        write_path: false,
        traverse_path: false,
        execute_command: false,
        access_domain: false,
        use_mcp_server: false,
        use_mcp_tool: false,
        spawn_subagent: false,
        switch_mode: false,
        modify_provider_config: true,
    },
    repo_home_root_files: &[],
};

const OPENCODE_SESSION_LOG_PATHS: &[PathTemplate] = &[];

const OPENCODE_SESSION_LOCATIONS: &[PathTemplate] = &[];

const OPENCODE_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.config/opencode/opencode.json"),
    PathTemplate::Static("opencode.json"),
];

const OPENCODE_MEMORY_FILES: &[PathTemplate] = &[PathTemplate::Static("AGENTS.md")];

const OPENCODE_OUTPUT_FORMATS: &[OutputFormatSupport] = &[OutputFormatSupport {
    format: OutputFormat::Json,
    native_name: "json",
    cli_flag: Some("--format"),
    stdin_supported: false,
    selector: OutputFormatSelector::FlagValue { flag: "--format" },
}];

const OPENCODE_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: Some("run"),
    required_flags: &[],
    mode: EntrypointMode::NonInteractive,
}];

static OPENCODE_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Unsupported,
        non_interactive: SystemPromptDelivery::Unsupported,
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Unsupported,
        non_interactive: SystemPromptDelivery::Unsupported,
    },
    memory_files: OPENCODE_MEMORY_FILES,
};

const OPENCODE_KNOWN_GAPS: &[KnownGap] = &[KnownGap {
    area: KnownGapArea::Other,
    note: "Populate claudine/docs/agent-cli/opencode.md",
    tracker: Some("claudine/docs/agent-cli/opencode.md"),
}];

pub(super) static OPENCODE_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::Hook {
                native_name: "session.created",
            },
            parse_aliases: &["session.created"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::Hook {
                native_name: "session.deleted",
            },
            parse_aliases: &["session.deleted"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::Hook {
                native_name: "chat.message",
            },
            parse_aliases: &["chat.message"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::Hook {
                native_name: "tool.execute.before",
            },
            parse_aliases: &["tool.execute.before"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Hook {
                native_name: "tool.execute.after",
            },
            parse_aliases: &["tool.execute.after"],
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
            support_level: EventSupportLevel::Hook {
                native_name: "permission.ask",
            },
            parse_aliases: &["permission.ask"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::Hook {
                native_name: "permission.asked",
            },
            parse_aliases: &["permission.asked"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook {
                native_name: "session.idle",
            },
            parse_aliases: &["session.idle"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::Hook {
                native_name: "session.error",
            },
            parse_aliases: &["session.error"],
            registration_target: true,
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
                native_name: "chat.params",
            },
            parse_aliases: &[
                "chat.params",
                "chat.headers",
                "experimental.chat.system.transform",
                "experimental.chat.messages.transform",
            ],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::Hook {
                native_name: "message.updated",
            },
            parse_aliases: &[
                "message.updated",
                "message.part.updated",
                "experimental.text.complete",
            ],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::Hook {
                native_name: "session.compacted",
            },
            parse_aliases: &[
                "session.compacted",
                "session.compacting",
                "experimental.session.compacting",
            ],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::Hook {
                native_name: "tui.toast.show",
            },
            parse_aliases: &["tui.toast.show", "event"],
            registration_target: true,
        },
    ],
};

const OPENCODE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata"],
    "claudine/docs/cross-referencing/opencode.md",
);
const OPENCODE_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "template", "agent", "model", "subtask"],
    "claudine/docs/cross-referencing/opencode.md",
);
const OPENCODE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["description"],
    &[
        "mode",
        "model",
        "temperature",
        "top_p",
        "tools",
        "permission",
        "steps",
        "color",
        "hidden",
        "disable",
        "prompt",
    ],
    "claudine/docs/cross-referencing/opencode.md",
);

fn build_opencode_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::OpenCode,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/skills",
            ".config/opencode/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills"])
        .with_properties(OPENCODE_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/commands",
            ".config/opencode/commands",
        )
        .with_properties(OPENCODE_COMMAND_SCHEMA),
        agents: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/agents",
            ".config/opencode/agents",
        )
        .with_properties(OPENCODE_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".opencode/scripts",
            ".config/opencode/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

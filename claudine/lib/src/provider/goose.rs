//! Goose provider definition.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::OutputFormatSelector;
use super::ProviderInfo;
use super::acp::{AcpEvent, AcpServerMode, AcpSupport};
use super::behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
use super::cli_sensitivity::CliSensitiveAxes;
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use super::known_gap::KnownGap;
use super::model_catalog_source::ModelCatalogSource;
use super::output_format::{EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport};
use super::path_template::PathTemplate;
use super::prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
use super::reasoning::{ReasoningCustomTag, ReasoningSupport};
use super::system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use super::yolo::YoloSupport;
use crate::adapters::ProviderAdapter;
use crate::agents::model::{area_confidence, frontmatter, path_vec, paths};
use crate::agents::{
    ActivationStyle, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, Confidence,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, InvocationStyle, LoggingCapabilities,
    ModelCapabilities, NonInteractiveCapabilities, PermissionCapabilities, ReasoningCapabilities,
    ReasoningStyle, RuntimeCapabilities, ScriptCapabilities, SkillsCapabilities,
    SlashCommandCapabilities, SubagentCapabilities, SystemPromptCapabilities,
};
use crate::config::AgentConfigurator;
use crate::events::AgenticEvent;
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport, SkillFrontmatter,
};
use crate::provider::EventSupportLevel;

#[derive(Debug)]
pub(super) struct GooseProvider;

pub(super) static GOOSE_PROVIDER: GooseProvider = GooseProvider;

impl ProviderBehavior for GooseProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // Goose has no representative raw hook payload shape in the catalog yet.
        false
    }
}
impl McpBehavior for GooseProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::Goose
    }
}
impl AdapterBehavior for GooseProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::GOOSE_ADAPTER
    }
}
impl ConfiguratorBehavior for GooseProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::GooseConfigurator)
    }
}

static GOOSE_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_goose_agent_capabilities);

static GOOSE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_goose_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &GOOSE_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &GOOSE_RESOURCE_SUPPORT
}

pub(super) static GOOSE_INFO: ProviderInfo = ProviderInfo {
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
        value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
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

// Compatibility facade for the legacy `agents::Agent` surface. The typed
// `ProviderInfo` fields above are authoritative for structured provider data.
fn build_goose_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::Goose,
            display_name: "Goose",
            binary: "goose",
        },
        docs: AgentDocs {
            homepage: Some("https://block.github.io/goose/"),
            docs: Some("https://block.github.io/goose/docs/getting-started/installation"),
            skills_docs: Some("https://block.github.io/goose/docs/"),
            slash_docs: Some("https://block.github.io/goose/docs/guides/goose-cli-commands/"),
            subagents_docs: Some("https://block.github.io/goose/docs/"),
            scripts_docs: Some("https://block.github.io/goose/docs/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&[
                "~/.config/goose/config.yaml",
                "~/.config/goose/permission.yaml",
            ]),
            project_files: vec![],
            local_files: vec![],
            format: Some(ConfigFormat::Yaml),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["goose run --provider", "goose run --model"],
                session_switch_commands: vec![],
                aliases: vec![],
                precedence_order: vec![
                    "GOOSE_PROVIDER/GOOSE_MODEL",
                    "~/.config/goose/config.yaml",
                    "configured defaults",
                ],
                notes: vec![
                    "Lead/worker model pattern is supported via GOOSE_LEAD_* env vars",
                    "Interactive sessions use configured model; run overrides are command-scoped",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "goose run -t <text>",
                    "goose run -i <file>",
                    "goose run -i -",
                    "goose run --recipe <recipe.yaml>",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: false,
                limitations: vec![
                    "stdin piping requires -i -",
                    "interactive session command lacks direct --provider/--model overrides",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    ".goosehints",
                    "goose run --system",
                    "GOOSE_MOIM_MESSAGE_TEXT/GOOSE_MOIM_MESSAGE_FILE",
                    "recipe instructions",
                ],
                full_replacement_supported: false,
                replacement_mechanisms: vec![],
                memory_files: vec![".goosehints"],
            },
            permissions: PermissionCapabilities {
                modes: vec!["auto", "smart_approve", "approve", "chat"],
                yolo_equivalent: Some("GOOSE_MODE=auto"),
                sandbox_modes: vec![],
                tool_allowlist_controls: vec!["~/.config/goose/permission.yaml"],
                tool_denylist_controls: vec!["~/.config/goose/permissions/tool_permissions.json"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::ProviderSpecific,
                levels_or_controls: vec!["GEMINI3_THINKING_LEVEL", "CODEX_REASONING_EFFORT"],
                notes: vec!["No global cross-provider thinking flag"],
            },
            logging: LoggingCapabilities {
                session_locations: vec!["~/.local/share/goose/sessions/sessions.db"],
                log_locations: vec![
                    "~/.local/state/goose/logs/cli/",
                    "~/.local/state/goose/logs/server/",
                    "~/.local/state/goose/logs/llm_request.*.jsonl",
                ],
                debug_controls: vec!["--debug"],
                telemetry_controls: vec!["OpenTelemetry env hooks", "Langfuse env hooks"],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::ProviderOnly],
                notes: vec!["Goose itself is free; all cost is provider API usage"],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::AutoMatch,
            user_consent_required: false,
            paths: paths(
                &[
                    "~/.claude/skills",
                    "~/.config/agents/skills",
                    "~/.config/goose/skills",
                ],
                &[".claude/skills", ".goose/skills", ".agents/skills"],
                &[],
                &[],
                &["later directories in the discovery chain take higher precedence"],
            ),
            reads_claude_dirs: true,
            reads_agents_dirs: true,
            frontmatter: frontmatter(
                &["name", "description"],
                &["license", "compatibility", "metadata", "allowed-tools"],
            ),
            docs_url: Some("https://block.github.io/goose/docs/"),
            notes: vec![
                "Reads both .claude/skills and .agents/skills",
                "Experimental support exists for allowed-tools frontmatter",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::RecipeYaml,
            paths: paths(
                &["~/.config/goose/recipes"],
                &[".goose/recipes"],
                &[],
                &[],
                &["recipe registration controls command exposure"],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://block.github.io/goose/docs/"),
            notes: vec!["Custom slash behavior is recipe-based rather than markdown command files"],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::NotApplicable,
            paths: paths(
                &[],
                &[],
                &[],
                &[],
                &["subagent behavior is runtime/delegation based"],
            ),
            enablement_controls: vec!["/mode auto", "explicit delegated requests"],
            invocation_style: InvocationStyle::AutomaticSpawn,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://block.github.io/goose/docs/"),
            notes: vec![
                "Subagents run in isolated sessions",
                "Nested subagent spawning is not documented as supported",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: vec![],
            plugin_dirs: path_vec(&["~/.config/goose/extensions"]),
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["GOOSE_STATUS_HOOK", "recipe execution pipeline"],
            docs_url: Some("https://block.github.io/goose/docs/"),
            notes: vec![
                "No dedicated global scripts directory",
                "Skill-local scripts and recipe tooling are preferred",
            ],
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

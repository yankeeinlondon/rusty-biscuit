//! Kimi Code provider definition.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use super::known_gap::KnownGap;
use super::output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport,
};
use super::path_template::PathTemplate;
use super::reasoning::ReasoningSupport;
use super::system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
use super::yolo::YoloSupport;
use crate::adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::stream::{ParserConfig, StreamProtocol};
use crate::stream::kimi_semantic::KimiSemanticStreamParser;
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

#[derive(Debug)]
pub(super) struct KimiProvider;

pub(super) static KIMI_PROVIDER: KimiProvider = KimiProvider;

impl ProviderBehavior for KimiProvider {
    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        _config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        Box::new(KimiSemanticStreamParser::new(sink))
    }
}
impl McpBehavior for KimiProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::KimiCode
    }
}
impl AdapterBehavior for KimiProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::KIMI_ADAPTER
    }
}
impl ConfiguratorBehavior for KimiProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::KimiCodeConfigurator)
    }
}

static KIMI_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_kimi_agent_capabilities);

static KIMI_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_kimi_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &KIMI_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &KIMI_RESOURCE_SUPPORT
}

pub(super) static KIMI_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::KimiCode,
    display_name: "Kimi Code",
    slug: "kimi_code",
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
};

const KIMI_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "~/.kimi/sessions/<dir-hash>/<session-id>/context.jsonl",
)];

const KIMI_SESSION_LOCATIONS: &[PathTemplate] = &[];

const KIMI_CONFIG_PATHS: &[PathTemplate] = &[
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
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: Some("--print"),
        stdin_supported: true,
    },
];

const KIMI_ENTRYPOINTS: &[EntrypointSpec] = &[EntrypointSpec {
    subcommand: None,
    required_flags: &["--print"],
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
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::NonHook,
            native_name: "TurnBegin",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::NonHook,
            native_name: "ToolCall",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::NonHook,
            native_name: "ToolResult",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::NonHook,
            native_name: "ToolResult",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::NonHook,
            native_name: "ApprovalRequest",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::NonHook,
            native_name: "ApprovalRequest",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::NonHook,
            native_name: "TurnEnd",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::NonHook,
            native_name: "prompt.status",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::NonHook,
            native_name: "SubagentEvent",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::NonHook,
            native_name: "SubagentEvent",
            parse_aliases: &[],
            registration_target: false,
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
            support_level: EventSupportLevel::NonHook,
            native_name: "ContentPart",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::NonHook,
            native_name: "CompactionBegin",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::NonHook,
            native_name: "StatusUpdate",
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

fn build_kimi_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::KimiCode,
            display_name: "Kimi Code CLI",
            binary: "kimi",
        },
        docs: AgentDocs {
            homepage: Some("https://www.kimi.com/code"),
            docs: Some("https://moonshotai.github.io/kimi-cli/en/"),
            skills_docs: Some("https://moonshotai.github.io/kimi-cli/en/"),
            slash_docs: Some("https://moonshotai.github.io/kimi-cli/en/reference/kimi-command"),
            subagents_docs: Some("https://moonshotai.github.io/kimi-cli/en/"),
            scripts_docs: Some("https://moonshotai.github.io/kimi-cli/en/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.kimi/config.toml", "~/.kimi/mcp.json"]),
            project_files: vec![],
            local_files: vec![],
            format: Some(ConfigFormat::Toml),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["-m", "--model"],
                session_switch_commands: vec!["/model"],
                aliases: vec![],
                precedence_order: vec![
                    "--model",
                    "configured default_model in ~/.kimi/config.toml",
                    "provider login defaults",
                ],
                notes: vec![
                    "Model name must exist in config definitions",
                    "KIMI_MODEL_* env vars can override model properties",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "kimi --print",
                    "kimi --quiet",
                    "kimi --print --input-format stream-json --output-format stream-json",
                    "kimi --wire",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "stream-json"],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec!["--print implies --yolo"],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec!["AGENTS.md via /init"],
                full_replacement_supported: true,
                replacement_mechanisms: vec!["--agent-file with system_prompt_path"],
                memory_files: vec!["AGENTS.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec!["prompted", "yolo"],
                yolo_equivalent: Some("--yolo"),
                sandbox_modes: vec![],
                tool_allowlist_controls: vec![],
                tool_denylist_controls: vec![],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::BinaryToggle,
                levels_or_controls: vec!["--thinking", "--no-thinking", "default_thinking"],
                notes: vec!["Thinking availability is model capability-gated"],
            },
            logging: LoggingCapabilities {
                session_locations: vec!["~/.kimi/sessions/<dir-hash>/<session-id>/context.jsonl"],
                log_locations: vec![
                    "~/.kimi/logs/kimi.log",
                    "~/.kimi/sessions/<dir-hash>/<session-id>/wire.jsonl",
                ],
                debug_controls: vec!["--debug", "--verbose"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::Subscription, BillingModel::PerToken],
                notes: vec![
                    "Kimi membership quota supports subscription-style usage",
                    "Moonshot and third-party providers use per-token billing",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &[
                    "~/.config/agents/skills",
                    "~/.agents/skills",
                    "~/.kimi/skills",
                    "~/.claude/skills",
                    "~/.codex/skills",
                ],
                &[
                    ".agents/skills",
                    ".kimi/skills",
                    ".claude/skills",
                    ".codex/skills",
                ],
                &[],
                &[],
                &["first existing directory in each layer wins"],
            ),
            reads_claude_dirs: true,
            reads_agents_dirs: true,
            frontmatter: frontmatter(
                &["name", "description"],
                &["type", "license", "compatibility", "metadata"],
            ),
            docs_url: Some("https://moonshotai.github.io/kimi-cli/en/"),
            notes: vec![
                "Reads .claude/skills and .codex/skills in fallback order",
                "Flow skills are invokable via /flow:<name>",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Partial,
            built_in_supported: true,
            custom_supported: false,
            custom_format: CommandFormat::None,
            paths: paths(&[], &[], &[], &[], &[]),
            supports_subdirectory_namespacing: false,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://moonshotai.github.io/kimi-cli/en/reference/kimi-command"),
            notes: vec![
                "No custom slash command file format is documented",
                "Skills expose /skill:<name> and /flow:<name> invocation routes",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::Yaml,
            paths: paths(
                &["~/.kimi/agents"],
                &[".kimi/agents"],
                &[],
                &[],
                &["--agent-file can override discovery with an explicit YAML path"],
            ),
            enablement_controls: vec!["--agent", "--agent-file"],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://moonshotai.github.io/kimi-cli/en/"),
            notes: vec![
                "Built-in agent specs include default and okabe",
                "Dynamic subagent creation can be enabled via tooling",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: vec![],
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["wire protocol hooks", "ACP server mode"],
            docs_url: Some("https://moonshotai.github.io/kimi-cli/en/"),
            notes: vec![
                "Scripts are colocated under <skill>/scripts rather than in a global directory",
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

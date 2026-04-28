//! OpenCode provider definition.

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
use super::output_format::{EntrypointMode, EntrypointSpec, OutputFormatSupport};
use super::path_template::PathTemplate;
use super::prompt_args::PromptArgConventions;
use super::reasoning::ReasoningSupport;
use super::system_prompt::{SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec};
use super::yolo::YoloSupport;
use crate::adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{McpInjector, OpenCodeInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::stream::opencode_semantic::OpenCodeSemanticStreamParser;
use crate::stream::parser::SemanticStreamParser;
use crate::stream::{ParserConfig, StreamProtocol};
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
pub(super) struct OpenCodeProvider;

pub(super) static OPENCODE_PROVIDER: OpenCodeProvider = OpenCodeProvider;

impl ProviderBehavior for OpenCodeProvider {
    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        Box::new(OpenCodeSemanticStreamParser::new(sink, config.model))
    }
}
impl McpBehavior for OpenCodeProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::OpenCode
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(OpenCodeInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_opencode_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_opencode_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".config").join("opencode").join("opencode.json"),
            Scope::Repo(root) => root.join("opencode.json"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_opencode_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_opencode_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for OpenCodeProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::OPENCODE_ADAPTER
    }
}
impl ConfiguratorBehavior for OpenCodeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::OpenCodeConfigurator)
    }
}

static OPENCODE_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_opencode_agent_capabilities);

static OPENCODE_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_opencode_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &OPENCODE_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &OPENCODE_RESOURCE_SUPPORT
}

pub(super) static OPENCODE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::OpenCode,
    display_name: "OpenCode",
    slug: "open_code",
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
    prompt_arg_conventions: PromptArgConventions::positional_after("run"),
};

const OPENCODE_SESSION_LOG_PATHS: &[PathTemplate] = &[];

const OPENCODE_SESSION_LOCATIONS: &[PathTemplate] = &[];

const OPENCODE_CONFIG_PATHS: &[PathTemplate] = &[
    PathTemplate::Static("~/.config/opencode/opencode.json"),
    PathTemplate::Static("opencode.json"),
];

const OPENCODE_MEMORY_FILES: &[PathTemplate] = &[PathTemplate::Static("AGENTS.md")];

const OPENCODE_OUTPUT_FORMATS: &[OutputFormatSupport] = &[];

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
            support_level: EventSupportLevel::Hook,
            native_name: "session.created",
            parse_aliases: &["session.created"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::Hook,
            native_name: "session.deleted",
            parse_aliases: &["session.deleted"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforePrompt,
            support_level: EventSupportLevel::Hook,
            native_name: "chat.message",
            parse_aliases: &["chat.message"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::Hook,
            native_name: "tool.execute.before",
            parse_aliases: &["tool.execute.before"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Hook,
            native_name: "tool.execute.after",
            parse_aliases: &["tool.execute.after"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::Hook,
            native_name: "permission.ask",
            parse_aliases: &["permission.ask"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::Hook,
            native_name: "permission.asked",
            parse_aliases: &["permission.asked"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook,
            native_name: "session.idle",
            parse_aliases: &["session.idle"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::Hook,
            native_name: "session.error",
            parse_aliases: &["session.error"],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeModel,
            support_level: EventSupportLevel::Hook,
            native_name: "chat.params",
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
            support_level: EventSupportLevel::Hook,
            native_name: "message.updated",
            parse_aliases: &[
                "message.updated",
                "message.part.updated",
                "experimental.text.complete",
            ],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::Hook,
            native_name: "session.compacted",
            parse_aliases: &[
                "session.compacted",
                "session.compacting",
                "experimental.session.compacting",
            ],
            registration_target: true,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::Hook,
            native_name: "tui.toast.show",
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

fn build_opencode_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::OpenCode,
            display_name: "OpenCode",
            binary: "opencode",
        },
        docs: AgentDocs {
            homepage: Some("https://opencode.ai"),
            docs: Some("https://opencode.ai/docs/"),
            skills_docs: Some("https://opencode.ai/docs/skills/"),
            slash_docs: Some("https://opencode.ai/docs/commands/"),
            subagents_docs: Some("https://opencode.ai/docs/agents/"),
            scripts_docs: Some("https://opencode.ai/docs/custom-tools/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.config/opencode/opencode.json"]),
            project_files: path_vec(&["opencode.json"]),
            local_files: vec![],
            format: Some(ConfigFormat::Jsonc),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec![],
                session_switch_commands: vec![],
                aliases: vec![],
                precedence_order: vec![],
                notes: vec![
                    "Model/runtime controls are under-documented until agent-cli/opencode.md is populated",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec!["opencode run"],
                stdin_supported: true,
                output_formats: vec![],
                structured_output_supported: false,
                resume_supported: false,
                limitations: vec![
                    "Slash commands are TUI-only",
                    "CLI runtime flags are not fully captured in current research",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec!["AGENTS.md"],
                full_replacement_supported: false,
                replacement_mechanisms: vec![],
                memory_files: vec!["AGENTS.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec!["allow", "ask", "deny"],
                yolo_equivalent: None,
                sandbox_modes: vec![],
                tool_allowlist_controls: vec![
                    "permission.skill patterns",
                    "permission.task patterns",
                ],
                tool_denylist_controls: vec!["permission.* deny patterns"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NotDocumented,
                levels_or_controls: vec![],
                notes: vec!["Reasoning controls are a known research gap"],
            },
            logging: LoggingCapabilities {
                session_locations: vec![],
                log_locations: vec![],
                debug_controls: vec!["--print-logs", "--log-level ERROR"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::ProviderOnly],
                notes: vec!["OpenCode delegates cost and billing to configured model providers"],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::ToolInvocation,
            user_consent_required: false,
            paths: paths(
                &[
                    "~/.config/opencode/skills",
                    "~/.claude/skills",
                    "~/.agents/skills",
                ],
                &[".opencode/skills", ".claude/skills", ".agents/skills"],
                &[],
                &[],
                &["project-local skill directories are discovered before global skill directories"],
            ),
            reads_claude_dirs: true,
            reads_agents_dirs: true,
            frontmatter: frontmatter(
                &["name", "description"],
                &["license", "compatibility", "metadata"],
            ),
            docs_url: Some("https://opencode.ai/docs/skills/"),
            notes: vec![
                "Skill loading is explicit through the built-in skill tool",
                "Claude skill directory fallback can be disabled via OPENCODE_DISABLE_CLAUDE_CODE",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Mixed,
            paths: paths(
                &["~/.config/opencode/commands"],
                &[".opencode/commands"],
                &[],
                &[],
                &["project command markdown overrides global command markdown"],
            ),
            supports_subdirectory_namespacing: false,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://opencode.ai/docs/commands/"),
            notes: vec![
                "Markdown command files are supported",
                "JSON command definitions are supported in opencode.json",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::MarkdownFrontmatter,
            paths: paths(
                &["~/.config/opencode/agents"],
                &[".opencode/agents"],
                &[],
                &[],
                &["project agents override global agents"],
            ),
            enablement_controls: vec!["task tool", "@mention"],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://opencode.ai/docs/agents/"),
            notes: vec![
                "Delegated child sessions are isolated and stateless",
                "Multiple task calls can run in parallel",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: path_vec(&[".opencode/tools", "~/.config/opencode/tools"]),
            plugin_dirs: path_vec(&[".opencode/plugins", "~/.config/opencode/plugins"]),
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["plugin lifecycle hooks", "custom tools"],
            docs_url: Some("https://opencode.ai/docs/custom-tools/"),
            notes: vec![
                "No dedicated scripts root; tools/plugins provide executable extension points",
                "Skill-local scripts are supported via <skill>/scripts",
            ],
        },
        confidence: ConfidenceProfile {
            overall: Confidence::Medium,
            by_area: vec![
                area_confidence("runtime", Confidence::Low),
                area_confidence("skills", Confidence::High),
                area_confidence("slash_commands", Confidence::High),
                area_confidence("subagents", Confidence::High),
                area_confidence("scripts", Confidence::High),
            ],
            gaps: vec!["Populate claudine/docs/agent-cli/opencode.md"],
        },
    }
}

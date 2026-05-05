//! Gemini CLI provider definition.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::acp::AcpSupport;
use super::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::cli_sensitivity::CliSensitiveAxes;
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use super::known_gap::KnownGap;
use super::model_catalog_source::ModelCatalogSource;
use super::output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport,
};
use super::OutputFormatSelector;
use super::path_template::PathTemplate;
use super::prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
use super::reasoning::ReasoningSupport;
use super::system_prompt::{SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec};
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
use crate::error::Result;
use crate::events::AgenticEvent;
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport, SkillFrontmatter,
};
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{GeminiInjector, McpInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::EventSupportLevel;
use crate::stream::parser::SemanticStreamParser;
use crate::stream::{ParserConfig, StreamProtocol};

#[derive(Debug)]
pub(super) struct GeminiProvider;

pub(super) static GEMINI_PROVIDER: GeminiProvider = GeminiProvider;

impl ProviderBehavior for GeminiProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Gemini, sink, config)
    }
}
impl McpBehavior for GeminiProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Gemini
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(GeminiInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_gemini_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_gemini_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".gemini").join("settings.json"),
            Scope::Repo(root) => root.join(".gemini").join("settings.json"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_json_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_gemini_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for GeminiProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        // Gemini emits two payload shapes: the standard `hook_event_name`
        // shape (where the name must be Gemini-only — Claude already owns
        // shared names like `Stop`/`PreToolUse` via display order) and a
        // legacy `event_name` field used by some Gemini integrations.
        if let Some(name) = raw.get("hook_event_name").and_then(|v| v.as_str()) {
            let claude_table = &super::claude::CLAUDE_EVENT_MAPPING;
            return GEMINI_EVENT_MAPPING.event_from_native_name(name).is_some()
                && claude_table.event_from_native_name(name).is_none();
        }
        raw.get("event_name").is_some()
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::GEMINI_ADAPTER
    }
}
impl ConfiguratorBehavior for GeminiProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::GeminiConfigurator)
    }
}

static GEMINI_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_gemini_agent_capabilities);

static GEMINI_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_gemini_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &GEMINI_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &GEMINI_RESOURCE_SUPPORT
}

pub(super) static GEMINI_INFO: ProviderInfo = ProviderInfo {
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
        interactive: SystemPromptDelivery::ShadowHomeFile {
            relative_path: ".gemini/GEMINI.md",
        },
        non_interactive: SystemPromptDelivery::ShadowHomeFile {
            relative_path: ".gemini/GEMINI.md",
        },
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::ShadowHomeFile {
            relative_path: ".gemini/GEMINI.md",
        },
        non_interactive: SystemPromptDelivery::ShadowHomeFile {
            relative_path: ".gemini/GEMINI.md",
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

// Compatibility facade for the legacy `agents::Agent` surface. The typed
// `ProviderInfo` fields above are authoritative for structured provider data.
fn build_gemini_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::Gemini,
            display_name: "Gemini CLI",
            binary: "gemini",
        },
        docs: AgentDocs {
            homepage: Some("https://github.com/google-gemini/gemini-cli"),
            docs: Some("https://geminicli.com/docs/"),
            skills_docs: Some("https://geminicli.com/docs/skills/"),
            slash_docs: Some("https://geminicli.com/docs/commands/"),
            subagents_docs: Some("https://geminicli.com/docs/agents/"),
            scripts_docs: Some("https://geminicli.com/docs/extensions/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.gemini/settings.json"]),
            project_files: path_vec(&[".gemini/settings.json"]),
            local_files: vec![],
            format: Some(ConfigFormat::Json),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["-m", "--model"],
                session_switch_commands: vec!["/model"],
                aliases: vec!["auto", "pro", "flash", "flash-lite"],
                precedence_order: vec![
                    "--model",
                    "GEMINI_MODEL",
                    "model.name in settings.json",
                    "auto",
                ],
                notes: vec![
                    "Model routing can switch internally based on quota and task complexity",
                    "Sub-agents can resolve models independently",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "gemini <prompt>",
                    "gemini -p <prompt>",
                    "gemini --prompt <prompt>",
                    "gemini -i <prompt>",
                    "stdin piping",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: false,
                limitations: vec![
                    "-p is deprecated in favor of positional prompts",
                    "interactive continuation requires -i",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec!["GEMINI.md hierarchy", "/memory", "@file imports"],
                full_replacement_supported: true,
                replacement_mechanisms: vec!["GEMINI_SYSTEM_MD"],
                memory_files: vec!["~/.gemini/GEMINI.md", ".gemini/GEMINI.md", "GEMINI.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec!["default", "auto_edit", "yolo", "plan"],
                yolo_equivalent: Some("--approval-mode yolo"),
                sandbox_modes: vec!["seatbelt (macOS)", "docker", "podman"],
                tool_allowlist_controls: vec![
                    "policy files allow decision",
                    "approval mode specific policy entries",
                ],
                tool_denylist_controls: vec!["policy files deny decision"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NumericBudget,
                levels_or_controls: vec!["thinkingBudget", "includeThoughts"],
                notes: vec!["Thinking controls are set in settings.json"],
            },
            logging: LoggingCapabilities {
                session_locations: vec!["~/.gemini/tmp/<project_hash>/chats/"],
                log_locations: vec![],
                debug_controls: vec!["--debug"],
                telemetry_controls: vec!["OpenTelemetry via settings/env"],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::Subscription, BillingModel::PerToken],
                notes: vec![
                    "Supports subscription-like plans and per-token API billing",
                    "Authentication mode determines billing model",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::ToolInvocation,
            user_consent_required: true,
            paths: paths(
                &["~/.gemini/skills", "~/.agents/skills"],
                &[".gemini/skills", ".agents/skills"],
                &[],
                &["<extension>/skills"],
                &["workspace > user > extension"],
            ),
            reads_claude_dirs: false,
            reads_agents_dirs: true,
            frontmatter: frontmatter(&["name", "description"], &[]),
            docs_url: Some("https://geminicli.com/docs/skills/"),
            notes: vec![
                "Skill activation requires explicit model tool call",
                "User consent is required before activation",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Toml,
            paths: paths(
                &["~/.gemini/commands"],
                &[".gemini/commands"],
                &[],
                &[],
                &["command path mirrors workspace/user precedence"],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: true,
            reads_claude_dirs: false,
            docs_url: Some("https://geminicli.com/docs/commands/"),
            notes: vec!["TOML commands support {{args}} interpolation"],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Experimental,
            definition_format: AgentDefinitionFormat::MarkdownFrontmatter,
            paths: paths(
                &["~/.gemini/agents"],
                &[".gemini/agents"],
                &[],
                &["A2A remote agents"],
                &["workspace agents override user agents"],
            ),
            enablement_controls: vec!["experimental.enableAgents setting"],
            invocation_style: InvocationStyle::AutomaticSpawn,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://geminicli.com/docs/agents/"),
            notes: vec![
                "Remote A2A agents are optional",
                "Agent feature remains experimental",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: vec![],
            plugin_dirs: path_vec(&[".gemini/extensions", "~/.gemini/extensions"]),
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["hooks", "extensions"],
            docs_url: Some("https://geminicli.com/docs/extensions/"),
            notes: vec![
                "No canonical global scripts folder",
                "Skill-local scripts and extensions are the primary executable surfaces",
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

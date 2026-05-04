//! Roo Code provider definition.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::acp::AcpSupport;
use super::behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
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
use super::prompt_args::PromptArgConventions;
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
use crate::error::Result;
use crate::events::AgenticEvent;
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport, SkillFrontmatter,
};
use crate::mcp::export::ExportServer;
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::EventSupportLevel;

#[derive(Debug)]
pub(super) struct RooProvider;

pub(super) static ROO_PROVIDER: RooProvider = RooProvider;

impl ProviderBehavior for RooProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // Roo has no representative raw hook payload shape in the catalog yet.
        false
    }
}
impl McpBehavior for RooProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::RooCode
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_roo_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_roo_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        match scope {
            Scope::User => None,
            Scope::Repo(root) => Some(root.join(".roo").join("mcp.json")),
        }
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
        crate::mcp::export::write_roo_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for RooProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::ROO_ADAPTER
    }
}
impl ConfiguratorBehavior for RooProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::RooConfigurator)
    }
}

static ROO_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_roo_agent_capabilities);

static ROO_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_roo_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &ROO_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &ROO_RESOURCE_SUPPORT
}

pub(super) static ROO_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::RooCode,
    display_name: "Roo Code",
    slug: "roo_code",
    short_name: "roo",
    binary: "roo",
    agent_offset: ".roo",
    cli_aliases: &["roo", "roocode", "roo_code", "roo-code"],
    docs_url: "https://github.com/RooVetGit/Roo-Code",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Roo,
    supports_skills: true,
    stream_protocol: None,
    event_mapping: &ROO_EVENT_MAPPING,
    behavior: &ROO_PROVIDER,
    mcp: &ROO_PROVIDER,
    adapter: &ROO_PROVIDER,
    configurator: &ROO_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
    session_log_paths: ROO_SESSION_LOG_PATHS,
    session_locations: ROO_SESSION_LOCATIONS,
    config_paths: ROO_CONFIG_PATHS,
    memory_files: ROO_MEMORY_FILES,
    output_formats: ROO_OUTPUT_FORMATS,
    entrypoints: ROO_ENTRYPOINTS,
    system_prompt: &ROO_SYSTEM_PROMPT,
    yolo: YoloSupport::None,
    reasoning: ReasoningSupport::ProviderSpecific(ReasoningCustomTag::RooModeBased),
    known_gaps: ROO_KNOWN_GAPS,
    acp: AcpSupport::NOT_SUPPORTED,
    prompt_arg_conventions: PromptArgConventions::positional_only(),
    static_models: &[],
    dynamic_source: ModelCatalogSource::None,
    model_env_vars: &["ROO_MODEL"],
    cli_sensitive_axes: CliSensitiveAxes::NONE,
    repo_home_root_files: &[],
};

const ROO_SESSION_LOG_PATHS: &[PathTemplate] = &[PathTemplate::Static(
    "VS Code globalStorage/rooveterinaryinc.roo-cline/",
)];

const ROO_SESSION_LOCATIONS: &[PathTemplate] =
    &[PathTemplate::Static("@roo-code/core debug-log output")];

const ROO_CONFIG_PATHS: &[PathTemplate] = &[
    // First element is the primary user-level config path consumed by
    // `config::discover_agents_full`. Roo Code is a VS Code extension
    // whose primary settings file is the JSON document below; the
    // custom-modes templates remain in subsequent slots so other
    // catalog consumers can still discover them.
    PathTemplate::Static("~/.roo/settings.json"),
    PathTemplate::Static("~/.roo/custom_modes.yaml"),
    PathTemplate::Static("~/.roo/custom_modes.json"),
    PathTemplate::Static(".roo/custom_modes.yaml"),
    PathTemplate::Static(".roo/custom_modes.json"),
    PathTemplate::Static(".roomodes"),
];

const ROO_MEMORY_FILES: &[PathTemplate] = &[
    PathTemplate::Static("AGENTS.md"),
    PathTemplate::Static("AGENT.md"),
];

const ROO_OUTPUT_FORMATS: &[OutputFormatSupport] = &[
    OutputFormatSupport {
        format: OutputFormat::Text,
        native_name: "text",
        cli_flag: None,
        stdin_supported: false,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Json,
        native_name: "json",
        cli_flag: None,
        stdin_supported: false,
        selector: OutputFormatSelector::Default,
    },
    OutputFormatSupport {
        format: OutputFormat::Stream,
        native_name: "stream-json",
        cli_flag: None,
        stdin_supported: false,
        selector: OutputFormatSelector::Default,
    },
];

const ROO_ENTRYPOINTS: &[EntrypointSpec] = &[
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--print"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--print", "--prompt-file"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--print", "--stdin-prompt-stream"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--oneshot"],
        mode: EntrypointMode::NonInteractive,
    },
    EntrypointSpec {
        subcommand: None,
        required_flags: &["--ephemeral"],
        mode: EntrypointMode::NonInteractive,
    },
];

static ROO_SYSTEM_PROMPT: SystemPromptSpec = SystemPromptSpec {
    append: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Unsupported,
        non_interactive: SystemPromptDelivery::Unsupported,
    },
    replace: SystemPromptDeliveryByMode {
        interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::RooModePromptFile),
        non_interactive: SystemPromptDelivery::Custom(SystemPromptCustomTag::RooModePromptFile),
    },
    memory_files: ROO_MEMORY_FILES,
};

const ROO_KNOWN_GAPS: &[KnownGap] = &[];

pub(super) static ROO_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskCreated",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SessionEnd,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskAborted",
            },
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
            support_level: EventSupportLevel::Wrapper {
                native_name: "ToolUseOutput",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::Wrapper {
                native_name: "ToolResultOutput",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskToolFailed",
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
            support_level: EventSupportLevel::Wrapper {
                native_name: "WaitingForInput",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskCompleted",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::Wrapper {
                native_name: "Error",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStart,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskSpawned",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::SubagentStop,
            support_level: EventSupportLevel::Wrapper {
                native_name: "TaskDelegationCompleted",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeModel,
            support_level: EventSupportLevel::Wrapper {
                native_name: "StreamingStarted",
            },
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::Wrapper {
                native_name: "StreamingEnded",
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
                native_name: "ModeChanged",
            },
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const ROO_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint", "mode"],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["slug", "name", "roleDefinition", "groups"],
    &["description", "whenToUse", "customInstructions"],
    "claudine/docs/cross-referencing/roo-code.md",
);

fn build_roo_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::RooCode,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".roo/skills", ".roo/skills")
            .with_note("Roo skills are compatible with markdown skill bundles")
            .with_properties(ROO_SKILL_SCHEMA),
        commands: ResourceSupport::full(ResourceFormat::Markdown, ".roo/commands", ".roo/commands")
            .with_properties(ROO_COMMAND_SCHEMA),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".roomodes",
            ".roo/custom_modes.yaml",
        )
        .with_note("Mode definitions via .roomodes/custom_modes.yaml")
        .with_properties(ROO_AGENT_SCHEMA),
        scripts: ResourceSupport::full(ResourceFormat::Executable, ".roo/scripts", ".roo/scripts"),
        skill_frontmatter: SkillFrontmatter::standard().with_allowed_tools(),
    }
}

// Compatibility facade for the legacy `agents::Agent` surface. The typed
// `ProviderInfo` fields above are authoritative for structured provider data.
fn build_roo_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::RooCode,
            display_name: "Roo Code",
            binary: "roo",
        },
        docs: AgentDocs {
            homepage: Some("https://roocode.com/"),
            docs: Some("https://docs.roocode.com/"),
            skills_docs: Some("https://docs.roocode.com/"),
            slash_docs: Some("https://docs.roocode.com/"),
            subagents_docs: Some("https://docs.roocode.com/"),
            scripts_docs: Some("https://docs.roocode.com/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.roo/custom_modes.yaml", "~/.roo/custom_modes.json"]),
            project_files: path_vec(&[
                ".roomodes",
                ".roo/custom_modes.yaml",
                ".roo/custom_modes.json",
            ]),
            local_files: vec![],
            format: Some(ConfigFormat::Mixed),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["--provider", "--model", "--reasoning-effort", "--api-key"],
                session_switch_commands: vec![],
                aliases: vec![],
                precedence_order: vec![
                    "CLI flags",
                    "provider-specific environment variables",
                    "roo auth stored credentials",
                    "roo defaults",
                ],
                notes: vec![
                    "VS Code extension supports profile-based model settings",
                    "CLI does not read a dedicated default model config file",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "roo --print <prompt>",
                    "roo --print --prompt-file <file>",
                    "roo --print --stdin-prompt-stream",
                    "roo --oneshot <prompt>",
                    "roo --ephemeral",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: false,
                limitations: vec![
                    "CLI defaults to auto-approve unless --require-approval is provided",
                    "prompt is required for print mode",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "rules directories",
                    ".roorules/.roorules-{mode}",
                    "AGENTS.md/AGENT.md",
                    ".rooignore",
                ],
                full_replacement_supported: true,
                replacement_mechanisms: vec![".roo/system-prompt-{mode-slug}"],
                memory_files: vec!["AGENTS.md", "AGENT.md"],
            },
            permissions: PermissionCapabilities {
                modes: vec![
                    "auto-approve (default)",
                    "manual approval (--require-approval)",
                ],
                yolo_equivalent: None,
                sandbox_modes: vec![],
                tool_allowlist_controls: vec!["VS Code auto-approve categories"],
                tool_denylist_controls: vec!["VS Code category toggles"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NamedLevels,
                levels_or_controls: vec![
                    "unspecified",
                    "disabled",
                    "none",
                    "minimal",
                    "low",
                    "medium",
                    "high",
                    "xhigh",
                ],
                notes: vec!["Configured through --reasoning-effort on CLI"],
            },
            logging: LoggingCapabilities {
                session_locations: vec!["VS Code globalStorage/rooveterinaryinc.roo-cline/"],
                log_locations: vec!["@roo-code/core debug-log output"],
                debug_controls: vec!["--debug"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::PrepaidCredits, BillingModel::PerToken],
                notes: vec![
                    "Roo Cloud uses prepaid credits",
                    "BYOK providers bill per token",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &["~/.roo/skills", "~/.roo/skills-<modeSlug>"],
                &[".roo/skills", ".roo/skills-<modeSlug>"],
                &[],
                &[],
                &["project mode-specific > project generic > user mode-specific > user generic"],
            ),
            reads_claude_dirs: false,
            reads_agents_dirs: false,
            frontmatter: frontmatter(&["name", "description"], &[]),
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec!["Symlinks are supported for skill directories"],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Markdown,
            paths: paths(
                &["~/.roo/commands"],
                &[".roo/commands"],
                &[],
                &[],
                &["project commands override user commands"],
            ),
            supports_subdirectory_namespacing: false,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec![
                "Command frontmatter includes description, argument-hint, and mode",
                "Programmatic run_slash_command tool is experimental",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Partial,
            definition_format: AgentDefinitionFormat::ModesYaml,
            paths: paths(
                &["~/.roo/custom_modes.yaml", "~/.roo/custom_modes.json"],
                &[
                    ".roomodes",
                    ".roo/custom_modes.yaml",
                    ".roo/custom_modes.json",
                ],
                &[],
                &[],
                &["mode files define orchestrated agent behavior instead of agent directories"],
            ),
            enablement_controls: vec!["new_task tool", "mode selection"],
            invocation_style: InvocationStyle::OrchestratorMode,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: None,
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec![
                "No directory-based .agents definition model",
                "Boomerang task orchestration is mode-centric",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: path_vec(&[".roo/tools", "~/.roo/tools"]),
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["MCP tools", "run_slash_command", "custom tools"],
            docs_url: Some("https://docs.roocode.com/"),
            notes: vec!["Custom tools are the primary script-like extension surface"],
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

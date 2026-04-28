//! Codex CLI provider definition.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::event_mapping::{EventMapping, EventMappingTable};
use super::identity::Provider;
use crate::adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{CodexInjector, McpInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::stream::{ParserConfig, StreamProtocol};
use crate::stream::codex_semantic::CodexSemanticStreamParser;
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
use crate::agents::model::{area_confidence, frontmatter, paths, path_vec};
use crate::linking::capabilities::{
    ProviderCapabilities, ResourceFormat, ResourcePropertySchema, ResourceSupport,
    SkillFrontmatter,
};

#[derive(Debug)]
pub(super) struct CodexProvider;

pub(super) static CODEX_PROVIDER: CodexProvider = CodexProvider;

impl ProviderBehavior for CodexProvider {
    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        Box::new(CodexSemanticStreamParser::new(sink, config.model))
    }
}
impl McpBehavior for CodexProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Codex
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(CodexInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_codex_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_codex_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".codex").join("config.toml"),
            Scope::Repo(root) => root.join(".codex").join("config.toml"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_codex_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_codex_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for CodexProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::adapters::CODEX_ADAPTER
    }
}
impl ConfiguratorBehavior for CodexProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::CodexConfigurator)
    }
}

static CODEX_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_codex_agent_capabilities);

static CODEX_RESOURCE_SUPPORT: LazyLock<ProviderCapabilities> =
    LazyLock::new(build_codex_resource_support);

fn agent_capabilities() -> &'static AgentCapabilities {
    &CODEX_AGENT_CAPABILITIES
}

fn resource_support() -> &'static ProviderCapabilities {
    &CODEX_RESOURCE_SUPPORT
}

pub(super) static CODEX_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Codex,
    display_name: "Codex",
    slug: "codex",
    binary: "codex",
    agent_offset: ".codex",
    cli_aliases: &["codex"],
    docs_url: "https://github.com/openai/codex",
    usage_dashboard_url: Some("https://platform.openai.com/usage"),
    sniff_binding: AiCli::Codex,
    supports_skills: true,
    stream_protocol: Some(StreamProtocol::Jsonl),
    event_mapping: &CODEX_EVENT_MAPPING,
    behavior: &CODEX_PROVIDER,
    mcp: &CODEX_PROVIDER,
    adapter: &CODEX_PROVIDER,
    configurator: &CODEX_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
};

pub(super) static CODEX_EVENT_MAPPING: EventMappingTable = EventMappingTable {
    mappings: &[
        EventMapping {
            event: AgenticEvent::SessionStart,
            support_level: EventSupportLevel::NonHook,
            native_name: "thread.started",
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
            native_name: "turn.started",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeTool,
            support_level: EventSupportLevel::NonHook,
            native_name: "item.started",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterTool,
            support_level: EventSupportLevel::NonHook,
            native_name: "item.completed",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::ToolError,
            support_level: EventSupportLevel::NonHook,
            native_name: "error",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::PermissionRequest,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::HumanInTheLoop,
            support_level: EventSupportLevel::NonHook,
            native_name: "tool/requestUserInput",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnComplete,
            support_level: EventSupportLevel::Hook,
            native_name: "turn.completed",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::TurnError,
            support_level: EventSupportLevel::NonHook,
            native_name: "turn.failed",
            parse_aliases: &[],
            registration_target: false,
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
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::AfterModel,
            support_level: EventSupportLevel::NonHook,
            native_name: "agent_message",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::BeforeCompact,
            support_level: EventSupportLevel::NotSupported,
            native_name: "",
            parse_aliases: &[],
            registration_target: false,
        },
        EventMapping {
            event: AgenticEvent::Notification,
            support_level: EventSupportLevel::NonHook,
            native_name: "reasoning",
            parse_aliases: &[],
            registration_target: false,
        },
    ],
};

const CODEX_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata"],
    "claudine/docs/cross-referencing/codex.md",
);
const CODEX_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint"],
    "claudine/docs/cross-referencing/codex.md",
);

fn build_codex_resource_support() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Codex,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".codex/skills", ".codex/skills")
            .with_also_reads(vec![".claude/skills", ".agents/skills"])
            .with_properties(CODEX_SKILL_SCHEMA),
        // Codex custom prompts are deprecated but still supported from user scope.
        // Current Codex docs state prompt files live in the local Codex home and
        // are not shared through the repository, so repo_path is intentionally empty.
        // TODO: remove when Codex fully drops prompt files.
        commands: ResourceSupport::custom_format(ResourceFormat::Markdown, "", ".codex/prompts")
            .with_note("Deprecated custom prompts; user scope only; prefer skills")
            .with_properties(CODEX_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".codex/agents", ".codex/agents"),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".codex/scripts",
            ".codex/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

fn build_codex_agent_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::Codex,
            display_name: "Codex CLI",
            binary: "codex",
        },
        docs: AgentDocs {
            homepage: Some("https://github.com/openai/codex"),
            docs: Some("https://developers.openai.com/codex/cli/"),
            skills_docs: Some("https://developers.openai.com/codex/cli/"),
            slash_docs: Some("https://developers.openai.com/codex/cli/reference/"),
            subagents_docs: Some("https://developers.openai.com/codex/cli/reference/"),
            scripts_docs: Some("https://developers.openai.com/codex/cli/reference/"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.codex/config.toml"]),
            project_files: path_vec(&[".codex/config.toml"]),
            local_files: vec![],
            format: Some(ConfigFormat::Toml),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec![
                    "-m",
                    "--model",
                    "--oss",
                    "--local-provider",
                    "-p",
                    "--profile",
                ],
                session_switch_commands: vec!["/model"],
                aliases: vec!["gpt-5.3-codex", "o3"],
                precedence_order: vec![
                    "session /model command",
                    "--model flag",
                    "model in ~/.codex/config.toml",
                    "codex default",
                ],
                notes: vec![
                    "Supports profile-specific models via [profiles.<name>]",
                    "Model migrations are recorded in [notice.model_migrations]",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "codex exec",
                    "codex review",
                    "codex exec review",
                    "codex exec resume",
                ],
                stdin_supported: true,
                output_formats: vec![
                    "text",
                    "jsonl (--json)",
                    "schema-constrained json (--output-schema)",
                ],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec![
                    "--search is interactive-only",
                    "approval prompting flags are not available in exec mode",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "AGENTS.md",
                    "AGENTS.override.md",
                    "developer_instructions",
                ],
                full_replacement_supported: true,
                replacement_mechanisms: vec!["model_instructions_file"],
                memory_files: vec![
                    "~/.codex/AGENTS.override.md",
                    "~/.codex/AGENTS.md",
                    "AGENTS.md",
                ],
            },
            permissions: PermissionCapabilities {
                modes: vec!["untrusted", "on-failure", "on-request", "never"],
                yolo_equivalent: Some("--dangerously-bypass-approvals-and-sandbox"),
                sandbox_modes: vec!["read-only", "workspace-write", "danger-full-access"],
                tool_allowlist_controls: vec![
                    "rules files (~/.codex/rules/default.rules, .codex/rules/*.rules)",
                    "trust_level in config.toml",
                ],
                tool_denylist_controls: vec![
                    "rules decisions: prompt|forbidden",
                    "workspace-write exclude_slash_tmp and writable_roots",
                ],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NamedLevels,
                levels_or_controls: vec!["minimal", "low", "medium", "high", "xhigh"],
                notes: vec![
                    "Configured via model_reasoning_effort",
                    "Reasoning summary verbosity can be configured independently",
                ],
            },
            logging: LoggingCapabilities {
                session_locations: vec![
                    "~/.codex/sessions/YYYY/MM/DD/<session-id>/",
                    "~/.codex/history.jsonl",
                ],
                log_locations: vec!["~/.codex/log/codex-tui.log", "~/.codex/shell_snapshots/"],
                debug_controls: vec!["--debug <level>", "RUST_LOG"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::Subscription, BillingModel::PerToken],
                notes: vec![
                    "Subscription via ChatGPT plans is supported",
                    "API-key usage is billed per token",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::AutoMatch,
            user_consent_required: false,
            paths: paths(
                &["~/.agents/skills", "~/.codex/skills"],
                &[".agents/skills", ".codex/skills"],
                &["/etc/codex/skills"],
                &[],
                &[
                    "project .agents/skills takes precedence over project .codex/skills",
                    "project skills override user and admin skills",
                ],
            ),
            reads_claude_dirs: false,
            reads_agents_dirs: true,
            frontmatter: frontmatter(
                &["name", "description"],
                &["license", "compatibility", "metadata"],
            ),
            docs_url: Some("https://developers.openai.com/codex/cli/"),
            notes: vec![
                "Does not read .claude/skills",
                "Supports sidecar configuration in agents/openai.yaml",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Deprecated,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Markdown,
            paths: paths(
                &["~/.codex/prompts"],
                &[],
                &[],
                &[],
                &[
                    "custom prompts are deprecated in favor of built-ins",
                    "custom prompts are documented as user-scoped only",
                ],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: false,
            reads_claude_dirs: false,
            docs_url: Some("https://developers.openai.com/codex/cli/reference/"),
            notes: vec![
                "Built-in slash commands are stable",
                "Custom prompt files remain available but deprecated",
                "Prompt files are not shared through the repository",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Experimental,
            definition_format: AgentDefinitionFormat::Toml,
            paths: paths(
                &["~/.codex/config.toml ([agents.*])"],
                &[],
                &[],
                &[],
                &["no repository-level subagent definition file is documented"],
            ),
            enablement_controls: vec![
                "[features].multi_agent = true",
                "[agents.<name>] sections in config.toml",
            ],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://developers.openai.com/codex/cli/reference/"),
            notes: vec!["Multi-agent roles are experimental"],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: vec![],
            tool_dirs: vec![],
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["notify command in config.toml"],
            docs_url: Some("https://developers.openai.com/codex/cli/reference/"),
            notes: vec![
                "No dedicated global scripts directory",
                "Skill-local scripts are the common pattern",
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

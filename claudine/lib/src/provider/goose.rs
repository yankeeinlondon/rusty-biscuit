//! Goose provider definition.

use std::sync::LazyLock;

use sniff::programs::AiCli;

use super::ProviderInfo;
use super::behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
use super::identity::Provider;
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
pub(super) struct GooseProvider;

pub(super) static GOOSE_PROVIDER: GooseProvider = GooseProvider;

impl ProviderBehavior for GooseProvider {}
impl McpBehavior for GooseProvider {}
impl AdapterBehavior for GooseProvider {}
impl ConfiguratorBehavior for GooseProvider {}

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
    binary: "goose",
    agent_offset: ".goose",
    cli_aliases: &["goose"],
    docs_url: "https://block.github.io/goose/",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Goose,
    supports_skills: false,
    behavior: &GOOSE_PROVIDER,
    mcp: &GOOSE_PROVIDER,
    adapter: &GOOSE_PROVIDER,
    configurator: &GOOSE_PROVIDER,
    agent_capabilities_fn: agent_capabilities,
    resource_support_fn: resource_support,
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
                yolo_equivalent: Some("auto mode"),
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

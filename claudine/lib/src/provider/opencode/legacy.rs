//! TEMPORARY legacy `AgentCapabilities` builders for the OpenCode
//! provider. Deleted at AgentCapabilities retirement
//! (design/module-split.md); nothing new may be added here.

use std::sync::LazyLock;

use crate::agents::model::{area_confidence, frontmatter, path_vec, paths};
use crate::agents::{
    ActivationStyle, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, Confidence,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, InvocationStyle, LoggingCapabilities,
    ModelCapabilities, NonInteractiveCapabilities, PermissionCapabilities, ReasoningCapabilities,
    ReasoningStyle, RuntimeCapabilities, ScriptCapabilities, SkillsCapabilities,
    SlashCommandCapabilities, SubagentCapabilities, SystemPromptCapabilities,
};
use crate::provider::identity::Provider;

static OPENCODE_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_opencode_agent_capabilities);

pub(super) fn agent_capabilities() -> &'static AgentCapabilities {
    &OPENCODE_AGENT_CAPABILITIES
}

// Compatibility facade for the legacy `agents::Agent` surface. The typed
// `ProviderInfo` fields above are authoritative for structured provider data.
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
                output_formats: vec!["json (--format json)"],
                structured_output_supported: true,
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
                yolo_equivalent: Some("--dangerously-skip-permissions"),
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

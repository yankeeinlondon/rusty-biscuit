//! TEMPORARY legacy `AgentCapabilities` builders for the Kimi Code
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

static KIMI_AGENT_CAPABILITIES: LazyLock<AgentCapabilities> =
    LazyLock::new(build_kimi_agent_capabilities);

pub(super) fn agent_capabilities() -> &'static AgentCapabilities {
    &KIMI_AGENT_CAPABILITIES
}

// Compatibility facade for the legacy `agents::Agent` surface. The typed
// `ProviderInfo` fields above are authoritative for structured provider data.
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
                entrypoints: vec!["kimi --wire"],
                stdin_supported: true,
                output_formats: vec!["text", "stream-json"],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec!["--wire uses JSON-RPC prompt delivery"],
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

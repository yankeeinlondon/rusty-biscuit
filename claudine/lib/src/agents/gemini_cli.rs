use crate::events::Provider;

use super::Confidence;
use super::model::{
    ActivationStyle, Agent, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, ConfidenceProfile,
    ConfigCapabilities, ConfigFormat, InvocationStyle, LoggingCapabilities, ModelCapabilities,
    NonInteractiveCapabilities, PermissionCapabilities, ReasoningCapabilities, ReasoningStyle,
    RuntimeCapabilities, ScriptCapabilities, SkillsCapabilities, SlashCommandCapabilities,
    SubagentCapabilities, SystemPromptCapabilities, area_confidence, frontmatter, path_vec, paths,
};

#[derive(Debug, Clone)]
pub struct GeminiCliAgent {
    caps: AgentCapabilities,
}

impl GeminiCliAgent {
    pub fn new() -> Self {
        Self {
            caps: gemini_cli_capabilities(),
        }
    }
}

impl Default for GeminiCliAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for GeminiCliAgent {
    fn id(&self) -> Provider {
        Provider::Gemini
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.caps
    }
}

fn gemini_cli_capabilities() -> AgentCapabilities {
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

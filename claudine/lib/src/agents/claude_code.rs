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
pub struct ClaudeCodeAgent {
    caps: AgentCapabilities,
}

impl ClaudeCodeAgent {
    pub fn new() -> Self {
        Self {
            caps: claude_code_capabilities(),
        }
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for ClaudeCodeAgent {
    fn id(&self) -> Provider {
        Provider::Claude
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.caps
    }
}

fn claude_code_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: Provider::Claude,
            display_name: "Claude Code",
            binary: "claude",
        },
        docs: AgentDocs {
            homepage: Some("https://claude.ai/code"),
            docs: Some("https://code.claude.com/docs/en"),
            skills_docs: Some("https://code.claude.com/docs/en/skills"),
            slash_docs: Some("https://code.claude.com/docs/en/slash-commands"),
            subagents_docs: Some("https://code.claude.com/docs/en/sub-agents"),
            scripts_docs: Some("https://code.claude.com/docs/en/hooks"),
        },
        config: ConfigCapabilities {
            user_files: path_vec(&["~/.claude/settings.json", "~/.claude.json"]),
            project_files: path_vec(&[".claude/settings.json", ".mcp.json"]),
            local_files: path_vec(&[".claude/settings.local.json"]),
            format: Some(ConfigFormat::Json),
        },
        runtime: RuntimeCapabilities {
            model: ModelCapabilities {
                cli_flags: vec!["--model"],
                session_switch_commands: vec!["/model"],
                aliases: vec![
                    "default",
                    "sonnet",
                    "opus",
                    "haiku",
                    "sonnet[1m]",
                    "opusplan",
                ],
                precedence_order: vec![
                    "/model in session",
                    "--model",
                    "ANTHROPIC_MODEL",
                    "model in settings.json",
                ],
                notes: vec![
                    "availableModels can restrict interactive selection",
                    "subagent model can be overridden with CLAUDE_CODE_SUBAGENT_MODEL",
                ],
            },
            non_interactive: NonInteractiveCapabilities {
                supported: true,
                entrypoints: vec![
                    "claude --print",
                    "claude -c --print",
                    "claude -r <session> --print",
                ],
                stdin_supported: true,
                output_formats: vec!["text", "json", "stream-json"],
                structured_output_supported: true,
                resume_supported: true,
                limitations: vec![
                    "print-mode specific flags require --print",
                    "some full-replacement mechanisms differ between interactive and print modes",
                ],
            },
            system_prompt: SystemPromptCapabilities {
                supplement_sources: vec![
                    "--append-system-prompt",
                    "--append-system-prompt-file",
                    "CLAUDE.md hierarchy",
                ],
                full_replacement_supported: true,
                replacement_mechanisms: vec!["--system-prompt", "--system-prompt-file"],
                memory_files: vec![
                    "~/.claude/CLAUDE.md",
                    "CLAUDE.md",
                    ".claude/CLAUDE.md",
                    ".claude/CLAUDE.local.md",
                ],
            },
            permissions: PermissionCapabilities {
                modes: vec![
                    "default",
                    "acceptEdits",
                    "plan",
                    "dontAsk",
                    "bypassPermissions",
                ],
                yolo_equivalent: Some("--dangerously-skip-permissions"),
                sandbox_modes: vec![],
                tool_allowlist_controls: vec![
                    "settings.json permissions",
                    "skill frontmatter allowed-tools",
                    "subagent frontmatter tools",
                ],
                tool_denylist_controls: vec!["subagent disallowedTools", "dontAsk mode auto-deny"],
            },
            reasoning: ReasoningCapabilities {
                style: ReasoningStyle::NamedLevels,
                levels_or_controls: vec!["low", "medium", "high"],
                notes: vec!["Extended thinking budget is model-dependent"],
            },
            logging: LoggingCapabilities {
                session_locations: vec![
                    "~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl",
                ],
                log_locations: vec!["~/.claude/projects/"],
                debug_controls: vec!["--verbose"],
                telemetry_controls: vec![],
            },
            billing: BillingCapabilities {
                models: vec![BillingModel::Subscription, BillingModel::PerToken],
                notes: vec![
                    "Subscription and pay-as-you-go API billing are both supported",
                    "Alternative backends include Bedrock, Vertex, and Foundry",
                ],
            },
        },
        skills: SkillsCapabilities {
            status: CapabilityStatus::Supported,
            activation: ActivationStyle::Mixed,
            user_consent_required: false,
            paths: paths(
                &["~/.claude/skills"],
                &[".claude/skills"],
                &["managed settings (enterprise skills)"],
                &["<plugin>/skills"],
                &[
                    "enterprise > personal > project for name collisions",
                    "plugin skills are namespaced",
                ],
            ),
            reads_claude_dirs: true,
            reads_agents_dirs: false,
            frontmatter: frontmatter(
                &[],
                &[
                    "name",
                    "description",
                    "argument-hint",
                    "disable-model-invocation",
                    "user-invocable",
                    "allowed-tools",
                    "model",
                    "context",
                    "agent",
                    "hooks",
                ],
            ),
            docs_url: Some("https://code.claude.com/docs/en/skills"),
            notes: vec![
                "Skill frontmatter has recommended fields, not hard-required fields",
                "Skills are the preferred replacement for legacy custom slash commands",
            ],
        },
        slash_commands: SlashCommandCapabilities {
            status: CapabilityStatus::Supported,
            built_in_supported: true,
            custom_supported: true,
            custom_format: CommandFormat::Markdown,
            paths: paths(
                &["~/.claude/commands"],
                &[".claude/commands"],
                &[],
                &[],
                &["legacy command files remain supported for backward compatibility"],
            ),
            supports_subdirectory_namespacing: true,
            supports_hot_reload: false,
            reads_claude_dirs: true,
            docs_url: Some("https://code.claude.com/docs/en/slash-commands"),
            notes: vec![
                "Custom command behavior is now primarily modeled through skills",
                "Legacy markdown commands still map to slash command names",
            ],
        },
        subagents: SubagentCapabilities {
            status: CapabilityStatus::Supported,
            definition_format: AgentDefinitionFormat::MarkdownFrontmatter,
            paths: paths(
                &["~/.claude/agents"],
                &[".claude/agents"],
                &[],
                &["<plugin>/agents"],
                &[
                    "--agents session payload has highest precedence",
                    "project agents override user and plugin agents",
                ],
            ),
            enablement_controls: vec!["/agents", "--agents <json>"],
            invocation_style: InvocationStyle::ToolDelegation,
            context_isolation: true,
            parallel_supported: true,
            nesting_supported: Some(false),
            docs_url: Some("https://code.claude.com/docs/en/sub-agents"),
            notes: vec![
                "Task-style subagent delegation is stable",
                "Subagents cannot recursively spawn additional subagents",
            ],
        },
        scripts: ScriptCapabilities {
            status: CapabilityStatus::Partial,
            dedicated_script_dirs: path_vec(&["~/.claude/hooks", ".claude/hooks"]),
            tool_dirs: vec![],
            plugin_dirs: vec![],
            skill_local_scripts_supported: true,
            hook_or_notify_mechanisms: vec!["settings.json hooks", "skill/agent scoped hooks"],
            docs_url: Some("https://code.claude.com/docs/en/hooks"),
            notes: vec![
                "No single global scripts convention; hook commands are the primary automation surface",
                "Skill-local scripts are commonly referenced from SKILL.md",
            ],
        },
        confidence: ConfidenceProfile {
            overall: Confidence::Medium,
            by_area: vec![
                area_confidence("runtime", Confidence::Medium),
                area_confidence("skills", Confidence::Medium),
                area_confidence("slash_commands", Confidence::Medium),
                area_confidence("subagents", Confidence::Medium),
                area_confidence("scripts", Confidence::Medium),
            ],
            gaps: vec!["Populate claudine/docs/cross-referencing/claude-code.md"],
        },
    }
}

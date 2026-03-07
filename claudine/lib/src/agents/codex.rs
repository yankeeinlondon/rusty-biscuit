use super::Confidence;
use super::model::{
    ActivationStyle, Agent, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentId,
    AgentMeta, BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, InvocationStyle, LoggingCapabilities,
    ModelCapabilities, NonInteractiveCapabilities, PermissionCapabilities, ReasoningCapabilities,
    ReasoningStyle, RuntimeCapabilities, ScriptCapabilities, SkillsCapabilities,
    SlashCommandCapabilities, SubagentCapabilities, SystemPromptCapabilities, area_confidence,
    frontmatter, path_vec, paths,
};

#[derive(Debug, Clone)]
pub struct CodexAgent {
    caps: AgentCapabilities,
}

impl CodexAgent {
    pub fn new() -> Self {
        Self {
            caps: codex_capabilities(),
        }
    }
}

impl Default for CodexAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CodexAgent {
    fn id(&self) -> AgentId {
        AgentId::Codex
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.caps
    }
}

fn codex_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: AgentId::Codex,
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
                debug_controls: vec!["--verbose", "RUST_LOG"],
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

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
pub struct RooCodeAgent {
    caps: AgentCapabilities,
}

impl RooCodeAgent {
    pub fn new() -> Self {
        Self {
            caps: roo_code_capabilities(),
        }
    }
}

impl Default for RooCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for RooCodeAgent {
    fn id(&self) -> AgentId {
        AgentId::RooCode
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.caps
    }
}

fn roo_code_capabilities() -> AgentCapabilities {
    AgentCapabilities {
        meta: AgentMeta {
            id: AgentId::RooCode,
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
                yolo_equivalent: Some("default auto-approve behavior"),
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
                session_locations: vec![
                    "VS Code globalStorage/rooveterinaryinc.roo-cline/",
                    "custom roo-cline storage path",
                ],
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

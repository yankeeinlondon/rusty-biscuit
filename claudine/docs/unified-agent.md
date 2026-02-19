# Unified Agent Capability Design

## Scope
This document consolidates the eight blinded agent designs in `claudine/docs/agent-designs/` into one canonical model.

Target agents:
- `claude-code`
- `codex`
- `gemini-cli`
- `goose`
- `kimi-code`
- `opencode`
- `qwen-cli`
- `roo-code`

## Design Objectives
1. Every supported CLI agent gets a similarly structured Rust capability struct.
2. All agent structs implement one shared `Agent` trait.
3. The model captures research-backed metadata for:
- Skills
- Slash commands
- Agents/subagents
- Scripts/extensibility
- Runtime controls (model, prompt, permissions, non-interactive, reasoning, logging, billing)
4. Unknown/incomplete research stays explicit (confidence + gaps), not silently inferred.

## Canonical Rust Model
```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub meta: AgentMeta,
    pub docs: AgentDocs,
    pub config: ConfigCapabilities,

    pub runtime: RuntimeCapabilities,
    pub skills: SkillsCapabilities,
    pub slash_commands: SlashCommandCapabilities,
    pub subagents: SubagentCapabilities,
    pub scripts: ScriptCapabilities,

    pub confidence: ConfidenceProfile,
}

#[derive(Debug, Clone)]
pub struct AgentMeta {
    pub id: AgentId,
    pub display_name: &'static str,
    pub binary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentId {
    ClaudeCode,
    Codex,
    GeminiCli,
    Goose,
    KimiCode,
    OpenCode,
    QwenCli,
    RooCode,
}

#[derive(Debug, Clone)]
pub struct AgentDocs {
    pub homepage: Option<&'static str>,
    pub docs: Option<&'static str>,
    pub skills_docs: Option<&'static str>,
    pub slash_docs: Option<&'static str>,
    pub subagents_docs: Option<&'static str>,
    pub scripts_docs: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ConfigCapabilities {
    pub user_files: Vec<PathBuf>,
    pub project_files: Vec<PathBuf>,
    pub local_files: Vec<PathBuf>,
    pub format: Option<ConfigFormat>,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigFormat {
    Json,
    Jsonc,
    Toml,
    Yaml,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilities {
    pub model: ModelCapabilities,
    pub non_interactive: NonInteractiveCapabilities,
    pub system_prompt: SystemPromptCapabilities,
    pub permissions: PermissionCapabilities,
    pub reasoning: ReasoningCapabilities,
    pub logging: LoggingCapabilities,
    pub billing: BillingCapabilities,
}

#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub cli_flags: Vec<&'static str>,
    pub session_switch_commands: Vec<&'static str>,
    pub aliases: Vec<&'static str>,
    pub precedence_order: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct NonInteractiveCapabilities {
    pub supported: bool,
    pub entrypoints: Vec<&'static str>,
    pub stdin_supported: bool,
    pub output_formats: Vec<&'static str>,
    pub structured_output_supported: bool,
    pub resume_supported: bool,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SystemPromptCapabilities {
    pub supplement_sources: Vec<&'static str>,
    pub full_replacement_supported: bool,
    pub replacement_mechanisms: Vec<&'static str>,
    pub memory_files: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PermissionCapabilities {
    pub modes: Vec<&'static str>,
    pub yolo_equivalent: Option<&'static str>,
    pub sandbox_modes: Vec<&'static str>,
    pub tool_allowlist_controls: Vec<&'static str>,
    pub tool_denylist_controls: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ReasoningCapabilities {
    pub style: ReasoningStyle,
    pub levels_or_controls: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReasoningStyle {
    NamedLevels,
    NumericBudget,
    BinaryToggle,
    ProviderSpecific,
    NotDocumented,
}

#[derive(Debug, Clone)]
pub struct LoggingCapabilities {
    pub session_locations: Vec<&'static str>,
    pub log_locations: Vec<&'static str>,
    pub debug_controls: Vec<&'static str>,
    pub telemetry_controls: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct BillingCapabilities {
    pub models: Vec<BillingModel>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub enum BillingModel {
    Subscription,
    PerToken,
    PrepaidCredits,
    ProviderOnly,
}

#[derive(Debug, Clone)]
pub struct SkillsCapabilities {
    pub status: CapabilityStatus,
    pub activation: ActivationStyle,
    pub user_consent_required: bool,
    pub paths: PathDiscovery,
    pub reads_claude_dirs: bool,
    pub reads_agents_dirs: bool,
    pub frontmatter: FrontmatterContract,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SlashCommandCapabilities {
    pub status: CapabilityStatus,
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_format: CommandFormat,
    pub paths: PathDiscovery,
    pub supports_subdirectory_namespacing: bool,
    pub supports_hot_reload: bool,
    pub reads_claude_dirs: bool,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SubagentCapabilities {
    pub status: CapabilityStatus,
    pub definition_format: AgentDefinitionFormat,
    pub paths: PathDiscovery,
    pub enablement_controls: Vec<&'static str>,
    pub invocation_style: InvocationStyle,
    pub context_isolation: bool,
    pub parallel_supported: bool,
    pub nesting_supported: Option<bool>,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ScriptCapabilities {
    pub status: CapabilityStatus,
    pub dedicated_script_dirs: Vec<PathBuf>,
    pub tool_dirs: Vec<PathBuf>,
    pub plugin_dirs: Vec<PathBuf>,
    pub skill_local_scripts_supported: bool,
    pub hook_or_notify_mechanisms: Vec<&'static str>,
    pub docs_url: Option<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityStatus {
    Supported,
    Partial,
    Deprecated,
    Experimental,
    NotSupported,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum ActivationStyle {
    AutoMatch,
    ToolInvocation,
    ExplicitCommand,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum InvocationStyle {
    ToolDelegation,
    OrchestratorMode,
    AutomaticSpawn,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum AgentDefinitionFormat {
    MarkdownFrontmatter,
    Yaml,
    Toml,
    ModesYaml,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum CommandFormat {
    Markdown,
    Toml,
    JsonConfig,
    RecipeYaml,
    Mixed,
    None,
}

#[derive(Debug, Clone)]
pub struct FrontmatterContract {
    pub required_fields: Vec<&'static str>,
    pub optional_fields: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PathDiscovery {
    pub user_paths: Vec<PathBuf>,
    pub project_paths: Vec<PathBuf>,
    pub admin_paths: Vec<PathBuf>,
    pub extension_paths: Vec<PathBuf>,
    pub precedence_rules: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ConfidenceProfile {
    pub overall: Confidence,
    pub by_area: Vec<AreaConfidence>,
    pub gaps: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct AreaConfidence {
    pub area: &'static str,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    High,
    Medium,
    Low,
}
```

## Canonical Trait
```rust
pub trait Agent: Send + Sync + std::fmt::Debug {
    fn id(&self) -> AgentId;
    fn capabilities(&self) -> &AgentCapabilities;

    fn meta(&self) -> &AgentMeta {
        &self.capabilities().meta
    }

    fn supports_skills(&self) -> bool {
        matches!(self.capabilities().skills.status, CapabilityStatus::Supported | CapabilityStatus::Experimental | CapabilityStatus::Partial)
    }

    fn supports_custom_slash_commands(&self) -> bool {
        self.capabilities().slash_commands.custom_supported
    }

    fn supports_subagents(&self) -> bool {
        matches!(self.capabilities().subagents.status, CapabilityStatus::Supported | CapabilityStatus::Experimental | CapabilityStatus::Partial)
    }

    fn skill_paths(&self) -> &PathDiscovery {
        &self.capabilities().skills.paths
    }

    fn slash_paths(&self) -> &PathDiscovery {
        &self.capabilities().slash_commands.paths
    }

    fn subagent_paths(&self) -> &PathDiscovery {
        &self.capabilities().subagents.paths
    }

    fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        let caps = self.capabilities();

        if caps.meta.binary.is_empty() {
            issues.push("binary is empty".to_string());
        }

        if caps.skills.status == CapabilityStatus::Supported && caps.skills.paths.user_paths.is_empty() && caps.skills.paths.project_paths.is_empty() {
            issues.push("skills marked supported but no discovery paths configured".to_string());
        }

        if caps.subagents.status == CapabilityStatus::Supported && caps.subagents.definition_format == AgentDefinitionFormat::Unknown {
            issues.push("subagents supported but definition format unknown".to_string());
        }

        issues
    }
}
```

## Per-Agent Struct Pattern
Each agent gets a dedicated type but shares the same storage shape:

```rust
#[derive(Debug, Clone)]
pub struct ClaudeCodeAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct CodexAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct GeminiCliAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct GooseAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct KimiCodeAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct OpenCodeAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct QwenCliAgent {
    caps: AgentCapabilities,
}

#[derive(Debug, Clone)]
pub struct RooCodeAgent {
    caps: AgentCapabilities,
}

impl Agent for ClaudeCodeAgent {
    fn id(&self) -> AgentId { AgentId::ClaudeCode }
    fn capabilities(&self) -> &AgentCapabilities { &self.caps }
}

// ... same pattern for other 7 agents
```

This preserves strongly named types per agent while keeping one uniform trait contract.

## Consolidated Capability Matrix

### Skills
- `claude-code`: supported (inferred), `.claude/skills/` (confidence medium)
- `codex`: supported, `.agents/skills/`, `~/.agents/skills/`, `/etc/codex/skills/`, does not read `.claude/skills/`
- `gemini-cli`: supported, `.gemini/skills/` + `.agents/skills/`, consent required on activation
- `goose`: supported, reads both `.claude/skills/` and `.agents/skills/`
- `kimi-code`: supported, layered discovery including `.claude/skills/` and `.codex/skills/`
- `opencode`: supported, reads `.opencode/skills/`, `.claude/skills/`, `.agents/skills/`
- `qwen-cli`: supported, `.qwen/skills/`, no `.claude` fallback
- `roo-code`: supported, `.roo/skills/` and `.roo/skills-<mode>/`, no `.claude` fallback

### Slash Commands
- `claude-code`: built-in + custom markdown (inferred paths `~/.claude/commands/`, `.claude/commands/`)
- `codex`: built-ins supported; custom prompts deprecated
- `gemini-cli`: built-in + custom TOML in `.gemini/commands/`
- `goose`: built-ins + recipe-based custom commands
- `kimi-code`: built-ins only; no custom command files
- `opencode`: built-in + custom markdown in `.opencode/commands/`
- `qwen-cli`: built-in + custom markdown (TOML deprecated) in `.qwen/commands/`
- `roo-code`: built-in + custom markdown in `.roo/commands/`

### Subagents / Delegation
- `claude-code`: Task-tool subagents (stable)
- `codex`: experimental multi-agent roles in TOML
- `gemini-cli`: experimental markdown subagents; optional remote A2A
- `goose`: isolated subagents (auto/explicit), no nesting
- `kimi-code`: YAML agent specs + subagents; optional dynamic creation
- `opencode`: markdown agents + task tool with isolated child sessions
- `qwen-cli`: markdown subagents in `.qwen/agents/`
- `roo-code`: mode-based orchestration (`new_task`), not directory-based agents

### Scripts / Extensibility
- Most agents do not have a dedicated global scripts directory.
- Common pattern: script colocation in skill directories (`<skill>/scripts/`).
- Additional extensibility surfaces:
  - `codex`: `notify` command hook
  - `opencode`: `tools/` and `plugins/`
  - `roo-code`: `.roo/tools/` custom tools
  - `goose`: recipes + extensions

## Consolidated Confidence + Gaps
- High confidence: `codex`, `gemini-cli`, `goose`, `kimi-code`, `qwen-cli`, `roo-code`.
- Medium confidence: `claude-code` (cross-reference placeholder), `opencode` (agent-cli placeholder).
- Required follow-ups:
  1. Populate `claudine/docs/cross-referencing/claude-code.md`.
  2. Populate `claudine/docs/agent-cli/opencode.md`.
  3. Re-run mapping validation once those docs exist.

## Recommended Implementation Sequence
1. Implement shared capability types and `Agent` trait.
2. Add one agent implementation (`CodexAgent`) as reference.
3. Add remaining agent structs with static capability builders.
4. Add `validate()` checks in tests for missing critical metadata.
5. Add snapshot tests for path/format/permission regressions per agent.

## Output Artifacts Consolidated
- `claudine/docs/agent-designs/claude-code.md`
- `claudine/docs/agent-designs/codex.md`
- `claudine/docs/agent-designs/gemini-cli.md`
- `claudine/docs/agent-designs/goose.md`
- `claudine/docs/agent-designs/kimi-code.md`
- `claudine/docs/agent-designs/opencode.md`
- `claudine/docs/agent-designs/qwen-cli.md`
- `claudine/docs/agent-designs/roo-code.md`
- `claudine/docs/unified-agent.md`

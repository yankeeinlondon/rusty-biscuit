# Codex Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/codex.md`
- `claudine/docs/agent-cli/codex.md`

## Design Intent
- Capture Codex-specific choices (TOML config layers, `.agents` ecosystem, deprecated custom prompts, experimental multi-agent).
- Keep definitions explicit so tooling can enforce compatibility and migration rules.

## Proposed Struct
```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct CodexCapabilities {
    pub identity: Identity,
    pub documentation: DocumentationLinks,

    pub instruction_model: InstructionModel,
    pub capability_matrix: CapabilityMatrix,
    pub path_matrix: PathMatrix,

    pub automation: AutomationSurface,
    pub safety: SafetySurface,
    pub observability: ObservabilitySurface,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub key: &'static str,
    pub binary: &'static str,
    pub config_format: &'static str,
    pub config_path: &'static str,
}

#[derive(Debug, Clone)]
pub struct DocumentationLinks {
    pub homepage: &'static str,
    pub docs: &'static str,
    pub skills: &'static str,
    pub slash: &'static str,
    pub agents: &'static str,
}

#[derive(Debug, Clone)]
pub struct InstructionModel {
    pub project_memory_files: Vec<&'static str>,
    pub full_prompt_replacement: Option<&'static str>,
    pub inline_developer_instructions: Option<&'static str>,
    pub precedence_order: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct CapabilityMatrix {
    pub skills: AreaCapabilities,
    pub slash_commands: AreaCapabilities,
    pub subagents: AreaCapabilities,
    pub scripts: AreaCapabilities,
}

#[derive(Debug, Clone)]
pub struct AreaCapabilities {
    pub enabled: bool,
    pub maturity: CapabilityMaturity,
    pub docs_url: Option<&'static str>,
    pub noteworthy_limits: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityMaturity {
    Stable,
    Experimental,
    Deprecated,
    Missing,
}

#[derive(Debug, Clone)]
pub struct PathMatrix {
    pub skills_paths: ScopedPaths,
    pub slash_paths: ScopedPaths,
    pub subagent_paths: ScopedPaths,
    pub scripts_paths: ScopedPaths,
}

#[derive(Debug, Clone)]
pub struct ScopedPaths {
    pub user: Vec<&'static str>,
    pub project: Vec<&'static str>,
    pub admin: Vec<&'static str>,
    pub extensions: Vec<&'static str>,
    pub precedence_notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct AutomationSurface {
    pub non_interactive_entrypoints: Vec<&'static str>,
    pub output_formats: Vec<&'static str>,
    pub headless_constraints: Vec<&'static str>,
    pub schema_output_supported: bool,
}

#[derive(Debug, Clone)]
pub struct SafetySurface {
    pub sandbox_modes: Vec<&'static str>,
    pub approval_modes: Vec<&'static str>,
    pub yolo_switches: Vec<&'static str>,
    pub reasoning_controls: BTreeMap<&'static str, Vec<&'static str>>,
}

#[derive(Debug, Clone)]
pub struct ObservabilitySurface {
    pub session_paths: Vec<&'static str>,
    pub log_paths: Vec<&'static str>,
    pub telemetry_controls: Vec<&'static str>,
}
```

## Proposed Trait
```rust
pub trait Agent {
    fn key(&self) -> &'static str;
    fn binary(&self) -> &'static str;

    fn supports_skills(&self) -> bool;
    fn supports_slash_commands(&self) -> bool;
    fn supports_subagents(&self) -> bool;

    fn user_paths_for(&self, area: CapabilityAreaKind) -> &[&'static str];
    fn project_paths_for(&self, area: CapabilityAreaKind) -> &[&'static str];

    fn maturity_of(&self, area: CapabilityAreaKind) -> CapabilityMaturity;
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityAreaKind {
    Skills,
    SlashCommands,
    Subagents,
    Scripts,
}
```

## Codex Mapping
- Identity:
  - `key`: `codex`
  - `binary`: `codex`
  - `config_format`: `toml`
  - `config_path`: `~/.codex/config.toml`
- Instructions:
  - Project memory: `AGENTS.md`, `AGENTS.override.md`
  - Inline supplement: `developer_instructions`
  - Full replacement: `model_instructions_file`

### Capability Matrix
- Skills:
  - Enabled: true
  - Maturity: stable
  - Paths:
    - Project: `.agents/skills/`, parent and repo-root variants
    - User: `~/.agents/skills/`, legacy `~/.codex/skills/`
    - Admin: `/etc/codex/skills/`
  - Notes:
    - Does not read `.claude/skills/`
    - Supports sidecar `agents/openai.yaml`
- Slash commands:
  - Built-ins: yes
  - Custom: deprecated (`~/.codex/prompts/*.md`)
  - Scope: user only; not repository-shared
  - Maturity: deprecated for custom, stable for built-ins
- Subagents:
  - Enabled: true
  - Maturity: experimental
  - Config: `[features].multi_agent = true`
  - Definition style: TOML `[agents.*]`
  - Limitation: no repo-level agent definitions yet
- Scripts:
  - No top-level scripts directory
  - Pattern: scripts within skill directories (`<skill>/scripts/`)
  - Hook-like entry: `notify` command array in config

### Automation + Safety + Observability
- Non-interactive:
  - `codex exec`, `codex review`, `codex exec review`, `codex exec resume`
  - `--json`, `--output-schema`, `--ephemeral`
- Sandbox modes:
  - `read-only`, `workspace-write`, `danger-full-access`
- Approval modes:
  - `untrusted`, `on-failure`, `on-request`, `never`
- Yolo:
  - `--dangerously-bypass-approvals-and-sandbox` (alias `--yolo`)
- Reasoning:
  - `model_reasoning_effort`: `minimal|low|medium|high|xhigh`
  - Summary verbosity and raw reasoning toggles available
- Logging:
  - `~/.codex/log/codex-tui.log`
  - `~/.codex/sessions/YYYY/MM/DD/<session>/`
  - `~/.codex/history.jsonl`
  - `~/.codex/shell_snapshots/`

## Return-to-Orchestrator Summary
- Produced a Codex-focused model that treats maturity as first-class metadata.
- File: `claudine/docs/agent-designs/codex.md`

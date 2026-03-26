# Claude Code Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/claude-code.md` (placeholder only)
- `claudine/docs/agent-cli/claude-code.md`
- Cross-check in peer research docs for Claude compatibility statements:
  - `claudine/docs/cross-referencing/codex.md`
  - `claudine/docs/cross-referencing/gemini-cli.md`
  - `claudine/docs/cross-referencing/goose.md`
  - `claudine/docs/cross-referencing/kimi-code.md`
  - `claudine/docs/cross-referencing/opencode.md`
  - `claudine/docs/cross-referencing/qwen-cli.md`
  - `claudine/docs/cross-referencing/roo-code.md`

## Design Goals
- Represent Claude Code in a way that is concrete enough for runtime decisions.
- Preserve unknowns explicitly because the dedicated cross-reference file is incomplete.
- Keep shape compatible with future unified trait/object model.

## Proposed Struct
```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ClaudeCodeCapabilities {
    pub id: &'static str,
    pub display_name: &'static str,
    pub binary: &'static str,

    pub docs: AgentDocs,
    pub config: ConfigSurface,
    pub models: ModelSurface,
    pub non_interactive: NonInteractiveSurface,
    pub prompts: PromptSurface,
    pub permissions: PermissionSurface,
    pub reasoning: ReasoningSurface,
    pub logging: LoggingSurface,

    pub skills: CapabilityArea,
    pub slash_commands: CapabilityArea,
    pub subagents: CapabilityArea,
    pub scripts: CapabilityArea,

    pub confidence: SourceConfidence,
}

#[derive(Debug, Clone)]
pub struct AgentDocs {
    pub homepage: &'static str,
    pub docs: &'static str,
    pub cli_reference: &'static str,
}

#[derive(Debug, Clone)]
pub struct ConfigSurface {
    pub user_files: Vec<PathBuf>,
    pub project_files: Vec<PathBuf>,
    pub local_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ModelSurface {
    pub cli_flag: &'static str,
    pub aliases: Vec<&'static str>,
    pub supports_session_switch: bool,
    pub supports_model_allowlist: bool,
}

#[derive(Debug, Clone)]
pub struct NonInteractiveSurface {
    pub print_flag: &'static str,
    pub supports_stdin: bool,
    pub supports_stream_json: bool,
    pub supports_json_schema: bool,
    pub supports_resume: bool,
}

#[derive(Debug, Clone)]
pub struct PromptSurface {
    pub replacement_flags: Vec<&'static str>,
    pub append_flags: Vec<&'static str>,
    pub memory_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PermissionSurface {
    pub modes: Vec<&'static str>,
    pub has_yolo_equivalent: bool,
    pub supports_allowlist: bool,
    pub supports_denylist: bool,
}

#[derive(Debug, Clone)]
pub struct ReasoningSurface {
    pub effort_levels: Vec<&'static str>,
    pub supports_extended_thinking_budget: bool,
}

#[derive(Debug, Clone)]
pub struct LoggingSurface {
    pub session_storage: Vec<PathBuf>,
    pub debug_flags: Vec<&'static str>,
    pub telemetry_env_vars: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct CapabilityArea {
    pub supported: bool,
    pub docs_url: Option<&'static str>,
    pub user_paths: Vec<PathBuf>,
    pub project_paths: Vec<PathBuf>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum SourceConfidence {
    High,
    Medium,
    Low,
}
```

## Proposed Trait
```rust
pub trait AgentDescriptor {
    fn id(&self) -> &'static str;
    fn binary(&self) -> &'static str;

    fn skills(&self) -> &CapabilityArea;
    fn slash_commands(&self) -> &CapabilityArea;
    fn subagents(&self) -> &CapabilityArea;
    fn scripts(&self) -> &CapabilityArea;

    fn permissions(&self) -> &PermissionSurface;
    fn non_interactive(&self) -> &NonInteractiveSurface;
    fn reasoning(&self) -> &ReasoningSurface;
}
```

## Claude Code Mapping (Recommended Initial Values)
- `id`: `"claude-code"`
- `binary`: `"claude"`
- `docs`: `https://code.claude.com/docs/en`
- `cli_reference`: `https://code.claude.com/docs/en/cli-usage`
- `config.user_files`:
  - `~/.claude/settings.json`
  - `~/.claude.json`
- `config.project_files`:
  - `.claude/settings.json`
  - `.mcp.json`
- `config.local_files`:
  - `.claude/settings.local.json`
- `models.cli_flag`: `--model`
- `models.aliases`:
  - `default`, `sonnet`, `opus`, `haiku`, `sonnet[1m]`, `opusplan`
- `non_interactive.print_flag`: `--print`
- `non_interactive.supports_stream_json`: true
- `non_interactive.supports_json_schema`: true
- `permissions.modes`:
  - `default`, `acceptEdits`, `plan`, `dontAsk`, `bypassPermissions`
- `permissions.has_yolo_equivalent`: true (`--dangerously-skip-permissions`)
- `reasoning.effort_levels`: `low`, `medium`, `high`
- `reasoning.supports_extended_thinking_budget`: true
- `logging.session_storage`:
  - `~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl`

### Capability Areas (best-effort with confidence annotation)
- `skills.supported`: true (inferred from CLI flag `--disable-slash-commands` and cross-tool compatibility notes)
- `skills.user_paths`: `~/.claude/skills/` (inferred from multiple peer docs)
- `skills.project_paths`: `.claude/skills/` (inferred from multiple peer docs)
- `slash_commands.supported`: true
- `slash_commands.user_paths`: `~/.claude/commands/` (inferred)
- `slash_commands.project_paths`: `.claude/commands/` (inferred)
- `subagents.supported`: true
- `subagents.user_paths`: `~/.claude/agents/` (inferred)
- `subagents.project_paths`: `.claude/agents/` (inferred)
- `scripts.supported`: partial (no dedicated scripts directory; script execution through tools)

## Risk Notes
- Dedicated Claude cross-reference content is currently missing; capability paths for skills/commands/agents are inferred.
- Confidence should be set to `SourceConfidence::Medium` until `cross-referencing/claude-code.md` is completed.

## Return-to-Orchestrator Summary
- Delivered a conservative, uncertainty-aware capability schema for Claude Code.
- File: `claudine/docs/agent-designs/claude-code.md`

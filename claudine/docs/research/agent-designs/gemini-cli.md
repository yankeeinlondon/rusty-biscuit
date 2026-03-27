# Gemini CLI Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/gemini-cli.md`
- `claudine/docs/agent-cli/gemini-cli.md`

## Primary Design Principle
Gemini CLI has strong feature breadth (skills, commands, subagents, extensions, hooks), so the struct should model not only support, but activation semantics and consent/safety behavior.

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct GeminiCliCapabilities {
    pub core: CoreIdentity,
    pub model_selection: ModelSelection,
    pub headless: HeadlessCapabilities,
    pub instruction_sources: InstructionSources,
    pub permissions: PermissionModel,
    pub thinking: ThinkingModel,
    pub telemetry: TelemetryModel,

    pub skills: SkillsModel,
    pub slash_commands: SlashCommandsModel,
    pub agents: AgentsModel,
    pub script_story: ScriptStory,
}

#[derive(Debug, Clone)]
pub struct CoreIdentity {
    pub id: &'static str,
    pub binary: &'static str,
    pub config_files: Vec<&'static str>,
    pub docs: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ModelSelection {
    pub cli_flag: &'static str,
    pub aliases: Vec<&'static str>,
    pub precedence: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct HeadlessCapabilities {
    pub supported: bool,
    pub entry_modes: Vec<&'static str>,
    pub output_formats: Vec<&'static str>,
    pub exit_codes: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct InstructionSources {
    pub context_files: Vec<&'static str>,
    pub full_replacement_paths: Vec<&'static str>,
    pub replacement_toggle_env_vars: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PermissionModel {
    pub approval_modes: Vec<&'static str>,
    pub yolo_supported: bool,
    pub policy_files: Vec<&'static str>,
    pub sandbox_options: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ThinkingModel {
    pub style: ThinkingStyle,
    pub controls: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub enum ThinkingStyle {
    NamedLevels,
    NumericBudget,
    BinaryToggle,
}

#[derive(Debug, Clone)]
pub struct TelemetryModel {
    pub session_storage: Vec<&'static str>,
    pub otel_supported: bool,
    pub config_keys: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SkillsModel {
    pub supported: bool,
    pub activation: SkillActivation,
    pub requires_user_consent_to_activate: bool,
    pub discovery_paths: Vec<&'static str>,
    pub precedence: Vec<&'static str>,
    pub compatible_alias_paths: Vec<&'static str>,
    pub frontmatter_fields_used: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub enum SkillActivation {
    AutoDescriptionMatch,
    ToolInvocation,
    ExplicitCommandOnly,
}

#[derive(Debug, Clone)]
pub struct SlashCommandsModel {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_format: Option<&'static str>,
    pub custom_paths: Vec<&'static str>,
    pub supports_subdirectory_namespacing: bool,
    pub supports_hot_reload: bool,
    pub docs_url: &'static str,
}

#[derive(Debug, Clone)]
pub struct AgentsModel {
    pub supported: bool,
    pub maturity: &'static str,
    pub enablement: Vec<&'static str>,
    pub definition_format: &'static str,
    pub definition_paths: Vec<&'static str>,
    pub supports_remote_agents: bool,
    pub context_isolation: bool,
}

#[derive(Debug, Clone)]
pub struct ScriptStory {
    pub has_dedicated_script_dir: bool,
    pub preferred_locations: Vec<&'static str>,
    pub extensions_can_bundle_scripts: bool,
}
```

## Proposed Trait
```rust
pub trait CliAgent {
    fn core(&self) -> &CoreIdentity;
    fn permissions(&self) -> &PermissionModel;

    fn skills(&self) -> &SkillsModel;
    fn slash_commands(&self) -> &SlashCommandsModel;
    fn agents(&self) -> &AgentsModel;

    fn supports_headless(&self) -> bool;
    fn supports_structured_output(&self) -> bool;
}
```

## Gemini CLI Mapping
- Core:
  - `id`: `gemini-cli`
  - `binary`: `gemini`
  - Config: `~/.gemini/settings.json`, `.gemini/settings.json`
- Model selection:
  - CLI: `--model`
  - Aliases: `auto`, `pro`, `flash`, `flash-lite`
  - Precedence: CLI > `GEMINI_MODEL` > config > default
- Headless:
  - Positional prompt, deprecated `--prompt`, stdin, prompt-interactive
  - Output: `text`, `json`, `stream-json`

### Skills
- Supported: yes
- Activation: model calls `activate_skill`
- Consent: yes (user confirmation before load)
- Paths:
  - Workspace: `.gemini/skills/`, `.agents/skills/`
  - User: `~/.gemini/skills/`, `~/.agents/skills/`
  - Extension-bundled skills
- Precedence: workspace > user > extension
- Frontmatter used: `name`, `description`
- Claude interop:
  - Does not read `.claude/skills/`
  - `.agents/skills/` is bridge path

### Slash Commands
- Built-ins: yes
- Custom: yes, TOML
- Paths: `~/.gemini/commands/*.toml`, `.gemini/commands/*.toml`
- Subdirectory namespacing: yes (`git/commit.toml` => `/git:commit`)
- Hot reload: yes (`/commands reload`)

### Agents/Subagents
- Supported: yes (experimental)
- Enablement: `settings.json -> experimental.enableAgents`
- Definition: Markdown + YAML frontmatter
- Paths: `~/.gemini/agents/*.md`, `.gemini/agents/*.md`
- Remote agents (A2A): yes
- Isolation: yes
- YOLO behavior for subagent tools: effectively yes unless tool list is restricted

### Permissions / Thinking / Telemetry
- Approval modes: `default`, `auto_edit`, `yolo`, `plan`
- Policy engine: TOML policy files (user/admin tiers)
- Sandboxing: seatbelt on macOS, Docker/Podman cross-platform
- Thinking: numeric budget (`thinkingBudget`) and `includeThoughts`
- Session storage: `~/.gemini/tmp/<project_hash>/chats/`
- OpenTelemetry: supported via settings/env

### Scripts
- Dedicated scripts directory: no
- Primary placement: inside skills (`<skill>/scripts/`)
- Extensions can also bundle executable behavior

## Return-to-Orchestrator Summary
- Delivered a capability model focused on activation semantics and explicit consent boundaries.
- File: `claudine/docs/agent-designs/gemini-cli.md`

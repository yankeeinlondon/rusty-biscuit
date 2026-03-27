# Qwen CLI Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/qwen-cli.md`
- `claudine/docs/agent-cli/qwen-cli.md`

## Design Focus
Qwen CLI should be modeled as a Gemini-derived agent with its own directory conventions (`.qwen/*`), strong command/subagent UX, and configurable approval/thinking behavior via settings.

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct QwenCliCapabilities {
    pub identity: Identity,
    pub modeling: ModelCapabilities,
    pub headless: HeadlessCapabilities,
    pub context: ContextCapabilities,
    pub permissions: PermissionCapabilities,
    pub reasoning: ReasoningCapabilities,
    pub logging: LoggingCapabilities,

    pub skills: SkillCapabilities,
    pub slash_commands: SlashCommandCapabilities,
    pub subagents: SubagentCapabilities,
    pub scripts: ScriptCapabilities,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub id: &'static str,
    pub binary: &'static str,
    pub docs: &'static str,
    pub config_file: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub cli_flag: &'static str,
    pub resolution_precedence: Vec<&'static str>,
    pub supports_auth_type_switching: bool,
    pub session_switch_command: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct HeadlessCapabilities {
    pub supported: bool,
    pub positional_prompt_preferred: bool,
    pub deprecated_prompt_flag: Option<&'static str>,
    pub output_formats: Vec<&'static str>,
    pub supports_resume_in_headless: bool,
}

#[derive(Debug, Clone)]
pub struct ContextCapabilities {
    pub memory_file_default: &'static str,
    pub memory_file_search_pattern: &'static str,
    pub supplement_not_replace: bool,
    pub runtime_commands: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PermissionCapabilities {
    pub approval_modes: Vec<&'static str>,
    pub yolo_supported: bool,
    pub tool_controls: Vec<&'static str>,
    pub sandbox_supported: bool,
}

#[derive(Debug, Clone)]
pub struct ReasoningCapabilities {
    pub has_cli_level_flag: bool,
    pub settings_keys: Vec<&'static str>,
    pub per_turn_prefix_controls: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct LoggingCapabilities {
    pub session_paths: Vec<&'static str>,
    pub debug_flag: Option<&'static str>,
    pub api_logging_supported: bool,
    pub telemetry_supported: bool,
}

#[derive(Debug, Clone)]
pub struct SkillCapabilities {
    pub supported: bool,
    pub maturity: &'static str,
    pub discovery_paths: Vec<&'static str>,
    pub precedence: Vec<&'static str>,
    pub reads_claude_dirs: bool,
    pub frontmatter_fields: Vec<&'static str>,
    pub explicit_invoke_command: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SlashCommandCapabilities {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_paths: Vec<&'static str>,
    pub custom_formats: Vec<&'static str>,
    pub supports_subdirectory_namespacing: bool,
    pub reads_claude_commands: bool,
}

#[derive(Debug, Clone)]
pub struct SubagentCapabilities {
    pub supported: bool,
    pub definition_paths: Vec<&'static str>,
    pub definition_format: &'static str,
    pub management_commands: Vec<&'static str>,
    pub context_isolation: bool,
    pub nested_delegation_supported: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ScriptCapabilities {
    pub dedicated_scripts_dir: bool,
    pub supported_script_locations: Vec<&'static str>,
}
```

## Proposed Trait
```rust
pub trait AgentFeatures {
    fn id(&self) -> &'static str;
    fn binary(&self) -> &'static str;

    fn skills_supported(&self) -> bool;
    fn custom_commands_supported(&self) -> bool;
    fn subagents_supported(&self) -> bool;

    fn skill_paths(&self) -> Vec<&'static str>;
    fn command_paths(&self) -> Vec<&'static str>;
    fn subagent_paths(&self) -> Vec<&'static str>;
}
```

## Qwen Mapping
- Identity:
  - `id`: `qwen-cli`
  - `binary`: `qwen`
  - config: `~/.qwen/settings.json`
- Modeling:
  - Flag: `--model`
  - Precedence: CLI > `OPENAI_MODEL` > config > auth-default model
  - Auth type selection: yes (`--auth-type`)

### Skills
- Supported: yes
- Maturity: experimental origins (v0.6.0), now documented as regular feature
- Paths:
  - User: `~/.qwen/skills/`
  - Project: `.qwen/skills/`
  - Extension-provided skills
- Precedence: project > user > extension
- Reads `.claude/skills`: no
- Frontmatter: `name`, `description`
- Explicit invoke: `/skills <name>`

### Slash Commands
- Built-in: yes
- Custom: yes
- Paths:
  - User: `~/.qwen/commands/`
  - Project: `.qwen/commands/`
- Formats:
  - Preferred: Markdown with optional YAML frontmatter
  - Deprecated: TOML
- Subdirectory namespacing: yes (`git/commit.md` -> `/git:commit`)
- Reads `.claude/commands`: no

### Subagents
- Supported: yes
- Paths:
  - User: `~/.qwen/agents/`
  - Project: `.qwen/agents/`
  - Extension: `agents/` in extension
- Format: Markdown + YAML (`name`, `description`, optional `tools`, `color`)
- Management commands: `/agents create`, `/agents manage`
- Isolation: yes
- Nested delegation: undocumented/assume no

### Permissions / Thinking / Logging
- Approval modes: `plan`, `default`, `auto-edit`, `yolo`
- YOLO switches: `--yolo` and `--approval-mode yolo`
- Tool controls in settings: `tools.allowed`, `tools.exclude`, `tools.core`
- Thinking:
  - No dedicated CLI level flag
  - Settings keys: `enable_thinking`, `thinking_budget`
  - Per-turn controls: `/think`, `/no_think`
- Logging:
  - Sessions: `~/.qwen/projects/<sanitized-cwd>/chats/`
  - API request/response logging supported
  - `--debug` + telemetry settings supported

### Scripts
- Dedicated scripts dir: no
- Script location: inside skills (`<skill>/scripts/`)

## Return-to-Orchestrator Summary
- Produced a Qwen profile optimized for `.qwen/*` conventions and settings-driven control surfaces.
- File: `claudine/docs/agent-designs/qwen-cli.md`

# Goose Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/goose.md`
- `claudine/docs/agent-cli/goose.md`

## Design Position
Goose should be modeled as a hybrid agent platform where skills, recipes, and subagents are deeply connected. A plain boolean matrix is not enough; reusable workflow metadata must be captured.

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct GooseCapabilities {
    pub agent_id: &'static str,
    pub binary: &'static str,

    pub config: GooseConfig,
    pub model_runtime: GooseModelRuntime,
    pub execution: GooseExecution,
    pub instructions: GooseInstructions,
    pub approvals: GooseApprovals,
    pub reasoning: GooseReasoning,
    pub logging: GooseLogging,

    pub skills: GooseSkills,
    pub commands: GooseCommands,
    pub subagents: GooseSubagents,
    pub scripts: GooseScripts,
    pub recipes: GooseRecipes,
}

#[derive(Debug, Clone)]
pub struct GooseConfig {
    pub config_file: &'static str,
    pub env_overrides: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseModelRuntime {
    pub provider_override_flags: Vec<&'static str>,
    pub supports_lead_worker_pattern: bool,
    pub lead_worker_envs: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseExecution {
    pub non_interactive_command: &'static str,
    pub prompt_inputs: Vec<&'static str>,
    pub output_formats: Vec<&'static str>,
    pub no_persistence_flag: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseInstructions {
    pub context_files: Vec<&'static str>,
    pub system_supplement_flags: Vec<&'static str>,
    pub persistent_memory_envs: Vec<&'static str>,
    pub supports_full_system_replacement: bool,
}

#[derive(Debug, Clone)]
pub struct GooseApprovals {
    pub default_mode: &'static str,
    pub modes: Vec<&'static str>,
    pub yolo_equivalent_mode: &'static str,
    pub permission_files: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseReasoning {
    pub configuration_style: &'static str,
    pub provider_specific_controls: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseLogging {
    pub log_locations: Vec<&'static str>,
    pub has_session_db: bool,
    pub observability_envs: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseSkills {
    pub supported: bool,
    pub discovery_priority_low_to_high: Vec<&'static str>,
    pub reads_claude_skill_dirs: bool,
    pub supports_agents_skills_bridge: bool,
    pub frontmatter_contract: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseCommands {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_mechanism: &'static str,
    pub reads_claude_commands: bool,
}

#[derive(Debug, Clone)]
pub struct GooseSubagents {
    pub supported: bool,
    pub trigger_modes: Vec<&'static str>,
    pub isolation: &'static str,
    pub can_nest: bool,
    pub parallel_execution_supported: bool,
}

#[derive(Debug, Clone)]
pub struct GooseScripts {
    pub dedicated_global_dir: bool,
    pub preferred_locations: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GooseRecipes {
    pub supported: bool,
    pub recipe_format: &'static str,
    pub can_register_as_slash_commands: bool,
    pub supports_subrecipes: bool,
    pub recipe_locations: Vec<&'static str>,
}
```

## Proposed Trait
```rust
pub trait AgentSurface {
    fn id(&self) -> &'static str;
    fn command(&self) -> &'static str;

    fn skills_enabled(&self) -> bool;
    fn commands_enabled(&self) -> bool;
    fn delegated_execution_enabled(&self) -> bool;

    fn instruction_files(&self) -> Vec<&'static str>;
    fn approval_modes(&self) -> Vec<&'static str>;
}
```

## Goose Mapping
- Config/runtime:
  - Config file: `~/.config/goose/config.yaml`
  - Provider/model envs: `GOOSE_PROVIDER`, `GOOSE_MODEL`
  - Lead/worker model support via `GOOSE_LEAD_*` variables
- Non-interactive:
  - `goose run` with `-t`, `-i`, `-i -`
  - Formats: `text`, `json`, `stream-json`
  - Stateless option: `--no-session`
- System prompt behavior:
  - Supplements only (no full replacement)
  - `.goosehints`, `--system`, recipe instructions, `GOOSE_MOIM_*`

### Skills
- Supported: yes
- Directory order (lowest -> highest):
  - `~/.claude/skills/`
  - `~/.config/agents/skills/`
  - `~/.config/goose/skills/`
  - `./.claude/skills/`
  - `./.goose/skills/`
  - `./.agents/skills/`
- Reads Claude skills: yes
- Frontmatter:
  - Required: `name`, `description`
  - Optional: `license`, `compatibility`, `metadata`, experimental `allowed-tools`

### Commands
- Built-ins: yes
- Custom mechanism: recipe registration in config
- Reads `.claude/commands`: no

### Subagents
- Supported: yes
- Trigger: autonomous (`auto` mode) or explicit request
- Isolation: full isolated session
- Nesting: not allowed
- Parallel: yes

### Scripts + Recipes
- Dedicated global scripts dir: no
- Script placement: skill-local `scripts/`
- Recipes: first-class YAML workflows with subrecipes and scheduling

### Permissions / Reasoning / Logging
- Modes: `auto`, `smart_approve`, `approve`, `chat`
- YOLO equivalent: `auto`
- Thinking controls: provider-specific env vars (Gemini/Codex providers)
- Logs:
  - `~/.local/state/goose/logs/cli/`
  - `~/.local/state/goose/logs/server/`
  - `~/.local/share/goose/sessions/sessions.db`
- Observability: OTEL + Langfuse env hooks

## Return-to-Orchestrator Summary
- Produced a Goose profile that treats recipes as a first-class capability peer to skills/commands/subagents.
- File: `claudine/docs/agent-designs/goose.md`

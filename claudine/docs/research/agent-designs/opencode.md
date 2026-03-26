# OpenCode Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/opencode.md`
- `claudine/docs/agent-cli/opencode.md` (placeholder only)

## Design Direction
OpenCode should be represented as a plugin/tool-first platform where skills, commands, agents, and tool permissions share one policy surface (`opencode.json`).

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct OpenCodeCapabilities {
    pub identity: OpenCodeIdentity,
    pub config: OpenCodeConfig,

    pub skills: OpenCodeSkills,
    pub commands: OpenCodeCommands,
    pub agents: OpenCodeAgents,
    pub scripts: OpenCodeScripts,

    pub permissions: OpenCodePermissions,
    pub session_behavior: OpenCodeSessionBehavior,
    pub confidence: Confidence,
}

#[derive(Debug, Clone)]
pub struct OpenCodeIdentity {
    pub id: &'static str,
    pub binary: &'static str,
    pub implementation_stack: &'static str,
}

#[derive(Debug, Clone)]
pub struct OpenCodeConfig {
    pub global_config: &'static str,
    pub project_config: &'static str,
    pub accepts_jsonc: bool,
    pub plural_dir_convention: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct OpenCodeSkills {
    pub supported: bool,
    pub docs_url: &'static str,
    pub discovery_paths: Vec<&'static str>,
    pub reads_claude_dirs: bool,
    pub claude_sync_disable_env: Option<&'static str>,
    pub activation_model: &'static str,
    pub frontmatter_fields: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct OpenCodeCommands {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_format: &'static str,
    pub project_paths: Vec<&'static str>,
    pub global_paths: Vec<&'static str>,
    pub supports_json_config_commands: bool,
    pub reads_claude_command_dirs: bool,
    pub placeholder_syntax: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct OpenCodeAgents {
    pub supported: bool,
    pub project_paths: Vec<&'static str>,
    pub global_paths: Vec<&'static str>,
    pub definition_format: &'static str,
    pub frontmatter_fields: Vec<&'static str>,
    pub primary_vs_subagent_mode: bool,
    pub task_isolation: bool,
    pub parallel_subagents_supported: bool,
}

#[derive(Debug, Clone)]
pub struct OpenCodeScripts {
    pub dedicated_scripts_dir: bool,
    pub custom_tools_dirs: Vec<&'static str>,
    pub plugins_dirs: Vec<&'static str>,
    pub skill_local_scripts_supported: bool,
}

#[derive(Debug, Clone)]
pub struct OpenCodePermissions {
    pub policy_object_root: &'static str,
    pub pattern_decisions: Vec<&'static str>,
    pub task_permission_controls: bool,
    pub skill_permission_controls: bool,
}

#[derive(Debug, Clone)]
pub struct OpenCodeSessionBehavior {
    pub child_sessions_are_isolated: bool,
    pub child_sessions_are_stateless: bool,
    pub tui_only_slash_commands: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    High,
    Medium,
    Low,
}
```

## Proposed Trait
```rust
pub trait AgentCapabilityProvider {
    fn id(&self) -> &'static str;

    fn skill_locations(&self) -> Vec<&'static str>;
    fn command_locations(&self) -> Vec<&'static str>;
    fn agent_locations(&self) -> Vec<&'static str>;

    fn has_unified_permission_surface(&self) -> bool;
    fn confidence(&self) -> Confidence;
}
```

## OpenCode Mapping
- Identity:
  - `id`: `opencode`
  - `binary`: `opencode`
  - Stack: TypeScript, TUI + desktop + IDE extension
- Config:
  - Global: `~/.config/opencode/opencode.json`
  - Project: `opencode.json`
  - JSON/JSONC supported
  - Primary directories: `.opencode/{agents,commands,modes,plugins,skills,tools,themes}`

### Skills
- Supported: yes
- Activation: explicit built-in `skill` tool (`skill({ name })`)
- Discovery paths:
  - Project: `.opencode/skills/`, `.claude/skills/`, `.agents/skills/`
  - Global: `~/.config/opencode/skills/`, `~/.claude/skills/`, `~/.agents/skills/`
- Reads Claude skills: yes (disable via `OPENCODE_DISABLE_CLAUDE_CODE=1`)
- Frontmatter: `name`, `description` (+ optional `license`, `compatibility`, `metadata`)

### Commands
- Built-in: yes
- Custom: yes
- Paths:
  - Project: `.opencode/commands/*.md`
  - Global: `~/.config/opencode/commands/*.md`
- Format: Markdown with optional frontmatter (`description`, `template`, `agent`, `model`, `subtask`)
- JSON alternative in `opencode.json` under `command`
- Reads Claude commands: no

### Agents/Subagents
- Supported: yes
- Paths:
  - Project: `.opencode/agents/*.md`
  - Global: `~/.config/opencode/agents/*.md`
- Format: Markdown + frontmatter (`description`, `mode`, `model`, `tools`, `permission`, etc.)
- Mode concept supports primary + subagent roles
- Delegation tool: `task`
- Isolated, stateless child sessions
- Parallel subagent execution: yes (multiple `task` calls)

### Scripts
- Dedicated scripts dir: no
- Extensibility surfaces:
  - Custom tools: `.opencode/tools/`, `~/.config/opencode/tools/`
  - Plugins: `.opencode/plugins/`, `~/.config/opencode/plugins/`
  - Skill-local scripts: `<skill>/scripts/`

### Permissions + Session
- Unified permission map in `opencode.json`
- Pattern decisions: `allow`, `deny`, `ask`
- Scope-specific permission keys for `skill` and `task`
- Slash commands are TUI-only

## Known Gaps
- `claudine/docs/agent-cli/opencode.md` is empty, so model/CLI flag/telemetry sections are not fully captured here.
- Set confidence to `Confidence::Medium` until the agent-cli research doc is populated.

## Return-to-Orchestrator Summary
- Delivered an OpenCode design emphasizing shared policy and multi-surface extensibility.
- File: `claudine/docs/agent-designs/opencode.md`

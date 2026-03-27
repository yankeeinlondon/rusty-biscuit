# Kimi Code Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/kimi-code.md`
- `claudine/docs/agent-cli/kimi-code.md`

## Design Thesis
Kimi Code is highly configurable around agent specs (YAML), runtime protocols (print/stream-json/wire/acp), and cross-tool skill discovery. The struct should emphasize layered discovery and protocol-level automation.

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct KimiCodeCapabilities {
    pub id: &'static str,
    pub binary: &'static str,

    pub transport: TransportModes,
    pub modeling: ModelingControls,
    pub prompting: PromptControls,
    pub approvals: ApprovalControls,
    pub reasoning: ReasoningControls,
    pub persistence: PersistenceControls,

    pub skills: SkillsCapabilities,
    pub slash: SlashCapabilities,
    pub agents: AgentCapabilities,
    pub scripts: ScriptCapabilities,
}

#[derive(Debug, Clone)]
pub struct TransportModes {
    pub interactive_shell: bool,
    pub print_mode: bool,
    pub stream_json_io: bool,
    pub wire_json_rpc: bool,
    pub acp_server: bool,
}

#[derive(Debug, Clone)]
pub struct ModelingControls {
    pub cli_model_flag: &'static str,
    pub model_must_be_predefined: bool,
    pub env_overrides: Vec<&'static str>,
    pub session_switch_command: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PromptControls {
    pub has_inline_system_replacement_flag: bool,
    pub custom_agent_file_support: bool,
    pub custom_agent_file_flag: Option<&'static str>,
    pub template_vars: Vec<&'static str>,
    pub memory_files: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ApprovalControls {
    pub default_mode: &'static str,
    pub yolo_switches: Vec<&'static str>,
    pub implicit_yolo_modes: Vec<&'static str>,
    pub per_tool_policy_supported: bool,
}

#[derive(Debug, Clone)]
pub struct ReasoningControls {
    pub model_capability_gated: bool,
    pub mode_type: &'static str,
    pub enable_flags: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PersistenceControls {
    pub base_dir: &'static str,
    pub session_paths: Vec<&'static str>,
    pub log_path: &'static str,
    pub env_base_dir_override: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SkillsCapabilities {
    pub supported: bool,
    pub built_in_skills: Vec<&'static str>,
    pub layered_discovery: Vec<DiscoveryLayer>,
    pub override_flag: Option<&'static str>,
    pub reads_claude_skills: bool,
    pub reads_codex_skills: bool,
    pub flow_skills_supported: bool,
    pub frontmatter_fields: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct DiscoveryLayer {
    pub layer_name: &'static str,
    pub paths_in_priority_order: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct SlashCapabilities {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub skill_prefix_commands: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub supported: bool,
    pub file_format: &'static str,
    pub built_in_agents: Vec<&'static str>,
    pub custom_agent_flag: Option<&'static str>,
    pub subagent_support: bool,
    pub dynamic_subagent_creation: bool,
    pub context_isolation: bool,
}

#[derive(Debug, Clone)]
pub struct ScriptCapabilities {
    pub dedicated_scripts_dir: bool,
    pub supported_locations: Vec<&'static str>,
}
```

## Proposed Trait
```rust
pub trait AgentRuntimeSpec {
    fn id(&self) -> &'static str;
    fn binary_name(&self) -> &'static str;

    fn supports_skill_discovery(&self) -> bool;
    fn supports_custom_slash_commands(&self) -> bool;
    fn supports_subagent_delegation(&self) -> bool;

    fn discovery_paths_for_skills(&self) -> Vec<&'static str>;
    fn prompt_override_mechanism(&self) -> Vec<&'static str>;
}
```

## Kimi Mapping
- Core:
  - `id`: `kimi-code`
  - `binary`: `kimi`
- Transport modes:
  - Interactive shell: yes
  - Print mode: yes (`--print`)
  - stream-json I/O: yes (`--input-format stream-json` + `--output-format stream-json`)
  - Wire: yes (`--wire`)
  - ACP: yes (`kimi acp`)

### Skills
- Supported: yes
- Built-ins: `kimi-cli-help`, `skill-creator`
- Layered discovery:
  - Built-in layer
  - User layer (first existing dir wins):
    - `~/.config/agents/skills/`
    - `~/.agents/skills/`
    - `~/.kimi/skills/`
    - `~/.claude/skills/`
    - `~/.codex/skills/`
  - Project layer (first existing dir wins):
    - `.agents/skills/`
    - `.kimi/skills/`
    - `.claude/skills/`
    - `.codex/skills/`
- Override flag: `--skills-dir`
- Reads Claude/Codex skills: yes
- Flow skills: yes (`type: flow`, `/flow:<name>`)

### Slash Commands
- Built-in: yes
- Custom command files: no
- Skill-as-command mechanism: `/skill:<name>` and `/flow:<name>`

### Agents/Subagents
- Supported: yes
- Format: YAML agent spec (`version: 1`, `agent` object)
- Built-ins: `default`, `okabe`
- Custom agent file: `--agent-file`
- Subagents: yes
- Dynamic subagent creation: optional `CreateSubagent` tool
- Context isolation: yes for task-delivered subagents

### Prompt/System Model
- No direct `--system-prompt` flag
- Prompt replacement path: custom agent YAML with `system_prompt_path`
- Template vars include `${KIMI_NOW}`, `${KIMI_WORK_DIR}`, `${KIMI_AGENTS_MD}`, `${KIMI_SKILLS}`

### Permissions/Thinking/Logging
- Default: prompt for sensitive actions
- YOLO: `--yolo` (`--print` implies yolo)
- Thinking: binary toggle (`--thinking` / `--no-thinking`), gated by model capability
- Logging:
  - `~/.kimi/logs/kimi.log`
  - Sessions under `~/.kimi/sessions/<dir-hash>/<session-id>/`
  - `KIMI_SHARE_DIR` can relocate base directory

### Scripts
- Dedicated scripts directory: no
- Supported location: `<skills-dir>/<skill>/scripts/`

## Return-to-Orchestrator Summary
- Produced a Kimi schema centered on layered skill discovery and protocol-level runtime capabilities.
- File: `claudine/docs/agent-designs/kimi-code.md`

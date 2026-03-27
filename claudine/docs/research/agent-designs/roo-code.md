# Roo Code Agent Design (Blinded)

## Sources Consulted
- `claudine/docs/cross-referencing/roo-code.md`
- `claudine/docs/agent-cli/roo-code.md`

## Design Stance
Roo Code is mode-centric rather than file-defined subagents. The struct should model mode orchestration (Boomerang tasks), mode-scoped skills, and dual runtime surfaces (CLI + VS Code extension).

## Proposed Struct
```rust
#[derive(Debug, Clone)]
pub struct RooCodeCapabilities {
    pub id: &'static str,
    pub binary: &'static str,

    pub runtime_surfaces: RuntimeSurfaces,
    pub model_controls: ModelControls,
    pub non_interactive: NonInteractiveControls,
    pub prompt_controls: PromptControls,
    pub permission_controls: PermissionControls,
    pub reasoning_controls: ReasoningControls,
    pub logging_controls: LoggingControls,

    pub skills: RooSkills,
    pub slash_commands: RooSlashCommands,
    pub agenting: RooAgenting,
    pub scripts: RooScripts,
}

#[derive(Debug, Clone)]
pub struct RuntimeSurfaces {
    pub cli_supported: bool,
    pub vscode_supported: bool,
    pub cli_binary: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModelControls {
    pub cli_provider_flag: &'static str,
    pub cli_model_flag: &'static str,
    pub cli_reasoning_flag: &'static str,
    pub extension_supports_profiles: bool,
    pub sticky_models_supported: bool,
}

#[derive(Debug, Clone)]
pub struct NonInteractiveControls {
    pub print_mode: bool,
    pub stream_mode: bool,
    pub oneshot_mode: bool,
    pub ephemeral_mode: bool,
    pub output_formats: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PromptControls {
    pub supplement_sources: Vec<&'static str>,
    pub full_replacement_supported: bool,
    pub replacement_paths: Vec<&'static str>,
    pub replacement_template_vars: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PermissionControls {
    pub cli_default_auto_approve: bool,
    pub cli_manual_override_flag: Option<&'static str>,
    pub extension_auto_approve_categories: Vec<&'static str>,
    pub has_named_yolo_mode: bool,
}

#[derive(Debug, Clone)]
pub struct ReasoningControls {
    pub levels: Vec<&'static str>,
    pub default_level: &'static str,
    pub extension_provider_specific_controls: bool,
}

#[derive(Debug, Clone)]
pub struct LoggingControls {
    pub cli_debug_flag: Option<&'static str>,
    pub extension_storage_paths: Vec<&'static str>,
    pub supports_custom_storage_path: bool,
}

#[derive(Debug, Clone)]
pub struct RooSkills {
    pub supported: bool,
    pub discovery_paths_by_scope: Vec<&'static str>,
    pub mode_specific_paths_supported: bool,
    pub supports_symlinks: bool,
    pub reads_claude_skills: bool,
    pub frontmatter_fields: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct RooSlashCommands {
    pub built_in_supported: bool,
    pub custom_supported: bool,
    pub custom_paths: Vec<&'static str>,
    pub format: &'static str,
    pub frontmatter_fields: Vec<&'static str>,
    pub programmatic_execution_tool: Option<&'static str>,
    pub reads_claude_commands: bool,
}

#[derive(Debug, Clone)]
pub struct RooAgenting {
    pub uses_modes_as_agents: bool,
    pub built_in_modes: Vec<&'static str>,
    pub custom_mode_files: Vec<&'static str>,
    pub orchestration_tool: Option<&'static str>,
    pub summary_return_pattern: &'static str,
    pub strict_context_isolation: bool,
}

#[derive(Debug, Clone)]
pub struct RooScripts {
    pub dedicated_scripts_dir: bool,
    pub custom_tools_paths: Vec<&'static str>,
    pub skill_script_support: bool,
    pub mcp_as_script_container: bool,
}
```

## Proposed Trait
```rust
pub trait AgentPlatform {
    fn id(&self) -> &'static str;

    fn has_skills(&self) -> bool;
    fn has_custom_slash_commands(&self) -> bool;
    fn has_delegation_model(&self) -> bool;

    fn delegation_style(&self) -> &'static str;
    fn instruction_override_style(&self) -> &'static str;
}
```

## Roo Mapping
- Runtime:
  - CLI binary: `roo`
  - VS Code extension remains primary feature surface
- Model controls:
  - CLI: `--provider`, `--model`, `--reasoning-effort`
  - Extension: API profiles + sticky models

### Skills
- Supported: yes
- Paths:
  - Project generic: `.roo/skills/<name>/SKILL.md`
  - Project mode-specific: `.roo/skills-<modeSlug>/<name>/SKILL.md`
  - User generic: `~/.roo/skills/<name>/SKILL.md`
  - User mode-specific: `~/.roo/skills-<modeSlug>/<name>/SKILL.md`
- Priority: project mode-specific > project generic > global mode-specific > global generic
- Symlinks supported (up to 5 levels)
- Reads `.claude/skills`: no
- Frontmatter: `name`, `description`

### Slash Commands
- Built-ins: yes
- Custom: yes
- Paths:
  - Project: `.roo/commands/*.md`
  - User: `~/.roo/commands/*.md`
- Format: Markdown + frontmatter
- Fields: `description`, `argument-hint`, `mode`
- Programmatic execution: experimental `run_slash_command` tool
- Reads `.claude/commands`: no

### Agenting / Subagents
- Directory-based `.agents/*.md`: no
- Architecture: modes + Boomerang tasks
- Built-in modes: `code`, `architect`, `ask`, `debug`, `orchestrator`
- Custom mode files: `custom_modes.yaml` / `custom_modes.json`, `.roomodes`
- Delegation tool: `new_task`
- Completion return: `attempt_completion(result=...)`
- Isolation: strict

### Scripts / Tools
- Dedicated scripts directory: no
- Primary executable extension: custom tools in `.roo/tools/` and `~/.roo/tools/`
- Skill-local scripts supported (`<skill>/scripts/`)
- MCP can provide external execution surface

### Prompts / Permissions / Reasoning / Logging
- Prompt supplements from rules directories and AGENTS.md/AGENT.md
- Full replacement supported via `.roo/system-prompt-{mode-slug}`
- CLI default behavior: auto-approve; `--require-approval` re-enables prompts
- No named yolo mode, but equivalent full auto-approval is available
- Reasoning levels: `unspecified`, `disabled`, `none`, `minimal`, `low`, `medium`, `high`, `xhigh`
- CLI debug via `--debug`; extension state in global storage paths

## Return-to-Orchestrator Summary
- Delivered a Roo model where modes are first-class agent definitions and Boomerang orchestration is explicit.
- File: `claudine/docs/agent-designs/roo-code.md`

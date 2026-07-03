---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://moonshotai.github.io/kimi-cli/en/
system_prompt_docs: https://moonshotai.github.io/kimi-cli/en/customization/agents.md
append_support: indirect
replace_support: agent_spec
cli_params:
  - flag: --agent-file
    mode: replace
    value_shape: path
    description: Load a custom agent specification YAML file. The file's system_prompt_path replaces the default system prompt for the session.
    example: kimi --agent-file ./my-agent.yaml
    notes: Mutually exclusive with --agent. The most direct way to replace the system prompt.
  - flag: --agent
    mode: other
    value_shape: string
    description: Select a built-in agent (default or okabe). Each built-in agent has its own hardcoded system prompt and toolset.
    example: kimi --agent okabe
    notes: Mutually exclusive with --agent-file.
  - flag: --config-file
    mode: modify
    value_shape: path
    description: Load an alternate TOML/JSON configuration file (default ~/.kimi/config.toml). Can change settings that affect prompt injections (hooks, skills, plan/afk mode).
    example: kimi --config-file ./custom-config.toml
    notes: Mutually exclusive with --config.
  - flag: --config
    mode: modify
    value_shape: string
    description: Pass configuration content inline as TOML/JSON.
    example: kimi --config '{"default_thinking":true}'
    notes: Mutually exclusive with --config-file.
  - flag: --skills-dir
    mode: modify
    value_shape: path
    description: Append additional skill directories. Repeatable. Replaces default user/project skill discovery.
    example: kimi --skills-dir ./extra-skills
    notes: Skills are injected into the system prompt as a list.
  - flag: --add-dir
    mode: modify
    value_shape: path
    description: Add an additional directory to the workspace scope. Adds ${KIMI_ADDITIONAL_DIRS_INFO} into the system prompt.
    example: kimi --add-dir /path/to/related-project
    notes: Repeatable; persisted with session state.
  - flag: --thinking / --no-thinking
    mode: modify
    value_shape: boolean
    description: Enable or disable thinking/reasoning mode. Requires model support; does not change the system prompt text itself.
    example: kimi --thinking
    notes: Overrides last session state and config default.
  - flag: --plan
    mode: modify
    value_shape: boolean
    description: Start in plan mode. Injects plan-mode constraints and restricts available tools.
    example: kimi --plan
    notes: Resumed sessions preserve existing plan mode unless --plan forces it on.
  - flag: --yolo / --afk
    mode: modify
    value_shape: boolean
    description: Auto-approve tool calls. AFK also auto-dismisses AskUserQuestion. AFK injects a system reminder; YOLO no longer does as of v1.40.0.
    example: kimi --yolo
    notes: Use with caution; affects runtime behavior and prompt injections.
config_sources:
  - os: all
    scope: repo
    path: AGENTS.md (from git project root to working directory, including .kimi/AGENTS.md)
    mode: append
    format: markdown
    notes: Merged content is injected into the default system prompt via ${KIMI_AGENTS_MD}. Deeper files take precedence under a 32 KiB budget cap.
  - os: all
    scope: repo
    path: .kimi/skills/, .claude/skills/, .codex/skills/, .agents/skills/
    mode: append
    format: markdown
    notes: Skills discovered by project root. Same-name skills resolved by specificity (Project > User > Extra > Built-in). Injected via ${KIMI_SKILLS}.
  - os: all
    scope: user
    path: ~/.kimi/skills/, ~/.claude/skills/, ~/.codex/skills/, ~/.config/agents/skills/, ~/.agents/skills/
    mode: append
    format: markdown
    notes: User-level skills loaded across projects. merge_all_available_skills config controls whether brand directories are merged.
  - os: all
    scope: user
    path: ~/.kimi/config.toml
    mode: modify
    format: toml
    notes: Configures hooks, skills dirs, loop control, plan/afk defaults, and prompt-injection-related settings. Does not contain a free-form system prompt key.
  - os: all
    scope: agent
    path: Custom agent YAML file referenced by --agent-file
    mode: replace
    format: yaml
    notes: "Defines system_prompt_path, tools, subagents, and system_prompt_args. Extends default via extend: default."
  - os: all
    scope: system
    path: src/kimi_cli/agents/default/system.md (built-in package file)
    mode: replace
    format: markdown
    notes: The built-in system prompt template. Not directly editable by users; replaced via --agent-file.
env_vars:
  - name: KIMI_SHARE_DIR
    effect: Changes the share directory for config, sessions, logs, and runtime data. Does not affect skills search paths.
    mode: modify
  - name: KIMI_CLI_NO_AUTO_UPDATE
    effect: Disables update checks and startup update gate.
    mode: modify
  - name: KIMI_CLI_PASTE_CHAR_THRESHOLD
    effect: Controls when pasted text is folded into a placeholder.
    mode: other
  - name: KIMI_CLI_PASTE_LINE_THRESHOLD
    effect: Controls when multiline pasted text is folded into a placeholder.
    mode: other
prompt_layers:
  - source: Built-in system.md template
    mode: replace
    scope:
      - session
    order_notes: Base layer. Replaced entirely by --agent-file with a custom system_prompt_path.
    notes: Jinja2/Moustache-style template using ${VAR} variables. Defines tool-use guidance, coding guidelines, working environment, skills, and AGENTS.md injection points.
  - source: system_prompt_args.ROLE_ADDITIONAL
    mode: append
    scope:
      - session
    order_notes: Rendered near the top of system.md, immediately after the agent identity paragraph and before tool-use instructions.
    notes: The cleanest append point when extending default via a custom agent YAML. No native flag exists to set this directly.
  - source: AGENTS.md
    mode: append
    scope:
      - repo
    order_notes: Injected in the Project Information section, after working environment details and before skills.
    notes: Merged from project root down to working directory. Deeper files override shallower ones under a 32 KiB cap.
  - source: Discovered skills
    mode: append
    scope:
      - user
      - repo
      - system
    order_notes: Injected in the Skills section, grouped by scope.
    notes: AI decides when to read individual SKILL.md files. Skills list itself is always in the system prompt.
  - source: Additional directories info
    mode: append
    scope:
      - session
    order_notes: Injected in Working Environment section when --add-dir is used.
    notes: Rendered via ${KIMI_ADDITIONAL_DIRS_INFO}.
  - source: Mode-specific injections
    mode: modify
    scope:
      - session
    order_notes: AFK mode may inject a system reminder. Plan mode restricts tools and injects plan constraints. YOLO no longer injects guidance as of v1.40.0.
    notes: Root-only as of v1.42.0; subagents do not receive plan/afk workflow injections.
  - source: Subagent system prompt
    mode: replace
    scope:
      - subagent
    order_notes: Each subagent loads its own agent YAML system_prompt_path in an isolated context.
    notes: Built-in subagent types (coder, explore, plan) have restricted tool lists. Subagents cannot nest the Agent tool.
agent_prompting:
  supported: true
  definition_surface: YAML agent files with Markdown system prompt files
  inheritance: extend directive recursively merges base agent config; omitted fields inherit, provided fields override. system_prompt_args are merged.
  isolation: Each subagent runs in its own context history under session/subagents/<agent_id>/ and returns a summary to the parent.
  limitations: Subagents cannot create their own subagents. Built-in explore/plan types skip some root workflow tools. Agent tool only available to root agent.
claudine_delivery:
  append_strategy: agent_spec
  replace_strategy: agent_spec
  temp_file_required: true
  argv_limit: No published argv limit documented; use temp files for agent YAML and system prompt Markdown.
  notes: >-
    For append, create a temporary agent YAML that extends default and sets system_prompt_args.ROLE_ADDITIONAL to the resolved append content.
    This avoids mutating user AGENTS.md or config.toml. For replace, create a temporary agent YAML with system_prompt_path pointing to a temporary Markdown file containing the full replacement prompt.
    Pass the temp YAML via --agent-file. Clean up temp files on exit.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: >-
    The native system prompt is Markdown with ${VAR} variable substitution and Jinja2-style conditionals, so appended instructions should be plain Markdown that blends into the existing structure.
    For replacements, the caller must supply all structure that the default prompt provided; XML-wrapped Markdown helps the model distinguish identity, rules, constraints, and examples.
recent_changes:
  - date: "2026-06-05"
    version: "1.47.0"
    change: Migration nudges to the new standalone Kimi Code; /upgrade command added.
    impact: The kimi-cli project is being wound down; future prompt mechanisms may differ in Kimi Code.
  - date: "2026-05-12"
    version: "1.43.0"
    change: Plan-mode and AFK-mode workflow prompts are no longer injected into subagents; they are root-only.
    impact: Subagent system prompts are cleaner but still share session-level mode state for persistence.
  - date: "2026-04-30"
    version: "1.40.0"
    change: YOLO mode no longer injects a non-interactive system reminder; AFK mode handles unattended execution.
    impact: System prompt injections now cleanly distinguish --yolo (approval bypass, user present) from --afk (unattended).
  - date: "2026-04-22"
    version: "1.39.0"
    change: Skills are grouped by scope in the system prompt and merge_all_available_skills defaults to true.
    impact: More skills may appear in the system prompt by default; same-name resolution follows Project > User > Extra > Built-in.
  - date: "2026-03-30"
    version: "1.28.0"
    change: Lifecycle hooks system added (Beta) with 13 events including PreToolUse, PostToolUse, SessionStart, SubagentStart.
    impact: Hooks can inspect or block tool calls but do not directly modify the system prompt text.
  - date: "2026-03-25"
    version: "1.27.0"
    change: System prompt strengthened to encourage tool use for coding tasks.
    impact: Default behavior leans more heavily toward taking action with tools.
quirks:
  - Kimi Code CLI has no native --system-prompt, --append-system-prompt, or --append-system-prompt-file flags.
  - The default system prompt is a Jinja2/Moustache-style template (system.md) that injects variables such as ${KIMI_AGENTS_MD}, ${KIMI_SKILLS}, and ${KIMI_ADDITIONAL_DIRS_INFO}.
  - AGENTS.md content is injected into the system prompt template, not loaded as a separate user/context message.
  - The ROLE_ADDITIONAL system_prompt_args variable is the only documented extension point for adding instructions without copying the entire default system.md.
  - When extending default in a custom agent YAML, any provided field overrides the inherited value; there is no field-level merge except system_prompt_args.
  - Skills directories from multiple agent brands (.kimi, .claude, .codex) are merged by default, which can add unexpected content to the system prompt.
  - The project is being gradually wound down in favor of the new standalone Kimi Code (kimi-code repo); prompt behavior may diverge in the successor.
  - Subagents run isolated but cannot recursively spawn their own subagents.
gaps:
  - No documented CLI flag or command exports the fully resolved built-in system prompt as plain text. kimi vis renders a _system_prompt card, but this is not a textual export.
  - No official way exists to append instructions at the end of the system prompt without mutating AGENTS.md or copying the default system.md content.
  - It is unclear whether --add-dir directories are scanned for AGENTS.md or only contribute ${KIMI_ADDITIONAL_DIRS_INFO}.
  - The migration path to the new Kimi Code may change or remove the current agent-spec YAML mechanism.
changes:
  - Refreshed research against Kimi CLI v1.48.0 documentation and source.
  - Documented agent-spec-based append/replace strategy for Claudine using ROLE_ADDITIONAL and system_prompt_path.
  - Corrected prior overstatement that AGENTS.md is the canonical append path; it mutates project state.
requires_claudine_update: true
reason: Kimi Code CLI lacks native --append-system-prompt/--replace-system-prompt flags. Claudine must implement delivery via temporary agent-spec YAML files, using system_prompt_args.ROLE_ADDITIONAL for append and system_prompt_path for replace.
---

## Overview

Kimi Code CLI (the `kimi` Python package from Moonshot AI) does not expose a direct `--system-prompt` or `--append-system-prompt` flag. Instead, it builds the effective system prompt from a layered, template-driven pipeline:

1. A built-in Markdown template (`system.md`) shipped with the package.
2. Optional variable substitutions such as `${ROLE_ADDITIONAL}`, `${KIMI_AGENTS_MD}`, `${KIMI_SKILLS}`, and `${KIMI_ADDITIONAL_DIRS_INFO}`.
3. Project-level `AGENTS.md` files discovered from the git root down to the working directory.
4. Discovered Agent Skills.
5. Mode-specific injections (plan mode, AFK mode).
6. Custom agent specifications loaded via `--agent-file`.

This means Claudine cannot pass a file directly to a native append/replace flag. The wrapper must generate a temporary agent-spec YAML file that either extends the built-in `default` agent and injects additional instructions, or replaces the system prompt entirely by pointing `system_prompt_path` at a temporary Markdown file.

Important transition note: the `kimi-cli` repository is being wound down in favor of a new standalone [Kimi Code](https://github.com/MoonshotAI/kimi-code) project. Installing Kimi Code migrates configuration and sessions automatically. The findings below apply to the documented `kimi-cli` behavior.

## CLI Parameters

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--agent-file <path>` | Replace | Loads a custom agent YAML. Its `system_prompt_path` becomes the session system prompt. |
| `--agent <name>` | Other | Selects a built-in agent (`default` or `okabe`), each with a hardcoded prompt and toolset. |
| `--config-file <path>` | Modify | Loads an alternate `~/.kimi/config.toml`; can change hooks, skills, and mode defaults that affect prompt injections. |
| `--config <string>` | Modify | Passes TOML/JSON config inline. |
| `--skills-dir <path>` | Modify | Adds skill directories; skills are injected into the system prompt. Repeatable. |
| `--add-dir <path>` | Modify | Adds workspace directories; rendered as `${KIMI_ADDITIONAL_DIRS_INFO}` in the system prompt. |
| `--thinking` / `--no-thinking` | Modify | Toggles reasoning mode; does not change system prompt text. |
| `--plan` | Modify | Starts in plan mode with tool restrictions and plan-mode constraints. |
| `--yolo` / `--afk` | Modify | Auto-approves tool calls. AFK injects a system reminder; YOLO no longer does as of v1.40.0. |

`--agent` and `--agent-file` are mutually exclusive. There is no equivalent of `--append-system-prompt-file`.

## Configuration and Discovery

### Agent specification files

Agents are defined in YAML. A minimal custom agent that replaces the system prompt looks like:

```yaml
version: 1
agent:
  name: my-agent
  system_prompt_path: ./system.md
  tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:ReadFile"
```

To keep the default toolset while replacing or extending the prompt, use `extend: default`:

```yaml
version: 1
agent:
  extend: default
  system_prompt_path: ./my-prompt.md
```

### `AGENTS.md` hierarchy

`AGENTS.md` files are discovered from the git project root down to the working directory, including `.kimi/AGENTS.md` at each level. Their merged content is injected into the built-in `system.md` template via `${KIMI_AGENTS_MD}`. Deeper files take precedence under a 32 KiB budget cap. This is the closest native equivalent to appending project instructions, but it requires writing files into the project tree.

### Skills

Skills are directories containing `SKILL.md`. Kimi Code CLI discovers them from user, project, and extra directories (including `.claude/skills/` and `.codex/skills/`). As of v1.39.0, all existing brand directories are merged by default (`merge_all_available_skills = true`). The list of discovered skills is injected into the system prompt; the AI decides when to read an individual `SKILL.md`.

### Configuration file

`~/.kimi/config.toml` controls hooks, skills directories, loop control, and mode defaults. It does not contain a free-form system prompt key.

## Prompt Layers and Precedence

The final system prompt is assembled from the following layers, from most foundational to most specific:

```mermaid
graph TD
    A[Built-in system.md template] --> B{system_prompt_args.ROLE_ADDITIONAL?}
    B -->|yes| C[ROLE_ADDITIONAL inserted near top]
    B -->|no| D[Default identity paragraph]
    C --> E[Tool use and coding guidelines]
    D --> E
    E --> F[Working environment: KIMI_OS, KIMI_SHELL, KIMI_WORK_DIR, KIMI_WORK_DIR_LS]
    F --> G[Project Information: merged AGENTS.md via KIMI_AGENTS_MD]
    G --> H[Skills list via KIMI_SKILLS]
    H --> I[Ultimate reminders]
    I --> J[Mode-specific injections: AFK, plan]
```

Notes:

- `--agent-file` with a custom `system_prompt_path` replaces layer A entirely. The custom file can still reference variables like `${KIMI_AGENTS_MD}` and `${KIMI_SKILLS}` if desired.
- `ROLE_ADDITIONAL` is the only built-in variable designed for arbitrary additional instructions.
- `AGENTS.md` and skills are injected into the template, not appended as separate context messages.

## Agents and Subagents

Kimi Code CLI supports custom agents and subagents through YAML specifications.

- Custom agents are loaded with `--agent-file`. The Markdown file referenced by `system_prompt_path` becomes the agent's system prompt.
- `extend: default` inherits the built-in default agent's tools and other fields; overridden fields replace inherited ones. `system_prompt_args` are merged.
- Subagents are defined under the `subagents` key and launched via the `Agent` tool. Each subagent has its own system prompt and isolated context history.
- Built-in subagent types (`coder`, `explore`, `plan`) have restricted tool lists.
- As of v1.42.0, plan-mode and AFK-mode workflow prompt injections are root-only; subagents share session mode state but do not receive those injections.
- Subagents cannot create their own subagents.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Plain Markdown | The native template is Markdown with `${VAR}` substitutions; Markdown headers and lists blend cleanly with the existing structure. |
| Replace | XML-wrapped Markdown | When the default template is replaced entirely, XML tags (`<identity>`, `<rules>`, `<constraints>`, `<examples>`) help preserve the structure that the model expects. |

Because a replacement drops the built-in tool-use guidance, coding conventions, and environment context, the replacement prompt must explicitly supply whatever the task still needs.

## Recent Changes

- **v1.47.0 (2026-06-05)**: Added migration nudges to the new standalone Kimi Code, including `/upgrade`.
- **v1.43.0 (2026-05-12)**: Plan-mode and AFK-mode workflow prompts are no longer injected into subagents.
- **v1.40.0 (2026-04-30)**: YOLO mode no longer injects a non-interactive system reminder; AFK mode handles unattended execution.
- **v1.39.0 (2026-04-22)**: Skills are grouped by scope in the system prompt and `merge_all_available_skills` defaults to `true`.
- **v1.28.0 (2026-03-30)**: Added lifecycle hooks (Beta) with 13 events.
- **v1.27.0 (2026-03-25)**: System prompt strengthened to encourage tool use for coding tasks.

## Quirks and Workarounds

- There is no native `--system-prompt` or `--append-system-prompt` flag. Developers who need ad-hoc prompt changes typically create a temporary agent YAML and pass it with `--agent-file`.
- The built-in `system.md` is a template, not a static string. Variables like `${KIMI_AGENTS_MD}` are filled at runtime.
- `ROLE_ADDITIONAL` is the cleanest append point when extending `default`, but it places content near the top of the prompt, not the end.
- To append at the end of the prompt without mutating the project, the only robust path is to copy the default `system.md` content into a custom `system_prompt_path` file and add instructions at the end.
- Skills from `.claude/skills/` and `.codex/skills/` can be merged into the Kimi prompt by default, which may surprise users migrating from other agents.
- `--add-dir` expands workspace scope but it is not documented whether it also causes `AGENTS.md` discovery in added directories.

## Claudine Delivery Notes

Claudine should deliver `--append-system-prompt` and `--replace-system-prompt` through temporary agent specifications:

- **Append**: Create a temporary agent YAML that extends `default` and sets `system_prompt_args.ROLE_ADDITIONAL` to the resolved append content. Pass the YAML via `--agent-file`. This avoids mutating the user's `AGENTS.md` or `config.toml`.
- **Replace**: Create a temporary agent YAML with `system_prompt_path` pointing to a temporary Markdown file containing the full replacement prompt. Pass the YAML via `--agent-file`.
- Both modes require temp files and cleanup on exit.
- Because the default system prompt template is not exposed through a CLI export, Claudine cannot automatically prepend the default content for an end-of-prompt append. `ROLE_ADDITIONAL` append is the practical default.

## Changelog

- Refreshed research against Kimi CLI v1.48.0 documentation and source.
- Documented agent-spec-based append/replace strategy for Claudine using `system_prompt_args.ROLE_ADDITIONAL` and `system_prompt_path`.
- Corrected prior overstatement that `AGENTS.md` is the canonical append path; it mutates project state.

## Sources

- [Kimi Code CLI documentation](https://moonshotai.github.io/kimi-cli/en/)
- [Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents.md)
- [Agent Skills](https://moonshotai.github.io/kimi-cli/en/customization/skills.md)
- [Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Config Overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.md)
- [`kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- [Print Mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.md)
- [Hooks (Beta)](https://moonshotai.github.io/kimi-cli/en/customization/hooks.md)
- [Sessions and Context](https://moonshotai.github.io/kimi-cli/en/guides/sessions.md)
- [Changelog](https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.md)
- [Kimi CLI source: `agentspec.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agentspec.py)
- [Kimi CLI source: `agents/default/agent.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/agent.yaml)
- [Kimi CLI source: `agents/default/system.md`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/system.md)
- [Kimi CLI GitHub repository](https://github.com/MoonshotAI/kimi-cli)

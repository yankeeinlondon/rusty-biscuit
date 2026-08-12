---
$schema: ./_schema.yaml
created: '2026-07-03'
last_updated: '2026-07-03'
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
system_prompt_docs: https://kilo.ai/docs/customize/custom-instructions
append_support: env
replace_support: agent_spec
cli_params:
  - flag: kilo run --agent <AGENT>
    mode: replace
    value_shape: agent name
    description: Selects the active agent for the run; the selected agent's prompt is included as that agent's system prompt layer.
    example: kilo run --agent claudine-replacement "Implement the requested change"
    notes: There is no direct system-prompt flag. Use with a temporary agent definition injected through KILO_CONFIG_CONTENT or KILO_CONFIG_DIR to avoid mutating user config.
  - flag: kilo run --model <PROVIDER/MODEL>
    mode: modify
    value_shape: provider/model string
    description: Selects the model for the run, which can indirectly select a model-family built-in prompt.
    example: kilo run --model anthropic/claude-sonnet-4-20250514 "Review this code"
    notes: Source selects built-in prompt text by model prompt metadata and model id family; this is not a prompt-control flag.
  - flag: kilo run --format json
    mode: inspect
    value_shape: enum value
    description: Streams raw session events as JSON.
    example: kilo run --format json "Say hello"
    notes: Useful for observing events, but current evidence does not show it exporting the effective system prompt.
  - flag: kilo run --file <PATH>
    mode: other
    value_shape: file path, repeatable
    description: Attaches files to the user message.
    example: kilo run --file ./AGENTS.md "Use this context"
    notes: This affects user-message context, not the system prompt.
  - flag: kilo agent create --path <DIR> --description <TEXT> --mode <MODE> --permissions <LIST>
    mode: replace
    value_shape: directory path plus generated agent metadata
    description: Creates an agent Markdown file whose body is the agent system prompt.
    example: kilo agent create --path /tmp/kilo-profile --description "Temporary wrapper agent" --mode primary --permissions read,grep,glob
    notes: Not recommended for Claudine delivery because it generates prompt text through an LLM and writes persistent files; useful evidence that agent Markdown body is the prompt surface.
  - flag: kilo agent list
    mode: inspect
    value_shape: command
    description: Lists available agents and permissions.
    example: kilo agent list
    notes: It does not print agent prompt text; session-export tests also show exported agent info omits prompt/options/permissions.
  - flag: /agents
    mode: replace
    value_shape: interactive slash command
    description: Opens the interactive agent switcher in the TUI.
    example: /agents
    notes: Interactive only; not suitable for non-interactive wrapper delivery.
  - flag: /init
    mode: append
    value_shape: interactive slash command
    description: Creates or updates project AGENTS.md.
    example: /init
    notes: Persistent project mutation; not suitable for ephemeral Claudine prompt injection.
config_sources:
  - os: macos
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Global config; can define instructions, default_agent, and agent.<name>.prompt. Source also loads kilo.json, opencode.json, opencode.jsonc, and config.json in this directory.
  - os: linux
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Linux global config equivalent; same prompt-related keys as macOS.
  - os: windows
    scope: user
    path: C:\Users\<username>\.config\kilo\kilo.jsonc
    mode: modify
    format: jsonc
    notes: Windows global config path documented by Kilo; same prompt-related keys.
  - os: macos
    scope: user
    path: ~/.config/kilo/AGENTS.md
    mode: append
    format: markdown
    notes: Global instruction file. Source also checks ~/.claude/CLAUDE.md unless KILO_DISABLE_CLAUDE_CODE_PROMPT is set.
  - os: linux
    scope: user
    path: ~/.config/kilo/AGENTS.md
    mode: append
    format: markdown
    notes: Linux global instruction file.
  - os: windows
    scope: user
    path: C:\Users\<username>\.config\kilo\AGENTS.md
    mode: append
    format: markdown
    notes: Windows global instruction file under Kilo's documented config root.
  - os: macos
    scope: user
    path: ~/.kilo/agent/*.md
    mode: replace
    format: markdown
    notes: Home config directory discovered from Global.Path.home; agent Markdown frontmatter carries metadata and body carries prompt.
  - os: linux
    scope: user
    path: ~/.kilo/agent/*.md
    mode: replace
    format: markdown
    notes: Linux equivalent. Legacy ~/.kilocode/agent/*.md is also scanned.
  - os: windows
    scope: user
    path: C:\Users\<username>\.kilo\agent\*.md
    mode: replace
    format: markdown
    notes: Windows equivalent. Legacy .kilocode agent directories are also scanned.
  - os: macos
    scope: repo
    path: ./kilo.jsonc
    mode: modify
    format: jsonc
    notes: Project config; can add instructions, set default_agent, or define/override agent.<name>.prompt.
  - os: linux
    scope: repo
    path: ./kilo.jsonc
    mode: modify
    format: jsonc
    notes: Linux project config equivalent.
  - os: windows
    scope: repo
    path: .\kilo.jsonc
    mode: modify
    format: jsonc
    notes: Windows project config equivalent.
  - os: macos
    scope: repo
    path: ./.kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Project config directory file; source also scans .kilocode as a legacy fallback.
  - os: linux
    scope: repo
    path: ./.kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Linux project config directory equivalent.
  - os: windows
    scope: repo
    path: .\.kilo\kilo.jsonc
    mode: modify
    format: jsonc
    notes: Windows project config directory equivalent.
  - os: macos
    scope: repo
    path: ./AGENTS.md
    mode: append
    format: markdown
    notes: Auto-discovered project instruction file. Source checks AGENTS.md, then CLAUDE.md unless disabled, then CONTEXT.md; first matching filename family wins from the project search.
  - os: linux
    scope: repo
    path: ./AGENTS.md
    mode: append
    format: markdown
    notes: Linux project instruction equivalent.
  - os: windows
    scope: repo
    path: .\AGENTS.md
    mode: append
    format: markdown
    notes: Windows project instruction equivalent.
  - os: macos
    scope: repo
    path: ./.kilo/agent/*.md
    mode: replace
    format: markdown
    notes: Project agent definitions. Body is the prompt; YAML frontmatter supplies description, mode, model, permissions, steps, and related metadata.
  - os: linux
    scope: repo
    path: ./.kilo/agent/*.md
    mode: replace
    format: markdown
    notes: Linux project agent definition equivalent.
  - os: windows
    scope: repo
    path: .\.kilo\agent\*.md
    mode: replace
    format: markdown
    notes: Windows project agent definition equivalent.
  - os: macos
    scope: system
    path: /Library/Application Support/kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Managed enterprise config directory; loaded after user, project, KILO_CONFIG_DIR, KILO_CONFIG_CONTENT, and active org config.
  - os: linux
    scope: system
    path: /etc/kilo/kilo.jsonc
    mode: modify
    format: jsonc
    notes: Linux managed enterprise config directory.
  - os: windows
    scope: system
    path: '%ProgramData%\kilo\kilo.jsonc'
    mode: modify
    format: jsonc
    notes: Windows managed enterprise config directory.
  - os: macos
    scope: system
    path: /Library/Managed Preferences/<user>/ai.opencode.managed.plist
    mode: modify
    format: json
    notes: macOS MDM managed preferences override regular config. Domain name remains ai.opencode.managed in source.
env_vars:
  - name: KILO_CONFIG_CONTENT
    effect: Inline JSON/JSONC config string merged late as local config; can define temporary instructions, default_agent, and agent.<name>.prompt.
    mode: modify
  - name: KILO_CONFIG
    effect: Path to an additional config file loaded after global config and before project config.
    mode: modify
  - name: KILO_CONFIG_DIR
    effect: Adds a config directory to discovery and makes AGENTS.md in that directory the first global instruction candidate.
    mode: modify
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: Skips project-level config files, config directories, and project instruction discovery.
    mode: disable
  - name: KILO_DISABLE_CLAUDE_CODE_PROMPT
    effect: Disables CLAUDE.md compatibility prompt loading while keeping AGENTS.md and CONTEXT.md discovery.
    mode: disable
  - name: KILO_DISABLE_CLAUDE_CODE
    effect: Broadly disables Claude Code compatibility surfaces, including CLAUDE.md prompt loading.
    mode: disable
  - name: KILO_PURE
    effect: Runs without external plugins, which disables plugin-based system prompt transforms.
    mode: disable
  - name: KILO_CLIENT
    effect: Marks client surface such as cli, vscode, or jetbrains; affects client-specific reminders but is not a direct prompt override.
    mode: other
prompt_layers:
  - source: model-family built-in prompt
    mode: append
    scope:
      - builtin
    order_notes: Source selects prompt text by provider model metadata or model id family before the rest of the request is built.
    notes: Current source imports default, Anthropic, Beast, Gemini, GPT, GPT-5.5, Kimi, Ling, Codex, and Trinity prompt text. No public CLI export for the effective built-in prompt was found.
  - source: dynamic Kilo environment block
    mode: append
    scope:
      - builtin
    order_notes: Prepended before instruction files in the request system array.
    notes: Includes model id, git-repo state, platform, date, project config hints, global config root, and editor context.
  - source: instruction files
    mode: append
    scope:
      - user
      - repo
    order_notes: 'Added after environment and before skills. Source wraps each file as "Instructions from: <path>\n<content>".'
    notes: Includes global/project AGENTS.md, CLAUDE.md compatibility unless disabled, CONTEXT.md, and paths/URLs from config.instructions.
  - source: skills inventory
    mode: append
    scope:
      - user
      - repo
      - agent
    order_notes: Added after instructions when the active agent has skill permission.
    notes: Kilo docs state skill metadata is included in the system prompt and the agent can load full skill content on demand.
  - source: structured output reminder
    mode: append
    scope:
      - other
    order_notes: Appended after skills only when the user requested JSON schema output.
    notes: Forces the StructuredOutput tool as the final response path.
  - source: agent prompt
    mode: replace
    scope:
      - agent
      - subagent
    order_notes: Agent prompt is not assembled in the same array snippet as environment/instructions, but docs and schema define agent Markdown body or agent.<name>.prompt as the agent system prompt.
    notes: Selecting a custom agent is the provider-native way to replace the active agent behavioral prompt layer without changing built-in Kilo source.
  - source: per-directory AGENTS.md
    mode: append
    scope:
      - repo
    order_notes: Not preloaded at session start; injected when Read accesses files under the directory.
    notes: Injected into the conversation as system-reminder text and de-duplicated per assistant message.
  - source: plugin experimental.chat.system.transform
    mode: modify
    scope:
      - extension
    order_notes: Triggered by LLM request code to modify the system prompt array.
    notes: Experimental plugin API; powerful but not appropriate as Claudine's default because it requires loading a plugin into Kilo.
agent_prompting:
  supported: true
  definition_surface: Agent Markdown files with YAML frontmatter, agent.<name>.prompt in kilo.jsonc, Settings UI, organization-managed modes, and KILO_CONFIG_CONTENT overlays.
  inheritance: Agent definitions merge by name across built-in, global, project, config-directory, and environment sources; higher-priority fields override lower-priority fields. Subagents are separate agents invoked by the task tool, and source code also inherits selected permission restrictions from the calling agent/session.
  isolation: Task-tool subagents run in child sessions with their own selected agent prompt and tool permissions. Kilo disallows nested task spawning and returns subtask results to the parent session.
  limitations: No direct CLI flag was found to set an arbitrary one-shot system prompt. Replacing only agent.<name>.prompt does not remove Kilo's model-family built-in prompt, environment block, instruction files, tool descriptions, or plugin transforms.
claudine_delivery:
  append_strategy: env_var_file
  replace_strategy: agent_spec
  temp_file_required: true
  argv_limit: KILO_CONFIG_CONTENT can carry small inline JSON, but use a temporary config directory/file plus instruction or agent prompt files for long prompts to avoid environment size and shell quoting limits.
  notes: For append, create a temporary config file or KILO_CONFIG_CONTENT that adds an instructions entry pointing at a temporary Markdown file, optionally with KILO_DISABLE_PROJECT_CONFIG=1 only when Claudine intentionally wants to suppress user/project prompt layers. For replace, define a temporary primary agent with agent.<name>.prompt or an agent Markdown file, set default_agent or pass kilo run --agent <name>, and use KILO_CONFIG_CONTENT/KILO_CONFIG_DIR so user config is not permanently mutated. There is no proven direct replacement of the built-in model prompt.
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: Official docs recommend Markdown for rules, AGENTS.md, and agent Markdown bodies. Kilo wraps dynamically discovered directory instructions in system-reminder tags itself, so Claudine should not XML-wrap ordinary append files unless deliberately creating reminder-style text.
recent_changes:
  - date: '2026-07-03'
    version: 7.4.1
    change: Latest npm package observed during research is @kilocode/cli 7.4.1.
    impact: Research reflects the current OpenCode-derived CLI rather than pre-1.0 or legacy extension-only behavior.
  - date: '2026-02-23'
    version: 7.0.26
    change: npm version series jumped from 1.0.x to 7.0.x.
    impact: Public docs warn that CLI docs apply to Kilo 1.0 and later; third-party or older issue behavior around .kilocode/system-prompt-* may not apply to current CLI.
  - date: unknown
    version: unknown
    change: Current docs describe agent Markdown files and kilo.jsonc agent entries as the custom-mode mechanism, replacing legacy custom_modes.yaml/.kilocodemodes workflows.
    impact: Claudine should target kilo.jsonc/KILO_CONFIG_CONTENT/agent Markdown surfaces, not legacy custom_modes.yaml.
quirks:
  - The current Kilo CLI has no documented or source-visible --append-system-prompt, --replace-system-prompt, --system-prompt, or --system-prompt-file equivalent.
  - Kilo docs say project-level instructions load before global instructions, while the current source systemPaths set adds the first global instruction candidate before project paths; conflict handling should not depend on natural-language priority alone.
  - The docs page for AGENTS.md says AGENT.md fallback is supported, but current source instruction filename list shows AGENTS.md, CLAUDE.md, and CONTEXT.md. Treat AGENT.md support as unverified for the current CLI source path.
  - KILO_CONFIG_CONTENT is the best ephemeral override, but it is an environment variable; large prompts should be passed through files referenced by inline config.
  - KILO_CONFIG_DIR does not replace the normal XDG global config everywhere; it is added to config-directory scanning and also influences one global AGENTS.md candidate.
  - Session export intentionally omits agent prompt/options/permissions, so exported sessions are not a reliable prompt-inspection mechanism.
  - Plugin hook experimental.chat.system.transform can modify the system prompt array, but it is experimental and requires plugin loading; KILO_PURE disables external plugins.
  - Local ~/.kilo inspection on this host found no files, so no user-specific examples were available.
gaps:
  - Did not execute an authenticated model call, so prompt assembly was verified from docs and source rather than by capturing a live provider request.
  - No official CLI command was found to print the fully assembled effective system prompt.
  - Exact internal placement of agent.prompt relative to model-family prompt is inferred from docs and schema; the final provider request path should be rechecked if implementing prompt-perfect replacement.
  - Current source/doc discrepancy around AGENT.md fallback needs follow-up before depending on AGENT.md.
changes: []
requires_claudine_update: true
reason: Kilo needs a provider-specific SystemPromptSpec that uses config/env/file delivery rather than native append/replace flags, and replacement should be represented as an agent-spec strategy with a caveat that the built-in model prompt is not directly removable.
---

# Kilo Code System Prompt Research

## Overview

Kilo Code's current CLI is an OpenCode-derived runtime with Kilo-specific configuration, agent, and prompt assembly. The important implementation fact for a wrapper is that Kilo does not expose Claude-style `--append-system-prompt` or `--replace-system-prompt` flags in the current `kilo run` command. Prompt control is delivered through configuration layers: instruction files append to the effective system context, and custom agents provide a distinct agent prompt.

For Claudine, append is practical and non-mutating through `KILO_CONFIG_CONTENT` or `KILO_CONFIG_DIR` pointing at a temporary instruction file. Replacement is only partial: Claudine can select a temporary custom agent whose prompt replaces the active agent's behavioral prompt, but current evidence does not show a supported way to remove Kilo's model-family built-in prompt, environment block, tool descriptions, or other runtime prompt layers.

Local inspection found no files under `~/.kilo` on this host, so there were no local user Kilo prompt/config examples to incorporate.

## CLI Parameters

| Parameter | Prompt Effect | Wrapper Use |
|---|---|---|
| `kilo run --agent <AGENT>` | Selects the active agent; custom agent prompt becomes the selected agent prompt. | Use with a temporary agent injected by config for replacement-style delivery. |
| `kilo run --model <PROVIDER/MODEL>` | Changes model selection and can indirectly change the built-in model-family prompt. | Not a direct prompt-control surface. |
| `kilo run --format json` | Streams raw events. | Useful for automation, but not a proven effective-prompt export. |
| `kilo agent create ...` | Creates a persistent agent Markdown file whose body is prompt text. | Avoid for Claudine; it writes files and may use an LLM to generate the prompt. |
| `kilo agent list` | Lists agents and permissions. | Does not expose prompt text. |
| `/agents` | Interactive agent switcher. | Not suitable for non-interactive wrappers. |
| `/init` | Creates or updates `AGENTS.md`. | Persistent mutation; not suitable for ephemeral prompt append. |

No current official docs or source path showed native flags named `--append-system-prompt`, `--replace-system-prompt`, `--system-prompt`, or `--system-prompt-file` for Kilo CLI 7.4.1.

## Configuration and Discovery

Kilo prompt-relevant config is JSONC-centric:

| Source | Effect |
|---|---|
| `agent.<name>.prompt` in `kilo.jsonc` | Defines or overrides an agent prompt. |
| Agent Markdown files under `agent/` or `agents/` directories | YAML frontmatter defines metadata; Markdown body is the prompt. |
| `instructions` array in `kilo.jsonc` | Adds instruction files, globs, or URLs. |
| `AGENTS.md` | Primary global/project instruction file. |
| `CLAUDE.md` | Compatibility instruction file unless disabled. |
| `CONTEXT.md` | Deprecated additional context file. |
| `KILO_CONFIG_CONTENT` | Late inline config override; best ephemeral wrapper hook for short config. |
| `KILO_CONFIG_DIR` | Adds a temporary config directory and can provide an `AGENTS.md` candidate. |
| `KILO_DISABLE_PROJECT_CONFIG=1` | Suppresses project config/instruction discovery. |

Config precedence is high enough that `KILO_CONFIG_CONTENT` can define a temporary agent and set `default_agent`, or add a temporary path to `instructions`. Managed enterprise config and macOS managed preferences still override it.

## Prompt Layers and Precedence

The source constructs the model request system array from environment, instruction, skill, and optional structured-output layers. Built-in model-family prompt text is selected separately by model metadata/id family.

```mermaid
flowchart TD
    A[Model-family built-in prompt] --> B[Kilo environment block]
    B --> C[Instruction files]
    C --> D[Skills inventory]
    D --> E[Optional structured-output reminder]
    F[Selected agent prompt] --> A
    G[Per-directory AGENTS.md] --> H[Later system-reminder after Read]
    I[Plugin system transform] --> B
```

Important wrapper implications:

- Instruction files append; they do not replace the provider base.
- Agent prompts give each primary agent or subagent distinct behavior.
- Per-directory `AGENTS.md` files are injected later when relevant files are read.
- Plugin transforms can mutate the system array, but are experimental and require plugin loading.

## Agents and Subagents

Kilo supports user-defined primary agents and subagents. Agent definitions can live in Markdown files or in `kilo.jsonc`; the Markdown body or `agent.<name>.prompt` is the prompt text. Agent mode controls availability: `primary`, `subagent`, or `all`.

Subagents are invoked through the task tool in child sessions. Current source rejects primary-only agents as subagents and disallows nested task spawning. Child sessions inherit selected permission restrictions from the parent/calling agent, but their agent prompt is distinct.

For Claudine replacement delivery, define a temporary `primary` or `all` agent and select it with `--agent` or `default_agent`. This replaces the selected agent prompt layer, not every Kilo system layer.

## Format Recommendations

Use Markdown for both append and replacement:

- Append: a temporary Markdown instruction file referenced from `instructions`.
- Replace: a temporary agent Markdown body or `agent.<name>.prompt` string containing Markdown.

Kilo's docs explicitly recommend Markdown for rules and use Markdown agent files as a native format. XML-wrapped Markdown is not required; Kilo itself wraps dynamically discovered directory instructions in `<system-reminder>` tags when it injects them later.

## Recent Changes

| Date | Version | Change | Impact |
|---|---|---|---|
| 2026-07-03 | 7.4.1 | Latest npm version observed during research. | Research targets current CLI behavior. |
| 2026-02-23 | 7.0.26 | npm version series moved from 1.0.x to 7.x. | Older Kilo/Roo-style system prompt override issues should not be treated as current CLI behavior without source confirmation. |
| unknown | unknown | Current docs use agent Markdown files and `kilo.jsonc` for custom modes. | Wrappers should target the new config/agent surfaces, not legacy `custom_modes.yaml`. |

## Quirks and Workarounds

- There is no supported direct one-shot system prompt replacement flag in current Kilo CLI.
- `KILO_CONFIG_CONTENT` is powerful but should carry only small JSON; for long prompts use temp files.
- `KILO_DISABLE_PROJECT_CONFIG=1` is useful for isolation, but it also removes legitimate user/repo context.
- Kilo docs and source disagree on `AGENT.md` fallback support; rely on `AGENTS.md`.
- Session export omits agent prompt/options/permissions, so it is not prompt inspection.
- A historical issue discusses `.kilocode/system-prompt-{mode}` behavior, but that path was not present in the current CLI prompt path researched here.

## Claudine Delivery Notes

Append without persistent mutation:

1. Write a temporary Markdown file containing Claudine's appended instructions.
2. Set `KILO_CONFIG_CONTENT` to JSON that adds that file to `instructions`.
3. Run `kilo run ...` normally.
4. Avoid `KILO_DISABLE_PROJECT_CONFIG` unless the user requested isolation from repo/user instructions.

Replacement without persistent mutation:

1. Write a temporary agent prompt as Markdown or inline JSON.
2. Set `KILO_CONFIG_CONTENT` with `agent.claudine-replacement.prompt`, `description`, and `mode: "primary"`.
3. Pass `kilo run --agent claudine-replacement ...` or set `default_agent`.
4. Document that this replaces the active agent prompt layer, not the hard-coded Kilo base/environment/tool layers.

For very long prompts, prefer `KILO_CONFIG_DIR` with temporary `kilo.jsonc` and agent/instruction files over embedding prompt text directly in an environment variable.

## Sources

- [Kilo CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo custom instructions documentation](https://kilo.ai/docs/customize/custom-instructions)
- [Kilo custom modes documentation](https://kilo.ai/docs/customize/custom-modes)
- [Kilo custom rules documentation](https://kilo.ai/docs/customize/custom-rules)
- [Kilo AGENTS.md documentation](https://kilo.ai/docs/customize/agents-md)
- [Kilo settings documentation](https://kilo.ai/docs/getting-started/settings)
- [Kilo plugins documentation](https://kilo.ai/docs/automate/extending/plugins)
- [Kilo source repository](https://github.com/Kilo-Org/kilocode)
- [@kilocode/cli npm package](https://www.npmjs.com/package/@kilocode/cli)
- [GitHub issue #4253: custom system prompt override appends unwanted user custom instructions](https://github.com/Kilo-Org/kilocode/issues/4253)

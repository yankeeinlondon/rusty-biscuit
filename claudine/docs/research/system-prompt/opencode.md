---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://opencode.ai/docs
system_prompt_docs: https://opencode.ai/docs/rules
append_support: config
replace_support: agent_spec
cli_params:
  - flag: --agent
    mode: modify
    value_shape: string
    description: Select an agent to run. Agents configured with a custom prompt replace the stock provider prompt for that session.
    example: opencode run "Refactor auth" --agent review
    notes: Does not accept raw system-prompt text. Must reference an agent defined in opencode.json or .opencode/agents/*.md.
  - flag: --model
    mode: other
    value_shape: string
    description: Select the model in provider/model format. Different models may use different stock system prompts.
    example: opencode run "Explain closures" -m anthropic/claude-sonnet-4-5
    notes: Does not directly manipulate the system prompt, but model selection determines which provider-specific stock prompt is used.
  - flag: --pure
    mode: disable
    value_shape: boolean
    description: Run without external plugins.
    example: opencode run --pure "Hello"
    notes: Disables plugin-injected behavior but does not disable AGENTS.md, instructions, or agent prompts.
  - flag: --auto
    mode: modify
    value_shape: boolean
    description: Auto-approve permissions that are not explicitly denied in non-interactive mode.
    example: opencode run --auto "Refactor auth"
    notes: Affects execution policy, not prompt content. Claudine uses an OPENCODE_CONFIG_CONTENT permission overlay for YOLO mode.
config_sources:
  - os: all
    scope: repo
    path: AGENTS.md
    mode: append
    format: markdown
    notes: Project-level instructions. OpenCode traverses up from the current working directory looking for AGENTS.md (and CLAUDE.md as a fallback).
  - os: all
    scope: user
    path: ~/.config/opencode/AGENTS.md
    mode: append
    format: markdown
    notes: Global rules applied across all sessions unless a project AGENTS.md is found.
  - os: all
    scope: repo
    path: opencode.json
    mode: append
    format: json
    notes: Project config. The instructions array lists file paths/globs to include as additional system instructions.
  - os: all
    scope: user
    path: ~/.config/opencode/opencode.json
    mode: append
    format: json
    notes: Global config with instructions array, agent definitions, permissions, and other settings.
  - os: all
    scope: agent
    path: .opencode/agents/*.md
    mode: replace
    format: markdown
    notes: Project agent definitions. Frontmatter prompt field or file body becomes the agent's system prompt and replaces the stock provider prompt.
  - os: all
    scope: agent
    path: ~/.config/opencode/agents/*.md
    mode: replace
    format: markdown
    notes: User-level agent definitions.
env_vars:
  - name: OPENCODE_CONFIG_CONTENT
    effect: Inline JSON config applied session-wide, including parent and subagent sessions. Can carry instructions for append or agent definitions for replace.
    mode: modify
  - name: OPENCODE_CONFIG
    effect: Path to a custom config file loaded between global and project configs.
    mode: modify
  - name: OPENCODE_CONFIG_DIR
    effect: Custom directory searched for agents, commands, modes, and plugins.
    mode: modify
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: Disables all .claude support (prompt and skills).
    mode: disable
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: Disables reading ~/.claude/CLAUDE.md.
    mode: disable
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: Disables loading .claude/skills.
    mode: disable
  - name: OPENCODE_DISABLE_AUTOCOMPACT
    effect: Disables automatic context compaction.
    mode: modify
prompt_layers:
  - source: Stock provider/model prompt
    mode: replace
    scope:
      - session
    order_notes: Base layer; selected by model/provider.
    notes: Not published verbatim. Replaced when a custom agent prompt is configured.
  - source: Agent prompt
    mode: replace
    scope:
      - session
      - subagent
    order_notes: Replaces the stock provider prompt when an agent with a prompt field is active.
    notes: Per GitHub issue #34721, agent.prompt uses replacement semantics and drops the stock provider prompt.
  - source: opencode.json instructions / OPENCODE_CONFIG_CONTENT instructions
    mode: append
    scope:
      - session
    order_notes: Appended after the agent or stock prompt.
    notes: Array of file paths/globs. Loaded from all config sources and merged.
  - source: AGENTS.md / CLAUDE.md
    mode: append
    scope:
      - user
      - repo
    order_notes: Loaded after instructions. Project AGENTS.md wins over global; CLAUDE.md is used only when AGENTS.md is absent.
    notes: Treated as project context / user system text.
  - source: Skills
    mode: append
    scope:
      - session
    order_notes: Skill metadata injected at startup; full SKILL.md loaded on demand via the skill tool.
    notes: Discovered in .opencode/skills, ~/.config/opencode/skills, .claude/skills, ~/.claude/skills, .agents/skills, and ~/.agents/skills.
agent_prompting:
  supported: true
  definition_surface: Markdown files with YAML frontmatter in ~/.config/opencode/agents/ or .opencode/agents/, or JSON agent objects in opencode.json
  inheritance: Subagents use the invoking primary agent's model when omitted; permissions and tools can be overridden per agent.
  isolation: Subagents run in isolated child sessions. Only final summaries return to the parent.
  limitations: Agent prompt replaces the stock provider prompt, not appends to it. No documented additive agent mode as of v1.17.13.
claudine_delivery:
  append_strategy: unsupported
  replace_strategy: agent_spec
  temp_file_required: true
  argv_limit: Not applicable; OpenCode run has no native system-prompt argv flags.
  notes: Append is currently unsupported as a first-class wrapper feature. The intended delivery is OPENCODE_CONFIG_CONTENT with an instructions array pointing to a temp file, but this requires a new SystemPromptDelivery variant. Replace can be delivered by injecting an agent definition into OPENCODE_CONFIG_CONTENT and passing --agent <name>; this replaces only the stock provider prompt layer, not the full effective system prompt.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: Appended instructions blend with AGENTS.md and the stock prompt; plain Markdown headers and lists work best. Agent prompts that replace the stock prompt should be self-contained; XML tags help structure rules, constraints, context, and examples.
recent_changes:
  - date: "2026-07-01"
    version: "v1.17.13"
    change: Latest stable release. No new system-prompt CLI flags.
    impact: Confirms the current CLI surface lacks direct system-prompt append/replace flags.
  - date: "2026-07-01"
    version: unknown
    change: GitHub issue #34721 documented that custom agent prompts replace rather than append to the stock provider prompt.
    impact: Clarifies replacement semantics and requests an additive agent mode for 2.0.
  - date: "2026-06-28"
    version: unknown
    change: Feature request #34341 for progressive path-scoped AGENTS.md loading in V2.
    impact: Future AGENTS.md may load on demand via read-tool plugin context rather than only at startup.
quirks:
  - OpenCode run has no --system, --system-prompt, or --append-system-prompt flag. System-prompt manipulation is config/agent-based only.
  - A custom agent's prompt replaces the provider-specific stock prompt, not the entire effective system prompt; instructions and AGENTS.md may still be appended.
  - AGENTS.md is discovered by walking up from the current working directory; there is no per-run flag to disable it.
  - OPENCODE_CONFIG_CONTENT applies session-wide, including to subagent sessions, so wrapper-injected instructions propagate to all child sessions.
  - The instructions array values are file paths/globs, not inline strings.
  - CLAUDE.md and ~/.claude/CLAUDE.md are read as fallbacks unless OPENCODE_DISABLE_CLAUDE_CODE* is set.
gaps:
  - No documented way to inspect or export the effective built-in system prompt as plain text (issue #26376).
  - Unclear whether config merging concatenates or replaces the instructions array when multiple config sources define it.
  - No native additive agent mode; agent prompts use replacement semantics.
  - No CLI flag to bypass AGENTS.md auto-discovery for a single run.
  - The exact precedence and merge behavior of instructions versus AGENTS.md versus user.system is not fully documented.
changes: []
requires_claudine_update: true
reason: OpenCode's SystemPromptSpec currently marks append and replace as Unsupported. The wrapper needs a new delivery mechanism (likely a Custom tag for OPENCODE_CONFIG_CONTENT instructions overlay) to support append, and agent-spec delivery to support replace.
---

## Overview

OpenCode CLI (Anomaly) does not expose a dedicated system-prompt flag in `opencode run` or the base TUI entrypoint. As of v1.17.13, the only CLI switch that changes the effective system prompt is `--agent`, which selects a preconfigured agent whose `prompt` replaces the provider-specific stock prompt. All other system-prompt manipulation happens through configuration files (`opencode.json`, `AGENTS.md`, `.opencode/agents/*.md`) or the `OPENCODE_CONFIG_CONTENT` environment variable. This makes OpenCode the outlier among the providers Claudine wraps: there is no inline or file-backed `--append-system-prompt` equivalent, and replacement is only partial (agent prompt replaces the provider layer, not project instruction files).

## CLI Parameters

OpenCode `run` accepts no flag whose primary purpose is to append or replace the system prompt.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--agent <name>` | modify | Selects a custom or built-in agent. If the agent defines a `prompt`, that prompt replaces the provider stock prompt for the session. |
| `--model <provider/model>` | other | Chooses the model. Model selection determines which provider-specific stock prompt is loaded, but does not accept custom text. |
| `--pure` | disable | Skips external plugins. Does not disable `AGENTS.md`, `instructions`, or agent prompts. |
| `--auto` | modify | Auto-approves permissions. Affects execution policy, not prompt content. |

There is no `--system`, `--system-prompt`, `--append-system-prompt`, `--prompt-instructions`, or similar switch in `opencode run --help` (v1.17.13).

## Configuration and Discovery

Effective instructions are assembled from several config-driven sources.

### `AGENTS.md` hierarchy

OpenCode discovers `AGENTS.md` files by walking up from the current working directory. `CLAUDE.md` is used as a fallback when no `AGENTS.md` exists. Precedence:

1. Project `AGENTS.md` (or `CLAUDE.md` if no `AGENTS.md`)
2. Global `~/.config/opencode/AGENTS.md` (or `~/.claude/CLAUDE.md` as fallback)

`AGENTS.md` content is treated as project/user context and appended to the system text. There is no documented CLI flag to skip this discovery for a single run.

### `opencode.json` and `instructions`

`opencode.json` (project, global, or inline via `OPENCODE_CONFIG_CONTENT`) supports an `instructions` array of file paths and glob patterns. These files are included as additional system instructions, appended after the agent/provider prompt layer.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "instructions": ["CONTRIBUTING.md", "docs/guidelines.md", ".cursor/rules/*.md"]
}
```

Values are file references, not inline strings. Remote URLs are also supported with a five-second timeout.

### Agent definitions

Agents can be defined in `opencode.json` or as Markdown files in `.opencode/agents/` or `~/.config/opencode/agents/`. The frontmatter `prompt` field (or the file body for Markdown agents) becomes the agent's system prompt. When active, this prompt replaces the provider-specific stock prompt.

```json
{
  "agent": {
    "review": {
      "mode": "subagent",
      "description": "Reviews code",
      "prompt": "You are a code reviewer. Focus on security, performance, and maintainability."
    }
  }
}
```

Primary agents (`mode: primary`) can be selected with `--agent`; subagents are invoked via `@mention` or the `task` tool.

### `OPENCODE_CONFIG_CONTENT`

`OPENCODE_CONFIG_CONTENT` holds a raw JSON object that is applied session-wide, including to subagent sessions. It is the natural per-invocation delivery surface for wrappers, because it does not require writing to the project `opencode.json` or `AGENTS.md`.

### `OPENCODE_DISABLE_CLAUDE_CODE*`

For users migrating from Claude Code, OpenCode reads `CLAUDE.md` and `.claude/skills/` by default. These can be disabled with:

- `OPENCODE_DISABLE_CLAUDE_CODE=1` — disable all `.claude` support
- `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT=1` — disable `~/.claude/CLAUDE.md`
- `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` — disable `.claude/skills`

## Prompt Layers and Precedence

The effective system text is built from the following layers, from base to final.

```mermaid
graph TD
    A[Stock provider/model prompt] --> B{Custom agent with prompt?}
    B -- yes --> C[Agent prompt]
    B -- no --> A
    C --> D[opencode.json instructions]
    A --> D
    D --> E[AGENTS.md / CLAUDE.md]
    E --> F[User message]
```

- The stock prompt is selected by model/provider and is not published verbatim.
- A custom agent `prompt` replaces the stock prompt (not appended).
- `instructions` files are appended after the agent/stock prompt.
- `AGENTS.md` / `CLAUDE.md` are appended after `instructions`.

This ordering is consistent with the request assembly shown in GitHub issue #34721, where `agent.prompt` is used in place of `SystemPrompt.provider(model)` and `input.system` / `input.user.system` are appended afterward.

## Agents and Subagents

OpenCode supports both primary and subagent definitions with their own prompts.

- **Primary agents** are cycled with the `Tab` key or selected via `--agent`. The built-in primary agents are `build` and `plan`.
- **Subagents** are invoked with `@mention` or automatically by the primary agent via the `task` tool. Built-in subagents include `general`, `explore`, and `scout`.
- Each agent can define its own `prompt`, `model`, `temperature`, `permissions`, and `tools`.
- Subagents run in isolated child sessions; only the final summary returns to the parent.
- As of v1.17.13, a custom agent prompt **replaces** the stock provider prompt. There is no documented additive mode (issue #34721 requests `systemMode: append` for 2.0).

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | Appended instructions are concatenated with `AGENTS.md` and the stock prompt; headers and bullet lists blend cleanly. |
| Replace (agent prompt) | XML-wrapped Markdown | The agent prompt must be self-contained because it replaces the provider-specific operational prompt. XML tags help the model distinguish rules, constraints, context, and examples. |

When replacing, the prompt author becomes responsible for any tool-calling guidance the task still needs, because the provider-specific stock prompt is removed.

## Recent Changes

- **v1.17.13 (2026-07-01)** — Latest stable release. No new system-prompt CLI flags were added.
- **Issue #34721 (2026-07-01)** — Documented that custom agent prompts currently replace the stock provider/model prompt rather than appending to it, and requested an additive mode for 2.0.
- **Issue #34341 (2026-06-28)** — Proposed progressive, path-scoped `AGENTS.md` loading in V2, where read-tool events trigger discovery of nearby instruction files.
- **Issue #26376 (2026-05-08)** — Requested persisting the dynamically generated system prompt in `opencode.db` so transcript exports include it. As of v1.17.13, no inspect/export surface exists.

## Quirks and Workarounds

- There is no direct CLI flag for appending or replacing the system prompt. Workarounds rely on `OPENCODE_CONFIG_CONTENT`, agent definitions, or temporary `AGENTS.md` files.
- Because `AGENTS.md` is auto-discovered by directory walk, a wrapper that wants a clean replacement must either use a custom agent or place files outside the discovery path; there is no `--no-agents-md` switch.
- `OPENCODE_CONFIG_CONTENT` propagates to subagent sessions, so wrapper-injected instructions are not limited to the parent run.
- The `instructions` array accepts file paths/globs, not inline strings. A wrapper must write prompt text to a temporary file and reference it.
- `--pure` disables external plugins but leaves config-based instructions intact.

## Claudine Delivery Notes

Claudine's `SystemPromptSpec` for OpenCode currently marks both append and replace as `Unsupported`. The research points to the following delivery paths:

- **Append**: Use `OPENCODE_CONFIG_CONTENT` to inject `{"instructions": ["<tmp_file>"]}`. The temp file contains the composed prompt text. This avoids mutating project `opencode.json` or `AGENTS.md`, but requires a new `SystemPromptDelivery` variant because `OPENCODE_CONFIG_CONTENT` is neither a file path nor a standard CLI config flag.
- **Replace**: Use `OPENCODE_CONFIG_CONTENT` to define a primary agent with a custom `prompt`, then invoke `opencode run` with `--agent <name>`. This replaces only the provider stock prompt layer; `instructions` and `AGENTS.md` layers may still be appended. A true full-system-prompt replacement is not possible without also disabling `AGENTS.md` discovery, which OpenCode does not currently expose per-run.

Because `OPENCODE_CONFIG_CONTENT` is already the channel Claudine uses for MCP injection and YOLO permissions, the existing `merge_overlay` helper in `claudine/lib/src/opencode_config.rs` should be reused so the system-prompt overlay does not clobber MCP or permission overlays.

## Sources

- [OpenCode docs homepage](https://opencode.ai/docs)
- [OpenCode rules / AGENTS.md docs](https://opencode.ai/docs/rules)
- [OpenCode config docs](https://opencode.ai/docs/config)
- [OpenCode agents docs](https://opencode.ai/docs/agents)
- [OpenCode CLI docs](https://opencode.ai/docs/cli)
- [OpenCode plugins docs](https://opencode.ai/docs/plugins)
- [OpenCode agent skills docs](https://opencode.ai/docs/skills)
- [OpenCode changelog](https://opencode.ai/changelog)
- [GitHub issue #34721 — agent: support additive custom system prompts](https://github.com/anomalyco/opencode/issues/34721)
- [GitHub issue #34341 — Load AGENTS.md progressively via read-tool plugin context](https://github.com/anomalyco/opencode/issues/34341)
- [GitHub issue #26376 — Save dynamically generated system prompt to opencode.db](https://github.com/anomalyco/opencode/issues/26376)

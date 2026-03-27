---
name: claudine
description: Details on the Claudine library and CLI, including deep research into Agentic CLI platforms such as Claude Code, Codex CLI, Goose, Opencode CLI, and all other Agentic CLI's supported by the Claudine library.
last_updated: 2026-03-17
---

## Claudine Library

Claudine is a universal event handler, skill linker, and MCP catalog manager for agentic CLIs. It normalizes 16 lifecycle events across 8 providers (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model, executes 6 action types -- TTS, sound effects, logging, shell commands, reports, and blocking calls -- when those events fire, synchronizes skills/commands/agents/scripts between providers, and manages provider-agnostic MCP storage plus provider-specific import/sync/runtime behavior.

The library is organized into fourteen modules plus the shared error type: `actions` (hook action types and responses), `adapters` (provider-specific event parsers), `agents` (capability catalog for all 8 CLIs), `badges` (styled terminal badge constants), `composition` (markdown frontmatter composition for inline and chained prompt pipelines), `config` (agent detection, hook registration, atomic writes, backups), `dispatch` (event processing pipeline), `events` (the normalized 16-event lifecycle model), `harness` (typed pre/post validations, timeouts, handler resolution, shell policy adapter, and recovery actions for composed prompt pipelines), `linking` (cross-provider skill synchronization), `mcp` (catalog, defaults, provider-state, import/export, session composition, runtime injectors), `reporting` (JSONL-to-SQLite metrics index), `services` (cross-provider policy engines such as Protect), and `stream` (structured stream parsing for 6 providers with summary/reporting).

The dispatch pipeline supports a Handlebars-style template engine with 28 variables across 5 categories (event, OS, hardware, git, and project), shell environment variable interpolation with optional defaults, and precompiled regex matchers for event filtering. Configuration merges user-scope and repo-scope configs with an intentionally asymmetric strategy: repo provider configs fully replace user-level configs, while global settings merge field-by-field.

- [Supported Platforms](supported-platforms.md)
- [Unified Hook/Event Model](unified-hooks.md)
- [Supported Actions](hook-actions.md)
- [Linking Strategy](linking-strategy.md)


## Claudine CLI

The `claudine` binary provides interactive setup, hook inspection, event handling, skill linking, MCP management, log reporting, and provider wrapping for agentic CLIs. It includes an `init` wizard that walks through 4 phases (agent discovery, provider preferences, action defaults, and write & register), with a `--quick` flag for sensible defaults and a `--repo` flag for project-scoped configuration. All user-facing output flows through a structured logging system that separates pipeable data (stdout) from status messages (stderr), with rich formatting via biscuit-terminal components including tables, prose markup, and OSC8 hyperlinks.

The CLI uses fuzzy provider matching (exact, prefix, and contains resolution) so users can type shorthand like `cl` for `claude`. The `dry-run` command accepts event names in multiple formats (canonical snake_case, native provider names, PascalCase, and kebab-case) and generates realistic mock payloads when no stdin is provided, making it easy to test hook configurations without triggering real events.

| Command | Description |
|---------|-------------|
| `claudine init [--quick] [--repo]` | Interactive setup wizard (or quick defaults) |
| `claudine hooks [provider]` | Show registered hooks for all or one provider |
| `claudine hooks --support` | Provider event support matrix |
| `claudine hooks --mapping` | Native event name mappings per provider |
| `claudine hooks --describe` | Event descriptions and payload schemas |
| `claudine hooks --variables` | Template variables with current values |
| `claudine link [provider] [--scope <user\|repo>] [--apply] [--filter] [--detailed]` | Analyze resource link states and optionally fix auto-repairable issues |
| `claudine link --support` | Provider resource support matrix |
| `claudine mcp [list\|init\|add\|config\|default\|alias\|remove\|check\|sync\|export] [--json]` | Manage the normalized MCP catalog, defaults, validation, refresh, and export state |
| `claudine logs [today\|week\|month\|sessions\|tools\|errors\|repos\|trends\|sync]` | Reporting and sync for Claudine JSONL logs |
| `claudine providers` | Provider capability matrix (skill/slash/agent/hooks) |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply hook registrations |
| `claudine handle <event> [--provider]` | Process event from stdin (called by hooks) |
| `claudine dry-run <event> [--provider]` | Test event handling without side effects |
| `claudine about` | Rich help documentation |
| `claudine claude\|codex\|gemini\|...` | Wrap a provider CLI with preflight checks, env sanitization, and structured streaming |
| `claudine completions <shell>` | Generate shell completions |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |

## MCP Support

Claudine stores normalized MCP data in `~/.claudine/mcp/catalog.json`, `~/.claudine/mcp/defaults.json`, and `~/.claudine/mcp/provider-state.json`, with optional repo defaults in `<repo>/.claudine/mcp.json`. Repo defaults replace user defaults.

Current provider rollout:

- Import and sync: Claude, Codex, Gemini, OpenCode, and Roo
- Runtime wrapper injection: Codex, Gemini, and OpenCode
- No MCP support yet: Goose, Kimi, and Qwen

Wrapper MCP behavior:

- `--mcp` launches with the effective defaults; `--use id-or-alias[,id-or-alias...]` adds explicit servers and also enables MCP mode.
- Initial Codex, Gemini, and OpenCode prompts resolve `#tags` and strip them before forwarding the prompt to the provider.
- Codex and Gemini runtime injection write provider config into a shadow HOME under `~/.claudine`; OpenCode uses `OPENCODE_CONFIG_CONTENT`.
- Claude, Goose, Kimi, and Qwen wrappers currently direct users to `claudine mcp export <provider> --apply` instead of runtime injection.

Read [claudine/docs/mcp-support.md](../../../claudine/docs/mcp-support.md) before changing MCP behavior or documenting new provider support.

## Research on Agentic CLI Platforms

### Hooks Research

Research into each Agentic CLI's provided hooks, payloads and return types.

- [Claude Code](research/hooks/claude-code.md)
- [Codex](research/hooks/codex.md)
- [Gemini CLI](research/hooks/gemini-cli.md)
- [Goose](research/hooks/goose.md)
- [Kimi Code](research/hooks/kimi-code.md)
- [OpenCode](research/hooks/opencode.md)
- [Qwen CLI](research/hooks/qwen-cli.md)
- [Roo Code](research/hooks/roo-code.md)

### Cross-referencing Research

Research into each Agentic CLI's support for features like agentic skills, slash commands, agents/subagents, and shared scripts folders.

- [Claude Code](research/cross-referencing/claude-code.md)
- [Codex](research/cross-referencing/codex.md)
- [Gemini CLI](research/cross-referencing/gemini-cli.md)
- [Goose](research/cross-referencing/goose.md)
- [Kimi Code](research/cross-referencing/kimi-code.md)
- [OpenCode](research/cross-referencing/opencode.md)
- [Qwen CLI](research/cross-referencing/qwen-cli.md)
- [Roo Code](research/cross-referencing/roo-code.md)

### ACP Support

Claudine does not use ACP today but we may add it in the future. If you're looking at anything related to ACP you should consider using the **acp** skill. If you're interested in how ACP might work with observability then use the **agent-observability** skill.



### CLI Research

Research into the subcommands and switches each Agentic CLI platform provides as well as providing insight into the various means of executing this platform in a non-interactive session, choosing which model to use, and more.

No CLI research documents are available yet.

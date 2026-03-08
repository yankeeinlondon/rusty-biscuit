---
name: claudine
description: Details on the Claudine library and CLI, including deep research into Agentic CLI platforms such as Claude Code, Codex CLI, Goose, Opencode CLI, and all other Agentic CLI's supported by the Claudine library.
last_updated: 2026-02-19
---

## Claudine Library

Claudine is a universal event handler and skill linker for agentic CLIs. It normalizes 16 lifecycle events across 8 providers (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model, then executes 6 action types -- TTS, sound effects, logging, shell commands, reports, and blocking calls -- when those events fire. The library also synchronizes skills, commands, agents, and scripts between providers via symlinks, enabling a single set of resources to be shared across all installed agentic CLIs.

The library is organized into seven core modules: `actions` (hook action types and response model with 6 action variants and 4 decision types), `adapters` (provider-specific event parsers implementing the `ProviderAdapter` trait), `agents` (comprehensive capability catalog for all 8 CLIs covering model selection, permissions, skill paths, and more), `config` (agent detection, hook registration, atomic file writes, and backup utilities), `dispatch` (the 6-step event processing pipeline including config loading, matcher evaluation, and action execution), `events` (the normalized 16-event lifecycle model with metadata, support levels, and provider mappings), and `linking` (cross-provider skill synchronization via a 4-phase discovery/hashing/analysis/linking algorithm).

The dispatch pipeline supports a Handlebars-style template engine with 28 variables across 5 categories (event, OS, hardware, git, and project), shell environment variable interpolation with optional defaults, and precompiled regex matchers for event filtering. Configuration merges user-scope and repo-scope configs with an intentionally asymmetric strategy: repo provider configs fully replace user-level configs, while global settings merge field-by-field.

- [Supported Platforms](supported-platforms.md)
- [Unified Hook/Event Model](unified-hooks.md)
- [Supported Actions](hook-actions.md)
- [Linking Strategy](linking-strategy.md)

## Claudine CLI

The `claudine` binary provides interactive setup, hook inspection, event handling, and skill linking for agentic CLIs. It includes an `init` wizard that walks through 5 phases (agent discovery, event selection, action configuration, global settings, and hook registration), with a `--quick` flag for sensible defaults and a `--repo` flag for project-scoped configuration. All user-facing output flows through a structured logging system that separates pipeable data (stdout) from status messages (stderr), with rich formatting via biscuit-terminal components including tables, prose markup, and OSC8 hyperlinks.

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
| `claudine providers` | Provider capability matrix (skill/slash/agent/hooks) |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply hook registrations |
| `claudine handle <event> [--provider]` | Process event from stdin (called by hooks) |
| `claudine dry-run <event> [--provider]` | Test event handling without side effects |
| `claudine about` | Rich help documentation |
| `claudine completions <shell>` | Generate shell completions |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |

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

### CLI Research

Research into the subcommands and switches each Agentic CLI platform provides as well as providing insight into the various means of executing this platform in a non-interactive session, choosing which model to use, and more.

No CLI research documents are available yet.

---
name: claudine
description: Use when working in the claudine/ package area or with the Claudine library/CLI — normalizing agentic-CLI lifecycle events and hooks, wrapping providers (Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Roo), composing Markdown prompts (compose/inline-compose/sequence), managing the MCP catalog, linking skills/commands/agents across providers, or researching agentic CLI platform behavior.
last_updated: 2026-05-23
---

## Overview

Claudine is a universal event handler, shared-resource linker, MCP catalog manager, and composition harness for agentic CLIs. It normalizes **16 lifecycle events** across **8 providers** (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model, fires **6 action types** — TTS, sound effects, logging, shell commands, reports, and blocking calls — when those events fire, synchronizes skills/commands/agents/scripts between providers, manages a provider-agnostic MCP catalog plus provider-specific import/sync/runtime behavior, and provides three Markdown composition commands (`compose`, `inline-compose`, `sequence`) that flow through the same wrapper-grade execution pipeline as the provider wrappers.

The package follows the monorepo `lib` + `cli` split: library crate `claudine`, CLI crate `claudine-cli` (binary `claudine`).

**Where to look next:**

- For library internals (module structure, key types, provider adapters, dispatch, stream parsing) → [architecture.md](architecture.md)
- For full per-command documentation → [cli-reference.md](cli-reference.md)
- For when/why a feature changed → [timeline.md](timeline.md)
- For a specific subsystem → the deep topic docs under [Reference Documents](#reference-documents)

## Library Module Map

Nineteen modules plus the shared error type and the flat `provider_id` leaf. Full detail in [architecture.md](architecture.md).

| Module | Responsibility |
|--------|----------------|
| `actions` | Hook action types and responses |
| `adapters` | Provider-specific event parsers |
| `agents` | Capability catalog for all 8 CLIs |
| `badges` | Styled terminal badge constants |
| `composition` | Markdown frontmatter composition (direct/inline/sequence) plus the loop engine |
| `config` | Agent detection, hook registration, atomic writes, backups |
| `dispatch` | Event processing pipeline, templates, matchers, expression bridge |
| `events` | The normalized 16-event lifecycle model |
| `harness` | Typed pre/post validations, timeouts, handler resolution, recovery actions |
| `linking` | Cross-provider resource sync with portability classification |
| `mcp` | Catalog, defaults, provider-state, import/export, runtime injectors |
| `messaging` | Outbound routes (Discord/Slack/Signal/WhatsApp); desktop notifications are separate and zero-config |
| `model_catalog` | Merged provider model catalog with cache and user overrides |
| `permissions` | Provider-agnostic policy engine (`PolicyEngine`) |
| `protect` | Standalone regex deny catalog (bash commands, write/edit paths, MCP responses) |
| `prompt_reporting` | System/user prompt reporting types and rendering |
| `reporting` | JSONL-to-SQLite metrics index |
| `stream` | Structured stream parsing for 6 providers (typed models in `stream::protocol`) |
| `system_prompt` | Launch-CWD detection (`LaunchContext`), `system-prompt.md` discovery, `EffectiveSystemPrompt` resolution |
| `provider_id` *(leaf)* | `Provider` enum, `provider_info()`, `PROVIDERS_DISPLAY_ORDER`, `OutputFormatSelector` (split from `provider/mod.rs` to break the `provider` ⇄ `stream` import cycle) |

## CLI Commands

The `claudine` binary provides interactive setup, hook inspection, event handling, shared-resource management, MCP management, log reporting, provider wrapping, and Markdown composition. Output flows through a structured logging system that separates pipeable data (stdout) from status messages (stderr), formatted with biscuit-terminal components. Provider names accept fuzzy matching (exact → prefix → contains), so `cl` resolves to `claude`. Full reference: [cli-reference.md](cli-reference.md).

**Shared Resources**

| Command | Description |
|---------|-------------|
| `claudine skills` | List skills across providers and show link/sync state |
| `claudine commands` | List slash commands across providers and show link/sync state |
| `claudine agents` | List agent/subagent definitions across providers and show link/sync state |
| `claudine mcp [list\|init\|show\|default\|alias\|remove\|sync] [--json]` | Manage the normalized MCP catalog, defaults, validation, refresh, and sync state |

**Hook Events and Actions**

| Command | Description |
|---------|-------------|
| `claudine hooks [provider]` | Show registered hooks for all or one provider |
| `claudine hooks --support` | Provider event support matrix |
| `claudine hooks --mapping` | Native event name mappings per provider |
| `claudine hooks --describe` | Event descriptions and payload schemas |
| `claudine hooks --variables` | Template variables with current values |
| `claudine actions` | Show which actions are configured and for which events |
| `claudine handle <event> [--provider]` | Process event from stdin (hidden; called by hook registrations) |

**Wrapped Execution**

| Command | Description |
|---------|-------------|
| `claudine claude\|codex\|gemini\|goose\|kimi\|opencode\|qwen` | Wrap a provider CLI with preflight checks, env sanitization, system prompt resolution, optional `--edit` prompt drafting, MCP injection, and structured streaming |

**Composition**

| Command | Description |
|---------|-------------|
| `claudine compose <file> [key=value ...]` | Compose a Markdown file and send the result as a prompt (no file mutation) |
| `claudine inline-compose <file> [key=value ...]` | Use frontmatter `prompt` to generate content and replace the body; preserves frontmatter, updates `last_updated` |
| `claudine sequence <file> [key=value ...]` | Run a serial sequence of composition steps with shared shell approval cache and `FAIL_FAST` propagation |

**Administration**

| Command | Description |
|---------|-------------|
| `claudine init [--quick] [--repo]` | Interactive setup wizard (4 phases) or quick defaults |
| `claudine config` | TUI for managing configuration, including messenger routes |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply hook registrations |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |
| `claudine providers` | Provider capability matrix (skill/slash/agent/hooks) |
| `claudine logs [today\|week\|month\|sessions\|tools\|errors\|repos\|trends\|sync]` | Reporting and sync for Claudine JSONL logs |
| `claudine completions <shell>` | Generate shell completions |
| `claudine context [--values\|--expressions\|--side-effects]` | Show Darkmatter runtime context, expression engine, and side effects |
| `claudine` *(no subcommand)* | Render rich grouped help |

**Wrapper & composition notes:**

- **Argv pre-parsing** — `argv::normalize` rewrites composition-subcommand argv before clap (provider booleans → `--provider`, `--help` hoisting, `--` separator insertion). See [CLI Pre-Parsing](../../../claudine/docs/topics/cli-pre-parsing.md).
- **System prompt** — file-backed only: `--append-system-prompt`/`--asp` and `--replace-system-prompt`/`--rsp`, with `system-prompt.md` discovery from the launch-CWD hierarchy; direct wrappers also support `--edit`. Delivery is spec-driven per provider. See [System Prompt](../../../claudine/docs/topics/system-prompt.md).
- **Timeouts** — two rules, `timeout` (wall-clock, opt-in) and `step_timeout` (stream-silence, default `30m`). See [Timeouts](../../../claudine/docs/topics/timeouts.md).
- **Transient overlays** — written to `<repo_root>/.claudine/tmp/` (or `<launch_cwd>/.claudine-tmp/`) and cleaned up on `Drop`.

**Config TUI — messenger routes:** `claudine config` manages bot-token routes (Discord, Slack, Signal, WhatsApp) and webhook routes (Discord/Slack webhooks). Webhook URLs use masked input and are validated before advancing; env-only routes (blank URL + env var) are allowed. A **Test Connection** workflow (`T` during webhook input) sends a test message without saving. Desktop notifications are intentionally absent — they are zero-config and triggered via lifecycle `notify` frontmatter only.

**Webhook redaction invariants:** inline webhook URLs never render raw (shown as `webhook: ********`); secret input buffers are masked during entry; all webhook send errors and test-connection failures run through `redact_webhook_urls` before display.

## MCP Support

Claudine stores normalized MCP data in `~/.claudine/mcp/catalog.json`, `~/.claudine/mcp/defaults.json`, and `~/.claudine/mcp/provider-state.json`, with optional repo defaults in `<repo>/.claudine/mcp.json` (repo defaults replace user defaults).

Provider rollout:

- **Import and sync:** Claude, Codex, Gemini, OpenCode, Roo
- **Runtime wrapper injection:** Codex, Gemini, OpenCode
- **No MCP support yet:** Goose, Kimi, Qwen

Wrapper behavior: `--mcp` launches with effective defaults; `--use id-or-alias[,...]` adds explicit servers and enables MCP mode. Codex/Gemini inject via a shadow HOME under `~/.claudine`; OpenCode uses `OPENCODE_CONFIG_CONTENT`. Claude, Goose, Kimi, and Qwen wrappers direct users to `claudine mcp export <provider> --apply` instead. Read [MCP Catalog](../../../claudine/docs/topics/mcp-catalog.md) and [MCP Mode](../../../claudine/docs/topics/mcp-mode.md) before changing MCP behavior.

## Reference Documents

### Skill docs (in this directory)

- [Architecture](architecture.md) — module structure, event matrix, key types, provider adapters, dispatch, stream parsing, composition, linking, exit codes
- [CLI Reference](cli-reference.md) — full per-command documentation, flags, output system, exit codes, file locations, env vars
- [Change Timeline](timeline.md) — condensed history of significant features and refactors
- [Supported Platforms](supported-platforms.md)
- [Unified Hook/Event Model](unified-hooks.md)
- [Supported Actions](hook-actions.md)
- [Linking Strategy](linking-strategy.md)
- [Non-Portable Assets](non-portable-assets.md)
- [PolicyEngine](policy-engine.md)
- [Validations and Handlers](validations-and-handlers.md)
- [OpenCode Event Sources](opencode-event-sources.md) — Dual-Source Contract, stderr promotion table, watchdog interaction

### Repo topic docs (canonical references, not duplicated here)

- [Composition](../../../claudine/docs/topics/composition.md) — `compose`, `inline-compose`, `sequence`, harness validations, handlers, provider selection
- [Timeouts](../../../claudine/docs/topics/timeouts.md) — the two timeout rules, four env vars, precedence, termination path, exit reasons
- [System Prompt](../../../claudine/docs/topics/system-prompt.md) — launch-context discovery, `--append`/`--replace`, Darkmatter preparation, per-provider delivery
- [MCP Catalog](../../../claudine/docs/topics/mcp-catalog.md) and [MCP Mode](../../../claudine/docs/topics/mcp-mode.md)
- [Protect Service](../../../claudine/docs/topics/protect-service.md) — deny catalog, scan surfaces, rule groups, merge semantics, dispatch integration
- [Traces and Logging](../../../claudine/docs/topics/traces-and-logging.md), [Log Reporting](../../../claudine/docs/topics/log-reporting.md)
- [CLI Pre-Parsing](../../../claudine/docs/topics/cli-pre-parsing.md) — argv normalization pipeline; rule-by-rule reference in [argv-normalization.md](../../../claudine/docs/topics/argv-normalization.md)
- [Shell Completions](../../../claudine/docs/topics/shell-completions.md) — dynamic completion engine, per-mode pipelines, magic `@` resolution

## Research on Agentic CLI Platforms

### Hooks Research

Each Agentic CLI's provided hooks, payloads, and return types.

- [Claude Code](research/hooks/claude-code.md) · [Codex](research/hooks/codex.md) · [Gemini CLI](research/hooks/gemini-cli.md) · [Goose](research/hooks/goose.md) · [Kimi Code](research/hooks/kimi-code.md) · [OpenCode](research/hooks/opencode.md) · [Qwen CLI](research/hooks/qwen-cli.md) · [Roo Code](research/hooks/roo-code.md)

### Cross-referencing Research

Each Agentic CLI's support for skills, slash commands, agents/subagents, and shared scripts folders.

- [Claude Code](research/cross-referencing/claude-code.md) · [Codex](research/cross-referencing/codex.md) · [Gemini CLI](research/cross-referencing/gemini-cli.md) · [Goose](research/cross-referencing/goose.md) · [Kimi Code](research/cross-referencing/kimi-code.md) · [OpenCode](research/cross-referencing/opencode.md) · [Qwen CLI](research/cross-referencing/qwen-cli.md) · [Roo Code](research/cross-referencing/roo-code.md)

### ACP Support

Claudine does not use ACP today but may add it. For ACP work use the **acp** skill; for ACP + observability use the **agent-observability** skill.

### CLI Research

Subcommands, switches, non-interactive execution, and model selection per platform. *No CLI research documents are available yet.*

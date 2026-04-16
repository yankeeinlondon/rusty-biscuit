---
name: claudine
description: Details on the Claudine library and CLI, including deep research into Agentic CLI platforms such as Claude Code, Codex CLI, Goose, Opencode CLI, and all other Agentic CLI's supported by the Claudine library.
last_updated: 2026-04-16
---

## Claudine Library

Claudine is a universal event handler, shared-resource linker, MCP catalog manager, and composition harness for agentic CLIs. It normalizes 16 lifecycle events across 8 providers (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model, executes 6 action types -- TTS, sound effects, logging, shell commands, reports, and blocking calls -- when those events fire, synchronizes skills/commands/agents/scripts between providers, manages provider-agnostic MCP storage plus provider-specific import/sync/runtime behavior, and provides three Markdown composition commands (`compose`, `inline-compose`, `sequence`) that flow through the same wrapper-grade execution pipeline as the provider wrappers.

The library is organized into seventeen modules plus the shared error type: `actions` (hook action types and responses), `adapters` (provider-specific event parsers), `agents` (capability catalog for all 8 CLIs), `badges` (styled terminal badge constants), `composition` (markdown frontmatter composition for direct, inline, and sequence prompt pipelines with lifecycle emitters, preflight shell approval, and closure write-back), `config` (agent detection, hook registration, atomic writes, backups), `dispatch` (event processing pipeline), `events` (the normalized 16-event lifecycle model), `harness` (typed pre/post validations, timeouts, handler resolution, shell policy adapter, and recovery actions for composed prompt pipelines), `linking` (cross-provider skill/command/agent/script synchronization with portability classification), `mcp` (catalog, defaults, provider-state, import/export, session composition, runtime injectors), `messaging` (outbound messaging routes for Discord/Slack/Signal/WhatsApp with secret and recipient resolution), `permissions` (provider-agnostic policy engine for permission queries, canonical modeling, and mutation planning), `reporting` (JSONL-to-SQLite metrics index), `services` (cross-provider runtime policy services such as ProtectService), `stream` (structured stream parsing for 6 providers backed by strongly typed protocol models in `stream::protocol`, plus summary and reporting), and `system_prompt` (launch-CWD workspace detection via `LaunchContext`, standard `system-prompt.md` discovery, Darkmatter preparation, `EffectiveSystemPrompt` resolution, and provider-specific launch-plan application).

`ProtectService` was refactored on 2026-04-06 into a standalone regex-backed deny catalog. It now evaluates bash commands, write/edit paths, and MCP tool responses with a strict `Allow` or `Block` outcome, has no posture or severity model, does not depend on `PolicyEngine`, and supports per-group toggles plus command-only `custom_patterns`. Config supports shorthand `"protect": true` or an expanded object with `enabled`, `rules`, and `custom_patterns`; repo/user merge semantics are Protect-specific: `enabled` is OR-merged, repo rule toggles override user toggles per group, and custom patterns combine as repo first then user. `allow_paths` is only valid for `filesystem_destruction` and `sensitive_paths`.

Stream parsing was refactored on 2026-04-11 onto strongly typed serde-derived protocol models in [`claudine/lib/src/stream/protocol/`](../../../claudine/lib/src/stream/protocol/), one module per supported provider (Claude, Codex, Gemini, OpenCode, Qwen, Kimi). Each module exports a tagged `*Event` enum (`#[serde(tag = "type")]`) plus one struct per variant payload. Every field is optional via `#[serde(default)]` so format evolution never breaks deserialization, and there is no `#[serde(deny_unknown_fields)]` anywhere in `protocol/`. Unknown event types fall through to a silent skip that matches the legacy `_ => Ok(None)` arm. Field aliases are resolved by helper methods on each struct (`resolved_tool_name()`, `take_input()`, `effective_cost_usd()`, `merge_started()`, `resolve()`, `into_init()`, `computed_percent()`, etc.) so handlers never see raw aliasing. Each parser's `feed_line` is now a two-pass dispatch: parse to `serde_json::Value` first (preserves the malformed-line error path and keeps a raw copy for `raw_summary` construction in result events), then attempt typed deserialization into the provider-specific `*Event` enum. Every protocol module has a `#[cfg(test)] mod tests` block covering each variant, the major field aliases, and the `unknown_event_type_fails_typed` contract — those tests are the safety net for provider format drift.

The dispatch pipeline supports a Handlebars-style template engine with 28 variables across 5 categories (event, OS, hardware, git, and project), shell environment variable interpolation with optional defaults, and precompiled regex matchers for event filtering. Configuration merges user-scope and repo-scope configs with an intentionally asymmetric strategy: repo provider configs fully replace user-level configs, while global settings merge field-by-field.

The live stderr surface was hardened on 2026-04-14 (feature: `2026-04-14-response-refinement`). `LiveSemanticSink` in [`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../../claudine/cli/src/commands/wrap/live_semantic_sink.rs) enforces a **9-section model** for non-interactive output (execution line, env, system prompt, agent prompt, session ID, thinking prose, tool/info events, final STDOUT, and metadata) with strictly enforced spacing. Thinking prose is rendered as a `BlockQuote` on stderr. Tool calls use a canonical `ToolCallDisplay` contract (`🔧 →` / `🔧 ←`) with humanized names and summarized inputs/results; raw JSON is never dumped to the terminal for known tools. The Gemini parser fix for markdown lists and OpenCode's `--yolo` forward to `--dangerously-skip-permissions` also landed in this cycle.

Hook handlers were hardened on 2026-04-14 (feature: `2026-04-14-more-meta-response`, Plan 1). `claudine handle` now enforces a **5-second execution deadline** (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`) to prevent hook handlers from blocking the parent agent session. When exceeded, the handler aborts and exits 124. Bash and messenger actions also have 3s timeouts when running inside a hook handler. Phase-level tracing spans (`handle_stdin_read`, `handle_dispatch_canonical`, etc.) ensure any future hang is diagnostic.

Composition rendering was unified on 2026-04-16 (fix: `2026-04-16-consistent-rendering`). `compose` and `inline-compose` now share one non-harness execution function (`execute_without_harness` in [`claudine/cli/src/commands/wrap/composition.rs`](../../../claudine/cli/src/commands/wrap/composition.rs)) parameterized by `CompositionExecutionMode::{Direct, Inline}`, one structured-stream helper (`run_structured_composition` returning `CompositionStreamResult`), and one summary emitter (`emit_composition_summary`) with a `defer_section_separator` flag that selects between compose's immediate emission and inline-compose's post-closure deferred emission. The Goose-only legacy (non-structured) path now calls `emit_minimal_composition_summary` to render the same stderr summary block as structured runs, replacing the previous JSONL-only silence from the deleted `emit_legacy_composition_session_event`. The `"agent did not provide a summarized message"` warning is removed from inline-compose (the `SessionEnd` JSONL event already records empty assistant text). Four inline-only behaviors remain guarded and commented: closure validation/file write, deferred summary timing, interrupted-session partial body report, and the writability pre-check.

The non-interactive stderr surface was strengthened on 2026-04-16 (fix: `2026-04-16-more-is-more`). Tool call rendering switched from `→ Name · summary` to `→ Name(summary)` / `← Name(slot)` so it reads like a function call; shell-style tools (`Bash`, `bash`, `shell`, `run_command`) prepend the canonical shell name to the command (`bash ls -la`); and the `Task` extractor now prefers `description → subject → prompt → task` so the agent's actual task body wins over fields like `subagent_type`. `StreamTextRenderer` gained `last_block_growth_at` plus `flush_if_idle()`, and the heartbeat thread calls `flush_if_idle(silence_window)` (default 30 s) before emitting any status line so a dangling final paragraph reaches the user even when the provider never closes stdout. OpenCode now exposes a typed `Reasoning` variant on `OpenCodeEvent` (top-level `text` and nested `part.text`) that routes into `SemanticEvent::Reasoning` instead of `ProviderExtension`; `render_thinking_block()` was widened from a thin border to `▌ ` so thinking matches System Prompt and Agent Prompt visually. Finally, `SemanticEvent::Error` gained a typed `kind: SemanticErrorKind` field (`Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, `Unknown`, with `#[serde(default) = Unknown]` for replay compatibility); each provider parser classifies its errors before emission; the live sink renders them as colored `BlockQuote`s with `▌ ` border (orange `Configuration Error`, red `Agent Error` / `API Error`, yellow `Interrupted`, red `Error`); and `From<SemanticErrorKind> for AgentErrorCategory` aligns live error kinds with the end-of-run [`AgentErrorReport`](../../../claudine/cli/src/output/error_report.rs) surface. Dispatch behavior remains keyed off `terminal: bool`; `kind` is classificatory metadata only.

- [Supported Platforms](supported-platforms.md)
- [Unified Hook/Event Model](unified-hooks.md)
- [Supported Actions](hook-actions.md)
- [Linking Strategy](linking-strategy.md)
- [Non-Portable Assets](non-portable-assets.md)
- [PolicyEngine](policy-engine.md)
- [Validations and Handlers](validations-and-handlers.md)

For deeper topic references in the repo (not duplicated here), see:

- [Composition](../../../claudine/docs/topics/composition.md) — `compose`, `inline-compose`, `sequence`, harness validations, handlers, provider selection
- [System Prompt](../../../claudine/docs/topics/system-prompt.md) — launch-context discovery, `--append-system-prompt` / `--replace-system-prompt`, Darkmatter preparation, per-provider delivery strategies, empty-body disable semantics
- [MCP Catalog](../../../claudine/docs/topics/mcp-catalog.md) and [MCP Mode](../../../claudine/docs/topics/mcp-mode.md)
- [Protect Service](../../../claudine/docs/topics/protect-service.md) — standalone deny catalog, scan surfaces, rule groups, merge semantics, dispatch integration
- [Traces and Logging](../../../claudine/docs/topics/traces-and-logging.md), [Log Reporting](../../../claudine/docs/topics/log-reporting.md)


## Claudine CLI

The `claudine` binary provides interactive setup, hook inspection, event handling, shared-resource management (skills/commands/agents), MCP management, log reporting, provider wrapping, and Markdown composition pipelines for agentic CLIs. It includes an `init` wizard that walks through 4 phases (agent discovery, provider preferences, action defaults, and write & register), with a `--quick` flag for sensible defaults and a `--repo` flag for project-scoped configuration. All user-facing output flows through a structured logging system that separates pipeable data (stdout) from status messages (stderr), with rich formatting via biscuit-terminal components including tables, prose markup, and OSC8 hyperlinks.

The CLI uses fuzzy provider matching (exact, prefix, and contains resolution) so users can type shorthand like `cl` for `claude`. The `handle` command accepts event names in multiple formats (canonical snake_case, native provider names, PascalCase, and kebab-case) and is normally invoked from hook registrations wired up by `claudine init`.

System prompt handling is shared across wrapped provider subcommands and the Markdown composition surfaces. The current contract is file-backed only: `--append-system-prompt` / `--asp` and `--replace-system-prompt` / `--rsp`, with standard `system-prompt.md` discovery from the launch CWD hierarchy when neither flag is provided. `compose`, `inline-compose`, and `sequence` all pass through the same `system_prompt` pipeline as the direct provider wrappers.

**Shared Resources**

| Command | Description |
|---------|-------------|
| `claudine skills` | List available skills across providers and show link/sync state |
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
| `claudine claude\|codex\|gemini\|goose\|kimi\|opencode\|qwen` | Wrap a provider CLI with preflight checks, env sanitization, launch-context-based system prompt resolution, provider-specific prompt injection, MCP injection, and structured streaming |

**Composition**

| Command | Description |
|---------|-------------|
| `claudine compose <file> [key=value ...]` | Compose a Markdown file and send the result as a prompt (no file mutation); accepts shorthand `key=value` overrides and shared system prompt flags |
| `claudine inline-compose <file> [key=value ...]` | Use frontmatter `prompt` to generate content and replace the body; preserves frontmatter, updates `last_updated`, accepts shorthand `key=value` overrides and system prompt flags |
| `claudine sequence <file> [key=value ...]` | Run a serial sequence of composition steps with shared shell approval cache, `FAIL_FAST` propagation, shorthand `key=value` overrides, and the shared system prompt pipeline |

**Administration**

| Command | Description |
|---------|-------------|
| `claudine init [--quick] [--repo]` | Interactive setup wizard (or quick defaults) |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply hook registrations |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |
| `claudine providers` | Provider capability matrix (skill/slash/agent/hooks) |
| `claudine logs [today\|week\|month\|sessions\|tools\|errors\|repos\|trends\|sync]` | Reporting and sync for Claudine JSONL logs |
| `claudine completions <shell>` | Generate shell completions |
| `claudine` *(no subcommand)* | Render rich grouped help (replaces retired `about` command) |

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

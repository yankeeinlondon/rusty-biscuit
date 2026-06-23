---
name: claudine
description: Use when working in the claudine/ package area or with the Claudine library/CLI — normalizing agentic-CLI lifecycle events and hooks, wrapping providers (Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Roo), composing Markdown prompts (compose/inline-compose/sequence), managing the MCP catalog, linking skills/commands/agents across providers, or researching agentic CLI platform behavior.
last_updated: 2026-06-22
hash: bbb32528c11dc53d-d1ec45ac69f76003
---

## Overview

Claudine is a universal event handler, shared-resource linker, MCP catalog manager, and composition harness for agentic CLIs. It normalizes **16 lifecycle events** across **8 providers** (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model, fires **6 action types** — TTS, sound effects, logging, shell commands, reports, and blocking calls — when those events fire, synchronizes skills/commands/agents/scripts between providers, manages a provider-agnostic MCP catalog plus provider-specific import/sync/runtime behavior, and provides three Markdown composition commands (`compose`, `inline-compose`, `sequence`) that flow through the same wrapper-grade execution pipeline as the provider wrappers.

The package follows the monorepo `lib` + `cli` split: library crate `claudine`, CLI crate `claudine-cli` (binary `claudine`). A third sub-crate, `claudine-contract` (`claudine/contract`), implements `biscuit_contract::inference::InferenceAdapter` by running a provider as a single non-interactive, **tool-free, filesystem-isolated** session and returning its final assistant text — letting deterministic consumers (Reaper, Darkmatter) delegate to an agentic CLI via `Arc<dyn InferenceAdapter>` without depending on `claudine` directly. It depends on `claudine` (lib) but **not** `claudine-cli`. v1 enables Claude and Codex; other providers are reported `Unsupported`. See `claudine/contract/README.md` for the provider support matrix and security posture, and the `biscuit-contract` skill for the contract itself.

**Where to look next:**

- For library internals (module structure, key types, provider adapters, dispatch, stream parsing) → [architecture.md](architecture.md)
- For full per-command documentation → [cli-reference.md](cli-reference.md)
- For when/why a feature changed → [timeline.md](timeline.md)
- For a specific subsystem → the deep topic docs under [Reference Documents](#reference-documents)

## Library Module Map

Twenty modules plus the shared error type and the flat `provider_id` leaf. Full detail in [architecture.md](architecture.md).

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
| `runaway` | Pure content-guard detector (exit-expressions, group-cycle repetition, volume cap) + per-layer config; trips map to `ProcessTermination::Aborted` |
| `stream` | Structured stream parsing for 6 providers (typed models in `stream::protocol`) |
| `system_prompt` | Launch-CWD detection (`LaunchContext`), `system-prompt.md` discovery, `ResolvedSystemPrompt` resolution |
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
| `claudine context [--values\|--expressions\|--side-effects]` | Show Darkmatter context variables, expression engine, and side-effect capabilities |
| `claudine` *(no subcommand)* | Render rich grouped help |

**Context command:** `claudine context` renders from Darkmatter's public typed descriptor catalogs, not from parsed Markdown. The default report shows every context variable grouped by category with `Property` (`ctx.NAME`), `Type`, and `Description` columns. `--values` replaces `Description` with live captured values (nulls shown, not dropped). `--expressions` shows the expression-language overview — precedence, truthiness, unary/comparison/arithmetic operators, variable access, parse modes, null propagation, and the complete function catalog grouped by category — with an `Example` column where layout permits. `--side-effects` shows the capability catalog with `Capability`, `Description`, `Safety`, and `Example` columns; the `Example` column is hidden below 70 characters to preserve the minimum-supported-width floor. It is documentation-only and does not invoke, probe, or check availability of any capability. All reports share a 140ch-inclusive width contract, inverse-styled inline code, and `UnorderedList` bullet formatting.

**Wrapper & composition notes:**

- **Argv pre-parsing** — `argv::normalize` rewrites composition-subcommand argv before clap (provider booleans → `--provider`, `--help` hoisting, `--` separator insertion). See [CLI Pre-Parsing](../../../claudine/docs/topics/cli-pre-parsing.md).
- **System prompt** — file-backed only: `--append-system-prompt`/`--asp` and `--replace-system-prompt`/`--rsp`, with `system-prompt.md` discovery from the launch-CWD hierarchy; direct wrappers also support `--edit`. Delivery is spec-driven per provider. See [System Prompt](../../../claudine/docs/topics/system-prompt.md).
- **Timeouts** — two rules, `timeout` (wall-clock, opt-in) and `step_timeout` (stream-silence, default `30m`). See [Timeouts](../../../claudine/docs/topics/timeouts.md).
- **Runaway content guards** — three *volume*-driven backstops the time-driven timeouts can't catch (a child that floods rather than goes silent): user-authored `exit_expressions` (literal/regex, scoped `{agent}/{model}`), group-cycle `runaway_repetition` (≥30 cycles, on by default), and a `runaway_volume` cap (50k lines / 32 MiB, on by default). They scan `OutputText`/`Reasoning` only — never tool payloads — and all map to `ProcessTermination::Aborted` → `AgentFailure` (fail-fast, **never** the `handle_timeout:` retry that would reproduce the runaway). The honest per-guard `error_kind` + `guard_context` thread into the programmatic `handle` payload (`CLAUDINE_ERROR_KIND` env + JSON). See [Timeouts — Content guards](../../../claudine/docs/topics/timeouts.md#content-guards-runaway-output).
- **Unified Ctrl+C** — every spawn path (`run_child`, `run_child_capture`, `run_child_stream_semantic`) routes through one signal-aware wait loop; the legacy no-signal `wait_with_timeout` was retired (it silently disabled Ctrl+C whenever a `timeout` was configured). Visible per-press stderr feedback, a shortened `SIGTERM → SIGKILL` ladder for non-interactive runs (F5), and a real Windows Job-Object/console-event implementation. **Outside** a child wait loop (prep, between loop iterations, blocking post-execute lifecycle side effects) the compose-scoped guard force-exits with `_exit(130)` on a second press (gated on a `wait_loop_active` depth counter), and the blocking lifecycle emitters (TTS, sound `effect`) are bounded by `run_blocking_with_timeout` — together closing the gap where a wedged synchronous call between phases could ignore Ctrl+C entirely. See [Signal Handling](../../../claudine/docs/topics/signal-handling.md).
- **Transient overlays** — written to `<repo_root>/.claudine/tmp/` (or `<launch_cwd>/.claudine-tmp/`) and cleaned up on `Drop`.
- **Schema validation** — when a composition document declares `$schema`, the prepare layer runs Darkmatter's `SimplifiedSchema` validation and emits typed claudine errors (`SchemaLoad`, `SchemaValidation`, `MissingProperties`, `UnsupportedInteractiveSchema`). Optional properties accept `null` as a sentinel for absent (equivalent to missing). Required-missing values trigger a `biscuit-tui` prompt loop when stdin+stderr are TTYs, `--silent` is off, and the user-config `prompt_for_missing` is `true` (default). Invalid optional values are dropped with a warning and validation retries once. `sequence` aggregates per-step failures into `SequenceMissingProperties` so all steps can be fixed in one pass. See [Composition — Schema Validation](../../../claudine/docs/topics/composition.md#schema-validation).
- **Composition warnings** — unknown expression functions and unknown `ctx.*` references detected during prepare emit non-fatal did-you-mean warnings to stderr, suppressed by `--silent`. String literals and code fences do not trigger the `ctx.*` diagnostic because it is parsed from the interpolation AST. See [Composition](../../../claudine/docs/topics/composition.md).
- **Whole-value frontmatter expansion strictness** — **key invariant: a frontmatter value that is exactly one expansion form is executable state, not text, and must never leak into effective frontmatter as raw syntax.** Enforced in Darkmatter composition. A frontmatter value whose trimmed content is exactly one `{{ ... }}` span must parse and evaluate (parse/eval failures abort composition **even when `fail_fast` is off**; the typed result is preserved, and undefined variables stay lenient → `null`). A frontmatter value whose trimmed content is exactly one `$(...)` shell expression must parse and expand when frontmatter shell expansion is enabled (a value still trimming to a whole-value `$(...)` candidate after the expansion pass is rejected; when shell expansion is disabled the `$(...)` is deferred unchanged). The strictness is scoped to whole-value spans only — mixed strings (`"a {{ x }}"`, `"literal $(echo ok)"`) and body interpolation keep their lenient warning-based behavior when `fail_fast` is off. This is the Darkmatter-layer defense complementing claudine's own lifecycle leak/undefined guards below. See [Composition — Whole-Value Frontmatter Expansion](../../../claudine/docs/topics/composition.md#whole-value-frontmatter-expansion-is-executable-state).
- **Lifecycle interpolation leak guard** — after Darkmatter composition, `prepare_direct`/`prepare_inline` scan every rendered lifecycle string across all seven events (`initialize`/`start`/`success`/`blocked`/`failure`/`finalize`/`loop`) and every communication surface (`say`/`say_first`/`message`/`stderr`/`notify`/`info`/`warn`/`success`), plus every reachable stack expression surface (`when:` clauses, action arguments, communication message bodies, shell commands, side-effect args, control-action operands), for surviving `{{ … }}` spans using Darkmatter's own `ExpressionFinder`. The first leak aborts preparation with `CompositionError::LifecycleInterpolationLeak { source_path, property, expression, reason }`, rendered through the standard `BlockError` path, so no lifecycle side effect (Discord/Slack/TTS/sound/stderr/notification) is dispatched with raw template syntax. This is defense-in-depth against Darkmatter's default non-fail-fast leniency for malformed or unresolvable expressions.
- **Lifecycle undefined-variable guard** — the leak guard only catches spans that *survive* composition. An unknown bare `{{ missing }}` instead resolves to an empty string with no Darkmatter warning (even in fail-fast mode), so it would silently dispatch a degraded message. `validate_no_undefined_lifecycle_variables` closes that gap by re-reading the **raw** (pre-composition) lifecycle strings — where the span is still present — and rejecting any frontmatter-scoped bare variable whose root key is absent from the composed frontmatter, with `CompositionError::LifecycleUndefinedVariable { source_path, property, variable }`. The check walks the whole parsed expression tree, so an undefined variable buried in a function argument (`{{ parent_dir(missing) }}`), comparison, arithmetic node, or ternary condition fails just like a top-level `{{ missing }}`. Fallback (`{{ x || 'y' }}`) subtrees and ternary branch operands are skipped wholesale because they intentionally tolerate undefined operands, while ternary conditions are descended. `ctx.*`/`env.*`/`doc` references resolve outside the frontmatter and are skipped.
- **Lifecycle stack-only globals** — inside lifecycle stack expressions (`when:` clauses and action arguments), three globals are available in addition to frontmatter/`ctx.*`/`env.*`/`doc.*`: `err` (`kind`, `variant`, `msg`) for `blocked`/`failure`/optional-error `finalize`, `timing` (`document_ms`, `total_ms`, `step_ms`), and `current` (`current.ctx.*`, `current.env.*` lazy snapshots at event time). Bare `err` references in no-error events (`initialize`, `start`, `success`, `loop`) are rejected at parse time; `doc.err` reaches a literal frontmatter `err` property everywhere.
- **Lifecycle action `no_error`** — the existing `shell.no_error` escape hatch is extended to every action category (communication, shell, side-effect, expression-function). An errored action marked `no_error: true` is logged but does not stop the stack or change the composition outcome.
- **Lifecycle `stdout` channel** — `stdout` is a statusless communication channel (field + `stdout(...)` action) that writes plain prose to stdout. It is the lone lifecycle channel targeting stdout (all others go to stderr/messaging/desktop), so it interleaves with composed/provider output — authors opt in deliberately. `stderr` is likewise statusless; `info`/`warn`/`success` carry a `Status` glyph.
- **Frontmatter YAML blocks in errors** — every composition error rooted in a prompt file's YAML frontmatter appends the authored frontmatter as a syntax-highlighted, line-numbered `darkmatter::CodeBlock` (delimiters included so block line N equals file line N), highlighting the offending line when the error's property maps to a locatable key. Covers the lifecycle guards (leak, undefined-variable, say-conflict, unknown-effect, invalid), prompt/agent/model/interactive type errors, schema errors, the inline-compose / sequence mismatch, and body-composition failures (`ComposeFailed`/`ShellExpansionFailed`, no highlight). Mechanism: `composition::FrontmatterExcerpt` (module `frontmatter_excerpt`) captures the block + highlight line; `CompositionError::enrich_frontmatter(source, stderr_is_tty)` wraps a frontmatter-rooted error in the transparent `CompositionError::WithFrontmatter { inner, excerpt }` variant at the **render boundary** (CLI handlers — `compose.rs`, after all control-flow `match`es), so upstream variant matching is unaffected; the CLI error walker (`output::error_walker`) renders the deepest typed block from `inner` and appends `excerpt.render_appendix(term)`. **TTY-gated**: withheld in non-TTY / piped / `NO_COLOR` output, unless `FORCE_COLOR=1` is set. `report_block_error` strips escapes for every variant at `ColorDepth::None`. This superseded the inline-compose mismatch's bespoke verbatim-YAML dump (the `raw_yaml`/`stderr_is_tty` fields were removed).

- **Protect extraction and posture** — `protect::observe` returns `ProtectObservation::Request | NoOpinion | Unparsed`. Bash-shaped tools are recognized by name (`bash`, `shell`, `exec`, `run_command`, `terminal`) and payload keys `command`/`cmd`/`script`/`input` or string arrays; write-shaped tools by name (`write`/`edit`/`create`/`delete`) and path keys `path`/`file_path`/`file`/`target`/`filename`/`dest`/`paths[]`. Unparsed command/write-shaped tools are blocked defensively with a `warn!`. `allow_paths` uses anchored component-sequence matching for relative entries and boundary-aware prefix matching for absolute entries. Custom patterns support `surface: bash_command | mcp_response` (default `bash_command`; `write_path` is rejected at validation). Protect remains best-effort defense-in-depth, not a security boundary. See [Protect Service](../../../claudine/docs/topics/protect-service.md).

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
- [Timeouts](../../../claudine/docs/topics/timeouts.md) — the two timeout rules, four env vars, precedence, termination path, exit reasons, and the three runaway content guards
- [Signal Handling](../../../claudine/docs/topics/signal-handling.md) — user Ctrl+C vs wrapper-driven SIGTERM/SIGKILL, the unified wait loop, termination labels (`Aborted`/`Interrupted`/`TimedOut`), non-interactive ladder, Windows parity
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
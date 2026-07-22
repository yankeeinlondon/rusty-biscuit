# Claudine Architecture

Deep technical documentation for Claudine's event model, provider adapters, dispatch pipeline, stream parsing, composition, and configuration system.

## Library Module Structure

```
claudine/lib/src/
├── actions/      → Hook action types and response model
├── badges/       → Styled terminal badge constants (YOLO, Non-Interactive, Interactive, etc.)
├── composition/  → Markdown frontmatter composition (inline and chained prompt pipelines)
│   ├── lifecycle/ → Lifecycle config/types, parsing, validation, actions, context, control, execution, and the pure transition core shared by composition preflight and the harness loop
│   ├── looping/   → Loop configuration, action DSL, condition evaluation, and execution orchestration (looping/{engine,types,seed,config,dsl,actions,expression}.rs — engine holds only execution/routing/gate logic; types holds the option/context/output/result value types; seed holds loop-seed construction)
│   └── schema/    → Schema-aware preparation, error translation, problem classification, and status reporting
├── config/       → Agent detection, hook registration, atomic writes, backups
├── dispatch/     → Event processing pipeline
│   ├── logging.rs        → Event metadata preparation and JSONL logging
│   ├── protect_bridge.rs → Protect observations mapped into hook responses
│   └── wrapper_flags.rs  → Wrapper environment flags and repository root extraction
├── events/       → Normalized event model and types (16 events, 10 providers)
├── hook_adapters/ → Native hook request/response adapters (ProviderAdapter trait) — parse provider hook payloads; distinct from stream/providers (stdout NDJSON parsers)
├── linking/      → Cross-provider skill synchronization (4 resource types) with portability classification
├── mcp/          → MCP catalog, defaults, import/export, session, and injection
├── permissions/  → Provider-agnostic PolicyEngine for permission queries and mutation planning
│   └── providers/common.rs → Format-agnostic helpers shared across provider backends (first_source_id, one_shot_plan constructor)
├── render/       → Functional render components (FinalMessage, AgentPrompt/SystemPrompt, EventRenderer + DISPATCH table, MetricsReport, StreamRenderable/AssistantStream); consume data + policy (DisplayPolicy), never `match provider`
├── reporting/    → JSONL-to-SQLite reporting index, sync, and typed queries
├── services/     → Cross-provider runtime policy services (ProtectService)
├── stream/       → Structured stream parsing for 8 providers (Kilo reuses OpenCode's) + summary/reporting
│   ├── logs/opencode/bridge/   → Stderr bridge: ingest dispatch, session tracking, stall guard, signals, formatting
│   ├── logs/opencode/classify/ → Error classification: asset, LLM, session, text utilities
│   └── logs/opencode/state.rs  → Shared stderr state and summary merge
└── error.rs      → ClaudineError enum
```

The per-provider modules under `lib/src/provider/<slug>/` split into two halves: `data.rs` is **generated** by `claudine-gen` (crate `claudine/gen`, sharing vocab enums with the leaf `claudine/catalog-types` crate) from roster + facts + research + overrides (regenerate with `claudine providers generate`; drift-checked in CI by the gen crate's drift test / `claudine-gen check`, which also verify the committed `docs/providers/catalog.json` superset), while `behavior.rs` is hand-written. Never edit a `data.rs` by hand — change the owning input file and regenerate.

**Dispatch drift guard (Phase I).** Decentralized `match Provider` / `matches!` / `==` / `!=` dispatch is prevented from regrowing by one site-level guard in `claudine-cli/tests/dispatch_inventory.rs`, covering **both** `lib/src` and `cli/src` (it retired the lib crate's earlier regex `no_unauthorized_match_provider_in_lib` guard). Every conditional, non-exempt dispatch site must be grandfathered in `GUARD_ALLOWLIST` with a tag + reason (the current sites are all `keep` — genuinely behavioral wire/shadow-HOME/stderr-bridge quirks and Claude's canonical linking role); a new one fails until migrated to a `ProviderInfo` field/trait or consciously listed. The live count is the allowlist length printed by the guard; the committed census is `docs/providers/dispatch-inventory.json`.

## Event Support Matrix

| Event | Claude | Codex | Gemini | Goose | Kimi | OpenCode | Qwen | Kilo | Pi | Antigravity |
|-------|:------:|:-----:|:------:|:-----:|:----:|:--------:|:----:|:----:|:--:|:-----------:|
| session_start | ✓ | ○ | ✓ | - | - | ✓ | - | ✓ | ○ | - |
| session_end | ✓ | - | ✓ | - | - | ✓ | - | ✓ | - | - |
| before_prompt | ✓ | ○ | ✓ | - | ○ | ✓ | - | ✓ | - | - |
| before_tool | ✓ | ○ | ✓ | - | ○ | ✓ | - | ✓ | ○ | ✓ |
| after_tool | ✓ | ○ | ✓ | - | ○ | ✓ | - | ✓ | ○ | ✓ |
| tool_error | ✓ | ○ | - | - | ○ | - | - | - | - | - |
| permission_request | ✓ | - | - | - | ○ | ✓ | - | ✓ | - | - |
| human_in_the_loop | ✓ | - | - | - | - | - | - | ✓ | - | - |
| turn_complete | ✓ | ✓ | ✓ | ○ | ○ | ✓ | ○ | ✓ | ○ | ✓ |
| turn_error | - | ○ | - | ○ | ○ | ✓ | ○ | ✓ | ○ | - |
| subagent_start | ✓ | - | - | ○ | ○ | - | - | - | - | - |
| subagent_stop | ✓ | - | - | ○ | ○ | - | - | - | - | - |
| before_model | - | - | ✓ | - | - | ✓ | - | ✓ | - | ✓ |
| after_model | - | ○ | ✓ | ○ | ○ | ✓ | ○ | ✓ | ○ | ✓ |
| before_compact | ✓ | - | ✓ | - | ○ | ✓ | - | ✓ | ○ | - |
| notification | ✓ | ○ | ✓ | ○ | ○ | ✓ | ○ | ✓ | - | - |

**Legend:** ✓ = Hook support (config file), ○ = NonHook (wrapper/proxy/stream-parse required), - = Not supported. The authoritative source is each provider's generated `lib/src/provider/<slug>/data.rs` `event_mapping` (or `claudine hooks --support`), where `○` further splits into StreamParse / WireProxy / Wrapper / Acp (🅐).

## Key Types

### AgenticEvent Enum

16-variant enum with snake_case serde, descriptions, payload schemas, return schemas, and abbreviations:

```rust
pub enum AgenticEvent {
    SessionStart, SessionEnd,
    BeforePrompt,
    BeforeTool, AfterTool, ToolError,
    PermissionRequest, HumanInTheLoop,
    TurnComplete, TurnError,
    SubagentStart, SubagentStop,
    BeforeModel, AfterModel,
    BeforeCompact,
    Notification,
}
```

### Provider Enum

7-variant enum (Claude, Codex, Gemini, Goose, KimiCode, OpenCode, QwenCode) with slug, docs URL, event support queries, and native event name mappings:

- `EventSupportLevel` — `Hook` | `NonHook` | `NotSupported` per provider-event pair

### HookAction Enum

6-variant tagged enum:

```rust
pub enum HookAction {
    Speak { message: String },
    Log { target: LogTarget },
    Report { handler: Option<ReportHandler> },
    SoundEffect { name: String, volume: f32, speed: f32 },
    FireAndForget { command: String, args: Option<Vec<String>> },
    Call { command: String, args: Option<Vec<String>>, timeout: Option<u64>, mapper: Option<Mapper> },
}

pub enum LogTarget {
    File { path: PathBuf },     // with daily rotation
    Server { url: Url },        // HTTP POST with timeout
}
```

### HookResponse / HookDecision

- `HookResponse` — Unified response a hook can return (decision, reason, updated input, additional context)
- `HookDecision` — 4-variant enum: `Allow`, `Deny`, `Ask`, `Continue`

### EventMeta

Normalized event metadata: provider, event, tool name, error, prompt, session ID, timestamps, environment context.

### EnvironmentContext

Auto-detected OS, hardware, git, and repo context (via `sniff`).

### Composition Session Interactivity

Composition commands resolve whether a session runs interactive via `SharedComposeArgs::resolve_session_interactivity`, which combines CLI flags with the authored `interactive` frontmatter property parsed into `EffectiveSelectionHints::interactive` by `parse_interactive_hint`:

1. `--no-interactive` → non-interactive (`SessionInteractivitySource::NoInteractiveFlag`)
2. `-i` / `--interactive` → interactive (`SessionInteractivitySource::InteractiveFlag`)
3. `interactive: true` / `false` frontmatter → that value (`SessionInteractivitySource::Frontmatter`)
4. Otherwise → non-interactive (`SessionInteractivitySource::Default`)

`--interactive` and `--no-interactive` are mutually exclusive at the clap level. The resolved value and its source are stored on `CompositionExecutionRequest` (`session_interactive` and `session_interactive_source`) so downstream diagnostics and dry-run metadata can attribute the mode. `claudine sequence` rejects `interactive: true` frontmatter because a sequence is serial automation; use the explicit `--interactive` flag when an interactive sequence step is required.

## Provider Adapters

Each provider has its own adapter implementing the `ProviderAdapter` trait. The `adapter_for(provider)` factory returns the appropriate adapter. Each adapter normalizes the provider's native JSON payload into `(AgenticEvent, EventMeta)` and can format `HookResponse` back into provider-native response payloads.

| Adapter | Parses | Status |
|---------|--------|--------|
| `claude` | `hook_event_name` field from settings.json hooks | Implemented |
| `codex` | JSONL stream fields + notify hook | Implemented |
| `gemini` | Settings.json hook events | Implemented |
| `opencode` | Plugin-based event names | Implemented |
| `goose` | Stream-json + env var (type/event field) | Implemented (non-blocking) |
| `kimicode` | Wire mode JSON-RPC (event_name/method field) | Implemented (blocking: tool, permission) |
| `qwen` | Stream-json output (event_name/type field) | Implemented (blocking: permission) |

### Claude Code

Hooks receive JSON on stdin, return JSON with control fields:

```json
// Input
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "npm test" }
}

// Output (optional)
{
  "hookSpecificOutput": {
    "permissionDecision": "allow|deny|ask",
    "updatedInput": { "command": "modified" }
  }
}
```

Exit codes: `0` = success, `2` = block action

**Stdin auto-detection**: provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need `--provider`.

### Codex CLI

Uses JSONL stream (`codex exec --json`) plus `notify` hook for turn_complete:

```json
{"type": "thread.started", "thread_id": "abc123"}
{"type": "item.completed", "item": {...}}
{"type": "turn.completed", "usage": {...}}
```

### Gemini CLI

Similar to Claude but with differences:
- Hooks have `name` field (Claudine uses `claudine-<event>`)
- Timeouts in milliseconds

### Goose

Events via stream-json output and `GOOSE_STATUS_HOOK` env var (NonHook):

```json
{"type": "complete", ...}
{"type": "message", ...}
{"type": "notification", ...}
```

### Kimi Code

Wire mode JSON-RPC proxy (NonHook, blocking for tool and permission events):

```json
{"method": "TurnBegin", "params": {...}}
{"method": "ToolCall", "params": {...}}
{"method": "TurnEnd", "params": {...}}
```

### OpenCode

Plugin-based hooks via `opencode.json`:

```typescript
export default (async ({ client, project }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        execFile("claudine", ["handle", "session_start"], {...})
      }
    }
  }
}) satisfies Plugin
```

### Qwen Code

Events via stream-json output (NonHook, blocking for permission):

```json
{"type": "result", ...}
{"type": "assistant", ...}
{"type": "system", ...}
```

## Dispatch Pipeline

The core event processing pipeline runs in 6 steps:

1. **Select adapter** — `adapter_for(provider)`
2. **Parse event** — adapter normalizes raw JSON into `(AgenticEvent, EventMeta)`
3. **Load config** — merges user (`~/.claudine/config.json`) and repo (`.claudine/config.json`) configs, precompiling matcher and mapper regexes
4. **Look up binding** — finds `RuntimeEventBinding` for the canonical event (provider-agnostic), checks enabled and non-empty actions
5. **Check matcher** — precompiled regex match against event metadata (filters actions)
6. **Execute actions** — runs each action via `runner::execute_actions()`, collecting blocking responses from `Call` actions

### Dispatch Sub-modules

- `logging` — Event metadata preparation, compact tool-detail rendering, and daily JSONL event logging
- `protect_bridge` — Protect observation evaluation and blocked-decision translation into hook responses
- `wrapper_flags` — Wrapper environment flag and runtime repository-root extraction
- `loader` — Config file discovery, loading, merge logic, runtime compilation (matchers + mappers), and config save/validation
- `template` — `{{placeholder}}` Handlebars-style interpolation engine with 28 variables across 5 categories (legacy `{placeholder}` single-brace syntax is deprecated with warnings)
- `matcher` — Regex-based event filtering against tool name, notification type, or error
- `runner` — Executes actions (TTS via biscuit-speaks, logging, shell commands, sound effects via playa, report formatting), evaluating per-action `when` conditions via the composite lookup

### Expression Lookup Architecture

All dispatch surfaces that evaluate expressions against an `EventMeta` share the same base adapter:

```text
                      ┌─────────────────────────────────────────────┐
                      │ EventMetaExpressionLookup                   │
                      │ (single source of truth for event paths)    │
                      └─────────────────────────────────────────────┘
                                    ▲
              ┌─────────────────────┴─────────────────────┐
              │                                           │
     templates (template.rs)                    matchers (matcher.rs)

                      ┌─────────────────────────────────────────────┐
                      │ EventMetaConditionLookup                    │
                      │ - composite EvaluationLookup                │
                      │ - ctx.* → Darkmatter ctx capture            │
                      │ - everything else → delegates to            │
                      │   EventMetaExpressionLookup                 │
                      └─────────────────────────────────────────────┘
                                    ▲
                                    │
                      Hook `when` (runner.rs::evaluate_when)
                      → parse_condition(expr) + evaluate(parsed, &lookup)
```

`EventMetaExpressionLookup` resolves `env.NAME`, `extra.<path>`, `tool_input.<path>`, `tool_response.<path>`, `os.*`, `hardware.*`, `git.*`, `project.*`, and top-level event fields. `EventMetaConditionLookup` layers `ctx.*` (e.g. `ctx.today`) on top for hook `when` evaluation. The old JSON-serialize-and-flatten path was removed by feature `2026-05-02-flattened-bridge`.

### Config Merge Strategy

User config (`ClaudineConfig`) is the full source of truth; repo config (`RepoOverrideConfig`) is an optional overlay where every field is `Option`-like. Per-event `actions` replace user-level actions per canonical event (a repo entry for `BeforeTool` fully replaces the user's `BeforeTool` actions; events not present in the repo config fall through to user). `canonical_provider` overrides when set. `messenger_override` uses three-state semantics: absent = inherit, `null` = disable, object = override. Global toggles (`logging`, `protect`, `preferred_agent`, etc.) live only in the user config.

## Configuration Schema

```rust
// User scope: ~/.claudine/config.json
pub struct ClaudineConfig {
    pub tts: TtsValue,
    pub messenger: Option<ClaudineMessengerConfig>,
    pub logging: bool,
    pub protect: ProtectConfig,
    /// Canonical-event → actions (provider-agnostic).
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,
    pub preferred_agent: Provider,
    pub canonical_provider: Option<Provider>,
    pub default_sounds: DefaultSounds,
}

// Repo scope: <repo>/.claudine/config.json (all fields optional)
pub struct RepoOverrideConfig {
    pub canonical_provider: Option<Provider>,
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,
    pub messenger_override: /* three-state: absent / null / object */,
    // ...other optional overrides
}
```

### Config Management

- `detect_agents()` — returns detected providers with their configurators
- `discover_agents_full()` — all 10 providers with install/registration status (`AgentInfo`)
- `get_configurator(provider)` — returns the configurator for a specific provider
- `AgentConfigurator` trait — `register()`, `deregister()`, `is_registered()`, `registered_events()`, `create_minimal_config()`, `supports_config_registration()`, `registerable_events()`, `is_cli_installed()`

Configurators handle each provider's config format:
- **Claude/Gemini**: JSON `settings.json` with hooks array
- **Codex**: TOML `config.toml` with notify section (format-preserving via `toml_edit`)
- **OpenCode**: JSON `opencode.json` with plugins
- **Goose/KimiCode/Qwen**: Wrapper-only (no config-based registration)

Atomic file writes (`config::atomic`) prevent corruption during concurrent access. Config backup utilities (`config::backup`) preserve originals before modification.

## Template Interpolation

28 variables in 5 categories. Template regex is lazy-compiled via `LazyLock<Regex>`.

### Event Fields

| Placeholder | Field |
|-------------|-------|
| `{{provider}}` | `meta.provider` |
| `{{event}}` | `meta.event` |
| `{{session_id}}` | `meta.session_id` |
| `{{cwd}}` | `meta.cwd` |
| `{{tool_name}}` | `meta.tool_name` |
| `{{error}}` | `meta.error` |
| `{{prompt}}` | `meta.prompt` |
| `{{timestamp}}` | `meta.timestamp` |
| `{{agent_type}}` | `meta.agent_type` |
| `{{notification_type}}` | `meta.notification_type` |

### Context Fields (auto-detected at runtime)

| Namespace | Placeholders |
|-----------|--------------|
| `os.*` | `{{os.name}}`, `{{os.type}}`, `{{os.version}}`, `{{os.hostname}}` |
| `hardware.*` | `{{hardware.arch}}`, `{{hardware.cpu}}`, `{{hardware.cores}}` |
| `git.*` | `{{git.branch}}`, `{{git.is_dirty}}`, `{{git.head_sha}}`, `{{git.head_message}}`, `{{git.remote}}`, `{{git.hosting}}`, `{{git.repo_name}}`, `{{git.repo_org}}` |
| `project.*` | `{{project.language}}`, `{{project.is_monorepo}}`, `{{project.monorepo_standard}}`, `{{project.monorepo_orchestrators}}`, `{{project.monorepo_tool}}` (deprecated alias) |

Shell environment variables are also supported via `{{env.VAR_NAME}}` with optional defaults: `{{env.MY_VAR || "fallback"}}`. The legacy single-pipe `|` form is no longer supported.

Unknown placeholders are left as-is. `None` values render as empty strings.

## Stream Parsing

Provider-native structured stream parsing for wrapped non-interactive sessions. Each provider's structured output (stream-json, JSONL, or NDJSON) is parsed live, extracting clean assistant text for stdout and metadata for stderr summaries and JSONL reporting.

### Section Model

All non-interactive runs follow a **9-section model** for rendered output, ensuring consistent spacing and structure across providers:
1. **Execution line** — header line withbadges. _stderr._
2. **ENV variables** — sanitized environment details. _stderr._
3. **System Prompt** — effective system prompt. _stderr._
4. **Agent Prompt** — user's startup prompt. _stderr._
5. **Session ID / Model** — provider-specific session metadata. _stderr._
6. **Thinking Prose** — dim-italic `BlockQuote` for reasoning feedback. _stderr._
7. **Tool / Info Events** — canonical `ToolCallDisplay` and status lines. _stderr._
8. **Final STDOUT** — reconstructed assistant response text. _stdout._
9. **Final Metadata** — timing, usage, cost, and summary line. _stderr._

Spacing is enforced at the sink level, with at most one blank line between any two sections.

### Markdown rendering boundary (triage)

The prose-bearing sections (System Prompt, **Agent Prompt**, Thinking Prose) render Markdown through `render::prompt::render_markdown_for_terminal` (the `AgentPrompt`/`SystemPrompt` components under `lib/src/render/prompt/`, which absorbed the former `prompt_reporting` module), which is **pure delegation** to darkmatter's `Markdown::as_terminal` — it only sets `max_width` and collapses blank lines. claudine owns **no** word-wrap, hanging-indent, or inline-style (code/bold/link) logic; all of that lives in darkmatter's fold + biscuit-terminal's render tree (`render_tree/render.rs`).

So when rendered prompt output shows wrong wrapping, spurious newlines, lines bleeding past the width, or mis-styled inline spans, the defect is in **darkmatter / biscuit-terminal**, not claudine. Reproduce at that layer (`md.as_terminal` with a fixed `max_width`) rather than through the claudine CLI. Known gotcha: a CommonMark *tight* list item carries its content as a flat run of inline siblings (`[Text, InlineCode, Text]`) with no wrapping `Paragraph`, and the terminal renderer must coalesce that run before wrapping — the wrap is per-list-item, not per-inline-node.

### Provider Parsers (6)

| Parser | Format | Summary source |
|--------|--------|----------------|
| `claude` | stream-json | `result` event with duration, usage, cost, turns |
| `codex` | JSONL (`exec --json`) | `turn.completed` usage + `--output-last-message` file for text |
| `gemini` | stream-json | `result.stats` with token counts |
| `kimi` | stream-json | Latest `StatusUpdate` snapshot (no aggregate result) |
| `opencode` | NDJSON (`json`) | Accumulated per-step usage/cost |
| `qwen` | stream-json | Final result/usage event |

### Infrastructure

- `providers/common` — shared parser skeleton the per-provider parsers delegate to: `base_extra`/`base_extra_parts` payload bases, `emit_provider_extension` + `emit_malformed_warning` fallbacks, `finish_summary` (stamps `provider` + derived badges onto a `..Default::default()`-built summary), and the `ErrorKeywords` classifier shape + `classify_error_by_keywords` cascade. The ordered tables live in generated `providers/vocabulary.rs`, projected from the schema-validated `docs/research/agent-errors/<slug>.md` frontmatter; evidence stays in research while bucket/item order survives as the runtime precedence contract. Immutable `docs/research/agent-errors/_seeds/<slug>.yaml` baselines preserve pre-graduation row identity for deterministic removal/re-kind/reorder checks. `providers/vocabulary_tests.rs` locks accepted research additions, precedence, exact numeric codes, and representative near misses. Provider files keep thin delegating methods plus their genuinely provider-specific typed dispatch.
- `parser` — `StreamParser` trait and `StreamEventSink` callback interface for coarse event handling (session start, turn lifecycle, tool events)
- `summary` — `StreamExecutionSummary` struct: provider-agnostic metadata (session ID, model, tokens, cost, duration, tool calls, rate limits, context usage)
- `token_usage` — `NormalizedTokenUsage` with input/output/total/cache_read fields
- `stderr` — Verbosity-aware stderr formatting (start summary, completion summary, compact line for `--quiet`)
- `reporting` — Converts `StreamExecutionSummary` to `EventMeta` for synthetic JSONL summary events

### Execution Modes (in CLI `wrap/exec.rs`)

- `run_child_stream()` — live parsing with assistant text piped to terminal
- `run_child_stream_capture()` — parsing with captured text for composition flows

## Composition

Markdown frontmatter-based composition pipelines for delivering prompts to provider sessions:

- **Inline composition** (`--frontmatter-prompt`): reads frontmatter `prompt` field as input, replaces document body with provider output
- **Chained composition** (`--compose`): composes full document as prompt without file mutation

The CLI executor under `cli/src/commands/wrap/composition/` keeps stable entry
points and public re-exports in `mod.rs`. `pipeline.rs` owns one
`CompositionAttempt` across ordered selection/launch, environment/MCP,
argv/system-prompt, lifecycle-runtime, initialize-routing, and provider-handoff
phases. Each phase returns `CompositionPhaseResult`, so completed dry runs,
blocked lifecycle routing, preparation failures, and normal progression stay
explicit without flattening attempt state into the entry module.

Other concerns remain split by responsibility:

- `selection.rs` — favorite-provider and model-override configuration loading
- `launch.rs` — launch-workspace selection and `--repo` detection enforcement
- `preflight.rs` — blocked/finalize lifecycle routing before provider launch
- `runner.rs` — the named per-iteration `run_composition_body` runner and its
  `CompositionRunCtx`
- `dry_run.rs`, `prep_context.rs`, `target.rs`, and `timeouts.rs` — rendering,
  shared discovery context, execution-target resolution, and timeout resolution

Composition execution headers are shared output helpers in
`cli/src/output/mod.rs`; they do not live in the executor pipeline.

### Sequences

`lib/src/composition/sequence/` owns the normalized plan; `cli/src/commands/wrap/sequence/`
owns orchestration. The split follows the two execution phases:

- `sequence/{model,normalize,reserved,source,grammar,data,expr}.rs` — typed step
  state, id/`sequence_id` generation, the reserved-key catalog, and the
  `<file-ref> [-> offset] [::op(args)]` source grammar. Data files load through
  `biscuit_file` and resolve through `FileReference::resolve_from(authoring_dir)`;
  string sources classify through `biscuit_file::ListFormat`.
- `sequence/preflight/` — the recursive task-graph loader. Walks inline tasks,
  `kind: task` / `kind: group` / `kind: group-catalog` files, and every `prompt:`
  document, keeping a canonical-path ancestry stack so a cycle reports its whole
  chain. Resolves shell bytes under an early-binding-only lookup so
  approved == executed.
- `sequence/task/` — `TaskExecution::run` → `TaskOutcome`. `run()` never returns
  `Err`: continuation is the scheduler's `fail_fast` decision, not an error
  escaping one task. `group.rs` adds serial and (via `std::thread::scope` plus a
  shared cursor) parallel groups.
- `composition/runtime_state.rs` — `RuntimeState`, the invocation-local cell
  holding accumulated `set` mutations and the `outputs` accumulator.
  `layered_set_overrides` is the single place the four-layer precedence
  (live frontmatter < user setters < mutations < reserved overlay) is encoded.
- `wrap/sequence/{jit,iterate,phase1c,task_run,task_frames}.rs` — just-in-time
  composition at each step's turn. `phase1c`'s validation compose and execution
  both route through `jit::compose_step`, so "validated == executed" holds
  without a second prepare implementation.

Two seams are worth knowing before editing:

- **YAML sources load through `composition::load_yaml_document`.** A `.yaml`
  sequence file is one document whose *root mapping is its frontmatter*. Both the
  initial resolution and the just-in-time re-read
  (`reload_composition_source`) must use that conversion; parsing a YAML source
  as plain Markdown yields an empty frontmatter that is indistinguishable
  downstream from a document declaring nothing.
- **`PrepareOptions::allow_empty_body`** is set only by a step that declares an
  executable. Such a step runs its task instead of the body, and a directly
  invoked `kind: sequence` YAML file has no body at all.

`LifecycleCatchProtocol` in `lib/src/composition/lifecycle/runtime.rs` is the
single provider-neutral owner of setup catch routing, terminal-slot
redesignation, finalize eligibility, active-error threading, and evaluation
error precedence. Every initialize/start/blocked catch and every
success/failure/loop terminal-evaluation path must consume the protocol's
requested steps and result. CLI adapters may build event contexts, execute
effects, and render the selected error; they must not independently reproduce
failure/finalize ordering or precedence.

## Harness Attempt Loop

`cli/src/commands/wrap/harness_orch/loop_control.rs` keeps one
`HarnessLoopState` across all retry, resume, and proxy iterations. It owns the
immutable run context alongside the attempt counter, prompt/session overrides,
retry/resume budgets, proxy chain, cached shell approvals, lifecycle guard,
run-level timing anchor, and accumulated performance data.

`run_harness_loop_inner` is an ordered coordinator over three typed phases:
prompt materialization/preflight (including lifecycle `start`), provider
attempt execution, and result classification/recovery. Re-entry and terminal
outcomes cross phase boundaries as `LoopStep` values. Provider command and
process details remain in `harness_orch/{launch,attempt}.rs`; lifecycle event
execution remains in `loop_control/lifecycle_events.rs`; and
`drive_terminal_recovery` in `loop_control/control_dispatch.rs` remains the
single terminal-tail executor for retry, resume, proxy, and finalize recovery.
Attempt preparation exposes separate prompt-preparation,
lifecycle-execution, and retry/proxy-control contracts so each transition can
mutate only the state family it owns.

## Error Handling — the audit before you commit

Full model in [error-architecture.md](error-architecture.md). The rules a change
to any error path must satisfy, and the guards that check them (`just test`,
`just lint-transport`):

1. **Retain the typed cause.** Concrete typed error, `#[from]`, a `#[source]`
   field, or `wrap_err` where the concrete source stays in the chain. Never
   `format!("…{e}")`, `map_err(|e| e.to_string())`, or a prose
   `reason`/`message` field that drops the value. `no_unallowlisted_typed_error_collapses`
   is provenance- and retention-aware, so `Foo { message: e.to_string(), source: e }`
   is *not* a defect — the chain is intact.
2. **Register a new `Diagnostic` impl** in `as_diagnostic`
   (`lib/src/diagnostics/discovery.rs`). `registry_lists_every_diagnostic_impl`
   re-derives the truth from the sources and fails in both directions. An
   unregistered impl is the motivating incident and is never allowlistable.
3. **Do not box a registered diagnostic you need to reach.** `Box<E>` on a chain
   publishes `Box<E>`; the walk skips `E` at every depth. Box the *context*
   instead, or unbox at the boundary (`Report::from(*error)`).
   `no_registered_diagnostic_is_reachable_only_through_a_box` covers both
   `#[source] Box<T>` fields and `Result<_, Box<T>>` returns.
4. **Set `role()` from what the variant forwards**, never from what it wraps. A
   wrapper over a typed Darkmatter cause is `Semantic` (a Darkmatter cause has no
   facets) — and must delegate `status_block` to that cause, or it replaces a
   rich block with one line of `Display`.
5. **Seed `detail()` from `null_detail_for(code)`.** Every declared key present;
   unavailable optionals `null`; never a top-level `null` for a registered code;
   never a key the catalog does not declare.
6. **Never invent a field.** A value the resolver cannot supply is `null` — not
   parsed from `Display`, not back-derived from a neighbouring facet.
7. **Extend the catalog additively.** New code or new detail field: non-breaking.
   Rename or removal: breaking — it silently kills author `when:` clauses.
   `code → disposition` stays 1:1; if two failures need different dispositions
   they are different codes.

**Adding an exception.** Both allowlists (`cli/tests/error_guards/*.toml`) key on
an enclosing **symbol** and require a `tag` and a substantive `reason`.
`retained` is permanent; any other tag is burn-down debt a follow-up spec closes.
A stale entry fails its own guard.

**Changing an error's behavior** means a pass over its rustdoc in the same
change, per the repo's authoring discipline — the rendering and propagation
claims in these doc comments are exactly the kind that drift.

## Test Placement

**Inline tests** (`#[cfg(test)] mod tests { … }`) are the default for small files. Once a file exceeds **~800 production lines** or its test module exceeds **~300 lines**, move tests to a sibling file declared via `#[cfg(test)] mod tests;` at the bottom of the parent. This pattern is already established in `lib/src/provider/`, `cli/…/wrap/composition/`, and `cli/…/wrap/exec/wiring/`.

`claudine-cli/tests/test_placement.rs` enforces this convention as a Level 1 package-area structural test. It scans every source tree in the Claudine family, counts production and inline-test modules written as `#[cfg(...test...)] mod ... { ... }` separately with a Rust-aware lexer, excludes generated sources only through explicit path/header rules, and rejects stale path-specific exceptions. Private modules and the supported Rust visibility forms (`pub`, `pub(crate)`, and `pub(super)`) are all recognized and governed by the same thresholds. Exceptions must be file-specific and explain why co-location materially clarifies a private invariant; stale or rationale-free entries fail the gate. The analyzer does not currently enforce a separate numeric ceiling for exceptions.

## Skill Linking

Cross-provider resource synchronization via symlinks and format-converted derived artifacts.

### Linkable Resources (4 types)

Skill, Command, Agent, Script

### Support Levels per Provider

Full, CustomFormat, Limited, None

### Algorithm (6 phases)

1. **Canonical selection** — elect one provider as the source of truth per `(scope, resource_type)` pair, preferring providers with existing valid assets
2. **Discovery** — scan provider directories for skills, commands, agents, and scripts
3. **Hashing** — xxHash each resource (recursive walk for skill directories, file content for single files)
4. **Conflict analysis** — classify resources as LinkCandidate, InSync, Conflict, or AlreadyLinked; also-reads-from providers are excluded from link targets to avoid redundant symlinks
5. **Compatibility classification** — parse canonical frontmatter, apply deterministic upgrades (alias duplication, name derivation), check required properties per target provider
6. **Apply** — create symlinks (absolute for user scope, relative for repo scope) or generate format-converted derived artifacts; never overwrites real directories

### Provider Skill Paths

| Provider | User Scope | Repo Scope | Also reads from |
|----------|-----------|------------|-----------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` | -- |
| Codex | `~/.codex/skills/` | `.codex/skills/` | `.claude/skills`, `.agents/skills` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` | -- |
| Goose | `~/.config/goose/skills/` | `.goose/skills/` | `.claude/skills`, `.agents/skills` |
| KimiCode | `~/.config/agents/skills/` | `.kimi/skills/` | `.claude/skills`, `.agents/skills`, `.codex/skills` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` | `.claude/skills`, `.agents/skills` |
| QwenCode | `~/.qwen/skills/` | `.qwen/skills/` | -- |

Note: OpenCode also reads `.claude/skills/` directly

## Exit Code Behavior

| Provider | Exit 0 | Exit 1 | Exit 2 |
|----------|--------|--------|--------|
| Claude | Success | Non-blocking error | Block action |
| Gemini | Success | Warning | Block action |
| Codex | Success | — | — |
| OpenCode | Success | — | Error |

## Capability Matrix

| Capability | Claude | Codex | Gemini | OpenCode |
|------------|:------:|:-----:|:------:|:--------:|
| Observe events | ✓ | ✓ | ✓ | ✓ |
| Block actions | ✓ | - | ✓ | ✓ |
| Modify tool input | ✓ | - | ✓ | ✓ |
| Inject context | ✓ | - | ✓ | ✓ |

## Key Lessons

- **Hook handlers must respond fast**: `claudine handle` enforces a hard **5-second execution deadline** (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`) to prevent blocking the parent agent session. When exceeded, the handler aborts and exits 124. Bash and messenger actions also have tighter 3s timeouts when running inside a hook handler. Phase-level tracing spans ensure any hang is diagnostic.
- **All 7 adapters are implemented**: each provider adapter has full event mapping, metadata extraction, and tests. Claude, Gemini, OpenCode, and Codex use config-based hooks; Goose, KimiCode, and Qwen parse stream-json or wire-mode payloads directly. KimiCode and Qwen support blocking responses; Goose is observation-only.
- **Sound effects are fire-and-forget**: TTS and sound playback spawn tokio tasks to avoid blocking the event pipeline. Log and report actions run inline because they're fast.
- **Atomic writes prevent config corruption**: all config file mutations go through `config::atomic` to handle concurrent hook firings safely.
- **Runtime config precompiles regexes**: matcher patterns and Call action mapper regexes are compiled once at config load time, failing fast on invalid patterns with contextual error messages.
- **Legacy single-brace templates are deprecated**: `{placeholder}` is automatically rewritten to `{{placeholder}}` with a tracing warning. New configs should use Handlebars-style double braces.

## Rendezvous Package-Area Family

`claudine/rendezvous/` is a first-class package-area family of **three crates** that back `claudine dashboard` and the (unwired) lifecycle `defer` scheduler. It follows a `core → {daemon, client}` dependency shape — both leaf crates depend on `rendezvous-core`, neither depends on the other.

| Crate | Path | Depends on | Public role |
|-------|------|-----------|-------------|
| `rendezvous-core` | `rendezvous/core` | *(leaf)* + `sniff` | Shared protobuf/gRPC stubs, `NodeIdentity`, `SignedEnvelope`/`EnvelopeSealer`/`EnvelopeInbox`, `DocumentId`/`ChunkId`, invitations, the sync wire framing (`rendezvous_core::sync`), and the typed `LocalEndpoint` contract (`local_endpoint`). Models and resolves the endpoint; performs **no** filesystem mutation and contains no listener. |
| `rendezvous-daemon` | `rendezvous/daemon` | `rendezvous-core`, `sniff` | The long-running service: the `RendezvousService` gRPC impl over the platform's local endpoint, the `redb → Loro → DuckDB` session-log pipeline, the register store, peer discovery/QUIC, and the direct-sync engine. Owns endpoint/data-root authorization, listener setup, and cleanup. Binary `main`; the gRPC and persistence layers live in library modules so integration tests exercise them without spawning a child. |
| `rendezvous-client` | `rendezvous/client` | `rendezvous-core` | Thin tonic gRPC client. Exposes the portable `connect(&LocalEndpoint)` every Claudine CLI call site uses, plus the `rendezvous-test-client` binary. |

Test commands run from the `claudine/rendezvous/` area justfile: `just check`, `just build`, `just test`, `just lint` each iterate all three crates (`test-l2` is a no-op — real-terminal tests do not apply). The root `cargo check --workspace --all-targets` includes all three. Native runtime coverage on macOS/Linux/Windows is gated by `.github/workflows/rendezvous-tests.yml` (no `continue-on-error` on any leg — cross-compilation is explicitly **not** accepted as Windows evidence).

### Local IPC — the typed endpoint

Authoritative doc: [`claudine/docs/rendezvous/local-ipc.md`](../../../claudine/docs/rendezvous/local-ipc.md). Read it before touching endpoint, daemon-boot, or connector code.

The load-bearing rules:

- **`LocalEndpoint::{UnixSocket(PathBuf), WindowsNamedPipe(OsString)}`** carries its own transport. There is deliberately no common `path()` accessor — a Windows pipe name is not a filesystem path. Use `as_unix_path()` / `as_windows_pipe_name()`; `Display` is lossy and human-facing only.
- **Per stable OS user.** `sniff::os::current_user_id()` supplies the effective UID (Unix/WSL) or process-token account SID (Windows). Never `$USER`/`%USERNAME%`. Resolution: `RENDEZVOUS_ENDPOINT` → `$XDG_RUNTIME_DIR/claudine/rendezvous/daemon.sock` (when the runtime dir passes inspection) → `<tempdir>/claudine-rendezvous-uid-<uid>/daemon.sock`; Windows: `\\.\pipe\claudine-rendezvous-sid-<sid>`. Failure is typed — **no username, `default`, or random fallback.**
- **One portable entry point.** `spawn_local_server(LocalEndpoint, DaemonConfig)`. `prepare_daemon` builds storage/projection/batcher/identity/registers/QUIC/discovery/workers/service exactly once, transport-neutral; `local_transport/{unix,windows}.rs` own *only* listener, accept, permission, and cleanup. A new transport must not grow a parallel boot path. `spawn_uds_server` survives only as a Unix test seam.
- **Data root** defaults to `<local-data-dir>/claudine/rendezvous` (`node.key`, `session.redb`, `projection.duckdb`), validated by the same `private_dir` contract as the Unix runtime directory. The legacy `<tempdir>/rendezvous-data` is never read or imported — a shared temp dir is not an ownership boundary, so an identity found there could be planted.
- **Overrides change location, not policy.** `--endpoint`/`RENDEZVOUS_ENDPOINT`, `--data-dir`/`RENDEZVOUS_DATA_DIR`. Tests use private temp parents or the core `test-support` feature; never weaken production checks for a fixture.
- **No production `cfg` branches at call sites.** Dashboard, requeue, hook forwarding, session reporting, and health probes all go through `rendezvous_client::connect`.
- The legacy vocabulary (`RENDEZVOUS_SOCKET`, `--socket`, `default_socket_path`, `ServerHandle::socket_path()`, the `socket` module) is **gone with no aliases**. `ServerHandle::local_endpoint()` replaces the last.

**Known open defect** (recorded in the fix's plan, deferred not forgotten): every `unix::serve` endpoint refusal runs *after* `prepare_daemon` has opened redb/DuckDB and spawned workers, and `PreparedDaemon` has no `Drop` — so a transport failure leaks those workers with storage handles open. Trips nextest's `leak-timeout` under parallel load (masked today by `retries = 3`) and risks the `DatabaseAlreadyOpen` trap in production. The fix is to bind before preparing.

### Session-log module boundary

`SessionLogManager` (`rendezvous/daemon/src/session_log/`) is the single public facade over the in-memory chunk map plus the redb source-of-truth and the DuckDB projection batcher. Its behavior is split across private sibling modules, each owning one responsibility, while the shared session state (`ChunkState`, `ManagerInner`, `SessionCursor`) and its Loro/redb-facing invariants stay private to the module tree:

- `append` — local append/rotation and the in-memory/on-disk read surfaces.
- `staging` — snapshot signing, version-vector advertising, delta export, and the two-phase stage→commit that validates a remote update/replace before it touches durable state.
- `rehydrate` — startup rehydration from redb, accepted-envelope replay (crash recovery), and DuckDB projection rebuild.
- `validate` — remote-document invariants: metadata identity, entry schema/monotonicity, and the append-only prefix guard (read-only over a staged doc; nothing persists).

Persistence ordering is invariant: an append persists the redb snapshot before mutating live in-memory state; the sync receive path persists the accepted envelope before committing the snapshot, so a crash window recovers on the next startup replay. The daemon's `sync` and `service` inline-test suites are divided by behavior into sibling `tests/` trees (sync: `envelope_validation`, `schema_validation`, `snapshot_replace`; service: `rpc`, `session_register`, `validation`).

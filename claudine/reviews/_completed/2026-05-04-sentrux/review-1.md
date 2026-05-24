---
title: Sentrux Quality Review — claudine package area
date: 2026-05-04
review_type: sentrux-baseline
quality_signal: 0.5372
suggestions: 29
suggestions_critical: 6
suggestions_urgent: 11
---

# Sentrux Quality Review — claudine package area

> **Method note:** The Sentrux MCP tools and `sentrux` CLI both required permissions
> not granted in this non-interactive session, so the analysis below is grounded in
> the existing `.sentrux/baseline.json` (`scan` snapshot from 2026-05-04) cross-referenced
> against direct codebase inspection. Each suggestion cites the file paths actually
> verified on disk.

## Baseline snapshot (`.sentrux/baseline.json`)

| Metric | Value | Indicator |
|---|---|---|
| `quality_signal` | **0.5372** | weak (geometric mean of all 5 metrics) |
| `coupling_score` | 0.2992 | high — > 25% files participate in cross-cluster edges |
| `cycle_count` | **2** | acyclicity fails — circular import chains exist |
| `god_file_count` | **1** | a top-N file is well above the size threshold |
| `complex_fn_count` | 133 | many functions exceed cyclomatic-complexity threshold |
| `max_depth` | **8** | call/import chains run 8 levels deep |
| `total_import_edges` | 595 | |
| `cross_module_edges` | **326 (54.8%)** | modularity weak — over half of edges cross module boundaries |

The two confirmed cycles, by direct inspection:

1. `lib/src/harness/error.rs` ⇄ `lib/src/harness/model.rs`
   `HarnessError` borrows `ValidationFailure` from `model.rs`, while `model.rs`
   imports `HarnessError` from `error.rs`.
2. `lib/src/provider/*` ⇄ `lib/src/stream/*`
   `provider/{claude,codex,gemini,kimi,opencode,qwen}.rs` import semantic stream
   parsers from `stream/`, while every `stream/*_semantic.rs` and
   `stream/{summary,badges,semantic,mod}.rs` import `provider::Provider`.

The "god file" is `cli/src/commands/wrap/mod.rs` at 4641 lines / 95 functions.

---

## `claudine`

The library crate. The data below covers the lib portion of the area
(`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/`).

### `critical`: Break the `harness::error` ⇄ `harness::model` cycle

**Problem.** Acyclicity (Martin 2003): `HarnessError` enum (`harness/error.rs`)
holds a `ValidationFailure` variant defined in `harness/model.rs`, while
`HarnessPlan` and friends in `harness/model.rs` reference `HarnessError` to
report parse failures. This is one of the two cycles counted in the baseline.

**Files touched.**

- `lib/src/harness/error.rs:5` — `use crate::harness::model::ValidationFailure;`
- `lib/src/harness/model.rs:8` — `use crate::harness::error::HarnessError;`

**Fix.** Extract the shared boundary type into a leaf module that neither
`error` nor `model` re-imports from each other.

```rust
// lib/src/harness/failure.rs  (new leaf module — depends on neither error nor model)
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationFailure {
    pub rule: String,
    pub source_path: PathBuf,
    pub detail: String,
    // …existing fields…
}
```

Then have both `error.rs` and `model.rs` `use crate::harness::failure::ValidationFailure;`.
Update `harness/mod.rs` to add `pub mod failure;` ahead of `error` and `model`.

---

### `critical`: Break the `provider` ⇄ `stream` cycle

**Problem.** Acyclicity (Martin 2003): every concrete provider in `provider/`
imports a sibling parser from `stream/`, and every parser in `stream/` reaches
back into `provider::Provider` to tag emitted events. This is the second cycle
counted in the baseline and the largest single contributor to the 54.8%
cross-module edge ratio.

**Files touched.**

- `lib/src/provider/{claude,codex,gemini,kimi,opencode,qwen,behavior}.rs` —
  each `use crate::stream::{*}_semantic::*StreamParser` and
  `use crate::stream::{ParserConfig, StreamProtocol, parser::SemanticStreamParser}`.
- `lib/src/stream/{mod,summary,semantic,badges,claude_semantic,codex_semantic,
  gemini_semantic,kimi_semantic,opencode_semantic,qwen_semantic}.rs` —
  each `use crate::provider::Provider`.

**Fix.** The `Provider` enum is a leaf identifier — it should not live in the
same module as the runtime wrappers that hold parser pointers.

1. Move just the `Provider` enum (and the pure functions `provider_info`,
   `PROVIDERS_DISPLAY_ORDER`, `OutputFormatSelector`) into a new
   `lib/src/provider_id.rs` (or `lib/src/ids.rs`).
2. Re-export it from both `provider::Provider` and `stream::Provider` for one
   compatibility cycle, then delete the re-exports.
3. After the move: `stream/*` imports `crate::provider_id::Provider`, and the
   concrete provider modules continue to import `stream/*`. The cycle is gone.

```rust
// lib/src/provider_id.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Provider { Claude, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Roo }
```

---

### `urgent`: Split the `dispatch::runner` god module

**Problem.** Modularity (Newman 2004) and complexity: `lib/src/dispatch/runner.rs`
is 2326 lines with 84 top-level functions and pulls in 18 separate `use`
statements covering actions, config, events, messaging, services, error and
provider. This file hosts the speak executor, the bash executor, the protect
short-circuit logic, the report formatter, the mapper machinery, and the JSON
helpers all in one place. It is one of the largest contributors to
`complex_fn_count: 133` and to the 326 cross-module edges.

**Files touched.**

- `lib/src/dispatch/runner.rs` (2326 lines, 84 fns, 49 inline tests)

**Fix.** Carve the file into focused submodules under `dispatch/runner/`:

```
lib/src/dispatch/runner/
├── mod.rs            // pub use … the orchestration entry point only
├── speak.rs          // execute_speak_from_claudine, tts_config_*
├── bash.rs           // execute_bash, run_command_blocking, warn_on_silent_token_split
├── report.rs         // execute_report, format_report, terminal_meta_{value,json}
├── mappers.rs        // apply_mapper, map_exit_code, map_json_*, map_regex_*
├── protect.rs        // should_short_circuit_call, decision_for_short_circuit, attach_protect_context
├── meta_json.rs      // event_meta_to_json, flatten_event_meta_aliases, strip_nulls*
└── decisions.rs      // should_replace_selected, parse_decision, dot_lookup
```

Each submodule keeps its inline `#[cfg(test)] mod tests` block, which preserves
the 49 existing tests and their colocation. The flat `use crate::actions::*`
import block at the top of each new file should narrow to only what that file
needs — this directly reduces cross-module edge count.

---

### `urgent`: Split the `harness::parse` parser module

**Problem.** Modularity and complexity: `lib/src/harness/parse.rs` is 2317 lines
with 89 top-level functions handling validation parsing, handler parsing,
overlay parsing, frontmatter extraction, and shape extraction in a single file.
56 inline tests have already accreted here.

**Files touched.**

- `lib/src/harness/parse.rs` (2317 lines, 89 fns)

**Fix.** Re-shape into `harness/parse/`:

```
lib/src/harness/parse/
├── mod.rs            // re-exports parse_harness_plan, has_harness_properties
├── validations.rs    // parse_checks, parse_single_validation, parse_validation_kind, validation_meta
├── handlers.rs       // parse_handlers, parse_handler_entry, parse_programmatic_handler, parse_handler_action, parse_failure_event
├── overlays.rs       // parse_set_overlay, normalize_handler_subject_key, tokenize_to_approved_command
├── frontmatter.rs    // extract_frontmatter_text, original_yaml_slice, reconstruct_yaml_snippet, build_rule_source
└── shapes.rs         // extract_shape, extract_file_ref, extract_string_field, extract_bool_field, extract_usize, extract_scalar_string
```

Use `pub(super) fn` on the helpers so the same internal API surface is
preserved without leaking implementation details to the rest of the crate.

---

### `urgent`: Split `lib/src/config/claudine_config.rs`

**Problem.** Equality (Gini 1912) and god-file pressure: `claudine_config.rs`
is 1955 lines and is imported by 13 sites in the dispatch loader alone. It
holds `ClaudineConfig`, gender enums, TTS values, voice selection, the
hierarchical merge logic, and all the `Default` impls.

**Files touched.**

- `lib/src/config/claudine_config.rs` (1955 lines)

**Fix.** Split by concern:

```
lib/src/config/
├── claudine_config.rs   // top-level struct + the merge entry point only
├── tts.rs               // TtsValue, VoiceSelection, Gender (already conceptually a subdomain)
├── messaging_block.rs   // Messaging-related config fields
└── merge.rs             // The repo-vs-user merge functions
```

Importers should switch to the narrower paths (`use crate::config::tts::Gender;`),
which lowers cross-module edge weight for the file.

---

### `urgent`: Split `lib/src/harness/validate.rs`

**Problem.** Modularity: `harness/validate.rs` is 1848 lines and houses every
validation rule (path checks, frontmatter checks, content checks, programmatic
checks) in one file, plus its own template-rendering logic.

**Files touched.**

- `lib/src/harness/validate.rs` (1848 lines)

**Fix.** Move each validation kind into its own `harness/validate/<kind>.rs`
file with a thin `mod.rs` that owns dispatch. The render helper that calls into
Darkmatter's `EventMetaExpressionLookup` should live in `validate/render.rs` so
it can be unit-tested independently.

---

### `urgent`: Group provider stream parsers under a sub-namespace

**Problem.** Modularity and redundancy (Kolmogorov): six provider-specific
semantic parsers — `claude_semantic.rs`, `codex_semantic.rs`,
`gemini_semantic.rs`, `kimi_semantic.rs`, `opencode_semantic.rs`,
`qwen_semantic.rs` — sit flat in `lib/src/stream/`, sized 1410–1916 lines each.
They share the same shape (line buffer → typed `*Event` enum → `SemanticEvent`)
but the flat namespace forces every consumer to know each parser's name.

**Files touched.**

- `lib/src/stream/{claude,codex,gemini,kimi,opencode,qwen}_semantic.rs`
- `lib/src/stream/mod.rs` (the `pub mod *_semantic;` declarations)
- `lib/src/provider/{claude,codex,gemini,kimi,opencode,qwen}.rs` (importers)

**Fix.** Group them under `stream::providers::`:

```
lib/src/stream/providers/
├── mod.rs            // pub mod claude; pub mod codex; …
├── claude.rs         // (was stream/claude_semantic.rs)
├── codex.rs          // (was stream/codex_semantic.rs)
└── …
```

Pair this with a `SemanticParser` trait whose `for_provider(p: Provider) ->
Box<dyn SemanticParser>` factory replaces the per-provider `match` blocks in
`provider/{claude,codex,gemini,kimi,opencode,qwen}.rs`. After this change the
provider modules import a single trait instead of a parser per file.

---

### `urgent`: Split `lib/src/stream/logs/opencode.rs`

**Problem.** God-file pressure inside `stream/logs/`: `opencode.rs` is 2066
lines, holding both the JSONL→event translation and 4+ inline test blocks for
badge/summary integration. This makes targeted edits and recompiles slow.

**Files touched.**

- `lib/src/stream/logs/opencode.rs` (2066 lines)

**Fix.** Split into `stream/logs/opencode/` with `events.rs` (translation),
`reasoning.rs` (the typed `Reasoning` variant routing called out in the area
skill), `errors.rs` (the `SemanticErrorKind` classification), and a `mod.rs`
that re-exports the same public surface. Tests follow their subject module.

---

### `important`: Reduce the 54.8% cross-module edge ratio in `dispatch::loader`

**Problem.** Modularity: `lib/src/dispatch/loader.rs` (1526 lines) imports from
`actions`, `config::atomic`, `config::claudine_config`, `config::migration`,
`dispatch::matcher`, `error`, `events`, `messaging`, and `services::protect`
(13 distinct `use crate::*` statements). The loader is a hub that pulls in
nearly every adjacent module, which explains a chunk of the 326 cross-module
edges.

**Files touched.**

- `lib/src/dispatch/loader.rs` (1526 lines, 13 cross-module imports)

**Fix.** Introduce a thin `dispatch::deps` façade module that re-exports the
narrow surface the loader actually uses (e.g. only the four config types it
needs, not the whole `claudine_config` module). The loader then imports a
single module path. This collapses inbound edge count without changing
runtime behavior. Pair with the runner split (above) since the runner uses a
similar import pattern.

---

### `important`: Flatten `services::protect::*` depth

**Problem.** Depth (Lakos 1996): the baseline's `max_depth: 8` is partly driven
by `services::protect::{catalog,config,decision,matcher,observe,report,service}`
— seven sibling modules that callers reach via 3-segment paths. The skill notes
ProtectService is now a **standalone** deny catalog with no `PolicyEngine`
dependency, so the extra `services::` wrapping no longer earns its keep.

**Files touched.**

- `lib/src/services/protect/*` (7 modules)
- All `use crate::services::protect::*` call sites (12 importers across
  `dispatch`, `adapters`, `events`, `config`, `composition`, `harness`)

**Fix.** Promote `protect` to a top-level `lib/src/protect/` module and delete
the now-empty `services/` namespace (the skill description lists `services` as
hosting "cross-provider runtime policy services such as `ProtectService`" — but
`ProtectService` is the only inhabitant). Importers shorten from
`crate::services::protect::decision::ProtectDecision` to
`crate::protect::decision::ProtectDecision`, dropping one path segment from
every protect-related edge.

---

### `important`: Add a `MessengerProvider` trait to remove redundancy

**Problem.** Redundancy (Kolmogorov): `lib/src/messaging/send.rs` (1185 lines)
hand-rolls dispatch for six routes (Discord bot, Discord webhook, Slack bot,
Slack webhook, Signal, WhatsApp) plus desktop notifications. The skill confirms
each route shares the same secret-and-recipient resolution path.

**Files touched.**

- `lib/src/messaging/send.rs`
- `lib/src/messaging/mod.rs`

**Fix.** Extract `trait MessengerRoute { fn send(&self, payload: &Payload) ->
Result<SendReceipt> }` and move each provider into its own `messaging/routes/`
file. The redaction invariants called out in the area skill
(`redact_webhook_urls`) become a single shared decorator instead of being
re-implemented per route.

---

### `important`: Split `lib/src/linking/skills.rs`

**Problem.** God-file pressure: `linking/skills.rs` is 1543 lines and
`linking/compatibility.rs` is 1202 lines, together carrying the bulk of the
cross-provider sync logic.

**Files touched.**

- `lib/src/linking/skills.rs` (1543 lines)
- `lib/src/linking/compatibility.rs` (1202 lines)

**Fix.** Pull the per-classification (portable, partially-portable,
non-portable) branches out into `linking/skills/{portable,partial,native}.rs`.
The compatibility table is data — move the 8-provider × N-feature matrix into
a `const` table in `linking/compatibility/table.rs` and keep only the
look-up/diff functions in `compatibility.rs`.

---

### `important`: Split `lib/src/reporting/queries.rs`

**Problem.** God-file pressure: `reporting/queries.rs` is 1581 lines and
contains every JSONL-to-SQLite query the `claudine logs` subcommand uses
(today, week, month, sessions, tools, errors, repos, trends, sync). The skill
notes reporting filters were "intentionally not rewritten in the
2026-04-29 expression-bridge pass" — this file is the deferred work.

**Files touched.**

- `lib/src/reporting/queries.rs` (1581 lines)

**Fix.** Split per-subcommand: `reporting/queries/{today,week,month,sessions,
tools,errors,repos,trends,sync}.rs`, each owning its own SQL fragments. The
shared aggregation helpers move to `reporting/queries/common.rs`.

---

### `nice-to-have`: Address the 133 complex functions

**Problem.** `complex_fn_count: 133` indicates many functions exceed the
configured cyclomatic-complexity threshold. The biggest contributors are
inside the files already flagged above (`runner.rs`, `parse.rs`, `validate.rs`,
`opencode.rs`, `claude_semantic.rs`, `kimi_semantic.rs`).

**Files touched.** All files listed in the urgent suggestions above.

**Fix.** Once the splits land, run `sentrux scan` again and target the
remaining `≥ 15` complexity functions individually. Most will already drop
below threshold simply because their surrounding context has been narrowed.

---

### `nice-to-have`: Confirm `composition/mod.rs` re-exports stay private

**Problem.** Modularity: `lib/src/composition/mod.rs` declares some children
(`error`, `guardrails`, `prepare`, `resolve`, `select`, `types`) as `mod`
(crate-private) and others as `pub mod`. If consumers reach into the private
ones via `pub use` chains, that defeats the modularity boundary.

**Files touched.**

- `lib/src/composition/mod.rs`
- Any `pub use` lines re-exporting from the private children.

**Fix.** Audit `pub use` lines in `composition/mod.rs` and delete any
re-exports that aren't documented public API. Where external callers truly
need a private type, promote that single type to a small `composition::api`
sub-module instead of widening the parent.

---

## `claudine-cli`

The CLI crate. The data below covers
`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/`.

### `critical`: Split the `commands::wrap::mod` god file

**Problem.** Equality (Gini 1912) and modularity: `cli/src/commands/wrap/mod.rs`
is **4641 lines** with 95 top-level functions — this is the single "god file"
the baseline counts. It currently owns: structured-plumbing construction,
flag detection, retired-flag rejection, harness prompt resolution, frontmatter
overlay merging, prompt-tag stripping, resume-arg normalization, harness
launch building, harness attempt execution, inline closure handling, and the
main `run_provider_wrapper` entry point.

**Files touched.**

- `cli/src/commands/wrap/mod.rs` (4641 lines, 95 fns, 44 inline tests)

**Fix.** Carve the orchestration responsibilities out of `mod.rs` into focused
sibling files (the `wrap/` directory already exists and follows this pattern):

```
cli/src/commands/wrap/
├── mod.rs                  // ONLY: re-exports + run_provider_wrapper entry
├── flags.rs                // has_flag, has_flag_value, reject_retired_composition_flags
├── prompt_source.rs        // maybe_edit_prompt_source*, materialize_passthrough_harness_seed
├── overlay.rs              // merge_frontmatter_overlay, frontmatter_map_to_value, strip_prompt_tags_for_provider
├── resume.rs               // normalize_resume_args, append_resume_passthrough_args
├── harness_orch.rs         // build_harness_launch, execute_harness_attempt, build_next_attempt_plan, apply_next_attempt_plan
├── inline.rs               // try_inline_closure, report_inline_agent_status
└── policy.rs               // harness_policy_root, find_git_root, harness_prompt_mode_label
```

Keep `pub use` re-exports in `mod.rs` so the public CLI surface is unchanged.

---

### `critical`: Split `commands::wrap::live_semantic_sink`

**Problem.** Equality and complexity: `live_semantic_sink.rs` is **4269 lines**
with 124 top-level functions and 78 inline tests. The skill's "9-section model"
is conceptually clear, but the implementation packs the section emitters,
the `ToolCallDisplay` contract, the `StreamTextRenderer` heartbeat logic,
the `SemanticErrorKind` rendering, the OpenCode `Reasoning` routing, and the
spacing rules into a single file.

**Files touched.**

- `cli/src/commands/wrap/live_semantic_sink.rs` (4269 lines, 124 fns)

**Fix.** Restructure to a `live_semantic_sink/` directory:

```
cli/src/commands/wrap/live_semantic_sink/
├── mod.rs              // LiveSemanticSink struct + entry point
├── sections.rs         // The 9 emitter functions (one per section)
├── spacing.rs          // The cross-section spacing state machine
├── tool_calls.rs       // ToolCallDisplay, humanize_tool_name, summarize_input
├── thinking.rs         // render_thinking_block, BlockQuote helpers
├── errors.rs           // SemanticErrorKind → BlockQuote color/border mapping
└── heartbeat.rs        // StreamTextRenderer, last_block_growth_at, flush_if_idle
```

Tests follow their subject module — most of the 78 tests will land in
`tool_calls.rs` and `sections.rs`.

---

### `critical`: Split `commands::wrap::exec`

**Problem.** Equality and complexity: `exec.rs` is **3091 lines** with 79 fns
and is where the watchdog ticker, child-process termination, signal handling,
and the timeout/step-timeout machinery all live (per the area skill's 2026-05-03
unified-watchdog change).

**Files touched.**

- `cli/src/commands/wrap/exec.rs` (3091 lines, 79 fns, 38 tests)

**Fix.** Split into `exec/`:

```
cli/src/commands/wrap/exec/
├── mod.rs              // run_wrapped_child entry point
├── spawn.rs            // child spawn, env injection, stdio wiring
├── watchdog.rs         // spawn_timeout_watchdog_ticker, WatchdogState, WatchdogTermination channel
├── termination.rs      // wait_with_signal_and_early_termination, SIGTERM→SIGKILL grace
├── timeouts.rs         // parse/resolve timeout/step_timeout precedence chain
└── exit.rs             // exit-reason classification, JSONL summary synthesis
```

This isolates the watchdog change so future timeout edits can stay scoped.

---

### `critical`: Split `commands::wrap::profile`

**Problem.** Equality and complexity: `profile.rs` is **3347 lines** and 105
inline tests — almost certainly hosting one large enum/match per provider.

**Files touched.**

- `cli/src/commands/wrap/profile.rs` (3347 lines, 105 tests)

**Fix.** Move each provider's `WrapperProfile` impl into its own file under
`commands/wrap/profile/`:

```
cli/src/commands/wrap/profile/
├── mod.rs              // trait WrapperProfile + the dispatcher
├── claude.rs
├── codex.rs
├── gemini.rs
├── goose.rs
├── kimi.rs
├── opencode.rs
└── qwen.rs
```

Each provider's tests follow it. This also reduces redundancy by surfacing
duplicated patterns the trait can absorb.

---

### `urgent`: Split `commands::wrap::composition`

**Problem.** Modularity: `wrap/composition.rs` is 2452 lines and is the single
non-harness execution surface that both `compose` and `inline-compose` route
through (per the 2026-04-16 "consistent rendering" fix in the skill).

**Files touched.**

- `cli/src/commands/wrap/composition.rs` (2452 lines)

**Fix.** Split into `wrap/composition/`:

```
cli/src/commands/wrap/composition/
├── mod.rs              // execute_without_harness entry + CompositionExecutionMode enum
├── structured.rs       // run_structured_composition + CompositionStreamResult
├── summary.rs          // emit_composition_summary, emit_minimal_composition_summary, defer_section_separator handling
├── inline_guards.rs    // The 4 inline-only guarded behaviors documented in the skill
└── legacy_goose.rs     // The remaining Goose-only legacy path
```

The `CompositionExecutionMode::{Direct, Inline}` enum stays in `mod.rs` since
both downstream files need it.

---

### `urgent`: Split `cli/src/argv.rs`

**Problem.** Modularity: `argv.rs` is 1605 lines and the area skill describes
**four discrete normalization rules** (provider-boolean rewrite, provider
canonicalisation, `--` insertion, trailing-help hoist) plus
`COMPOSITION_FLAGS_WITH_VALUE`. They are conceptually independent but currently
co-located.

**Files touched.**

- `cli/src/argv.rs` (1605 lines)

**Fix.** Split into `cli/src/argv/`:

```
cli/src/argv/
├── mod.rs                  // pub fn normalize() — the orchestrator
├── rule1_provider_bool.rs  // --claude → --provider claude rewrite
├── rule2_canonicalize.rs   // Provider::fuzzy_match_cli_name application
├── rule3_separator.rs      // -- insertion before key=value setters
├── rule4_help_hoist.rs     // trailing --help/-h to position 1
└── flag_surface.rs         // COMPOSITION_FLAGS_WITH_VALUE + the drift-detection test
```

Each rule keeps its own tests. The orchestrator stays small and the
ordering invariant ("Rule 4 must run before Rule 3") becomes a single
documented sequence in `mod.rs`.

---

### `urgent`: Split `commands::config_tui::tabs::messenger`

**Problem.** Modularity and complexity: `config_tui/tabs/messenger.rs` is 1770
lines and houses every messenger-route widget (Discord bot, Slack bot, Signal,
WhatsApp, Discord webhook, Slack webhook), the masked-input handling, the
test-connection flow, the redaction invariants, and the validation regex.

**Files touched.**

- `cli/src/commands/config_tui/tabs/messenger.rs` (1770 lines)

**Fix.** Split into `tabs/messenger/`:

```
cli/src/commands/config_tui/tabs/messenger/
├── mod.rs              // tab entry, route list, dispatch
├── routes/
│   ├── discord_bot.rs
│   ├── slack_bot.rs
│   ├── signal.rs
│   ├── whatsapp.rs
│   ├── discord_webhook.rs
│   └── slack_webhook.rs
├── masked_input.rs     // bullet rendering + buffer separation
├── test_connection.rs  // T-key handler + modal-local status
└── redaction.rs        // redact_webhook_urls + render-time invariants
```

This pairs with the lib-side `MessengerProvider` trait suggestion to keep TUI
and library structure aligned.

---

### `urgent`: Split `commands::wrap::wire_io`

**Problem.** Modularity: `cli/src/commands/wrap/wire_io.rs` is 1611 lines and
sits adjacent to `exec.rs`, `stream_io.rs`, and `subagent_watchdog.rs` —
the wrap subdirectory has too many large peers.

**Files touched.**

- `cli/src/commands/wrap/wire_io.rs` (1611 lines)
- `cli/src/commands/wrap/subagent_watchdog.rs` (1204 lines)

**Fix.** Audit overlap with `exec.rs` and `stream_io.rs` — `wire_io` is likely
better merged into `exec/` as `exec/wiring.rs`, while `subagent_watchdog`
becomes `exec/subagent_watchdog.rs`. This cuts the wrap directory's
top-level fan-out.

---

### `urgent`: Split `completion::composition`

**Problem.** Modularity: `cli/src/completion/composition.rs` is 1489 lines and
implements per-mode composition pipelines (`compose` / `inline-compose` /
`sequence`), the magic `@` resolution, setter-value file references, and the
performance strategy described in the area skill.

**Files touched.**

- `cli/src/completion/composition.rs` (1489 lines)

**Fix.** Split into `completion/composition/`:

```
cli/src/completion/composition/
├── mod.rs              // trait/dispatch
├── compose.rs          // pipeline for `compose`
├── inline_compose.rs   // pipeline for `inline-compose`
├── sequence.rs         // pipeline for `sequence`
├── magic_at.rs         // @ resolution
└── setter_value.rs     // setter-value file reference resolution
```

---

### `important`: Split `commands::config_tui::tabs::actions`

**Problem.** God-file pressure inside the TUI: `config_tui/tabs/actions.rs` is
1474 lines, paralleling the structure of `messenger.rs` (also called out
above).

**Files touched.**

- `cli/src/commands/config_tui/tabs/actions.rs` (1474 lines)

**Fix.** Apply the same per-action-type split treatment as the messenger tab.

---

### `important`: Split `commands::logs`

**Problem.** Modularity: `cli/src/commands/logs.rs` is 1300 lines and dispatches
**nine** subcommands (today, week, month, sessions, tools, errors, repos,
trends, sync). This pairs with the lib-side `reporting/queries.rs` split.

**Files touched.**

- `cli/src/commands/logs.rs` (1300 lines)

**Fix.** Split into `commands/logs/{today,week,month,sessions,tools,errors,
repos,trends,sync}.rs` with a thin `mod.rs` dispatcher. Each subcommand
file then pairs 1:1 with its `reporting/queries/<name>.rs` counterpart.

---

### `important`: Split `commands::hooks` and `commands::mcp`

**Problem.** Modularity: `commands/hooks.rs` (1237 lines) houses six display
modes (`hooks`, `--support`, `--mapping`, `--describe`, `--variables`, plus
the per-provider listing), and `commands/mcp.rs` (1209 lines) houses seven
MCP subcommands (`list`, `init`, `show`, `default`, `alias`, `remove`, `sync`).

**Files touched.**

- `cli/src/commands/hooks.rs` (1237 lines)
- `cli/src/commands/mcp.rs` (1209 lines)

**Fix.** Split each into a directory with one file per display mode /
subcommand. This is the same recipe as `commands/logs/`.

---

### `nice-to-have`: Resolve the `output.rs` vs `output/` duplication

**Problem.** Equality (small): `cli/src/output.rs` (1173 lines) and
`cli/src/output/` (with `error_report.rs`, `error_walker.rs`) coexist. Rust
allows this but the convention is one or the other.

**Files touched.**

- `cli/src/output.rs`
- `cli/src/output/{error_report,error_walker}.rs`

**Fix.** Convert `output.rs` into `output/mod.rs` and split its contents into
`output/{prose,tables,hyperlinks,…}.rs`. The two existing `output/` children
then sit naturally as siblings.

---

### `nice-to-have`: Re-run Sentrux after the critical splits land

**Problem.** Verification: the suggestions above are grounded in file size,
fn count, and direct cycle-tracing — but the Sentrux DSM and `test_gaps`
output (which would have ranked the highest-risk untested files) was
unavailable in this session.

**Fix.** After the four `critical` splits land, run `sentrux scan
$WORKDIR/claudine` and the `dsm` / `test_gaps` MCP tools to confirm
`cycle_count` drops to 0, `god_file_count` drops to 0, and to surface the
next round of urgent items the file-size heuristic alone can't see.

# Claudine Module-Structure Review

**Date:** 2026-07-11
**Scope:** the `claudine/` package area — `lib`, `cli`, `contract`, `catalog-types`, `gen`, and the `rendezvous/{core,daemon,client}` sub-crates.
**Method:** `hug god-files` over the area (41 high-risk / 154 moderate-risk files), followed by four parallel deep-dives (composition, CLI wrap, stream, area-wide survey) with symbol-level inventories and duplication verification by grep.

---

## Headline findings

1. **Roughly half of every god-file is inline test code.** `lifecycle.rs` is 7,549 raw lines but tests start at line 3,613; `loop_control.rs` 6,064 with tests from 2,697; `error.rs` 4,519 with tests from 3,120. Any refactor plan that ignores this over-estimates the logic problem and under-uses the cheapest fix (mechanical test extraction).
2. **The worst structural problem is not a file — it is a layering violation.** The lifecycle *runtime* control flow (blocked → failure → finalize routing, terminal-control dispatch, budgets, proxy chains) lives in the **CLI crate** (`cli/src/commands/wrap/harness_orch/loop_control.rs`) even though the lib's `lifecycle_executor.rs:1-18` docstring explicitly defers that wiring to "the composition runtime." That runtime is now implemented ~2.5 times: `loop_control.rs`, `wrap/composition/mod.rs` (self-documented at `composition/mod.rs:194`: *"Mirrors `harness_orch::loop_control::surface_catch_evaluation_error`"*), and partially in lib's `loop_engine.rs`.
3. **The 8 stream parsers share no common skeleton — it is copy-pasted 8 ways.** Every new provider (and the Phase H ladder keeps adding them) re-copies `feed_line`, `emit_provider_extension`, `emit_malformed_warning`, `base_extra`, the `finish` summary builder, and `classify_error`. Estimated 600–900 duplicated non-test lines today, growing linearly per provider.
4. **Public-API blast radius for a `composition/` split is small.** `composition/mod.rs` lines 39–112 is a `pub use` barrel and only **three** deep-path import sites exist outside it (`wrap/inline.rs` → `closure`, `output/error_walker.rs` + `harness_orch` → `lifecycle_context`, `harness_orch` → `lifecycle_executor`). Splits that preserve the barrel (or add `pub use` facades) are invisible to the other ~38 consuming files.

---

## Critical / Must Fix

### C1. Extract inline test modules from all files > ~1,500 lines

The single highest-leverage, lowest-risk change. Move each `#[cfg(test)] mod tests` block to a sibling file (`foo/tests.rs` declared via `#[cfg(test)] mod tests;`), a pattern already established in the area (`lib/src/provider/tests.rs`, `cli/src/commands/wrap/composition/tests.rs`, `exec/wiring/tests.rs`, `exec/watchdog/tests/`).

Immediate effect on the worst offenders:

| File | Raw lines | Tests start | Post-extraction size |
|---|---|---|---|
| `lib/src/composition/lifecycle.rs` | 7,549 | 3,613 | ~3,600 |
| `cli/…/harness_orch/loop_control.rs` | 6,064 | 2,697 | ~2,700 |
| `lib/src/composition/error.rs` | 4,519 | 3,120 | ~3,100 |
| `lib/src/stream/logs/opencode/reasoning.rs` | 4,024 | 1,744 (real tests; the `cfg(test)` at 526 is a test-only accessor) | ~1,750 |
| `lib/src/composition/lifecycle_executor.rs` | 3,634 | 1,294 | ~1,300 |
| `lib/src/composition/loop_engine.rs` | 3,476 | 1,532 | ~1,550 |
| `lib/src/composition/schema_validation.rs` | 3,398 | 1,665 | ~1,700 |
| `rendezvous/daemon/src/session_log.rs` | 3,154 | 1,345 | ~1,350 |

This must be paired with a **written convention** (see C6/Other Observations): inline tests remain the norm for small files; above a size threshold, tests move to a sibling. Without the written rule the area drifts back.

### C2. Split `lib/src/composition/lifecycle.rs` by its six existing clusters

After test extraction (~3,600 lines) the file still mixes six unrelated responsibilities. Split into a `composition/lifecycle/` subdirectory:

| New module | Current lines | Content |
|---|---|---|
| `lifecycle/mod.rs` | 43–426, 793–957 | Config/type model (`LifecycleConfig`, `LifecycleStacks`, `LifecycleSignal`, `LifecycleRuntimeState`), `LifecycleEmitter` trait, `LifecycleRunGuard` |
| `lifecycle/parse.rs` | 1,056–1,789 | `parse_lifecycle_config`, event/stack/item parsers, `annotate_stack_error`, `scan_removed_validation_keys` |
| `lifecycle/action_shape.rs` | 1,886–2,517 | positional arity checks, `build_action_from_params`, `parse_lifecycle_control_long`, `did_you_mean_verb` |
| `lifecycle/validate.rs` | 2,585–3,320 | the three `validate_no_*` static scans, expression walking (`iter_stack_expression_surfaces`, `visit_string_literals`), `collect_lifecycle_shell_commands` |
| `lifecycle/audio.rs` | 3,432–3,612 | TTS/sound blocking emission, `run_blocking_with_timeout`, `emit_lifecycle_signal` |

Fold the four sibling files into the same subdirectory so the lifecycle family lives in one place: `lifecycle_actions.rs` → `lifecycle/actions.rs` (+ carve its signature registry, lines 344–628, into `lifecycle/signatures.rs`), `lifecycle_context.rs` → `lifecycle/context.rs`, `lifecycle_control.rs` → `lifecycle/control.rs`, `lifecycle_executor.rs` → `lifecycle/executor.rs`. Keep the current public module names alive via facades in `composition/mod.rs` (`pub use self::lifecycle::executor as lifecycle_executor;`) so the three deep-path importers compile unchanged.

### C3. Break up `run_harness_loop` (1,001 sloc, `loop_control.rs:1494–2695`) and de-duplicate its terminal-recovery sequence

The function is a de-facto state machine written as one `loop` with 13 sequential phases. Two concrete defects beyond size:

- **The 5-step terminal-recovery sequence is copy-pasted three times** (failure path 2,321–2,458; inline-closure path 2,460–2,581; success path 2,583–2,693): *execute/classify terminal event → `handle_terminal_evaluation_error` → `dispatch_terminal_control` → `run_finalize_with_recovery` → return*. The Abort arms at 2,396–2,422 / 2,527–2,553 / 2,638–2,664 are near-verbatim. One `drive_terminal_recovery(...) -> LoopStep` helper collapses ~400 lines.
- **Every phase threads the same ~10 arguments**, which is why the file carries 15 `#[allow(clippy::too_many_arguments)]`. Introduce a `HarnessLoopCtx<'a>` context struct and an `enum LoopStep { NextAttempt, Return(...), Abort(...) }`, making each phase a method.

Then split `loop_control.rs`'s six mixed concerns into `harness_orch/` siblings: `lifecycle_events.rs` (terminal-event execution + stack-context assembly), `error_routing.rs` (the `emit_*_with_err` / `handle_*_evaluation_error` family), `control_dispatch.rs` (`dispatch_terminal_control`, `ControlBudgets`, `TerminalControlAction`), `proxy.rs` (`ProxyTracking`, `run_target_initialize`), and `requeue.rs` (see C4).

### C4. Consolidate the lifecycle runtime into the lib crate (the layering fix)

The blocked/failure/finalize routing algorithm exists in three places today:

1. `cli/.../loop_control.rs` — `emit_blocked_finalize_with_err` (329), `emit_failure_finalize_with_err` (467), `surface_catch_evaluation_error` (1,265), `run_finalize_with_recovery` (1,429)
2. `cli/.../composition/mod.rs` — `emit_preflight_blocked_and_finalize` (226), documented at line 194 as a mirror of #1
3. `lib/src/composition/loop_engine.rs` — `route_init_failure*` (1,052–1,152), `run_loop_gate` (1,154)

This logic is provider-agnostic and pure (decisions, budgets, proxy-chain bookkeeping); only prompt materialization, `build_harness_launch`, and `execute_harness_attempt` genuinely need the CLI/process layer. Promote the shared routing into a lib module beside the executor (e.g. `lib/src/composition/lifecycle/runtime.rs`) consumed by both the harness loop and the compose preflight. This is the fix that prevents the *next* divergence bug — the three copies already drift in return types (`CompositionError` vs loop-step).

Related: `IterationSummarySignals` is defined in the **CLI** (`wrap/composition/mod.rs:407`) but is conceptually the sibling of lib `loop_engine`'s `LoopIterationOutput` (loop_engine.rs:126–140) and flows back into lib. It belongs in lib.

### C5. Create `stream/providers/common.rs` and collapse the 8-way parser copy-paste

Verified duplicated shapes (identical except the `Provider::X` literal):

- `feed_line` skeleton — `claude.rs:828`, `codex.rs:524`, `opencode.rs:435`, `kimi.rs:985`, plus qwen/gemini/pi
- `emit_provider_extension` — 7 copies: `claude.rs:656`, `codex.rs:99`, `opencode.rs:410`, `kimi.rs:960`, `qwen.rs:217`, `gemini.rs:317`, `pi.rs:284`
- `emit_malformed_warning` — `claude.rs:670`, `codex.rs:513`, `opencode.rs:424`, `kimi.rs:974`, …
- `base_extra` — `claude.rs:120`, `codex.rs:88`, `opencode.rs:81`, `kimi.rs:120`
- `finish` summary builder + `derive_badges` call — `claude.rs:920`, `codex.rs:594`, `opencode.rs:518`, `kimi.rs:1069`
- `classify_error` keyword cascade — `claude.rs:961`, `codex.rs:647`, `opencode.rs:612`, `qwen.rs:356`, `gemini.rs:493` (str-variant in `pi.rs:418`, `antigravity.rs:265`)

Introduce a `ParserShared` struct (sink, line_num, session_id, model, token_usage, cost — the ~12 fields every parser re-declares) with the shared emit/summary/classify methods, plus a generic `feed_line` driver parameterized over the typed event enum and a per-provider dispatch closure. Each provider file shrinks to its typed dispatch arms — which is the only part that actually differs. This directly serves the provider-ladder roadmap: the next provider costs one dispatch match, not a 1,000-line copy.

> **Status (2026-07-11, implemented):** landed as free-function delegation in `stream/providers/common.rs`, **not** a `ParserShared` state struct — the Phase 6.0 discovery ([`phase6-discovery.md`](phase6-discovery.md)) showed only 8 of ~15 fields are identical across the 8 structs (`token_usage`/`cost_usd`/`num_turns` split Option-vs-plain), so state unification was abandoned in favor of shared helpers (`base_extra`, `emit_provider_extension`, `emit_malformed_warning`), `finish_summary` + `..Default::default()` assembly, and `ErrorKeywords` ordered-bucket classify tables. The generic `feed_line` driver was evaluated and **demoted to Nice-to-Have** (see N7).

### C6. Split `composition/error.rs` into data model vs rendering

`error.rs` holds four concerns: the 70-variant `CompositionError` enum + aux types (39–1,557), constructors/frontmatter enrichment (1,558–1,809), the `impl BlockError` rendering layer whose `status_block` alone is ~743 sloc (1,828–2,571) plus ~240 lines of `render_*` free functions, and the `impl Diagnostic` projection (2,878–3,120). Move rendering to `composition/error/render.rs`. **Zero external blast radius** — the barrel exports the types, not the render functions. This is the best "first split" to establish the pattern.

---

## Strong Candidates

### S1. `stream/logs/opencode/reasoning.rs` — rename and split

The file is misnamed: it contains zero reasoning-content handling; it is the OpenCode **stderr bridge**. Post test-extraction (~1,750 lines) split the 1,100-line `impl OpenCodeLogBridge` by its existing seams into `logs/opencode/bridge/{mod,errors,session,stall_guard,signals,format}.rs` plus `state.rs` (`SharedStderrState` + `merge_stderr_state_into_summary`). Also delete or relocate the orphaned 31-line `logs/opencode_tests_final.rs` sitting at the wrong level.

### S2. `stream/logs/opencode/errors.rs` — facade + classified submodules

A stateless classification function library (~830 production lines, 59% tests). Split into `classify/{asset,llm,session,text_util}.rs` behind the existing `classify()`/`classify_raw()` entry points; extract tests.

### S3. `dispatch/mod.rs` — five concerns in the module root

The worst logic-in-`mod.rs` offender (1,534 lines, ~725 production). Keep the entry points + `DispatchRuntimeContext` in `mod.rs`; move: event logging (`write_dispatch_event_to`, `log_dispatch_event`, `tool_detail_for_log`, `compact_value_for_log`, `truncate_for_log`, lines 411–580 — already has its own `mod logging_tests`) → `dispatch/logging.rs`; protect evaluation/mapping (580–632) → `dispatch/protect_bridge.rs`; wrapper flag extraction (656–682) → `dispatch/wrapper_flags.rs`.

### S4. Group the loop family into `composition/looping/`

Mirror the C2 lifecycle grouping for the loop family (~6.3k lines across 4 files): `loop_engine.rs`, `loop_config.rs`, `loop_actions.rs`, `loop_expression.rs` → `composition/looping/{engine,config,dsl,actions,expression}.rs` (`loop` is a keyword; `looping/` avoids `r#loop`). Within `loop_config.rs`, the DSL action parsers (`parse_dsl_action`, `parse_structured_action*`, `parse_dsl_value`) are a distinct cluster from env-resolution and key validation — split them into `dsl.rs`.

### S5. `wrap/composition/mod.rs` is a grab-bag; name the 1,668-line closure

Six unrelated concerns in one 2,037-line file. Keep the `execute_composition_request*` pipeline; move preflight lifecycle routing to `preflight_lifecycle.rs` (or the lib module from C4), `IterationSummarySignals` to lib, `SelectionConfig` loading to its own file, `emit_execution_header` to output helpers. The `run_body` closure (1,349–1,727) inside `execute_composition_request_inner_with_guard` captures ~20 locals and is invoked twice — promote it to a named `fn` taking a context struct (same disease as C3).

### S6. Hoist shared helpers in `permissions/providers/`

The `ProviderPolicyBackend` trait design is sound, but leaf helpers below each impl are near-verbatim copies: `first_source_id` (`codex.rs:1016` vs `gemini.rs:843` — identical except the fallback literal), string-array extraction ×4 (`claude.rs:918`, `qwen.rs:636`, `gemini.rs`, `codex.rs:770`), `has_cli*` ×4, `build_one_shot_plan` ×4, `choose_target*` ×4, approval-mode mappers ×4. Only `json_utils.rs` is hoisted today. Add `permissions/providers/common.rs` for the format-agnostic ones; keep genuinely format-specific (TOML vs JSON) helpers local.

### S7. Kill the small composition-wide duplications

- `fn json_type_name` — **five private copies**: `lifecycle.rs:2549`, `loop_config.rs:605`, `prepare.rs:621`, `sequence.rs:368`, `loop_actions.rs:501`. One shared helper.
- Reserved-identifier logic ×3: `types.rs:183 AmbientVariable::is_reserved`, `loop_config.rs:232 is_reserved_identifier`, `loop_actions.rs:434 reject_reserved_property` — should share one source of truth for reserved roots, ideally unified with `lifecycle.rs:108 LATE_BINDING_ROOTS`.
- Index-annotating error wrappers pair: `lifecycle.rs:1352 annotate_stack_error` / `loop_config.rs:356 annotate_action_error`.

### S8. Split `composition/schema_validation.rs` by its five layers

~1,700 production lines with clean seams: prepare wrappers + `PreValidatedSchema` stay in `schema/mod.rs`; compose-error translation (186–408) → `schema/translate.rs`; problem categorization/classification + status report (596–1,258) → `schema/classify.rs`.

### S9. Move the dead-code rendezvous requeue block out of `loop_control.rs`

Lines 876–1,048 (`RequeueEnqueueError`, `write_requeue_fallback`, `enqueue_requeue_entry*`, `try_enqueue_via_daemon`) are entirely `#[allow(dead_code)]`, retained for the future rendezvous backend, and pull `rendezvous_client`/`tonic`/`tokio` into the harness-loop layer. Move to `harness_orch/requeue.rs` (or behind a feature) until the `defer` backend lands.

---

## Nice to Haves

### N1. Rename `lib/src/adapters/` → `hook_adapters/` (or `hooks/`)

There is **no** responsibility overlap between `adapters/` (hook request/response parsing) and `stream/providers/` (stdout stream parsing) — verified — but four provider-keyed directories (`adapters/`, `stream/providers/`, `stream/protocol/`, `stream/logs/`) plus docs that call stream parsers "adapters" invite confusion. Naming-only change.

### N2. `mod.rs` hygiene for the remaining logic-in-root files

`linking/compatibility/mod.rs` (~640 logic lines over one submodule declaration) and `render/event_renderer/mod.rs` (~440 logic lines) should move their computation into named submodules, leaving declaration/re-export roots. `provider/mod.rs` and `reporting/mod.rs` are already clean and can serve as the template.

### N3. CLI reporting/completion splits

- `commands/context.rs` (1,286): the nine `render_expressions_*` fns (351–717) → `context/expressions.rs`; side-effects report → `context/effects.rs`; value formatting (112–204) → `context/format.rs`. Also merge its two separate `#[cfg(test)]` blocks (lines 29 and 869).
- `completion/schema_completion.rs` (1,438): schema-key extraction vs file-candidate matching are separable clusters.
- `commands/schema_interactive.rs` (1,135): status rendering (57–154) vs interactive collection (463+).
- `completion/engine.rs` (1,211): token predicates (561–611) → `engine/tokens.rs` if it grows further.

### N4. `composition/prepare.rs` — carve out hint parsing

Selection-hint parsing (`parse_selection_hints_from_frontmatter`, `ParsedAgentHint`, model/agent/interactive hint parsers, lines ~480–620) is a distinct concern from prompt preparation → `composition/hints.rs`.

### N5. `loop_engine.rs` type hive-off

The result/option/context types (29–276: `LoopExecutionOptions`, `LoopIterationContext/Output`, `LoopExecutionResult`) and seed building (276–370) can split from the engine proper when S4 lands.

### N6. Relocate protocol fixture-replay tests

`stream/protocol/kimi.rs` is 48% tests, `protocol/codex.rs` 37% — deserialization-fidelity tests over the `fixtures/kimi/*.jsonl` corpus. Reasonable to co-locate, but moving the fixture-replay portions to integration tests would shrink the compile units. Low urgency; the models themselves are legitimately large (Kimi wire has 4 envelope shapes) and hand-written by design.

---

### N7. Generic `feed_line` driver for the line-oriented stream parsers

Demoted from C5 after the 6a–6c extractions landed (2026-07-11): the residual per-parser skeleton is a 6-line prologue plus a pure-delegation fallback arm, while kimi (envelope classifier, synthetic `raw_kind`) and gemini (pre-dispatch `flush_pending_text`) deviate structurally. A generic driver would save ~14 lines × 6 parsers at the cost of generic machinery plus per-provider hooks. Revisit only if a future provider's `feed_line` grows a genuinely new skeleton. Antigravity (buffered JSON, no line loop) is excluded by design regardless. Evidence: [`phase6-discovery.md`](phase6-discovery.md).

## Suggested sequencing

1. **C1 test extraction** (mechanical, zero behavior risk, halves the problem) + the written test-placement convention.
2. **C6 error.rs split** (zero blast radius — proves the split pattern).
3. **C2 lifecycle/ + S4 looping/ directory moves** (barrel facades keep the 3 deep-path importers compiling).
4. **C3 run_harness_loop decomposition**, then **C4 lib promotion** (C4 depends on C3's extraction of the pure routing).
5. **C5 stream ParserShared** — ideally *before* the next provider lands on the Phase H ladder.
6. S-tier items opportunistically alongside feature work touching those files.

The monorepo has no installed user base yet (per repo rules, refactoring cost is at its lifetime low), and `composition/mod.rs`'s barrel means most of these moves are invisible outside the crate. The main coordination hazard is in-flight branches touching `lifecycle.rs`/`loop_control.rs` — land the directory moves at a quiet point.

---

## Other Observations

- **The `rendezvous/` sub-area is undocumented in the area's conventions.** `claudine/rendezvous/{core,daemon,client}` are workspace members (root `Cargo.toml:52-54`) with their own tri-crate split (diverging from the documented `lib`/`cli`/`contract`/`catalog-types`/`gen` layout), but neither the repo `CLAUDE.md` package-area conventions nor the claudine skill's crate list mentions them. Drift-maintenance rule says update these alongside layout changes. Also `rendezvous/daemon/src/session_log.rs` (3,154 lines) is already a top-5 god-file in a *young* crate — worth applying the test-extraction convention there now, before it hardens.
- **Two divergent event-time interpolation strategies coexist in composition/**: the loop family rolls its own lookup-render stack (`loop_actions.rs:149 render_action_value` → `render_string_with_lookup` + `SizedLookup`), while lifecycle routes through Darkmatter DM2 subtree compose (`lifecycle_executor.rs:879,961`). Not literal copy-paste, but two mechanisms for the same conceptual operation ("render `{{ … }}` at event-time") is a consistency risk — a semantic fix in one will silently miss the other. Worth a design decision on converging loop actions onto DM2.
- **Test-placement convention is de-facto inconsistent**: 216 lib files + 56 cli files use inline `#[cfg(test)]`, while sibling `tests.rs` appears in exactly one lib module (`provider/`) and a cluster under `cli/.../wrap/`. Neither `CLAUDE.md` nor the rust-testing skill states a rule. Recommend documenting: inline for small files; sibling `tests.rs` once a file exceeds ~800 lines or tests exceed ~300 lines.
- **Generated files are correctly identifiable and should be excluded from god-file metrics**: `lib/src/signals/generated.rs` (2,425) and all ten `provider/*/data.rs` files carry `// GENERATED by claudine-gen — DO NOT EDIT BY HAND.` headers (~7.9k lines total). If `hug god-files` grows an exclude mechanism, these are the first candidates; until then reviewers should discount them manually.
- **No cross-crate (lib↔cli) duplication found** — a genuine bright spot. Table rendering, provider fuzzy matching, and error rendering are all correctly hoisted to lib or shared crates (`biscuit_terminal`, `darkmatter`); `cli/src/completion/fuzzy.rs` is a different algorithm (fzf-style subsequence) with a different purpose, not a copy. The completion/schema layers reuse `darkmatter::markdown::schemas` and `claudine::composition` rather than reimplementing.
- **`config/claudine_config.rs` looks like a god-file but isn't**: ~475 production lines; the 14-field `ClaudineConfig` root spans ~9 domains but each delegates to a typed field whose logic lives elsewhere. It is a schema aggregator — low urgency.
- **`exec/` under wrap is well-factored** (spawn/termination/timeouts/exit + watchdog/wiring sub-packages) and can serve as the internal template for what `harness_orch/` should look like after C3.
- **`hug god-files` counts sloc, raw `wc -l` counts differ** (lifecycle.rs: 5,837 sloc vs 7,549 raw). Reviews mixing the two metrics will disagree by ~20–25%; this document uses raw lines with test-start markers, which is the more actionable pair for split planning.

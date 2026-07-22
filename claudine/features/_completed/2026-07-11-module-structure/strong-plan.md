---
agent: open_code/zai-coding-plan/glm-5.2
total_phases: 7
created: 2026-07-11
start_phase: 1
prerequisite: critical-plan.md (C1–C6) fully landed
source_code:
  # Phase 1 — S8 schema_validation split
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/src/composition/schema/translate.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 2 — S4 loop family → looping/
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/loop_config.rs
  - claudine/lib/src/composition/loop_actions.rs
  - claudine/lib/src/composition/loop_expression.rs
  - claudine/lib/src/composition/looping/mod.rs
  - claudine/lib/src/composition/looping/engine.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/looping/dsl.rs
  - claudine/lib/src/composition/looping/actions.rs
  - claudine/lib/src/composition/looping/expression.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 3 — S7 composition-wide dedup
  - claudine/lib/src/composition/json_util.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/looping/actions.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/sequence.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/reserved.rs
  # Phase 4 — S3 dispatch/mod.rs split
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/dispatch/logging.rs
  - claudine/lib/src/dispatch/protect_bridge.rs
  - claudine/lib/src/dispatch/wrapper_flags.rs
  # Phase 5 — S1 + S2 opencode logs cleanup
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/stream/logs/opencode/bridge/mod.rs
  - claudine/lib/src/stream/logs/opencode/bridge/errors.rs
  - claudine/lib/src/stream/logs/opencode/bridge/session.rs
  - claudine/lib/src/stream/logs/opencode/bridge/stall_guard.rs
  - claudine/lib/src/stream/logs/opencode/bridge/signals.rs
  - claudine/lib/src/stream/logs/opencode/bridge/format.rs
  - claudine/lib/src/stream/logs/opencode/state.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/classify/mod.rs
  - claudine/lib/src/stream/logs/opencode/classify/asset.rs
  - claudine/lib/src/stream/logs/opencode/classify/llm.rs
  - claudine/lib/src/stream/logs/opencode/classify/session.rs
  - claudine/lib/src/stream/logs/opencode/classify/text_util.rs
  - claudine/lib/src/stream/logs/opencode/mod.rs
  - claudine/lib/src/stream/logs/opencode_tests_final.rs
  # Phase 6 — S6 permissions/providers hoist
  - claudine/lib/src/permissions/providers/common.rs
  - claudine/lib/src/permissions/providers/mod.rs
  - claudine/lib/src/permissions/providers/claude.rs
  - claudine/lib/src/permissions/providers/codex.rs
  - claudine/lib/src/permissions/providers/gemini.rs
  - claudine/lib/src/permissions/providers/opencode.rs
  - claudine/lib/src/permissions/providers/qwen.rs
  # Phase 7 — S5 wrap/composition/mod.rs cleanup
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/launch.rs
  - claudine/cli/src/commands/wrap/composition/preflight.rs
  - claudine/cli/src/commands/wrap/composition/selection.rs
  - claudine/cli/src/commands/wrap/composition/runner.rs
  - claudine/cli/src/commands/wrap/composition/tests.rs
  - claudine/cli/src/output/mod.rs
documentation:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .claude/skills/claudine/architecture.md
  - .opencode/skill/claudine/architecture.md
  - .claude/skills/claudine/opencode-event-sources.md
  - .opencode/skill/claudine/opencode-event-sources.md
  - .claude/skills/claudine/timeline.md
  - .opencode/skill/claudine/timeline.md
packages:
  - claudine
source_files_during_phase_1:
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/src/composition/schema/translate.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/composition/schema/tests.rs
docs_updated_during_phase_1:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .claude/skills/claudine/architecture.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_2:
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/parse.rs
  - claudine/lib/src/composition/looping/mod.rs
  - claudine/lib/src/composition/looping/actions.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/looping/dsl.rs
  - claudine/lib/src/composition/looping/engine.rs
  - claudine/lib/src/composition/looping/engine/tests.rs
  - claudine/lib/src/composition/looping/expression.rs
docs_updated_during_phase_2:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .claude/skills/claudine/architecture.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_3:
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/json_util.rs
  - claudine/lib/src/composition/lifecycle/action_shape.rs
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/parse.rs
  - claudine/lib/src/composition/looping/actions.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/looping/dsl.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/reserved.rs
  - claudine/lib/src/composition/sequence.rs
  - claudine/lib/src/composition/types.rs
docs_updated_during_phase_3:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/dispatch/logging.rs
  - claudine/lib/src/dispatch/protect_bridge.rs
  - claudine/lib/src/dispatch/wrapper_flags.rs
docs_updated_during_phase_4:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .claude/skills/claudine/architecture.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_5:
  - claudine/lib/src/stream/logs/opencode/mod.rs
  - claudine/lib/src/stream/logs/opencode/bridge/mod.rs
  - claudine/lib/src/stream/logs/opencode/bridge/errors.rs
  - claudine/lib/src/stream/logs/opencode/bridge/session.rs
  - claudine/lib/src/stream/logs/opencode/bridge/stall_guard.rs
  - claudine/lib/src/stream/logs/opencode/bridge/signals.rs
  - claudine/lib/src/stream/logs/opencode/bridge/format.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests.rs
  - claudine/lib/src/stream/logs/opencode/state.rs
  - claudine/lib/src/stream/logs/opencode/classify/mod.rs
  - claudine/lib/src/stream/logs/opencode/classify/asset.rs
  - claudine/lib/src/stream/logs/opencode/classify/llm.rs
  - claudine/lib/src/stream/logs/opencode/classify/session.rs
  - claudine/lib/src/stream/logs/opencode/classify/text_util.rs
  - claudine/lib/src/stream/logs/opencode/classify/tests.rs
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/stream/semantic.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_5:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .opencode/skill/claudine/architecture.md
  - .opencode/skill/claudine/opencode-event-sources.md
  - .opencode/skill/claudine/timeline.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/opencode-event-sources.md
  - .claude/skills/claudine/timeline.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/claudine/architecture.md
  - .opencode/skill/claudine/opencode-event-sources.md
  - .opencode/skill/claudine/timeline.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/opencode-event-sources.md
  - .claude/skills/claudine/timeline.md
source_files_during_phase_6:
  - claudine/lib/src/permissions/providers/common.rs
  - claudine/lib/src/permissions/providers/mod.rs
  - claudine/lib/src/permissions/providers/codex.rs
  - claudine/lib/src/permissions/providers/gemini.rs
  - claudine/lib/src/permissions/providers/claude.rs
  - claudine/lib/src/permissions/providers/qwen.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_6:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .opencode/skill/claudine/architecture.md
  - .claude/skills/claudine/architecture.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .opencode/skill/claudine/architecture.md
  - .claude/skills/claudine/architecture.md
source_files_during_phase_7:
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/composition/launch.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/preflight.rs
  - claudine/cli/src/commands/wrap/composition/runner.rs
  - claudine/cli/src/commands/wrap/composition/selection.rs
  - claudine/cli/src/commands/wrap/composition/tests.rs
  - claudine/cli/src/output/mod.rs
docs_updated_during_phase_7:
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - .claude/skills/claudine/architecture.md
  - .opencode/skill/claudine/architecture.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/architecture.md
  - .opencode/skill/claudine/architecture.md
---

# Strong-Candidate Module-Structure Refactor — Execution Plan

Converts the nine **Strong Candidate** findings (S1–S9) from
[`review.md`](review.md) into a dependency-ordered, high-confidence plan that
executes **after** [`critical-plan.md`](critical-plan.md) (C1–C6) is fully
landed.

Every phase is observable through a compile, a `just test` run, or a `just
lint` pass. No phase changes runtime behavior — all are pure structural moves
or mechanical de-duplications. Where a closure/function promotion touches live
control flow (Phase 7), the existing test suite is the safety net.

## Scope note: S9 is already absorbed by the critical plan

Finding **S9** ("Move the dead-code rendezvous requeue block out of
`loop_control.rs`") is implemented by critical-plan Phase 4, Step 4.3, which
creates `harness_orch/requeue.rs` for exactly the `RequeueEnqueueError` /
`write_requeue_fallback` / `enqueue_requeue_entry*` / `try_enqueue_via_daemon`
block (lines 876–1,048). **No separate work is required in this plan.** S9 is
listed here only for completeness; it is closed when the critical plan lands.

## Context (recap)

With the critical plan landed, tests live in sibling `tests.rs` files,
`composition/lifecycle/` and `composition/error/` are directories, the harness
loop is decomposed, the lifecycle runtime is consolidated in
`lib/…/lifecycle/runtime.rs`, `IterationSummarySignals` lives in the lib, and
the stream parsers share `ParserShared`. The remaining structural debt is a set
of medium-sized files that each mix 2–6 concerns, plus small but real
duplication across the composition and permissions areas. None of it is
correctness-critical, but each item grows linearly with every new provider or
feature and is cheapest to fix now (no installed user base, per repo rules).

## Dependency graph

```
Phase 1 (S8) ──────────────────────────────────────────────────┐
Phase 2 (S4) ──────────────────────────────────────────┐        │
                                                       ▼        │
Phase 3 (S7) ── depends on S4 (loop files in final home) + C2 ──┘

Phase 4 (S3) ── independent
Phase 5 (S1+S2) ── independent (single directory, two findings)
Phase 6 (S6) ── independent
Phase 7 (S5) ── depends on C4 (lib promotion already done)
```

Phases 4, 5, and 6 are mutually independent and independent of Phases 1–3; they
can be parallelized or reordered once the critical plan is complete. Phase 3
must follow Phase 2 because the `json_type_name` copies in `loop_config.rs` and
`loop_actions.rs` land in their final `looping/` homes first. Phase 7 is last
because its `run_body` closure promotion is the only step that restructures
live control flow.

## Verified structural facts driving this plan

1. **The loop family has zero deep-path importers outside `composition/`.**
   `composition/mod.rs:26–29` declares `pub mod loop_actions / loop_config /
   loop_engine / loop_expression` and re-exports them (`pub use` at 70–79).
   A repo-wide search for `composition::loop_*` / `use …loop_engine` outside
   `composition/` returns nothing — every consumer goes through the barrel. The
   `looping/` directory move (Phase 2) is therefore invisible to all consumers
   as long as the `pub use` facades are preserved.

2. **`schema_validation.rs` has clean three-way seams.** Prepare wrappers +
   `PreValidatedSchema` (1,258–1,667) + `InteractiveSchemaOptions` form the
   entry layer; compose-error translation (`schema_error_to_composition_error`
   186, `handle_compose_error` 228, `translate_schema_failure` 258,
   `handle_retry_error` 350) is the second; problem categorization +
   classification (`CategorizedProblems` 681, `categorize_problems` 688,
   `build_schema_status_report` 1,099, `classify_unresolved_file_reference` 799,
   `SchemaStatusReport` / `PropertyStatus` / `PropertyState` 1,044–1,075) is the
   third.

3. **`dispatch/mod.rs` has three cleanly separable clusters** after the
   `DispatchRuntimeContext` + entry pipeline (1–410). Event logging
   (`write_dispatch_event_to` 411, `log_dispatch_event` 434,
   `prepare_meta_for_dispatch` 448, `tool_detail_for_log` 518,
   `compact_value_for_log` 555, `compact_scalar_for_log` 560,
   `truncate_for_log` 570) already has its own `mod logging_tests` at 1,516.
   Protect bridging (`map_protect_block` 580, `evaluate_protect_observation`
   605, `synthetic_unparsed_block` 632). Wrapper-flag extraction
   (`wrapper_interactive_flag` 656, `wrapper_yolo_flag` 660,
   `wrapper_interactive_flag_from` 664, `wrapper_flag_from` 671,
   `runtime_repo_root` 649).

4. **`reasoning.rs` is misnamed.** It contains zero reasoning-content handling;
   it is the OpenCode **stderr bridge**. The 1,100-line `impl OpenCodeLogBridge`
   (375–1,470) plus the shared-state types (`SharedStderrState` 342,
   `OpenCodeLogBridge` 375, `StalledGenerationContext` 81, `EarlyTermination`
   103) and the free-function formatting tail (`format_llm_call_message` 1,552,
   `format_permission_message` 1,583, `format_http_response_message` 1,614,
   `base_extra` 1,696, `merge_stderr_state_into_summary` 1,722) are the real
   seams. The 31-line `opencode_tests_final.rs` at the directory root is an
   orphaned partial test (it starts mid-function with no `#[test]` attribute and
   no module wrapper) — verified broken/dead.

5. **`errors.rs` is a stateless classification library** (~823 production lines,
   59% tests) behind two public entry points (`classify` 31, `classify_raw` 72)
   and `merge_rate_limit` (806). The private classify family groups naturally:
   asset classification (`classify_malformed_asset` 83, `detect_asset_suffix`
   118, `summarize_error_json` 133, `extract_provider_message` 203), LLM/session
   failure (`classify_llm_failure` 323, `classify_lifecycle` 490,
   `classify_session` 634, `classify_llm_call` 643, `classify_session_prompt`
   660, `classify_permission` 684), and text utilities (`contains_any_ci` 698,
   `extract_status_code` 703, `extract_reset_at` 715, `looks_like_uncaught_error`
   722, `infer_service_from_message` 523, `has_trailing_keyword` 565,
   `tag_value_stripped` 586, `classify_default_service` 592).

6. **The permissions provider duplication is real but heterogeneous.** Verified
   near-verbatim copies: `first_source_id` (codex:1,016 vs gemini:843),
   `has_cli*` (qwen:863, codex:1,024, gemini:851, claude:968 as
   `has_cli_overrides`), `choose_target*` (qwen:871, gemini:717 as
   `choose_targets`, opencode:479, claude:729 as `choose_target_path`),
   `build_one_shot_plan` (qwen:997, codex:892, claude:849, gemini:809,
   opencode:584 — **note:** return types differ: some `Option<…>`, some
   `Result<Option<…>>`). Only `permissions/json_utils.rs` is hoisted today.
   `mod.rs` is a clean declaration/re-export root — no logic.

7. **`json_type_name` is five identical private copies** (verified identical
   signatures): `sequence.rs:368`, `lifecycle.rs:2,549` (→ `lifecycle/validate.rs`
   post-C2), `loop_actions.rs:501` (→ `looping/actions.rs` post-Phase-2),
   `loop_config.rs:605` (→ `looping/config.rs` post-Phase-2),
   `prepare.rs:621`. Each is `fn(&serde_json::Value) -> &'static str`.

8. **The reserved-identifier triplication is intentional, not accidental.**
   The three checks guard *different layers* with *different sets*:
   - `AmbientVariable::is_reserved` (`types.rs:183`) — the five `_loop_*`
     ambient variable names.
   - `is_reserved_identifier` (`loop_config.rs:232`) — expression-language
     roots `true`/`false`/`doc`/`env`/`_loop_` prefix.
   - `reject_reserved_property` (`loop_actions.rs:434`) — set-action property
     blocklist `loop`/`replace` + `AmbientVariable::is_reserved`.
   These are not the same predicate; Phase 3 unifies the *string literals* into
   one registry each layer queries, not the predicates themselves. The
   `annotate_stack_error` (`lifecycle.rs:1,352`) / `annotate_action_error`
   (`loop_config.rs:356`) pair **is** a true shape duplicate (both wrap a
   `CompositionError` with an index annotation) and collapses to one generic
   helper.

9. **`wrap/composition/mod.rs` post-critical-plan scope is reduced.** C4 already
   moved `IterationSummarySignals` to the lib and consolidated preflight routing
   behind the lib `route_*` API. The remaining concerns are: `SelectionConfig`
   + its two loaders (2,008–2,037), `emit_execution_header` (479–526), the six
   `select_launch_workspace` / launch-detection helpers (91–144), the
   `PreflightBlockedOutcome` / `surface_preflight_catch_error` /
   `preflight_blocked_control_error` / `setup_phase_deferred` cluster (146–379),
   and the `run_body` closure (1,349–1,727) inside
   `execute_composition_request_inner_with_guard` which captures ~20 locals and
   is invoked twice (1,733 and 2,003).

---

## Phase 1 — S8: Split `composition/schema_validation.rs` into a `schema/` Subdirectory

**Goal:** Split the ~1,667-line file (tests already extracted by C1) along its
three clean seams: entry/prepare layer, compose-error translation, and problem
categorization/classification + status reporting.

**Risk:** Low. `composition/mod.rs:34` declares `pub mod schema_validation;`
and `pub use schema_validation::{…}` (89). The split preserves both via a
facade, so no consumer outside `composition/` is affected.

### Step 1.1 — Convert to `schema/` directory, keep entry layer in `mod.rs`

**Files:**
- Move: `claudine/lib/src/composition/schema_validation.rs` →
  `claudine/lib/src/composition/schema/mod.rs`
- Create: `claudine/lib/src/composition/schema/translate.rs`
- Create: `claudine/lib/src/composition/schema/classify.rs`

- [x] `git mv schema_validation.rs schema/mod.rs`
- [x] What stays in `schema/mod.rs`: `InteractiveSchemaOptions` (68),
      `prepare_direct_with_schema` (119), `prepare_inline_with_schema` (133),
      `prepare_with_schema` (146), `run_prepare` (164), `post_shell_validate`
      (408), `load_effective_schema` (534), `PreValidatedSchema` (1,258),
      `pre_validate_schema` (1,311). Plus `pub mod translate; pub mod classify;`
      declarations.
- [x] Make the cross-module helpers (`atom_for_property`, `is_required`,
      `provided_partial_value`, `is_eager_file_problem`,
      `interactive_shape_for_atom`, `min_max_constraints`,
      `string_length_constraints`, `type_label_for_atom`) `pub(super)` so
      `translate.rs` and `classify.rs` can reach the ones they share.

### Step 1.2 — Carve translation layer into `schema/translate.rs`

**Files:**
- Create: `claudine/lib/src/composition/schema/translate.rs`

Move the compose-error translation cluster:
- `schema_error_to_composition_error` (186)
- `handle_compose_error` (228)
- `translate_schema_failure` (258)
- `handle_retry_error` (350)
- `build_schema_validation_error` (550), `build_missing_properties_error` (573)
- `source_with_dropped_optionals` (596), `options_with_dropped_optionals`
  (634), `filter_droppable_invalid_optionals` (664)

- [x] Prefix with `use super::*;` (or targeted `use super::{…}` for the
      `pub(super)` entry-layer helpers it calls)
- [x] Verify: `just test` + `just lint`

### Step 1.3 — Carve classification + status layer into `schema/classify.rs`

**Files:**
- Create: `claudine/lib/src/composition/schema/classify.rs`

Move the problem-categorization + status-report cluster:
- `CategorizedProblems` (681), `categorize_problems` (688)
- `classify_unresolved_file_reference` (799)
- `SchemaStatusReport` (1,044), `PropertyStatus` (1,062), `PropertyState`
  (1,075), `build_schema_status_report` (1,099)
- The atom/shape query helpers that are classification-only
  (`atom_for_property`, `is_required`, `is_eager_file_problem`,
  `interactive_shape_for_atom`, `min_max_constraints`,
  `string_length_constraints`, `type_label_for_atom`,
  `provided_partial_value`) move here if `translate.rs` does not also need
  them; otherwise they stay `pub(super)` in `mod.rs` and both submodules
  import them.

- [x] Verify: `just test` + `just lint`

### Step 1.4 — Preserve the barrel facade in `composition/mod.rs`

- [x] Replace `pub mod schema_validation;` (34) with `pub mod schema;`
- [x] Replace `pub use schema_validation::{…}` (89) with
      `pub use schema::{…}` — same exported names, consumers unchanged
- [x] Grep for any `::schema_validation` deep-path import outside
      `composition/`; if found, point it at the facade (none expected per
      the barrel analysis)
- [x] Verify: `just test` + `just lint`

### Phase 1 Exit Criteria

- [x] `just test` + `just lint` pass for claudine lib
- [x] `schema/mod.rs` is the entry/prepare layer (~600 lines)
- [x] `schema/translate.rs` is the translation layer (~400 lines)
- [x] `schema/classify.rs` is the classification + status layer (~650 lines)
- [x] No consumer outside `composition/` needed modification

---

## Phase 2 — S4: Group the Loop Family into `composition/looping/`

**Goal:** Mirror the C2 lifecycle grouping for the loop family (~4,400 lines
across 4 files post-test-extraction). `loop_engine.rs`, `loop_config.rs`,
`loop_actions.rs`, `loop_expression.rs` → `composition/looping/{engine,config,
dsl,actions,expression}.rs`. Within `config.rs`, carve the DSL action parsers
into `dsl.rs`. `loop` is a keyword; `looping/` avoids `r#loop`.

**Risk:** Low. Verified zero deep-path importers outside `composition/` — the
barrel (`pub use` at 70–79) is the only consumer surface.

### Step 2.1 — Create the `looping/` directory and move the four files

**Files:**
- Move: `loop_engine.rs` → `looping/engine.rs`
- Move: `loop_config.rs` → `looping/config.rs`
- Move: `loop_actions.rs` → `looping/actions.rs`
- Move: `loop_expression.rs` → `looping/expression.rs`
- Create: `looping/mod.rs` (declaration + `pub use` facade)
- Create: `looping/dsl.rs` (carved from `config.rs`)
- Modify: `composition/mod.rs`

- [x] Create `looping/mod.rs`:
      ```rust
      mod engine;
      mod config;
      mod dsl;
      mod actions;
      mod expression;

      pub use engine::*;
      pub use config::*;
      pub use actions::*;
      pub use expression::*;
      ```
      (visibility tuned so the existing barrel names still resolve)
- [x] `git mv` each file into `looping/`; fix any `use super::…` paths that
      referenced the old `composition` parent to `use super::…` within the new
      parent (sibling references between the four files become
      `use super::{config, actions, expression}`)
- [x] The sibling `tests.rs` files (engine/tests.rs etc., created by C1) move
      with their parents; `#[cfg(test)] mod tests;` declarations travel

### Step 2.2 — Carve the DSL parsers out of `config.rs` into `dsl.rs`

**Files:**
- Modify: `looping/config.rs`
- Create: `looping/dsl.rs`

The DSL action parsers are a distinct cluster from env-resolution and key
validation. Move from `config.rs`:

- `parse_dsl_action` (376), `parse_structured_action` (407),
      `parse_structured_action_value` (444), `parse_unary_action` (452),
      `parse_value_action` (466), `parse_dsl_value` (519), and the action
      dispatch helpers `parse_action` (365), `parse_actions` (337),
      `parse_string` / `parse_property` / `parse_positive_usize` (482–502),
      `annotate_action_error` (356)

What stays in `config.rs`: env resolution (`resolve_fail_fast_from_env`,
      `resolve_max_iterations_from_env`, `resolve_pause_reset_margin_from_env`,
      `resolve_loop_config`, `extract_control_variables`), key validation
      (`reject_unknown_loop_keys`, `suggest_loop_key`, `KNOWN_LOOP_KEYS`),
      identifier collection (`collect_value_template_identifiers`,
      `collect_identifiers`, `is_reserved_identifier`), condition parsing
      (`parse_condition`), and `json_type_name` (handled in Phase 3).

- [x] `config.rs` calls into the sibling `dsl` module declared by `looping/mod.rs`;
      `dsl.rs` uses
      `super::*` for shared types (`LoopAction`, `CompositionError`,
      `ActionContext`)
- [x] Verify: `just test` + `just lint`

### Step 2.3 — Preserve the barrel facade in `composition/mod.rs`

- [x] Replace the four module declarations (26–29) with `pub mod looping;`
- [x] Replace the four `pub use loop_*::{…}` (70–79) with the equivalent
      `pub use looping::{…}` — same exported names
- [x] Update internal `composition` references: `preflight.rs`, `prepare.rs`,
      `sequence.rs`, and the `lifecycle/` submodules that reference
      `super::loop_engine` etc. now reference `super::looping` (or the barrel)
- [x] Verify: `just test` + `just lint` across claudine lib + claudine-cli

### Phase 2 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] The loop family lives under `composition/looping/`
- [x] No consumer outside `composition/` needed modification
- [x] `config.rs` is env-resolution + key validation only; `dsl.rs` holds the
      action parsers

---

## Phase 3 — S7: Kill the Composition-Wide Duplications

**Goal:** Collapse three classes of duplication now that every file is in its
final post-Phase-2 location.

**Risk:** Low for `json_type_name` and the annotate pair (mechanical). Medium
judgment for the reserved-identifier registry (the sets are intentionally
different — see verified fact #8).

**Prerequisite:** Phase 2 complete (loop files in `looping/`) and the critical
plan's C2 complete (lifecycle split, so the lifecycle `json_type_name` copy is
in `lifecycle/validate.rs`).

### Step 3.1 — Hoist `json_type_name` into one shared helper

**Files:**
- Create: `claudine/lib/src/composition/json_util.rs`
- Modify (delete the local copy, import the shared one):
  `composition/lifecycle/validate.rs`, `composition/looping/config.rs`,
  `composition/looping/actions.rs`, `composition/prepare.rs`,
  `composition/sequence.rs`

- [x] Create `json_util.rs` with:
      ```rust
      pub(crate) fn json_type_name(value: &serde_json::Value) -> &'static str { … }
      ```
- [x] Declare `pub(crate) mod json_util;` in `composition/mod.rs`
- [x] Delete the five private copies; replace call sites with
      `super::json_util::json_type_name(…)` (or `crate::composition::json_util::…`)
- [x] Verify: `just test` + `just lint`

### Step 3.2 — Unify the index-annotating error wrappers

**Files:**
- Modify: `composition/lifecycle/parse.rs` (home of `annotate_stack_error`
  post-C2 — it annotates a stack *item*, so it likely lives in `parse.rs`
  beside the stack parser; confirm against the C2 split)
- Modify: `composition/looping/dsl.rs` (home of `annotate_action_error`
  post-Phase-2)

`annotate_stack_error(err, property, idx)` and
`annotate_action_error(err, index)` share the shape "wrap a
`CompositionError` with a positional index annotation." The variant they
produce differs (`StackItem`-flavored vs `Action`-flavored), so they may not
collapse to one fn — but the *pattern* (extract-index, rewrap) should live in
one place.

- [x] If both produce the same variant shape, collapse to one generic
      `annotate_with_index(err, idx) -> CompositionError`
- [x] If they produce different variants (likely), extract only the shared
      index-extraction logic and leave each variant constructor local — do not
      force a false unification
- [x] Verify: `just test`

### Step 3.3 — Introduce a reserved-roots registry

**Files:**
- Create: `claudine/lib/src/composition/reserved.rs`
- Modify: `composition/types.rs` (`AmbientVariable`), `composition/looping/config.rs`
  (`is_reserved_identifier`), `composition/looping/actions.rs`
  (`reject_reserved_property`), `composition/lifecycle/mod.rs`
  (`LATE_BINDING_ROOTS`)

The three reserved checks guard different layers (verified fact #8). Do **not**
collapse the predicates. Instead, give each layer one place to declare its
reserved set and cross-reference the others:

- [x] `reserved.rs` holds the canonical string constants:
      `AMBIENT_VARIABLE_NAMES` (the five `_loop_*`), `EXPRESSION_RESERVED_ROOTS`
      (`true`/`false`/`doc`/`env`), `SET_ACTION_BLOCKLIST` (`loop`/`replace`),
      and re-exports `LATE_BINDING_ROOTS` from `lifecycle` (or moves it here and
      has `lifecycle` re-export — pick one owner, prefer `reserved.rs` as the
      single source)
- [x] `AmbientVariable::is_reserved`, `is_reserved_identifier`, and
      `reject_reserved_property` query `reserved.rs` instead of inlining
      literals
- [x] Verify: `just test` + `just lint`

### Phase 3 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] Exactly one `json_type_name` definition
- [x] Reserved string literals declared once in `reserved.rs`
- [x] No behavior change (all reserved-check tests pass)

---

## Phase 4 — S3: Split `dispatch/mod.rs` into Named Submodules

**Goal:** `dispatch/mod.rs` (1,534 lines, ~725 production) keeps the entry
pipeline + `DispatchRuntimeContext`; event logging → `dispatch/logging.rs`;
protect bridging → `dispatch/protect_bridge.rs`; wrapper-flag extraction →
`dispatch/wrapper_flags.rs`.

**Risk:** Low. The three clusters are private helpers; only
`write_dispatch_event_to` / `log_dispatch_event` are `pub` (and only used
inside the dispatch pipeline).

### Step 4.1 — Carve event logging into `dispatch/logging.rs`

**Files:**
- Modify: `claudine/lib/src/dispatch/mod.rs`
- Create: `claudine/lib/src/dispatch/logging.rs`

Move the logging cluster (411–578): `write_dispatch_event_to`, `log_dispatch_event`,
`prepare_meta_for_dispatch`, `tool_detail_for_log`, `compact_value_for_log`,
`compact_scalar_for_log`, `truncate_for_log`.

- [x] The existing `mod logging_tests` (1,516) moves with them →
      `logging.rs` gains `#[cfg(test)] mod logging_tests;` or the tests fold
      into `dispatch/tests.rs` if one exists
- [x] Keep `write_dispatch_event_to` / `log_dispatch_event` re-exported from
      `mod.rs` (`pub use logging::{log_dispatch_event, write_dispatch_event_to};`)
      since they are public API
- [x] Verify: `just test` + `just lint`

### Step 4.2 — Carve protect bridging into `dispatch/protect_bridge.rs`

**Files:**
- Create: `claudine/lib/src/dispatch/protect_bridge.rs`

Move (580–647): `map_protect_block`, `evaluate_protect_observation`,
`synthetic_unparsed_block`.

- [x] `mod.rs` declares `mod protect_bridge;` and calls into it
- [x] Verify: `just test`

### Step 4.3 — Carve wrapper-flag extraction into `dispatch/wrapper_flags.rs`

**Files:**
- Create: `claudine/lib/src/dispatch/wrapper_flags.rs`

Move (649–681): `runtime_repo_root`, `wrapper_interactive_flag`,
`wrapper_yolo_flag`, `wrapper_interactive_flag_from`, `wrapper_flag_from`.

- [x] Verify: `just test` + `just lint`

### Step 4.4 — Slim `mod.rs` to the entry pipeline

- [x] `mod.rs` retains: `DispatchRuntimeContext` (30), `DispatchOutcome` (81),
      the entry dispatch pipeline (the `pub` functions that call into the three
      submodules), `finalize_response` (682)
- [x] Confirm `mod.rs` is now a declaration/re-export root under ~450 lines
- [x] Update `.opencode/skill/claudine/architecture.md` module map

### Phase 4 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] `dispatch/mod.rs` is under ~450 production lines
- [x] `logging.rs`, `protect_bridge.rs`, `wrapper_flags.rs` each hold one
      concern

---

## Phase 5 — S1 + S2: OpenCode Logs Cleanup (Rename/Split `reasoning.rs`, Facade `errors.rs`)

**Goal:** Two findings in the same directory, done together to avoid
module-wiring churn. S1: rename `reasoning.rs` (it is the stderr bridge, not a
reasoning handler) and split its 1,100-line impl along its seams. S2: split
`errors.rs` into `classify/` submodules behind the existing entry points. Also
delete the orphaned `opencode_tests_final.rs`.

**Risk:** Low-medium. Both files are well-tested (tests extracted to
`reasoning/tests.rs`; `errors.rs` is 59% tests). The splits are along existing
seams.

### Step 5.1 — S1: Rename `reasoning.rs` → `bridge/` and split the impl

**Files:**
- Move: `claudine/lib/src/stream/logs/opencode/reasoning.rs` →
  `claudine/lib/src/stream/logs/opencode/bridge/mod.rs`
- Move: `reasoning/tests.rs` → `bridge/tests.rs`
- Create: `bridge/errors.rs`, `bridge/session.rs`, `bridge/stall_guard.rs`,
  `bridge/signals.rs`, `bridge/format.rs`
- Create: `claudine/lib/src/stream/logs/opencode/state.rs`

Split `impl OpenCodeLogBridge` (375–1,470) by its existing seams:

| New module | Content (current lines) |
|------------|-------------------------|
| `bridge/mod.rs` | `OpenCodeLogBridge` struct (375), `SharedStderrState` (342), constructors, `ingest` dispatch, `shared_state` (550), and the outcome/context types (`StderrIngestOutcome` 45, `StuckSubagentInfo` 62, `StalledGenerationContext` 81, `EarlyTermination` 103, `StalledGenerationProgress` 303). Plus `pub mod` declarations. |
| `bridge/errors.rs` | stream-error handling: `is_stream_error` (1,471), `stream_error_fingerprint` (1,485), `MAX_CONSECUTIVE_STREAM_ERRORS` (441) |
| `bridge/session.rs` | `ChildSessionInfo` (465), session tracking, subagent lifecycle dispatch within the impl |
| `bridge/stall_guard.rs` | `MAX_GENERATIONS_WITHOUT_PROGRESS` (455), the live-but-dead stall detection arms, `EarlyTermination` evaluation, `render_stalled_generation_badge` (1,513) |
| `bridge/signals.rs` | early-termination channel signaling, the `StderrIngestOutcome::Consumed`/`Signal` routing |
| `bridge/format.rs` | `format_llm_call_message` (1,552), `format_permission_message` (1,583), `summarize_permission_action` (1,599), `format_http_response_message` (1,614), `format_snapshot_message` (1,643), `summarize_snapshot_tags` (1,659), `truncate_for_inline` (1,687), `base_extra` (1,696), `non_empty` (1,498), `duration_as_millis_u64` (1,505) |
| `state.rs` (sibling of `bridge/`, under `opencode/`) | `SharedStderrState` (342) + `merge_stderr_state_into_summary` (1,722) if they are consumed outside the bridge; otherwise keep in `bridge/mod.rs` |

- [x] Each submodule starts with `use super::*;`
- [x] Update `opencode/mod.rs:20` (`pub mod reasoning;`) →
      `pub mod bridge;` and the `pub use reasoning::{…}` (27) →
      `pub use bridge::{…}` — same exported names (`OpenCodeLogBridge`,
      `StderrIngestOutcome`, etc.)
- [x] Verify: `just test` + `just lint`

### Step 5.2 — S2: Split `errors.rs` into `classify/` submodules

**Files:**
- Move: `claudine/lib/src/stream/logs/opencode/errors.rs` →
  `claudine/lib/src/stream/logs/opencode/classify/mod.rs`
- Create: `classify/asset.rs`, `classify/llm.rs`, `classify/session.rs`,
  `classify/text_util.rs`

Keep the two public entry points (`classify` 31, `classify_raw` 72) and
`merge_rate_limit` (806) in `classify/mod.rs`; move the private families:

| New module | Content |
|------------|---------|
| `classify/asset.rs` | `classify_malformed_asset` (83), `detect_asset_suffix` (118), `summarize_error_json` (133), `extract_provider_message` (203), `AssetType` enum if local |
| `classify/llm.rs` | `classify_llm_failure` (323), `classify_default_service` (592), `infer_service_from_message` (523), `has_trailing_keyword` (565), `tag_value_stripped` (586), `classify_lifecycle` (490) |
| `classify/session.rs` | `classify_session` (634), `classify_llm_call` (643), `classify_session_prompt` (660), `classify_permission` (684), `looks_like_uncaught_error` (722) |
| `classify/text_util.rs` | `contains_any_ci` (698), `extract_status_code` (703), `extract_reset_at` (715) |

- [x] `classify/mod.rs` declares the four submodules and re-exports nothing
      new (entry points stay put)
- [x] Update `opencode/mod.rs:18` (`pub mod errors;`) →
      `pub mod classify;` and `pub use errors::{classify, classify_raw,
      merge_rate_limit};` (23) → `pub use classify::{…}`
- [x] Extract the `errors.rs` inline tests to `classify/tests.rs`
      (following the C1 convention)
- [x] Verify: `just test` + `just lint`

### Step 5.3 — Delete the orphaned `opencode_tests_final.rs`

**Files:**
- Delete: `claudine/lib/src/stream/logs/opencode_tests_final.rs`

- [x] Confirm it is not referenced by any `mod` declaration (grep
      `opencode_tests_final` across the crate)
- [x] Its 31 lines are a partial, unwrapped test fragment (starts
      mid-function, no `#[test]`, no module wrapper) — verified dead. If any
      assertion in it is still valuable, port it into
      `bridge/tests.rs` first; otherwise delete outright
- [x] Verify: `just test` (compile confirms no missing `mod`)

### Phase 5 Exit Criteria

- [x] `just test` + `just lint` pass for claudine lib
- [x] `reasoning.rs` is gone; the stderr bridge lives under `bridge/`
- [x] `errors.rs` is gone; classification lives under `classify/`
- [x] `opencode_tests_final.rs` is deleted
- [x] `opencode/mod.rs` exports are unchanged (consumers unaffected)

---

## Phase 6 — S6: Hoist Shared Helpers in `permissions/providers/`

**Goal:** Add `permissions/providers/common.rs` for the format-agnostic helpers
that are near-verbatim across the provider backends; keep genuinely
format-specific (TOML vs JSON) helpers local.

**Risk:** Low-medium. The `ProviderPolicyBackend` trait design is sound; the
leaf helpers are private. The duplication is heterogeneous (return types differ
— see verified fact #6), so each hoist must be type-checked per provider.

### Step 6.1 — Create `permissions/providers/common.rs`

**Files:**
- Create: `claudine/lib/src/permissions/providers/common.rs`
- Modify: `claudine/lib/src/permissions/providers/mod.rs`

Hoist the format-agnostic helpers, parameterizing over the per-provider types:

- `first_source_id` — identical except the fallback literal (codex:1,016,
      gemini:843). Parameterize over the fallback string:
      `fn first_source_id(native: &NativeEffectivePolicy, fallback: &str) -> String`
- `has_cli*` — the `has_cli` / `has_cli_overrides` family (qwen, codex,
      gemini, claude). Each checks a different `*CliOverrides` struct for
      `Some`-ness of overlapping fields. Extract the *shared field-check
      skeleton* if the field sets overlap; otherwise leave local
- `choose_target*` — the `choose_target` / `choose_targets` /
      `choose_target_path` family (qwen, gemini, opencode, claude). These vary
      in target multiplicity (single vs multi) — extract only the truly
      identical single-target variant
- `build_one_shot_plan` — **five copies with different return types**
      (`Option<OneShotMutationPlan>` vs `Result<Option<…>>`). Standardize on
      one return type in `common.rs` and adapt each call site, or provide two
      wrappers; do not force the `Result`-returning providers into the
      `Option` shape without checking their error paths

- [x] Declare `pub(crate) mod common;` in `providers/mod.rs`
- [x] For each hoisted helper, make the shared version generic over the
      provider-specific config struct (a trait bound or a closure that extracts
      the needed fields), keeping format-specific parsing local
- [x] Verify after each provider migration: `just test` + `just lint`

### Step 6.2 — Migrate each provider to the shared helpers

**Files (one at a time, verify after each):**
- `claude.rs`, `codex.rs`, `gemini.rs`, `qwen.rs`, `opencode.rs`

For each:
- [x] Replace the local copy with a call (or trait impl) into `common`
- [x] Keep genuinely format-specific helpers (TOML serialization for codex,
      JSON for claude/gemini, YAML for qwen) local — `json_utils.rs` remains
      the JSON-specific hoist
- [x] Verify: `just test` (the `permissions` test suite + the dispatch
      inventory guard test)

### Phase 6 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] `common.rs` holds the format-agnostic helpers
- [x] Each provider backend is smaller; no near-verbatim `first_source_id` /
      `has_cli` copies remain
- [x] `claudine-cli/tests/dispatch_inventory.rs` still passes

---

## Phase 7 — S5: Slim `wrap/composition/mod.rs` to the Execution Pipeline

**Goal:** With C4 landed, `IterationSummarySignals` is already in the lib and
preflight routing is already consolidated behind the lib `route_*` API. The
remaining work: extract `SelectionConfig` + loaders to their own file, relocate
`emit_execution_header` to output helpers, and promote the `run_body` closure
to a named function taking a context struct.

**Risk:** Medium — the `run_body` closure promotion is the only step that
restructures live control flow. It captures ~20 locals and is invoked twice.
The existing `composition/tests.rs` suite is the safety net.

### Step 7.1 — Extract `SelectionConfig` to `composition/selection.rs`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition/mod.rs`
- Create: `claudine/cli/src/commands/wrap/composition/selection.rs`

Move `SelectionConfig` (2,008), `load_selection_config` (2,014),
`load_selection_config_for_repo` (2,028).

- [x] `mod.rs` declares `mod selection;` and re-exports if needed
- [x] Verify: `just test`

### Step 7.2 — Relocate `emit_execution_header` to output helpers

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition/mod.rs`
- Modify: the output helpers module (under `cli/src/output/` or
      `cli/src/commands/wrap/output/` — confirm against the area's actual
      output-module location)

Move `emit_execution_header` (479–526) next to the other stderr/header
rendering helpers.

- [x] Update the call site in `execute_composition_request_inner_with_guard`
- [x] Verify: `just test` + `just lint`

### Step 7.3 — Extract the launch-workspace + preflight-blocked helpers

**Files:**
- Modify: `composition/mod.rs`
- Create or extend: `composition/launch.rs` (or keep in `launch_workspace.rs`
  if one exists — the lib already has `composition::launch_workspace`)

Move the cluster that is incidental to the execution pipeline:
`select_launch_workspace` (91), `launch_workspace_fallback_count_for_tests`
(105), `reset_launch_workspace_fallbacks_for_tests` (112),
`enforce_repo_launch_detection` (126), `PreflightBlockedOutcome` (146),
`surface_preflight_catch_error` (199), `preflight_blocked_control_error` (353),
`setup_phase_deferred` (371).

- [x] If C4 already routed `emit_preflight_blocked_and_finalize` to the lib,
      confirm `surface_preflight_catch_error` + `preflight_blocked_control_error`
      are now thin adapters over the lib API; co-locate them with the preflight
      call site or move to `preflight_lifecycle.rs`
- [x] Verify: `just test`

### Step 7.4 — Promote the `run_body` closure to a named function

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition/mod.rs`
- Create: `claudine/cli/src/commands/wrap/composition/runner.rs`

The `run_body` closure (1,349–1,727) inside
`execute_composition_request_inner_with_guard` captures ~20 locals and is
invoked twice (1,733 for the init-proxy/skip-preflight path; 2,003 for the
main path). Same disease as C3's `run_harness_loop`.

- [x] Define a `CompositionRunCtx<'a>` context struct bundling the ~20
      captured locals (guard, skip_preflight, init_proxy_target, the shared
      mutable state, the preflight adapters, etc.)
- [x] Promote the closure to `fn run_composition_body(ctx: &mut
      CompositionRunCtx, …) -> …` in `runner.rs`
- [x] Replace both invocation sites (`return run_body(guard, …)` at 1,733 and
      `run_body(&mut guard, false, …)` at 2,003) with calls to the named fn
- [x] Verify: `just test` — especially the preflight-skip, init-proxy, and
      normal-path tests in `composition/tests.rs`

### Step 7.5 — Slim `mod.rs` to the execution pipeline

- [x] `mod.rs` retains: `SingleCompositionOutcome` (380),
      `execute_composition_request` (528),
      `execute_composition_request_inner` (544),
      `execute_composition_attempt` (567),
      `execute_composition_request_inner_with_guard` (584, now much shorter),
      `record_substage` (613)
- [x] Confirm `mod.rs` is under ~800 lines (from 2,037)
- [x] Update `.opencode/skill/claudine/architecture.md`

### Phase 7 Exit Criteria

- [x] `just test` + `just lint` pass for claudine-cli
- [x] `run_body` is a named function taking a context struct
- [x] `wrap/composition/mod.rs` is the execution pipeline only (~800 lines)
- [x] `SelectionConfig`, `emit_execution_header`, and the launch/preflight
      helpers each live in a named submodule

---

## Global Exit Criteria

After all 7 phases:

- [x] `just test claudine` passes at the repo root
- [x] `just lint` passes in the claudine area
- [x] No file in `claudine/lib/src/composition/` or
      `claudine/lib/src/stream/logs/opencode/` exceeds ~1,000 production lines
- [x] The loop family is grouped under `composition/looping/`; the schema
      family under `composition/schema/`; the OpenCode bridge under
      `logs/opencode/bridge/`; OpenCode classification under
      `logs/opencode/classify/`
- [x] `dispatch/mod.rs` is a declaration/entry root, not a logic file
- [x] Exactly one `json_type_name` and one reserved-roots registry in
      `composition/`
- [x] Permissions provider backends share format-agnostic helpers via
      `permissions/providers/common.rs`
- [x] `wrap/composition/mod.rs` is the execution pipeline only
- [x] `.opencode/skill/claudine/architecture.md` reflects every new directory

## Coordination hazards

Phases 1–3 all touch `claudine/lib/src/composition/` and must be sequenced
(Phase 3 depends on Phase 2's file moves; Phase 2's barrel edit touches the
same `composition/mod.rs` as Phase 1's). Phases 4, 5, and 6 touch disjoint
areas and can proceed in parallel with each other and with Phases 1–3. Phase 7
is CLI-only and depends solely on the critical plan's C4 being landed.

The monorepo has no installed user base yet (per repo rules, refactoring cost
is at its lifetime low). Land the `composition/` directory moves (Phases 1–3)
at a quiet point with no active feature branches on those files.

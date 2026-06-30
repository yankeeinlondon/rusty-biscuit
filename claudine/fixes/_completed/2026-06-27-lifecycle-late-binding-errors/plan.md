---
agent: "codex/"
phases: 5
created: 2026-06-28
start_phase: 1
yolo: true
packages:
    - claudine
source_files_during_phase_1:
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_1:
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_2:
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/error.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/loop_engine.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_3:
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/lib/src/composition/error.rs
    - claudine/cli/src/output/error_walker.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/tests/wrap_compose_validation.rs
docs_updated_during_phase_5:
    - claudine/docs/topics/lifecycle.md
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
source_code:
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/output/error_walker.rs
    - claudine/cli/tests/wrap_compose_validation.rs
    - claudine/lib/src/composition/error.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/loop_engine.rs
documentation:
    - claudine/docs/topics/lifecycle.md
    - claudine/fixes/2026-06-27-lifecycle-late-binding-errors/plan.md
    - .claude/skills/claudine/SKILL.md
---

# Late-Binding Lifecycle Evaluation Errors Execution Plan

## Assumptions

- The later closure instruction for `agent` (`codex/`) is treated as authoritative because duplicate YAML keys would make the frontmatter ambiguous.
- The implementation should preserve the existing distinction between expression/binding errors and side-effect dispatch failures.
- `finalize` is the catch point for terminal-phase evaluation errors; terminal-phase evaluation errors do not retroactively fire `failure`.
- Validation should use package-area recipes (`just test`, `just lint`) from `claudine/` unless the implementer intentionally narrows to a crate test while iterating.

## Success Criteria

- Late-binding evaluation errors from `when:` guards, top-level lifecycle communication strings, and action-string interpolation are classified separately from side-effect dispatch failures.
- Any lifecycle event evaluation error is rendered once to user-facing stderr and causes a non-zero run outcome.
- Terminal-phase evaluation errors run `finalize` exactly once with `err` populated, except an evaluation error inside `finalize` itself, which must surface and halt without recursive `finalize` re-entry.
- Existing action-dispatch behavior remains unchanged, including terminal-phase log-and-continue semantics and `no_error: true`.
- L1 tests cover the behavior matrix in the specification.

## Phase 1: Characterize the Existing Failure Path

- [x] Confirm the current lifecycle outcome model in `claudine/lib/src/composition/lifecycle_executor.rs`, especially `LifecycleEventOutcome`, `execute_stack_inner`, `when_matches`, `resolve_emit`, `resolve_string_value`, and `run_action`.
- [x] Confirm the current routing policy in `claudine/lib/src/composition/lifecycle.rs`, especially `LifecycleSignal::routes_action_error_to_failure`, `LifecycleRunGuard::execute_event`, and `emit_finalize_once`.
- [x] Confirm where terminal outcomes are converted into `CompositionError` and exit status in `claudine/lib/src/composition/loop_engine.rs`.
- [x] Confirm the CLI render boundary that turns `CompositionError` into styled stderr output, including any helper that should be reused instead of adding ad hoc terminal writes.
- [x] Add or identify a focused failing test that reproduces a `success.when` late-binding error being swallowed before changing behavior.
- [x] Validation checkpoint: run the focused failing test and record that it fails for the expected reason, not due to fixture setup.

Parallelizable work:

- [x] In parallel, inspect existing lifecycle executor unit tests for direct `LifecycleEventOutcome` assertions that will need updates.
- [x] In parallel, inspect CLI composition tests for non-interactive exit-code and stderr assertions that can host the end-to-end check.

### Phase 1 findings (characterization)

- **Outcome model** (`lifecycle_executor.rs`): `LifecycleEventOutcome { control, action_error }`.
  `execute_stack_inner` (`:599`) maps a `when_matches` `Err` into
  `action_error: Some(_)` (the **same** channel as a real action-dispatch error
  from `run_action`). `resolve_emit`/`emit_top_level` failures also become
  `action_error`. So an *evaluation* raise and a *dispatch* failure are
  currently indistinguishable — the root cause behind Decision #1.
- **Routing policy** (`lifecycle.rs`): `routes_action_error_to_failure` (`:953`)
  is `true` only for `Initialize | Start | Blocked`; `routes_to_failure`
  (`lifecycle_executor.rs:133`) gates on it. Terminal-phase events return
  `false`, so their `action_error` is never routed.
- **Terminal-event handling / swallow site**: the success/finalize orchestration
  lives in `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
  (`run_attempt_loop`, `execute_terminal_event`, `dispatch_terminal_control`),
  not `loop_engine.rs`. `execute_terminal_event` (`:55`) inspects only
  `outcome.control` (for `StackControl::Error`); a `success`-path
  `action_error` is returned but then `run_attempt_loop` (`:2070`) dispatches
  only `success.outcome.control`, and `dispatch_terminal_control` (`:931`)
  returns `Fallthrough` when `control` is `None`. The `action_error` is dropped
  → exit 0, no stderr. `loop_engine.rs` converts `LoopExecutionResult.error`
  (a `CompositionError`) into the run outcome for `--loop` runs; the per-attempt
  terminal decision is the CLI orchestrator's.
- **CLI render boundary**: `claudine/cli/src/output/error_walker.rs` (+
  `error_report.rs`) renders a returned `eyre::Report`/`CompositionError` to
  styled stderr (red `Error:`), TTY-gated, with `FrontmatterExcerpt` appendix.
  This is the surface to reuse for evaluation-error surfacing (Phase 4) rather
  than ad hoc terminal writes.
- **Reproduction**: `loop_control.rs`
  `terminal_event_tests::success_when_evaluation_error_is_not_swallowed_as_action_error`
  drives `execute_terminal_event(Success)` over a stack whose first `when:`
  references an undefined root. Sanity assertions (`control.is_none()`,
  emitter empty) pass; the defect assertion (`action_error.is_none()`) **fails**
  because the raise lands in `action_error` — confirmed RED for the right
  reason, not fixture setup. Marked `#[ignore]` so the suite stays green; Phase 2
  removes the ignore once the evaluation-error channel exists.
- **Executor unit tests needing Phase 2 updates** (`lifecycle_executor.rs`):
  `:1853`/`:1882` (Start vs Success `action_error` + `routes_to_failure`
  distinction), `:1949`/`:1982` (action_error some/none), `:2804`/`:2833`/`:2865`
  (typo / surviving-span fail-closed). Many `assert_eq!(outcome,
  LifecycleEventOutcome::default())` sites will need the new field too.
- **CLI hosts for the end-to-end exit-code/stderr check** (Phase 5): L2 suites
  `cli/tests/level2_lifecycle_dispatch.rs`, `level2_lifecycle_control.rs`,
  `level2_lifecycle_loop.rs`, plus `wrap_compose_validation.rs` /
  `contextual_errors.rs` for non-interactive exit-code + stderr assertions.

## Phase 2: Add an Evaluation-Error Outcome Channel

- [x] Extend `LifecycleEventOutcome` with a distinct `evaluation_error: Option<LifecycleErrorInfo>` or equivalent typed field that is not controlled by `routes_action_error_to_failure`.
- [x] Update `LifecycleEventOutcome::routes_to_failure` so it continues to represent only setup-phase action-error routing, preserving current side-effect behavior.
- [x] Add a helper such as `has_evaluation_error` or `terminal_evaluation_error` if it keeps orchestration call sites explicit.
- [x] Change `execute_stack_inner` so `when_matches` failures populate the evaluation-error channel, not `action_error`.
- [x] Change top-level lifecycle notification resolution failures from `resolve_emit` / `emit_top_level` so they populate the evaluation-error channel.
- [x] Audit action execution paths that call `resolve_string_value` or `eval_expr`; route expression-layer failures into the evaluation-error channel while leaving side-effect failures in `action_error`.
- [x] Keep `no_error: true` scoped to side-effect/action-dispatch failures; do not let it suppress expression-layer evaluation failures.
- [x] Update direct unit tests in `lifecycle_executor.rs` so clean falsy guards still return no error, unknown roots still fail closed, and side-effect failures still use `action_error`.
- [x] Validation checkpoint: run the focused lifecycle executor tests and confirm evaluation errors and action errors are distinguishable in assertions.

Parallelizable work:

- [x] In parallel, update comments/docs adjacent to changed symbols so they describe evaluation errors versus action errors without restating implementation steps.
- [x] In parallel, search for downstream pattern matches or equality assertions on `LifecycleEventOutcome` and update only the affected tests.

### Phase 2 findings (evaluation-error channel)

- **New channel:** `LifecycleEventOutcome.evaluation_error: Option<LifecycleErrorInfo>`
  is independent of `routes_action_error_to_failure`. `routes_to_failure`
  is unchanged (still `action_error`-keyed); `has_evaluation_error()` is the
  new accessor Phase 3 orchestration will consult on every phase.
- **Layer split:** a private `ActionFailure { Evaluation, Dispatch }` enum tags
  each action failure at its source. `execute_action_inner`/`run_shell_action`/
  `dispatch_side_effect`/`emit_top_level`/`resolve_emit` now return it.
  Expression-layer raises (control-arg eval, `render_message`/interpolation,
  side-effect *argument* eval, `when_matches`) → `Evaluation`; side-effect
  dispatch (shell non-zero/spawn, effect-engine error, missing-arg/unknown-verb,
  invalid resolved effect name) → `Dispatch`. `run_action` maps `Evaluation` →
  `ActionStep::EvaluationErrored` (never suppressed by `no_error`) and
  `Dispatch` → existing `Errored`/`no_error` policy.
- **Tests:** four fail-closed assertions (`when_unknown_root_typo_fails_closed`,
  `unknown_root_typo_fails_closed`, `top_level_unknown_root_fails_event_closed`,
  `post_dm2_surviving_span_fails_before_dispatch`) moved from `action_error` to
  `evaluation_error`; added `no_error_does_not_suppress_evaluation_raise`. The
  Phase 1 reproduction in `loop_control.rs` is un-ignored and now asserts the
  raise lands in `evaluation_error`, not `action_error`. Shell/effect dispatch
  tests are unchanged (still `action_error`).

## Phase 3: Propagate Terminal-Phase Evaluation Errors Through Orchestration

- [x] Add orchestration handling for setup-phase evaluation errors so `initialize`, `start`, and `blocked` continue routing through `failure` and `finalize`, now using the unified evaluation-error path.
- [x] Add orchestration handling for terminal-phase evaluation errors in `success`, `failure`, and `loop`: record the error as the run outcome, run `finalize` once with `err`, and return non-zero.
- [x] Add a guard for evaluation errors raised while executing `finalize`: surface and return the non-zero outcome without re-entering `finalize`.
- [x] Thread the `LifecycleErrorInfo` from terminal evaluation errors into the `LifecycleRuntimeContext` / stack context used by `finalize.with_error`.
- [x] Ensure loop-gate evaluation errors return a failure outcome before evaluating loop conditions or applying loop mutations.
- [x] Preserve explicit lifecycle control behavior (`error`, `stop`, `retry`, `resume`, `proxy`, `defer`) unless it directly intersects with an evaluation error path.
- [x] Validation checkpoint: add unit tests proving terminal evaluation errors produce failure outcomes while terminal side-effect dispatch failures keep the previous outcome.

Parallelizable work:

- [x] In parallel, verify that `LifecycleRunGuard` terminal/finalize bookkeeping still prevents duplicate `finalize` emission across success, failure, blocked, and loop paths.
- [x] In parallel, verify `LifecycleErrorInfo` has enough information for `err.kind`, `err.variant`, and `err.msg` in `finalize` without adding a new public authoring surface.

### Phase 3 findings (orchestration propagation)

- **Carrier error:** a new typed `CompositionError::LifecycleEvaluationError
  { source_path, event, surface, message }` (plus a `lifecycle_evaluation`
  constructor that lifts `surface`/`message` from a `LifecycleErrorInfo`)
  classifies an evaluation raise separately from a dispatch failure and exits
  non-zero on every phase. It falls through to the error walker's catch-all
  render arm for now; Phase 4 will give it a dedicated styled surface.
- **Three orchestrators touched.** The harness loop
  (`loop_control.rs::run_harness_loop`) owns the per-attempt `start`/`success`/
  `failure`/`finalize` decision; the non-loop compose path
  (`composition/mod.rs`) owns `initialize`; the loop engine
  (`loop_engine.rs`) owns the loop-engine `initialize` and the post-finalize
  `loop` gate. Each consults `outcome.evaluation_error` (the Phase 2 channel),
  which is independent of `routes_to_failure` (still `action_error`-keyed).
- **Two helpers in `loop_control.rs`:** `handle_terminal_evaluation_error`
  (terminal-phase: run `finalize` once with the eval `err`, do **not** fire
  `failure` — Decision #3) and `handle_setup_evaluation_error` (setup-phase:
  route through `failure` → `finalize` with the eval `err` — Decision #5). Both
  return the typed run failure or `None`. Wired at the `success`, `failure`
  (classify + inline-closure), `start`, and proxy-`initialize` sites.
- **`finalize` re-entry guard:** `run_finalize_with_recovery` checks the
  finalize outcome's `evaluation_error` and returns `Abort` directly, so a raise
  inside `finalize` never recurses into `finalize`.
- **Top-level (success/blocked) propagation:** `emit_top_level_for_signal` now
  returns the late-binding `evaluation_error` (still logging dispatch failures),
  so `execute_terminal_event` halts a success/blocked event whose **top-level**
  string raised, mirroring `execute_event`'s fail-closed-before-stack behavior.
- **Loop gate:** an evaluation error in the gate stack returns
  `LoopGateOutcome::Fail(LifecycleEvaluationError)` *before* the `while`/`until`
  condition is evaluated and before any gate mutation is applied.
- **Tests:** four L1 tests in `loop_control.rs::terminal_event_tests`
  (`success_evaluation_error_runs_finalize_with_err_and_returns_failure`,
  `terminal_dispatch_failure_keeps_previous_outcome`,
  `setup_evaluation_error_routes_through_failure_and_finalize`,
  `finalize_evaluation_error_aborts_without_reentry`) and one in
  `loop_engine.rs::tests`
  (`loop_gate_evaluation_error_fails_before_condition_and_mutation`). The
  user-facing stderr surfacing assertion is Phase 4.

## Phase 4: Surface User-Facing Errors Once

- [x] Identify the existing styled composition-error rendering path in the CLI and choose the narrowest reusable API for lifecycle evaluation errors.
- [x] Add a helper that converts a lifecycle evaluation error into a user-facing stderr message including the event name and, where available, the offending surface (`when`, top-level field, or action value).
- [x] Ensure the helper emits exactly once for each evaluation error, even when the same error is also stored as the run outcome and passed to `finalize`.
- [x] Ensure non-TTY / `NO_COLOR` behavior follows existing CLI error rendering conventions rather than forcing ANSI styling.
- [x] Add stderr assertions for the `success.when` failure case, including enough text to distinguish a crashed guard from a clean false guard.
- [x] Add stderr assertions for an evaluation error inside `finalize`, proving it is visible and non-recursive.
- [x] Validation checkpoint: run the focused CLI or integration tests and confirm stderr is visible without `RUST_LOG` or `--debug`.

Parallelizable work:

- [x] In parallel, update any snapshots or expected output fixtures touched by the new user-facing stderr message.
- [x] In parallel, verify no existing debug-only `tracing::warn!` behavior is removed for side-effect dispatch failures.

### Phase 4 findings (user-facing surfacing)

- **Reused render path, no new emit surface.** The narrowest reusable API is the
  existing `CompositionError: BlockError` rendering. `LifecycleEvaluationError`
  previously fell through the catch-all `_ =>` arm of
  `error.rs::status_block` (flat `"composition failed"` header); Phase 4 gives it
  a dedicated styled arm (header `lifecycle evaluation error`, event name,
  surface label, raised reason, crashed-vs-false hint). It renders through the
  same chain as every other composition error: orchestrator returns the typed
  error → `main.rs::render_top_level_error` → `error_walker::try_render_block_report`
  → `report_block_error`. No ad-hoc terminal writes were added.
- **Emit-once is structural.** The orchestrators (`loop_control.rs`,
  `composition/mod.rs`, `loop_engine.rs`) **return** the
  `LifecycleEvaluationError` as the run outcome — they never print it at the
  point of error. Threading the `LifecycleErrorInfo` into `finalize`'s `err`
  global is context, not a render. So the single top-level boundary in
  `main.rs` emits it exactly once.
- **Surface label helper** (`lifecycle_evaluation_surface_label`): maps the
  raised `LifecycleErrorInfo::variant` to a human phrase — `when` → "the `when:`
  guard", `interpolation` → "an interpolated string", any action verb → "the
  `<verb>` action value".
- **Non-TTY / NO_COLOR** inherits `report_block_error`'s existing
  `ColorDepth::None` escape-stripping; no styling is forced.
- **Tests:** three L1 renders in `error_walker.rs`
  (`renders_lifecycle_evaluation_error_for_success_when`,
  `renders_lifecycle_evaluation_error_for_finalize`,
  `lifecycle_evaluation_error_is_plain_without_color`). The success-`when` test
  asserts the crashed-vs-clean-false distinction text; the finalize test proves
  the in-`finalize` raise is visible (non-recursion is covered by the Phase 3
  `finalize_evaluation_error_aborts_without_reentry` L1 test). Visibility is
  proven without `RUST_LOG`/`--debug` because the block renders through the
  styled stderr path, not a `tracing` line.

## Phase 5: Full Validation and Documentation Pass

- [x] Add L1 tests for the full behavior matrix: setup-phase evaluation error, terminal-phase `success.when` error, terminal-phase clean falsy guard, terminal-phase side-effect failure, `no_error: true`, `finalize` evaluation error, and loop-gate evaluation error.
- [x] Add a non-interactive process-level test or CLI test that asserts a late-binding evaluation error exits non-zero.
- [x] Run `just test` from `claudine/` and resolve any lifecycle-related failures.
- [x] Run `just lint` from `claudine/` and resolve any warnings or lints introduced by the change.
- [x] Review lifecycle docs and comments changed by this work for drift, especially any text that says terminal-phase action errors leave outcomes unchanged; qualify it so evaluation errors are excluded.
- [x] Update `claudine/docs/topics/lifecycle.md` only if public behavior is documented there; keep the change scoped to the late-binding error behavior.
- [x] Update `claudine/.claude/skills/claudine/SKILL.md` only if the implementation changes architecture or workflow details that the skill currently describes. (Skill lives at repo-root `.claude/skills/claudine/SKILL.md`; qualified the strict-mode fail-closed and `no_error` bullets so an evaluation raise is distinguished from a dispatch failure; hash re-stamped with `md hash --save`.)
- [x] Final validation checkpoint: verify the original reproduction now emits stderr, runs `finalize` with `err`, and exits non-zero.

Parallelizable work:

- [x] In parallel, one implementer can complete docs/comment drift review while another runs the full validation recipes.
- [x] In parallel, one implementer can inspect any failing tests for expected-output churn while another verifies the original reproduction prompt.

### Phase 5 findings (validation & documentation)

- **Behavior matrix already largely covered by Phases 2–4.** The one orchestration-layer
  gap was the *terminal clean-`false` guard* cell — added
  `terminal_clean_false_guard_skips_without_halting` (`loop_control.rs`), the direct
  counterpart to the `success.when` raise test, proving a clean `false` neither files
  an `evaluation_error` nor halts. Setup-phase, terminal `success.when`, terminal
  dispatch failure, `no_error`, `finalize` raise, and loop-gate cells were each covered
  by existing L1 tests (`loop_control.rs`, `lifecycle_executor.rs`, `loop_engine.rs`).
- **Process-level non-zero-exit proof.** `compose_initialize_when_evaluation_error_exits_non_zero`
  (`wrap_compose_validation.rs`) drives the real `claudine` binary over a prompt whose
  `initialize` `when:` references an undefined root. The setup-phase raise halts
  *before* the stub provider is launched, exits non-zero, prints the styled
  `lifecycle evaluation error` to stderr (with the crashed-vs-clean-`false` hint), and
  never sends the body — the end-to-end stderr + exit-code assertion the spec asks for.
  Terminal-phase exit-code behavior needs a real provider run and is proven at L1 via
  `success_evaluation_error_runs_finalize_with_err_and_returns_failure`.
- **Docs drift.** `lifecycle.md` documented fail-closed resolution and `no_error` but
  predated the halt/surface behavior. Added a bullet under *When Lifecycle Properties
  Interpolate* (an evaluation error halts every phase, fires `finalize` with `err`, no
  retroactive `failure`, clean-`false` excluded) and scoped both `no_error` descriptions
  to the side-effect dispatch layer. The repo-root `SKILL.md` got the matching
  qualifiers. No text claiming "terminal-phase action errors leave outcomes unchanged"
  existed verbatim; the `no_error` text was the closest drift surface and is now corrected.
- **Validation:** `just test` → 1773 passed (2 pre-existing `sequence_perf` flakes,
  unrelated); `just lint` → clean.

## Implementation Notes

- Keep the behavior change narrow: expression-layer failures halt; side-effect dispatch failures keep current policy.
- Prefer existing `LifecycleErrorInfo` construction methods unless a new variant/source label is needed for clearer diagnostics.
- Do not introduce new lifecycle syntax or change DM2 strict-mode semantics.
- Do not run `cargo fmt` unless explicitly requested; match surrounding style by hand.

---
agent: "codex/"
phases: 5
created: 2026-06-28
start_phase: 1
yolo: true
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

- [ ] Confirm the current lifecycle outcome model in `claudine/lib/src/composition/lifecycle_executor.rs`, especially `LifecycleEventOutcome`, `execute_stack_inner`, `when_matches`, `resolve_emit`, `resolve_string_value`, and `run_action`.
- [ ] Confirm the current routing policy in `claudine/lib/src/composition/lifecycle.rs`, especially `LifecycleSignal::routes_action_error_to_failure`, `LifecycleRunGuard::execute_event`, and `emit_finalize_once`.
- [ ] Confirm where terminal outcomes are converted into `CompositionError` and exit status in `claudine/lib/src/composition/loop_engine.rs`.
- [ ] Confirm the CLI render boundary that turns `CompositionError` into styled stderr output, including any helper that should be reused instead of adding ad hoc terminal writes.
- [ ] Add or identify a focused failing test that reproduces a `success.when` late-binding error being swallowed before changing behavior.
- [ ] Validation checkpoint: run the focused failing test and record that it fails for the expected reason, not due to fixture setup.

Parallelizable work:

- [ ] In parallel, inspect existing lifecycle executor unit tests for direct `LifecycleEventOutcome` assertions that will need updates.
- [ ] In parallel, inspect CLI composition tests for non-interactive exit-code and stderr assertions that can host the end-to-end check.

## Phase 2: Add an Evaluation-Error Outcome Channel

- [ ] Extend `LifecycleEventOutcome` with a distinct `evaluation_error: Option<LifecycleErrorInfo>` or equivalent typed field that is not controlled by `routes_action_error_to_failure`.
- [ ] Update `LifecycleEventOutcome::routes_to_failure` so it continues to represent only setup-phase action-error routing, preserving current side-effect behavior.
- [ ] Add a helper such as `has_evaluation_error` or `terminal_evaluation_error` if it keeps orchestration call sites explicit.
- [ ] Change `execute_stack_inner` so `when_matches` failures populate the evaluation-error channel, not `action_error`.
- [ ] Change top-level lifecycle notification resolution failures from `resolve_emit` / `emit_top_level` so they populate the evaluation-error channel.
- [ ] Audit action execution paths that call `resolve_string_value` or `eval_expr`; route expression-layer failures into the evaluation-error channel while leaving side-effect failures in `action_error`.
- [ ] Keep `no_error: true` scoped to side-effect/action-dispatch failures; do not let it suppress expression-layer evaluation failures.
- [ ] Update direct unit tests in `lifecycle_executor.rs` so clean falsy guards still return no error, unknown roots still fail closed, and side-effect failures still use `action_error`.
- [ ] Validation checkpoint: run the focused lifecycle executor tests and confirm evaluation errors and action errors are distinguishable in assertions.

Parallelizable work:

- [ ] In parallel, update comments/docs adjacent to changed symbols so they describe evaluation errors versus action errors without restating implementation steps.
- [ ] In parallel, search for downstream pattern matches or equality assertions on `LifecycleEventOutcome` and update only the affected tests.

## Phase 3: Propagate Terminal-Phase Evaluation Errors Through Orchestration

- [ ] Add orchestration handling for setup-phase evaluation errors so `initialize`, `start`, and `blocked` continue routing through `failure` and `finalize`, now using the unified evaluation-error path.
- [ ] Add orchestration handling for terminal-phase evaluation errors in `success`, `failure`, and `loop`: record the error as the run outcome, run `finalize` once with `err`, and return non-zero.
- [ ] Add a guard for evaluation errors raised while executing `finalize`: surface and return the non-zero outcome without re-entering `finalize`.
- [ ] Thread the `LifecycleErrorInfo` from terminal evaluation errors into the `LifecycleRuntimeContext` / stack context used by `finalize.with_error`.
- [ ] Ensure loop-gate evaluation errors return a failure outcome before evaluating loop conditions or applying loop mutations.
- [ ] Preserve explicit lifecycle control behavior (`error`, `stop`, `retry`, `resume`, `proxy`, `defer`) unless it directly intersects with an evaluation error path.
- [ ] Validation checkpoint: add unit tests proving terminal evaluation errors produce failure outcomes while terminal side-effect dispatch failures keep the previous outcome.

Parallelizable work:

- [ ] In parallel, verify that `LifecycleRunGuard` terminal/finalize bookkeeping still prevents duplicate `finalize` emission across success, failure, blocked, and loop paths.
- [ ] In parallel, verify `LifecycleErrorInfo` has enough information for `err.kind`, `err.variant`, and `err.msg` in `finalize` without adding a new public authoring surface.

## Phase 4: Surface User-Facing Errors Once

- [ ] Identify the existing styled composition-error rendering path in the CLI and choose the narrowest reusable API for lifecycle evaluation errors.
- [ ] Add a helper that converts a lifecycle evaluation error into a user-facing stderr message including the event name and, where available, the offending surface (`when`, top-level field, or action value).
- [ ] Ensure the helper emits exactly once for each evaluation error, even when the same error is also stored as the run outcome and passed to `finalize`.
- [ ] Ensure non-TTY / `NO_COLOR` behavior follows existing CLI error rendering conventions rather than forcing ANSI styling.
- [ ] Add stderr assertions for the `success.when` failure case, including enough text to distinguish a crashed guard from a clean false guard.
- [ ] Add stderr assertions for an evaluation error inside `finalize`, proving it is visible and non-recursive.
- [ ] Validation checkpoint: run the focused CLI or integration tests and confirm stderr is visible without `RUST_LOG` or `--debug`.

Parallelizable work:

- [ ] In parallel, update any snapshots or expected output fixtures touched by the new user-facing stderr message.
- [ ] In parallel, verify no existing debug-only `tracing::warn!` behavior is removed for side-effect dispatch failures.

## Phase 5: Full Validation and Documentation Pass

- [ ] Add L1 tests for the full behavior matrix: setup-phase evaluation error, terminal-phase `success.when` error, terminal-phase clean falsy guard, terminal-phase side-effect failure, `no_error: true`, `finalize` evaluation error, and loop-gate evaluation error.
- [ ] Add a non-interactive process-level test or CLI test that asserts a late-binding evaluation error exits non-zero.
- [ ] Run `just test` from `claudine/` and resolve any lifecycle-related failures.
- [ ] Run `just lint` from `claudine/` and resolve any warnings or lints introduced by the change.
- [ ] Review lifecycle docs and comments changed by this work for drift, especially any text that says terminal-phase action errors leave outcomes unchanged; qualify it so evaluation errors are excluded.
- [ ] Update `claudine/docs/topics/lifecycle.md` only if public behavior is documented there; keep the change scoped to the late-binding error behavior.
- [ ] Update `claudine/.claude/skills/claudine/SKILL.md` only if the implementation changes architecture or workflow details that the skill currently describes.
- [ ] Final validation checkpoint: verify the original reproduction now emits stderr, runs `finalize` with `err`, and exits non-zero.

Parallelizable work:

- [ ] In parallel, one implementer can complete docs/comment drift review while another runs the full validation recipes.
- [ ] In parallel, one implementer can inspect any failing tests for expected-output churn while another verifies the original reproduction prompt.

## Implementation Notes

- Keep the behavior change narrow: expression-layer failures halt; side-effect dispatch failures keep current policy.
- Prefer existing `LifecycleErrorInfo` construction methods unless a new variant/source label is needed for clearer diagnostics.
- Do not introduce new lifecycle syntax or change DM2 strict-mode semantics.
- Do not run `cargo fmt` unless explicitly requested; match surrounding style by hand.

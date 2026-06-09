---
phases: 6
created: 2026-06-05
start_phase: 1
---

# Always-Harness Execution Plan

Unify `compose` and `inline-compose` so every non-dry-run composition executes through `run_harness_loop`, including documents with no harness frontmatter.

## Phase 1: Baseline and Convergence Tests

- [ ] Inspect `claudine/cli/src/commands/wrap/composition/mod.rs`, `composition/structured.rs`, `composition/legacy_goose.rs`, `composition/inline_guards.rs`, `wrap/harness_orch.rs`, and `wrap/inline.rs` to confirm the exact duplicated responsibilities before editing.
- [ ] Add a focused CLI integration test file, for example `claudine/cli/tests/always_harness.rs`, with fake provider fixtures for the smallest provider mode that exposes structured final-response filtering.
- [ ] Add a no-harness `inline-compose` test where the fake provider emits interstitial assistant text before a tool call and a final response after the last tool call; assert the rewritten Markdown body contains only the final response.
- [ ] Add the matching harness-enabled `inline-compose` test using equivalent frontmatter plus a minimal harness marker such as empty `post_checks`; assert the rewritten Markdown body is identical to the no-harness case.
- [ ] Add a direct `compose` convergence test that runs the same prompt with and without minimal harness frontmatter and asserts exit code, stdout data, and key stderr summary markers remain equivalent after ANSI stripping.
- [ ] Validation checkpoint: run the new targeted tests before the refactor and record which assertions expose the current path divergence; do not weaken the assertions to match legacy harness leakage.
- [ ] Parallelizable: while one implementer builds fake provider fixtures, another can inspect existing compose tests for reusable helpers and output-normalization patterns.

## Phase 2: Degenerate Harness Plan API

- [ ] Resolve the spec/code mismatch: `HarnessPlan` currently has no `max_retries` field, while the spec describes `max_retries: 0`; confirm whether a handler-free plan already guarantees one attempt or whether retry limits need a typed model change.
- [ ] Add `harness_plan_for_bare_composition` in the harness parsing/model layer if it belongs in `claudine`, or in the CLI composition layer if it only exists to bridge wrapper orchestration.
- [ ] Construct the bare plan with `source_path` set to the resolved composition path, empty post-checks, empty handlers, no programmatic handler, and no timeout or warning values.
- [ ] Include `inline_writability_pre_check(source_path)` as the only pre-check for inline mode and no pre-checks for direct compose mode.
- [ ] Export the helper through `claudine/lib/src/harness/mod.rs` only if the helper lands in the library.
- [ ] Add unit tests proving direct compose produces an empty validation plan and inline compose produces exactly one system-owned `HasWritePermission` pre-check for the source path.
- [ ] Validation checkpoint: run the targeted harness-plan unit tests, for example `cargo test -p claudine harness_plan_for_bare_composition --lib --color=never`.
- [ ] Parallelizable: unit tests for direct and inline bare plans can be written independently once the helper signature is chosen.

## Phase 3: Route Composition Through the Harness Loop

- [ ] In `execute_composition_request`, replace the `if harness_enabled { ... } else { execute_without_harness(...) }` execution branch with one `run_harness_loop` call for both parsed and bare harness plans.
- [ ] Preserve the dry-run early return exactly where it is so `--dry-run` still performs composition, shell approval, and metadata rendering without launching a provider.
- [ ] Change preflight so harness shell approval receives either the parsed harness plan or the synthesized bare plan; keep compose template shell approval behavior unchanged.
- [ ] Remove the non-harness inline permission probe branch after the inline bare plan covers writability through the harness pre-check pipeline.
- [ ] Build `HarnessPromptState` for all composition modes, using `HarnessPromptMode::Inline` for `inline-compose` and `HarnessPromptMode::Compose` for direct `compose`.
- [ ] Defuse the outer `LifecycleRunGuard` before calling `run_harness_loop` for every non-dry-run composition so lifecycle start/success/failure is emitted by one owner.
- [ ] Preserve `SingleCompositionOutcome` fields as far as current harness loop data allows; document any remaining `iteration_signals: None` limitation for `compose --loop` if it still applies.
- [ ] Validation checkpoint: run `cargo check -p claudine-cli --color=never` and resolve only compile errors caused by the routing change.

## Phase 4: Preserve Output, Closure, and Timeout Semantics

- [ ] Verify `run_harness_loop` receives the same `base_args`, `base_env`, child cwd, structured output settings, noise filters, stream verbosity, dispatch context, and materialized prompt data that the old harness branch used.
- [ ] Confirm direct compose summary emission now consistently flows through `policy::emit_stream_summary`; update tests only for intentional section-separator differences documented in the spec.
- [ ] Confirm inline compose closure now consistently flows through `wrap/inline.rs::try_inline_closure` and uses the structured `final_response`, not accumulated `assistant_text`.
- [ ] Confirm timeout and warning resolution still uses CLI timeout values plus parsed plan timeout values, and that bare plans preserve current no-frontmatter timeout behavior.
- [ ] Confirm lifecycle notifications fire once for success, failure, and pre-launch blocked failures after the outer manual non-harness guard is removed.
- [ ] Add or update tests for one failing-provider case to assert non-harness direct compose still returns the provider exit code and emits failure lifecycle behavior through the harness loop.
- [ ] Validation checkpoint: run targeted compose and inline-compose tests, including the new convergence tests and existing receipt/banner tests.
- [ ] Parallelizable: timeout/lifecycle verification can proceed separately from inline closure verification after Phase 3 compiles.

## Phase 5: Remove Dead Code and Drifted Comments

- [ ] Delete `CompositionExecutionMode` from `claudine/cli/src/commands/wrap/composition/mod.rs`.
- [ ] Delete `execute_without_harness` from `claudine/cli/src/commands/wrap/composition/mod.rs`.
- [ ] Delete `composition/structured.rs::run_structured_branch` if no call sites remain; keep lower-level shared structured runners that `harness_orch.rs` still uses.
- [ ] Delete `composition/legacy_goose.rs` or reduce it to only code still used by the harness attempt path; remove its dependency on `CompositionExecutionMode`.
- [ ] Delete `composition/inline_guards.rs::apply_inline_closure` if no call sites remain, without touching the library closure implementation used by `wrap/inline.rs`.
- [ ] Remove stale doc comments and inline comments that describe a non-harness execution path, manual lifecycle guard ownership, or bespoke summary emission.
- [ ] Update `claudine/docs/topics/composition.md` and the claudine skill docs only if public behavior or architecture docs still describe separate harness and non-harness execution paths.
- [ ] Validation checkpoint: run `rg -n "execute_without_harness|CompositionExecutionMode|run_structured_branch|inline_guards|non-harness path|without harness" claudine/cli/src claudine/docs claudine/.claude/skills` and confirm remaining hits are intentional historical references or removed.
- [ ] Parallelizable: documentation drift cleanup can start once Phase 3 establishes the final behavior, but should be reviewed after dead-code deletion.

## Phase 6: Final Verification and Handoff

- [ ] Run `cargo test -p claudine harness_plan_for_bare_composition --lib --color=never` or the exact helper-test target chosen in Phase 2.
- [ ] Run the new convergence integration tests, for example `cargo test -p claudine-cli --test always_harness --color=never`.
- [ ] Run existing targeted composition coverage: `cargo test -p claudine-cli --test compose_receipt_banner --color=never`, `cargo test -p claudine-cli --test compose_schema_cli --color=never`, and `cargo test -p claudine-cli --test loop_cli --color=never`.
- [ ] Run `cargo check -p claudine -p claudine-cli --color=never`.
- [ ] If targeted checks are clean, run the claudine-area test recipe from `claudine/justfile` that best matches current repo convention.
- [ ] Perform a manual smoke test with fake provider scripts for `compose`, `inline-compose`, harness-enabled `compose`, and harness-enabled `inline-compose`; verify one provider launch per invocation and no unexpected retries.
- [ ] Inspect `git diff -- claudine/lib/src/harness claudine/cli/src/commands/wrap claudine/cli/tests claudine/docs/topics/composition.md claudine/.claude/skills/claudine` and confirm the change set is limited to the always-harness migration, tests, and required comment/doc drift fixes.
- [ ] Validation checkpoint: acceptance is met when bare and harness composition use the same harness-loop execution path, convergence tests pass, deleted duplicate code has no references, and targeted compile/tests are clean.

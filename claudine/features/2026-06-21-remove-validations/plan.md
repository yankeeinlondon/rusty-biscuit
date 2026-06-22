---
agent: codex/
phases: 7
created: 2026-06-22
start_phase: 1
yolo: true
---

# Remove Harness Validations and Handlers Execution Plan

## Phase 1: Lifecycle Readiness Gate

- [ ] Confirm the lifecycle dependency from `claudine/features/2026-05-12-lifecycle/spec.md` is implemented and merged, including `initialize`, `success`, `failure`, `finalize`, lifecycle `stack`, `Error`, `Skip`, `Proxy`, `Retry`, `Resume`, and `Requeue`.
- [ ] Verify lifecycle `blocked` and `failure` recovery actions are covered by existing tests or add minimal missing coverage before removing `resolve_handler`.
- [ ] Confirm lifecycle parsing already owns typed diagnostics for lifecycle keys so the removed-key diagnostic can run before generic unknown-field validation.
- [ ] Identify the current owner modules for lifecycle frontmatter validation, lifecycle error variants, frontmatter excerpt enrichment, shell audit collection, timeout parsing, runaway guards, and attempt classification.
- [ ] Run `rg "pre_checks|post_checks|handle_|handle:|deviate|evaluate_pre_checks|evaluate_post_checks|capture_pre_run_snapshot|resolve_handler|PreRunSnapshot|ValidationRule|ValidationKind|HandlerTable" claudine` and save the hit list as the deletion checklist.
- [ ] Validation checkpoint: no deletion starts until lifecycle recovery behavior can replace `handle_timeout` and `handle_agent_failure` without losing retry/resume coverage.

Parallelizable: lifecycle readiness review, symbol inventory, and doc inventory can run in parallel.

## Phase 2: Compatibility Diagnostics

- [ ] Add a dedicated `CompositionError` variant for removed validation/handler DSL keys carrying source path, offending key, and replacement guidance.
- [ ] Implement the removed-key scanner before generic lifecycle unknown-field validation.
- [ ] Reject exact top-level keys `pre_checks`, `post_checks`, `handle`, and `deviate`.
- [ ] Reject any top-level key matching `handle_` plus a non-empty suffix, including subject-specific keys such as `handle_timeout` and `handle_inline_body_unchanged`.
- [ ] Map diagnostics to the replacement surfaces: `pre_checks` to `initialize` or `start` stack, `post_checks` to `success` or `finalize` stack, `handle_*` to `blocked` or `failure` recovery actions, `handle` to lifecycle shell/action bridge, and `deviate` to lifecycle shell action plus recovery action.
- [ ] Wire the new error through existing frontmatter excerpt enrichment so TTY-capable output highlights the removed key.
- [ ] Confirm non-color and piped output strips escapes and still includes source path, key, and replacement guidance.
- [ ] Add L1 tests for `pre_checks`, `post_checks`, `handle`, `handle_timeout`, `handle_inline_body_unchanged`, and `deviate`.
- [ ] Validation checkpoint: removed keys fail with typed, actionable errors rather than being accepted, ignored, or reported as generic unknown fields.

Parallelizable: diagnostic tests for individual removed keys can be authored independently after the scanner contract is defined.

## Phase 3: Remove Validation Models and Evaluation

- [ ] Delete or reduce `claudine/lib/src/harness/validate/` so `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`, `PreRunSnapshot`, and `check_write_permission` no longer exist unless a kept surface still proves a dependency.
- [ ] Remove `claudine/lib/src/harness/parse/validations.rs`.
- [ ] Remove `ValidationRule`, `ValidationKind`, and validation-only `HandlerTable` fields from harness model types.
- [ ] Remove validation-specific path-resolution code in `claudine/lib/src/harness/resolve.rs` unless shell audit or timeout infrastructure still needs a narrow helper.
- [ ] Trim `claudine/lib/src/harness/failure.rs` to retain process termination, attempt outcome, `FailureEvent`, and failure classification while removing validation-only event, phase, and failure taxonomy.
- [ ] Remove validation-specific report rendering from `claudine/lib/src/harness/report.rs`, including pre/post validation sections and rule-source reporting.
- [ ] Update harness module exports so removed validation APIs are no longer public or reachable.
- [ ] Validation checkpoint: `rg "evaluate_pre_checks|evaluate_post_checks|capture_pre_run_snapshot|PreRunSnapshot|ValidationRule|ValidationKind|ValidationFailure|FailurePhase::PreCheck|FailurePhase::PostCheck" claudine/lib` returns no active code references.

Parallelizable: model cleanup, failure taxonomy cleanup, and report cleanup can proceed in parallel after the removed-key diagnostics are in place.

## Phase 4: Remove Handler Recovery DSL

- [ ] Delete or reduce `claudine/lib/src/harness/handlers.rs` and `claudine/lib/src/harness/parse/handlers.rs` so `resolve_handler`, `FailureContext`, `HandlerAction`, `execute_deviate_command`, `validate_resume`, and `build_*_failure_context` are gone.
- [ ] Remove handler table parsing for subject-specific handlers, generic handlers, `handle:`, and `deviate:`.
- [ ] Replace `try_resolve_handler` recovery branches in `claudine/cli/src/commands/wrap/resume.rs` with lifecycle `failure` or `blocked` event recovery routing.
- [ ] Replace handler recovery branches in `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` with lifecycle recovery action execution.
- [ ] Preserve agent-failure classification inputs needed by lifecycle recovery, including timeout, interruption, abort, exit status, and stream failure details.
- [ ] Add or update an end-to-end test proving a provider failure recovers through a lifecycle `failure` `Retry` or `Resume` action.
- [ ] Validation checkpoint: `rg "resolve_handler|try_resolve_handler|HandlerAction|FailureContext|execute_deviate_command|validate_resume|handle_agent_failure|handle_timeout" claudine/lib claudine/cli` finds no removed recovery path in active code.

Parallelizable: parser removal and CLI recovery replacement can be developed separately once lifecycle recovery APIs are confirmed.

## Phase 5: Update Wrap and Composition Orchestration

- [ ] Remove pre-check evaluation from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`.
- [ ] Remove post-check evaluation from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`.
- [ ] Remove harness snapshot capture from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` and `claudine/cli/src/commands/wrap/composition/mod.rs`.
- [ ] Confirm shell audit still runs during pre-flight and walks every reachable lifecycle stack shell command.
- [ ] Confirm schema validation still produces `blocked` behavior where required by lifecycle orchestration.
- [ ] Confirm timeout parsing and relational checks still accept `timeout`, `timeout_warn`, `step_timeout`, and `step_timeout_warn`.
- [ ] Confirm runaway guards and `ProcessTermination::Aborted` continue routing to lifecycle failure without invoking removed handler retry logic.
- [ ] Remove or repoint `claudine/cli/src/bin/validation_reporter_pty_harness.rs` and `claudine/cli/tests/fixtures/validation_reporter/missing_file.md` to lifecycle behavior.
- [ ] Add or update an inline-compose regression proving an agent-modified `prompt` frontmatter property is reverted by `composition/closure.rs` after harness snapshot removal.
- [ ] Validation checkpoint: shell audit denial still routes to `blocked`, timeout configuration still validates, and inline-compose frontmatter restoration still works.

Parallelizable: orchestration call-site deletion, validation reporter cleanup, and inline-compose regression coverage can proceed in parallel after Phases 3 and 4 land.

## Phase 6: Documentation and Metadata

- [ ] Update `claudine/features/2026-05-12-lifecycle/spec.md` wording so pre-flight means shell audit plus schema validation only.
- [ ] Update `claudine/docs/topics/composition.md` to remove accepted validation/handler DSL documentation and add the migration mapping table.
- [ ] Update `claudine/docs/topics/frontmatter-properties.md` so removed keys are documented only as rejected legacy keys, if referenced at all.
- [ ] Remove or replace `claudine/docs/topics/pre-flight-checks.md` with lifecycle pre-flight wording.
- [ ] Remove or rewrite `.claude/skills/claudine/validations-and-handlers.md` so it points to lifecycle stacks instead of documenting the retired DSL.
- [ ] Update `.claude/skills/claudine/SKILL.md` module map so `harness` is described as shell audit, timeouts, runtime classification, speech helpers, and kept lifecycle infrastructure.
- [ ] Remove validation/handler DSL references from CLI reference, help text, shell-completion metadata, frontmatter-completion metadata, and any examples.
- [ ] Validation checkpoint: `rg "pre_checks|post_checks|handle_|handle:|deviate:|ValidationKind|validation DSL|handler DSL" claudine/docs claudine/.claude claudine/cli` shows only intentional legacy diagnostic or migration references.

Parallelizable: docs updates, skill updates, and CLI metadata cleanup can proceed in parallel once the final code surface is known.

## Phase 7: Final Verification and Cleanup

- [ ] Run `just test` from the `claudine` package area for unit coverage.
- [ ] Run `just test-l2` from the `claudine` package area for integration coverage.
- [ ] Run `just lint` from the `claudine` package area.
- [ ] Run the regression sweep with `rg` for all removed symbols and removed frontmatter keys across `claudine/lib`, `claudine/cli`, `claudine/docs`, and `.claude/skills/claudine`.
- [ ] Confirm acceptance criteria: removed keys reject with typed diagnostics, removed functions and call sites are gone, kept surfaces still work, lifecycle recovery covers former handler behavior, inline-compose closure still restores frontmatter, and docs no longer advertise the DSL.
- [ ] Review touched rustdoc and inline comments near changed behavior, deleting stale validation/handler comments and keeping comments that explain retained contracts.
- [ ] Confirm no unrelated formatting churn was introduced and do not run `cargo fmt` unless explicitly requested.
- [ ] Validation checkpoint: all required tests and sweeps pass, or each failure is documented with owner, file, and next action.

Parallelizable: `rg` sweeps and documentation checks can run while tests execute, but final acceptance review depends on all test results.

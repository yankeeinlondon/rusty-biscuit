---
agent: codex/
phases: 8
created: 2026-06-22
start_phase: 1
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/features/2026-06-21-remove-validations/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/cli/tests/compose_cli.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/loop_cli.rs
  - claudine/cli/tests/wrap_compose_agent.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
docs_updated_during_phase_2:
  - claudine/features/2026-06-21-remove-validations/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/sequence/phase1c.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_lifecycle_loop.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/parse/handlers.rs
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/report.rs
docs_updated_during_phase_3:
  - claudine/lib/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/sequence/phase1c.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/lib/src/composition/lifecycle_context.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/runtime.rs
  - claudine/lib/src/harness/shell.rs
  - claudine/lib/src/harness/handlers.rs
  - claudine/lib/src/harness/parse/handlers.rs
  - claudine/lib/src/harness/parse/overlays.rs
  - claudine/lib/tests/runaway_handler_payload.rs
docs_updated_during_phase_4:
  - claudine/features/2026-06-21-remove-validations/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_lifecycle_loop.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
docs_updated_during_phase_5:
  - claudine/lib/README.md
  - claudine/features/2026-06-21-remove-validations/plan.md
  - claudine/features/2026-06-21-remove-validations/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/signal-handling.md
  - claudine/docs/topics/pre-flight-checks.md
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/README.md
  - claudine/features/2026-05-12-lifecycle/spec.md
  - claudine/features/2026-06-21-remove-validations/plan.md
docs_created_during_phase_6: []
docs_deleted_during_phase_6:
  - claudine/docs/topics/validations-and-handlers.md
  - claudine/docs/topics/validations/validation-rules.md
  - claudine/docs/topics/validations/validations.md
  - claudine/docs/topics/validations/pre-validation.md
  - claudine/docs/topics/validations/post-validation.md
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/validations-and-handlers.md
source_files_during_phase_7:
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/speech.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_7:
  - claudine/features/2026-06-21-remove-validations/plan.md
  - claudine/lib/README.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - claudine/docs/research/non-interactive-sessions/_details.md
  - claudine/features/2026-06-21-remove-validations/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_code:
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/sequence/phase1c.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/compose_cli.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_lifecycle_loop.rs
  - claudine/cli/tests/loop_cli.rs
  - claudine/cli/tests/wrap_compose_agent.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle_context.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/handlers.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/parse/handlers.rs
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/parse/overlays.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/runtime.rs
  - claudine/lib/src/harness/shell.rs
  - claudine/lib/src/harness/speech.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/tests/runaway_handler_payload.rs
documentation:
  - claudine/README.md
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/pre-flight-checks.md
  - claudine/docs/topics/signal-handling.md
  - claudine/docs/topics/timeouts.md
  - claudine/features/2026-05-12-lifecycle/spec.md
  - claudine/features/2026-06-21-remove-validations/plan.md
  - claudine/features/2026-06-21-remove-validations/spec.md
  - claudine/lib/README.md
packages:
  - claudine
---

# Remove Harness Validations and Handlers Execution Plan

## Phase 1: Lifecycle Readiness Gate

- [x] Confirm the lifecycle dependency from `claudine/features/2026-05-12-lifecycle/spec.md` is implemented and merged, including `initialize`, `success`, `failure`, `finalize`, lifecycle `stack`, `Error`, `Skip`, `Proxy`, `Retry`, `Resume`, and `Requeue`.
- [x] Verify lifecycle `blocked` and `failure` recovery actions are covered by existing tests or add minimal missing coverage before removing `resolve_handler`.
- [x] Confirm lifecycle parsing already owns typed diagnostics for lifecycle keys so the removed-key diagnostic can run before generic unknown-field validation.
- [x] Identify the current owner modules for lifecycle frontmatter validation, lifecycle error variants, frontmatter excerpt enrichment, shell audit collection, timeout parsing, runaway guards, and attempt classification.
- [x] Run `rg "pre_checks|post_checks|handle_|handle:|deviate|evaluate_pre_checks|evaluate_post_checks|capture_pre_run_snapshot|resolve_handler|PreRunSnapshot|ValidationRule|ValidationKind|HandlerTable" claudine` and save the hit list as the deletion checklist.
- [x] Validation checkpoint: no deletion starts until lifecycle recovery behavior can replace `handle_timeout` and `handle_agent_failure` without losing retry/resume coverage.

Parallelizable: lifecycle readiness review, symbol inventory, and doc inventory can run in parallel.

## Phase 2: Compatibility Diagnostics

- [x] Add a dedicated `CompositionError` variant for removed validation/handler DSL keys carrying source path, offending key, and replacement guidance.
- [x] Implement the removed-key scanner before generic lifecycle unknown-field validation.
- [x] Reject exact top-level keys `pre_checks`, `post_checks`, `handle`, and `deviate`.
- [x] Reject any top-level key matching `handle_` plus a non-empty suffix, including subject-specific keys such as `handle_timeout` and `handle_inline_body_unchanged`.
- [x] Map diagnostics to the replacement surfaces: `pre_checks` to `initialize` or `start` stack, `post_checks` to `success` or `finalize` stack, `handle_*` to `blocked` or `failure` recovery actions, `handle` to lifecycle shell/action bridge, and `deviate` to lifecycle shell action plus recovery action.
- [x] Wire the new error through existing frontmatter excerpt enrichment so TTY-capable output highlights the removed key.
- [x] Confirm non-color and piped output strips escapes and still includes source path, key, and replacement guidance.
- [x] Add L1 tests for `pre_checks`, `post_checks`, `handle`, `handle_timeout`, `handle_inline_body_unchanged`, and `deviate`.
- [x] Validation checkpoint: removed keys fail with typed, actionable errors rather than being accepted, ignored, or reported as generic unknown fields.

Parallelizable: diagnostic tests for individual removed keys can be authored independently after the scanner contract is defined.

## Phase 3: Remove Validation Models and Evaluation

- [x] Delete or reduce `claudine/lib/src/harness/validate/` so `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`, `PreRunSnapshot`, and `check_write_permission` no longer exist unless a kept surface still proves a dependency.
- [x] Remove `claudine/lib/src/harness/parse/validations.rs`.
- [x] Remove `ValidationRule`, `ValidationKind`, and validation-only `HandlerTable` fields from harness model types.
- [x] Remove validation-specific path-resolution code in `claudine/lib/src/harness/resolve.rs` unless shell audit or timeout infrastructure still needs a narrow helper.
- [x] Trim `claudine/lib/src/harness/failure.rs` to retain process termination, attempt outcome, `FailureEvent`, and failure classification while removing validation-only event, phase, and failure taxonomy.
- [x] Remove validation-specific report rendering from `claudine/lib/src/harness/report.rs`, including pre/post validation sections and rule-source reporting.
- [x] Update harness module exports so removed validation APIs are no longer public or reachable.
- [x] Validation checkpoint: `rg "evaluate_pre_checks|evaluate_post_checks|capture_pre_run_snapshot|PreRunSnapshot|ValidationRule|ValidationKind|ValidationFailure|FailurePhase::PreCheck|FailurePhase::PostCheck" claudine/lib` returns no active code references.

Parallelizable: model cleanup, failure taxonomy cleanup, and report cleanup can proceed in parallel after the removed-key diagnostics are in place.

## Phase 4: Remove Handler Recovery DSL

- [x] Delete or reduce `claudine/lib/src/harness/handlers.rs` and `claudine/lib/src/harness/parse/handlers.rs` so `resolve_handler`, `FailureContext`, `HandlerAction`, `execute_deviate_command`, `validate_resume`, and `build_*_failure_context` are gone.
- [x] Remove handler table parsing for subject-specific handlers, generic handlers, `handle:`, and `deviate:`.
- [x] Replace `try_resolve_handler` recovery branches in `claudine/cli/src/commands/wrap/resume.rs` with lifecycle `failure` or `blocked` event recovery routing.
- [x] Replace handler recovery branches in `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` with lifecycle recovery action execution.
- [x] Preserve agent-failure classification inputs needed by lifecycle recovery, including timeout, interruption, abort, exit status, and stream failure details.
- [x] Add or update an end-to-end test proving a provider failure recovers through a lifecycle `failure` `Retry` or `Resume` action.
- [x] Validation checkpoint: `rg "resolve_handler|try_resolve_handler|HandlerAction|FailureContext|execute_deviate_command|validate_resume|handle_agent_failure|handle_timeout" claudine/lib claudine/cli` finds no removed recovery path in active code.

Parallelizable: parser removal and CLI recovery replacement can be developed separately once lifecycle recovery APIs are confirmed.

## Phase 5: Update Wrap and Composition Orchestration

- [x] Remove pre-check evaluation from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`.
- [x] Remove post-check evaluation from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`.
- [x] Remove harness snapshot capture from `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` and `claudine/cli/src/commands/wrap/composition/mod.rs`.
- [x] Confirm shell audit still runs during pre-flight and walks every reachable lifecycle stack shell command.
- [x] Confirm schema validation still produces `blocked` behavior where required by lifecycle orchestration.
- [x] Confirm timeout parsing and relational checks still accept `timeout`, `timeout_warn`, `step_timeout`, and `step_timeout_warn`.
- [x] Confirm runaway guards and `ProcessTermination::Aborted` continue routing to lifecycle failure without invoking removed handler retry logic.
- [x] Remove or repoint `claudine/cli/src/bin/validation_reporter_pty_harness.rs` and `claudine/cli/tests/fixtures/validation_reporter/missing_file.md` to lifecycle behavior.
- [x] Add or update an inline-compose regression proving an agent-modified `prompt` frontmatter property is reverted by `composition/closure.rs` after harness snapshot removal.
- [x] Validation checkpoint: shell audit denial still routes to `blocked`, timeout configuration still validates, and inline-compose frontmatter restoration still works.

Parallelizable: orchestration call-site deletion, validation reporter cleanup, and inline-compose regression coverage can proceed in parallel after Phases 3 and 4 land.

## Phase 6: Documentation and Metadata

- [x] Update `claudine/features/2026-05-12-lifecycle/spec.md` wording so pre-flight means shell audit plus schema validation only.
- [x] Update `claudine/docs/topics/composition.md` to remove accepted validation/handler DSL documentation and add the migration mapping table.
- [x] Update `claudine/docs/topics/frontmatter-properties.md` so removed keys are documented only as rejected legacy keys, if referenced at all.
- [x] Remove or replace `claudine/docs/topics/pre-flight-checks.md` with lifecycle pre-flight wording.
- [x] Remove or rewrite `.claude/skills/claudine/validations-and-handlers.md` so it points to lifecycle stacks instead of documenting the retired DSL.
- [x] Update `.claude/skills/claudine/SKILL.md` module map so `harness` is described as shell audit, timeouts, runtime classification, speech helpers, and kept lifecycle infrastructure.
- [x] Remove validation/handler DSL references from CLI reference, help text, shell-completion metadata, frontmatter-completion metadata, and any examples.
- [x] Validation checkpoint: `rg "pre_checks|post_checks|handle_|handle:|deviate:|ValidationKind|validation DSL|handler DSL" claudine/docs claudine/.claude claudine/cli` shows only intentional legacy diagnostic or migration references.

Parallelizable: docs updates, skill updates, and CLI metadata cleanup can proceed in parallel once the final code surface is known.

## Phase 7: Final Verification and Cleanup

- [x] Run `just test` from the `claudine` package area for unit coverage.
- [x] Run `just test-l2` from the `claudine` package area for integration coverage.
- [x] Run `just lint` from the `claudine` package area.
- [x] Run the regression sweep with `rg` for all removed symbols and removed frontmatter keys across `claudine/lib`, `claudine/cli`, `claudine/docs`, and `.claude/skills/claudine`.
- [x] Confirm acceptance criteria: removed keys reject with typed diagnostics, removed functions and call sites are gone, kept surfaces still work, lifecycle recovery covers former handler behavior, inline-compose closure still restores frontmatter, and docs no longer advertise the DSL.
- [x] Review touched rustdoc and inline comments near changed behavior, deleting stale validation/handler comments and keeping comments that explain retained contracts.
- [x] Confirm no unrelated formatting churn was introduced and do not run `cargo fmt` unless explicitly requested.
- [x] Validation checkpoint: all required tests and sweeps pass, or each failure is documented with owner, file, and next action.

Parallelizable: `rg` sweeps and documentation checks can run while tests execute, but final acceptance review depends on all test results.

## Phase 8: Post-Implementation Audit and Stray Driver Migration

- [x] Run a full `rg` sweep for removed frontmatter keys (`pre_checks`, `post_checks`, `handle_*`, `handle:`, `deviate:`) across **active** composition documents (`claudine/docs` excluding `docs/research`, `claudine/prompts`) and confirm zero hits outside historical `_completed/` feature records.
- [x] Identify and migrate the leftover research sequence driver `claudine/docs/research/non-interactive-sessions/_details.md` that still declared the removed `post_checks:` key (plus retired `skip_when:` and `error:` aliases) to lifecycle stacks (`initialize` + `skip`, `finalize` + `error`, `failure`).
- [x] Confirm the migrated driver uses valid lifecycle stack syntax: `initialize` stack with `when: file_exists(...)` + `skip`, `finalize` stack with `when: !file_exists(...)` + `error(...)`, and `failure` replacing the old `error:` alias.
- [x] Run `just test` from the `claudine` package area and confirm all 1681 tests pass.
- [x] Run `just lint` from the `claudine` package area and confirm zero warnings.
- [x] Re-run the phase 3/4/6/7 regression sweeps (`evaluate_pre_checks`, `resolve_handler`, removed-key frontmatter) and confirm they remain clean after the driver migration.
- [x] Validation checkpoint: no active composition document in `claudine/` declares a removed DSL key; the only remaining references are intentional migration mappings in docs/skills and historical records in `features/_completed/`.

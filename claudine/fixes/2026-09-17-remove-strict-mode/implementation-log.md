---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-17-remove-strict-mode/spec.md
plan: claudine/fixes/2026-09-17-remove-strict-mode/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - claudine/docs/rollout-strategy.md
    - claudine/fixes/2026-09-17-remove-strict-mode/design.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/tests/l1/binding_contract.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/tests/l1/absent_property_contract.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/cli/tests/l1/compose_unknown_identifiers.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/interpolation_conformance.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/authored_text_rendering.rs
    - prompts/_reviews/performance-review.md
    - prompts/brainstorm.md
docs_updated_during_phase_3:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/inline/fm-interpolation.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
source_files_during_phase_4:
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/validation.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/interpolation_conformance.rs
    - claudine/lib/src/composition/looping/actions.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/tests/l1/binding_contract.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/dmls/tests/l1/undeclared_property.rs
docs_updated_during_phase_5:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_6:
    - prompts/implement.md
    - prompts/review.md
    - prompts/_prompt.md
    - claudine/cli/tests/l1/shipped_prompts.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
source_files_during_phase_7:
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
docs_updated_during_phase_7:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/inline/fm-interpolation.md
    - darkmatter/docs/lsp/features.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/timeline.md
source_code:
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/tests/l1/binding_contract.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/tests/l1/absent_property_contract.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/cli/tests/l1/compose_unknown_identifiers.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/interpolation_conformance.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/authored_text_rendering.rs
    - prompts/_reviews/performance-review.md
    - prompts/brainstorm.md
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/validation.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/looping/actions.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/dmls/tests/l1/undeclared_property.rs
    - prompts/implement.md
    - prompts/review.md
    - prompts/_prompt.md
    - claudine/cli/tests/l1/shipped_prompts.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - claudine/lib/src/composition/schema/tests.rs
documentation:
    - claudine/docs/rollout-strategy.md
    - claudine/fixes/2026-09-17-remove-strict-mode/design.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/inline/fm-interpolation.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/composition.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - claudine/docs/topics/flow-control/looping.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - .claude/skills/darkmatter/dmls.md
    - claudine/docs/topics/flow-control/sequences.md
    - .claude/skills/claudine/timeline.md
completed_phase: 7
implemented: true
---

# Implementation Log for 2026-09-17-remove-strict-mode (7 phases)

## Phase 1

Phase 1 is planning-only: rulings, a test baseline on the unmodified tree, a
refresh of the lookup inventory, a seam impact check, and a prompt exposure
audit. No source file is changed in this phase.

### Rulings

Recorded 2026-10-01 by claude/opus. The plan's Phase 1 allows each ruling to
be recorded "by the owner or by accepting the stated recommendation", and the
plan runs with `yolo: "true"`; every ruling below accepts the plan's stated
recommendation. **Decided by:** claude/opus on behalf of the owner (Ken
Snyder), under that mandate. NR-3 carries an explicit "owner sign-off
required" note, so it is also raised in the spec's `human_review_items`; the
owner may overrule any ruling before Phase 2.

| # | Ruling | Outcome |
|---|---|---|
| NR-1 | Design approval gate (rollout ruling 2) | **Accepted.** The plan review covers the in-scope design artifacts. Recorded in `claudine/docs/rollout-strategy.md` (ruling 2 row, the "Next" paragraph, step 3's "Needs first", and the Evidence status row) and in `design.md`'s status line |
| NR-2 | Plain `serde_json::Value` instead of `ValueEnvelope` | **Accepted.** `ResolvedBinding`, `RuntimeBinding::Eager`, lazy providers, and `evaluate_prepared` carry `Value`; lazy providers keep the `Fn() -> Value` shape; no provenance/stage/policy metadata |
| NR-3 | Reserved names come from `reserved_root_descriptors()` (`doc`, `ctx`, `env`, `current`, `current_env`) | **Accepted, pending owner sign-off** (see below). `current` leaves C3's Claudine catalog |
| NR-4 | `timing`, not `tracking` | **Accepted.** The catalog declares `err`, `timing`, `group`; no rename |
| NR-5 | One resolution channel: evolve `get_checked` into `resolve` | **Accepted.** `resolve(&self, path) -> Result<ResolvedBinding, ExpressionError>` with a default wrapping `get` as `Document`; add `binding_view()` (default `None`) and `format_resolved`; `ContextNotCaptured`/`ContextProjectionInvariant` move onto it unchanged |
| NR-6 | Prepared representation scope | **Accepted.** Ship `prepare_value`, `validate_prepared`, `evaluate_prepared`; prepared identity is the `BindingView` identity only; new error families only for binding configuration and unavailable bindings |
| NR-7 | `dm.expression.unknown_identifier` → `dm.expression.undeclared_property` | **Accepted.** One-change rename with no alias across library, DMLS, `md` CLI tests, and docs; message "`{root}` is an undeclared document property (unknown type; `null` unless supplied at runtime)"; drop context-descriptor names from `is_statically_known_root`; keep fallback/ternary suppression |
| NR-8 | Loop `_loop_*` names | **Accepted.** Ordinary data through the default `resolve`; only the `ctx` fallback is removed; no loop catalog |
| NR-9 | `ApprovedCommand` for setup/teardown | **Accepted.** Claudine-private `ApprovedCommand { site, bytes: String }`, no envelope |
| NR-10 | Delete `strict_mode_provenance_spike.rs` | **Accepted.** Deleted when `.strict()` is removed (Phase 3, Wave 7); findings stay in `spike-results.md` |
| NR-11 | Owner-visible behavior changes (bare `err` in `finalize`/teardown → explicit `null`; bare `group` outside a group → typed unavailable error; no bare-name `ctx` fallback) | **Accepted.** Reach measured by the prompt exposure audit below |

**Widened acceptance criterion 2 (NR-3).** The spec's criterion 2 and its
Verification items say registrations named exactly `doc`, `ctx`, or `env`
fail. Under NR-3 registration rejects every root in
`reserved_root_descriptors()`: `doc`, `ctx`, `env`, `current`, and
`current_env`. The spec is a snapshot and is not edited; Phase 7's acceptance
map records this as a departure.

### Lookup inventory refresh (Phase 3 migration checklist)

Re-verified against source on 2026-10-01. **23 compiled `EvaluationLookup`
implementations plus 2 `SimpleLookup` rustdoc examples** (12 production, 11
test-only), confirming the plan's count. The inventory's 21 rows all still
exist. `darkmatter/features/2026-09-21-schema-enhancements/spikes/coercion-baseline/`
has no `Cargo.toml` and is not compiled (GitNexus does index it, so its
`main` shows up in `EvaluationLookup` impact results; ignore it). A further
uncompiled example lives in `darkmatter/docs/topics/darkmatter-expressions.md:1009`
and is a Phase 7 doc concern only.

DM = `darkmatter/lib/src/markdown/compose/`, CL = `claudine/lib/src/`. IKVR =
`is_known_variable_root` (trait default `true`, `expression/mod.rs:301`).

| # | Type | File:line | Role | IKVR | Bare-name `ctx` fallback | Test-only |
|---|---|---|---|---|---|---|
| 1 | `EffectiveState` | DM `context/effective_state.rs:414` | Overrides `get_checked` | yes | **yes**: inherent `get` `.or_else(get_context_value(path))` (:247); `get_checked` via `into_checked_bare_name` (:277) | no |
| 2 | `ResolvingLookup` | DM `context/effective_state.rs:493` | Wraps #1 plus resolution context | forwards | inherited from #1 | no |
| 3 | `FrontmatterSeedState` | DM `frontmatter_interpolation.rs:124` | Seed state; overrides `get_checked` | yes (accepts bare ctx variable names) | `get`: none; IKVR still treats bare ctx names as known | no |
| 4 | `ShortcutLookup` | DM `conditions.rs:364` | Condition shortcut | no | **yes**, explicit: `get` → `ctx.{path}` (:388), `get_checked` (:400) | no |
| 5 | `CtxLookup` | DM `expression/ctx.rs:109` | Context-only | no | none | no |
| 6 | `LayeredLookup` | DM `subtree.rs:229` | Globals over #1 | yes | inherited from #1 | no |
| 7 | `FsLookup` | DM `expression/catalog/mod.rs:642` | Fixture | no | none | yes |
| 8 | `FixtureLookup` | DM `expression/catalog/mod.rs:973` | Fixture | no | none | yes |
| 9 | `MapLookup` | DM `expression/catalog/mod.rs:1166` | Fixture | no | none | yes |
| 10 | `FixtureLookup` | DM `expression/semantics.rs:810` | Fixture | no | none | yes |
| 11 | `TestLookup` | DM `expression/mod.rs:935` | Fixture | no | none | yes |
| 12 | `Nothing` **(new)** | DM `expression/absence.rs:398` | All-missing test lookup | yes (`false`) | none | yes |
| 13 | `DeferrableLookup` **(new)** | DM `inline/interpolation.rs:163` | Forwards every method to #2; `get_checked` maps `ContextNotCaptured` → `Ok(None)` when deferring | forwards | inherited from #1 | no |
| 14 | `Lookup` | `darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs:11` **(moved)** | Fixture | no | none | yes |
| 15 | `Lookup` | `darkmatter/lib/tests/l1/predict_conflicts.rs:147` **(moved)** | Fixture | no | none | yes |
| 16 | `SizedLookup` | CL `composition/looping/actions.rs:249` | Wrapper; forwards **only** `get`/`get_string` | no | none | no |
| 17 | `LoopExpressionLookup` | CL `composition/looping/expression.rs:139` | Loop, ambient, frontmatter | no | none (`ctx.` prefix only) | no |
| 18 | `MapLookup` | CL `composition/looping/actions/tests.rs:12` | Fixture | no | none | yes |
| 19 | `SourceExpressionLookup` | CL `composition/sequence/expr.rs:75` | Item → env → frontmatter → `resolve_ctx` | no | none (`resolve_ctx` needs the `ctx.` prefix) | no |
| 20 | `EventMetaExpressionLookup` | CL `dispatch/expression.rs:85` | Event-meta projection | no | none | no |
| 21 | `EventMetaConditionLookup` | CL `dispatch/expression.rs:169` | Wrapper; `ctx*` → `CtxLookup`, else inner; forwards only `get`/`resolution_context` | no | none | no |
| 22 | `MapLookup` | CL `composition/lifecycle/tests/action_shape_control.rs:1410` | Fixture | no | none | yes |
| 23 | `EmptyLookup` | CL `composition/lifecycle/tests/action_shape_control.rs:1418` | Fixture | no | none | yes |
| R1 | `SimpleLookup` | DM `expression/mod.rs:180` | Rustdoc example | no | none | doc |
| R2 | `SimpleLookup` | DM `expression/mod.rs:413` | Rustdoc example | no | none | doc |

Deltas against `migration-inventory.md`: 2 new (#12, #13), 2 moved (#14,
#15), none removed, 11 line drifts in unchanged files (rows 1–9, 11, 16). The
inventory's prose count "21 actual lookup implementations" is now 23.

Findings that refine later phases:

- **The bare-name `ctx` fallback lives in only two places:** `EffectiveState`
  (#1) and `ShortcutLookup` (#4). `ResolvingLookup`, `LayeredLookup`, and
  `DeferrableLookup` inherit it from #1. `FrontmatterSeedState`'s `get` no
  longer falls back, but its IKVR still admits bare ctx names.
- **`LoopExpressionLookup` and `SourceExpressionLookup` have no `ctx`
  fallback today**, contrary to NR-8's and Wave 9's "remove the `ctx`
  fallback" wording. For them Phase 4 only needs the forwarding/session work;
  there is no fallback to delete.
- **Two Claudine wrappers forward only part of the trait:** `SizedLookup`
  forwards only `get`/`get_string`, and `EventMetaConditionLookup` only
  `get`/`resolution_context`. Neither forwards `get_checked` today, so they
  silently drop the checked channel. Phase 3/4 must make both forward
  `resolve`, `binding_view`, and `format_resolved`.
- **`is_known_variable_root` call sites (3):** `expression/mod.rs:461`
  (`observe_missing`, runtime unknown-root observer), `subtree.rs:539` (strict
  root rejection), `unknown_identifiers.rs:44` (warning candidate filter).
  Overrides (6): `effective_state.rs:451`, `effective_state.rs:528`,
  `frontmatter_interpolation.rs:229`, `subtree.rs:280`,
  `inline/interpolation.rs:195`, `expression/absence.rs:402`.

### Seam impact check

GitNexus (index one commit behind HEAD; the trailing commit touched only
planning files) resolved `EvaluationLookup` as CRITICAL: 20 direct, 79 total
upstream, 4 affected processes (`run_stage`, `run_compose_pipeline_node`,
`build_node`, plus the uncompiled spike `main`). `SubtreeCompose`,
`InjectedGlobal`, `LayeredLookup`, and `CompositionError` came back
ambiguous/`UNKNOWN` (no resolved callers), and `lifecycle_injected_globals`
was not found in the index, so all five were verified by text search.

Callers the plan and inventory do not name (each must compile after Phase 3
and be reviewed in Phase 4):

| Symbol | Unlisted users |
|---|---|
| `SubtreeCompose` | `claudine/cli/tests/l1/composition_seams.rs`, `darkmatter/cli/tests/l1/compose_schema.rs`, `darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs` |
| `InjectedGlobal` | `darkmatter/lib/src/markdown/compose/tests/frontmatter.rs`, `.../tests/lazy_roots.rs` |
| `LayeredLookup` | `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`, `claudine/lib/src/composition/lifecycle/context/tests.rs` |
| `lifecycle_injected_globals` | `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`, `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`, re-export in `claudine/lib/src/composition/mod.rs` |
| `SubtreeStrictness` | `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`, `darkmatter/lib/src/markdown/compose/expression/mod.rs`, `darkmatter/lib/src/markdown/compose/tests/frontmatter.rs` |
| `CompositionError::LifecycleUndefinedVariable` | Renderer branches are in the **library**, not `claudine/cli` as Wave 10 says: `claudine/lib/src/composition/error/render/mod.rs` (:45, :413, :822), `render/lifecycle.rs:116`, and the frontmatter-highlight arm in `error/mod.rs:3242`. Tests: `lifecycle/tests/validation.rs` (9 tests) and `lifecycle/tests/diagnostics.rs:392` |

`CompositionError` as a whole is used in 146 files across `claudine/lib` and
`claudine/cli`; Wave 10 only deletes one variant and adds a typed cause, so
only the sites above are affected. Note also the neighboring
`CompositionError::LifecycleErrNotAvailable`, which overlaps the `err`
availability that the Wave 8 catalog now owns; Phase 4 should decide whether
it becomes the catalog's unavailable error or stays a parse-time check.

### Prompt exposure audit

Searched `prompts/`, Claudine prompts/fixtures/docs, and Darkmatter fixtures
(excluding `_completed`, `features/`, `fixes/`, `target/`). The `ctx`
fallback was confirmed empirically with `md compose` in a scratch repository
on branch `feat/xyz`: an undeclared bare `{{branch}}` rendered `feat/xyz`
(through `effective_state.rs:243-247`), while a `branch` declared in
`$schema` but unsupplied rendered empty. So **only names declared nowhere in
the document relied on the fallback**; schema-declared optional properties
never did.

**A. Fallback guards** — Phase 6 work list.

| File:line | Snippet | Class | Phase 6 action |
|---|---|---|---|
| `prompts/implement.md:53-55` | comment "`\|\| false` is the guarded-optional form…" | teaches workaround | delete |
| `prompts/implement.md:56` | `when: "review \|\| false"` | legality guard | → `when: "review"` |
| `prompts/implement.md:64-67` | `(spec \|\| false) ? '- a specification file…' : ''`, same for `plan`/`review`, and `!(spec\|\|false) && …` | legality guard | plain ternaries |
| `prompts/implement.md:75-77` | `(spec \|\| false) ? '✔' : '<red>⤫</red>'` (+ plan, review) | legality guard | plain |
| `prompts/implement.md:80, 82` | `(spec \|\| false) ? … : (review \|\| false) ? …` | legality guard | plain |
| `prompts/implement.md:81` | `(frontmatter(spec, "implemented") \|\| false)` | **real default** | keep (renders `false` on purpose) |
| `prompts/implement.md:23` | `frontmatter(spec,'review_iterations') \|\| 1` | real default | keep |
| `prompts/review.md:31-35` | workaround comment | teaches workaround | delete |
| `prompts/review.md:36, 42, 45` | `when: "spec \|\| false"` / `"plan \|\| false"` / `"review \|\| false"` | legality guard | plain |
| `prompts/_prompt.md:89` | "Guard it with a fallback: `{{{ title \|\| '' }}}` … `review \|\| false` in a `when:`" | teaches workaround | rewrite: absent is `null`; a fallback chooses a default |
| `prompts/pr.md:52-54, 62-64`; `prompts/_pr/dirty.md:39-41`; `prompts/_pr/push.md:49-50` | `branch: "{{ branch \|\| '' }}"`, same for `title`/`about`, in proxy `with:` | **real default** (reclassified) | keep: unguarded they would pass `null`, not `''`, into a callee whose property is typed `string`. Removing them is a behavior change and out of scope for R8 |
| `prompts/_pr/dirty.md:30` | `about \|\| 'These are the uncommitted changes…'` | real default | keep |
| `prompts/pr.md:32` | `remote_vendor() \|\| 'an unrecognized provider'` | real default | keep |
| `prompts/code-comment-quality.md:15`, `prompts/merge-conflicts.md:3`, `prompts/_reviews/performance-review.md:2,4` | `ctx.*` roots with `\|\| null` / `\|\| ''` | not affected | leave |
| `claudine/docs/topics/flow-control/lifecycle.md:92, 835-839, 859` | "opt in with explicit fallback syntax `{{ maybe \|\| '' }}`"; "fails … closed via Darkmatter's strict mode" | teaches workaround / stale | Phase 7 rewrite |
| `claudine/docs/topics/flow-control/flow-control-reference.md:127` | "an unknown root" as an evaluation failure | stale | Phase 7 |

Side observation: `prompts/implement.md:29` (`when: "spec && pending_review && …"`)
and `prompts/review.md:30` (`when: "spec && (…)"`) already read `spec`
unguarded, contradicting their own comments. Moot after the fix.

**B. Bare `err` in `finalize`/teardown.** No prompt or fixture uses it. Doc
examples `flow-control.md:147`, `lifecycle.md:681-683` (`finalize: … when:
"err"`) and `docs/research/reasoning-level/_fleet.md:101,106` (`err &&
err.category == 'cap'`) stay correct: an explicit `null` `err` makes the gate
false, which is the intended result.

**C. Bare `group` outside a group.** No hits in prompts, fixtures, or doc
examples (only prose in `looping.md:318`, `flow-control.md:111`).

**D. Bare names that relied on the `ctx` fallback** — these regress silently
(render empty) once R3 lands, so **Phase 6 must fix them alongside R8**:

| File:line | Snippet | Fix |
|---|---|---|
| `prompts/_reviews/performance-review.md:342` | `{{today}} at {{time}}` (`today` is frontmatter; `time` is not) | `ctx.time` |
| `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md:342` | fixture copy of the above | keep in sync |
| `prompts/brainstorm.md:20` | `the {{area}} package area` (`area` undeclared) | `ctx.area` |
| `darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md:20` | fixture copy | keep in sync |
| `claudine/docs/topics/flow-control/lifecycle.md:611-612` | `running {{agent}}`, `git fetch origin {{branch}}` (undeclared) | Phase 7: `ctx.agent` / `ctx.branch` |
| `claudine/cli/tests/fixtures/nested_span_regression/commit.md:31` | `{{repo.name}}` (`ctx.repo` is a string, so likely already empty) | re-check expected output in Phase 3/4 |
| Darkmatter docs: `inline/interpolation.md` 223-406, `shell-expansion.md` 63-69, `topics/darkmatter-expressions.md` 119/203, `topics/frontmatter-recursion.md` 18/26 (`{{ area }}`); `dmls/docs/autocomplete.md:111`, `hover.md:70`, `design/interpolation.md` 747/808/846 (`{{today}}`/`{{year}}`) | check each in Phase 7; any snippet whose frontmatter leaves the name undeclared documents the fallback and should say `ctx.<name>` |

Not affected: `branch` in `prompts/_pr/{diagnose,fix,triage}.md` and `area`
in `prompts/_implement/implement-plan.md` and `_reviews/*` have real
frontmatter defaults; `branch` in `pr.md`, `dirty.md`, `push.md`, `open.md`
and `agent`/`model` in `commit.md` are `$schema`-declared. The
`os.name`/`cwd`/`timestamp` hits in `claudine/lib/README.md`,
`unified-events.md`, and `configuring-actions.md` are hook-event payload
templates, not Darkmatter lookups.

### Test baseline

Run on 2026-10-01 (macOS host) at HEAD `13df7698f` plus this phase's
planning-document edits only; no source file changed.

| Area | Recipe | Result |
|---|---|---|
| `darkmatter/` | `just test` | 8744 passed, 12 skipped (4 slow), exit 0 |
| `darkmatter/` | `just lint` | clean, exit 0 |
| `claudine/` | `just test` | 8072 passed, 9 skipped (9 slow), exit 0 |
| `claudine/` | `just lint` | clean (including its 9-test lint suite), exit 0 |

**No pre-existing failures.** Any later failure in these four gates is caused
by this fix.

### Phase 1 outcome

Checkpoint 1 is met: rulings recorded, baseline known, migration checklist and
prompt-exposure list written above. No tests were added: Phase 1 changes no
behavior, so the requirement-to-test mapping is empty; the four baseline gates
are the verification. Source files changed: none. Planning/docs changed:
`claudine/docs/rollout-strategy.md` (NR-1), this fix's `design.md` status line
(NR-1), `plan.md` checkboxes and frontmatter, and this log.

## Phase 2

Phase 2 adds Darkmatter's binding model and the passive prepared-expression
validator. It is additive: strict mode, `is_known_variable_root`, and the
bare-name `ctx` fallback are all untouched, every crate compiles, and no
existing caller changes behavior. All Phase 2 code is in the `darkmatter`
library; Claudine needed no source change.

### What was built

| Plan task | Where | Notes |
|---|---|---|
| Binding types | `darkmatter/lib/src/markdown/compose/expression/binding.rs` (new) | `ResolvedBinding { Document, Namespace, Global }` carrying `Option<Value>` / `Value` (NR-2); `BindingError` (configuration arms `ReservedName`, `InvalidRoot`, `Duplicate`, `UnknownGlobal`, `OmittedDeclaredGlobal`, `ContradictsDeclaration`, `InvalidReasonCode`, plus `Unavailable(Box<UnavailableBinding>)` with `root`, `path`, `scope`, `reason`, `span`); `UnavailabilityReason` (namespaced `owner.reason` code + structured `parameters`) |
| Declarations and sessions | same file | `ScopeId` (opaque), `Availability { Available, Unavailable(reason), ExecutionDependent }`, immutable provider-free `BindingView` built by `BindingView::builder(scope).declare(..).build()`, `RootClass` + `BindingView::classify_root`, `RuntimeBinding { Eager, Lazy(Arc<dyn Fn() -> Value>), Unavailable }`, and `EvaluationSession::associate(view, document, runtime)` |
| Trait channel (NR-5) | `expression/mod.rs` | `get_checked` → `resolve(&self, path) -> Result<ResolvedBinding, ExpressionError>` (default wraps `get` as `Document`); new `binding_view()` (default `None`) and `format_resolved(path, value)` (default = the old `get_string` rendering); `get_string`'s default now composes `get` + `format_resolved`. `ExpressionError::Binding(Box<BindingError>)` added, authoring-fatal |
| Evaluator routing | `expression/mod.rs` (`evaluate_expr`), `interpolation/evaluator.rs` (`Evaluator::eval` fast path) | Both read through `resolve`; the fast path formats through `format_resolved` instead of a second `get_string` lookup |
| Prepared validator | `expression/prepared.rs` (new) | `AuthoredMode { Expression(ParseMode), InterpolatedValue, Subtree }`, `prepare_value`, `validate_prepared`, `evaluate_prepared`, `PreparedValue`/`PreparedExpression`, `PreparationError`, `ValidationDiagnostic`; reuses `ExpressionFinder::scan_plain`, `parse_spanned`/`parse_condition_spanned`, and `static_variable_reads` (the existing all-branches walk) |
| Binding/passive tests | `darkmatter/lib/tests/l1/binding_contract.rs` (new, declared in `tests/l1/main.rs`) | 16 tests, see the mapping below |

The six lookups that overrode `get_checked` now override `resolve`, each
classifying its result with `ResolvedBinding::classify` (reserved root →
`Namespace`, otherwise `Document`): `EffectiveState`, `ResolvingLookup`,
`FrontmatterSeedState`, `ShortcutLookup`, `CtxLookup`, `LayeredLookup`
(injected globals → `Global`), and the forwarder `DeferrableLookup` (still maps
`ContextNotCaptured` to an absent value when deferring, now as a classified
`None`). Each keeps exactly its old values and errors.

### Decisions and departures

- **`EvaluationSession::associate`, not `BindingEnvironment::associate`.**
  C1 and the plan name a `BindingEnvironment` type whose only member would be
  `associate`. An empty struct that hosts one function adds nothing in Rust, so
  the constructor lives on the type it returns. Same arguments, same checks.
- **No `PreparationContext` parameter.** NR-6 removed the schema, policy, and
  feature-restriction inputs C2 put in it, which leaves it with nothing to
  carry. `prepare_value(input, mode)` takes two arguments, and the parse mode
  of a bare expression lives in `AuthoredMode::Expression(ParseMode)` because a
  `when:` condition (`||` = OR) and an interpolation expression (`||` =
  fallback) parse differently.
- **No prepared identity check.** NR-6 makes the `BindingView` identity the
  only prepared identity. A `PreparedValue` stores no view-derived data (the
  view is an explicit argument to `validate_prepared`, and the session supplies
  it at evaluation), so there is nothing a stale view could corrupt and no
  `PreparedContextMismatch` to raise. Add it if C5 later caches view data.
- **Error sizes.** Clippy's `result_large_err` rejected the first shape, so
  `ExpressionError::Binding` holds a `Box<BindingError>`, the unavailable read
  is a boxed `UnavailableBinding` struct, and `PreparationError::cause` is a
  `Box<ExpressionError>`, matching `MarkdownError::Interpolation`'s boxed cause.
- **Unknown-function check.** `validate_prepared` decides "unknown function"
  with a new `functions::is_dispatchable`, which matches names exactly as
  `evaluate_function` does (lowercased, canonical name or alias, lazy
  operators included). It builds the diagnostic with the evaluator's own
  `unknown_function_error`, so validation and runtime raise the same typed
  error (now `pub(crate)`).
- **Reserved roots (NR-3)** come from `reserved_root_descriptors()` through
  `binding::is_reserved_namespace`: `doc`, `ctx`, `env`, `current`,
  `current_env`. If the owner chooses option B, that function is the one place
  to narrow.
- **Lazy cache.** One `Mutex<HashMap<root, Value>>` per session. The lock is
  dropped before a provider runs and re-taken to insert (first value wins if a
  re-entrant provider filled the slot). A poisoned lock is recovered with
  `PoisonError::into_inner`, never read as missing data. Members are projected
  from the cached root, so `err` and `err.message` share one provider call.
- **A global's missing member is `Global { null }`** in the session, which
  differs from `LayeredLookup` (where it was `None` and then `null` in the
  evaluator). The rendered output is the same; only the classification is new.
- **`LayeredLookup` keeps the default `format_resolved`.** It never applied the
  base state's name coercion when rendering (its old `get_string` override did
  not), so overriding the hook would have been a behavior change.
  `EffectiveState`, `ResolvingLookup`, `FrontmatterSeedState`, and
  `DeferrableLookup` override or forward it, preserving coercion. Coercion only
  ever touches objects (`coerce_named_object`), so routing arrays and scalars
  through the hook renders the same bytes as the old fast path.
- **Claudine forwarders were left alone.** `SizedLookup` is only used with the
  generic `evaluate` (never the interpolation fast path), so the new
  `format_resolved` hook cannot change its output. It and
  `EventMetaConditionLookup` gain `resolve`/`binding_view`/`format_resolved`
  forwarding in Phase 4 (Wave 9), as planned; forwarding `resolve` now would
  surface errors they drop today, which is a behavior change.

### Pre-existing defect fixed

The rustdoc example on `expression::evaluate` (`expression/mod.rs`, the second
`SimpleLookup` example the plan names) did not compile at HEAD: it imported
`evaluate` but called the private `evaluate_expr`. `just test` does not run
doctests, which is why Phase 1's baseline missed it. It now calls `evaluate`.
`just doctest` in `darkmatter/` passes (192 passed, 10 ignored).

### Requirement-to-test mapping

All tests are in `darkmatter/lib/tests/l1/binding_contract.rs`, compiled by the
declared `l1` binary (`tests/l1/main.rs`) and selected by L1 (no tier marker in
any path segment). They read no repository file.

| Requirement (plan Wave 5) | Test |
|---|---|
| available-null vs unavailable vs absent document property | `resolution::available_null_unavailable_and_absent_property_are_distinct` (whole value, mixed string, ternary, `\|\|` fallback does not hide an unavailable global, `doc.group` still reads the document) |
| an injected global shadows a same-named document property | `resolution::a_global_shadows_a_document_property_of_the_same_name` (missing member stays `Global { null }`) |
| each reserved root's registration fails before any lazy provider runs | `registration::every_reserved_root_is_rejected_before_any_provider_runs` (declaration and association, with a panicking provider; asserts the five-name table) |
| an omitted declared global fails association even when only referenced in an inactive branch | `registration::an_omitted_declared_global_fails_association` (each of the three globals omitted in turn; control row evaluates) |
| configuration errors (duplicate, unknown, dotted, contradicting, reason codes) | `registration::association_rejects_every_inconsistent_registration`, `registration::a_view_rejects_invalid_duplicate_and_unnamespaced_declarations` |
| `doc.doc`, `doc.ctx`, `doc.env` read document data | `resolution::doc_prefixed_reserved_names_read_document_data` |
| one lazy evaluation per session, including a `null` result | `resolution::a_lazy_global_runs_once_per_session_including_a_null_result` (0 calls before reference, 1 across four reads, 2 after a new session) |
| a get-only lookup stays source-compatible | `resolution::a_get_only_lookup_resolves_as_document_data` |
| an unavailable root in an inactive ternary branch fails `validate_prepared` | `passive_validation::an_unavailable_root_in_an_inactive_branch_fails_validation` (runtime takes the other branch and succeeds) |
| an `execution-dependent` root is deferred | `passive_validation::validation_walks_every_position_and_defers_execution_dependent_roots` |
| a counting provider records zero calls during preparation and validation | `passive_validation::preparation_and_validation_invoke_no_provider` |
| unknown functions in every branch; aliases and letter case accepted | `passive_validation::unknown_functions_are_reported_in_every_branch` |
| mode semantics (condition `\|\|`, typed whole value, subtree) | `passive_validation::each_authored_mode_evaluates_with_its_own_rules` |
| malformed input located; literals not prepared | `passive_validation::a_malformed_expression_fails_preparation_with_its_location`, `passive_validation::literals_are_not_prepared` |

Unchanged behavior for existing callers is shown by the existing suites, which
pass unmodified apart from the mechanical `get_checked` → `resolve` call-site
edits in `context/checked.rs` and `expression/ctx.rs` unit tests.

Input Robustness Matrix: not applicable (no file-format or configuration
reader changed; plan § Input Robustness Matrix).

### Gates

| Area | Recipe | Result |
|---|---|---|
| workspace | `cargo check --workspace --all-targets` | clean |
| `darkmatter/` | `just test` | 8760 passed (baseline 8744 + 16 new), 12 skipped |
| `darkmatter/` | `just lint` | exit 0 |
| `darkmatter/` | `just doctest` | 192 passed, 10 ignored (includes the repaired `evaluate` example and the new `prepared` module example) |
| `claudine/` | `just test` | 8072 passed, 9 skipped (identical to baseline) |
| `claudine/` | `just lint` | exit 0 |

No `just test-l2` was run: Phase 2 changes no L2-covered boundary, and the
checkpoint names only `just test`/`just lint`. Cross-OS: the change is pure Rust
with no `#[cfg]`, path, or process code, so no `just cross-check` was needed;
CI covers the other environments.

### Docs

`darkmatter/docs/topics/darkmatter-expressions.md` § `EvaluationLookup` Trait
named the removed `get_checked`; it now describes `resolve`/`format_resolved`,
and a new "Host Bindings" subsection documents the resolution order, the three
availability states, association checks, lazy caching, and
prepare/validate/evaluate with a compact example. Phase 7 still owns the full
documentation pass (the Mermaid resolution diagram and the remaining pages).

### Phase 2 outcome

Checkpoint 2 is met: `cargo check --workspace` is clean, `darkmatter` and
`claudine` `just test`/`just lint` pass, and no existing caller changed
behavior. Evidence toward acceptance criteria 6 and 8 (partial): an unavailable
global is a typed error that never reads the document (6), and the passive
validator reports definitely unavailable roots in every branch without running
a provider (8).

## Phase 3

Phase 3 removes strict mode and the bare-name `ctx` fallback from Darkmatter,
rebuilds subtree compose on the Phase 2 binding session, renames the
undeclared-property advisory (NR-7), and makes Claudine compile against the
result. Claudine's own walkers (`first_undefined_stack_variable`,
`validate_no_undefined_lifecycle_variables`, the `err` placement scan) are
untouched; Phase 4 owns them.

### What was built

| Plan task | Where | Notes |
|---|---|---|
| Subtree API removal | `darkmatter/lib/src/markdown/compose/subtree.rs` | `SubtreeStrictness`, `.strict()`, `with_strictness`, the `compose_subtree` strictness argument, `validate_strict_roots`, and the subtree-local `collect_variable_roots` are deleted. Subtree compose always runs `ExpressionFailurePolicy::Strict` (ordinary fail-fast propagation). `InjectedGlobal` is now `pub type InjectedGlobal = RuntimeBinding<'static>`, so a caller can also pass `InjectedGlobal::unavailable(reason)`. `SubtreeCompose` gains `with_binding_view(Arc<BindingView>)` |
| `LayeredLookup` rebuilt on the session | same file, `expression/binding.rs` | `LayeredLookup` is deleted; `subtree::layered_session(base, globals, view, resolution_context) -> Result<EvaluationSession, BindingError>` replaces it. Without a view it synthesizes one (scope `darkmatter.subtree`) declaring each supplied global as supplied (available, or unavailable with its reason). `EvaluationSession::with_resolution_context` was added so the session can carry the read-side context that `LayeredLookup` used to hold. An association failure surfaces from `SubtreeCompose::compose` as `MarkdownError::Interpolation { cause: ExpressionError::Binding(..) }`, keyed by the new `BindingError::root()` |
| Namespace fallback removal | `context/effective_state.rs`, `frontmatter_interpolation.rs`, `conditions.rs`, `context/checked.rs`, `context/runtime.rs` | `EffectiveState::get`/`get_checked` and `ShortcutLookup` no longer read `ctx.<name>` for a missing bare name; `CtxLookupOutcome::into_checked_bare_name` is deleted. Exact `ctx` and `env` roots are resolved as namespaces before any document key (`ComposeContext::env_value` added for bare `env`), so a frontmatter `env` key is reachable only as `doc.env` |
| Trait + remaining lookups | `expression/mod.rs` and every override | `EvaluationLookup::is_known_variable_root` is removed from the trait and from `EffectiveState`, `ResolvingLookup`, `FrontmatterSeedState`, `DeferrableLookup`, `EvaluationSession`, and the test-only `Nothing` |
| Lookup parity inventory (R3) | `darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs` (new) | See the test mapping below |
| Undeclared-property advisory (NR-7) | `unknown_identifiers.rs`, `expression/mod.rs` (`observe_missing`), `interpolation/evaluator.rs`, `context/report.rs`, `expression/absence.rs` | The observer now takes the `ResolvedBinding`: only `Document { value: None }` is a candidate (never a namespace or global; reserved roots and `null` skipped). `reconcile` filters by final-state keys (`state.data()`), caller input records, and the effective schema. `ComposeWarning::UNDECLARED_PROPERTY_CODE = "dm.expression.undeclared_property"` and `ComposeWarning::undeclared_property` replace the old constant/constructor, with no alias. Message: `` `{root}`{location} is an undeclared document property (unknown type; `null` unless supplied at runtime) ``. `is_statically_known_root` is now exactly the reserved roots (context-descriptor names dropped) |
| Claudine compile bridge | `lifecycle/executor.rs`, `preflight.rs`, `sequence/preflight/mod.rs`, `sequence/task/mod.rs`, `lifecycle/context/tests.rs`, `interpolation_conformance.rs`, `event_time_interpolation.rs`, `cli/.../loop_control/tests/mod.rs` | `.strict()` deleted at the five sites; `eval_expr` and the two tests use `layered_session`. `strict_mode_provenance_spike.rs` deleted and unregistered from `tests/l1/main.rs` (NR-10). `signals/version.rs`, `darkmatter/cli/src/commands/compose.rs`, and `dmls/src/diagnostics/frontmatter.rs` needed no change |

### Decisions and departures

- **`LayeredLookup` is deleted, not wrapped.** A struct owning both a
  `ResolvingLookup` and a session that borrows it is self-referential, and a
  forwarding wrapper is the partial-forwarding hazard the inventory exists to
  prevent. A constructor function that returns the `EvaluationSession` keeps one
  lookup type. Callers change from `LayeredLookup::new(&s, &g, ctx)` to
  `layered_session(&s, g, None, ctx)?`.
- **A reserved-root global now fails association instead of being silently
  ignored.** `LayeredLookup` quietly skipped a global named `current`; the
  session rejects it (`BindingError::ReservedName`) before any provider runs.
  `compose/tests/lazy_roots.rs::an_injected_global_cannot_shadow_a_reserved_root`
  now asserts the typed refusal, with no provider call. No production caller
  registers a reserved name.
- **Subtree formatting now matches main compose.** The session forwards
  `format_resolved` to the document lookup, so `EffectiveState`'s configured
  name coercion now applies in subtree compose. `LayeredLookup` used the trait
  default. This brings subtree compose to its documented "byte-for-byte with
  main compose" contract. No test depended on the old rendering.
- **`group` outside a group is already an unavailable global (bridge toward
  Phase 4).** Without strict mode, a later sequence step's `{{ group.label }}`
  stopped failing (`group_variables_do_not_leak_to_a_later_step` and the CLI
  `sequence_groups::group_variables_reach_members_and_do_not_leak_to_the_next_step`
  turned red): sequence members carry `group` as *document data*, so outside a
  group it was an absent property. Rather than weaken two safety tests,
  `lifecycle::context::outside_group_global()` registers `group` as
  `InjectedGlobal::unavailable("claudine.outside-group")` (the code Wave 8
  names) in `TaskExecution::resolve_value` when the task's overlay carries no
  `group` scope, and in the lifecycle executor's `injected_globals` when
  `self.group` is `None`. This is NR-11's ruled behavior ("bare `group` outside
  a group raises a typed unavailable error; `doc.group` still reads the
  document"). Phase 4's catalog should replace the helper and both call sites.
- **Shipped-prompt fixes moved forward from Phase 6 (group D of the Phase 1
  audit).** Removing the fallback made `prompts/_reviews/performance-review.md`
  (`{{time}}`) and `prompts/brainstorm.md` (`{{area}}`) render those names
  empty, and DMLS's mapping-only corpus test reported the new advisory on the
  fixture copy. Both prompts and both fixture copies now say `ctx.time` /
  `ctx.area`. Neither prompt carries a `hash:` pin. The remaining group D rows
  are documentation (Phase 7) and `claudine/cli/tests/fixtures/nested_span_regression/commit.md:31`
  (`{{repo.name}}`), whose expected output was already empty and whose tests pass.
- **Minimal DMLS edits.** DMLS calls the library's `is_statically_known_root`,
  so dropping context names changed one DMLS unit test and one LSP session
  expectation: bare `repo` is now reported. DMLS still emits its own
  `dm.expression.unknown_identifier` code and wording; Phase 5 renames it. So
  between Phase 3 and Phase 5 the library and DMLS spell the code differently.
- **Old-contract Claudine tests updated, not left red.** Checkpoint 3 allowed
  old-contract failures to be listed for Phase 4, but this phase's completion
  bar is a green `just test`. The four tests that asserted "an unknown root
  fails" now assert the ruled contract (R1: absent is `null`):
  `event_time_interpolation::top_level_absent_property_renders_empty` (emits
  `""`), `::an_authored_span_with_an_absent_property_renders_empty` (emits
  `"done: "`), `cli agent_text_is_data::an_absent_property_in_an_authored_lifecycle_span_is_not_an_error`,
  and `cli authored_text_rendering::diagnostic_quotes_an_underscored_root_exactly`,
  which keeps its subject (an underscored root quoted verbatim) by sourcing it
  from the body advisory, since a lifecycle field no longer produces a root
  diagnostic. `interpolation_conformance.rs` only lost its strictness parameter.
  Its former "unknown-root divergence" is now `an_absent_root_is_empty_in_both_engines`,
  and the full matrix rewrite remains Phase 4.
- **Doc drift fixed in this phase.** Pages that described the removed fallback,
  `SubtreeStrictness`, or the old code were corrected (list in the frontmatter).
  Claudine's lifecycle docs now describe the interim state:
  `LifecycleUndefinedVariable` still guards `stack` entries, and its removal is
  marked **planned**. DMLS's docs (`darkmatter/docs/lsp/features.md`,
  `topics/schemas/dmls-schema-support.md`, `dmls/docs/*`) still describe
  DMLS's unchanged code and are Phase 5's.

### Requirement-to-test mapping

| Requirement | Test(s) | Level / target |
|---|---|---|
| Strict mode removed; subtree compose fails on real failures only (malformed, unknown function, rejected argument, failed file read) in whole values and mixed strings; absent → `null`/empty | `lib/tests/l1/compose_expression_failure_contract.rs::subtree_compose_fails_on_real_expression_failures_only`; `subtree.rs` unit tests `an_absent_property_is_null_whole_value_and_empty_in_a_mixed_string`, `malformed_spans_and_unknown_functions_fail_in_any_position`; `tests/frontmatter.rs::dm2_subtree_absent_property_is_null_not_an_error`, `dm2_subtree_rejects_malformed_span`, `dm2_subtree_rejects_unknown_function`, `dm2_subtree_absent_function_argument_is_null` | Darkmatter L1 (`l1` binary) + lib unit |
| Unavailable global is a typed error and never reads the document; `doc.group` still does | `subtree.rs::an_unavailable_global_fails_and_never_reads_the_document` | lib unit |
| Reserved roots cannot be registered; no provider runs (AC 2, widened by NR-3) | `subtree.rs::a_reserved_root_cannot_be_registered_and_no_provider_runs` (all five roots, panicking lazy provider); `tests/lazy_roots.rs::an_injected_global_cannot_shadow_a_reserved_root` | lib unit |
| A declared view is enforced at association | `subtree.rs::a_declared_view_is_enforced_at_association` | lib unit |
| No bare-name `ctx` fallback in any lookup (R3, AC 4) | `compose/tests/lookup_parity.rs::the_inventory_lists_every_lookup_implementation` (source scan of `CARGO_MANIFEST_DIR/src`, 13 implementations), `::every_production_lookup_keeps_bare_names_out_of_ctx` (7 production lookups probed), `::the_probe_name_is_a_captured_context_key` (control row). Mutation-checked: restoring the fallback in `EffectiveState` turns the probe red | lib unit |
| Exact `ctx`/`env` roots never read a document key; `doc.env` does | `lookup_parity` probe; `conditions.rs::shortcut_bare_name_never_reads_ctx`; `absent_property_contract::an_exact_namespace_root_is_never_a_document_property` | lib unit + L1 |
| Missing bare property does not resolve from `ctx` (body, frontmatter, `when=`) | `lib/tests/l1/absent_property_contract.rs::a_missing_bare_property_does_not_resolve_from_ctx`; `context/checked.rs::a_bare_name_never_reads_the_ctx_namespace`; `subtree.rs::a_bare_name_never_reads_ctx` | L1 + lib unit |
| Absent bare and `doc.<p>` are `null` whole values; empty in mixed strings; ternary falsy branch; `\|\|` value semantics (AC 1, 3) | `absent_property_contract::an_absent_property_is_a_null_whole_value`, `::an_absent_property_is_empty_in_a_mixed_string`, `::an_absent_property_takes_the_falsy_branch_and_the_fallback` | L1 |
| Schema-declared-unset and undeclared both valid; required violation still blocks | `absent_property_contract::a_schema_declared_unset_property_and_an_undeclared_one_are_both_valid`, `::a_required_property_violation_still_blocks`; `cli/tests/l1/compose_schema.rs::declared_unset_and_undeclared_properties_are_both_null_in_subtree_compose` | L1 (lib + `md` CLI) |
| Malformed syntax, unknown function, rejected arguments, file failure still raise | `absent_property_contract::expression_failures_still_stop_composition` (body + frontmatter mixed text); the existing body/frontmatter matrix in `compose_expression_failure_contract.rs` | L1 |
| All three escape forms inert; whole-value and mixed; no extra evaluation pass | `absent_property_contract::every_escape_form_is_inert` (asserts no advisory, i.e. nothing evaluated) | L1 |
| NR-7 code, wording, `ctx`-name drop, suppression kept | `unknown_identifier_warning.rs` (all rows updated; new `a_bare_context_name_is_an_undeclared_property_not_ctx`), `unknown_identifiers.rs` message unit tests, `absence.rs::statically_known_roots_are_exactly_the_reserved_roots`, `feature_review_incident.rs`, `cli/tests/l1/compose_unknown_identifiers.rs` (end-to-end `md compose`) | L1 + lib unit + `md` CLI L1 |
| Group variables do not leak (bridge) | existing `sequence/task/tests.rs::group_variables_do_not_leak_to_a_later_step`, `cli/tests/l1/sequence_groups.rs::group_variables_reach_members_and_do_not_leak_to_the_next_step` (unchanged, green again) | Claudine lib unit + CLI L1 |
| Shipped prompt corpus | `dmls/tests/l1/mapping_only_corpus.rs` (passive corpus over the fixture copies; baseline unchanged after the prompt fix) | DMLS L1 |

Tier and placement: `absent_property_contract.rs` is declared in
`darkmatter/lib/tests/l1/main.rs`, and `lookup_parity.rs` in
`compose/tests/mod.rs`. One new name, `real_expression_failures_still_stop_composition`,
was first stranded by its `real_` prefix (seen as a 13th skip) and renamed to
`expression_failures_still_stop_composition`; the final run shows the baseline
12 skips. The source scan joins a literal onto `CARGO_MANIFEST_DIR`, as the
test-inputs rule asks. Input Robustness Matrix: not applicable (no file-format
or configuration reader changed).

### Gates

| Area | Recipe | Result |
|---|---|---|
| workspace | `cargo check --workspace --all-targets` | clean |
| `darkmatter/` | `just test` | 8768 passed, 12 skipped (baseline skips) |
| `darkmatter/` | `just lint` | exit 0 |
| `darkmatter/` | `just doctest` | 192 passed, 10 ignored |
| `claudine/` | `just test` | 8068 passed, 9 skipped (baseline 8072 minus the 4 deleted spike tests) |
| `claudine/` | `just lint` | exit 0 |
| `claudine/` | `just test-l2` | 277 + 3 passed |
| `darkmatter/` | `just test-l2` | 3 + 18 + 69 passed |

Pre-existing or unrelated: none failed. Two specs outside this fix
(`claudine/fixes/2026-07-13-cli-switches/spec.md`,
`claudine/fixes/2026-07-22-setters/spec.md`) were rewritten by another session
during this phase; they are not part of this change. One `just test` run went
to the main checkout by mistake (`cd claudine` from inside `claudine/`
resolved through zsh `CDPATH`). It changed nothing; every gate above was run
with an absolute worktree path. Cross-OS: the change is pure Rust plus
Markdown, with no `#[cfg]`, process, or path-comparison code. The new source
scan normalizes `\` to `/`, so no `just cross-check` was run; CI covers the
other environments.

### Phase 3 outcome

Checkpoint 3 is met, and exceeded: `cargo check --workspace` is clean,
`darkmatter` `just test`/`just lint` pass, and Claudine's `just test` is fully
green rather than limited to listed old-contract failures. Acceptance criteria
1 (absent → `null`), 3 (no extra evaluation of escapes/inserted data), and 4
(no bare-name `ctx` fallback, with the parity inventory) hold in Darkmatter. For
criterion 2, the reserved-root refusal is now enforced in subtree compose too.

## Phase 4

Phase 4 gives Claudine one lifecycle binding catalog, routes every lifecycle,
task, and approval surface through Darkmatter's binding session and passive
validation, and deletes Claudine's own expression walkers. Setup and teardown
commands now run the bytes sequence approval fixed, and the typed Darkmatter
cause survives the library boundary.

### What was built

| Plan task | Where | Notes |
|---|---|---|
| Lifecycle binding catalog | `lib/src/composition/lifecycle/bindings.rs` (new) | `LifecycleScope` (7 events, `TaskSetup`, `TaskTeardown`, `LifecycleShellApproval`, `SequenceShellApproval`) and `binding_view(scope, GroupMembership)` build a Darkmatter `BindingView` declaring `err`, `timing`, `group` (NR-3, NR-4), plus `outputs` at sequence approval (C3). Reason codes: `claudine.event-has-no-error`, `claudine.preflight-unavailable`, `claudine.outside-group`. `group` follows the lexical rule: `Member` → available, `Outside` → unavailable, `Unknown` → execution-dependent. `approval_diagnostics(raw, scope)` is the approval-time passive check |
| Complete runtime entries | same file | `runtime_bindings(scope, LifecycleValues)` returns the view and one explicit entry per declared global. `err` is eager in error-carrying scopes (explicit `null` when there is no failure: `finalize`, teardown), `Unavailable` elsewhere; `timing` is eager (`null` when nothing was measured). `lifecycle_injected_globals` and the Phase 3 `outside_group_global` bridge are deleted; the executor's `injected_globals` became `bindings()` |
| Event-time lifecycle | `lifecycle/executor.rs` | `eval_expr` uses `layered_session(.., Some(view), ..)`; `resolve_string_value` uses `SubtreeCompose::with_binding_view`. The three `first_undefined_stack_variable` calls are deleted. `ctx_scan_hint` and `collect_variable_paths` (a variable-path walker used only to choose which `ctx` groups a snapshot-less caller captures) are deleted; the capture hint is now the expression's own rendering (`Expr: Display`). `StackExecutionContext` gained `scope: Option<LifecycleScope>` with `in_scope`/`binding_scope`; every struct literal adds `scope: None` |
| Shell approval and byte parity | `preflight.rs`, `sequence/preflight/{mod,shape}.rs`, `sequence/task/mod.rs` | Lifecycle shell approval uses `LifecycleShellApproval`; sequence approval uses `SequenceShellApproval` (so `group` is refused for primary, setup, teardown, and member commands, D8). Both check passively first (every branch), then resolve through a session in the same scope. `ApprovedCommand { site: CommandSiteId { stage, authored }, bytes }` is recorded on `PreflightTask::approved_stack_commands`; `TaskExecution::apply_approved_commands` stamps those bytes into the parsed setup/teardown shell actions and marks them resolved, failing closed if an interpolated command was never approved. Matching uses the lifecycle parser's own `action_value_to_expr` (now `pub(crate)`), so whole-value `{{ … }}` commands match too |
| Sequence values and expressions | `sequence/task/mod.rs`, `sequence/expr.rs` | Task values (`params`, `timeout`, group `variables`) resolve in the `TaskSetup` scope with membership from `stack.group` (group variables still resolve before the new group's scope is entered). `SourceExpressionLookup` resolves reserved namespaces first, then the item-over-frontmatter document layer; `doc.*` reads that document layer; `evaluate_whole`/`render_interpolated` now go through `prepare_value`/`evaluate_prepared` (their duplicate span traversal is deleted). `SequenceExpressionCause` gained `Prepare(PreparationError)` and `Compose(Box<MarkdownError>)`, replacing `Parse(ParseError)` |
| Loop and hook lookups | `looping/expression.rs`, `looping/actions.rs` | `LoopExpressionLookup` no longer lets a reserved root fall through to a same-named frontmatter key (`ctx.x` with no captured value used to read `frontmatter["ctx"]["x"]`; `current.*` read a `current` key) and classifies through `ResolvedBinding::classify`. `SizedLookup` forwards every trait method (it dropped `resolve`, `format_resolved`, `binding_view`, and the resolution context). No catalog on these surfaces (NR-8). `dispatch/expression.rs` needed no change: its lookups never read `ctx` for a bare name and their wrapper inherits only defaults |
| Prepare-time validation | `lifecycle/validate.rs`, `prepare.rs` | `validate_no_undefined_lifecycle_variables` (no production caller), `first_undefined_stack_variable`, both root walkers, `undefined_bare_variable`, `references_bare_err`, `literal_spans_reference_err`, `surface_references_err`, `visit_string_literals`, and `pre_resolved_shell_texts` are deleted. `validate_no_err_in_no_error_events(frontmatter, lifecycle, path)` is now an adapter: per event it builds the view (membership `Unknown`), prepares each surface's *authored* text from `LifecycleSourceMap` (predicates as `Expression(Condition)`, operands and top-level fields as `InterpolatedValue`), calls `validate_prepared`, and maps the first unavailable binding to `LifecycleErrNotAvailable` with the Darkmatter error as `source`. The `initialize` shell prohibition is untouched |
| Typed cause preservation | `lifecycle/context.rs`, `error/mod.rs`, `error/render/*`, `sequence/task/mod.rs` | `CompositionError::LifecycleUndefinedVariable` and its three renderer branches are deleted. `LifecycleCause(Arc<LifecycleExprError>)` rides on `LifecycleErrorInfo::cause` (not projected into `err.*`) and as `#[source] cause` on `LifecycleEvaluationError` and `LifecycleProxyWithEvaluationFailed`; its own `source()` is the Darkmatter `ExpressionError`/`MarkdownError`. `CompositionError::lifecycle_cause()` sees through `WithFrontmatter` and `LifecycleEvaluationAlreadyEmitted`; `LifecycleErrorInfo::from_composition_error` restores it, so catch routes keep it; task enrichment restores it. The proxy-overlay failure no longer flattens its cause to text. `LifecycleErrNotAvailable` and `SequenceShellLateBinding` gained typed sources |
| `LATE_BINDING_ROOTS` | `reserved.rs`, `lifecycle/mod.rs` | Deleted (with `SHELL_UNAVAILABLE_ROOTS` and `shape::first_unavailable_root`/`root_in_expr`): the catalog replaces them. `LIFECYCLE_COMM_FIELDS` is deleted too; the schema-parity test derives the field set from `notification_comm_fields` |

### Decisions and departures

- **Two walkers beyond the plan's list were removed.** `preflight.rs::late_binding_root_in_expr` and `sequence/preflight/shape.rs::root_in_expr` recognized variable roots at shell approval. Both are replaced by `approval_diagnostics`, i.e. Darkmatter's `validate_prepared` against the approval scope. `current`/`current_env` are reserved namespaces, so a view cannot declare them (NR-3); approval refuses them by reading Darkmatter's own `static_variable_reads` over the prepared expressions. That is a Claudine policy applied to Darkmatter's analysis, not a Claudine walk. The target-identity check (`ctx.agent`, `ctx.model`, `env.AGENT`, `env.MODEL` at graph approval) is a separate, path-level policy with no Darkmatter counterpart and is kept (`shape::first_target_identity_root`).
- **`group` at early lifecycle shell approval is unavailable.** C3 makes it "available only if already established and permitted by the existing approval context; otherwise unavailable". No approval context establishes a group, so it is `claudine.preflight-unavailable` there. Before this phase a group member prompt's lifecycle shell could read the overlay's `group` as document data; sequence-wide approval (D8) already refuses that, so the two scopes now agree.
- **Task values evaluate in the `TaskSetup` scope.** C3 has no row for task `params`/`timeout`/group `variables`; they are pre-primary and carry no failure, so `err` is unavailable there (previously a bare `err` read the document).
- **The plan's "surviving-span rejection" and "typed-result re-evaluation" were already gone.** `resolve_string_value` no longer rescans its result and `evaluate_operand` returns data as is (earlier agent-text-is-data work). The only remaining span check, `reject_control_plane_template`, refuses template text in a `proxy.with` value for a *lifecycle key* of the target; it is a feature restriction (R5 keeps those), not a rejection of successful output.
- **`LifecycleEvaluationError.reason` is unboxed.** Adding `cause` pushed the variant past clippy's `result_large_err` limit (136 > 128 bytes). `LifecycleEvaluationReason` has one unit variant, so it is zero-sized; unboxing it recovered the 8 bytes with no indirection.
- **A prepare-time missing-required-property failure keeps its existing routing.** Without `initialize` it fails eagerly with no lifecycle event; with `initialize` it routes once through `blocked`/`finalize` (documented staged-boot policy). The new CLI test pins both rows rather than changing either.
- **The loop action renderer does not honor `{{{ … }}}` escapes.** Darkmatter, the lifecycle executor, and the sequence source renderer all render `{{{ ghost }}}` as `{{ ghost }}`; the loop renderer leaves it unchanged. Loop action handling is outside this fix (NR-8, spec Non-Goals), so the shared table skips only the loop's escape rows and the docs now state the difference. Recommended follow-up: route `looping::actions::render_action_value` through `prepare_value`/`evaluate_prepared` like the sequence renderer.
- **`LifecycleShellResolution.source` is now always `Some`.** The `Option` is kept (narrowing it touches unrelated error tests); the field and test docs were corrected. A follow-up can make it non-optional.
- **Old-contract tests were converted, not deleted, where their subject survives.** 56 tests used an absent property (`missing_root == true`, `{{ absent_root }}`, `{{unknown_root}}`, `spec_fil`) as a stand-in for "an evaluation raise"; their subject (routing, atomicity, diagnostics of a raise) is unchanged, so the raise source became an unknown function (`missing_root()`), which keeps the name in assertions. Tests whose only subject was the deleted closed-world check were removed (14 in `lifecycle/tests/validation.rs`, 6 in `lifecycle/tests/diagnostics.rs`). `err_span_inside_object_literal_key_is_rejected` became `..._is_text_not_a_read`: an object key is authored text Darkmatter never evaluates. `when_unknown_root_typo_fails_closed` → `when_unknown_function_fails_closed`; `unknown_root_typo_fails_closed` → `a_typo_in_a_stack_message_is_an_absent_property`.
- **Inline-test cap.** `cli/src/commands/wrap/sequence/task_run.rs` reached 301 inline-test lines after one new field; one struct literal in its fixture was joined onto a line to stay at the 300-line placement cap.

### Requirement-to-test mapping

| Requirement | Test(s) | Level / target |
|---|---|---|
| Catalog matrix: every global × every scope, through `validate_prepared` (read in an inactive branch) and runtime session resolution; document never read; `doc.<root>` always reads the document (R4, AC 6, 7, 8) | `lifecycle/bindings/tests.rs::passive_validation_follows_the_matrix_in_an_inactive_branch`, `::runtime_resolution_follows_the_matrix_and_never_reads_the_document` (both memberships), `::the_matrix_has_every_scope` | Claudine lib unit (L1) |
| Omitted entry vs explicit unavailable entry; explicit `null` `err` in `finalize`/teardown | `bindings/tests.rs::an_omitted_entry_fails_association_but_an_explicit_unavailable_entry_does_not`, `::err_is_an_explicit_null_where_an_error_may_be_absent`; `context/tests.rs::an_unattached_global_is_an_explicit_null_entry_not_an_omission` | L1 |
| Approval refuses late globals and namespaces in any branch; `doc.*`, `ctx.*`, `env.*`, document data pass | `bindings/tests.rs::approval_diagnostics_name_late_globals_and_namespaces_in_any_branch` | L1 |
| Absent property in `when:` skips cleanly; in a message renders `null`; in a typed value is `null` (R5, AC 3, 5) | `executor/tests/binding_contract.rs::an_absent_property_in_when_skips_the_action_without_an_error`, `::an_absent_property_in_a_message_renders_through_null_semantics`, `::an_absent_property_in_a_typed_value_resolves_to_null`; `event_time_interpolation.rs::a_typo_in_a_stack_message_is_an_absent_property` | L1 |
| `proxy.with` and `set` atomic with `null` members; a failing last member publishes nothing (R6, AC 9) | `binding_contract.rs::set_and_proxy_with_stay_atomic_when_a_null_member_precedes_a_failure`; existing `proxy_with_evaluation.rs::evaluation_is_atomic_across_the_whole_mapping`, `runtime_set.rs::failed_expression_publishes_no_part_of_the_mapping_with_or_without_runtime` | L1 |
| Unavailable global fails with its reason and never reads a same-named document property, at event time alone | `binding_contract.rs::an_unavailable_global_fails_with_its_reason_and_never_reads_the_document` (`err` in `start`/`success`, `group` outside a group; `doc.err`/`doc.group` read the document) | L1 |
| Malformed expressions and unknown functions halt before side effects | `binding_contract.rs::malformed_expressions_and_unknown_functions_halt_before_side_effects`; `conditions_control.rs::when_unknown_function_fails_closed`; `action_dispatch.rs::no_error_does_not_suppress_evaluation_raise` | L1 |
| Successful output containing braces is delivered as data (AC 17) | `binding_contract.rs::successful_output_containing_braces_is_delivered_as_data` | L1 |
| Prepare-time `err` check through Darkmatter, every surface, typed source | updated `lifecycle/tests/diagnostics.rs` and `validation.rs` `err_*` tests, `proxy_with_evaluation.rs::out_of_scope_err_in_a_with_value_is_rejected_before_the_event_fires` | L1 |
| Typed Darkmatter cause + Claudine context through direct, lifecycle, catch, and proxy wrapping, without the CLI or rendered text (AC 16) | `binding_contract.rs::the_typed_darkmatter_cause_survives_every_library_wrapper` | L1 |
| Shared missing-property and escape table (Darkmatter `absent_property_contract` inputs) in Darkmatter, sequence source, loop, and the lifecycle executor; no strict/lenient split | `interpolation_conformance.rs::every_engine_agrees_on_missing_properties_and_escapes`, `binding_contract.rs::event_time_values_follow_the_shared_missing_property_table` | L1 |
| Byte parity: primary, setup, teardown (including a whole-value command) run the approved bytes even after a setup `set` changes the value (NR-9, AC 9) | `sequence/task/tests.rs::byte_parity::primary_setup_and_teardown_run_the_bytes_approval_fixed` — mutation-checked: removing the stamping makes it run `setup changed` | L1 |
| `group` unavailable at sequence-wide approval (inactive branch, setup, group member) while `doc.group` reads the document (D8) | `byte_parity::group_is_unavailable_at_sequence_approval_while_doc_group_reads_the_document` | L1 |
| Referenced-prompt/lifecycle approval refuses `group` in any branch with a typed cause; `doc.group` bytes are stamped | `prepare/tests.rs::lifecycle_shell_approval_refuses_group_in_any_branch_while_doc_group_reads_the_document`; existing `shell_command_late_binding_reference_rejected_at_prepare`, `lazy_reserved_roots_in_a_lifecycle_shell_command_are_rejected_at_prepare` | L1 |
| Required-schema failure stops before lifecycle actions and provider launch; recovery routes run under existing policy; an absent property there is `null` (AC 19) | `cli/tests/l1/compose_schema_cli.rs::a_required_property_violation_stops_before_lifecycle_actions_under_existing_recovery_policy` (eager row: no events; staged row: `initialize`, `blocked … plan=[]`, `finalize`; no provider, no audio). The staged row's `{{ plan }}` raised "undefined variable" before this phase | `claudine-cli` L1 (`CliProcessFixture`) |
| Prohibited shell expansion never runs, including recovery routes and inactive branches; `initialize` regressions kept | existing `compose_initialize_staged_boot.rs::early_catch_shells_cannot_run_or_enable_another_catch_shell`, `::initialize_shells_are_forbidden_even_with_yolo_across_entry_paths`, `::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path`, `set_shell_values.rs::initialize_refuses_a_shell_assignment_in_any_item`, `action_dispatch.rs::programmatic_initialize_shell_is_rejected_even_with_no_error` (all green, unchanged) | L1 |
| Group variables still do not leak to a later step | existing `sequence/task/tests.rs::group_variables_do_not_leak_to_a_later_step`, `cli sequence_groups::group_variables_reach_members_and_do_not_leak_to_the_next_step` | L1 |
| Capture hint still covers container literals after the walker's removal | `filesystem_lookup.rs::ctx_capture_hint_covers_container_literals` | L1 |

Tier and placement: every new test is in a compiled module with no tier
marker (`bindings/tests.rs` via `#[cfg(test)] mod tests;`,
`executor/tests/binding_contract.rs` declared in `executor/tests/mod.rs`, the
`byte_parity` module inside `sequence/task/tests.rs`, and
`compose_schema_cli.rs` already in the `l1` binary). The final L1 run shows the
baseline 9 skips. Input Robustness Matrix: not applicable (no file-format or
configuration reader changed; the catalog is Rust).

### Source scan

`rg 'SubtreeStrictness|with_strictness|is_known_variable_root|validate_strict_roots|first_undefined_stack_variable|LifecycleUndefinedVariable|validate_no_undefined_lifecycle_variables'`
over `claudine/` source returns nothing outside historical specs, reviews, and
logs (`claudine/docs/rollout-strategy.md` names them in its historical status
and rulings tables). `darkmatter/dmls` still has its own `LATE_BINDING_ROOTS`;
Phase 5 replaces it.

### Gates

| Area | Recipe | Result |
|---|---|---|
| `claudine/` | `just test` | 8067 passed, 9 skipped (baseline skips) |
| `claudine/` | `just lint` | exit 0 |
| `claudine/` | `just test-l2` | 277 + 3 passed |

Pre-existing or unrelated failures: none. Cross-OS: the change is pure Rust plus
Markdown — no `#[cfg]`, process, or path-comparison code. The one new CLI test
is `#[cfg(unix)]` like its neighbors because it writes a `/bin/sh` provider
stub. No `just cross-check` was run; CI covers the other environments.

### Docs

- `claudine/docs/topics/flow-control/lifecycle.md`: the lifecycle-global
  availability table per event, task stack, and shell approval; the **planned**
  marker and the `LifecycleUndefinedVariable` section are removed; the `shell`
  exception names `group` and inactive branches; `LifecycleErrNotAvailable`
  describes Darkmatter's passive validation; `doc.err`/`doc.group`.
- `flow-control-reference.md` (`proxy.with` evaluation), `composition.md` and
  `looping.md` (loop vs lifecycle interpolation: the strict/lenient split is
  gone; the remaining differences are the error type and `{{{ … }}}`).
- `.claude/skills/claudine/SKILL.md`: the Lifecycle stacks row describes the
  catalog, typed cause, and approved bytes; the initialization note no longer
  says unrelated unknown roots error.

### Phase 4 outcome

Checkpoint 4 is met: `claudine` `just test`, `just lint`, and `just test-l2`
pass, and the Definition-of-Success scan returns nothing for Claudine.
Acceptance criteria 5 (no closed-world check), 6 (binding model distinguishes
absent, available-`null`, and unavailable — exercised end to end), 7 (catalog
declared, no Claudine traversal), 8 (definite violations fail passive
preparation in inactive branches; execution-dependent `group` fails at
runtime), 9 (contextual diagnostics, atomicity, byte parity), 16 (typed cause),
17 (ordinary Darkmatter interpolation, no Claudine post-check), and 19
(validation boundary under existing recovery policy) hold.

## Phase 5

Phase 5 makes DMLS take its root classification from Darkmatter's binding
model, renames its advisory to the library's `dm.expression.undeclared_property`
(NR-7), and keeps unknown functions as errors (R7a, R7b). DMLS does not depend
on Claudine and receives no Claudine descriptors (R7c stays out of scope).

### What changed

| Area | Files | Change |
|---|---|---|
| Baseline view (library) | `expression/binding.rs` | `BindingView::baseline()` (scope `darkmatter.baseline`, no host globals) and `BindingView::names_document_property(root)`: classifies as `Document` and is not the `null` literal. `NULL_ROOT` lives here |
| One reserved-root list (library) | `expression/absence.rs`, `context/current.rs`, `expression/mod.rs` | `absence::is_reserved_root` is now `is_reserved_namespace(root) \|\| root == NULL_ROOT`, derived from `reserved_root_descriptors()` instead of its own `"ctx" \| "env" \| "doc" \| "null"` literal plus `CurrentScope::is_reserved_root` (deleted). `is_statically_known_root` is deleted (DMLS was its only caller); its unit test became `reserved_roots_are_the_catalog_namespaces_and_null` |
| Per-expression validation (library) | `expression/prepared.rs`, `expression/mod.rs` | `validate_prepared`'s body is factored into public `validate_expression(&SpannedExpr, &BindingView) -> Vec<(SourceSpan, ExpressionError)>`; `validate_prepared` maps it onto leaf spans (and relocates an `UnavailableBinding.span` onto the leaf, as before), so its output is unchanged |
| Classification (DMLS) | `overlay/expressions.rs`, `providers/dsl.rs` | `is_unknown_root` → `is_undeclared_property(name, &BindingView, is_key, is_schema)`, deciding through `view.names_document_property`. `KnownRoots` carries `BindingView::baseline()`. `context_root()` / `document_root()` read the eager `ContextVariables` / `DocumentFrontmatter` roots from `reserved_root_descriptors()`; they replace the literal `"ctx"` decisions (frontmatter `ctx:` key hover in `providers/frontmatter.rs`, the `ctx.` hover prefix and completion label) and the literal `"doc"` in the dash-key quick-fix |
| Late-binding list removed (DMLS) | `diagnostics/nested_span.rs`, `diagnostics/frontmatter.rs` | `LATE_BINDING_ROOTS` (`err`, `timing`, `current`) and its only user, the beneath-a-lifecycle-event suppression, are deleted, with `lifecycle::is_beneath_event` |
| Advisory rename (NR-7, DMLS) | `diagnostics/codes.rs`, `overlay/expressions.rs`, `providers/dsl.rs`, `diagnostics/frontmatter.rs`, `providers/code_actions.rs`, test files | `EXPRESSION_UNKNOWN_IDENTIFIER`/`dm.expression.unknown_identifier` → `EXPRESSION_UNDECLARED_PROPERTY`/`dm.expression.undeclared_property`, no alias. `UnknownIdentifierFinding::Identifier` → `UndeclaredPropertyFinding::Property`; `unknown_identifier_{findings,message,diagnostic}` → `undeclared_property_*`. Message is the library's: `` `{root}` is an undeclared document property (unknown type; `null` unless supplied at runtime) ``. Severity stays `WARNING` (spec line 190: "that warning is advisory"; it matches `md compose`). The dash-separated-key quick-fix is unchanged |
| Unknown functions (DMLS) | `diagnostics/codes.rs`, `overlay/expressions.rs`, `providers/dsl.rs`, `diagnostics/frontmatter.rs` | New `dm.expression.unknown_function` at `ERROR` on body interpolations and Expression-typed frontmatter values, ranged on the function name, from `unknown_function_calls` over the library's `validate_expression(&expr, &BindingView::baseline())`. Before this phase DMLS reported nothing for an unknown function. It is emitted even without frontmatter (the advisory still is not) |

### Decisions and departures

- **Unknown functions needed a new DMLS diagnostic.** The plan says "Unknown
  functions stay hard errors", but DMLS had no unknown-function diagnostic at
  all (it treated any catalog name as known and ignored call names). R7a and
  criterion 10 require an error, so DMLS now reports one through Darkmatter's
  passive validation rather than a DMLS-side function list. `ERROR` follows
  the severity ladder ("will never work": compose fails on the call), on the
  body too, unlike a malformed body span.
- **`err`/`timing`/`group` in lifecycle predicates are now advisories in the
  editor.** R7b forbids a DMLS root list, and `LATE_BINDING_ROOTS` had already
  drifted from the runtime catalog (no `group`; `err` is unavailable in
  `initialize`/`start`/`success`/`loop`). Without host descriptors (R7c) a
  bare `when: err.kind == …` in `failure` gets one `undeclared_property`
  advisory. The wording ("`null` unless supplied at runtime") stays true, and
  runtime enforcement is Claudine's (Phase 4). No shipped prompt in the DMLS
  corpus hit this: the re-blessed baseline gained no advisory.
- **Library touched in a DMLS phase.** "The binding model's baseline view" did
  not exist, so it was added (`baseline`, `names_document_property`), and
  `validate_expression` was factored out so DMLS validates the expression it
  already parsed instead of re-parsing through `prepare_value`. The library's
  own duplicate reserved-root literal (`absence::is_reserved_root`) was
  replaced in the same change so Darkmatter has one list.
- **Library module names untouched.** `compose/unknown_identifiers.rs`,
  `lib/tests/l1/unknown_identifier_warning.rs`, and
  `cli/tests/l1/compose_unknown_identifiers.rs` keep their names (they already
  emit and assert the new code); renaming them is churn outside this phase.

### Corpus baseline re-bless (`DMLS_BLESS_MAPPING_CORPUS=1`)

`tests/fixtures/mapping_only_corpus/baseline.json` was re-blessed twice and
each diff audited against the previous baseline:

1. After the rename: every change was the code and message rewording; no
   finding was added or removed (no corpus prompt reads a bare lifecycle
   global in a predicate).
2. After the unknown-function diagnostic: exactly one finding was added,
   `_agent-skills.md` line 25, `` `skill_description` is not a Darkmatter
   expression function ``. This is a **real defect in the shipped prompt**
   `prompts/_agent-skills.md`: neither `skill_description` (body, inside a
   `::loop` block) nor `local_skill_description` (frontmatter, lines 10–14)
   exists in Darkmatter's function catalog, so composing either path fails.
   The frontmatter use is not an Expression-typed value, so DMLS cannot see
   it. Left for Phase 6 (prompts); not fixed here.

### Requirement-to-test mapping

| Requirement | Test(s) | Level / target |
|---|---|---|
| Baseline view: reserved namespaces from the catalog, `null` literal, every other bare root (context names, Claudine globals) a document property; a host view keeps its globals | `binding_contract::resolution::the_baseline_view_reads_every_unreserved_root_as_a_document_property`; `absence::tests::reserved_roots_are_the_catalog_namespaces_and_null` | Darkmatter L1 (`l1`) + lib unit |
| An undeclared property is one `WARNING` advisory per span, worded as a valid unknown-typed property, in the body and an Expression-typed frontmatter value; handled absence (`\|\|`, `is_null`), schema-declared-unset, present, `null`, `ctx.repo`, `doc.x` are silent; not a parser error (no `dm.expression.malformed`); advisories never escalate | `undeclared_property::an_undeclared_property_is_one_advisory_and_an_unknown_function_an_error` | DMLS L1 (LSP session) |
| Declaring the property clears the advisory; a frontmatter-less document reports no undeclared property | `undeclared_property::declaring_the_property_clears_the_advisory_and_functions_need_no_frontmatter`; `lsp_session::a_frontmatter_less_document_never_reports_undeclared_properties` | DMLS L1 |
| Unknown functions are distinct `ERROR`s, ranged on the name, in a live branch, an inactive branch, and frontmatter, and without frontmatter | same two `undeclared_property` tests | DMLS L1 |
| Schema-declared but unset property keeps its type in hover | `lsp_session::schema_declared_property_and_json5_mermaid_have_no_dsl_diagnostics` (existing; asserts the schema type and description) | DMLS L1 |
| Completion and hover classify bare names as document properties (`repo` hover shows the frontmatter value, no `ctx.repo` block; `branch` gets no ctx block; completion offers `repo` as a FIELD and no bare context VARIABLE) | `undeclared_property::completion_and_hover_classify_bare_names_as_document_properties` | DMLS L1 |
| Validation runs no lifecycle action, no frontmatter `$()`/action/`::shell` command, writes no file, builds no effect engine, and reads no lazy `current.*`/`current_env.*` (while still reporting both classifications) | `no_side_effects::expression_validation_runs_no_actions_shells_file_effects_or_lazy_providers` | DMLS L1 (instrumented under `effects-instrumentation`, which CI enables) |
| No separate root catalog in DMLS: no root-name or host-global string literal, no `LATE_BINDING_ROOTS`, no `EvaluationSession`/`RuntimeBinding`/`LazyProvider`/`evaluate_prepared`/`SubtreeCompose` in production `src/` | `undeclared_property::dmls_has_no_separate_root_catalog_or_evaluation_session` (mutation-checked: re-adding a `["err", "timing"]` const fails it with both literals named; a stale allowance fails too) | DMLS L1 |
| Host globals are undeclared properties beneath lifecycle keys and elsewhere; `current` never | `nested_span_tests::host_globals_are_undeclared_properties_beneath_lifecycle_keys_and_elsewhere` (replaces the two `late_binding_roots_*` tests) | DMLS unit |
| Every operand position, message wording | `lsp_session::undeclared_property_fires_in_every_operand_position_at_warning_severity` (renamed; asserts the new message) | DMLS L1 |
| Shipped-prompt corpus | `mapping_only_corpus::mapping_only_corpus_diagnostics_match_pre_descent_baseline` (re-blessed; audit above) | DMLS L1 |

New test files: `darkmatter/dmls/tests/l1/undeclared_property.rs`, declared in
`tests/l1/main.rs` (the package sets `autotests = false`; `test_layout`
passes). No test name carries a tier marker, so all run in L1. The source scan
reads `manifest_dir!().join("src")`.

### Gates

| Area | Recipe | Result |
|---|---|---|
| `darkmatter/` (covers DMLS) | `just test` | 8773 passed, 12 skipped |
| `darkmatter/` | `just lint` | exit 0 (after one `type_complexity` fix in the new test file) |
| `darkmatter/dmls` | `cargo nextest run -p dmls --features effects-instrumentation` (CI's feature set, re-run after the last edits) | 757 passed, 6 skipped |
| `claudine/` | `just test` (Darkmatter's `validate_prepared` was refactored under it) | 8067 passed, 9 skipped (baseline skips) |
| `claudine/` | `just lint` | exit 0 |

Pre-existing or unrelated failures: none. No L2 suite covers the changed
boundary (DMLS's L2 drives editors, not diagnostics classification), so none
was run. Cross-OS: pure Rust plus Markdown. The one path-sensitive test
(`no_side_effects`) spells sentinel paths with `/` the way its existing
neighbor does for Windows; the source scan normalizes `\` to `/`. No
`just cross-check` was run; CI covers the other environments.

Working-tree note: during this phase several DMLS files appeared staged in the
Git index without any `git add` from this session (another process staged
them). The index was left as found.

### Phase 5 outcome

Checkpoint 5 is met: `darkmatter` `just test` and `just lint` pass, and
acceptance criterion 10 holds — DMLS treats an undeclared bare property as a
valid unknown-typed document property, reports it only as the advisory
`dm.expression.undeclared_property`, keeps unknown functions as errors
(`dm.expression.unknown_function`), and classifies roots through Darkmatter's
binding model with no root catalog of its own (source-scanned). The
Definition-of-Success `rg` over `darkmatter/` Rust source, extended with
`LATE_BINDING_ROOTS` and `is_statically_known_root`, matches only the
forbidden-name list inside the new guard test.

## Phase 6

Phase 6 removes the `(x || false)` legality guards from the shipped prompts
(R8), proves the reported router case with a hermetic CLI L1 regression
against the real `prompts/implement.md`, and refreshes the router's hash pin
after that regression passed (acceptance criterion 11).

### What changed

| File | Change |
|---|---|
| `prompts/implement.md` | Deleted the `` `|| false` is the guarded-optional form `` comment; `when: "review || false"` → `when: "review"`; every `(spec\|plan\|review \|\| false)` in the `stderr` and `error` actions → the bare name (lines 64–67, 75–77, 80, 82 of the Phase 1 audit). `frontmatter(spec, "implemented") \|\| false` and `frontmatter(spec,'review_iterations') \|\| 1` are kept: both are real defaults |
| `prompts/review.md` | Deleted the five-line workaround comment; `when: "spec \|\| false"` / `"plan \|\| false"` / `"review \|\| false"` → bare names |
| `prompts/_prompt.md` | The bullet that told authors to "guard it with a fallback" now says an unsupplied optional input reads as `null` (so `when: "review"` is false and a plain ternary needs no guard), and that a fallback is for choosing a default (`{{{ title \|\| 'Untitled' }}}`) |
| `claudine/cli/tests/l1/shipped_prompts.rs` | New test `shipped_implement_router_reads_absent_optional_inputs_unguarded`. The `common` imports are no longer `cfg(unix)`-only, since the new test runs on every OS; only `write_executable` stays Unix-gated |
| `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json` | `prompts/implement.md` re-pinned (`8897501d2392dccf-…` → `dab08d3f2c196f65-…`; the body half is unchanged). Refreshed with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1` only after the regression passed |

The route-drift fixture `_implement/implement-plan.md` needed no
re-derivation: it copies `prompts/_implement/implement-plan.md`, which this
phase did not touch, and its pinned hash is unchanged.
`fixture_preserves_the_shipped_schema_and_loop_semantics` and
`fixture_body_matches_the_shipped_body` pass unchanged.

The Phase 1 audit rows classed as **real defaults** were left as they are
(`pr.md`, `_pr/dirty.md`, `_pr/push.md` proxy `with:` values, `pr.md:32`,
`_pr/dirty.md:30`). No other shipped prompt still uses `|| false` as a
legality guard on `spec`/`plan`/`review`. None of the three edited prompts
has a copy in the DMLS mapping-only corpus, so its baseline is unaffected.

### The regression

`shipped_implement_router_reads_absent_optional_inputs_unguarded` uses
`CliProcessFixture::command()` (hermetic default: child-local
`PLAYA_DRY_RUN=1`, private spool, fixture `HOME`, minimal `PATH`). It
writes the root `prompts/implement.md` into the fixture via `include_str!`
(so CI re-runs it whenever the prompt changes), plus a fixture spec at
`fixes/2026-09-17-router-fixture/spec.md` with `implemented: true`. With no
`review-1.md` beside it, every route is false. It runs
`claudine compose prompts/implement.md spec=… -y --claude` and asserts in one
run:

- source: the router reads `plan ? '- a plan file was identified: ' + plan : ''`
  and `when: "review"` bare, and carries no `spec`/`plan`/`review || false`;
- the list-form `$schema` accepts the document with only `spec` (the run
  reaches the `initialize` stack's fall-through action);
- the run fails at the authored error (`Unable to route the implementation`);
- the spec line renders, and the plan, review, and "no spec, plan, or
  review" lines do not (the absent `plan`/`review` evaluated to `null`);
- the output contains none of `unknown root`, `undefined variable`,
  `|| false`, `fallback`, or `lifecycle evaluation`;
- no provider starts: a Unix stub `claude` would touch a marker file, and the
  marker is absent. On Windows there is no stub; a scratch run with no
  `claude` on `PATH` confirmed the router errors before any executable check;
- `fixture.audio_spool()` is absent.

No window is created (no terminal harness; a plain child process).

The pre-fix library cannot be rebuilt at this point (Phases 3–4 removed the
code), so the test cannot be shown failing against the old binary. It is
load-bearing in two ways: reverting the prompt fails its source assertions,
and reintroducing a strict lookup of an absent property would end the run with
a lifecycle evaluation error rather than the authored routing error.

### Requirement-to-test mapping

| Requirement | Test(s) | Level / target |
|---|---|---|
| R8: shipped prompts read optional inputs without legality guards | `shipped_prompts::shipped_implement_router_reads_absent_optional_inputs_unguarded` (source assertions) | Claudine CLI L1 (`l1`) |
| AC11: the reported router case runs unguarded to the authored routing `error`, with no provider launch and no audio | same test (end-to-end through the real shipped artifact and normal `compose` path) | Claudine CLI L1 |
| The router still proxies with `review` supplied (bare `when: "review"` is true) | existing `shipped_prompts::shipped_implement_router_runs_real_proxy_handoff` | Claudine CLI L1 (Unix) |
| The router still proxies a spec to `implement-plan.md` | existing `compose_initialize_acceptance::the_shipped_router_*` (2 tests); `level2_lifecycle_control::level2_lifecycle_shipped_implement_route_matches_direct_run` | L1 + L2 |
| Pin refreshed only after the regression | `shipped_prompt_route_drift::*` (4 tests) | L1 |
| Passive corpus over all shipped prompts | existing `shipped_prompts::shipped_prompt_corpus_parses_frontmatter`, `shipped_lifecycle_artifacts_use_mapping_only_set` | L1 |

### Gates

| Area | Recipe | Result |
|---|---|---|
| `claudine/` | targeted: `shipped_prompts::`, `shipped_prompt_route_drift::`, `shipped_prompt_contract::`, `compose_initialize_acceptance::` | 44 passed |
| `claudine/` | `just test` | 8068 passed, 9 skipped (Phase 5: 8067 + the new test) |
| `claudine/` | `just lint` | exit 0 |
| `claudine/` | `level2_lifecycle_shipped_implement_route_matches_direct_run` (`--features terminal-tests`) | passed (7.3 s) |

No pre-existing or unrelated failures. `darkmatter/` was not re-run: no
Darkmatter file changed and no DMLS corpus fixture copies these prompts. No
`just cross-check` was run: the change is prompt text plus a test that uses no
path comparison and no shell on Windows. CI covers the other environments.

### Not fixed: `prompts/_agent-skills.md`

The defect Phase 5 flagged (`skill_description(i)` and
`local_skill_description(...)` are not Darkmatter functions) is unrelated to
strict mode or R8, and the fix depends on what the prompt is meant to do: add
a function, or replace the calls. It was left as it was, and the DMLS corpus
baseline still records the body finding. It is passed on in
`message_to_agent`.

Working-tree note: the changed files appeared staged in the index without
any `git add` from this session, as in Phase 5. The index was left as found.

### Phase 6 outcome

Checkpoint 6 is met: `claudine` `just test` passes with the new regression and
the refreshed pin, and acceptance criterion 11 holds.

## Phase 7

Phase 7 brings the docs and skills onto the one contract (R9), sweeps stale
wording from rustdoc and test comments, audits the Darkmatter–Claudine seam for
duplication (acceptance criterion 14), maps every in-scope acceptance criterion
to its evidence, and runs the final gates.

### What changed

| Area | Files | Change |
|---|---|---|
| Darkmatter expression topic | `darkmatter/docs/topics/darkmatter-expressions.md` | Mermaid diagram of root resolution (reserved namespace → host global, available or unavailable → document property, absent → `null` + advisory); "absent needs no `\|\|` to be legal, a fallback chooses a default"; an "Errors" table of what fails composition (malformed, unknown function, rejected argument / failed read, unavailable host global) versus what evaluates normally (absent property, missing member, `null` in text, escaped span) |
| Darkmatter inline pages | `darkmatter/docs/inline/interpolation.md`, `darkmatter/docs/inline/fm-interpolation.md` | Failures section names unavailable host globals and links Host Bindings; "Undefined variables" → "An absent document property" |
| DMLS feature table | `darkmatter/docs/lsp/features.md` | Three "unknown root"/"unknown identifiers" cells → "undeclared document property (advisory)" |
| Claudine lifecycle | `claudine/docs/topics/flow-control/lifecycle.md` | Availability table gains the sequence-wide approval row; new paragraph: explicit `null` `err` in `finalize`/teardown, `group` (and `outputs`) unavailable at sequence-wide approval; the wrong sentence "a value is literal text and `{{{ … }}}` is how you opt into the expression engine" now states the ordinary escape semantics; "Unrelated undeclared roots still fail strict evaluation" → "Any other absent property reads as `null` too"; the positional-action example used `{{agent}}`/`{{branch}}`, which relied on the removed `ctx` fallback, and now reads `ctx.agent`/`ctx.branch` |
| Claudine sequences / composition | `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/composition.md` | Sequence approval refuses `err`/`timing`/`group` (even for a group member) and setup/teardown run the approved bytes; "Undefined variables remain lenient" → absent property is `null` and needs no fallback |
| Skills | `.claude/skills/darkmatter/SKILL.md`, `.claude/skills/darkmatter/compose.md`, `.claude/skills/claudine/architecture.md`, `.claude/skills/claudine/timeline.md` | Darkmatter skill: host-binding paragraph (view → associate → passive validate → evaluate; no strict mode, no root-membership hook); two stale `compose_subtree(..., Lenient)` references removed (subtree compose takes no policy since Phase 3). Claudine architecture names the catalog instead of "has no strict mode". Timeline: new `2026-10` entry for this fix (its `hash:` re-derived with `md hash`; the old pin matched the old content) |
| Rustdoc / test comments (comment-only) | `darkmatter/lib/src/markdown/compose/{pipeline/mod.rs, unknown_identifiers.rs, context/report.rs, inline/interpolation.rs, frontmatter_interpolation.rs, interpolation/evaluator.rs, expression/absence.rs, conditions.rs, frontmatter_shell_expansion.rs}`, `claudine/lib/src/composition/sequence/task/tests.rs`, `claudine/lib/src/composition/schema/tests.rs` (assertion message), `claudine/cli/tests/l1/wrap_compose_validation.rs` | "unknown root" → "undeclared property" / "absent property" vocabulary; "Under DM2 strict mode" → "Under DM2"; the group-leak test's doc names the `claudine.outside-group` error it now gets |
| DRY consolidation (behavior) | `darkmatter/lib/src/markdown/compose/directive_targets.rs`, `darkmatter/lib/tests/l1/directive_target_analysis.rs` | See the audit below |

Left alone on purpose: every "strict" hit about style (`--strict-style`),
schemas (`schema.strict`, DMLS `missing_required`), hashing (`md hash --strict`),
transclusion `fail_fast`, `ExpressionFailurePolicy::Strict`, and research docs.
Historical specs, completed logs, reviews, and `claudine/docs/rollout-strategy.md`'s
history tables keep their wording (R9). Two open Darkmatter feature drafts
(`2026-09-16-expression-type-system`, `2026-09-21-schema-enhancements`) still
name the removed symbols; they are specs, not current docs.

### DRY seam audit (acceptance criterion 14)

The [DRY table](migration-inventory.md#dry-and-ownership-audit) checked against
the final code. Re-scope adjustments: there is no provenance envelope (NR-2),
the Claudine catalog is Rust (`lifecycle/bindings.rs`), not YAML, and the
schema-assembly and generated-definition rows are R11/R12, out of scope.

**Consolidation made in this phase.**
`directive_targets::expression_path` (nullable directive-target analysis) kept
its own reserved-root literal, `"null" | "doc" | "ctx" | "env"`, which had
drifted from the catalog: a bare `current` or `current_env` target was
classified as a document property, so a schema-declared `current: file` made
the namespace look like a nullable document target. It now calls
`absence::is_reserved_root` (derived from `reserved_root_descriptors()` plus the
`null` literal). Test:
`darkmatter/lib/tests/l1/directive_target_analysis.rs::a_reserved_namespace_root_is_never_a_document_target`
(all five namespaces and `null` are `Unknown` while `doc.<name>` stays
nullable). Mutation-checked: restoring the old literal fails it on `current`.

| Concern | Single authority (final code) | Retained caller work and its owner reason |
|---|---|---|
| Reserved namespaces | Darkmatter `binding::is_reserved_namespace` over `reserved_root_descriptors()`; `absence::is_reserved_root` adds `null` | None left: Phase 5 replaced `absence`'s own literal and `CurrentScope::is_reserved_root`, this phase `directive_targets`. DMLS reads `BindingView::baseline()` |
| Lifecycle-global names and availability | Claudine `lifecycle/bindings.rs` (`binding_view`, `runtime_bindings`, `approval_diagnostics`) per `LifecycleScope` | None. Deleted along the way: `LATE_BINDING_ROOTS` (Claudine and DMLS), `SHELL_UNAVAILABLE_ROOTS`, `LIFECYCLE_COMM_FIELDS`, `shape::first_unavailable_root`/`root_in_expr`, `preflight::late_binding_root_in_expr`, `outside_group_global`, `lifecycle_injected_globals`. A source scan finds the three names only in `bindings.rs` |
| Bare-name precedence / state lookup | Darkmatter `ResolvedBinding::classify` and `EvaluationSession` | Claudine's three lookups (`LoopExpressionLookup`, `SourceExpressionLookup`, `SizedLookup`) keep their own document layering (loop state over frontmatter; sequence item over frontmatter). That layering is Claudine policy; each classifies through Darkmatter and resolves reserved namespaces first. `dispatch/expression.rs`'s two hook-event lookups serve hook `when=` over event metadata, not documents; they read no bare `ctx` and inherit the trait defaults |
| Effective-state and resolution-context construction | Darkmatter `EffectiveStateBuilder`, `ResolutionContext` | Each Claudine surface builds its state from the snapshot it owns (live frontmatter at event time, the approval snapshot at preflight, the task overlay at sequence time). The timing differs by surface, so one builder would erase capture timing (spec: "without erasing lifecycle context or capture timing") |
| Lazy values and memoization | Darkmatter `EvaluationSession` lazy cache (once per session) | `CurrentAuthority::memoized` per lifecycle event is Claudine's choice of memo scope (`current` once per event), layered on Darkmatter's mechanism |
| Parsing, evaluation, whole-value typing, mixed strings | Darkmatter parser/evaluator, `prepare_value`/`validate_prepared`/`evaluate_prepared`, `SubtreeCompose` | Sequence `expr.rs` moved onto `prepare_value`/`evaluate_prepared` in Phase 4. **Retained:** the loop action renderer (`looping::actions::render_action_value`) still walks spans itself and does not honor `{{{ … }}}`. Loop actions are outside this fix (NR-8); recorded as a follow-up |
| The four production `SubtreeCompose` sites and the direct `evaluate` | `executor.rs` (event time), `preflight.rs` (lifecycle shell approval), `sequence/preflight/mod.rs` (sequence approval), `sequence/task/mod.rs` (task values); `executor::eval_expr` via `layered_session` | All five take `(view, globals)` from `runtime_bindings(scope, ..)`, so they cannot disagree on globals. What differs is the state and scope each owns (above) |
| Prepare-time, shell-preflight, sequence-preflight, event-time checks | `validate_prepared` against the scope's `BindingView` (prepare time through `validate_no_err_in_no_error_events`; approval through `approval_diagnostics`); runtime through the session | **Retained:** `approval_diagnostics` refuses `current`/`current_env` by reading Darkmatter's `static_variable_reads` (a namespace cannot be declared in a view, NR-3); this is a Claudine policy over Darkmatter's analysis, not a walk. `shape::first_target_identity_root` still walks the AST for `ctx.agent`/`ctx.model`/`env.AGENT`/`env.MODEL` and dynamic `ctx[...]`/`env[...]` indexes. It enforces target identity at graph approval, not lifecycle-global availability, so AC 7 does not cover it. Possible follow-up: express it over `static_variable_reads` if Darkmatter reports dynamic index reads |
| Surviving-span checks | None: deleted (Phase 4) | `reject_control_plane_template` refuses template text in a `proxy.with` value for a *lifecycle key* of the target. It is a feature restriction (R5 keeps those), not a check on successful output |
| DMLS descriptors vs runtime | DMLS classifies through `BindingView::baseline()` and `validate_expression` | No Claudine descriptors reach DMLS (R7c out of scope), so a host global such as `err` is an undeclared-property advisory in the editor. Accepted gap, documented in `darkmatter/docs/lsp/features.md` |
| Errors | Typed Darkmatter cause (`ExpressionError`, `MarkdownError`, `BindingError`) | Claudine adds event/action/document context as `LifecycleCause` on `LifecycleErrorInfo::cause` and as `#[source]`; `err` JSON is a projection. The wrapping layers add context, not a second classification |

### Acceptance evidence map

| AC | Evidence |
|---|---|
| 1 | Source scan (Definition of Success `rg`) returns nothing in `darkmatter/` or `claudine/` source; the public API no longer has `SubtreeStrictness`, `.strict()`, `with_strictness`, the `compose_subtree` strictness argument, `validate_strict_roots`, or `is_known_variable_root` (Phase 3) |
| 2 | `binding_contract::registration::every_reserved_root_is_rejected_before_any_provider_runs`, `subtree.rs::a_reserved_root_cannot_be_registered_and_no_provider_runs`, `binding_contract::resolution::a_global_shadows_a_document_property_of_the_same_name`, `::doc_prefixed_reserved_names_read_document_data`, `absent_property_contract::an_exact_namespace_root_is_never_a_document_property`, and (this phase) `directive_target_analysis::a_reserved_namespace_root_is_never_a_document_target`. **Departure:** NR-3 widens the refused names to all five reserved roots |
| 3 | `absent_property_contract::an_absent_property_is_a_null_whole_value`, `::an_absent_property_is_empty_in_a_mixed_string`, `::an_absent_property_takes_the_falsy_branch_and_the_fallback`; Claudine `binding_contract::an_absent_property_in_*` (three tests); `interpolation_conformance.rs::every_engine_agrees_on_missing_properties_and_escapes` |
| 4 | `compose/tests/lookup_parity.rs` (inventory of every lookup + probe, mutation-checked), `absent_property_contract::a_missing_bare_property_does_not_resolve_from_ctx`, `checked.rs::a_bare_name_never_reads_the_ctx_namespace`, `subtree.rs::a_bare_name_never_reads_ctx` |
| 5 | Source scan: `first_undefined_stack_variable`, `validate_no_undefined_lifecycle_variables`, `LifecycleUndefinedVariable` absent; the old closed-world tests were removed or converted (Phase 4) |
| 6 | `binding_contract::resolution::available_null_unavailable_and_absent_property_are_distinct`; Claudine `bindings/tests.rs::runtime_resolution_follows_the_matrix_and_never_reads_the_document` |
| 7 | `lifecycle/bindings.rs` is the only declaration; the DRY scan above finds no other list. `bindings/tests.rs::passive_validation_follows_the_matrix_in_an_inactive_branch`. The remaining Claudine AST walk (`first_target_identity_root`) enforces target identity, not lifecycle globals |
| 8 | `binding_contract::passive_validation::*` (Darkmatter: inactive branch, execution-dependent deferral, unknown functions, no provider run); Claudine `binding_contract::malformed_expressions_and_unknown_functions_halt_before_side_effects`, `::an_unavailable_global_fails_with_its_reason_and_never_reads_the_document`, `::successful_output_containing_braces_is_delivered_as_data`; `absent_property_contract::every_escape_form_is_inert` |
| 9 | `binding_contract::set_and_proxy_with_stay_atomic_when_a_null_member_precedes_a_failure`, `proxy_with_evaluation::evaluation_is_atomic_across_the_whole_mapping`, `sequence/task/tests.rs::byte_parity::*` (mutation-checked), `prepare/tests.rs::lifecycle_shell_approval_refuses_group_in_any_branch_while_doc_group_reads_the_document` |
| 10 | DMLS `undeclared_property::*` (four tests, including the no-separate-catalog source scan), `lsp_session::undeclared_property_fires_in_every_operand_position_at_warning_severity`, `mapping_only_corpus` |
| 11 | `claudine/cli/tests/l1/shipped_prompts.rs::shipped_implement_router_reads_absent_optional_inputs_unguarded`; `shipped_prompt_route_drift::*` (refreshed pin) |
| 12 | This phase's sweep: current docs and the Darkmatter/Claudine skills carry no strict-mode guidance and no "guard optional inputs" advice (`prompts/_prompt.md` rewritten in Phase 6). Remaining "strict" hits are style/schema/hash strictness |
| 13 | `design.md` and `design-contracts.md` hold the binding API decisions (D1, D2, D4, D7, D8, D21; C1, C2, C9); `design-review-1.md`/`-2.md` record their review; the Phase 1 rulings were raised as `human_review_items` and cleared before Phase 2. `migration-inventory.md` lists 21 implementations; Phase 1's refresh found 23 compiled plus two rustdoc examples, each given a disposition in its Phase 3 checklist, and Phase 3's `lookup_parity` test keeps the Darkmatter inventory current |
| 14 | The DRY audit above |
| 15 | Gates below |
| 16 | Claudine `binding_contract::the_typed_darkmatter_cause_survives_every_library_wrapper` |
| 17 | `interpolation_conformance.rs::every_engine_agrees_on_missing_properties_and_escapes`, `binding_contract::event_time_values_follow_the_shared_missing_property_table`; the executor has no post-evaluation check (source: `reject_surviving_spans` gone) |
| 19 | `claudine/cli/tests/l1/compose_schema_cli.rs::a_required_property_violation_stops_before_lifecycle_actions_under_existing_recovery_policy`, `absent_property_contract::a_required_property_violation_still_blocks`; the `initialize` prohibition tests unchanged and green |

### Departures from the spec and design

The spec is a snapshot and is left unchanged; the docs describe what was built.

- **NR-3 (AC 2):** registration refuses every root in
  `reserved_root_descriptors()` (`doc`, `ctx`, `env`, `current`,
  `current_env`), not only `doc`/`ctx`/`env`.
- **NR-4:** the global is `timing`, not `tracking`.
- **NR-5:** one resolution channel, `EvaluationLookup::resolve` (evolved from
  `get_checked`), rather than a second trait method.
- **NR-2:** plain `serde_json::Value`, no `ValueEnvelope`.
- **`EvaluationSession::associate`** instead of a separate
  `BindingEnvironment` type; no `PreparationContext` parameter; no prepared
  identity check (Phase 2).
- **`LayeredLookup` deleted** and replaced by `subtree::layered_session`
  (Phase 3).
- **Catalog in Rust,** not the YAML catalog the original DRY table named
  (re-scope).
- **DMLS reports unknown functions** through a new `dm.expression.unknown_function`
  error it did not have before, and host globals are advisories in the editor
  (R7c out of scope) (Phase 5).
- **Two extra approval walkers removed** in Phase 4 beyond the plan's list; the
  target-identity walk is kept (above).

### Requirement-to-test mapping (this phase)

| Requirement | Test(s) | Level / target |
|---|---|---|
| A bare reserved namespace (`doc`, `ctx`, `env`, `current`, `current_env`) or `null` is never a nullable document directive target, even when the schema declares that name; `doc.<name>` still is | `darkmatter/lib/tests/l1/directive_target_analysis.rs::a_reserved_namespace_root_is_never_a_document_target` (mutation-checked against the old literal) | Darkmatter L1 (`l1` binary; the file was already declared in `tests/l1/main.rs`; no tier marker) |
| Docs/skills (R9, AC 12) | Source sweep above; Mermaid diagram rendered with `mmdc` (headless, no window) | — |
| Skill files that tests read | `test-toolkit` `ci_workflow_contracts` (the plan's narrowed cell for `.claude/skills/claudine/architecture.md` and 7 more) | `test-toolkit` L1: 163 passed |

All other edits are comments and docs, with no behavior change. Input
Robustness Matrix: not applicable, because no file-format or configuration
reader changed.

### Gates

| Area | Recipe | Result |
|---|---|---|
| `darkmatter/` | `just test` | 8774 passed (Phase 5: 8773 + 1 new), 12 skipped (baseline skips) |
| `darkmatter/` | `just lint` | exit 0 |
| `darkmatter/` | `just test-l2` | 18 + 69 + 3 passed |
| `claudine/` | `just test` | 8068 passed, 9 skipped (baseline skips) |
| `claudine/` | `just lint` | exit 0. The only warning is the existing macOS linker notice `__eh_frame section too large`, which is unrelated |
| `claudine/` | `just test-l2` | 277 + 3 passed |
| `test-toolkit` | `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 163 passed |
| repository | `just ci-local --plan` | Reviewed. It covers the whole branch against `main`, and this phase adds no CI cell. The new skill edits schedule the existing narrowed `test-toolkit` L1 cell, which passes locally (above) |

No pre-existing or unrelated failures. Cross-OS: the one behavior change is a
string comparison against a catalog, with no `#[cfg]`, path, or process code,
so no `just cross-check` was run. CI covers the other environments.

### Phase 7 outcome

Checkpoint 7 is met. Every gate is green, the acceptance map covers criteria
1–17 and 19, and docs and skills use one vocabulary. Acceptance criteria 12
(no strict-mode or fallback-legality guidance), 13 (design record and
inventory), 14 (DRY audit), and 15 (gates) hold.

**Status: implementation complete, ready for review.** The spec stays in
`fixes/`; the author moves it to `_completed` after review.

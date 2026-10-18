---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-25-loops-and-flow-control/spec.md
plan: claudine/features/2026-10-25-loops-and-flow-control/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/features/2026-10-25-loops-and-flow-control/spike/loop_flow_spike.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/dependency-audit.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/path-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/coverage-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/matrix-contracts.md
    - claudine/features/2026-10-25-loops-and-flow-control/spike/README.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/lifecycle_directive_matrix.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_grammar_matrix.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/signatures.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/looping/dsl.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/gate_ambient.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/engine/tests/mod.rs
    - claudine/lib/src/composition/looping/engine/tests/post_checked_timing.rs
    - claudine/lib/src/composition/looping/engine/tests/rate_limits.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/data.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_function_contracts.rs
docs_updated_during_phase_2:
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/remote-url-references.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - dmls
human_review: false
human_review_items: []
source_files_during_phase_3:
    - claudine/cli/src/commands/compose/loop_run.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/tests/l1/error_guards/boxed-diagnostic-allow.toml
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_gate_ambient.rs
    - claudine/cli/tests/l1/loop_primitives.rs
    - claudine/cli/tests/l1/loop_retry_passes.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_grammar_matrix.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/gate_ambient.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/engine/tests/mod.rs
    - claudine/lib/src/composition/looping/engine/tests/pass_order.rs
    - claudine/lib/src/composition/looping/engine/tests/post_checked_timing.rs
    - claudine/lib/src/composition/looping/engine/tests/rate_limits.rs
    - claudine/lib/src/composition/looping/engine/tests/seed_state.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/frame.rs
    - claudine/lib/src/composition/looping/frame/tests.rs
    - claudine/lib/src/composition/looping/mod.rs
    - claudine/lib/src/composition/looping/types.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
docs_updated_during_phase_3:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/lib/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_4:
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/dry_run.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/report.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/l1/lifecycle_directive_matrix.rs
    - claudine/cli/tests/l1/lifecycle_prep.rs
    - claudine/cli/tests/l1/loop_break.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_containers.rs
    - claudine/cli/tests/l1/sequence_dry_run.rs
    - claudine/cli/tests/l1/sequence_grammar_matrix.rs
    - claudine/cli/tests/l1/task_guards.rs
    - claudine/lib/src/composition/coordinator/commit.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/prep.rs
    - claudine/lib/src/composition/lifecycle/prep/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/frame.rs
    - claudine/lib/src/composition/looping/frame/tests.rs
    - claudine/lib/src/composition/looping/types.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
docs_updated_during_phase_4:
    - claudine/cli/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/getting-started/index.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/schemas.md
    - claudine/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_5:
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
docs_updated_during_phase_5:
    - claudine/cli/README.md
    - claudine/lib/README.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
    - claudine/lib/src/composition/error/tests.rs
    - claudine/cli/tests/l1/sequence_frozen_graph.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/compose_presence_parity.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/os/windows.md
source_code:
    - claudine/cli/src/commands/compose/loop_run.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/dry_run.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/report.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_presence_parity.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/error_guards/boxed-diagnostic-allow.toml
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/lifecycle_directive_matrix.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/lifecycle_prep.rs
    - claudine/cli/tests/l1/loop_break.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/loop_gate_ambient.rs
    - claudine/cli/tests/l1/loop_primitives.rs
    - claudine/cli/tests/l1/loop_retry_passes.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_containers.rs
    - claudine/cli/tests/l1/sequence_dry_run.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_frozen_graph.rs
    - claudine/cli/tests/l1/sequence_grammar_matrix.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/task_guards.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/features/2026-10-25-loops-and-flow-control/spike/loop_flow_spike.rs
    - claudine/lib/src/composition/coordinator/commit.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prep.rs
    - claudine/lib/src/composition/lifecycle/prep/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/signatures.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/dsl.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/gate_ambient.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/engine/tests/mod.rs
    - claudine/lib/src/composition/looping/engine/tests/pass_order.rs
    - claudine/lib/src/composition/looping/engine/tests/post_checked_timing.rs
    - claudine/lib/src/composition/looping/engine/tests/rate_limits.rs
    - claudine/lib/src/composition/looping/engine/tests/seed_state.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/frame.rs
    - claudine/lib/src/composition/looping/frame/tests.rs
    - claudine/lib/src/composition/looping/mod.rs
    - claudine/lib/src/composition/looping/types.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/sequence/data.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_function_contracts.rs
documentation:
    - claudine/cli/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/getting-started/index.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/schemas.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/coverage-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/dependency-audit.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/matrix-contracts.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/path-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/spike/README.md
    - claudine/lib/README.md
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/remote-url-references.md
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-10-25-loops-and-flow-control (6 phases)

## Phase 1

Phase 1 started 2026-10-16. All four dependency specs report
`implemented: true` (2026-10-20-lifecycle-grammar,
2026-10-21-claudine-schemas, 2026-10-23-shell-template-approval,
2026-10-24-sequence-containers); 2026-10-26-task-inputs-and-providers is not
implemented, so task inputs stay `params:`.

### Wave 1: inventories

Three read-only inventories ran in parallel. Their reports are the Phase 1
deliverables, and later phases read them:

- `phase-1/dependency-audit.md`: delivered behavior with file:line evidence,
  the seams, the cause of the looping-document defect, the `on_rate_limit`
  inventory, every loose loop verb's signature, and a proposed owner for each
  shared file (§7).
- `phase-1/path-inventory.md`: every Darkmatter path function's return type,
  missing-target and URL behavior, and errors; the Phase 2 path fixture table
  (§5); and the smallest change sites (§6).
- `phase-1/coverage-inventory.md`: existing tests, test registration rules,
  where the prompt tests live, how route drift derives its hash, which formats
  each entry point accepts, and the acceptance-criterion → test → task map
  (§6).

**Dependency gate: PASS.** Lifecycle grammar (node tree, last-item rule,
planned `break`/`prep` in the schemas and the `PLANNED` parity list), shell
template approval (exact versus template, `ShellPreApprovals` on
`PreflightGraph`), and sequence containers (`ContainerRunner`, `cancelled`,
group `fail_fast`, `seq`/`group`/`loop` roots) are all in code. Task
`setup:`/`teardown:` is rejected grammar; only dead plumbing remains.
`break` and `prep` are parsed nowhere. The things the plan says this feature
removes are still present, as expected: loop `fail_fast`, predictive
`is_last` (`looping/engine.rs:284-306`), and the action applied after the
last pass (`engine.rs:496-522`).

Baseline caveat: the sequence-containers review fixes are uncommitted in this
worktree (20 modified claudine files and 3 untracked L1 tests). Phase 2 must
start from that tree and must not stash or reset it.

The defect in spec §3 is a routing gap. The loop engine is reached only from
`prepare_and_run_active_document` → `execute_loop_or_single`. A `prompt:`
task goes `WrapperPromptRunner::run` (`cli/wrap/sequence/task_run.rs:620`) →
`execute_composition_request_inner`, which runs a single attempt and never
reads `loop:`. A document reached by `proxy` inside a step does loop. The
shipped `review-loop.md` → `implement-plan.md` route hits the defect today.

### Compatibility ruling: `on_rate_limit` (plan rule 8)

Inventory: `loop.on_rate_limit` (`pause`/`abort`/`continue`, default
`pause`) and `--on-rate-limit` act only in the document loop, between
iterations, when the provider stream reports throttling. `abort` exits 75.
No shipped prompt or frozen fixture uses it. A container has no provider
stream, so the key cannot mean anything on a group or sequence loop.

The author has not ruled; this session is non-interactive. Recommendation,
recorded as a human review item: keep it for documents only, reject it with a
typed error on container `loop:` blocks, and apply the policy only after the
condition and cap decide another pass will run. That replaces today's
predicted-last-pass skip. **Until the author rules, current behavior is
preserved**, and Phase 2 must not remove or extend the key.

### Other rulings and findings

- **Empty collections** (plan rule 9): `gate: []` and `action: []` are
  explicit no-ops. `gate: null`, `gate: {}`, `action: ""`, and `loop: {}`
  are errors. `prep.with: {}` is an explicit empty overlay; `with: null` is
  an error, as for `proxy.with`. Today `gate: null` and `gate: {}` parse as an
  empty gate, so Phase 2 tightens them.
- **Duplicate keys**: YAML frontmatter and standalone YAML already reject
  them. JSON, JSON5, JSONL, and NDJSON are last-wins today. Phase 2 adds a
  duplicate-detecting read for executable task sources, before the
  `serde_json::Value` conversion.
- **Unclosed frontmatter fence** reads as empty frontmatter (Darkmatter). It
  is kept: it is Markdown grammar, which this feature does not change.
- **`implemented` guard**: characterized with `md compose`
  (`phase-1/matrix-contracts.md`, review metadata). A quoted `'true'` counts
  as implemented under `!= true`. The spec's semantics are kept.
- **`review_iterations` presence**: `frontmatter()` returns null for absent
  and for explicit null. The migrated prompt uses the existing
  `has_key(frontmatter(spec), 'review_iterations')` to tell them apart; no
  new read is needed.
- **`file_exists` swallows errors**: permission probe errors, invalid
  references, missing home or vault, and failed fetches all read `false`
  (`darkmatter/.../functions/mod.rs:1619-1622`). Spec §10 says such errors
  never become missing-file success. Recorded as a human review item for
  Phase 2.
- **`dirname` expectation changes**: preserving the absolute anchor reverses
  outputs pinned by `darkmatter/lib/src/markdown/compose/tests/schema.rs:565,
  795, 874`, `claudine/cli/tests/l1/compose_caller_file_provenance.rs:86`, and
  possibly `darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs:146`.
  The spec requires the change; Phase 2 updates these on purpose and logs it.
- **`review-loop.md` agents**: the live prompt takes `review_agent` and
  `repair_agent`; the spec's version takes one `agent`. Recorded for the
  author before Phase 5.
- **`commit.md`** already ends `initialize` with `skip`, so spec §9's edit
  has landed. Only its acceptance test (AC12) remains.
- **Windows recorder gap**: `launch_recorder` is Unix-only, and no portable
  prompt recorder exists. New prompt-recording CLI tests are `cfg(unix)`, as
  existing loop tests are, until a portable recorder lands.
- **Stale comment noticed, left as is** (no behavior change in this phase):
  `StackExecutionContext::scope` (`lifecycle/executor.rs:579`) still
  mentions task `setup:`/`teardown:` stacks. The owner of that file fixes it
  in Phase 2.

### Wave 2: control-flow spike

Ran once and passed 6/6. It drove the real `ContainerRunner` and
`LifecycleRunGuard` with fake bodies, as a temporary in-crate test that was
removed afterwards. The archive and the interface contracts are in
`spike/README.md` and `spike/loop_flow_spike.rs`. Summary:

1. A break settles a container through the existing guard (`defuse()`, then
   `finalize` once). No new guard state is needed, and Drop synthesizes no
   terminal event.
2. A failed `finalize` (`Finished { failed: true }` with a primary error)
   discards the pending break.
3. Group pass settlement is a pure declaration-order fold, run after every
   leg settles. A handled leg failure keeps the break; `fail_fast` failure
   wins.
4. `prep` is a returning **action** through an injected runner, like
   `shell_runner`, not a `StackControl`. The executor already continues the
   list and after a conditional branch.
5. **New seam needed:** `RuntimeState` cannot restore its mutation layer.
   `replace_sequence_scope` restores outputs only. Add a pass-checkpoint
   restore that keeps `proxy_chain`.
6. Call ancestry can reuse `enter_container_proxy`'s RAII guard: repeated
   calls are legal, active-ancestor calls are refused, and branches are
   isolated. Cancellation uses `current().child()`. **No deadline carrier
   exists**; add an absolute-instant `CallDeadline` that a child can only
   tighten.

### Matrix contracts

`phase-1/matrix-contracts.md` expands every column: each loose verb from its
real signature, each format's duplicate-key and trailing-content behavior,
and the measured review-metadata decisions. Each cell is marked covered,
Phase N, or N/A.

Implemented the parameterized traversal for the one format that carries a
loop field today, YAML frontmatter on a document, as
`claudine/cli/tests/l1/loop_config_matrix.rs`. It is registered `cfg(unix)`
in `cli/tests/l1/main.rs` because it uses the Unix-only `launch_recorder`.
It has a `matrix!` macro with one `#[test]` per cell: 36 cells, each one
textual edit of the counting control, asserting exit status, launch count,
prompt text, that a rejection names the document, and that no audio spool
exists. All 36 pass, the slowest at about 1.6 s.

Deviation: the plan asks for one traversal per supported format. The other
formats (standalone YAML, JSON, JSON5, JSONL/NDJSON) carry only container
and task fields that do not exist until the Phase 2 grammar, so their
traversals are written in Phase 2 against the contract table. The probe run
measured 18 cells that differ from the contract today. They are not
registered, so `just test` stays green, and each is listed with its current
result in the contract:

- **11 unlocated errors**: `LoopInvalid` messages carry no document path.
  The cells are `loop: null`, `loop: {}`, a missing condition, null
  `until`/`while`/`max`/`action`/`actions`, both conditions together, both
  aliases together, and `action: ""`. Phase 2 makes the loop error located.
- **Accepted when they should be rejected**: `fail_fast: true`,
  `gate: null`, and `gate: {}`.
- **Rejected when it should resolve**: templated `max: "{{ cap }}"`.
- **Rejected but unlocated**: `fail_fast: null` and `on_rate_limit: null`.
- **Wrong exit status**: `max: 2` with the condition unmet exits 0, because
  the action runs after the last pass. The loop engine fixes this in Phase 3.

### Acceptance mapping

Every criterion (1–14) is mapped to an observable assertion, a proposed test
file, and its owning plan task in `phase-1/coverage-inventory.md` §6. That
table is the Phase 1 acceptance mapping.

### Verification (macOS)

- `just test` in `claudine/`: 9892 passed, 0 failed, 9 skipped, 27 slow. All
  slow tests are pre-existing. No `loop_config_matrix` test is slow; the
  longest takes about 2.7 s under full-suite load.
- `just lint` in `claudine/`: first run failed on `clippy::unnecessary_to_owned`
  in the new matrix macro's control row. Fixed, then clean.
- `cargo nextest run -p claudine-cli --test l1 -E 'test(loop_config_matrix)'`
  after the fix: 36/36 passed.
- `just check-tier-coverage claudine`: nothing stranded. The new tests carry
  no tier marker and run in L1.
- Not run: Darkmatter tests (no Darkmatter code changed) and other OS rigs.
  The only new test is `cfg(unix)` subprocess composition with no path or
  OS-specific logic, so Linux and WSL CI cover it. The temporary spike module
  was removed and `lifecycle/tests/mod.rs` is back to its prior content.

Requirement-to-test mapping for this phase: Phase 1 changes no behavior. Its
one test deliverable is the robustness-matrix traversal: 36 cells, each
named in `phase-1/matrix-contracts.md`. The 18 cells that differ from the
contract today are listed there with their measured current result and the
phase that changes them. The spike tests were a one-time experiment and are
archived, not registered.

Phase 1 status: complete, ready for review. Phase 2 is blocked on the
`on_rate_limit` ruling and, for the path work, on the `file_exists` ruling
(see `human_review_items`).

## Phase 2

Phase 2 started 2026-10-16 (claude/opus). The worktree was clean at start:
the sequence-containers review fixes the Phase 1 log calls uncommitted were
committed before this phase (`ec331c303` and earlier), so no stash or reset
question arose.

### Rulings taken for this phase

The session is non-interactive and the author's answers to the Phase 1
human-review items had not arrived. Each item that blocks Phase 2 is decided
the conservative way below and listed again for the author.

- **`loop.on_rate_limit`: kept for documents, rejected on container loops**
  (Phase 1 option A). Document behavior is unchanged. A group or sequence
  `loop:` naming the key gets a typed, located error. This can be relaxed
  later without breaking anyone; accepting it now could not be undone.
- **`file_exists` reports probe errors** (Phase 1 option A). Spec §10 is
  explicit: permission errors, invalid references, and remote-fetch failures
  are never turned into missing-file success, and existence checks keep their
  missing-file behavior. A genuinely absent file still answers `false`.
- **Grammar ahead of behavior.** Phase 2 parses and validates container
  `loop:`, task `when:`, `break`, and `prep`, but their behavior arrives in
  Phases 3 and 4. Between those phases a valid construct is refused with a
  typed "not yet supported" error rather than accepted and ignored, because
  an ignored `when:` would run a guarded task and an ignored container loop
  would reproduce the run-once defect. The refusal is removed by the change
  that lands each behavior.

### Author rulings (2026-10-17)

The author confirmed or settled every open decision before Phase 3:

1. **`loop.on_rate_limit`:** documents only; a group or sequence `loop:` naming it is a located error. Confirms the Phase 2 ruling.
2. **`file_exists()` probe failures are errors**; a genuinely absent file (or a directory) is `false`. Confirms the Phase 2 ruling.
3. **`file_exists()` takes `string | null`.** `null` (an optional reference never supplied) is `false`; any other non-string argument is a type error. Implemented in this phase.
4. **`absolute()` on a recursive `%` reference:** an existing match returns its path; no match is a `not_found` error (path-inventory row 6b).
5. **A `break` reason may interpolate** (`Expr`, like `error`'s reason), departing from the plan's `Option<String>`.
6. **A malformed `CLAUDINE_MAX_ITERATIONS` fails only runs that would use it**; under `--max-iterations` it is never read. Implemented in this phase.
7. **The review-loop migration is deferred.** `prompts/review-loop.md` was retired; `just fix` stays the loop. Phase 5 skips "Review prompt migration" and acceptance criterion 11 (see the Phase 5 note below). The one-agent-or-two question is moot.

### What Phase 2 delivered

- **Loop configuration** (`looping/config.rs`): one parser for every owner (`parse_loop_block` with `LoopOwner` and a located `LoopLocation`), strict shapes, did-you-mean for unknown keys, `fail_fast` as removed grammar, `on_rate_limit` document-only, templated `max` resolved once.
- **Lifecycle directives:** `break` (reason, code `0..=255`) and `prep` (target, `with`) parse everywhere but `cancelled`; non-final `prep` is permitted; both are refused at run time in `executor.rs::resolve_control` until Phase 4. `gate: null` and `gate: {}` are parser errors.
- **Sequence and group grammar:** container `loop:` on group objects, sequence roots, and inline nested sequences, validated then refused (`ContainerLoopNotYetSupported`); task `when:` validated then refused (`TaskWhenNotYetSupported`); `TaskLoopMisplaced` for a `loop` on any other task; `WhenMisplaced` at a sequence root or on a group object; JSON-family duplicate keys rejected before the lossy value conversion (`sequence/data.rs`, `DuplicateScan`).
- **Schemas:** `claudine/schemas/{action,lifecycle,claudine}.yaml` carry the new keys, with planned markers for run-time refusal.
- **Darkmatter path contracts:** `absolute` builds a missing target's first candidate and rejects HTTP(S); `dirname` and the file-index family keep an absolute input's anchor; `file_exists` reports probe failures.
- **Robustness matrices:** `loop_config_matrix` (YAML frontmatter), `lifecycle_directive_matrix`, and `sequence_grammar_matrix` (YAML frontmatter, standalone YAML, JSON, JSON5, JSONL, NDJSON), each a control row plus one-edit cells asserting the public result.

### Deviations

- `break` reason is an expression (ruling 5).
- The task guard is validated but not stored on the preflight task; Phase 4 adds the field.
- `file_exists`'s permission-error row is Unix-only; Windows would need an ACL fixture.
- The matrices were written before the robustness rule changed (field shapes once per validator, in process); they spawn one process per cell, and 23 NDJSON cells repeat the JSONL ones through the same loader.

### Process note

Phase 2's first run took over three hours instead of 30–40 minutes. Three parallel subagents each ran the full claudine suite (about 10,000 tests) on one host, and its load average reached about 200; timing tests then failed or timed out, and each failure triggered another full rerun. Every timed-out test passed when rerun alone. `prompts/_headless-orchestration.md` and the `rust-testing` skill now require subagents to run only their slice's tests, with the full suite once per phase by the orchestrator.

### Verification (macOS, 2026-10-17)

Run one at a time, with no other build or test on the host:

- `just test` in `claudine/`: 10,226 passed, 9 skipped.
- `just lint` in `claudine/`: clean (the only warning is the macOS linker's unwind-table notice).
- `just check-tier-coverage claudine`: 0 stranded.
- `just test` in `darkmatter/`: 10,127 passed, 10 skipped, after two test fixes for Phase 2's schema changes: `darkmatter-cli`'s `schema_validate` test now expects the union message for `loop.max: 0` (the field is a count or a `{{ cap }}` template), and `dmls`'s `lifecycle_inventory_matches_the_authored_claudine_schema` now treats `fail_fast` as removed grammar the parser recognizes but the schema omits.
- `just lint` in `darkmatter/`: clean.
- Darkmatter's other dependents (biscuit-icon, biscuit-speaks, messenger, playa, research, sniff) were not run: none uses the changed expression functions, and Phase 2's only Darkmatter API change is additive.
- Not run: native Windows, Linux, and WSL2 for the final tree. The path-contract work was cross-checked on native Windows in its own worktree; CI covers the rest.

Phase 2 status: complete, ready for review. Phase 3 starts from `phase: 3` in the plan.

### Note for Phase 5: the review loop changed after this spec closed

Recorded 2026-10-17, outside this feature's own work, so the Phase 5 "Review
prompt migration" task does not undo it.

- **`prompts/review-loop.md` was deleted on purpose.** It had never run, and
  the loop in use is `just fix` (`just/review.just`). Spec §11 and acceptance
  criterion 11 describe migrating a file that no longer exists; settle with
  the author whether Phase 5 recreates it as a looping sequence or migrates
  `just fix` instead, before starting that task.
- **A ready review now gets one follow-up pass.** `implement-suggestions.md`
  fixes the review's small medium and low findings, records the rest and
  every observation in the area's `fixes/_unscheduled/<spec>-follow-ups/`
  backlog, and marks the review `implemented`; no review follows it.
  `just fix` runs that pass after a ready review and on a rerun whose ready
  review is still open. Review frontmatter `findings` now lists every
  priority (`<ID> (<priority>): <title>`); only a high finding makes `ready`
  false.
- **This departs from spec §11**, whose loop breaks on `ready` before any
  repair, and from criterion 11's "first review ready, no commit step
  launches". The departure is intended: whatever replaces `just fix` must
  keep the follow-up pass. `claudine/cli/tests/prompts/review_fix_rehearsal.rs`
  is the behavioral contract to carry over.

## Phase 3

Phase 3 ran 2026-10-17 (claude/opus), one orchestrator, no subagents: the
engine, the adapters, and the retry work share the same few files, so they were
built in order rather than in parallel waves.

### What Phase 3 delivered

- **One loop mechanism** (`looping/frame.rs::LoopFrame`). The frame is
  allocated before `initialize`, which now reads pass 1's values. The cap is
  override > authored > 100. `is_last` is `count == max` only; the predictive
  evaluation is gone. `decide` reads the condition before it classifies the
  cap (`Complete`/`Continue`/`Exhausted`), and `stage_actions` runs only on
  `Continue`. `record_pass` runs before a pass's terminal event, so that event,
  `finalize`, the gate, the condition, and the action read the pass's own
  typed result (text, the sequence pass's array slice, the group's dictionary);
  `retry_pass` gives a retried attempt the previous result back.
  `LoopOutcome::{ConditionMet, Exhausted, Broken}` is on `LoopExecutionResult`
  (`Broken` is set by Phase 4).
- **Document engine** (`execute_loop_with_lifecycle`) rebuilt on the frame.
  Loop-level fail-fast is removed (`LoopExecutionOptions::fail_fast`,
  `LoopConfig::fail_fast`, `resolve_fail_fast_from_env`; `CLAUDINE_FAIL_FAST`
  is no longer read): a failed pass ends the loop before its gate. The
  rate-limit policy (documents only, as ruled) now applies only when another
  pass will run. The gate is a shared `run_loop_gate`.
- **Document adapter.** A `prompt:` task whose document declares `loop:` (a
  sequence step or a group leg) runs through `prepare_and_run_active_document`
  inside the task's `StepScope` (`task_run.rs::run_looping_document`,
  `iterate.rs::run_step_document`). The harness hands a looping document's
  terminal events the just-finished pass's output.
- **Container adapters.** Nested sequences (`sequence/task/sequence.rs`),
  groups (`group.rs`), and the root sequence (`wrap/sequence/container.rs`,
  `mod.rs`) allocate a frame from `PreflightLoop` (frozen on
  `PreflightSequence`/`PreflightGroup.loop_config`; the gate joins the
  container's `loop` stack, so preflight approves its shells) and run
  `looping/container.rs::finish_container_pass` after a successful pass, then
  `ContainerRunner::begin_next_pass`. The container's `loop.*` reaches its
  events, gate, legs and children, task templates and params
  (`runtime_bindings_in_loop`), and non-looping prompt documents
  (`PromptTaskRequest.loop_values`, `StepComposeContext.loop_values`). The
  root runs its passes inside `execute_sequence_inner` and settles the final
  pass in the existing outer path. `ContainerLoopNotYetSupported` is removed.
- **Retry in a looping container.** `PassCheckpoint` (mutations and output
  count at pass entry) is restored on retry; `RuntimeState::restore_pass`
  keeps position and proxy chain.
- **Diagnostics.** `LoopEvaluationFailed` (condition or action, located at the
  `loop:` key and the pass), `LoopActionWriteRefused` (a container action the
  runtime layer refuses), and `LifecycleLoopGateFailed` now carry the pass.
- **Schemas, docs, skill.** The container-loop "planned" markers are gone from
  `claudine/schemas/`; `looping.md` (pass order with a Mermaid diagram, results
  per primitive, retry inside a looping container, "When a pass fails"),
  `sequences.md`, `lifecycle.md`, `composition.md`,
  `frontmatter-properties.md`, `lib/README.md`, and the `claudine` skill
  (SKILL, architecture, cli-reference, timeline) describe the behavior.

### Decisions and deviations

- **Retry restores the mutation layer only in a looping container.** Spec §2
  resets a looping container's pass to its entry state; the
  sequence-containers rule ("child results reset, runtime writes survive") is
  kept for a container without a loop, because the two rules disagree for a
  one-pass container and the older one is documented and tested. Recorded in
  `looping.md`.
- **The gate and condition of a document loop read the pass's effective
  frontmatter** beneath the loop's control state (new
  `LoopIterationOutput::frontmatter`). The control seed alone left a template
  approved by shape (a sequence prompt task's gate shell) with nothing to fill
  `{{ out }}` from. Actions still stage onto the control state.
- **Runtime loop locations of a root step read `tasks[N]`**, as every other
  preflight task diagnostic does, while grammar errors say `sequence[N]`
  (`group_loop_is_carried_on_the_preflight_group`). Not unified here.
- **`LoopEvaluationFailed` boxes its `CompositionError` source** (recursion);
  allowlisted in `boxed-diagnostic-allow.toml` with the tests that prove the
  rendering. A retired `_loop_*` read stays `RemovedGrammar`, unwrapped.
- **Existing tests that pinned removed behavior were changed, not kept:**
  the refusal tests (`sequence_cli`, `sequence_errors_cli`, `sequence_groups`,
  lib grammar/preflight tests), the `*_loop_is_refused` matrix cells (now
  `*_loop_runs`), the `CLAUDINE_FAIL_FAST` tests (`loop_cli`,
  `lifecycle_downgrade_outcome`, `wrap_sigint`), and predicted-`is_last`
  expectations (`loop_gate_ambient`, engine unit tests). The three
  `handoff_owners` "refused request leaves no entry" CLI tests depended on a
  loop continuing past a failed pass, which no longer exists; they now assert
  that a refused handoff ends the loop with one report. The ledger property
  itself stays covered by `coordinator::tests`
  (`a_refused_*_leaves_the_ledger_untouched`).
- **Pre-existing limitation found:** a nested inline sequence inside a
  *referenced formal* document (JSON-family `sequence: "@tasks.json"`) cannot
  be reread at its turn ("nested sequence disappeared before preparation").
  The matrix cells `json*/jsonl/ndjson::nested_loop_is_accepted` pin that the
  loop is accepted and then hit this limitation; it is not loop-specific.
- **Not done in this phase:** a summary line naming the loop outcome; the
  typed outcome is on `LoopExecutionResult` and exhaustion is its own error,
  but no renderer prints "condition met"/"cap reached". Phase 4's `break`
  adds the third outcome.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| AC1: the counting document loops directly, as a sequence step, and as a group leg | `loop_primitives::a_looping_document_runs_three_passes_{under_compose,as_the_only_sequence_step,as_the_only_group_leg}` |
| AC2: looping three-step sequence and two-leg group, events and outputs | `loop_primitives::a_looping_sequence_runs_three_passes_with_per_pass_events_and_flat_outputs`, `a_looping_group_runs_three_passes_and_publishes_its_final_dictionary`, `nested_loops_shadow_the_outer_loop_and_restart_on_each_outer_pass`, `a_document_pass_reads_its_own_result_from_its_terminal_events` |
| AC3: condition met, met on the final pass, cap reached (no action after the last pass) | `loop_primitives::{sequence,group}_{condition_met,condition_met_on_the_final_pass,cap_reached}`; engine `pass_order::*`; `frame::tests::*`; `loop_config_matrix` for documents |
| AC4: runtime cap override per loop; typed key errors | `pass_order::a_runtime_cap_override_outranks_the_authored_cap`, `frame::tests::cap_precedence_is_override_then_authored_then_default`, `an_unresolved_templated_cap_is_refused_at_its_authored_location`; key errors from Phase 2 matrices |
| AC14: retry inside a looping container | `loop_retry_passes::{a_looping_sequence_retry_restores_the_pass_entry_and_keeps_earlier_passes, a_looping_group_retry_rebuilds_the_current_pass_leg_results, a_nested_looping_sequence_retry_keeps_its_earlier_passes}`, `frame::tests::a_retried_pass_reads_the_previous_pass_result_again` |
| No publication after a failed loop; atomic container action; live file reads | `loop_primitives::a_failed_{sequence,group}_loop_publishes_nothing_to_its_parent`, `a_failed_container_action_commits_nothing_and_names_the_pass`, `a_container_condition_reads_files_written_by_the_pass` |
| Loop-level fail-fast removed | `loop_cli::compose_loop_stops_on_a_failed_pass_even_with_claudine_fail_fast_false`, `iteration_actions::a_failed_pass_ends_the_loop_before_its_gate_and_action`, `a_failed_action_stage_commits_nothing_and_fails_the_loop` |

The new CLI files (`loop_primitives.rs`, `loop_retry_passes.rs`) are
`cfg(unix)`: they use the Unix-only `launch_recorder` or POSIX stub
executables, like the existing loop tests (Phase 1 finding: no portable
prompt recorder exists).

### Verification (macOS, 2026-10-17)

Run one at a time, nothing else on the host:

- `just test` in `claudine/`: 10,252 passed, 9 skipped (16 slow, all
  pre-existing files; `ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents`
  takes 3.5 s alone, so it is load-sensitive rather than over budget).
- `just lint` in `claudine/`: clean.
- `just check-tier-coverage claudine`: 0 stranded.
- `darkmatter-cli` `schema_validate` and `dmls` `nested_span` tests (they read
  `claudine/schemas/`): 147 passed.
- No other package depends on `claudine`.
- Not run: Windows/Linux/WSL2 cross-checks. The change has no path or
  OS-specific code; the new CLI tests are Unix-only like their neighbors, and
  CI covers the rest.

Phase 3 status: complete, ready for review.

## Phase 4

Phase 4 ran 2026-10-17 (claude/opus), one orchestrator and no subagents:
Phase 3 is uncommitted, so a subagent worktree would not hold it, and the four
Wave 7 tasks edit the same container files (`sequence/task/{mod,sequence,group}.rs`,
`wrap/sequence/iterate.rs`), so parallel builds would break one another.

### Task guards (`when:`) and caller values

- `sequence/grammar.rs::task_when` no longer refuses a valid guard
  (`TaskWhenNotYetSupported` is gone). A `when:` on a step without an
  executable (a step that runs the document body) is `WhenMisplaced`: the
  spec puts the guard "beside its executable", and accepting it there would
  leave it as step data that nothing reads.
- The guard is evaluated at the task's turn by
  `sequence::task::evaluate_task_guard` (read-only: shells disabled, no live
  document cell, task-setup binding scope) through
  `StackExecutionContext::evaluate_task_guard`. False gives a `skipped`
  outcome (`TaskOutcome.skipped`, null result, nothing published, no event);
  a raising guard is `TaskWhenEvaluationFailed`, located at the guard.
- Nested children and group legs evaluate it in `TaskExecution::run_guarded`.
  A group leg evaluates it in its worker against the member state and buffer
  built from the group-start snapshot, so a late-admitted leg sees the start
  snapshot while file reads stay live. A root step evaluates it in
  `iterate.rs::run_one_step` before the step composes
  (`task_run.rs::root_step_guard`, reading the authored frontmatter under the
  step's override layers), then runs `TaskExecution::run_admitted`.
- Summaries: `SequenceStepResult.skipped`, `SequenceTaskResult.skipped`, and
  `GroupTaskResult.skipped`; a root step prints `step N/M skipped`; a group
  breakdown prints `skipped`; `group.legs.<name>.outcome` is `skipped` with a
  null output (existing `LegOutcome::Skipped`).
- Root step guard locations read `tasks[N].when`, as every root step's
  run-time diagnostic does (Phase 3 left that convention in place).
- **Caller values (§7)** already worked through the template-approval
  dependency: a setter in the first and a later `shell:` task and in step
  data, and a missing one failing preflight. One defect was found and fixed:
  a first task's exact approval replayed its pass-1 bytes on later outer
  passes after a loop action changed a value it reads. `PreflightApproval::verify`
  now layers the invocation's runtime writes over the preflight values and
  refuses changed bytes (`SequenceShellApprovalChanged`, now naming the
  changed runtime value, `changed: n`).

### `break`

- **Design: a break slot per loop, not a returned signal.** Every
  `LoopFrame` owns a `LoopBreakSlot`, and `LoopFrame::ambient()` hands it out
  on `LoopAmbient.breaks`, so every event, gate, and descendant page whose
  innermost loop the frame is records into the same slot (the `loop.*`
  plumbing Phase 3 built already reaches every one of them, the CLI harness
  included). The executor's `resolve_control` records the evaluated
  `LoopBreak { reason, code }` there and returns `StackControl::Break`, which
  ends the event like `stop` (`decide_control` maps it to `Stop`). With no
  `loop_values`, it is `LifecycleBreakWithoutLoop` (an evaluation error,
  reason `BreakWithoutLoop`, which `no_error` cannot suppress). The spike's
  typed `Settled::Broken` return would have threaded a new value through
  `TaskOutcome`, `PromptRunOutcome`, the harness's `LoopStep`, and every
  container; the slot gives the same nearest-loop semantics with the frame's
  identity as the loop id. Equality of `LoopAmbient` ignores the slot.
- **What each layer does.** A container sees the slot pending after a child
  and starts no later child; after its children it skips its terminal event
  (`guard.defuse()`, then `finalize` once), and publishes its partial result.
  `ContainerRunner::record_outcome` maps a break in `initialize`/`start` to
  `Finalize`. A looping owner ends with `ContainerPassEnd::Broken` (checked
  before and after the gate in `finish_container_pass`); the document engine
  checks after a pass, after the gate, and handles an `initialize` break
  (runs `finalize`, no pass). In the CLI harness a `start` break or a handoff
  target's `initialize` break settles through `settle_break_before_launch`;
  a document's `initialize` break in `pipeline.rs::route_initialize` runs
  `finalize` and returns with nothing published. The root sequence stops its
  steps (`iterate.rs`), settles through `RootRun::settle_pass`, and exits
  with the code (`RootRun::broken_exit_code`); the root compose loop does the
  same in `prep.rs` (only when `step_scope` is `None`, the root invocation).
- **Groups.** Each leg gets `LoopAmbient::with_isolated_breaks()`, so a
  pending break stops no sibling and no admission. After the fold, a
  successful group records the first succeeded leg's break in declaration
  order into its own slot. A failed group, a failed `finalize`, or a retry
  discards a pending break (`LoopBreakSlot::discard`), so a failure always
  wins.
- **Publication.** `PromptRunOutcome.withheld` (no launch and nothing
  captured: an `initialize` skip or a break before launch) keeps a document
  that published nothing out of `seq.outputs`; a skipped group leg is
  `skipped` with a null output by the existing rule. A looping document's
  `final_output` is `None` when no pass ran. This also stops an `initialize`
  `skip` from appending an empty entry, which spec §2 ("Skipped children add
  no array entry") requires.
- **Summary.** A broken loop prints `loop broken: <reason> (code N)` as an
  info line (`LoopBreak::summary`), for nested containers, document loops,
  and the root. Condition-met and cap-reached loops still print no line
  (Phase 3 gap, unchanged).
- **Pre-existing gap fixed on the way:** the prompt barrier
  (`WrapperPromptRunner::validate`) composed a group leg's document without
  the enclosing loop's values, so a non-looping leg document reading
  `loop.count` failed at its barrier. It now sets `context.loop_values`.

### `prep`

- **Lib seam** (`lifecycle/prep.rs`): `PrepRunner::run(&PrepCall) -> Result<(), PrepFailure>`,
  installed per thread (`prep::install`), inherited by group workers
  (`prep::inherit().enter()`, beside the cancellation scope), with an active
  call chain on the same thread-local (`enter_call` → `CallFrame`, released
  on drop, so a repeated call is legal and branches keep their own chains).
  The chain is one cycle domain with handoffs: `commit_proxy` and
  `RuntimeState::enter_container_proxy` refuse a proxy to a document with an
  active call (`proxy_reaches_an_active_caller`).
- **Executor:** `prep` is a returning action (`execute_action_inner` →
  `run_prep`), not a `StackControl`. The target and the whole overlay
  evaluate first (as `proxy`'s do); a failed target is a dispatch failure of
  the event (`LifecyclePrepFailed`, the target's message and snapshot
  attached), so it takes the event's ordinary error route and `no_error`
  tolerates it like a failed shell. A thread with no runner refuses with
  `LifecyclePrepUnavailable`. `LifecycleDirectiveNotYetSupported` and its
  `DirectiveNotYetSupported` reason are removed.
- **CLI runner** (`cli/src/commands/prep_target.rs`), installed by `compose`,
  `inline-compose`, and `sequence`: resolves the target relative to the
  authoring document (`resolve_proxy_target`), enters the call, and runs the
  target in this process through the same internal entry points those
  commands use (`run_composition_inner`, `sequence::run_sequence_target`).
  The target therefore gets its own `initialize`, schema check, approval,
  runtime layer, and output, and none of the caller's setters or
  `loop`/`seq`/`group` roots. The overlay becomes its only caller values,
  encoded as data (`PrepCall::overlay_as_data`, the closure's literal
  tokens), so a produced `{{ … }}` is never evaluated again. Exit `0`
  returns (a `skip`, or a root `break` with code 0); any other status, or an
  error, fails the call. Ctrl-C reaches the target through the shared
  cancellation and the run exits 130.
- **Deviations, recorded for review:**
  - **No static approval inventory of `prep` targets** in sequence preflight.
    A target is approved as its own invocation when it runs, with the
    caller's approval flags (`--yolo`, the policy). Inventorying literal
    targets would also need the target run to accept the caller's closed
    approvals, which the in-process entry points do not take. Acceptance
    criterion 6's "transitive preflight approval" cell is therefore not
    covered.
  - **`kind: group` files are not runnable `prep` targets**: the sequence
    entry point rejects a group document as not directly executable, and a
    synthetic one-step sequence over the group file trips the preflight
    self-reference cycle check. The call fails with that typed message.
  - **Deadlines:** Claudine has no invocation-wide deadline; `--timeout` and
    `--step-timeout` are per provider child. A target inherits the caller's
    flags, so each of its children gets the same limits; nothing is reset
    because nothing invocation-wide exists. The spike's `CallDeadline` was
    not added.
  - **Re-entering the command entry points in process** is how the "one
    shared target runner" is realized; no child `claudine` process is spawned.
- **Diagnostics:** a `start`/`initialize` action failure now reports its
  message (`lifecycle start failed: <err.msg>`) instead of the bare phrase,
  so a failed call names the target and its cause.

### Dry run

- `--dry-run-shell` (`SharedComposeArgs`, clap `requires = "dry_run"`, so it
  alone is a usage error, exit 2) with help text saying it mutates; the
  `--dry-run` help now says composition's `$( … )` still runs.
- Under `--dry-run`, `TaskExecution::run_shell` renders each command
  (`rendered (dry-run): would run `<bytes or template>``) unless
  `PromptTaskRunner::dry_run_shell`; `run_side_effect` always renders the
  authored action. Both return `PrimaryOutcome::withheld()`, so nothing
  reaches `seq.outputs`; a side effect's file write or runtime `set` never
  happens under either flag combination.
- Loops: a dry run now allocates the frame (Phase 3 returned none), so the
  one previewed pass reads pass 1's `loop.*` instead of failing with "loop
  unavailable"; containers skip the gate and condition under dry run, and a
  note says repetition is not evaluated (`looping::dry_run_loop_note`, root
  and nested). Guards are shown, not evaluated (lib and root). Literal `prep`
  targets are listed for the root sequence and for each previewed document
  (`LifecycleConfig::literal_prep_targets`). The final line reads
  `Sequence previewed (dry-run): N step(s) rendered, none run` instead of a
  succeeded count.

### Wave 8 — integration

- The directive matrix now proves every valid `break` shape ends a
  three-pass loop before a launch (`Completes(0)`), that outside a loop it is
  a run-time error, and that every valid `prep` shape runs its target once
  and returns (`Calls { helper: 1, caller: 1 }`), including from
  `initialize` and the gate. Schema descriptions for `break`, `prep`, and
  task `when:` drop their planned markers (`claudine_schemas` asserts the new
  wording; `defer` stays planned).
- Cross-surface cases: a guarded looping container (`task_guards`), a target
  that hands off and returns, a failed call in `failure` keeping the original
  error with one `finalize`, a helper called on every outer pass
  (`lifecycle_prep`), and a dry run of a loop with a helper (`sequence_dry_run`).

### Docs and skill

The Documenter subagent updated `flow-control.md` (with a Mermaid diagram of
break propagation), `flow-control-reference.md`, `looping.md`,
`sequences.md`, `lifecycle.md`, `composition.md`, `schemas.md`, and both
READMEs; this session fixed the dry-run text in `docs/cli/sequence.md`,
`docs/topics/execution-flow.md`, and `docs/getting-started/index.md`, and
updated the `claudine` skill (SKILL, cli-reference, timeline). Left for
Phase 5: the `FAIL_FAST` mentions in both READMEs (pre-existing), and a
`=== Document N of M ===` divider described in `composition.md` that the CLI
may no longer print.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| AC5 `break` from a leg document in a looping group; from a group in a looping sequence; nested loops end only the innermost; root-only exit code | `loop_break::{a_leg_document_breaks_its_looping_group_and_the_parent_continues, a_group_breaks_a_looping_sequence_and_its_code_is_the_exit_status, nested_loops_end_only_the_innermost_and_its_code_stays_in_the_summary, a_looping_document_break_ends_its_loop_with_the_root_exit_code}` |
| AC5 from `initialize` (no launch, nothing appended, `finalize`, success); no enclosing loop | `loop_break::{a_break_from_initialize_launches_nothing_appends_nothing_and_finalizes, a_break_with_no_enclosing_loop_is_a_typed_runtime_error}`, `break_prep::a_break_outside_any_loop_is_a_located_error_that_no_error_cannot_suppress`, directive matrix `break_*` |
| AC5 pending break in a concurrent group (declaration order, queued legs), sibling failure beats it, handled failure keeps it, failed cleanup beats it, Ctrl-C overrides it | `loop_break::{a_pending_break_waits_for_every_leg_and_takes_the_first_in_declaration_order, queued_legs_still_run_after_a_pending_break, an_unhandled_sibling_failure_beats_a_pending_break, a_handled_leg_failure_keeps_the_pending_break, a_failed_finalize_during_a_break_beats_it, ctrl_c_overrides_a_pending_break}` |
| AC5 exactly-once cleanup from `finalize` and the gate; partial results | `loop_break::{breaks_from_finalize_and_the_gate_clean_up_exactly_once, a_broken_pass_publishes_its_completed_partial_results}`, `container_runtime::container_break_before_its_children_fires_only_finalize_once`, `frame::tests::a_break_recorded_through_any_ambient_of_the_frame_reaches_it_and_the_first_wins` |
| AC6 `prep` returns to the next item (also after a branch), repeated calls, overlay, caller-relative resolution | `lifecycle_prep::prep_returns_to_the_next_item_and_a_repeated_call_is_legal`, `break_prep::prep_returns_to_the_next_item_including_after_its_branch`, `lifecycle_prep::a_helper_called_on_every_outer_pass_runs_each_time` |
| AC6 disk-only return, isolated roots | `lifecycle_prep::prep_returns_disk_only_with_isolated_runtime_roots` |
| AC6 target failure fails the event with its cause; `no_error`; failure in `failure` keeps the original | `lifecycle_prep::{a_failed_target_fails_the_callers_event_with_its_cause, a_failed_call_in_failure_keeps_the_original_error_and_cleanup_runs_once}`, `break_prep::a_failed_target_fails_the_event_with_its_cause_unless_no_error_tolerates_it` |
| AC6 cycles across mixed `prep`/`proxy`; hop limit; chain release and branch scoping | `lifecycle_prep::a_call_or_handoff_back_to_an_active_caller_is_refused`, `prep::tests::*` |
| AC6 `prep` from `initialize` with the parent's preparation after return; child skip returns | `lifecycle_prep::prep_from_initialize_runs_before_the_callers_preparation` |
| AC6 break in the target does not escape; non-zero root break code fails the call; target proxy | `lifecycle_prep::{a_targets_break_stays_in_the_target_and_its_code_decides_the_call, a_target_that_hands_off_returns_when_the_handoff_completes}` |
| AC6 cancellation passes through | `lifecycle_prep::ctrl_c_during_a_target_cancels_the_run` |
| AC6 `break`/`prep` under `cancelled` are parse errors | directive matrix `break_under_cancelled_is_rejected`, `prep_under_cancelled_is_rejected` (Phase 2) |
| AC6 transitive preflight approval of a target's shell; deadline pass-through | **not covered** (deviations above) |
| AC7 `when:` false runs/fires/appends nothing, records skipped; re-evaluated per pass; expression error; false guard over an invalid task; root `when:`; delayed admission snapshot vs live files | `task_guards::*`, `sequence_grammar_matrix::*::{step,leg,task}_when_*` (true/false cells per format), `sequence::tests` grammar cases |
| AC8 caller setter in shell tasks and step data; missing value fails preflight; first-task changed bytes on a later pass | `task_guards::{a_caller_setter_is_read_by_shell_tasks_and_step_data, a_missing_caller_value_fails_preflight, a_first_task_whose_bytes_change_on_a_later_pass_is_refused}` |
| AC9 dry run: index untouched / staged with `--dry-run-shell`; usage error; file-dependent loop previews once with no lifecycle or `prep`; side effects rendered under both flags | `sequence_dry_run::*` |

The new CLI files `loop_break.rs` and `lifecycle_prep.rs` are `cfg(unix)`
(Unix-only `launch_recorder`, POSIX `sh` barriers, `SignalledRun`), as the
Phase 3 loop tests are. `task_guards.rs` is portable; in `sequence_dry_run.rs`
only the two Git staging tests are `cfg(unix)` (the fixture's minimal Windows
`PATH` has no `git`).

### Verification (macOS, 2026-10-17)

Run one at a time, with no subagent or other build running:

- `just test` in `claudine/`: 10,316 passed, 9 skipped, 0 failed (11 slow,
  all in pre-existing files). Two earlier full runs found three tests that
  pinned the old behavior or broke a guard, each fixed and rerun alone:
  `sequence_containers::recursive_dry_run_fires_no_container_events`
  (pinned a dry run running `shell:` tasks; now asserts it does not),
  `sequence_errors_cli::a_{unitless,zero}_shell_timeout_is_rejected` (the
  dry-run render now runs after the task's `timeout:` is validated), and
  `spawn_site_guard::migrated_l1_tests_keep_the_isolation_the_builder_gave_them`
  (the dry-run Git tests now use `common::init_git_repo` and read
  `.git/index` instead of spawning `git` with `current_dir`).
- `just lint` in `claudine/`: clean. Its error-transport guard caught three
  string collapses on the way (`TaskWhenEvaluationFailed` now carries a typed
  `TaskGuardFailure` source; the `prep` target reader wraps its
  `MarkdownError`).
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents reading `claudine/schemas/`: `darkmatter-cli` `schema_validate`
  (130 passed) and `dmls` `nested_span`/`lifecycle_inventory` (17 passed). No
  other package depends on `claudine`.
- Not run: the Windows cross-check. The change adds no OS-specific code
  (thread-locals, paths through `canonicalize_simplified`), and every new
  test that asserts on shell output, uses `launch_recorder`, or drives a
  signal is `cfg(unix)`; the portable tests (`task_guards`, part of
  `sequence_dry_run`) use `side_effect:` tasks or fail before a command runs.
  Consequence to note: `prep`, `break` through a prompt document, and
  `--dry-run-shell` have no native-Windows test yet (the same gap Phase 3
  recorded for loop tests: no portable prompt recorder).

Phase 4 status: complete, ready for review.

## Phase 5

Phase 5 ran 2026-10-17 (claude/opus), one orchestrator, no subagents: the
work was a docs review plus one prompt test, too small to split.

### Review prompt migration: skipped

Not implemented, per the author's 2026-10-17 ruling (Phase 2, ruling 7):
`prompts/review-loop.md` was retired and `just fix` stays the loop.
Acceptance criterion 11 and the `review-loop.md` rehearsals are deferred with
it. `commit.md` already used `skip` (Phase 4), so its `initialize` opt-out
needed no change.

### Current behavior docs

The Phase 4 docs were already current; this pass fixed what was left:

- `cli/README.md` and `lib/README.md` said a sequence had "`FAIL_FAST`
  propagation". Loop and sequence fail-fast are removed grammar. Both now say
  a failed step ends the sequence unless a lifecycle stack recovers it.
- `docs/topics/composition.md` said a sequence dry run prints a
  `=== Document N of M ===` divider. The CLI draws a horizontal rule, and
  `wrap_sequence_composition::sequence_dry_run_concatenates_bodies_with_dividers`
  already asserts the old text is gone. The doc was wrong (code wins); so was
  that test's doc comment, which is fixed too (comment only).
- `docs/topics/flow-control/flow-control.md` did not explain why an opt-out
  `initialize` uses `skip`, not `stop` (plan item). The `skip` section now
  explains it with a nothing-staged commit example and notes that a skipped
  document adds no `seq.outputs` entry. The `stop` section links to it.
- Checked and already current: iteration-order and break-propagation Mermaid
  diagrams (`looping.md`, `flow-control.md`), cap-only `is_last`, retry pass
  restoration, partial results, root-only exit codes, disk-only `prep`
  return, guard snapshots, template approval, dry-run limits. No topic page
  names a feature or fix. Darkmatter's `darkmatter-expressions.md`,
  `expression-functions.yaml`, and the `darkmatter` skill already describe
  `absolute` on a missing target (first candidate, HTTP(S) rejected) and
  `dirname` keeping an absolute input's anchor. No crate dependency changed,
  so `docs/dependencies.md` is unchanged.
- Still planned, left as is: `defer`, persisted checkpoint/resume,
  `kind: group` files as `prep` targets, and `prep` targets in a sequence's
  up-front approval.
- The `claudine` skill needed no change: Phase 4 already updated it, and
  this phase changed no behavior.

### Workflow rehearsals

- **Empty staged commit (new test):**
  `shipped_prompt_contract::shipped_commit_prompt_with_nothing_staged_launches_no_agent`
  (`prompts` binary, `prompt-tests`, L1 tier, F3: real shipped `commit.md`,
  the real CLI, the compiled fake Codex, a scratch Git repository with an
  unstaged file only). It asserts exit 0, no provider launch, no audio spool,
  and a clean lifecycle. To prove the test checks something, `skip` in
  `prompts/commit.md` was changed to `stop` for one run: the test failed with
  "no agent may launch". The file was restored and `git status` is clean.
  A first draft also asserted that the `message:` text was printed. That was
  wrong: `message` goes to the outbound messaging route, not the terminal.
  The assertion was dropped.
- **Route drift (failure from an earlier commit, fixed):** `shipped_prompt_route_drift` failed
  because `prompts/_implement/implement-plan.md` changed one prose line ("name
  the boundary … fidelity rung (F1/F2/F3) and tier") in a commit before this
  phase, and nobody refreshed the fixture. The change is body prose, with no
  schema or loop meaning, so the line was copied into
  `cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
  and the hash was refreshed with
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 just test-prompts shipped_prompt_route_drift::`.
- `just test-prompts`: 87 passed (15 slow, all in the existing
  `pr_flow_rehearsal`/`prompt_guide_defects`).
- `compose_caller_file_provenance` and `compose_initialize_acceptance` are L1
  and run in `just test` below.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Empty staged state launches no provider; no host audio spool | `shipped_prompt_contract::shipped_commit_prompt_with_nothing_staged_launches_no_agent` (new) |
| A staged commit still composes and launches (control) | `shipped_prompt_contract::shipped_commit_prompt_composes_resides_in_and_fires_success_cleanly` |
| Frozen route fixture agrees with the shipped prompt | `shipped_prompt_route_drift::*` |
| Shipped prompt contracts, caller-file provenance, initialize acceptance | `shipped_prompt_contract::*`, `compose_caller_file_provenance::*`, `compose_initialize_acceptance::*` |
| Dry-run divider text described in docs | `wrap_sequence_composition::sequence_dry_run_concatenates_bodies_with_dividers` (existing) |

The input robustness matrix does not apply: no parser, loader, or config
reader changed.

### Verification (macOS, 2026-10-17)

Run one at a time, with no other build or test on the host:

- `just test-prompts` in `claudine/`: 87 passed, 0 failed.
- `just test` in `claudine/`: 10,321 passed, 9 skipped, 0 failed (131 slow).
- `just lint` in `claudine/`: clean.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: not run. This phase changed only `claudine-cli` test files,
  test fixtures, and docs. No package compiles them, and `claudine/schemas/`
  (read by `darkmatter-cli` and `dmls`) is unchanged.
- Windows cross-check: not run. No production code changed. The new test is
  `cfg(unix)` like its neighbor commit test, because the fixture's Windows
  `PATH` has no `git` to stage with.

Phase 5 status: complete, ready for review. Acceptance criterion 11 stays
deferred by the author's ruling.

## Phase 6

Phase 6 ran 2026-10-17 (claude/opus), one orchestrator, no subagents: the
phase is verification, and its gates must run one at a time on this host.

### Claudine verification

- `just test` in `claudine/`: 10,321 passed, 9 skipped, 0 failed.
- `just lint` in `claudine/`: clean.
- `just test-prompts` (feature `prompt-tests`): 87 passed.
- `just check-tier-coverage claudine` (repository root): 0 stranded.
- Affected L2, `just _test_l2 claudine-cli --features terminal-tests
  level2_lifecycle_control level2_sequence`: 114 passed. These tests run in
  detached tmux panes, so no window gains focus. The other L2 and L3 tests
  are not affected by this feature, which changed no terminal rendering.
- Review items:
  - `prep` spawns no `claudine` process. `commands/prep_target.rs`,
    `compose/prep.rs`, and `lifecycle/prep.rs` contain no `Command::new` or
    `current_exe`, and the runner re-enters the command entry points in
    process.
  - `break` and `prep` are in `signatures.rs::LIFECYCLE_CONTROL_VERBS` and in
    `schemas/lifecycle.yaml`, and the existing parity tests pass.
  - Every new test is in a declared binary: the layout gate test and the
    tier audit both pass.

### Darkmatter verification

- `just test` in `darkmatter/`: 10,133 passed, 10 skipped, 0 failed.
- `just lint` in `darkmatter/`: clean (including the `zed-dmls` wasm check).
- Darkmatter source did not change after Phase 2. That phase checked the
  path-function dependents (`darkmatter-cli`, `dmls`, Claudine), and
  Claudine's suite above covers the callers of `absolute` and `dirname`.

### Portable evidence

Remote runs with `just cross-check`, from this worktree's uncommitted tree.
Because the tree is uncommitted, no run published a rig observation; these
logs are targeted evidence, not CI receipts.

| Environment | Package | First run | After fixes |
|---|---|---|---|
| macOS (local) | claudine, claudine-cli | all pass | all pass (final gate below) |
| native Windows | claudine | 5,400 passed, **1 failed** | 5,401 passed |
| native Windows | claudine-cli | 3,622 passed, **19 failed** | 3,641 passed, 12 skipped |
| Linux | claudine-cli | — | 4,545 passed, 304 skipped |
| Linux | claudine | — | 5,459 passed |
| WSL2 | — | not run | the nightly schedule runs it; no local evidence |

All 20 Windows failures were in tests, and one fix attempt resolved them. Only one
failing test came from this feature. The others came from features this
branch depends on (sequence containers, shell templates, the frozen-graph
fix, core schema presence). Earlier phases never ran Windows, so none of them
had been seen. The production code was unchanged. The fixes:

- `lib/src/composition/error/tests.rs`
  `a_break_outside_a_loop_renders_as_its_own_error_not_as_a_crashed_expression`
  (this feature): it rendered through an ambient `Terminal::default()`. The
  Windows runner detects no color, and at `ColorDepth::None` inline code keeps
  its backticks, so "lifecycle break outside a loop" did not match. To
  confirm the cause on macOS, the test was first pinned to `ColorDepth::None`:
  it failed the same way. It now pins `Terminal::new_optimistic(80)`.
- `cli/tests/l1/sequence_frozen_graph.rs` (11 tests): `redact_rewrites`
  parsed `{{ seq.outputs }}` as JSON. A lifecycle `stdout` action prints
  `\\` as `\` (also true on macOS: `stdout: 'x\\y'` prints `x\y`, and
  `{{ ['a\b"q'] }}` prints `["a\b\"q"]`), so a Windows path reached stdout
  unescaped. Path strings are now redacted in the text before parsing.
- `cli/tests/l1/ctx_per_run.rs`
  `container_events_refresh_current_after_nested_child_work`: its `git add`
  shell task used the default minimal `PATH`, which has no Git for Windows.
  It now uses the file's existing `host_path()` escape, with the reason
  written at the call site.
- `cli/tests/l1/document_environment_routes.rs`
  `parallel_group_leg_shells_start_from_their_legs_map`: two parallel legs
  appended to one `events.log` with `cmd`'s `>>`, and Windows failed one leg
  with a sharing violation. Each leg now writes its own log, and the
  assertion merges them.
- `cli/tests/l1/compose_presence_parity.rs` (6 tests): the Windows stub was a
  `goose.cmd`, and Rust refuses to pass the multi-line prompt to a batch file
  ("batch file arguments are invalid"). It now copies the compiled
  `claudine-fake-goose`, whose marker mode is driven by `PROVIDER_MARKER` (the
  Unix stub reads the same variable now).

The `os` skill (`windows.md`) gained three of these traps: concurrent `cmd`
appends, a lifecycle `stdout` printing `\\` as `\`, and the ambient terminal
in rendering tests. The batch-file rule was already there.

### Acceptance criteria accounted for

| AC | State | Evidence |
|---|---|---|
| 1–4, 14 | covered | Phase 3 mapping: `loop_primitives`, `loop_retry_passes`, engine `pass_order`, `frame::tests`, `loop_config_matrix` |
| 5 | covered | Phase 4 mapping: `loop_break::*`, `break_prep::*`, `container_runtime` |
| 6 | covered except two cells | Phase 4 mapping (`lifecycle_prep::*`, `prep::tests::*`). **Not covered:** transitive preflight approval of a statically known target's shell (not built: targets are approved as their own invocation when they run), and deadline pass-through (Claudine has no invocation-wide deadline; `--timeout` and `--step-timeout` are per provider child and the target inherits them) |
| 7–9 | covered | `task_guards::*`, `sequence_grammar_matrix`, `sequence_dry_run::*` |
| 10 | covered | Phase 2 Darkmatter path-anchor fixtures (Darkmatter `just test` above) |
| 11 | deferred | author ruling 2026-10-17: `review-loop.md` retired, `just fix` stays the loop |
| 12, 13 | covered | `shipped_prompt_contract::*` (including the Phase 5 empty-staged commit test), `compose_caller_file_provenance`, `compose_initialize_acceptance`, `shipped_prompt_route_drift` |

Robustness: no parser or loader changed in this phase, so the input
robustness matrix does not apply.

### Remaining review risks

- **No native-Windows test of the new behavior.** Most new `prep`, `break`,
  and loop tests through a prompt document, and the `--dry-run-shell` tests,
  are `cfg(unix)` because `launch_recorder` is Unix-only. The Windows runs
  above prove that what compiles there passes, not that these paths work
  there.
- **`stdout` prints `\\` as `\`.** This lifecycle `stdout` behavior is older
  than this feature and was not changed here. A `{{ … }}` value printed by
  `stdout` is not reliably JSON when it holds a backslash. If that is
  unintended, it needs its own fix.
- The two AC6 cells above, and the deferred AC11.

### Final gates (macOS, after the Windows fixes)

- `just lint` in `claudine/`: clean.
- `just test` in `claudine/`: 10,321 passed, 9 skipped, 0 failed.
- Scope calculator on the uncommitted tree (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages` = `claudine`, `claudine-cli`,
  `test-toolkit`, `darkmatter`; `change_class` = `package`; source packages
  `claudine`, `claudine-cli`; reverse dependency `claudine-contract`.
  - `darkmatter` and `test-toolkit` are test-input cells for changed docs and
    skills. Darkmatter's full suite passed above.
  - `just test test-toolkit`: 474 passed, **2 failed, both pre-existing and
    unrelated**. `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior`
    requires `.claude/skills/os/ci-evidence.md` to say
    `BISCUIT_CI_CONSTRAINTS_DIR`, and the committed file at `HEAD` does not.
    `ci_workflow_contracts::just_commit_is_deterministic_under_ci` checks the
    root `justfile`'s `commit` recipe. This phase changed neither file; both
    match `HEAD`.
  - `claudine-contract`: not run separately. It has no code that this
    phase's test-only changes affect.

Phase 6 status: implementation complete, ready for review.

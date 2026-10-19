---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/spec.md
plan: claudine/features/2026-10-26-task-inputs-and-providers/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
    - claudine-cli
    - biscuit-tui
    - darkmatter
    - darkmatter-cli
    - dmls
human_review: false
source_files_during_phase_2:
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/lifecycle.yaml
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/with_file_provenance.rs
    - claudine/cli/tests/level2/level2_file_resolution_capture.rs
docs_updated_during_phase_2:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/pre-flight-checks.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_3:
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/selection.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/types.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/providers.rs
    - claudine/cli/src/commands/wrap/sequence/resolve.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/tests/common/review_screen.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_task_providers.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/level2/level2_sequence_review_screen.rs
    - claudine/docs/providers/dispatch-inventory.json
    - biscuit-tui/lib/src/components/input_table/table.rs
    - biscuit-tui/lib/src/components/input_table/table/tests.rs
docs_updated_during_phase_3:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/execution-flow.md
    - biscuit-tui/docs/components/input_table.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_4:
    - claudine/schemas/sequence.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/fixtures/sequence-schema/both-triggers-malformed-with.md
    - claudine/lib/tests/fixtures/sequence-schema/deeper-than-typed-levels.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-kinds.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-published-outputs.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-sibling-cancellation.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-teardown-migration.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-tolerant-group.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-looping-prompt-step.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-task-guard.md
    - claudine/lib/tests/fixtures/sequence-schema/group-execution.md
    - claudine/lib/tests/fixtures/sequence-schema/group-misspelled-max-parallel.md
    - claudine/lib/tests/fixtures/sequence-schema/level1-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level1-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/level2-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level2-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/level3-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level3-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/root-fail-fast.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-absent.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-mapping.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-null.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-number.md
    - claudine/lib/tests/fixtures/sequence-schema/step-data.md
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
docs_updated_during_phase_4:
    - claudine/schemas/README.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_5:
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/types.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/tests/l1/sequence_schema.rs
docs_updated_during_phase_5:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/sequences.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
source_code:
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/lifecycle.yaml
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/with_file_provenance.rs
    - claudine/cli/tests/level2/level2_file_resolution_capture.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests.rs
    - claudine/lib/src/composition/error/render/selection.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/src/commands/wrap/sequence/providers.rs
    - claudine/cli/src/commands/wrap/sequence/resolve.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/common/review_screen.rs
    - claudine/cli/tests/l1/sequence_task_providers.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/level2/level2_sequence_review_screen.rs
    - claudine/docs/providers/dispatch-inventory.json
    - biscuit-tui/lib/src/components/input_table/table.rs
    - biscuit-tui/lib/src/components/input_table/table/tests.rs
    - claudine/schemas/sequence.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/schemas/claudine.yaml
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/fixtures/sequence-schema/both-triggers-malformed-with.md
    - claudine/lib/tests/fixtures/sequence-schema/deeper-than-typed-levels.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-kinds.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-published-outputs.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-sibling-cancellation.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-teardown-migration.md
    - claudine/lib/tests/fixtures/sequence-schema/example-containers-tolerant-group.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-looping-prompt-step.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md
    - claudine/lib/tests/fixtures/sequence-schema/example-loops-task-guard.md
    - claudine/lib/tests/fixtures/sequence-schema/group-execution.md
    - claudine/lib/tests/fixtures/sequence-schema/group-misspelled-max-parallel.md
    - claudine/lib/tests/fixtures/sequence-schema/level1-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level1-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/level2-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level2-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/level3-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/level3-valid.md
    - claudine/lib/tests/fixtures/sequence-schema/root-fail-fast.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-absent.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-mapping.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-null.md
    - claudine/lib/tests/fixtures/sequence-schema/sequence-number.md
    - claudine/lib/tests/fixtures/sequence-schema/step-data.md
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/tests/l1/sequence_schema.rs
documentation:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/pre-flight-checks.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/timeline.md
    - claudine/docs/topics/execution-flow.md
    - biscuit-tui/docs/components/input_table.md
    - claudine/schemas/README.md
    - claudine/docs/topics/schemas.md
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-10-26-task-inputs-and-providers (5 phases)

## Phase 1

### Dependency check and rulings (Rules 1, 2, 14)

All dependency and related specs are implemented (`implemented: true`):

| Spec | Location | State |
| --- | --- | --- |
| `2026-10-20-lifecycle-grammar` | `_completed` | implemented |
| `2026-10-21-claudine-schemas` | `_completed` | implemented |
| `2026-10-23-shell-template-approval` | `_completed` | implemented |
| `2026-10-24-sequence-containers` | `_completed` | implemented |
| `2026-10-25-loops-and-flow-control` | active (review-1 present, uncommitted spec edit in working tree — not touched) | implemented |
| `2026-10-14-union-type-constraints` | `_completed` | implemented |
| `2026-10-13-named-type-presence-and-local-names` | `_completed` | implemented |
| `2026-10-16-core-schema-types` | `_completed` | implemented (no in-flight edits to coordinate with any more) |

- **Rule 1 (dependency gate) — resolved, no ASK needed.** The schema and loop
  specs have landed, so Phase 4 is unblocked. `claudine/schemas/lifecycle.yaml`
  already exports `stack`, `initialize-stack`, `loop` (line 124),
  `cancelled-stack` (line 299, typed at all five conditional levels), and the
  container types `step`/`task`/`group` plus `step-2`/`task-2`/`group-2`
  (two typed container levels). Phase 4's "add `cancelled-stack`" task is
  therefore already done by the dependency; Phase 4 reuses these and extends
  the container expansion to three typed levels in `sequence-types.yaml`.
  Note: `lifecycle.yaml`'s `task` type still declares
  `params: "object -> parameters passed to the executable"` (and its `task-2`
  copy); the rename must update it.
- **Rule 2 (error-family names) — resolved.** `2026-10-20-lifecycle-grammar`
  already renamed the family. The live variants are `LifecycleWithNotMapping`,
  `LifecycleWithDynamicKey`, `LifecycleWithWholeMapping`,
  `LifecycleWithEvaluationFailed`, and `LifecycleWithUnsupported`; no
  `LifecycleProxyWith*` variant remains (only the unrelated
  `LifecycleProxyWithoutOwningCoordinator`). Phase 2 keeps this family and
  renames nothing.
- **Rule 14 (schema file ownership) — resolved.** `claudine/schemas/` is the
  only definition. `claudine/lib/src/composition/schema/bundle.rs` embeds the
  files with an explicit `include_str!` list (`CLAUDINE_SCHEMA_FILES`) and
  names the triggers in `CLAUDINE_TRIGGER_FILES`; both lists must gain
  `sequence.yaml`/`sequence-types.yaml` in Phase 4 (code, not a build or
  `just` step). `claudine/docs/schemas/{claudine,compose,inline-compose}.yaml`
  are **empty files** and `action.yaml` there is an old draft;
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs` classifies
  `claudine/docs/schemas` as "design drafts, not loaded by any reader". They
  are not mirrors and need no sync. `darkmatter/docs/schemas/` contains no
  `claudine.yaml` (the plan's premise was wrong); nothing to coordinate there.
  `claudine/cli/tests/l1/shipped_prompts.rs:15` and
  `darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs:100` read
  `claudine/docs/schemas` (the partials) and are unaffected.
- `claudine/schemas/README.md` says "the five files" and must be updated in
  Phase 4 to seven.

### Test baseline (2026-10-17, macOS host)

Run before any change, sequentially, with no subagent running tests:

| Gate | Result |
| --- | --- |
| `claudine/`: `just test` | 10321 passed, 9 skipped, 13 slow (196.7 s wall); exit 0 |
| `claudine/`: `just lint` | exit 0 |
| `darkmatter/`: `just test schema` (lib, CLI, DMLS tests whose names contain `schema`) | 2262 passed; exit 0 |

No pre-existing failures. Unrelated working-tree changes, left untouched:
`claudine/features/2026-10-25-loops-and-flow-control/spec.md` (modified) and
`claudine/features/2026-10-25-loops-and-flow-control/review-1.md` (untracked).
The host load average was 57–66 during the run (other sessions); the suite
was green regardless.

### Contract inventory

Paths are relative to `claudine/`. Line numbers are as of this phase's start.

#### Task `params` readers and writers

- **Reserved-key lists** (`lib/src/composition/sequence/reserved.rs`):
  `TASK_OPTION_KEYS` (`:25-28`, `"params"` at `:26`) and
  `allowed_task_options()` (`:70-79`; `Prompt` arm `:72` is
  `["name","params","operation","flow","when"]`). Module doc `:4-6`.
- **Option rejection.** `normalize.rs:228-262` raises
  `CompositionError::SequenceInvalidTaskField { index, field, executable }`
  (`error/mod.rs:1694-1706`, "sequence step at index {index} sets `{field}`,
  which is not valid for a `{executable}` task") for an option outside
  `allowed_task_options`. Task/group/file references are checked in
  `preflight/mod.rs:2186-2240` (`parse_task_object`) with
  `SequenceReferenceInvalid { context: "task", .. }`. `grammar.rs:580`
  `removed_task_key` is the existing hard-rename pattern if a `params` hint is
  ever wanted (not planned: `params` becomes ordinary step data).
- **Fields.** `preflight/shape.rs:38-40` `PreflightTask.params` (unevaluated;
  origin `PreflightTask.origin_path`); `preflight/mod.rs:160-164`
  `PromptReference.params` + `deferred_params`; `task/mod.rs:205-212`
  `PromptTaskRequest.{set_overrides, params, params_origin_path}`;
  `task/mod.rs:434` `TaskExecution.user_setters`.
- **Flow.** `preflight/mod.rs:797` reads `option_map(options, "params")` →
  `record_prompt_reference` (`:1910-1958`, evaluates with
  `reapproval::compose_approval_value` against the authoring file's
  `source_context_for(origin)`, defers late values) → `:877` stores it.
  `task/mod.rs:910-932` `prompt_request` → `evaluate_params()` (`:1410-1426`,
  raises `SequenceTaskParamReserved`) → `layered_overrides` (`:1430-1439`);
  `:820-840` re-resolves at the task's turn. `task/sequence.rs:177,477` clear
  params for container body/proxy tasks. `lifecycle/prewarm/sequence.rs:107`
  treats param keys as late. CLI: `wrap/sequence/mod.rs:142-149` applies
  `reference.params` as data during discovery compose; `task_run.rs:336,360`
  scans them for ctx reads; **`task_run.rs:772-795` already turns them into
  origin-anchored caller records** (`CallerInputLayers::from_caller_overrides`
  with `derive_source(&request.params_origin_path)`), so task inputs already
  follow the authoring-file rule; only the key changes.
- **Precedence today** (`task/mod.rs:1434-1439`), lowest first: document
  frontmatter → task `params` (`OverrideOrigin::Data`) → sequence user setters
  (`Authored`) → runtime mutation snapshot → reserved overlay. Matches Rule 4.

#### `with` evaluators (proxy / retry / prep)

- **Parse** (`lifecycle/parse.rs`): `:893-903` accepts `with` on
  `proxy|retry|prep` only, else `LifecycleWithUnsupported` (`error/mod.rs:
  1026-1037`; **its doc and message are stale** — they say "only valid on
  `proxy` and `retry`" and omit `prep`; Phase 2 fixes that). `:1129-1195`
  `parse_with` raises `LifecycleWithWholeMapping`, `LifecycleWithNotMapping`,
  `LifecycleWithDynamicKey`. Types in `lifecycle/actions.rs`: `WithOverlay`
  (`:528`), `WithOverlayError` (`:359`), `type_with_value` (`:588`). This is
  the evaluator Phase 2 should share with task `with`.
- **Evaluate** (`lifecycle/executor.rs`): proxy `:2067-2078`, retry
  `:2083-2104`, `run_prep` `:2150-2185`, all through `resolve_with_overlay`
  (`:2194-2220`) and the atomic `walk_with_overlay` (`:2223-2251`,
  `LifecycleWithEvaluationFailed`). Result is a plain `IndexMap<String,
  Value>` with **no origin**.
- **File resolution today:**
  - `proxy.with` / `retry.with`: stored in `harness_orch/types.rs:54,63`,
    merged by `wrap/overlay.rs:17`, handed over as
    `PrepareOptions.proxy_overlay` and layered by
    `LayeredOverrides::proxy_overlay` (`runtime_state.rs:321`) as `Data` with
    **no `CallerInputRecord`**, so a relative file value resolves against the
    **target** document.
  - `prep.with`: `PrepCall::overlay_as_data()` (`prep.rs:46`) →
    `cli/src/commands/prep_target.rs:39-50` sets it as `shared.set`, so the
    entry point captures it as a caller setter with the **launch** context
    (`compose/prep.rs:234-237`, `sequence.rs:392-395`). **`prep.with` today
    resolves from the launch directory**, which the plan did not mention; Phase
    2 moves it to authoring-file-relative too.
- **Caller provenance types to reuse:** `composition/types.rs:584`
  `CallerInputLayers` / `:642` `from_caller_overrides(set, origin:
  FileResolutionContext)` → Darkmatter `CallerInputRecord` /
  `CallerInputRecords` (`darkmatter/lib/src/markdown/compose/context/
  options.rs:27,53`), applied in `prepare.rs:103,444` and
  `schema/translate.rs:217`. The origin carrier for `with` values should be a
  `CallerInputRecord` built with the authoring document's
  `FileResolutionContext`, layered below caller setters.

#### Planned-target / provider consumers (all under `cli/src/commands/wrap/`)

- **Gate:** `sequence/mod.rs::execute_sequence_inner` (`:379`); `:482-484`
  document-level hints only; `:629-649` `classify_agent_resolution` →
  `AgentResolutionFailed` (`lib/.../error/mod.rs:283`) when headless. Runs
  before the graph is inspected, so shell-only sequences abort today.
- **Drafts:** `mod.rs:651-732`, one `SequenceStepDraft` per step from the
  document's hints.
- **Review screen:** `sequence/review.rs` (`is_review_eligible :23`,
  `baseline_targets :35`, `merge_reviewed_targets :66`, `review_targets :93`,
  `needs_review :116`, `live_targets :129`), called from `mod.rs:754-781`;
  UI `selection_ui::review_sequence`.
- **Status/report:** `iterate.rs:729-760` `emit_step_status(.., provider)`
  from `StepOutcome` ← `WrapperPromptRunner::take_provider()` (`task_run.rs:
  191,519,604`, last-writer-wins per step). `report.rs:21` names no provider.
- **Env:** `jit.rs:131` `step_env_overrides` (sets `AGENT`, `MODEL`, `YOLO`),
  called from `iterate.rs:375-386` and `phase1c.rs:229`;
  `sequence/resolve.rs:142` `StepSelection::for_document`;
  **`task_run.rs:611-630` `document_target` hands the step's planned target to
  `composition/target.rs:99-106` `resolve_execution_target`, which reuses it
  verbatim — this is why a task document's own `agent:` is ignored today.**
- **Launch rebuild:** `harness_orch/loop_control/target_launch.rs:328`
  `rebuild_launch_identity` → `:524` `select_rebuilt_provider`;
  `loop_control.rs:1397` `apply_target_env_overrides` (`:1200`, `:1689`).

#### Acceptance criteria → tests

| AC | Existing coverage | To add |
| --- | --- | --- |
| 1 `with` on tasks | `sequence/tests.rs::shell_step_rejects_prompt_only_task_option`, `preflight/tests.rs::meaningless_task_field_is_rejected`, `cli/tests/l1/sequence_errors_cli.rs::a_task_field_meaningless_for_the_executable_is_rejected`, `task/tests.rs::params_are_evaluated_just_in_time_against_the_effective_state` | rename to `with`; `params` read as `seq.params`; rejection on `side_effect`/`group`/`task`/nested `sequence`; `task:` + sibling `with` |
| 2 provenance | `cli/tests/l1/compose_caller_file_provenance.rs` (task params vs CLI origins, proxy sibling paths, caller outranks `proxy.with`, malformed/eager/lazy failures, second hop), `executor/tests/{proxy,retry}_with_evaluation.rs` atomicity | authoring-file anchoring for `proxy`/`retry`/`prep` `with` with target in another directory; `kind: task` file elsewhere; lazy/remote/ambiguous arms; forwarded anchored value; chained schema change; prep atomicity |
| 3 shape errors | `lifecycle/tests/action_shape_control.rs` (proxy rows), `tests/grammar.rs`, `tests/break_prep.rs::prep_with_follows_proxy_with_rules`, `error/tests.rs` | every row for task `with`, `retry.with`, `prep.with`, source-located |
| 4 tasks name providers | none (nearest `sequence_groups.rs`, `sequence_planned_model.rs`) | looping sequence, no doc `agent`, stub per task |
| 5 precedence | `task/tests.rs::layered_overrides_run_params_then_setters_then_mutations_then_overlay`, provenance shadow tests, `sequence_planned_model.rs`, `target_launch/tests.rs` provider-switch tests | `agent=gemini` vs `with.agent`; nested sequence's own `agent`; list hints; model env order |
| 6 early failure | `wrap_sequence_composition.rs::sequence_live_*_aborts_*` (document-level) | per-task uninstalled provider, false `when`; **invert `sequence_live_shell_only_no_agent_still_aborts_headless` (`:538`), which asserts the behavior the spec removes** |
| 7 review (L2) | `cli/tests/level2/level2_sequence_review_screen.rs` (4), L3 keys test, `sequence/review/tests.rs` | read-only own-provider rows; edited row launches |
| 8 decided at launch | none (related: `target_launch/tests.rs::a_refreshed_unavailable_scalar_agent_refuses_like_direct_selection`) | `seq.outputs`, runtime-mutated, concurrent-leg snapshot, purity stub, per-attempt identity, skipped/blocked claim none, pre-spawn failure |
| 9 schema | `lib/tests/l1/claudine_schemas.rs::{ac5_trigger_selection, sequence_container_schema_contract}`, `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`, `darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs` | `sequence.yaml` trigger, example fixtures, `max_paralel`/root `fail_fast`, malformed `sequence:`, malformed task `with`, three-level + deeper fixtures, installed bundle |
| 10 sweep | n/a | see migration inventory |

### Migration inventory

No live file under `prompts/`, `claudine/prompts/`, or `darkmatter/prompts/`
uses task `params:` (`prompts/review-loop.md` was deleted in a6134aac1). No
existing test or shipped prompt depends on target-relative `proxy.with` file
values, so the provenance change rewrites no shipped `with:` map; it is a docs
change plus new tests.

**Convert** (task-input `params:` → `with:`):

| Path | Hits | Note |
| --- | --- | --- |
| `claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md` | 18 | only remaining copy of review-loop; read by `cli/src/commands/wrap/sequence/review/tests.rs:23`; `agent` inside `params` feeds prediction, so convert despite "frozen" header |
| `claudine/cli/tests/l1/sequence_groups.rs` | 4 (`:280,284,331,488`) | |
| `claudine/cli/tests/l1/agent_text_is_data.rs` | 3 (`:165,206,834`) | |
| `claudine/cli/tests/l1/sequence_shell_templates.rs` | 3 (`:265,447,717`) | test named `sequence-lifecycle-params` |
| `claudine/cli/tests/l1/compose_caller_file_provenance.rs` | 3 (`:493,543,590`) | relative path, already authoring-relative; seed for AC2 |
| `claudine/cli/tests/level2/level2_file_resolution_capture.rs` | 1 (`:586`) | |
| `claudine/cli/tests/l1/inline_completion_lifecycle.rs` | `:871-931` | fixture + comment |
| `claudine/cli/tests/l1/sequence_errors_cli.rs` | `:133-144` | must switch to `with:` and the new error, else it passes silently once `params` is data |
| `claudine/lib/tests/l1/claudine_schemas.rs` | `:1199` | |
| `claudine/lib/src/composition/sequence/tests.rs` | 5 (`:1110-1144`) | includes shell rejection |
| `claudine/lib/src/composition/sequence/task/tests.rs` | ~20 (`:3384-4364`) | error-path strings `params.which` |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | ~8 (`:873,1526-1547,2043-2083`) | |
| `claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs` | `:178` | |
| `claudine/cli/src/commands/wrap/sequence/task_run/tests.rs` | `:86` | struct field |
| `claudine/schemas/lifecycle.yaml` | `:406,454` | `task`/`task-2` types |
| `claudine/docs/cli/sequence.md` | `:74,121,147,161` | |
| `claudine/docs/topics/flow-control/sequences.md` | `:156,160,172,331,620` | skill `topics` symlinks here — edit once |
| `claudine/docs/topics/composition.md` | `:1603` | |
| `claudine/docs/topics/flow-control/lifecycle.md` | `:618` | |
| `claudine/docs/topics/pre-flight-checks.md` | `:369` | |
| `.claude/skills/claudine/SKILL.md` | `:40` | |
| `.claude/skills/claudine/architecture.md` | `:666` | |
| `.claude/skills/claudine/cli-commands.md` | `:87` | |

**Rewrite-relative** (docs stating or implying the old `with` file rule):
`claudine/docs/topics/composition.md:1052-1059`;
`claudine/docs/topics/flow-control/flow-control-reference.md:89-175`
("Passing values with `with:`"); `claudine/docs/topics/flow-control/
flow-control.md:185,244-260,354-373` (one sentence each);
`.claude/skills/claudine/architecture.md:866-873` (states a launch-relative
rule for `proxy.with`, contradicting both old docs and the spec).

**Check, no change expected:** `prompts/implement.md:34-37,50-51` builds `log`
from `dirname(spec)`; `spec` is an eager file and should already be absolute.

**Preserve:** `.claude/skills/claudine/timeline.md:36` (evidence; add a new
entry instead), `claudine/docs/rollout-strategy.md:53` (describes the rename),
all HookAction `Bash { params }`, JSON-RPC/LSP/MCP `params`, Rust function
parameters, OpenCode `chat.params`, Darkmatter `::directive [params]`;
`proxy`/`retry`/`prep` `with:` tests whose router and target share a
directory (`compose_caller_file_provenance.rs:203,242,277`), expression
arguments (`agent_text_is_data.rs:592,627,678`), shell strings
(`level2_lifecycle_control.rs:6951`), and scalar/empty maps elsewhere.

**Amendments file.** `claudine/features/2026-09-27-sequence-improvements/
amendments-provider-and-with.md` is already gone from the worktree (removed in
adfae0873 / ea7c217ff). Phase 5's "remove the amendments file" task has
nothing to delete. The spec's sentence saying "that file is removed in this
change" is now out of date; per repository rules the spec is left as written
and this departure is recorded here.

### Bounded-expansion schema spike (Wave 2)

Run once on this macOS host in a throwaway git repository under `$TMPDIR`
(triggers are discovered only for a document inside a repository, so the
spike placed the six `claudine/schemas/*.yaml` files plus the two spike files
in that repository's root `schemas/` folder). No repository file was edited.
Validated with `target/release/md` and `target/debug/md` through
`md schema validate --format json` and `md schema triggers`.

**Working construction.** `sequence-types.yaml` (`kind: schema`) defines
`steps`/`step`/`task`/`group`, then `-2` copies with references bumped
(`task.sequence: steps-2`, `group.tasks: "task-2[](required; min(1))"`, …),
then `-3` copies whose containers are the untyped boundary
(`task-3.sequence: ["unknown[] -> untyped boundary …", "string -> a sequence
file reference"]`, `group-3.tasks: "unknown[](required; min(1))"`). Lifecycle
types are imported (`loop@./lifecycle.yaml`, `stack@…`, `initialize-stack@…`,
`cancelled-stack@…`); `steps` is `["step[] -> …", "string -> …"]`; `step` is
`["string -> …", task]`; `task` keeps `"<string>": "any -> authored step
data"`. Level-1 task and group as loaded:

```yaml
task:
    name: "string -> the step or leg name"
    when: "expression -> run this task only when true"
    with: "object -> frontmatter values for the document this task runs"
    prompt: "file -> a prompt document to compose and run"
    shell: [shell, "shell[]"]
    side_effect: "object -> one lifecycle action"
    group: [group, "string -> a kind: group file, or name@catalog-file"]
    task: "file -> an externalized kind: task file"
    sequence: steps-2
    timeout: "string -> per-command shell timeout"
    operation: string
    flow: string
    loop: "loop@./lifecycle.yaml -> only beside an inline sequence list"
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml        # also blocked/success/failure/finalize
    cancelled: cancelled-stack@./lifecycle.yaml
    "<string>": "any -> authored step data"
group:
    name: string(required)
    description: string
    variables: "object -> read as group.<variable>"
    tasks: "task-2[](required; min(1))"
    max_parallel: number(integer; min(1))
    fail_fast: "boolean -> true (default) = the legs are atomic"
    operation: string
    flow: string
    loop: loop@./lifecycle.yaml
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml        # also blocked/success/failure/finalize
    cancelled: cancelled-stack@./lifecycle.yaml
```

The trigger that loads:

```yaml
kind: trigger-schema
match:
    any:
        - sequence: string(required)
        - sequence: number(required)
        - sequence: boolean(required)
        - sequence: object(required)
        - sequence: "unknown[](required)"
$schema:
    sequence: steps@./sequence-types.yaml
    template: "object -> values merged into every step (formal sequence documents)"
    loop: loop@./lifecycle.yaml
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml
    blocked: stack@./lifecycle.yaml
    success: stack@./lifecycle.yaml
    failure: stack@./lifecycle.yaml
    cancelled: cancelled-stack@./lifecycle.yaml
    finalize: stack@./lifecycle.yaml
```

**Confirmations:**

1. No cycle rejection; `md schema triggers` reports `sequence.yaml — matched`.
2. Bare local names resolve (errors reported at typed paths three levels
   down, e.g. `/sequence/0/sequence/0/sequence/0/name`).
3. `step[]` list-of-union accepted (bare string step + task + group → valid).
4. Untyped boundary: a seven-level task/group document with unknown keys at
   level 7 is valid; a level-4 task with `name: [x]`, `execution:`, `with: [1]`
   is valid.
5. Rejections at each typed level: group `execution:` at L1, L2, L3 (anyOf
   error at `/sequence/0/group`, …); task `name: [x]`/`name: 7` at L2/L3;
   L3 `with: [1]`; group `max_parallel: 0`; `cancelled: [{retry: 2}]` at L1
   and L3; `loop: {max: 0}`. With both Claudine triggers matched, the group
   `execution:` error is still reported.
6. DMLS uses the same library roots and compile/validate path
   (`darkmatter/dmls/src/overlay/mod.rs:630` `trigger_registry` →
   `overlay/schema.rs:958` `with_trigger_registry`/`effective_for`; `md schema
   validate` → `triggers::scan_with_embedded` → the same `effective_for`).
   DMLS discovers triggers only inside a workspace folder. DMLS itself was not
   driven over stdio in the spike; Phase 4's DMLS L1 test covers it.

**Timing** (one document, wall time, three samples):

| Document | release `md` | debug `md` |
| --- | --- | --- |
| no trigger | 1.44 s | 4.86 s |
| `claudine.yaml` trigger only | 1.44 s | 5.16 s |
| `sequence.yaml` trigger, valid three-level doc | 2.19–2.22 s | 8.3–8.4 s |

The sequence trigger adds about 0.8 s (release) / 3.5 s (debug) per document;
13 documents in one `md` call took 56 s in debug (cost is per document, not
amortized).

**Findings that change Phase 4 (departures from the spec, recorded here; spec
left as written):**

- **Spec §6's `match:` does not parse.** A property-level union in a match
  condition is rejected (`property sequence in a match condition may not be a
  property-level union; use nested any: instead`), which would make the
  trigger unparseable for every document. Use the `any:` form above (same
  form as `claudine.yaml`).
- **Root `fail_fast` is not reported by this construction.** The trigger
  payload leaves the document root open, so an undeclared root key is
  accepted; group `execution`/`max_paralel` are reported because group objects
  are closed. AC9 requires root `fail_fast` to be reported by name, so Phase 4
  needs an explicit construction for it (see `message_to_agent`).
- **Number→string coercion is depth-dependent.** `name: 7` and `when: 5` are
  accepted at L1 (and `sequence: 42` coerces through the `string` arm) but
  rejected at L2/L3, following Darkmatter's documented nested-coercion limit.
  Fixtures should use non-coercible invalid values (lists, objects) when one
  test must fail at every level, and `sequence: 42` "fails payload validation"
  only if the `string` arm is not taken; Phase 4 must check what AC9's
  `sequence: 42` case actually produces and assert the real outcome.
- **Cost.** Validation-heavy L1 tests must stay in-process (or batch several
  documents per spawn) to keep under the 5 s L1 budget; one debug `md` spawn
  per fixture costs about 8 s.

### Phase 1 close

- **Behavior changed this phase:** none. Phase 1 changed no source, doc,
  schema, or skill file (only this log and the plan's checkboxes and
  frontmatter), so it added no tests. The requirement-to-test mapping for the
  later phases is the "Acceptance criteria → tests" table above. Input
  robustness matrix: not exercised this phase (no reader changed).
- **Gate decision:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: ["darkmatter"]`, `change_class: package`. The cause is
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`, which scans
  `claudine/features/` (each changed Markdown file under `claudine/features/`
  selects it, including the unrelated loops-spec edits). Because `packages`
  is non-empty, the gates ran:
  - `claudine/`: `just test` → 10321 passed, 9 skipped (196.6 s); exit 0
  - `claudine/`: `just lint` → exit 0
  - `darkmatter/`: `just test shipped_schema_corpus` → 9 passed; exit 0
- **Skipped / pre-existing failures:** none. No cross-OS check: no code
  changed, so there is no OS risk.
- **`claudine` skill:** not updated; Phase 1 changed no behavior or
  architecture.
- **Open ASK items:** none. Rules 1, 2, and 14 were resolved from the
  repository state (see "Dependency check and rulings").

## Phase 2

### Wave 3 — Rename and error family (done)

- **Rename.** `params` → `with` in `reserved.rs` (`TASK_OPTION_KEYS`, the
  `Prompt` arm of `allowed_task_options`; `params` is in neither list, so a
  step's `params:` is ordinary step data read as `seq.params`). Fields renamed:
  `PreflightTask.with`, `PromptReference.{with, deferred_with}`,
  `PromptTaskRequest.{with, with_origin_path}`; `evaluate_params` →
  `evaluate_with`. Doc comments updated in the same change (task/mod.rs,
  preflight, prewarm/sequence.rs, reapproval.rs, model.rs, CLI
  `wrap/sequence/{mod,jit,iterate,task_run}.rs`). `lifecycle.yaml`'s
  `task`/`task-2` types now declare `with` instead of `params` (Phase 4
  rewrites these types; the one-line change keeps the shipped schema honest
  meanwhile).
- **Acceptance.** `with` is allowed only on `prompt:` tasks; every other
  executable (including `task:` references) raises the existing
  `SequenceInvalidTaskField` naming `with`; a `kind: task` file carries its own.
- **Error family (Rule 2: no rename).** One shared shape check,
  `lifecycle::with_mapping_shape` (parse.rs), now backs both lifecycle
  `parse_with` and the task grammar (`grammar.rs::task_with`, run from
  `validate_task_grammar`, so inline steps, group legs, and `kind: task`
  files all get it): whole-mapping first, then not-mapping (`null` included),
  then dynamic key (`{{ }}` or `$(`). `WithOverlay::new` uses the same
  `is_dynamic_with_key`. A task value that fails at its turn raises
  `LifecycleWithEvaluationFailed` (via `resolve_with_entry`) with the task's
  source file and property; `SequenceTaskParamReserved` became
  `SequenceTaskWithReserved` (its message no longer claimed only
  `seq`/`group`/`loop`: generated step fields such as `index` are refused too).
- **Departure from the plan: one *shape* checker, not one *evaluator*.**
  Lifecycle `with` values are typed as action expressions and resolved by the
  stack executor; task `with` values are resolved by the task's
  `SubtreeCompose` against the sequence's live state (same typing rules:
  whole-span keeps type, text-with-spans is a string, YAML types kept).
  Merging the two evaluators would mean moving the task onto the lifecycle
  executor's context, which is a larger refactor than this spec asks for; the
  observable contract (errors, typing, atomicity) is shared.
- **Error rendering.** `with_property_path` joins owner and `with` path (an
  external task file's owner is empty, so the path is just `with`);
  `overlay_directive` now returns `Some(verb)` only for `proxy`/`retry`/`prep`
  and `None` for a task; `with_purpose` renders "the `proxy` to `x`" or "the
  task running `x`". The status block labels a task's error "Task input".
  `LifecycleWithUnsupported`'s doc, message, JSON projection, and hint now
  include `prep` (stale since `prep` gained `with`).
- **Pre-existing drift noticed, not changed:** static grammar errors root a
  sequence step at `sequence[N]` (the authored key), while preflight's runtime
  task diagnostics root it at `tasks[N]` (`preflight/mod.rs` root loop; the
  same path `task guard \`tasks[1].when\`` already uses). A task `with`
  shape error therefore says `sequence[0].with`, an evaluation failure says
  `tasks[0].with.x`. Unifying the runtime root on `sequence[N]` touches every
  root-step diagnostic and is out of this spec's scope.

Tests added or changed (all L1, in-process unless noted):

| Requirement | Test |
| --- | --- |
| AC1 `with` reaches the document, JIT, typed | `task::tests::prompt_tasks::with_is_evaluated_just_in_time_against_the_effective_state` (renamed) |
| AC1/Rule 4 precedence `with` < setters < mutations < overlay | `…::layered_overrides_run_with_then_setters_then_mutations_then_overlay` (renamed) |
| AC1 `params` is step data, read as `seq.params` | `sequence::tests::params_on_a_step_is_ordinary_step_data` (prompt, shell, body steps), `task::tests::prompt_tasks::a_step_params_value_is_step_data_read_as_seq_params` |
| AC1 `with` rejected on shell/side_effect/group/task/sequence | `sequence::tests::with_is_rejected_on_every_executable_but_prompt`; CLI wiring `sequence_errors_cli::a_task_field_meaningless_for_the_executable_is_rejected` (switched to `with:`); `preflight::tests::…::meaningless_task_field_is_rejected` (group leg) |
| Rule 3 `task:` + sibling `with` rejected; task file carries its own | `preflight::tests::loading::with_beside_a_task_reference_is_rejected`, `…::a_task_file_carries_its_own_with` |
| AC3 shape rows for task `with` (null, number, string, list, whole mapping, `{{ }}` key, `$( )` key, group leg), source-located; `{}` valid | `sequence::tests::task_with_shape_errors_are_the_with_family_at_the_task`; external file: `preflight::tests::loading::a_task_file_with_a_malformed_with_is_located_at_the_file`; CLI wiring `sequence_errors_cli::a_malformed_task_with_is_the_with_family_error_at_the_task` |
| `with: {}` equals no `with` | `task::tests::prompt_tasks::an_empty_with_equals_no_with` |
| AC3 evaluation failure → `LifecycleWithEvaluationFailed`, nothing launched | `task::tests::outcome_contract::a_task_value_failure_keeps_the_typed_darkmatter_cause` (`with.x` row asserts the variant and location) |
| reserved generated bindings protected | `task::tests::prompt_tasks::a_with_key_targeting_a_reserved_key_is_rejected_before_the_provider_runs` (`seq`, `loop`, `index`) |

Input-robustness matrix, `with` column: absent, `null`, whole-field wrong type,
empty, and the special rows go through the field validator above. Duplicate
key (`with:` twice, a key twice inside `with`) and trailing content are
decided by the frontmatter loader before any field is read; checked by hand
against the built CLI (`frontmatter parse failed` for all three) and owned by
Darkmatter's loader tests, so no new per-field cell. Element rows do not apply
(`with` is a mapping).

Converted fixtures/tests (`params:` → `with:`): `cli/tests/l1/{sequence_groups,
agent_text_is_data,sequence_shell_templates,compose_caller_file_provenance,
inline_completion_lifecycle,sequence_errors_cli}.rs`,
`cli/tests/level2/level2_file_resolution_capture.rs`,
`cli/tests/fixtures/frozen_prompts/prompts/review-loop.md`,
`lib/tests/l1/claudine_schemas.rs`, and the lib unit tests
(`sequence/{tests,task/tests,preflight/tests}.rs`,
`lifecycle/prewarm/sequence/tests.rs`, CLI `task_run/tests.rs`).
Scoped run: 594 + 1285 lib tests and 471 CLI L1 tests green.

### Wave 4 — Provenance (done)

**What was wrong before.** A `proxy.with` value had no origin. It was merged
into the target's authored frontmatter and also handed to compose as a data
layer, so where a relative file value resolved depended on the caller: with
no caller setters, Darkmatter's fallback treated every override (the overlay
included) as launch-area caller input; with any setter, the overlay had no
record and resolved as if the target wrote it. `prep.with` resolved from the
launch directory (it became the target's `--set`). Task `with` was already
authoring-file-relative (Phase 1 finding).

**Origin carrier.** New `composition::AnchoredOverlay` (`lib/src/composition/types.rs`):
the evaluated values plus a per-key `FileResolutionContext`; `layered` lets a
retry's value replace the proxy's with its origin; `input_records` yields
Darkmatter `CallerInputRecords` (a `null` removal carries none); `Debug` names
keys only. It replaces the bare `IndexMap` in `PrepareOptions::proxy_overlay`,
`CompositionExecutionRequest::proxy_overlay`, `HarnessPromptState::overlay`,
and every CLI pass-through (`compose/prep.rs`, `staged_boot.rs`,
`sequence/{iterate,container}.rs`).

- Anchoring sites: the compose coordinator loop, the sequence step proxy loop
  (`iterate.rs`), and the root sequence proxy (`container.rs`) anchor a
  committed handoff at the active (source) document's context; the harness's
  in-place adoption (`coordinator.rs::repoint_prompt_state`) anchors at
  `ProxyProvenance::source_path` — **not** the prompt state, which for an
  adopted handoff already points at the target (found by a failing provenance
  row: the second preparation re-anchored the overlay on the target). A
  pending `retry.with` is anchored at the retried document itself
  (`prompt.rs::active_overlay`).
- Preparation: `PrepareOptions::compose_caller_records` returns the captured
  caller records unchanged when no overlay value is anchored (no behavior
  change for the common case); otherwise `schema_caller_records` = the
  caller's records (or the launch-area fallback for `set_overrides` when there
  are none) plus each overlay record whose key no higher override layer
  supplies (caller setters, runtime mutations). Used for compose, the authored
  schema load, and both `rebase_caller_file_problems` sites.
- `prep.with`: `CliPrepRunner` sets `SharedComposeArgs::set_records` (records
  anchored at `call.file_resolution_context.for_source(call.caller)`);
  `compose::caller_input_records` uses a carried record only while its raw
  value still equals the effective one, else the launch-area record. A first
  draft used `Option<FileResolutionContext>`, which the CLI's
  context-construction guard forbids (the optional-context gate takes no
  allowance); records avoid it.
- The interactive missing-value collection (`prepare_collecting_missing`) now
  keeps existing records and adds launch records only for collected keys
  (previously it relabeled every value as launch input).
- Container `retry.with` (a sequence/group retry) writes runtime state through
  `set_batch`, not a document's frontmatter, so it has no file materialization
  and was left alone.

Tests added (all L1):

| Requirement (AC2/AC3) | Test |
| --- | --- |
| proxy.with from the router, with and without caller setters | `cli/tests/l1/with_file_provenance.rs::a_proxy_with_file_resolves_from_the_router_not_the_target_or_the_launch_directory`; in-process `lib composition::prepare::tests::an_anchored_overlay_file_value_resolves_from_its_authoring_file` |
| caller setter from launch dir outranks `with` (value and origin) | `…::a_caller_setter_still_resolves_from_the_launch_directory_and_outranks_proxy_with`; lib test above (`pinned` case) |
| retry.with from the retried document (Unix stub) | `…::a_retry_with_file_resolves_from_the_retried_document` |
| prep.with from the calling document | `…::a_prep_with_file_resolves_from_the_calling_document` |
| `kind: task` file in another directory | `…::a_task_file_with_resolves_from_the_task_file_in_another_directory` |
| proxy from inside a sequence task | `…::a_proxy_from_a_sequence_task_resolves_from_the_router` |
| eager present / eager missing | `…::an_eager_file_resolves_from_the_authoring_file`, `…::an_eager_missing_file_fails_from_the_authoring_file_and_launches_nothing` |
| lazy missing binds from origin | `…::a_lazy_missing_file_binds_from_the_authoring_file` |
| remote lazy stays URL | `…::a_lazy_remote_value_stays_a_url` |
| ambiguous `[file, string]` stays raw | `…::an_ambiguous_file_or_string_value_stays_raw` |
| forwarded anchored value keeps identity | `…::a_forwarded_anchored_value_keeps_its_identity` |
| chained handoff, middle schema `string` vs `file` | `…::a_chained_handoff_follows_each_hops_schema` |
| no partial overlay, nothing launched (proxy, prep) | `…::an_evaluation_error_installs_no_partial_overlay`, `…::a_prep_evaluation_error_calls_no_target` |
| layering keeps origin with value; null has none; Debug redacts | `lib composition::prepare::tests::anchored_overlays_layer_value_and_origin_together` |
| AC3 shape rows for proxy/retry/prep, located | `lib composition::lifecycle::tests::action_shape_control::proxy_retry_and_prep_share_the_with_shape_errors` |

Regression proof: with the overlay records and `set_records` disabled, six
rows fail (proxy, retry, prep, lazy-missing, forwarded, chained) and the lib
test reports `ROUTE=launch`; the other rows are controls for unchanged rules.

Changed existing test: `harness_orch/loop_control/tests/overlay_layering.rs::a_file_valued_overlay_property_resolves_through_the_targets_own_context`
pinned the old launch-area rule. It is now
`a_file_valued_overlay_property_resolves_from_the_router_that_authored_it`,
with decoys in the target's directory and the launch area and a bare
(`spec.md`) reference. The other harness unit tests moved to the new type
(`overlay.values().get`, `AnchoredOverlay::new`).

**Docs and skill (done in this phase, because the behavior changed now).**
`docs/topics/flow-control/sequences.md` (new "Task inputs (`with:`)" section,
the reserved-key list, `params` as step data), `flow-control-reference.md`
(new "Relative files resolve from the file that wrote them", with the
intentional-change note), `composition.md` (the one-rule table in "Caller
File Provenance and Materialization"), `lifecycle.md`, `pre-flight-checks.md`,
`docs/cli/sequence.md` (`params` → `with` only; its provider section is
Phase 3/5's). Skill: `SKILL.md`, `architecture.md` (provenance paragraph
rewritten; it stated a launch-relative `proxy.with` rule), `cli-commands.md`,
`timeline.md` (new entry). Phase 5's docs task still owns the provider
prediction text and a final review of these pages.

### Phase 2 close

**Follow-ups noticed, not changed:**

- An eager-missing file error says `did not match a file from \`<launch dir>\``
  for a `with` value, because Darkmatter's no-match text names the origin
  context's `base_dir`/`cwd` (the launch directory) even when a `./` reference
  is resolved from the context's `source_path`. Resolution is correct (the
  eager-present and lazy rows prove the directory); only the wording names the
  wrong base. Same for task `with`. `claudine/lib/src/composition/schema/supplied.rs::caller_no_match_reason`
  uses `origin.cwd()` the same way.
- A body step (no executable) still drops task options silently, now `with:`
  as it did `params:` (the spec keeps body-step option rules).
- Root-step task diagnostics use `tasks[N]`, static grammar uses `sequence[N]`
  (see Wave 3).
- The LifecycleEvaluationError status block for a `proxy`/`prep` overlay
  failure names `initialize[0].proxy` and the reason, not the nested
  `with.<key>` path (pre-existing rendering of `LifecycleErrorInfo`).

**Robustness smell grep** (`#[serde(default)]`, `Option<T>`,
`unwrap_or_default()`, `filter_map(.. as_str())`, `.ok()` on the changed
parses): `option_map(options, "with")` maps a non-object to an empty map, but
every path that reaches it (inline steps, legs, `kind: task` files) has passed
`grammar::task_with` first, so a non-mapping is already an error;
`deferred_with … unwrap_or_default()` in `run_primary` is pre-existing and
means "no reference recorded → nothing deferred"; the coordinator's
`derive_source(..).ok()` falls back to `for_source` on the same path. No other
hit on the changed `with` parses.

**Gates (macOS host, load average 55–75 from other sessions):**

| Gate | Result |
| --- | --- |
| scope: `git status … \| xargs python3 scripts/ci/affected_scope.py` | `packages: [claudine, claudine-cli, test-toolkit, darkmatter, darkmatter-cli, dmls]`, `change_class: package` → gates required |
| `claudine/`: `just lint` | exit 0 |
| `claudine/`: `just test` | 10347 passed, 9 skipped, 30 slow (254.8 s); exit 0. A first run failed one test, `context_construction_guard::production_source_builds_contexts_only_through_the_builder`, on the `Option<FileResolutionContext>` draft; fixed (see Wave 4) and rerun green |
| `just check-tier-coverage claudine` | 0 stranded |
| darkmatter (reads `claudine/schemas`, docs, skills): `cargo nextest run -p darkmatter -p darkmatter-cli -p dmls -E 'test(/claudine_schema_bundle\|schema_validate\|shipped_schema_corpus\|claudine_schema_discovery\|base_schema_end_to_end\|lsp_session\|compose_transclusion\|ambient_ctx_capture/)'` | 302 passed |
| test-toolkit (selected by changed test files): layout/nextest-config tests | 17 passed. The `tools/test-audit` Vitest suite was **skipped**: this worktree has no `node_modules` and installing needs the network; CI runs it |
| `claudine-contract` (reverse dependency) | inside `just test` |
| Windows cross-check (`./scripts/cross-check.sh <pkg> --os windows <filters>`; `just cross-check` mangles a parenthesized `-E` filter) | `claudine` filtered to `composition::{prepare,sequence,lifecycle::tests::action_shape,error}`: 836 passed; `claudine-cli` filtered to the provenance, overlay, sequence-error, prep, guard, and group tests: 123 passed (13 of 14 provenance rows; the retry row is Unix-only) |

The 30 slow tests (13 at baseline) are existing schema-matrix and CLI tests,
none added here; the host was under load from other sessions. No test failed
on rerun.

## Phase 3

### Wave 5 — Prediction core (done)

- **Where the prediction lives.** `lib/src/composition/sequence/preflight/providers.rs`.
  `Loader::load_task` records, for every `prompt:` task, a `ProviderPrediction`
  (`agent` and `model`, each a `HintState`: `Absent`, `Static`, `DecidedAtLaunch`
  with a `LaunchDecision`, `Failed` with a typed `LifecycleCause`, or `Circular`)
  on the new `PreflightTask.provider`. Layers, lowest first: the prompt
  document's frontmatter, the task's `with` (reusing the evaluated/deferred split
  `record_prompt_reference` already computes, which now returns it), the caller
  setters. Runtime `set` is not predicted: the launch rebuild stays
  authoritative.
- **Classification.** A value is *decided at launch* when it (or any key it
  reads, transitively, through `derived_deferred_keys`) reads `seq`, `loop`,
  `group`, or `current`, a deferred `with` key, a `$( … )`, or a key any graph
  `set` may write. Written keys are collected over-inclusively (any `set:`
  mapping in the root and nested sequence frontmatter, every prompt document's
  frontmatter, inline container events, group maps, side-effect actions) and
  applied in `Loader::finish` (`ProviderPrediction::defer_runtime_written`),
  because the whole graph has to be known first. A read of `env.AGENT`,
  `env.MODEL`, `ctx.agent`, or `ctx.model` (the existing
  `first_target_identity_root`) is `Circular`. Only a value with `{{` that is
  none of these is evaluated (`reapproval::compose_approval_value`, the
  shell-approval early-binding scope).
- **Identity.** `PreflightTask.identity: TaskIdentity` (`tasks[N]`,
  `.sequence[N]`, `.group.tasks[N]`), assigned in `Loader::finish`, carried on
  `PromptTaskRequest.task_identity`; a nested sequence's body task gets
  `sequence_child(i)`. JIT re-preparation keeps the frozen children, so
  identities survive. The runtime convention (`tasks[N]`) was followed, as the
  Phase 2 message advised.
- **Purity.** Prediction runs no lifecycle action, `prep`, shell, or body
  composition, and the CLI resolves models with `refresh: false`.
  `prediction_calls_the_evaluator_only_for_an_evaluable_template` drives
  `predict` with an evaluator that panics; `a_shell_substitution_is_never_evaluated_at_preflight`
  asserts a `$(touch …)` marker is never created.
- **Typed error.** `CompositionError::SequenceTaskProviderSelection { task,
  task_path, hint_path, property, containers, problem: Box<SequenceTaskProviderProblem> }`
  with `Unresolvable { state, installed }`, `WrongType { key, found }`,
  `Evaluation(LifecycleCause)` (a `#[source]`), `Circular(variable)`; rendered in
  `error/render/selection.rs` (header "task provider selection failed"; body
  reuses `render_agent_resolution_failed_body`). Boxed because the state would
  widen every `CompositionError`.

### Wave 6 — Gate, review, launch, reporting (done)

- **Gate** (`wrap/sequence/mod.rs`): the old document-level classification,
  drafts, and `dry_run_sequence_target` are gone. `providers::collect_sites`
  walks the graph (prompt tasks at any depth, body steps), `providers::select`
  resolves each with the canonical non-interactive rule (`--<provider>` decides
  all), and only an unresolved site gates: no terminal → `AgentResolutionFailed`
  for the sequence document (or `SequenceTaskProviderSelection` for a task's own
  ambiguous list), terminal → review. A static unusable task hint fails before
  any task starts, `when: false` included. A body step's hints are the sequence
  document's, so an invalid root `agent` keeps its old review/abort behavior.
  `--dry-run` renders an unusable task hint instead of failing (preview rule).
- **Fallbacks.** A task with no `agent` takes the nearest enclosing sequence
  document's (`PreflightSequence.frontmatter` for a referenced file, then
  outward); its own `model` stays above the document's.
- **Review screen** (`review.rs` rewritten; `selection_ui.rs`): one draft per
  site; `SequenceStepDraft.read_only: Option<String>` marks own-provider
  (`Codex`) and decided-at-launch rows, which render as static cells and return
  no target. Choices map back by row order to `TaskIdentity` and are stored as
  `TaskTarget::Fallback`. Rows are labeled with the root step position and the
  task's name (a group leg shows its leg name, so the L2 fixture's row
  `5 build` is now `5 member`).
- **biscuit-tui** (needed for read-only rows): `InputTableState::try_new`
  accepts a `CellValue::StaticText` row value in a column of any type and pins
  that one cell read-only. Documented in `biscuit-tui/docs/components/input_table.md`.
- **Launch** (`task_run.rs`): `WrapperPromptRunner::document_target` prepares
  each task under its own selection's target (setting `AGENT` and `MODEL`, or
  removing both); `launch_target` resolves a decided-at-launch task from its
  composed hints and fails with `SequenceTaskProviderSelection` before spawn.
  `StepSelection::for_document` now takes the hints its model is re-selected
  from (the task's own). The launch rebuild is unchanged and still wins.
- **Reporting.** `LaunchLog` (lib `types.rs`) on `CompositionExecutionRequest`
  and `StepScope`; `HarnessPromptState.launched_providers` records each
  attempt whose child spawned (`loop_control.rs`), and `runner.rs` copies it into
  the log. `StepOutcome::Succeeded.providers` replaces the planned provider;
  status lines print `via <final>` plus `; attempts: A → B` when they differ. A
  preview, a skip, and a shell task claim none (dry-run steps now say
  `rendered (dry-run)` instead of `succeeded (via …)`). `--dry-run` (and
  `--verbose` live) lists `Providers:` rows (`name → Codex`,
  `→ Claude (from the sequence)`, `→ decided at launch`).

### Departures and rulings

- **Null and empty hints follow the canonical parser.** The plan's matrix
  wanted `agent: null` and `agent: []` to be typed errors. Rule 11 ("one
  resolver") wins: the canonical `parse_selection_hints_from_frontmatter`
  reads both as no agent, so such a task is *missing a provider* (fallback or
  gate), exactly as a directly composed document would be. Wrong-type whole
  fields and elements (`7`, `[codex, 7]`, `[7]`) are typed errors naming the
  property (`invalid_hint_shapes_fail_as_the_task`).
- **Stub providers.** Codex and OpenCode stubs hang under
  `common::launch_recorder` (OpenCode keeps stdin inherited, Codex waits on
  its structured protocol), so the L1 tests launch Claude and Goose stubs and
  use Gemini as the uninstalled provider. The behavior under test is
  provider-agnostic.
- **Model catalog refresh dropped for sequences.** The old planned-target path
  refreshed a provider's listing once (cache warming for the drift channel);
  prediction must not contact providers, so per-task targets resolve with
  `refresh: false` and still warn once per provider.
- **Incident, fixed.** One `python3` edit ran with the shell's cwd reset to
  the main checkout and ticked three Wave 5 boxes in
  `/Volumes/coding/personal/rusty-biscuit/claudine/features/2026-10-26-task-inputs-and-providers/plan.md`.
  Those three lines were restored to `[ ]`; that file's other pre-existing
  edits (its Wave 4 boxes) were not this session's and were left untouched.

### Tests added or changed

| Requirement | Test |
| --- | --- |
| Each prediction state, layer, property | lib `preflight::providers::tests::{a_literal_document_agent_is_predicted_from_the_document, a_task_with_agent_outranks_the_document_and_names_its_property, a_caller_setter_outranks_the_task_with, a_document_agent_template_over_static_values_is_evaluated, a_task_without_an_agent_is_missing_a_provider_but_keeps_its_model}` |
| AC8 decided at launch: late value, runtime write, shell | `…::{a_hint_reading_a_later_output_is_decided_at_launch, a_hint_reading_a_key_the_graph_writes_at_runtime_is_decided_at_launch, a_shell_substitution_is_never_evaluated_at_preflight}` |
| Circular `env.AGENT`/`env.MODEL`, evaluation failure | `…::{a_hint_reading_its_own_identity_is_circular, an_otherwise_static_hint_that_fails_to_evaluate_is_a_failure}` |
| AC8 purity (evaluator stub) | `…::prediction_calls_the_evaluator_only_for_an_evaluable_template` |
| Identity + ancestry | `…::every_task_gets_a_distinct_graph_identity` |
| Selection classes, fallbacks, nested file `agent`, flag, list hints | CLI `sequence::review::tests::{tasks_that_name_their_providers_need_no_document_agent, a_task_without_a_hint_takes_the_nearest_enclosing_sequence_agent, a_task_missing_a_provider_with_no_enclosing_agent_is_unresolved, a_hint_decided_at_launch_keeps_the_enclosing_agent_only_as_a_fallback, an_explicit_provider_flag_decides_every_task, list_hints_follow_the_canonical_installed_provider_rule, a_body_step_uses_the_sequence_documents_agent}` |
| AC6 early failure, shape matrix, circular | `…::{an_uninstalled_provider_fails_preflight_even_behind_a_false_when, invalid_hint_shapes_fail_as_the_task, a_hint_reading_its_own_identity_fails_as_circular}` |
| Review mapping by identity, read-only rows, cancel, count mismatch | `…::{review_edits_only_missing_tasks_and_keys_choices_by_identity, leaving_the_review_starts_nothing_and_a_wrong_row_count_is_an_error}`; `selection_ui::tests::read_only_drafts_render_static_provider_and_model_cells` |
| biscuit-tui static cell | `input_table::table::tests::a_static_row_value_pins_one_cell_of_an_editable_column_read_only` |
| AC4 (looping, no doc `agent`, own prompts, status lines) | CLI L1 `sequence_task_providers::tasks_name_their_providers_in_a_looping_sequence_with_no_document_agent` |
| AC5 caller setter over `with`, nested file `agent`, model drop/stale `MODEL`, runtime `set` | `…::{a_caller_agent_outranks_with_and_a_nested_sequence_agent_applies_to_its_tasks, a_provider_switch_drops_the_inherited_model, a_task_agent_read_from_a_runtime_write_is_decided_at_launch}` |
| AC6 uninstalled + false `when`, no launch; shell/side-effect-only | `…::{an_uninstalled_task_provider_fails_before_any_task_launches, a_shell_and_side_effect_graph_needs_no_provider}`; `wrap_sequence_composition::sequence_live_shell_only_no_agent_runs_headless` (inverted from `…_still_aborts_headless`) |
| AC8 `seq.outputs`, skipped claims none, concurrent-leg snapshot, unavailable dynamic hint pre-spawn, retry attempt identities | `…::{a_task_agent_read_from_an_earlier_output_is_decided_at_launch, a_concurrent_leg_hint_reads_the_group_start_snapshot, an_unavailable_dynamic_hint_fails_before_its_task_spawns, a_retry_that_switches_provider_reports_every_attempt}` |
| Dry-run rows | `…::a_dry_run_lists_each_tasks_predicted_provider` |
| AC7 (L2) read-only row, editable choice launches | `level2_sequence_review_screen::level2_tmux_review_screen_shows_own_provider_tasks_read_only` (fixture `ReviewFixture::stage_with`) |

Not covered: a `proxy` provider switch's attempt identities inside a sequence
(the attempt log is shared by every document of the run, and the retry row
proves the chain rendering), and a "blocked" task (no launch → empty log, the
same code path as the skipped row).

**Robustness smell grep** (changed parses): `providers.rs` uses
`.unwrap_or_default()` only for a prompt document missing from the graph
(impossible after `load_prompt`) and `.ok()` on a nested sequence's hints,
which makes an unparsable enclosing `agent` "does not resolve" (the gate then
applies) rather than a silent pick; no `#[serde(default)]`. `null` agent is
absent by the canonical parser's definition (see Departures).

### Gates (macOS host)

| Gate | Result |
| --- | --- |
| scope: `git status … \| xargs python3 scripts/ci/affected_scope.py` | `packages: [biscuit-tui, claudine, claudine-cli, test-toolkit, darkmatter, darkmatter-cli, dmls]`, `change_class: package` (Phase 2 is still uncommitted in this tree) → gates required |
| `claudine/`: `just lint` | exit 0 (after allowing `result_large_err` on the new selection functions, as the area does elsewhere, and keeping typed causes for the transport guard) |
| `claudine/`: `just test` | 10366 passed, 9 skipped, 20 slow; exit 0. A first run failed `dispatch_inventory_matches_committed_file`: the new sites were all in test files (`review/tests.rs`, `sequence/tests.rs`; production `review.rs` lost one), so the inventory was regenerated with `CLAUDINE_UPDATE_INVENTORY=1` |
| `claudine/`: `just test-l2 sequence_review_screen` | 5 passed |
| `just check-tier-coverage claudine` | 0 stranded |
| `biscuit-tui/`: `just test` / `just lint` | 1013 passed / exit 0 |
| darkmatter (reads claudine docs, schemas, skills) | the Phase 2 filter: 302 passed; `schema_roots`, `schema_phase_validation`: 40 passed |
| test-toolkit layout/nextest tests | 20 passed |
| Windows cross-check (`./scripts/cross-check.sh`) | `claudine` `preflight::providers`: 12 passed; `claudine-cli` `sequence::review selection_ui sequence::tests`: 42 passed; `biscuit-tui` `input_table`: 96 passed. The L1 CLI tests are Unix-only (shell stubs) |

**Trap noted:** running `cargo nextest -p claudine-cli` without
`--features test-fixtures` replaces the fixture-enabled `claudine` binary, and
`document_environment_routes` then fails (`captures=0`). Use `just test` or pass
the feature.

## Phase 4

### Wave 7 — Schema authoring (done)

- **`claudine/schemas/sequence-types.yaml`** (new, `kind: schema`): `steps`,
  `step`, `task`, `group`, unrolled into three typed container levels (`-2`,
  `-3` copies with bare local references); the third level's nested
  `sequence:` and `tasks:` are `unknown[]` (the untyped boundary). Field
  types are the ones `lifecycle.yaml`'s container types already shipped (they
  were tuned to the parser), not the spec sketch where the two differ:
  `prompt` stays `string` (a `file` arm rejects `{{ … }}` and `::operator`
  values, measured), `shell`/`side_effect`/`task` stay parser-checked
  `unknown`, and `step` keeps its `number`/`boolean` arms (the parser's lenient
  scalar step). The task object is open (`<string>: unknown`, step data); the
  group object is closed.
- **`claudine/schemas/sequence.yaml`** (new, `kind: trigger-schema`): the
  five `sequence: <type>(required)` arms under `any:` (the spec's
  property-level union is unparseable, per the Phase 1 spike); payload
  `sequence: steps@./sequence-types.yaml`, `template: object`, and
  `fail_fast: null` so a root `fail_fast` is reported at `/fail_fast`.
- **`lifecycle.yaml`**: `cancelled-stack` already existed (dependency spec),
  so "Lifecycle additions" needed no new type. The container types
  (`step`/`task`/`group`/`-2`) **moved out** to `sequence-types.yaml` so there
  is one definition, and the header comment now points there.
- **`claudine.yaml`**: the `sequence` declaration was **removed**, not
  upgraded (see departures). The header comment says `sequence.yaml` is its
  only declaration.
- **Embedding**: `bundle.rs` `CLAUDINE_SCHEMA_FILES` gains both files
  (alphabetical, matching the byte-for-byte test) and `CLAUDINE_TRIGGER_FILES`
  gains `sequence.yaml`, so `is_claudine_owned` treats `sequence`, `template`,
  and `fail_fast` as Claudine controls (never dropped by cleanup). Disk
  discovery needs no code: it reads every trigger in the schema roots.
  Install docs now copy `claudine/schemas/*.yaml`.

#### Cost measurements (the reason for the departures)

Every trigger payload is loaded whenever trigger discovery runs, whether or
not it matches, so a payload's size is paid by **every** Claudine launch and
every `md`/DMLS validation (plain `prompt:` + `start:` documents included).
Wall time of one `md schema validate` of a plain prompt document, this host
(load 10–45 from other sessions; three samples agreed within 0.02 s):

| Construction | release `md` | debug `md` |
| --- | --- | --- |
| before this phase (two container levels, in `claudine.yaml`) | 0.20 s | 0.79 s |
| spec as sketched: container events typed with `stack`, container `loop` typed, three levels, `sequence` in both triggers | 3.27 s | — |
| events loose, `cancelled` typed at all three levels, both triggers | 0.48 s | 1.95 s |
| `cancelled` typed on levels 1–2 only, both triggers | 0.29 s | 1.23 s |
| … and `sequence` only in `sequence.yaml` | 0.24 s | 0.89 s |
| … and no root events in `sequence.yaml` | — | 0.70 s |
| … and the shared `loop` type on level-1 containers (**shipped**) | — | 0.76 s |
| shared `loop` type on levels 1–2 | — | 0.91 s |
| level-1 container events typed with `stack` | — | 1.28 s |

Each container level inlines the next twice (a task's `sequence:` and its
group's `tasks:`), so a container field's cost grows about fourfold per level.
The shipped construction is slightly cheaper than the bundle before this phase.

### Departures and rulings (spec left as written)

1. **Container events stay shape-only; `cancelled` is typed on levels 1–2;
   the shared `loop` type is used on level 1 only** (an object below). The
   spec asks for each event "typed with the event type"; doing so costs about
   3 s per launch (table above). This keeps the contract the
   sequence-containers spec ratified (`grandchild_cancelled_is_left_to_the_task_parser`
   still holds) and reuses the shared `loop` type rather than forking one
   (Rule 13).
2. **`claudine.yaml` no longer declares `sequence`, and `sequence.yaml`
   declares no root events or `loop`.** The spec asks for identical shared
   declarations in both triggers. Every document with a non-null `sequence`
   selects `sequence.yaml`, and every document with an event or `loop` selects
   `claudine.yaml`, so each key has exactly one declaration and no payload can
   weaken another; the duplicate would cost about 0.34 s per debug process.
   Consequence: in the editor, a sequence document with no event does not get
   completion for event keys until `claudine.yaml` applies.
3. **`sequence: 42` activates the trigger but passes the payload.**
   Darkmatter coerces a top-level number or boolean to the file-reference
   `string` arm (documented default-on coercion; no type both rejects a native
   number and accepts every reference form). The sequence parser rejects it
   (`SequenceInvalid`), which the test asserts. A mapping fails the payload.
4. **"Reported by name" for a group key** is the union problem at
   `/…/group`, whose message carries the group's value and so the key
   (`"execution"`, `"max_paralel"`): a group value is `group | string`, and
   Darkmatter reports a non-discriminated union as one `anyOf` problem.
   Narrowing that report is a Darkmatter change, out of scope here.
5. **Root `fail_fast: null`** passes the schema (null reads as absent); the
   parser still rejects any present `fail_fast`.
6. **The launch verdict now judges every sequence document.** A sequence
   document with no other control previously selected no trigger; it now
   selects `sequence.yaml`, so `claudine sequence` rejects a malformed task
   field before any step launches. The full CLI sequence/schema subset
   (1210 tests) passed against it with no fixture change.

### Wave 8 — Fixtures and tests (done)

Fixtures: `claudine/lib/tests/fixtures/sequence-schema/` (24 files). The
eight `example-*` files are every sequence example of
`2026-10-24-sequence-containers` and `2026-10-25-loops-and-flow-control`
(the sibling-cancellation and teardown-migration fragments wrapped as a
step; the review loop given the `spec`/`agent` values its own `$schema`
requires). The rest are one edit per cell: valid and invalid at each typed
level, a deeper-than-typed document, group `execution`, `max_paralel`, root
`fail_fast`, step data, and the trigger-activation shapes.

| AC9 requirement | Test (boundary, tier) |
| --- | --- |
| `md schema triggers` picks up `sequence.yaml` with no `$schema:` | `darkmatter/cli/tests/l1/claudine_schema_bundle.rs::sequence_documents_select_the_sequence_trigger_and_validate_through_the_cli` (spawned `md`, L1); `lib/tests/l1/sequence_schema.rs::sequence_trigger_activation_follows_the_value_shape` (registry, in-process) |
| every container/loop example is valid (md, DMLS) | `sequence_schema::every_sequence_example_of_the_container_and_loop_specs_is_valid` (all eight, disk discovery = DMLS/`md` path); md CLI test (one example); `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs::sequence_documents_are_typed_by_the_sequence_trigger` (LSP session) |
| `execution`, `max_paralel`, root `fail_fast` reported by name | `sequence_schema::removed_and_misspelled_keys_are_reported_by_name` (+ parser rejects root `fail_fast`); md CLI test; DMLS `removed_sequence_keys_are_reported_at_their_paths`; embedded payload test |
| `sequence: 42`/mapping activates; null/missing does not | `sequence_trigger_activation_follows_the_value_shape` (mapping invalid at `/sequence`; 42 selected, schema-valid by coercion, parser `SequenceInvalid`; `[]` valid; one bad item and every bad item invalid at their indexes; null and absent not selected) |
| both triggers active, malformed task `with` still fails | `sequence_schema::malformed_task_with_fails_when_both_claudine_triggers_apply` |
| installed launch resolves embedded imports, no checkout | `claudine_schemas_embedded::{embedded_files_are_the_repository_files_byte_for_byte, embedded_payloads_apply_without_disk_schemas, embedded_sequence_types_exports_resolve_without_disk_schemas}` (no disk schema anywhere); `cli/tests/l1/launch_trigger_schemas.rs::a_sequence_document_is_judged_by_the_embedded_sequence_trigger_before_any_step` (spawned `claudine sequence`: control launches one step; `operation: [review]` fails at `/sequence/0/operation`, nothing launches) |
| valid + invalid at each of three typed levels; deeper valid | `sequence_schema::each_typed_container_level_validates_and_deeper_levels_are_untyped` |
| unknown step key is step data | `sequence_schema::undeclared_step_keys_are_step_data`; DMLS test |
| fixtures all named | `sequence_schema::sequence_schema_fixtures_are_all_named` |

Changed existing tests (the bundle grew, the container types moved):
`claudine_schemas.rs` (`shipped_bundle!` gains both files; imported types are
`@../schemas/sequence-types.yaml`; three triggers; `sequence-types` exports
resolve; `fail_fast` is declared exactly when `sequence` is present; level-1
container loops are typed with `loop`, deeper ones are objects;
`Discovered::selects` added), `claudine_schemas_embedded.rs` (seven files,
three triggers, sequence payload section, exports probes),
`cli/src/completion/schema_completion/tests.rs` (`task@./sequence-types.yaml`),
`darkmatter/cli/tests/l1/claudine_schema_bundle.rs` and
`darkmatter/dmls/tests/l1/claudine_schema_discovery.rs` (seven-file bundles),
`darkmatter/lib/tests/l1/shipped_schema_corpus.rs` (`sequence.yaml` is a
trigger; core-type users).

**Speed.** The first DMLS test opened five documents in one session (7.4 s
alone at load 57); it was split into two tests of two documents each
(about 1.3 s). The embedded exports test probed all twelve
`sequence-types` exports (4.4 s under suite load); it now probes the first
level and the untyped boundary (1.6 s), the middle levels being judged in
`sequence_schema.rs`.

**Input robustness matrix (trigger column).** Absent → not selected; null →
not selected; whole-field wrong type: mapping → selected and invalid, number
→ selected, coerced (departure 3), parser rejects; one bad element and every
bad element → invalid at each index; empty `[]` → selected and valid.
Duplicate keys and trailing content are decided by Darkmatter's frontmatter
loader, which this phase did not change (not re-tested here). No Rust
reader changed this phase, so the smell grep has nothing to inspect: the
changes are schema files, the two embedding lists, and tests.

### Gates (macOS host)

| Gate | Result |
| --- | --- |
| scope: `git status … \| xargs python3 scripts/ci/affected_scope.py` | `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`, `change_class: package` → gates required |
| `claudine/`: `just test` | 10375 passed, 9 skipped, 44 slow (308 s, host load 40–86 from other sessions); exit 0 |
| `claudine/`: `just lint` | first run failed `clippy::result_large_err` on a test helper returning `Result<(), CompositionError>`; it now returns `Option<CompositionError>`; rerun exit 0 |
| `just check-tier-coverage claudine` | 0 stranded |
| `claudine-cli` sequence/schema/launch subset (before the gate) | 1210 tests; one failure (`shipped_container_schema_resolves_for_authored_parameter_completion`, still importing `task@./lifecycle.yaml`), fixed |
| `darkmatter/`: `just test schema` | 2264 passed; exit 0 (DMLS test then split and rerun: 11 passed) |
| `darkmatter/`: `just lint` | exit 0 |
| `test-toolkit` (whole package, 478 tests) | 476 passed, 2 failed, both **pre-existing and unrelated**: `just_commit_is_deterministic_under_ci` (root `justfile` `commit` recipe has no CI branch) and `the_ci_documentation_states_the_implemented_behavior` (`.claude/skills/os/ci-evidence.md` lacks `BISCUIT_CI_CONSTRAINTS_DIR`); neither file is changed in this tree |
| cross-OS | not run: the change is schema YAML, two `include_str!` lists, and tests over temp directories; no path comparison or OS-specific code |

`just test-l2` was not run: this phase changed no review-screen code.

### Phase 4 close

- `claudine` skill updated (`SKILL.md` container-grammar paragraph,
  `timeline.md` entry).
- Spec unchanged; departures are recorded above and the docs state the
  shipped behavior (`docs/topics/schemas.md`, `schemas/README.md`,
  `docs/topics/composition.md`).

## Phase 5

### Wave 9 — Sweep and documentation (done)

- **Sweep.** Phase 2 already converted every task-input `params:` in the
  migration inventory. A final grep over `claudine/`, `prompts/`,
  `claudine/prompts/`, `darkmatter/prompts/`, and `.claude/skills/`
  (excluding `features/`, `fixes/`, `docs/research/`, `stream/`) finds only
  intended uses: Rust struct fields (`HookAction`, lifecycle signatures),
  JSON-RPC `params`, the `step-data.md` schema fixture and the
  `params_on_a_step_is_ordinary_step_data` tests (step data on purpose), and
  `docs/rollout-strategy.md:53` (preserved; describes the rename). The frozen
  `review-loop.md` copy uses `with:` (17 hits, no `params`). No
  `task@./lifecycle.yaml` or `step[]@./lifecycle.yaml` import remains. One
  stale count was fixed: the skill's Reference Documents line said "the five
  `claudine/schemas/` files"; it now says seven.
- **Sweep, user-facing text.** `CompositionError::SequenceShellLateBinding`'s
  message told authors a sequence shell command may read "`state`, `params`,
  `fm.*` …". Neither `state` nor `params` is a root. It now names "step values,
  a task's `with`, `fm.*`, `ctx.*`, `env.*`" (matching `sequences.md`). Test:
  `preflight::tests::shell::lifecycle_late_binding_in_a_shell_command_is_rejected`
  now asserts the message names `` a task's `with` `` and no longer names
  `` `params` ``.
- **Docs.** The pages the plan names were rewritten in Phases 2–4 (the log
  records which). This phase's review found them complete, with a Mermaid
  diagram for the provider decision flow in `docs/cli/sequence.md`, the
  intentional `proxy.with` change in `flow-control-reference.md`, and no
  page naming this feature. Two drifted sentences were corrected:
  - `docs/topics/flow-control/sequences.md` ("Provider and model resolve once
    per task"): said "the provider stands as planned" for every task; a task
    *decided at launch* resolves its provider from its composed document just
    before it spawns.
  - `docs/topics/composition.md:1575`: "A sequence step's plan is its reviewed
    or planned target" now names the per-task selection (own prediction,
    enclosing sequence document's `agent`, or a review-screen choice).
- **Skills and amendments.** `SKILL.md` count fixed (above). The amendments
  file `2026-09-27-sequence-improvements/amendments-provider-and-with.md` does
  not exist anywhere under `claudine/features/` (verified with `find`; Phase 1
  recorded its removal), so there was nothing to delete. No crate was added
  or removed by any phase of this feature (no `Cargo.toml` in its commits or
  the working tree's changes for it), so `docs/dependencies.md` is unchanged.
- **Plan housekeeping.** The Phase 2 "Wave 4 — Provenance" boxes were still
  `[ ]` (Phase 4's note). The Wave 4 log section records each item done, with
  its tests (`cli/tests/l1/with_file_provenance.rs` exists, 14 rows) and Phase 2's
  gates, so they were ticked here without re-running the work.

### Comment drift pass (done)

Reviewed the `///`/`//!`/`//` comments of every non-test source file touched
by the feature commits (52938048d, 02b623da2) and the Phase 4 working tree.
Drift found (comment was wrong; code kept), all fixed in comment-only edits
(`git diff -U0` shows no non-comment line):

| Location | Comment said | Code does |
| --- | --- | --- |
| `lib/.../sequence/preflight/shape.rs` `PreflightTask::with` | leftover first line: values are "passed … as user setters" | `with` is a data layer *below* user setters (line deleted) |
| `lib/.../composition/types.rs` `SequenceStepDraft` | one draft per step, choices map back by `step_index` | one draft per provider-launching task; choices map by editable-row order; `step_index` is a label |
| `cli/.../compose/prep.rs` `StepScope` | "a task's evaluated `params`" | `with` |
| `cli/.../compose/prep.rs:310` | rebuilt caller records stay anchored at the launch origin | a `prep` overlay value keeps its own record (anchored at the calling document) |
| `cli/.../wrap/sequence/task_run.rs` `last_run` | "perf record and provider" | `PromptRunRecord` has no provider field now |
| `cli/.../wrap/sequence/review.rs` module doc | only a task with no own provider is editable | a task taking the sequence's fallback is read-only too; only an unresolved task is editable |
| `cli/.../wrap/selection_ui.rs` `review_sequence` | "only the steps offered for review" | drafts are per task |
| `cli/.../wrap/selection_ui.rs` `determine_unified_provider` | "all drafts / every row" | read-only drafts are filtered out first |
| `cli/.../loop_control/target_launch.rs` `fallback_model`, `planned_fallback_model` | a step's model is "planned from the sequence document" / "review screen or the sequence document" | preflight plans it per task, from the task's own hints or the sequence document's |

Reviewed with no drift: the remaining lib files of the feature (`prepare.rs`,
`lifecycle/{actions,parse,mod}.rs`, `prewarm/sequence.rs`,
`schema/{bundle,mod,translate}.rs`, `sequence/{grammar,model,reserved}.rs`,
`sequence/task/{mod,sequence}.rs`, `preflight/{mod,providers}.rs`,
`error/mod.rs`), the CLI files (`compose/mod.rs`, `prep_target.rs`,
`sequence.rs`, `wrap/composition/{runner,staged_boot,target}.rs`,
`harness_orch/{prompt,types,loop_control,coordinator}.rs`,
`wrap/sequence/{mod,iterate,resolve,container,jit,providers}.rs`), and
`biscuit-tui` `input_table/table.rs`.

### Follow-ups for review (found, not changed)

1. **Possible stale model on a sequence task's retry (unverified).**
   `planned_fallback_model(target, sequence = true)` carries a sequence task's
   model whatever its reason. This predates the feature, and was right when a
   step's model came from the *sequence* document. Since Phase 3 a task's
   target model can come from its own prompt document (`FrontmatterSingle`),
   which the rebuild re-derives on every fresh read. If that document drops
   `model:` between attempts, the retry likely still launches the old model.
   `document_target` also injects the task's `MODEL` into its composition
   environment, which the rebuild's environment precedence reads, so a fix in
   `planned_fallback_model` alone may not be enough. Not reproduced. The
   docs (`composition.md`, "Launch identity is rebuilt …") state the intended
   rule.
2. **Sequence documents validate slower since Phase 4.** Measured this phase
   with `md schema validate` and the shipped schemas as the file-tree root,
   committed (pre-Phase 4) vs working-tree files, same binary, three samples
   each, host load 15–35:

   | Document | release `md` before → after | debug `md` before → after |
   | --- | --- | --- |
   | sequence document (`step_timeout` + two body steps) | 0.17 s → 0.25 s | 0.53 s → 0.84 s |
   | plain prompt (`step_timeout` only) | 0.18 s → 0.21 s | 0.54 s → 0.64 s |

   Phase 4's cost table measured only a plain prompt. The cost comes from the
   three typed container levels in `sequence-types.yaml`, which a sequence
   document now always validates. A debug `claudine sequence` validates its
   document several times (preflight plus each body step), so the effect
   shows up in CLI tests (below).
3. **`step_timeout` of 0.5 s renders as `0s`** in "no stream activity for 0s;
   terminating due to step_timeout" (whole-second rendering; pre-existing).

### Changed test: load-sensitive timing bound

`cli/tests/l1/sequence_schema.rs::sequence_per_step_step_timeout_override`
failed twice under suite load (6.4 s, then 4.1 s, host load average ~127
from other sessions) against a 4 s wall-clock bound. Alone it takes 2.39 s
on every run (its comment said about 1.4 s; follow-up 2 explains much of the
difference). At 2.4 s the bound no longer caught the case it was written for:
a budget reverting to a whole second would finish in about 2.9 s, still under
4 s. The wall-clock bound was replaced with the evidence of which deadline
fired: stderr must contain `no stream activity for 0` and `terminating due to
step_timeout`. A 1 s or 30 s budget prints `1s`/`30s`, and a run killed by the
command's 10 s timeout prints neither. Passes alone in 2.4 s.

### Wave 10 — Final validation (macOS host)

| Gate | Result |
| --- | --- |
| scope: `git status --porcelain --untracked-files=all \| awk '{print $NF}' \| xargs python3 scripts/ci/affected_scope.py` | `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`, `change_class: package` (Phase 4 is still uncommitted in this tree) → gates required |
| `claudine/`: `just test` | first two runs: 1 failure each (`sequence_per_step_step_timeout_override`, above; fail-fast left ~3,100 unrun); after the test change: **10375 passed, 9 skipped, 21 slow, exit 0** (244 s) |
| `claudine/`: `just lint` | exit 0 |
| `just check-tier-coverage claudine` | 0 stranded |
| `claudine/`: `just test-l2 level2_sequence_review_screen` | 5 passed (`just test-l2 sequence_review_screen` matches no test name; the files are named `level2_…`) |
| darkmatter/DMLS, tests that read Claudine schemas, docs, or skills: `cargo nextest run -p darkmatter -p darkmatter-cli -p dmls -E 'test(/claudine_schema_bundle\|schema_validate\|shipped_schema_corpus\|claudine_schema_discovery\|base_schema_end_to_end\|lsp_session\|compose_transclusion\|ambient_ctx_capture\|schema_roots\|schema_phase_validation/)'` | 350 passed |
| darkmatter, the other files that name `claudine/docs` or the skill (`side_effect_partial`, `schemas_core_type_grammar`, `current_root_migration_guard`, `backslash_escape_spans`, `schema_validate_shipped_artifacts`) | 32 passed |
| test-toolkit layout/nextest-config tests | 17 passed |
| `just ci-local --plan` | the plan reflects the whole branch against `main` (messenger, sniff, and others from unrelated work). This feature adds no CI cell, matrix entry, nextest profile, or gate; its packages are scheduled by the ordinary package scope |
| cross-OS | not run: this phase changed docs, comments, one error message, and one `#[cfg(unix)]` test assertion; no path, process, or OS-specific code |

**Acceptance walk** (test names are in the sections cited):

| AC | Evidence |
| --- | --- |
| 1 `with` on tasks | Phase 2 Wave 3 table (`with_is_evaluated_just_in_time_against_the_effective_state`, `params_on_a_step_is_ordinary_step_data`, `with_is_rejected_on_every_executable_but_prompt`, …) |
| 2 provenance | Phase 2 Wave 4 table (`with_file_provenance.rs`, 14 rows + lib tests) |
| 3 shape errors | Wave 3 (`task_with_shape_errors_are_the_with_family_at_the_task`, …) and Wave 4 (`proxy_retry_and_prep_share_the_with_shape_errors`) |
| 4–6, 8 providers | Phase 3 "Tests added or changed" (`sequence_task_providers.rs`, `review/tests.rs`, `preflight/providers/tests.rs`) |
| 7 review screen (L2) | `level2_tmux_review_screen_shows_own_provider_tasks_read_only` (run green this phase) |
| 9 schema | Phase 4 Wave 8 table; departure 3 (`sequence: 42` is coerced, the parser rejects it) stands |
| 10 sweep | this phase's grep (above) |

**Robustness smell grep.** No reader, parser, or deserializer changed in
Phase 5 (one error message, comments, docs, one test assertion), so there is
no new hit. The hits on the changed `with`/`agent`/`model` parses were
justified in the Phase 2 and Phase 3 logs.

### Phase 5 close

- Spec unchanged. Its departures (amendments file already gone; Phase 3's
  null/empty hints; Phase 4's schema construction) are recorded in this log,
  and the docs state the shipped behavior.
- Status: implementation complete, ready for review. Nothing moved to
  `_completed`, nothing staged or committed.

---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-20-lifecycle-grammar/spec.md
plan: claudine/features/2026-10-20-lifecycle-grammar/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/compose_schema_cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter-cli
    - claudine-cli
    - claudine
    - claudine-gen
    - darkmatter
    - dmls
human_review: false
message_to_agent: |-
    Phase 4 finished Wave 6. Read the log's "## Phase 4" section.

    - `just test-l2` passes (282/282, tmux + WezTerm). The two
      `level2_drain_ctrl_c_kitty` tests were NOT run: on macOS they open a
      visible Kitty window. Record them as unrun, not passed.
    - The opt-in prompts binary (`-F prompt-tests,test-fixtures --test
      prompts`) is 63/71. The 8 failures predate this feature: seven assert
      `prompts/_implement/implement-plan.md` content (`epilog: null`,
      `_finish-before-exit.md`, the final-phase warning) that the author
      removed in commit dddf5c988 before Phase 4; the router test looks for a
      body heading in a 20-row stderr preview (body unchanged here). Report
      them separately in Wave 8; do not fix them under this feature.
    - New corpus test `internal_lifecycle_corpus_parses_under_current_grammar`
      passes; it excludes `prompts/fix.md` by name (a draft using the
      not-yet-existing `prep`/`interview` verbs).
    - Route hashes were refreshed and pass without the update flag (AC11).
    - `LifecycleRequeueEnqueueFailed` (unconstructed) keeps its variant name;
      only its message now says `defer` (planned).
source_files_during_phase_2:
    - claudine/cli/src/commands/compose/loop_run.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/budget_scoping.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/recovery_identity.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/common/incomplete_subagents.rs
    - claudine/cli/tests/common/terminal_interrupt.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/clarify.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/format.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/implement.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review.md
    - claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/nested_span_regression/commit.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/characterization_error_routes.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_initialize_acceptance.rs
    - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
    - claudine/cli/tests/l1/compose_presence_parity.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/ctx_launch_anchor.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/effective_diagnostic_render.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
    - claudine/cli/tests/l1/level1_provider_overlay_home.rs
    - claudine/cli/tests/l1/level1_review_router_partial_pty.rs
    - claudine/cli/tests/l1/level1_structured_error_message.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/lifecycle_literal_escapes.rs
    - claudine/cli/tests/l1/lifecycle_set_shell_values.rs
    - claudine/cli/tests/l1/loop_gate_ambient.rs
    - claudine/cli/tests/l1/loop_initialize_state.rs
    - claudine/cli/tests/l1/preflight_execution_parity.rs
    - claudine/cli/tests/l1/provider_error_finalize.rs
    - claudine/cli/tests/l1/provider_tail_launch.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/l1/sequence_budget.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_initialize_include_preflight.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/l1/setter_after_switch.rs
    - claudine/cli/tests/l1/wrap_compose_preflight.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/cli/tests/l1/wrap_opencode.rs
    - claudine/cli/tests/l1/wrap_perf.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/lib/src/composition/coordinator/commit.rs
    - claudine/lib/src/composition/coordinator/handoff.rs
    - claudine/lib/src/composition/coordinator/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/frontmatter_excerpt/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/actions/tests.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/runtime/tests.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
    - claudine/lib/src/composition/lifecycle/tests/validation.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/actions.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/gate_ambient.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_bindings.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/engine/tests/mod.rs
    - claudine/lib/src/composition/looping/engine/tests/post_checked_timing.rs
    - claudine/lib/src/composition/looping/engine/tests/removed_loop_reads.rs
    - claudine/lib/src/composition/looping/engine/tests/seed_state.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/expression/tests.rs
    - claudine/lib/src/composition/looping/types.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/fixtures/frozen_prompts/prompts/_reviews/cross-platform.md
    - claudine/lib/tests/fixtures/frozen_prompts/prompts/clarify.md
    - claudine/lib/tests/fixtures/frozen_prompts/prompts/review.md
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/lib/tests/l1/main.rs
    - darkmatter/dmls/src/overlay/expressions/frontmatter_inventory_tests.rs
    - darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_compose.rs
    - darkmatter/lib/tests/l1/host_globals_compose.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs
docs_updated_during_phase_2:
    - claudine/docs/research/agent-errors/_fleet.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/retry_with_evaluation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/budget_scoping.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/tests/l1/compose_removed_grammar.rs
    - claudine/cli/tests/l1/compose_retry_overlay.rs
    - claudine/cli/tests/l1/main.rs
    - darkmatter/docs/schemas/claudine.yaml
    - darkmatter/docs/schemas/claudine-types.yaml
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/expressions/frontmatter_inventory_tests.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/mapping_orders.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs
    - darkmatter/lib/tests/fixtures/frozen_prompts/prompts/_reviews/feature-review.md
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/compose_dasherized_identifiers.rs
    - darkmatter/cli/tests/fixtures/frozen_prompts/prompts/plan.md
docs_updated_during_phase_3:
    - claudine/docs/providers/dispatch-inventory.json
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/inline/interpolation.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/level2/level2_ac28_lifecycle_lazy_roots.rs
    - claudine/cli/tests/level2/level2_ac2_worktree_compose.rs
    - claudine/cli/tests/level2/level2_file_resolution_capture.rs
    - claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs
    - claudine/cli/tests/level2/level2_lifecycle_action_forms.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/cli/tests/level2/level2_lifecycle_dispatch.rs
    - claudine/cli/tests/level2/level2_lifecycle_loop.rs
    - claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
    - claudine/cli/tests/prompts/shipped_prompts.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/types.rs
    - prompts/_add/add-context-variables.md
    - prompts/_add/add-expressions.md
    - prompts/_clarify/findings.md
    - prompts/_implement/implement-feature.md
    - prompts/_implement/implement-plan.md
    - prompts/_implement/implement-review.md
    - prompts/_implement/implement-suggestions.md
    - prompts/_pr/diagnose.md
    - prompts/_pr/dirty.md
    - prompts/_pr/fix.md
    - prompts/_pr/open.md
    - prompts/_pr/push.md
    - prompts/_pr/triage.md
    - prompts/_prompt.md
    - prompts/_reviews/cross-platform.md
    - prompts/_reviews/dry.md
    - prompts/_reviews/feature-review.md
    - prompts/clarify.md
    - prompts/commit.md
    - prompts/format.md
    - prompts/implement.md
    - prompts/merge-conflicts.md
    - prompts/plan.md
    - prompts/pr.md
    - prompts/review.md
docs_updated_during_phase_4:
    - claudine/docs/getting-started/index.md
    - claudine/docs/research/_TEMPLATE.md
    - claudine/docs/research/acp/_fleet.md
    - claudine/docs/research/agent-cli/_fleet.md
    - claudine/docs/research/agent-logging/_fleet.md
    - claudine/docs/research/agent-models/_fleet.md
    - claudine/docs/research/agent-permissions/_fleet.md
    - claudine/docs/research/hooks/_fleet.md
    - claudine/docs/research/local_runners/_fleet.md
    - claudine/docs/research/mcp/_fleet.md
    - claudine/docs/research/memory/_fleet.md
    - claudine/docs/research/model-config/_fleet.md
    - claudine/docs/research/non-interactive-sessions/_fleet.md
    - claudine/docs/research/plugins/_fleet.md
    - claudine/docs/research/reasoning-level/_fleet.md
    - claudine/docs/research/resume/_fleet.md
    - claudine/docs/research/signals/_fleet.md
    - claudine/docs/research/skills/_fleet.md
    - claudine/docs/research/slash-commands/_fleet.md
    - claudine/docs/research/steering/_fleet.md
    - claudine/docs/research/subagents/_fleet.md
    - claudine/docs/research/system-prompt/_fleet.md
    - claudine/docs/research/usage/_fleet.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/state-management/side-effects.md
    - claudine/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/claudine/validations-and-handlers.md
---
# Implementation Log for 2026-10-20-lifecycle-grammar (5 phases)

## Phase 1

Phase 1 is readiness, inventory, and contract freezing. No grammar code, doc,
or skill changed. The one source edit fixes a pre-existing test failure (see
"Phase 1 verification").

### Prerequisite: `2026-10-18-fm-namespace`

Ready. Evidence, from code rather than the spec alone:

- The spec is closed (`_completed/`, implementation log through review cycle 6).
- `fm` is a Darkmatter fixed root: `darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs`
  `RESERVED_ROOTS` = `fm`, `ctx`, `env`, `current`.
- Claudine's reserved list is `["true", "false", "fm", "env"]`
  (`claudine/lib/src/composition/reserved.rs:13`); `doc` is gone.
- `current_env` survives only in tests that assert it is now ordinary
  frontmatter data (`lifecycle/bindings/tests.rs:236`,
  `prepare/tests.rs::current_env_in_a_lifecycle_shell_command_reads_the_frontmatter`,
  DMLS `overlay/expressions.rs:1343-1377`). `LATE_NAMESPACES = ["current"]`.
- Host-declared binding mechanism exists and is what `loop` will use:
  `darkmatter::markdown::compose::expression::{BindingView, BindingViewBuilder::declare,
  GlobalDeclaration, Availability::{Available, Unavailable, ExecutionDependent},
  UnavailabilityReason, RuntimeBinding}` (`expression/binding.rs:186-340`), with
  Claudine's per-scope catalog in `claudine/lib/src/composition/lifecycle/bindings.rs`
  (`err`/`timing`/`group`/`outputs`). Root classification order is reserved
  root → declared global → document property, so a declared `loop` wins over
  the `loop:` configuration block, which stays readable as `fm.loop`.

### Baseline inventory (AC16 search, 2026-10-13)

Pattern `^\s+stack:|say_first|^\s+- action:|_loop_(count|is_first|is_last|last_output|last_exit_code)`
over `prompts/ claudine/docs claudine/lib claudine/cli darkmatter/docs
darkmatter/dmls darkmatter/lib darkmatter/prompts claudine/prompts
.claude/skills/claudine`: **161 files, 1180 hits (73 Rust, 88 non-Rust)**.
`darkmatter/prompts/` and `claudine/prompts/` have no hits.

Known false positives (not grammar; leave as-is):

- Rust struct fields named `stack:` — `darkmatter/lib/src/markdown/render_tree/fold.rs`,
  `darkmatter/lib/src/markdown/schemas/resolve.rs`,
  `darkmatter/lib/src/markdown/compose/transclusion/types.rs`,
  `claudine/cli/src/commands/wrap/sequence/task_run.rs:164`.
- `claudine/lib/src/reporting/queries/common.rs:189` — SQL alias
  `human_in_loop_count` matches `_loop_count`.
- `darkmatter/dmls/docs/diagnostics.md:200` — `- action: archive` is
  `content_policy` data.
- `claudine/docs/schemas/partials/stack.yaml` — `stack:` is a type name in the
  draft schema set owned by `2026-10-21-claudine-schemas`; out of scope.
- `claudine/lib/src/harness/report.rs:202` — quoted literal data in a report test.
- `claudine/docs/research/hooks/*.md` other than `_fleet.md` — provider-hook
  tables; excluded by the spec.
- `darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md` — a dated
  snapshot used as Darkmatter compose input; historical data.

Buckets to migrate (owners below):

- **Root prompts (24):** `_add/add-context-variables`, `_add/add-expressions`,
  `_clarify/findings`, `_implement/implement-plan`, `_implement/implement-review`,
  `_implement/implement-suggestions`, `_pr/{diagnose,dirty,fix,open,push,triage}`,
  `_prompt`, `_reviews/{cross-platform,dry,feature-review}`, `clarify`, `commit`,
  `format`, `implement`, `merge-conflicts`, `plan`, `pr`, `review`. Matches the
  spec's list. `_prompt.md` and `_implement/implement-plan.md` also read `_loop_*`.
- **Research Markdown:** `claudine/docs/research/_TEMPLATE.md` and all 22
  `claudine/docs/research/*/_fleet.md` (acp, agent-cli, agent-errors,
  agent-logging, agent-models, agent-permissions, hooks, local_runners, mcp,
  memory, model-config, non-interactive-sessions, plugins, reasoning-level,
  resume, signals, skills, slash-commands, steering, subagents, system-prompt,
  usage); every one holds `stack:`. `hooks/_fleet.md` is a fleet and is
  migrated; only the other `hooks/*.md` provider tables are excluded.
- **Claudine docs:** `getting-started/index.md`, `topics/composition.md`,
  `topics/flow-control/{lifecycle,flow-control,flow-control-reference,looping,sequences}.md`,
  `topics/frontmatter-properties.md`, `topics/pre-flight-checks.md`,
  `topics/signal-handling.md`, `topics/state-management/side-effects.md`.
  `composition.md`, `pre-flight-checks.md`, and `signal-handling.md` are not in
  the spec's AC13 list but carry old grammar and must be swept too.
- **Skills:** `.claude/skills/claudine/SKILL.md:10` (`say_first`). Other skill
  pages need a structural sweep for `stack:` examples in Phase 4.
- **Frozen/fixture Markdown:** `claudine/cli/tests/fixtures/frozen_prompts/prompts/{_implement/implement-plan,clarify,format,implement,plan,review}.md`,
  `claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md`,
  `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md` (hash-pinned),
  `claudine/cli/tests/fixtures/nested_span_regression/commit.md`,
  `claudine/lib/tests/fixtures/frozen_prompts/prompts/{_reviews/cross-platform,clarify,review}.md`,
  `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`,
  `darkmatter/lib/tests/fixtures/frozen_prompts/prompts/_reviews/feature-review.md`
  (Darkmatter compose input; migrate so the frozen copy matches its source).
- **Darkmatter schema extension:** `darkmatter/docs/schemas/claudine.yaml` (5),
  `darkmatter/docs/schemas/claudine-types.yaml` (6).
- **Claudine Rust source:** `lifecycle/{parse,mod,executor,audio}.rs`,
  `error/mod.rs`, `error/render/mod.rs`, `reserved.rs`,
  `looping/{expression,types,engine,config}.rs`, `prepare/bootstrap/tests.rs`,
  `prepare/tests.rs`, `sequence/task/{group,tests}.rs`,
  `cli/src/commands/compose/loop_run.rs` (comment),
  `cli/src/commands/wrap/harness_orch/prompt/tests.rs` (`- action:` fixture).
  Lifecycle unit tests: `lifecycle/tests/{audio_emission,diagnostics,nested_span,parse_config}.rs`;
  loop tests: `looping/engine/tests/{gate_ambient,iteration_actions,post_checked_timing,seed_state}.rs`,
  `looping/{actions,config}/tests.rs`.
- **Claudine CLI tests:** 24 `cli/tests/l1` files (largest:
  `compose_initialize_acceptance` 39, `characterization_error_routes` 37,
  `lifecycle_literal_escapes` 19, `compose_initialize_staged_boot` 16,
  `handoff_owners` 11), `cli/tests/l1/error_guards/source_scan.rs`,
  `cli/tests/common/{incomplete_subagents,terminal_interrupt}.rs`, and 9
  `cli/tests/level2` files (`level2_lifecycle_control` alone has 339 hits;
  `level2_lifecycle_loop` 36, `level2_typed_error_render_capture` 18,
  `level2_lifecycle_action_forms` 17).
- **Darkmatter Rust:** `dmls/src/diagnostics/nested_span.rs:40` (`say_first`
  in the structural field list), `lib/src/markdown/compose/expression/parser.rs:2141`,
  `lib/tests/l1/dasherized_identifier_compose.rs:77-78`,
  `lib/tests/prompts/dasherized_identifier_corpus.rs:435`. These use
  `_loop_count` as a dasherized-identifier sample; rename the sample (for
  example `_iter_count`) rather than teach Darkmatter a `loop` root.

Not found by the regex but in scope (structural search in Phase 4):

- The old **structured discriminator long form** `{action: verb, …}` is live in
  `lifecycle/parse.rs:1038` (`parse_stack_item_action_object` →
  `parse_long_form_action_object`) and the variadic error text in
  `error/mod.rs` names it. It is removed grammar and gets the removed-grammar
  diagnostic with a `verb: { … }` replacement hint.
- Communication long-form aliases `message`/`sound` and the dead `route`
  parameter: `lifecycle/action_shape.rs:421-431`.
- Root-dictionary notification fields (`start: {message: …}`) stay legal: they
  are exactly the new root-dictionary bundle. Only `say_first` and `stack`
  inside them are removed.

### Consumer trace

| Concern | Production sites |
| --- | --- |
| `LifecycleStackItem` | `lifecycle/{actions,action_shape,parse,executor,mod}.rs`, `composition/mod.rs` (re-export), `sequence/task/mod.rs` |
| `LifecycleNotification` / notification fields / `loop_concerns` | `lifecycle/{mod,parse,action_shape,audio,executor}.rs`, `looping/config.rs` (`LIFECYCLE_CONCERN_KEYS` filtering, lines 270-281); tests `lifecycle/tests/{nested_span,audio_emission,parse_config,action_shape_control}.rs` |
| `audio_phases` / `say_first` | `lifecycle/{audio,executor,mod}.rs`, `lifecycle/tests/audio_emission.rs` |
| Expression surfaces and source map | `lifecycle/source_map.rs` (`LifecycleSurfacePath`, `LifecycleSourceMap`), `lifecycle/validate.rs` (`iter_stack_expression_surfaces`, nested spans, `err` availability, shell collection) |
| Shell approval | `lifecycle/validate.rs::collect_lifecycle_shell_commands{,_for}` ← `composition/preflight.rs`; `parse.rs::runs_a_shell` (initialize prohibition) |
| `StackControl` | defined `lifecycle/executor.rs:96`; consumed by `lifecycle/{control,runtime}.rs`, `looping/engine.rs`, CLI `wrap/harness_orch/loop_control{,/control_dispatch,/lifecycle_events,/proxy}.rs`, `wrap/composition/{mod,pipeline,preflight}.rs` |
| `ControlDispatch` | `lifecycle/{control,runtime}.rs`, CLI `loop_control{,/control_dispatch}.rs`; tests `lib/tests/l1/{lifecycle_control_flow_spike,agent_errors_fleet}.rs` |
| Proxy overlay transport | `StackControl::Proxy.overlay` → `control_dispatch.rs:195-215` (`EvaluatedProxyRequest`) → `HarnessPromptState.overlay` → `harness_orch/prompt.rs::load_overlaid_source`/`load_overlaid_document` (fresh read + `merge_frontmatter_overlay`) and `prompt.rs:221` `proxy_overlay`; compose path `cli/src/commands/compose/prep.rs` (`proxy_overlay` threaded through prep) → `lib/src/composition/prepare.rs:94,382` → `runtime_state.rs::LayeredOverrides::proxy_overlay`; `wrap/composition/staged_boot.rs:325` |
| Loop values | seeded by `looping/types.rs::insert_ambient_overrides` (called from `looping/types.rs:86` and `looping/engine.rs:603`), resolved by `looping/expression.rs:247-251`, named in `reserved.rs::AMBIENT_VARIABLE_NAMES` and `composition/types.rs:172-175` (`NAMES`), excluded in `looping/config.rs:256`; comments in `looping/engine.rs:50,279,598`, `cli/.../loop_run.rs:103`, `cli/.../prep.rs:1732`, `error/render/mod.rs:214` |
| Removed-key scan precedent | `parse.rs::scan_removed_validation_keys` ← `composition/prepare.rs`; CLI boundary test `cli/tests/l1/compose_removed_validation_keys.rs` |
| Errors to rename/add | `error/mod.rs` (`LifecycleProxyWith{NotMapping,WholeMapping,DynamicKey,EvaluationFailed}`, `LifecycleProxyOnlyParameter`, `LifecycleSayConflict`), `error/render/{mod,lifecycle}.rs`, `error/tests.rs`, CLI `loop_control/error_routing.rs` and its tests `unowned_handoff.rs`, `recovery_identity.rs`; `lib/tests/l1/diagnostic_detail_conformance.rs` |
| AC9 forbidden reference today | `parse.rs:297` `playa::SoundEffect::from_name` and the doc comment at `parse.rs:68` |

### Frozen contracts

These are binding for Phases 2–5. Later phases change them only by recording
the departure here.

1. **Node model** (`lifecycle/actions.rs`, lift-ready):
   `pub enum LifecycleNode { Action(LifecycleAction), Conditional(LifecycleConditional) }`;
   `pub struct LifecycleConditional { pub when: Expr, pub then: Vec<LifecycleNode>, pub otherwise: Vec<LifecycleNode>, pub path: LifecyclePath }`
   (`otherwise` empty means no `else`; the parser has already rejected an
   explicit empty `else`). `LifecycleAction` gains `pub path: LifecyclePath`
   (the authored path of the verb key). A stack is `Vec<LifecycleNode>`.
   `LifecycleStackItem` and `LifecycleNotification` are deleted;
   `LifecycleConfig` keeps only `stacks: LifecycleStacks`, whose fields become
   `Vec<LifecycleNode>` (empty = omitted/null/`[]`/`{}`; no `Option`).
   Task `setup`/`teardown` use the same `Vec<LifecycleNode>`.
2. **Dictionary canonicalization** (`parse.rs`): a bundle expands to leaf
   actions in this fixed order: `set`; `stdout`, `stderr`, `info`, `warn`,
   `success`, `message`, `notify`, `effect`, `say`/`speak`; `shell`; side-effect
   and expression-function verbs by ascending ASCII canonical name; the one
   control directive. A guarded list item becomes one `Conditional` whose
   `then` is the expansion. Canonical order replaces authored mapping order, so
   lifecycle parsing stops consulting `AuthoredOrder`/`mapping_orders` for
   bundle order (other `AuthoredOrder` users in `sequence/` are untouched).
3. **Depth accounting:** a `when` in a stack root list is depth 1; a `when`
   inside a `then`/`else` body is parent depth + 1; siblings share depth.
   Depth 6 → `LifecycleStackInvalidShape` with the authored path of the sixth
   `when`. Bundles containing `when` count the same as `then` items.
4. **Effect-catalog injection:** `parse.rs` exposes
   `parse_lifecycle_config_with(frontmatter, source_file, &LifecycleParseOptions)`
   where `LifecycleParseOptions { is_known_effect: &dyn Fn(&str) -> bool, authored: Option<&Frontmatter> }`
   (task stacks take the same options). The public
   `parse_lifecycle_config(frontmatter, source_file)` moves to
   `lifecycle/mod.rs` (not lift-ready) and supplies a predicate defined in
   `lifecycle/audio.rs`: `playa::SoundEffect::from_name(name).is_some()`. Call
   sites and `LifecycleUnknownEffect` timing (parse time, literal names only;
   `{{ }}` names deferred to event time) are unchanged.
5. **Authored path:** promote `source_map.rs::LifecycleSurfacePath` to the
   shared `pub struct LifecyclePath` (segments `Field`, `MapKey`, `Index`;
   lift-ready). `Display` renders `failure[0].else[1].shell.command`;
   `MapKey` segments that are not identifier-safe render bracket-quoted
   (`with["a.b"]`), matching the existing safe-segment rule of
   `LifecycleProxyWithDynamicKey`. Expansion records the authored key
   (`speak` stays `speak`), never a synthetic execution index. Errors keep the
   existing `property: String` field and fill it with the rendered path of the
   offending node; the post-hoc "stack annotator" (`parse.rs` around 700-760)
   that rewrote `property` to `{event}.stack[i]` is deleted.
6. **Walker API:** one recursive walker in `validate.rs`,
   `walk_stack(&[LifecycleNode], &mut impl FnMut(&LifecyclePath, Surface<'_>))`,
   visits every expression surface in both branches, `when` guards, `set`
   values, and `retry.with`/`proxy.with` values. Nested-span checks,
   `err`-availability checks, binding validation, `_loop_*` detection, and
   shell collection all use it. DMLS mirrors the same path spelling.
7. **Host binding `loop`:** declared in `lifecycle/bindings.rs` like `group`
   (lexical). Inside a loop (the loop `gate` scope and document events of a
   loop iteration) it is `Available`; known absence is
   `Unavailable("claudine.outside-loop")` (new constant `OUTSIDE_LOOP`); unknown
   membership during passive checks is `ExecutionDependent`. The runtime value
   is the object `{count, is_first, is_last, last_output, last_exit_code}`
   supplied as a `RuntimeBinding`; `looping/expression.rs` resolves `loop`,
   `loop.<member>`, and `loop['<member>']` for loop conditions. A missing member
   reads `null` from the object and never falls back to frontmatter.
   Composition of a loop iteration declares the same global for body
   templates. Plain `md compose` gains nothing. `insert_ambient_overrides`,
   `AMBIENT_VARIABLE_NAMES`, and `AmbientVariables::NAMES` are deleted.
8. **Retry overlay transport:** rename the overlay model for sharing —
   `ProxyWith` → `WithOverlay`, `ProxyWithValue` → `WithValue`,
   `ProxyWithError` → `WithOverlayError`. `LifecycleControlAction::Retry` gains
   `with: Option<WithOverlay>`; `StackControl::Retry` gains
   `overlay: IndexMap<String, Value>` (resolved data, empty when omitted),
   evaluated once by the executor with the same code path as `proxy.with`.
   `ControlDispatch::Retry` is unchanged; the wrapper reads the overlay from
   the `StackControl` (as `control_dispatch.rs` already does for proxy) and
   stores it as `HarnessPromptState.pending_retry_overlay: Option<IndexMap<…>>`.
   `load_overlaid_source` merges `state.overlay` (retained proxy overlay) first
   and then `pending_retry_overlay.take()` key-wise on top, so retry wins and is
   consumed by exactly one fresh read. The compose (pre-launch re-entry) path
   passes the same merged map through the existing `proxy_overlay` option.
   `resume` never sets it. An overlay evaluation failure raises before any
   `ControlDispatch`, so no transition and no budget is spent.

### Chosen diagnostic names

- **Removed grammar:** `CompositionError::RemovedGrammar { source_path: PathBuf, path: String, removed: String, replacement: String }`.
  One variant for every removed spelling: `stack`, `- action:`/`action` beside
  `when`, the `{action: verb}` discriminator map, `say_first`, communication
  `message`/`sound`/`route` parameters, and reads of the five `_loop_*` names
  (frontmatter expression or body span). `path` is the authored path (for a
  body span, `body` plus the span's line). It is not `Lifecycle*`-prefixed
  because `_loop_*` reads in a document body are not lifecycle grammar.
- **Shared `with` family:** `LifecycleWithNotMapping`,
  `LifecycleWithWholeMapping`, `LifecycleWithDynamicKey`,
  `LifecycleWithEvaluationFailed` (adds `verb: String`; `target` stays and names
  the document the overlay is built for — the proxy target, or the current
  document for `retry`), and `LifecycleWithUnsupported { source_path, property, verb }`
  replacing `LifecycleProxyOnlyParameter` (`with` is the only parameter it ever
  carried).
- **Loop-block shapes:** mixing loose verbs with `gate:` and `when` at the loop
  root reuse `LifecycleStackInvalidShape` (no new name).
- `LifecycleSayConflict` keeps its name; its meaning becomes `say` and `speak`
  in one dictionary.
- `LifecycleUnknownEffect`, `LifecycleActionOrder` (now list-local),
  `LifecycleMultipleLifecycleActions` keep their names.

### File ownership

| Owner | Files |
| --- | --- |
| Integration (Wave 2 + merges) | `lifecycle/{actions,action_shape,signatures,mod}.rs`, `composition/error/**`, `composition/mod.rs` re-exports, consolidated test registrations (`lib/tests/l1/main.rs`, `cli/tests/l1/main.rs`, `cli/tests/prompts/main.rs`), this log |
| Parser/source-map/validator (Wave 3) | `lifecycle/{parse,source_map,validate}.rs`, `lifecycle/tests/**`, `looping/config.rs` (concern extraction), `sequence/task/**` |
| Binding/loop/expression (Wave 3) | `lifecycle/bindings.rs`, `lifecycle/context*`, `looping/{expression,engine,types,seed}.rs` and `looping/engine/tests/**`, `reserved.rs`, `composition/types.rs`, `cli/src/commands/compose/{loop_run,prep}.rs` loop parts, Darkmatter `_loop_count` identifier samples |
| Executor/audio (Wave 4) | `lifecycle/{executor,audio,runtime}.rs`, `lifecycle/executor/tests/**` |
| Darkmatter schema/DMLS (Wave 4) | `darkmatter/docs/schemas/claudine{,-types}.yaml`, `darkmatter/dmls/src/diagnostics/nested_span{.rs,/**}`, `darkmatter/dmls/src/overlay/frontmatter.rs`, `darkmatter/dmls/tests/fixtures/sequence_descent/**` |
| Retry overlay (Wave 5) | `lifecycle/control.rs`, CLI `wrap/harness_orch/**`, `wrap/composition/**`, `compose/prep.rs` overlay parts |
| Prompts / fixtures / docs (Wave 6) | as split in the plan |

### Planned AC-to-test locations

| AC | Where the evidence lands |
| --- | --- |
| AC1, AC2, AC3, AC4, AC17 (parser) | `lifecycle/tests/parse_config.rs`, `lifecycle/tests/action_shape_control.rs`, new `lifecycle/tests/grammar.rs`; task stacks in `sequence/task/tests.rs` |
| AC4, AC17 (executor) | `lifecycle/executor/tests/{conditions_control,action_dispatch,mutation_visibility}.rs` |
| AC5 | parse: `lifecycle/tests/action_shape_control.rs`; evaluation: `executor/tests/proxy_with_evaluation.rs`; transitions: CLI `loop_control/tests/{retry_resume,proxy,active_state_wiring}.rs` |
| AC6 | `lifecycle/tests/validation.rs`; CLI boundary: new `cli/tests/l1/compose_removed_grammar.rs` modeled on `compose_removed_validation_keys.rs` (`compose`, `inline-compose`, `sequence`) |
| AC7 | `looping/engine/tests/gate_ambient.rs` (migrated), `looping/expression.rs` tests, `lifecycle/bindings/tests.rs`, `cli/tests/l1/loop_gate_ambient.rs` |
| AC8 | `lifecycle/tests/audio_emission.rs` (fake publisher, both orders, survival across transition); spool boundary via `test_toolkit::LockedAudioSpool` as in `lib/tests/l1/tts_phase5_contract.rs` |
| AC9 | new `claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs` reading the six files via `include_str!` |
| AC10 | opt-in `cli/tests/prompts/shipped_prompts.rs`; default `cli/tests/l1/shipped_prompts.rs` over frozen fixtures |
| AC11 | `cli/tests/prompts/shipped_prompt_route_drift.rs` |
| AC12 | `darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs` (holds `lifecycle_inventory_matches_the_authored_claudine_schema`), `dmls/src/overlay/frontmatter.rs` tests, `dmls/tests/l1/mapping_only_corpus.rs` |
| AC13–AC16 | docs facet lint, skill grep, final inventory in Phase 5 |

### Phase 1 verification

- **Pre-existing failure found and fixed.** The first baseline `just test`
  failed one test unrelated to this feature:
  `claudine-cli::l1 compose_schema_cli::compose_frozen_clarify_reads_its_ordinary_doc_property`.
  The frozen `clarify.md` (a verbatim copy of `prompts/clarify.md`, added in
  `5c5d243be`) ends its specification line with `{{doc}}. ` — a trailing
  space — and the test, added in `6e9e4c4f0`, trimmed only the start of each
  line before `ends_with("demo-spec.md.")`. It failed deterministically on
  this branch before any Phase 1 edit. Fix: the test's line matcher now uses
  `str::trim` (`claudine/cli/tests/l1/compose_schema_cli.rs:1289-1296`). The
  fixture is left byte-identical to its source prompt.
- `just test` in `claudine/` (macOS): **8740 run, 8740 passed, 9 skipped**
  (the recipe's standing skips; none added here).
- `just lint` in `claudine/`: passed.
- No other OS run: the only code edit is a string-trim in one L1 test, with no
  platform-dependent behavior.

### Checkpoint

Prerequisite available; inventory, ownership, diagnostic names, and contracts
recorded above; no design ruling blocks Wave 2.

## Phase 2

Phase 2 is Wave 2 (shared action model) and Wave 3 (parser rewrite and loop
bindings).

### Wave 2 — shared model (done by the integration owner)

- **Node model** as frozen: `LifecycleNode { Action, Conditional }`,
  `LifecycleConditional { when, then, otherwise, path }`, and
  `LifecycleAction { kind, no_error, path }` (`lifecycle/actions.rs`).
  `LifecycleStackItem` and `LifecycleNotification` are deleted. Stacks are
  `Vec<LifecycleNode>`; `LifecycleConfig` keeps only `stacks`, and
  `stack(signal)` returns a slice (empty means absent).
  `for_each_action`/`for_each_action_mut` give the structural walk (both
  branches, guards not evaluated) used by preflight stamping, the
  `initialize` shell refusal, task approved-command stamping, and shell
  collection.
- **`LifecyclePath`** is the promoted `source_map.rs::LifecycleSurfacePath`,
  now `pub` and re-exported from `actions` and `composition`. It gained
  `parent()` and `root_name()`.
- **`CommunicationAction`** is `{ channel, text }`; `route` and the
  `message`/`sound` payload aliases are gone from the model.
- **`LifecycleControlAction::Retry`** gained `with: WithOverlay`.
  *Departure from contract 8:* the field is `WithOverlay` (empty when omitted),
  not `Option<WithOverlay>`, matching `Proxy.with`, because an omitted and an
  empty overlay already mean the same thing for `proxy`. The executor does not
  yet evaluate or transport it (`StackControl::Retry` is unchanged); that is
  Wave 5.
- **`ActionLocation`** (`coordinator/handoff.rs`) is now
  `{ signal, path: LifecyclePath }`, no longer `Copy`, and renders the
  authored path (`failure[0].else[1].proxy`) instead of
  `failure.stack[0].action[1]`. `stack_index()`/`action_index()` are gone.
  The proxy commit-time file-reference error now names `{event}[*].proxy`.
- **Errors** (`composition/error/**`): added `RemovedGrammar`; replaced
  `LifecycleProxyOnlyParameter` with `LifecycleWithUnsupported { source_path,
  property, verb }`; renamed the `LifecycleProxyWith*` family to
  `LifecycleWith*`. *Departure from "Chosen diagnostic names":*
  `LifecycleWithEvaluationFailed` does **not** store a `verb` field. Adding a
  sixth `String` pushed `CompositionError` past the workspace
  `large-error-threshold` (136 bytes; clippy `result_large_err` fired at 226
  sites). The verb is the last segment of the variant's `property` (the
  action path, e.g. `failure[0].proxy`), so `error::overlay_directive(property)`
  derives it for `Display`, the status block, and the detail `message`.
  `LifecycleSayConflict` now means `say` + `speak` in one dictionary. Removed
  `LifecycleStackAmbiguous` and `LifecycleSetLongFormRemoved`: no grammar can
  produce them now (a multi-key dictionary is a legal bundle, and
  `{action: set, …}` is `RemovedGrammar`). Renderer text that described
  `stack:`/`action:` grammar was rewritten. `RemovedGrammar` uses the shared
  `composition.lifecycle_invalid` code with `property`/`message` detail.
- **Effect catalog injection** (contract 4): `parse.rs` takes
  `LifecycleParseOptions { is_known_effect, authored }`; the public
  `parse_lifecycle_config`, `parse_task_action_stack`, and
  `parse_single_action` now live in `lifecycle/mod.rs` and supply
  `audio.rs::is_known_effect` (Playa). `audio_phases`/`AudioPhase` and
  `normalize_empty_string` are deleted.
- **AC9 guard:** `lib/tests/l1/lifecycle_lift_ready_guard.rs` (one test per
  slice file via `include_str!`, plus a self-test of the scan). The six files
  name no forbidden type.

### Executor work pulled forward from Wave 4 (departure)

Deleting `LifecycleNotification` (frozen contract 1) removes the
notification-field execution path, so part of Wave 4 had to land here for
the crates to compile:

- `StackExecutionContext::execute_event` now runs only the stack; the
  recursive `execute_nodes` walks nodes serially, evaluates each reached guard
  once, runs only the selected body, and returns a directive or failure
  through every enclosing list. Diagnostics use each action's authored path;
  `StackRoot` is gone. `execute_action_stack(nodes)` lost its `property`
  argument (paths are rooted by the parser).
- `emit_top_level`, `resolve_emit`, and `emit_top_level_for_signal` are
  deleted; the CLI's success/blocked path
  (`loop_control/lifecycle_events.rs`) now runs only the stack.
- The guard's own `emit_signal` (drop safety net and `emit_*_once` helpers,
  which have no execution context) now emits the stack's root-level,
  unguarded, literal `stderr`/`message`/`notify`/`say`/`speak`/`effect`
  actions in authored order. Phase 3 should decide whether that safety net
  stays.

Audio is therefore already published in stack order; the Wave 4 owner still
owns the ordered-publication tests (AC8) and the durable-queue coverage.

### Wave 3 — parser (done by the integration owner)

`parse.rs` was rewritten around a `StackParser` (list → item → bundle):

- Stacks accept omitted/`null`/`[]`/`{}` (empty), a list, or a root
  dictionary (no `when`, `then`, or `else`). Task `setup:`/`teardown:` accept
  the same, rooted at the task property.
- Bundles expand in canonical order (`set`; `stdout`, `stderr`, `info`,
  `warn`, `success`, `message`, `notify`, `effect`, `say`/`speak`; `shell`;
  other verbs by ASCII name; the directive). Lifecycle parsing no longer reads
  authored mapping order except for a `set:` mapping's keys.
- `when` items: `then` (+ optional `else`) or sibling verbs; depth limit five
  (`LifecycleStackInvalidShape` at the sixth `…when`); list-local
  `LifecycleActionOrder` reported at the unconditional directive's path.
- `no_error`: long-form parameter or bundle sibling; YAML boolean only;
  rejected beside `then`/`else`, beside a lone `set`, and when both homes are
  set; never applied to `set`.
- Removed grammar (`stack`, `action`, `say_first` keys; communication
  `message`/`sound`/`route` long-form parameters) raises `RemovedGrammar` with
  the authored path and a replacement hint, checked before any other shape
  error. At the `loop:` root, `action`/`actions` stay iteration controls.
- `with` is accepted on `proxy` and `retry`; elsewhere
  `LifecycleWithUnsupported`. `resume.message` stays valid.
- `loop:` concerns: loose verbs (a root bundle) or `gate:` (a stack), never
  both; `when` at the loop root is rejected. `looping/config.rs` now accepts
  every lifecycle-concern key (any verb, `gate`, `no_error`, and `when`/
  `stack`/`say_first` so the lifecycle parser reports them typed) and shares
  `LOOP_ITERATION_KEYS` with the lifecycle parser.
- `source_map.rs` walks the new grammar; `validate.rs` has the one recursive
  `walk_stack` (contract 6) used by nested-span, `err`-availability, and
  shell-collection checks. Shell collection walks both branches, with paths
  like `failure[0].else[1].shell.command`.
- `sequence/preflight/shape.rs` (raw shell collection) — see the test log
  below for its long-form fix.

Parser evidence: `lifecycle/tests/grammar.rs` (56 tests: AC1–AC4, AC17, the
parse side of AC5 and AC6). First run: 52/56; the failures were one wrong
expected path in the test and a real bug — the removed-key check rejected the
loop-root iteration `action:`. Fixed; 56/56 and 7/7 guard tests pass.

Drift fixed in touched code: the long-form `defer` missing-delay message said
"`requeue`"; it now says "`defer`" (the separate
`LifecycleDeferNotImplemented` wording stays with the Phase 4 docs sweep).
`audio.rs` carried a doc comment describing a deleted serde helper on
`tts_config_from_settings`; it now describes the function.

Also in the parser pass: `sequence/preflight/shape.rs::collect_stack_shell_commands`
(the raw-JSON collector sequence preflight uses for task `setup:`/`teardown:`)
now reads the long form `shell: {command, on_error}`; it previously only
recognized the removed `{action: shell, command: …}` map, so a long-form task
shell would have escaped sequence approval. A bad-path bug in the executor was
found by the migrated tests: a `set` action's diagnostics read
`start[0].set.set.out` (the executor appended `.set` to a path that already
ended in `set`); fixed in `executor.rs`.

### Wave 3 — loop bindings (contract 7)

Two workers. The first landed the binding declaration, condition lookups,
seeding removal, and the pre-launch `RemovedGrammar` scan; the second wired
the iteration's values into every surface. Design as built:

- `lifecycle/bindings.rs` declares `loop` like `group`, with
  `OUTSIDE_LOOP = "claudine.outside-loop"` and a three-state
  `EnclosingLoop { Iteration(&LoopAmbient), Pending, Outside }`. `Pending` is a
  loop document prepared before its first iteration (seed, bootstrap read,
  launch audit): `loop` is execution-dependent and reads `null`, as the
  `_loop_*` names did before the loop seeded them.
- `StackExecutionContext.loop_values: Option<&LoopAmbient>` feeds
  `runtime_bindings_in_loop`; the loop gate passes the just-finished
  iteration; CLI iteration events read it from a new
  `MaterializedHarnessPrompt` field. `PrepareOptions` and
  `CallerInputLayers` carry `loop_values`, so an iteration's re-preparations
  (stabilized reread, retry, resume) keep it; a proxy target clears it.
- `looping/expression.rs` resolves `loop`, `loop.<m>`, and `loop['<m>']` in
  `while`/`until` with no frontmatter fallback. `insert_ambient_overrides`,
  `AMBIENT_VARIABLE_NAMES`, and `AmbientVariable::NAMES` are deleted
  (`reserved.rs` keeps `LOOP_ROOT` and `RETIRED_LOOP_NAMES`).
- **Darkmatter main compose gained host-declared globals** (not anticipated by
  the plan: Phase 1 found the binding machinery, but only subtree compose used
  it). `HostGlobals::associate(view, entries)` (shared registration checks with
  `EvaluationSession::associate`) and `ComposeOptions::with_host_globals`. The
  globals travel on `ResolutionContext` like the `current` authority; both the
  frontmatter and body lookups read them; root classification is reserved root
  → declared global → document property, so bare `loop`/`loop.x`/`loop['x']`
  read the global and `fm.loop` reads the configuration; a declared global is
  exempt from the deferred-key check (an explicit `fm.loop` read is not);
  transcluded documents inherit the globals; the globals join the options
  fingerprint only when present, so with none supplied Darkmatter is
  byte-identical (whole suite green). Plain `md compose` supplies none.
- Retired names: `RemovedGrammar` before launch from parsed expressions only
  (variable roots, `fm._loop_x`, `fm['_loop_x']`) in lifecycle surfaces
  (`validate.rs::find_removed_loop_read` on `walk_stack`), `while`/`until`,
  loop actions, other frontmatter strings, and body spans outside code blocks
  (`path: body:<line>`). The five names are also declared **unavailable host
  globals** (reason `claudine.removed-loop-name`), so a read introduced only by
  a later render (a transclusion) is mapped to `RemovedGrammar` with
  `path: "render"`. Literal text, Markdown, code fences, and `_loop_notes`
  stay data.
- Darkmatter's dasherized-identifier sample `_loop_count` became `_iter_count`.

Departures and known gaps (recorded, not hidden):

- Passive lifecycle validation treats `loop` as execution-dependent, so a
  non-loop document reading `loop` in an event is rejected when the event
  fires (typed `claudine.outside-loop`), not before launch.
- Sequence shell approval treats `loop` as unavailable (sequence approval
  fixes every command before any step runs); lifecycle shell approval treats
  it as lexical.
- A proxy target reached from a loop iteration does not receive `loop`
  (before, it inherited `_loop_*` through the override layer). It is not an
  iteration.
- Later-render detection does not catch an explicit `fm._loop_*` inside a
  transcluded document, does not reach the context-free branch-discovery pass
  of frontmatter `$( … )` ternaries, and reports `path: "render"` with the
  root document rather than the transcluded file and line.

### Test migration (Phase 4 work pulled forward)

`just test` must be green at the end of every phase, and the parser now
rejects the old grammar, so every default-tier test and fixture that authored
it had to migrate in this phase (planned for Wave 6). Four workers migrated
them with disjoint ownership:

- **Lib unit and L1 tests:** all lifecycle, executor, coordinator, error,
  prepare, preflight, schema, and sequence tests; lib frozen prompts
  (`_reviews/cross-platform`, `clarify`, `review`); and the live fleet
  `claudine/docs/research/agent-errors/_fleet.md` (read by
  `lib/tests/l1/agent_errors_fleet.rs`; same guards, order, and parameters).
  `claudine/gen/tests/l1/agent_errors_check.rs` reads the same fleet and was
  adjusted (`success` is now the list). Tests whose only subject was removed
  behavior were deleted and are covered by `grammar.rs` or rewritten
  equivalents: `audio_order_*` (5), `blank_notify_is_normalized_to_none`,
  `trims_empty_strings_to_none`, `parse_serde_unknown_field_*`,
  `stack_as_map_reports_sequence_mismatch_not_unknown_property`,
  `rejects_ambiguous_multi_key_action_object`,
  `positional_proxy_plus_sibling_with_stays_ambiguous`,
  `rejects_unknown_stack_item_key`, `rejects_with_on_every_other_action`,
  `emit_top_level_for_signal_fires_comm_without_running_stack`.
  `audio_emission.rs` now proves stack-order publication through the executor
  in both directions, interleaved with branches, before a `retry`, and on the
  durable spool.
- **CLI unit tests** (`cli/src/**`): about 55 JSON fixtures; three
  "top-level before stack" tests renamed to authored-order tests.
- **CLI L1 + fixtures:** every `cli/tests/l1` file with old grammar,
  `cli/tests/common/{incomplete_subagents,terminal_interrupt}.rs`, the CLI
  frozen prompts (`_implement/implement-plan`, `clarify`, `format`,
  `implement`, `plan`, `review`, `frozen_prompts/shipped_implement_route`),
  and `nested_span_regression/commit.md`.
  `compose_initialize_acceptance::an_initialize_stop_ends_the_stack_…` had an
  action after `stop` (now an order error); the stop moved into a `then` body
  so the item after it still proves it never runs.
- **Read by default L1 and therefore migrated now:**
  `cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
  (the hash-pinned route fixture; `shipped_prompts.rs` parses it) and
  `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md` (same
  corpus); the DMLS inventory assertion that named
  `initialize.stack[0].action[0].message` now names
  `initialize[0].then[0].message`. The route fixture's pinned hashes cover the
  shipped prompts, not the fixture, and the shipped prompts are unchanged, so
  the opt-in `shipped_prompt_route_drift` fixture-vs-prompt comparisons will
  disagree until Phase 4 migrates `prompts/_implement/implement-plan.md` and
  re-derives the fixture (AC11).
- **Docs:** only the loop example in
  `claudine/docs/topics/flow-control/lifecycle.md` ("Loop lifecycle concerns")
  and its following sentence, because
  `loop_gate_ambient::lifecycle_reference_loop_example_runs_as_written`
  executes it. The rest of the page is Phase 4.
- **Not migrated (Phase 4):** `cli/tests/level2/**` (compile verified with all
  features; YAML still old — `level2_lifecycle_control.rs` alone has ~426
  hits), `cli/tests/prompts/**`, root `prompts/`, research templates and other
  fleets, and docs/skill pages other than the above.

### Requirement-to-test map (Phase 2 scope)

| Requirement | Tests |
| --- | --- |
| AC1 lists/root dictionaries in every stack home | `lifecycle/tests/grammar.rs`: `every_event_accepts_a_flat_action_list`, `every_event_accepts_a_root_dictionary_in_canonical_order`, `a_bundle_expands_set_communication_shell_effects_then_the_directive`, `a_root_dictionary_never_carries_when`, `the_loop_gate_is_a_stack`, `loose_loop_verbs_are_a_root_dictionary_beside_iteration_controls`, `loose_loop_verbs_cannot_mix_with_gate`, `when_at_the_loop_root_is_rejected`, `task_stacks_accept_a_list_or_a_dictionary_rooted_at_the_task_property` |
| AC2 depth five / six | `conditionals_nest_five_levels_deep`, `a_sixth_level_is_a_typed_error_naming_its_path`, `an_else_if_chain_counts_each_nested_when_and_siblings_do_not` |
| AC3 list-local placement | `an_action_after_an_unconditional_directive_is_rejected_in_every_list`, `a_when_item_never_counts_as_unconditional`, `the_same_directive_in_sibling_branches_is_legal`, `two_directives_in_one_dictionary_are_rejected`, `skip_stays_initialize_only_inside_branches` |
| AC4 `no_error` homes | `no_error_is_accepted_as_a_long_form_parameter`, `no_error_is_accepted_beside_the_verbs_of_a_bundle_but_never_reaches_set`, `a_body_may_be_a_dictionary_with_its_own_no_error`, `then_cannot_sit_beside_sibling_verbs_or_no_error`, `no_error_beside_a_lone_set_is_rejected`, `set_has_no_long_form_no_error`, `inner_and_sibling_no_error_are_ambiguous_even_when_they_agree`, `no_error_must_be_a_yaml_boolean_in_both_homes`, `no_error_defaults_to_false`; executor side in `executor/tests/{set_shell_values,runtime_set}.rs` |
| AC5 (parse side) `with` placement | `with_is_accepted_on_retry_and_proxy`, `with_is_rejected_on_every_other_verb_resume_included`, `a_retry_overlay_has_the_proxy_shape_rules`, `resume_message_stays_a_valid_parameter` |
| AC6 removed grammar (parse) | `the_stack_key_is_removed_grammar_in_every_stack`, `the_action_aggregator_and_discriminator_are_removed_grammar`, `say_first_is_removed_grammar`, `communication_message_sound_and_route_parameters_are_removed_grammar`, `removed_keys_win_over_every_other_shape_error` |
| AC6 retired `_loop_*` reads | `looping/engine/tests/removed_loop_reads.rs`, `bindings/tests.rs::a_retired_loop_read_is_found_only_in_parsed_reads`, `looping/expression/tests.rs::a_condition_reading_a_retired_loop_name_is_removed_grammar`, `iteration_bindings::a_retired_read_a_transcluded_document_introduces_is_removed_grammar`, CLI `loop_gate_ambient::{a_retired_loop_read_stops_compose_before_launch, naming_a_retired_loop_spelling_in_text_is_data}` |
| AC7 `loop.*` | `bindings/tests.rs` (matrix, `the_loop_global_carries_one_iteration_and_never_falls_back_to_the_document`, `the_loop_global_outside_a_loop_is_the_typed_scope_diagnostic`), `looping/expression/tests.rs` (bare/dotted/bracket, no fallback, `fm.loop`), `looping/engine/tests/gate_ambient.rs` (migrated, coverage kept), `looping/engine/tests/iteration_bindings.rs` (body + compose-time frontmatter, pending, outside, lifecycle shell bytes), `seed_state.rs`, CLI `loop_gate_ambient.rs` (gate on every pass, iteration events, body/frontmatter/gate); Darkmatter `tests/l1/host_globals_compose.rs` (7, including "undeclared `loop` stays a document property") |
| AC9 lift-ready slice | `lib/tests/l1/lifecycle_lift_ready_guard.rs` (6 files + scan self-test) |
| AC17 parser coverage | `grammar.rs`: `explicitly_empty_branches_are_rejected`, `orphan_branches_guard_only_and_flag_only_items_are_rejected`, `several_sibling_verbs_use_short_form_only`, `say_and_speak_cannot_share_a_dictionary`, `speak_keeps_its_authored_key_in_the_path`, `unknown_item_keys_are_unknown_verbs_not_discarded`, `unknown_long_form_parameters_are_rejected`, `communication_long_form_takes_text`, `short_form_arity_is_checked_with_the_authored_path`, `object_arguments_still_pass_through_whole_value_interpolation`, `an_unknown_literal_effect_is_rejected_in_any_branch_and_a_template_is_deferred`, `initialize_refuses_shells_in_either_branch`, `initialize_refuses_a_shell_valued_set_in_a_dead_branch`, `both_branches_contribute_shells_to_approval`, `absent_null_and_empty_stacks_are_all_empty`, `a_stack_of_the_wrong_type_is_rejected_not_read_as_empty` |
| Executor recursion (pulled forward; AC8/AC17 executor side owned by Phase 3) | migrated `executor/tests/conditions_control.rs`, `lifecycle/tests/audio_emission.rs` (stack-order publication both directions) |

The configuration Input Robustness Matrix is omitted per the plan (text
grammar); its shape cells that do apply are covered above: absent/null/empty
stacks (empty), wrong-type stack (rejected), explicit empty branches
(rejected), non-boolean `no_error` (rejected), unknown keys (rejected).

### Gates (macOS, this host)

- `just test` in `claudine/`: **8810 run, 8810 passed, 9 skipped** (the
  recipe's standing skips; Phase 1 baseline 8740).
- `just lint` in `claudine/` (incl. `lint-lifecycle-doc-facets`): passed. The
  linker's `__eh_frame section too large` warning is environmental.
- `just test` in `darkmatter/`: **9947 run, 9947 passed, 10 skipped**;
  `just lint` in `darkmatter/` passed (incl. the `zed-dmls` wasm32 build).
- No other OS was exercised. The changes are platform-neutral Rust (parsing,
  path rendering, binding plumbing) with no `cfg`, filesystem-path, or process
  behavior; CI's Linux/macOS cells and the nightly Windows/WSL2 cells cover
  them.

### Observed, not in scope

A CLI L1 worker reported: in a `fail_fast: false` loop, a crashed lifecycle
expression in `success` (e.g. `append_line: ["events.log", "{{ no_such_root() }}"]`)
keeps the loop from advancing (the `increment` action never runs), so the
provider relaunches up to the iteration cap (100 launches, exit 1, ~20 s),
while the diagnostic says "the run halts". Not verified against `main`;
likely pre-existing and unrelated to the grammar. Recorded for a fix
specification rather than changed here.

### Checkpoint

Shared node, path, binding, and overlay interfaces are stable; the parser,
walker, source map, and binding paths agree; AC9 passes. Wave 3 is complete
for parser grammar and loop bindings. Remaining for Phase 3: AC8
durable-queue/fake-publisher coverage beyond the migrated tests, the guard's
drop safety-net decision, `retry.with` evaluation and transport (Wave 5),
the CLI-boundary removed-grammar tests (`compose`, `inline-compose`,
`sequence`), and the DMLS `nested_span.rs`/`overlay/frontmatter.rs`/schema
extension work (Wave 4).

## Phase 3

Phase 3 is Wave 4 (executor/audio proof and editor integration, run in
parallel by two owners with disjoint files) and Wave 5 (retry overlay
evaluation, transport, and consumption, plus the CLI-boundary
removed-grammar tests). The Wave 5 owner ran concurrently with the Wave 4
test owner because the Wave 4 executor code had already landed in Phase 2
(see "Executor work pulled forward"); the Wave 4 owner was restricted to test
files and `executor.rs` belonged to Wave 5.

### Wave 4 — executor and audio proof

The executor recursion landed in Phase 2; this owner audited it against the
Wave 4 contracts and added tests where coverage was missing. No defect was
found in `executor.rs`. `cargo nextest run -p claudine --lib -E 'test(/lifecycle::/)'`:
529/529.

| Contract | Existing evidence | Added in Phase 3 |
| --- | --- | --- |
| AC8 fake publisher, both orders | `audio_emission.rs::executor_publishes_{an_effect_then_speech,speech_then_an_effect}_in_stack_order`, `executor_interleaves_audio_with_other_actions_and_branches_in_order` | — |
| AC8 audio survives a transition | `audio_before_a_directive_is_published_before_the_transition` | `executor_durably_queues_the_selected_branch_in_stack_order_across_a_transition` (LockedAudioSpool + eSpeak stub, dry run removed only in the fixture; queue `[File, Command, File]`, sequences 1–3, outcome `Retry`, the unselected branch queued nothing) |
| AC8 dry run publishes nothing | — | `executor_publishes_no_audio_under_dry_run` (no spool directory created) |
| Selected branch only (effects and operands) | `conditions_control::when_false_skips_item_when_true_runs` | `only_the_selected_branch_evaluates_its_operands_and_runs_effects`, `an_unselected_raising_operand_is_never_evaluated` (both directions) |
| Guard evaluated once; `set` visibility | `mutation_visibility::stack_action_sees_prior_set_frontmatter`, `runtime_set::consecutive_set_actions_observe_each_others_updates` | `a_guard_is_evaluated_once_and_set_is_visible_to_later_actions_and_guards` |
| Guard failure follows failure routing, never selects `else` | `when_unknown_function_fails_closed` | `a_raising_nested_guard_fails_the_event_without_selecting_else` (`start[0].then[1].when`) |
| Nested directive ends the event | `control_action_terminates_remaining_items` (depth 1) | `a_depth_two_directive_terminates_every_enclosing_list` (`then` and `else`) |
| `no_error`: dispatch vs expression | `action_dispatch::{no_error_suppresses_propagation_and_continues, no_error_does_not_suppress_evaluation_raise}` | `nested_dispatch_failure_obeys_no_error_and_otherwise_stops_the_event`, `nested_no_error_does_not_suppress_an_evaluation_error`, `runtime_set::bundle_no_error_cannot_suppress_its_set_failure` (AC4) |

**Drop safety net: kept.** `LifecycleRunGuard::emit_signal` is reached in
production only from the guard's `Drop` (a run that started but never emitted a
terminal event). It is the last-resort terminal notice, as the old
notification-field drop path was. Its doc comment now says "stack order (a root
dictionary's canonical order)" and lists what it skips.

**Bug fixed (`lifecycle/mod.rs::emit_signal`).** Text with an embedded
template (`message: "done: {{ fm.title }}"`) parses as a `StringLiteral`, so
the safety net sent the raw `{{ fm.title }}`. It now skips any text in which
`ExpressionFinder::find_all_plain` finds a span, because it has no context to
evaluate one. Before the fix, the new
`guard_runtime.rs::guard_drop_emits_only_root_literal_communication` test
failed with `Message { text: "done: {{ fm.title }}" }`. The test asserts that
only root literal communication is emitted; guarded items, whole-value
templates, embedded templates, and `shell` are skipped.

### Wave 4 — editor integration (AC12)

Darkmatter-only owner. `just test` in `darkmatter/`: **9950 run, 9950
passed, 10 skipped**; `just lint` in `darkmatter/` passed (including the
`zed-dmls` wasm32 build). `mapping_only_corpus` passed against its existing
baseline: **no re-bless**.

- **Schemas, edited in place** (same paths, imports, and extension loading):
  - `claudine-types.yaml`: `lifecycle-event` is a list of entries or a root
    dictionary. An entry is a bare verb string or a `lifecycle-stack-item` with
    `when` (expression), `then`, `else`, `no_error`, and any verb key; it is
    never empty. A `then`/`else` body is a non-empty list or a non-empty
    dictionary. `lifecycle-branch-item` types `when` one level down; deeper
    bodies are loose, and the parser owns the depth limit. `loop-event` gained
    `gate:` and loose verb keys. `lifecycle-action`, `stack`/`action`,
    `say_first`, and the fixed communication-field list are removed.
  - `claudine.yaml`: the five generated `_loop_*` properties are removed and
    the `loop` description is updated.
  - Checked with `md schema validate`: every new-grammar shape (including five
    levels) passes; `then: []`/`{}`, `start: run-now`, `- 5`, `- {}`, and
    `gate: hi` are rejected.
- **DMLS mirror** (`dmls/src/diagnostics/nested_span.rs`): one classifier
  that follows Claudine's path spelling: an event or `loop.gate` is a list or
  a dictionary; in an item, `when` is a condition, `then`/`else` are bodies,
  and any other key is a verb; everything under a verb is checked (operands,
  long-form parameters, `set`/`with` data) except a long-form `no_error`;
  loose `loop:` keys are verbs, and iteration controls and `gate` itself are
  not. New constants `STRUCTURAL_ITEM_KEYS` and `LOOP_ITERATION_KEYS`;
  `COMMUNICATION_FIELDS` and the `stack`/`action`/`say_first` special cases are
  gone. **Intentional behavior change:** a whole-value `{{ }}` in a dictionary
  `effect` is now checked, matching Claudine's own validation (it used to be
  skipped).
- **Tests mapped to AC12:**
  - Extension loads at its documented paths and accepts the new grammar:
    `cli/tests/l1/schema_validate.rs` (`…_accepts_reused_lifecycle_and_loop_types`
    with `gate`, `…_rejects_an_empty_lifecycle_item` at `/initialize/0`),
    `lib/tests/l1/base_schema_end_to_end.rs` (`claudine_schema_accepts_typed_lifecycle_stack`,
    `…_rejects_malformed_lifecycle_stack`,
    `…_accepts_loop_controls_and_lifecycle_concerns_together`),
    `lib/tests/l1/schemas_constraints_block_matrix.rs` (now reads
    `lifecycle-branch-bundle`).
  - Inventory guard still active:
    `nested_span_tests::lifecycle_inventory_matches_the_authored_claudine_schema`
    (rewritten: union arms, structural keys plus any verb, loop keys equal
    `LOOP_ITERATION_KEYS` + `gate`, `when`/`while`/`until` expressions); the
    opt-in `dasherized_identifier_corpus::condition_keys_match_the_shipped_claudine_schema`
    (run, passing). Claudine's
    `single_pass_inventory_matches_the_authored_claudine_schema` still passes.
  - Classification of list item, `then`/`else` body, root dictionary, and
    `gate`: `nested_span_tests::{path_matcher_classifies_representative_lifecycle_paths,
    item_operands_proxy_with_values_and_predicates_are_diagnosed,
    root_dictionaries_branch_bodies_and_the_loop_gate_are_diagnosed,
    loose_loop_verbs_are_diagnosed_and_iteration_controls_are_not}`; the nine
    lifecycle tests in `providers/frontmatter/sequence_tests.rs`, including
    `failure[0].else[1].when` and `loop.gate[3].when`.
  - Pointer tests (`overlay/frontmatter.rs`): `test_mapping_items_descend_to_their_keys`
    (`initialize[0].when`, `initialize[1].then[0]`) and the new
    `test_lifecycle_pointers_mirror_the_claudine_authored_path`
    (`start.say`, `failure[0].else[1].shell.command`, `loop.gate[0].then[0].warn`).
  - Also migrated: `dmls/tests/l1/no_side_effects.rs` (hover lines moved by
    one), `lib/src/markdown/{mapping_orders,frontmatter}.rs` tests,
    `capture/groups.rs`, the frozen `darkmatter/cli/.../prompts/plan.md` and
    `darkmatter/lib/.../prompts/_reviews/feature-review.md`, and
    `_loop_count` → `_iter_count` in `cli/tests/l1/compose_dasherized_identifiers.rs`.
- **Docs:** `dmls/docs/diagnostics.md` (now says how deep `when` is checked),
  `docs/lsp/features.md`, `docs/inline/interpolation.md`.
- **Remaining `darkmatter/` AC16 hits, all intentional:** `features/_completed/**`
  and `fixes/_completed/**` (snapshots);
  `lib/tests/fixtures/clean/prompts-commit-464e2d90e.md` (must stay
  byte-identical to a pinned git blob); `dmls/docs/diagnostics.md:200`
  `- action: archive` (a content-policy example, not lifecycle grammar);
  `stack` struct fields in `render_tree/fold.rs`, `schemas/resolve.rs`, and
  `transclusion/types.rs` (unrelated Rust).
- **Known limitation:** DMLS checks a nested `{{ }}` inside a `when` only where
  the schema types `when` as an expression (the first two conditional levels).
  A deeper `when` is accepted by the editor and caught by Claudine's parser
  before launch.

### Wave 5 — retry overlays (AC5) and CLI removed grammar (AC6)

**Design as built**

- `StackControl::Retry` gained `overlay: IndexMap<String, Value>` (empty
  when `with` is omitted). `resolve_proxy_with`/`walk_proxy_with` became
  `resolve_with_overlay`/`walk_with_overlay`, and `retry` uses the same path as
  `proxy`: native values, literal data, nested diagnostic paths, and the
  lifecycle-key template rejection are unchanged for proxy. For a retry, the
  error's `property` is the retry's authored path (`failure[0].then[0].retry`),
  so `overlay_directive` reads "retry", and `target` is the current document.
  The overlay resolves inside `resolve_control`, so a failure leaves
  `control: None` plus an `evaluation_error`: nothing is dispatched, no budget
  is spent, and `no_error` cannot hide it.
- `control.rs::decide_control` ignores the overlay; `ControlDispatch::Retry`
  is unchanged (contract 8).
- CLI: `HarnessPromptState.pending_retry_overlay: Option<IndexMap<…>>`.
  `control_dispatch.rs` sets it on retry (`None` for an empty overlay, so a
  later retry without `with` clears it) and clears it on resume;
  `coordinator.rs` clears it on proxy adoption. `prompt.rs::active_overlay(state)`
  merges `state.overlay` (retained proxy overlay) and then the retry overlay
  key-wise on top; both `load_overlaid_source` and `harness_prepare_options`
  (the existing `proxy_overlay` field, which is how pre-launch re-entry gets
  it) use it. Nothing is written to disk; values pass through the proxy data
  layer and are never evaluated as templates. `compose/prep.rs` needed no
  change.

**Departure from contract 8 (consumption point).** A retried attempt reads its
document twice: the shell preflight (`preflight_harness_document`), then
`materialize_harness_prompt`. A `take()` inside `load_overlaid_source` would
hide the overlay from the second read, which is the one the provider runs
against. The preflight read therefore only looks at the overlay, and
`materialize_harness_prompt` consumes it, even when that read fails. A failed
preflight also clears it. The overlay still applies to exactly one transition
(test `the_retried_attempts_preflight_and_materialization_both_see_the_retry_overlay`).

**Bug fixed outside the planned files (`lib/src/composition/preflight.rs`).**
In plain `compose`, a `_loop_count` read introduced only by a transcluded
document surfaced as a generic `MarkdownError: interpolation failed`. The
shell-discovery walk wrapped it as `PreFlightDiscoveryFailed` before
composition could map it to `RemovedGrammar`. A `discovery_failed()` helper now
runs `removed_loop_read_in_composition` first at both walk sites. This closes
part of the Phase 2 gap "later-render detection does not reach the
context-free branch-discovery pass"; the error still reports `path: "render"`.

**Requirement → test**

| Requirement | Test |
| --- | --- |
| AC5 evaluation: native values, static keys, literal data | `executor/tests/retry_with_evaluation.rs::a_retry_overlay_keeps_native_values_static_keys_and_literal_data` |
| AC5 omitted/empty overlay; other parameters unchanged | `…::an_omitted_or_empty_retry_overlay_is_empty_and_the_other_parameters_are_unchanged` |
| AC5 `err` feeds the overlay | `…::the_err_global_feeds_a_failure_retry_overlay` |
| AC5 failure diagnostic (verb, document, nested path) | `…::a_retry_overlay_failure_names_retry_the_current_document_and_the_nested_path` |
| AC5 atomic failure; `no_error` cannot hide it | `…::a_failing_retry_overlay_is_atomic`, `…::no_error_does_not_suppress_a_retry_overlay_failure` |
| AC5 lifecycle-key templates rejected | `…::run_time_template_text_cannot_become_lifecycle_configuration_through_a_retry` |
| AC5 repeated retry | `loop_control/tests/overlay_layering.rs::{a_retry_overlay_applies_to_its_own_fresh_read_only, a_later_retry_replaces_an_unconsumed_retry_overlay}` |
| AC5 retry then resume | `…::a_resume_never_sees_a_retry_overlay` |
| AC5 proxy-owned document | `…::{a_retry_overlay_layers_over_the_retained_proxy_overlay, proxy_adoption_drops_an_unconsumed_retry_overlay}` |
| AC5 no disk persistence / no template re-evaluation | `…::{a_retry_overlay_never_writes_to_disk, a_retry_overlay_value_is_never_evaluated_as_a_template}` |
| AC5 refresh validation failure | `…::a_retry_overlay_the_schema_rejects_fails_the_refresh` |
| AC5 evaluation failure spends no attempt | `retry_resume.rs::a_failed_retry_overlay_spends_no_attempt_and_stages_nothing` |
| AC5 end to end (binary) | `cli/tests/l1/compose_retry_overlay.rs::a_retry_overlay_reaches_the_second_attempts_prompt` (prompts are exactly `["reason=first try", "reason=first try, then retried"]`; file bytes unchanged) |
| AC6 CLI boundary | `cli/tests/l1/compose_removed_grammar.rs`: `macro_rules!` rows `stack_key`, `action_aggregator`, `action_beside_when`, `say_first`, `communication_{message,sound,route}`, `loop_read_in_{frontmatter,body}` × `compose`/`inline-compose`/`sequence` (27 tests; exit 1, "removed grammar", authored path, removed spelling, replacement, launch marker absent); guard `every_table_row_generates_its_tests`; late render `a_retired_read_introduced_by_a_transclusion_is_rejected_at_render` |

Both new L1 modules are `#[cfg(unix)]` in `cli/tests/l1/main.rs` because
their provider stubs are `/bin/sh` scripts, following the existing stub L1
pattern. Windows coverage of the same contracts comes from the in-process
`overlay_layering.rs`/`retry_with_evaluation.rs` tests, which are not gated.

### Integration gates (macOS, this host)

- First `just test` in `claudine/`: one failure,
  `dispatch_inventory::dispatch_inventory_matches_committed_file` (1849 → 1851
  sites). The two new sites are `direct-ref` references, test fixtures naming
  a provider, not conditional dispatch. Regenerated
  `claudine/docs/providers/dispatch-inventory.json` with
  `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`.
- `just test` in `claudine/`: **8868 run, 8868 passed, 9 skipped** (the
  recipe's standing skips; Phase 2 ended at 8810).
- `just lint` in `claudine/` (incl. `lint-lifecycle-doc-facets`): passed.
- `just test` / `just lint` in `darkmatter/`: passed (see Wave 4 above).
- No other OS exercised. The changes are platform-neutral Rust (overlay
  merging, schema YAML, a DMLS path classifier); the two new `cfg(unix)` L1
  files follow the existing stub pattern. CI's Linux/macOS cells and the
  nightly Windows/WSL2 cells cover the rest.
- Not run: `just test-l2` (still fails until Phase 4 migrates
  `cli/tests/level2/**`, as recorded in Phase 2).

### Docs Phase 4 must update (found here, not edited)

The flow-control pages still describe the Phase 2–removed names, so they are
left for the planned Phase 4 rewrite rather than patched piecemeal:

- `flow-control-reference.md` "Passing values with `with:`" (~line 82–185): add
  `retry.with` (one transition's fresh re-read, layered on top of a retained
  proxy overlay so retry wins a shared key; a later retry without `with`, a
  resume, or a proxy adoption never sees it; an evaluation failure takes no
  transition and spends no budget; the diagnostic reads "`retry` to `<current
  document>`"). Rename `LifecycleProxyWith*` → `LifecycleWith*`, replace
  `LifecycleProxyOnlyParameter` with `LifecycleWithUnsupported`, and drop the
  `LifecycleStackAmbiguous` row. Say that `resume` takes no `with`.
- `flow-control.md`: add `with` to the `retry` parameters.
- `composition.md` (~line 1428, "keep the document's overlay"): a retry adds a
  one-read retry overlay.
- `lifecycle.md` (~line 863): include `retry … with` beside `proxy … with`.
- `pre-flight-checks.md`: a retired `_loop_*` read met during discovery now
  reports `RemovedGrammar` (`path: render`), not `PreFlightDiscoveryFailed`.

### Requirement-to-test summary (Phase 3)

AC4 (executor side), AC8, and AC17 (executor side): Wave 4 executor table.
AC5: Wave 5 table. AC6 (CLI boundary): `compose_removed_grammar.rs`. AC12:
Wave 4 editor section. No configuration Input Robustness Matrix applies
(text grammar; see Phase 1). No pre-existing failures remain; nothing was
skipped beyond the recipes' standing skips.

### Checkpoint

Selected-branch execution and ordered audio tests pass; the editor extension
loads and accepts the new grammar; parser and editor paths agree. The AC5
transition and AC6 CLI tests pass; no retry overlay survives past its
transition. Runtime, parser, and editor changes compile and test together.

## Phase 4

Phase 4 is Wave 6: authored prompt migration, the remaining fixture and L2
migration, and current docs and skills. Five subagents with disjoint files ran
in parallel (root prompts + route fixture + opt-in corpus test; research
template and fleets; `cli/tests/level2/**`; the five flow-control pages; the
other docs, READMEs, skills, and error wording). The orchestrator integrated
and ran the gates.

### Authored prompts (AC10, AC11)

- **Copied from migrated frozen copies.** `prompts/{clarify,format,implement,plan,review}.md`
  were byte-identical to their CLI frozen copies before Phase 2 migrated those
  copies (checked against `b3f8197e2^`), and `prompts/_reviews/cross-platform.md`
  to its lib frozen copy (`434c91b1d^`). Each shipped prompt is now its
  migrated frozen copy, so the frozen fixtures still match their sources.
- **Migrated by structure:** `_add/{add-context-variables,add-expressions}`,
  `_clarify/findings`, `_implement/{implement-plan,implement-review,implement-suggestions,implement-feature}`,
  `_pr/{diagnose,dirty,fix,open,push,triage}`, `_prompt`, `_reviews/{dry,feature-review}`,
  `commit`, `merge-conflicts`, `pr`. Non-mechanical decisions:
  - Former notification fields fired before the stack, so an event that had
    both became a list with those fields first, in their old fixed order
    (`implement-plan.md` `start` = `[message, when…]`, `success` =
    `[success, message, say, when…]`; `_pr/dirty.md`'s `with:` overlay of
    `commit.md`'s `success` = `[message, proxy]`).
  - `_reviews/dry.md`: the two complementary guards (`file_exists(file)` /
    `!file_exists(file)`) became one `when`/`then`/`else`; nothing between them
    can create the file.
  - `_reviews/feature-review.md`: its `finalize` had bare `stop` lines inside
    mappings and was invalid YAML at HEAD; they are `stop: null` inside the
    `when` bundles. In those bundles `info` now prints before `message`
    (canonical order).
  - `_implement/implement-plan.md`: `_loop_is_last` → `loop.is_last` in
    `pass_icon` and the body `::block when=`.
  - `_prompt.md` (the prompt-writing guide): its YAML example, "two action
    forms" bullet, and loop-value names describe the new grammar.
  - `_implement/implement-feature.md` (a draft outside the 24-file inventory)
    had a `- stack:` item; migrated so the internal corpus parses.
- `darkmatter/prompts/` and `claudine/prompts/` have no lifecycle grammar.
- **Research Markdown:** `claudine/docs/research/_TEMPLATE.md` and the 21
  remaining `*/_fleet.md` (agent-errors was done in Phase 2) use the
  agent-errors pattern; guards, order, and mutations unchanged. Fleets whose
  `failure:` was a notification-only dictionary stay root dictionaries.
  Observed, not changed: `local_runners/_fleet.md` `initialize` ends with an
  unguarded `stderr` + `skip`, so that event always skips — before and after
  the migration. `claudine-gen` (250/250) and `agent_errors_fleet` (2/2) pass;
  nothing else reads fleet lifecycle structure (`just research` passes the
  path to `claudine sequence`).

### Fixtures and tests

- **Route fixture (AC11):** `cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
  re-derived from the migrated shipped prompt minus seven side-effect lines
  (the `say` in `success`, the four `shell` items in the dirty-files guards,
  the `say` and `effect` in `failure`). The previous fixture had been derived
  from a version older than the author's `dddf5c988` and still carried the
  `epilog` sets and log-handoff guard; re-deriving drops them. Hashes refreshed
  with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli -F prompt-tests,test-fixtures --test prompts shipped_prompt_route_drift::`
  and re-run without the flag: 4/4.
- **Opt-in corpus (AC10):** new `cli/tests/prompts/shipped_prompts.rs::internal_lifecycle_corpus_parses_under_current_grammar`
  runs `parse_lifecycle_config` on every frontmatter `.md` under root,
  Claudine, and Darkmatter `prompts/`, the research template, and every
  research fleet; repository reads are literals joined onto `workspace_root()`
  inside the test. `DRAFT_LIFECYCLE_PROMPTS` excludes `prompts/fix.md` by name:
  it is a draft written in verbs that do not exist (`prep`, `interview`) and
  says so; migrating its grammar cannot make it parse.
- **L2 (`cli/tests/level2/**`):** 10 files migrated. No test deleted. Renamed
  to authored-order tests because their subject (notification fields before
  the stack) is gone:
  `level2_lifecycle_success_communication_fires_in_authored_order`,
  `level2_lifecycle_blocked_communication_fires_in_authored_order`,
  `…_success_error_downgrades_to_failure_preserving_earlier_communication`,
  `level2_lifecycle_success_communication_fires_beside_a_side_effect`.
  AC25 `proxy … with:` overlays that tried to install an `initialize` shell are
  now `{initialize: [{shell: …}]}`, still refused. Expected paths in
  `level2_typed_error_render_capture.rs` are now `initialize[*].proxy` /
  `failure[*].proxy`. `level2_ac28_lifecycle_lazy_roots.rs` loop items moved
  under `gate:`. Where YAML was a `\`-continued Rust string, the rewrite puts
  one YAML line per source line (large diff in `level2_lifecycle_control.rs`).
- **L1 residue audited, all intentional:** `loop_gate_ambient.rs` (negative
  `RemovedGrammar` tests and the "text is data" test),
  `prose_escape_guard.rs:7` (prose), `authored_text_rendering.rs` (`_loop_countx`
  is an unrelated property), `error_guards/source_scan.rs` and
  `cli/src/commands/wrap/sequence/task_run.rs` (Rust `stack` fields).
- The default L1 test that reads the route fixture
  (`compose_initialize_acceptance::`) passes 24/24. Dispatch inventory needed
  no regeneration (it scans only `lib/src`/`cli/src`).

### Docs and skills (AC13, AC14)

- **Flow-control pages** (`lifecycle.md` largely rewritten, `flow-control.md`,
  `flow-control-reference.md`, `looping.md`, `sequences.md`): list/dictionary
  events, `when`/`then`/`else` to five levels, canonical order, list-local
  directive placement, both `no_error` homes, branch validation vs selected
  execution, a removed-spellings table, audio in list position and message
  delivery draining, `loop.*` (with `fm.loop`, `claudine.outside-loop`, and
  `_loop_*` as a hard error; `state.loop` and the alias promise are gone), the
  gate on every pass, `retry.with` (a "Retry overlays" section with every
  Phase 3 rule), the `LifecycleWith*` names, `resume` takes no `with`,
  planned `break` with `code`, planned `defer`. A Mermaid diagram in
  `lifecycle.md` shows branch selection, a failed guard never choosing `else`,
  and a directive ending the event. Every `yaml`/`md` block was run through
  `claudine compose --dry-run --claude` (private `HOME`, `PLAYA_DRY_RUN=1`,
  private spool); only intended-negative examples fail. The executed example
  (`loop_gate_ambient::lifecycle_reference_loop_example_runs_as_written`) and
  Darkmatter's `current_root_documentation_contract` pass.
- **Other pages:** `getting-started/index.md`, `composition.md` (retry overlay
  over the retained proxy overlay), `frontmatter-properties.md` (`loop.*`
  table, communication verbs), `pre-flight-checks.md` (`RemovedGrammar` with
  `path: render` during discovery), `signal-handling.md` (the drop safety net
  as built), `state-management/side-effects.md`. `claudine/lib/README.md`
  gained a compatibility note (the README's existing convention for API
  removals).
- **Skills:** `SKILL.md`, `architecture.md` (new "Lifecycle grammar"
  subsection), `cli-commands.md`, `validations-and-handlers.md`,
  `research-contracts.md`, and one new `timeline.md` entry (existing entries
  untouched).
- **Drift found and corrected (code wins):**
  - looping docs listed `state`, `state.loop`, `state.seq` as reserved `set`
    names; `SET_ACTION_BLOCKLIST` reserves `loop`, `replace`, and the `_loop_*`
    names;
  - `loop:` also accepts `on_rate_limit` (row added);
  - TTS config lives in `~/.claudine/config.json`, not `claudine.toml`;
  - "Empty String Normalization" described deleted behavior (removed);
  - `with:` beside a short-form directive is `LifecycleUnknownVerb`, not
    `LifecycleStackAmbiguous`.
- **Source wording** (`lib/src/composition/error/{mod.rs,render/lifecycle.rs}`,
  `lifecycle/mod.rs`, `composition/types.rs`): `LifecycleDeferNotImplemented`
  already said `defer`; the stale "requeue" text was in the
  `LifecycleSetupPhaseRecoveryUnsupported` doc and in the unconstructed
  `LifecycleRequeueEnqueueFailed` message, now "lifecycle `defer` …" and marked
  planned (variant name kept). Doc comments and messages that still described
  the `{action: verb}` form, notification properties, or "top-level fires
  before the stack" now describe short/long form and stacks. No test asserted
  the old texts.

### Gates (macOS, this host)

| Gate | Result |
| --- | --- |
| `just test` in `claudine/` | 8868 run, 8868 passed, 9 skipped (standing recipe skips) |
| `just lint` in `claudine/` (incl. `lint-lifecycle-doc-facets`) | passed (linker `__eh_frame` warning is environmental) |
| `just test-l2` in `claudine/` | claudine-cli 282/282, claudine-gen 3/3; backends tmux (260) and WezTerm (20, off-screen `biscuit-bg` workspace); 0 skips |
| `just test` in `darkmatter/` | 9950 run, 9950 passed, 10 skipped (docs read by its contract tests) |
| opt-in `cargo nextest run -p claudine-cli -F prompt-tests,test-fixtures --test prompts` | 71 run, 63 passed, 8 failed (pre-existing, below) |
| L2 shipped-prompt rows with `prompt-tests` (`level2_lifecycle_shipped_implement_route_matches_direct_run`, `level2_shipped_implement_plan_*_commit_message_*`) | passed |

**Not run:** the two `level2_drain_ctrl_c_kitty.rs` tests (on macOS they
open a visible Kitty window; the no-focus rule forbids it). They are unrun,
not passing. No other OS: Phase 4 changes Markdown, test YAML strings, docs,
and error wording, with no `cfg`, path, or process behavior.

**Pre-existing opt-in failures (not caused by this feature):** seven tests
assert content of `prompts/_implement/implement-plan.md` that the author
removed in `dddf5c988` before this phase (HEAD's prompt contains none of
`epilog`, `_finish-before-exit`, "Final Phase"):
`shipped_prompt_corpus_parses_frontmatter`, `shipped_prompts_use_mapping_only_set`,
`shipped_implement_prompts_transclude_finish_before_exit`,
`shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`,
`shipped_implement_plan_final_phase_delivers_completion_flags`,
`shipped_implement_plan_warns_when_final_phase_exits_without_flags`,
`shipped_prompt_contract::shipped_implement_plan_launches_without_a_sibling_spec`.
The eighth, `shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan`,
looks for the body heading "Implement Review Suggestions" in a 20-row stderr
preview filled by the rule blocks; Phase 4 changed only that prompt's
frontmatter, and the routing itself ran correctly. Grammar evidence in the same
run: the new corpus test, all four route-drift guards, and all 13
`pr_flow_rehearsal` tests pass.

### Residue (AC16 pattern, current files)

Remaining hits outside snapshots: docs that name the removed spellings
(`lifecycle.md` removed-spellings table and validation row, `looping.md`,
`frontmatter-properties.md` "Replaces `_loop_*`" notes, `pre-flight-checks.md`,
skill `architecture.md`), negative tests (`grammar.rs`,
`compose_removed_grammar.rs`, `removed_loop_reads.rs`, `loop_gate_ambient.rs`,
`bindings/tests.rs`, `expression/tests.rs`), `reserved.rs`/`bindings.rs`/`parse.rs`
retired-name constants, Rust `stack` fields, provider hook tables
(`research/hooks/*.md`), and the out-of-scope draft `docs/schemas/partials/stack.yaml`.
The Wave 8 audit re-takes this list.

### Checkpoint

Migrated artifacts parse with zero grammar errors (the opt-in corpus test);
refreshed hashes pass without update mode; `lint-lifecycle-doc-facets` passes;
frozen fixtures still prove their behavior (`just test` green). Wave 6 is
complete.

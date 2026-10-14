---
total_phases: 5
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
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
# Lifecycle Grammar Implementation Plan

## Phase 1 — Scope and implementation contracts

### Necessary Rules

- **Prerequisite:** implement and verify `2026-10-18-fm-namespace` before grammar implementation. The inspected binding code still names `current_env`; a finalized prerequisite specification alone does not establish readiness. Use its host-declared binding mechanism for `loop`, and retain bare-name frontmatter lookup through `fm`.
- **Authority:** the finalized functional specification controls behavior. Its historical schema spike measures schema cost; recommendations in that report do not override the current parser-only five-level limit or authorize new schema infrastructure. No compatibility aliases or acceptance window are introduced.
- **Scope:** Claudine owns the parser, typed actions, executor, loop bindings, and migration. Darkmatter changes preserve its existing extension and editor integration. New authoritative Claudine schema files, `$defs`/`$ref` work, TTS preparation/prewarm, new sequence containers, `break`/`prep` implementation, and parser relocation remain outside this feature. `defer` keeps its existing not-implemented runtime result.
- **Diagnostic choices:** choose one typed removed-grammar `CompositionError` variant carrying authored path and replacement; choose shared `with` error names to replace the proxy-only family. Record the exact names before parallel work. Keep `LifecycleUnknownEffect` and pre-launch validation timing by injecting the effect-name predicate from a runtime caller.
- **Source identity:** dictionary expansion must preserve authored paths, including arbitrary map keys and array indices. Agree on the recursive action/conditional representation, walker API, and source-path storage before changing consumers; synthetic execution indices cannot become diagnostic locations.
- **Overlay contract:** retain proxy-owned overlays across refreshes; evaluate a retry overlay once in the source context and consume it on exactly one transition. Apply the proxy overlay beneath the retry overlay before target schema validation/composition. Later retry/resume operations must not inherit retry data. No new path-rebasing rule is introduced.
- **Input robustness matrix:** omit the configuration matrix because this work changes a Markdown/frontmatter text grammar and its editor mirror, expressly excluded by the planning instructions. Still test the specified empty-stack, malformed-shape, parameter-type, and branch-validation contracts. Preserve the existing YAML decoder and its duplicate-key policy; do not introduce another decoder or reinterpret arbitrary `set`/`with` data as grammar.
- **Spikes:** schedule no new spike. [Existing results](spike-results.md) already establish schema expressiveness and scale (39.9 MB resolved schema and roughly 4 seconds per validation for the full depth-five table). Repeating or widening those measurements does not inform this parser change. Wider DMLS performance measurements belong to an author ruling in the separate schema workstream, not a gate here.
- **Execution discipline:** use existing nextest-backed recipes and test-toolkit fixtures. No real provider, network, audible playback, or focus-taking windows in hermetic coverage. Preserve existing CI scope and event/environment scheduling; add no environment or gate. No commit, lifecycle-directory move, or `cargo fmt` is part of implementation authorization.

### Work and success

Replace notification dictionaries plus `stack`/`action` aggregators with a single ordered action model supporting compact bundles, recursive `when`/`then`/`else`, loop gates, retry overlays, and `loop.*` bindings. Carry the change through validation, approval, execution, diagnostics, existing editor integration, shipped prompts, frozen fixtures, docs, and skills.

Completion means all AC1–AC17 pass: authored order controls effects and audio publication; both branches validate while only the selected branch evaluates event-time operands; nested directives stop the whole event; retired grammar produces typed, correctly located errors; retry overlays are transient; the lift-ready files have no prohibited dependencies; every current artifact uses the new grammar; required scoped verification has passing evidence. The terminal state is **implementation complete, ready for review**.

Wave numbers are unique across the plan. A wave is a dependency step; tasks marked **parallel** can be assigned to separate subagents with disjoint file ownership. Complete a wave's checkpoint before dependent waves begin. Every subagent brief must state that the session tree is non-interactive. Tests for each changed contract land with its implementation, rather than being deferred to final verification.

### Wave 1 — Readiness and inventory

- [x] **Confirm prerequisite**
  - Verify the `fm` namespace, captured environment, reserved-root precedence, and host-binding APIs using the prerequisite's acceptance evidence and actual code. If absent, record the blocking dependency; do not implement it implicitly in this feature.
  - Read package instructions and the Claudine, rust-testing, and just skills. Load the OS skill before platform-specific edits or claims about unavailable OS evidence; load the terminal harness skill before changing L2 fixtures.
- [x] **Map migration**
  - Re-take AC16's inventory across root/Claudine/Darkmatter prompts, docs, library/CLI/DMLS Rust, fixtures, and Claudine skills. Also inspect structured `action` discriminator maps and communication parameter aliases; exclude legitimate loop `action`/`actions`, message verbs, and `resume.message`.
  - Create `implementation-log.md` beside this plan with the baseline inventory, file ownership, chosen diagnostic names, dependency evidence, and AC-to-test locations. Preserve spec/review snapshots, historical timeline entries, and provider-hook tables.
  - Trace consumers of `LifecycleStackItem`, notification fields, expression surfaces/source maps, `StackControl`/`ControlDispatch`, active-document refresh, and loop lookups. Assign shared error definitions and test registration files to one integration owner.
- [x] **Freeze contracts**
  - Record the node model, dictionary canonicalization, conditional depth accounting, effect-catalog injection boundary, authored-path representation, host binding surface, and retry-overlay transport/consumption seam.
  - Keep one executor path and reuse current expression parsing, overlay resolution, approval, and error-routing mechanisms. Do not add a separate interpreter or YAML decoder.

**Checkpoint:** prerequisite behavior is available; all necessary implementation choices are recorded; inventory and ownership are concrete; no unanswered design ruling prevents Wave 2. A prerequisite blocker is reported rather than bypassed.

## Phase 2 — Grammar and binding foundations

### Wave 2 — Shared action model

- [x] **Define recursive model**
  - Update `claudine/lib/src/composition/lifecycle/{actions.rs,action_shape.rs,signatures.rs,mod.rs}` to represent ordered leaf actions and conditional nodes with ordered branch bodies. Compact dictionaries expand into that model during parsing.
  - Add typed retry `with` data using the existing proxy-overlay value contract; remove communication payload aliases and unused `route` model fields. Preserve action signatures, literal/native-value rules, positional arity, defaults, `skip` placement, and planned directive behavior.
  - Introduce the agreed removed-grammar and shared-overlay diagnostics in composition errors/rendering; establish authored paths usable by parser, walker, executor, and editor tests. Update affected comments and compile-time consumers sufficiently to make the next wave reviewable.
  - Put runtime settings and the effect-name catalog adapter outside the six lift-ready files. Add AC9's guard, including textual references in those files, and verify unchanged unknown-effect diagnostic timing.

**Checkpoint:** shared node, path, binding, and overlay interfaces are stable; action inventory and lift-ready guard agree with the specification. Parallel owners receive these interfaces before Wave 3.

### Wave 3 — Parser and loop bindings (parallel)

- [x] **Implement stack grammar** — **parallel**, lifecycle parser/source-map/validator owner
  - Rewrite `parse.rs` for flat lists and root dictionaries across all six events and task `setup`/`teardown`. Parse recursive branches with root guards at depth one, siblings at the same depth, and a path-bearing error at depth six.
  - Expand dictionaries as `set`, ordered communication (`stdout`, `stderr`, `info`, `warn`, `success`, `message`, `notify`, `effect`, `say`), `shell`, ASCII-sorted canonical side-effect/expression names, then one control directive. Reject `say`/`speak` collisions, root `when`, multiple directives, and multi-verb long forms.
  - Enforce list-local control-last placement; a guarded item never counts as unconditional. Enforce both `no_error` homes, YAML boolean typing, inner/sibling conflict rejection, prohibition beside `then`/`else`, and exclusion from standalone or bundled `set` failures.
  - Accept omitted/null/empty root stacks; reject empty explicitly supplied branches, orphan `then`/`else`, guard-only/flag-only items, wrong shapes, unknown keys, unknown parameters, and invalid arity. Preserve object-argument semantics through whole-value interpolation.
  - Parse loop loose verbs or `gate` through the same grammar; reject mixing them and root `when`. Check removed keys before extracting lifecycle concerns; iteration `action`/`actions` remain controls. Keep gate execution on every condition check, including the exit pass.
  - Extend the shared expression walker and `source_map.rs` through both branches, bundles, mutations, and `retry.with`. Both branches undergo binding, expression-syntax, nested-span, and shell-approval validation; nested `initialize` shells and shell-valued `set` fail even in dead branches.
  - Add focused tests for AC1–AC6 and AC17, with authored paths such as `failure[0].else[1].shell.command`; exercise every stack home, not only document events.
- [x] **Introduce loop bindings** — **parallel**, binding/loop/expression owner
  - Update lifecycle binding views, runtime context plumbing, `looping/expression.rs`, `looping/engine.rs`, `reserved.rs`, reporting queries, and `cli/src/commands/compose/loop_run.rs` to supply the five unchanged values through captured host bindings under `loop`.
  - Cover bare/dotted/static-bracket access; `fm.loop` remains configuration. Missing ambient members cannot fall back to frontmatter, and declared `loop` outside a loop produces the typed scope diagnostic. Plain Darkmatter composition gains no ambient loop global.
  - Remove `_loop_*` seeding and detect expression reads of the five exact retired names via shared parsing/validation, including explicit `fm` forms and static brackets. Check discoverable frontmatter/body reads before launch and late-introduced expressions when prepared for rendering.
  - Preserve literals, ordinary Markdown/code examples, unrelated `_loop_notes`, and ordinary semantics for dynamically computed keys. Migrate `gate_ambient.rs` rather than reducing its coverage, and update Darkmatter's affected identifier tests without installing fixed loop roots there.
  - Add AC6/AC7 scope and lookup tests across composition, lifecycle, approval, and condition-evaluation surfaces. Route shared error-definition or walker edits through the integration owner.

**Checkpoint:** focused parser/binding tests and AC9 pass. Both branches and every stack home traverse one validation/source-map contract. Record grammar fixture examples for runtime and editor owners.

## Phase 3 — Execution and editor integration

### Wave 4 — Runtime and editor behavior (parallel)

- [x] **Execute ordered nodes** — **parallel**, lifecycle executor/audio owner
  - Adapt `executor.rs`, runtime context, and audio helpers to dispatch the shared ordered nodes serially. Evaluate each reached guard exactly once against live state using existing truthiness; only the selected body evaluates operands. Guard failure follows existing failure routing and cannot select `else`.
  - Preserve `set` snapshot/commit semantics and visibility to following actions. Return a nested control result through every enclosing list immediately; retain existing catch-event routing and exactly-once `finalize` behavior.
  - Remove the notification-field execution path, `say_first`, and `audio_phases`. Publish each audio action at its position; preserve Playa queue durability, tracked message registration, and ordinary CLI shutdown draining.
  - Prove ordered publication in both directions with a fake publisher, continued durability after transitions, selected-branch effects only, nested event termination, mutation visibility, and dispatch-versus-expression failure behavior under `no_error` (AC4, AC8, AC17).
  - For durable queue coverage, use `LockedAudioSpool` and stub TTS preparation; disable dry run only inside that isolated fixture while preventing playback. Separately assert dry-run prompt executions publish no audio.
- [x] **Preserve editor integration** — **parallel**, Darkmatter schema/DMLS owner
  - Update `darkmatter/docs/schemas/claudine.yaml` and `claudine-types.yaml` in place for new-grammar examples and recursive bodies. Keep existing paths, imports, extension loading, and `lifecycle_inventory_matches_the_authored_claudine_schema` active.
  - Permit legal deep bodies loosely where existing schema types cannot express them; parser enforcement owns depth five and list ordering. Do not introduce full action typing or new authoritative schema files.
  - Update `darkmatter/dmls/src/diagnostics/nested_span.rs`, its tests, and `overlay/frontmatter.rs` pointers for list items, branch bodies, root dictionaries, and `gate`; remove old structural fields.
  - Migrate the sequence-descent fixture and affected extension/configuration examples. Run targeted extension/path tests and `mapping_only_corpus`; re-bless only an explained intentional diagnostic delta, with the reason in the implementation log (AC12).

**Checkpoint:** selected-branch execution and ordered audio tests pass; editor extension remains loadable and accepts legal new grammar; parser/editor paths agree. Runtime edits are integrated before the retry owner edits the executor.

### Wave 5 — Retry overlay integration

- [x] **Consume retry overlays**
  - Generalize the existing source-side proxy overlay evaluation for `retry.with` without changing proxy behavior. Preserve native values, static keys, literal target data, captured context, and exact nested diagnostic paths; reject whole-map interpolation and executable spans in lifecycle-key values.
  - Carry resolved overlays through `StackControl`, `control.rs` dispatch, runtime decisions, wrapper `loop_control/control_dispatch.rs`, active-state refresh, and compose preparation. Apply before the fresh read's schema validation/composition and consume only the transition's retry overlay.
  - Prove evaluation failure installs no overlay and spends no retry transition; `no_error` cannot hide it. Preserve additional-attempt budgets, delay/backoff, pre/post-launch re-entry, initialize-once behavior, and resume session compatibility checks.
  - With fixture-owned documents and fake providers, cover repeated retry, retry then resume, and retry on a proxy-owned active document; prove proxy retention beneath retry precedence, later retry without overlay, no disk persistence, no template re-evaluation, and refresh validation failure (AC5).
  - Finish CLI-boundary diagnostics for removed grammar in `compose`, `inline-compose`, and `sequence`, following `compose_removed_validation_keys.rs`; assert no provider launch for static defects and typed errors at late render preparation (AC6).

**Checkpoint:** AC5/AC6 transition and CLI tests pass; no stale retry overlay survives into subsequent transitions. Runtime, parser, and editor branches compile together.

## Phase 4 — Repository migration and current documentation

### Wave 6 — Artifact migration (parallel)

- [x] **Migrate authored prompts** — **parallel**, live Markdown artifact owner
  - Rewrite affected root `prompts/`, `claudine/prompts/`, `darkmatter/prompts/`, research `_TEMPLATE.md`, and every research `_fleet.md` found in Wave 1. Preserve behavior, guard selection, action order, mutations, and transition intent; inspect structure rather than replacing strings blindly.
  - Convert `_prompt.md` and `_implement/implement-plan.md` loop expressions in the same pass as their lifecycle grammar. Keep internal live-prompt tests opt-in and out of default L1/CI.
- [x] **Migrate frozen fixtures** — **parallel**, Claudine test/fixture owner
  - Update lib/CLI frozen prompts, `nested_span_regression/commit.md`, and the shipped implementation-route fixture, preserving each tested behavior. Migrate remaining library, CLI L1, sequence-task, and L2 authored grammar fixtures; retain negative tests for removed spellings.
  - Extend `cli/tests/prompts/shipped_prompts.rs` to parse every internal `.md` and research fleet/template; extend default frozen-artifact coverage separately. Use resolvable repository-read spellings from rust-testing and register new tests in existing consolidated targets.
  - Keep a representative hermetic CLI case recording branch-specific effects in order and proving the other branch performs no work. Use `CliProcessFixture` and existing recorder seams; real provider binaries/home/audio must remain unreachable.
  - After the authored-prompt owner finishes the source prompt and the route fixture delta is reviewed, refresh its pinned hashes from the repository root with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli -F prompt-tests,test-fixtures --test prompts shipped_prompt_route_drift::`. Rerun without the update flag to prove the final pin (AC10/AC11).
- [x] **Update current guidance** — **parallel**, docs/skills owner
  - Update Claudine READMEs and lifecycle, flow-control, flow-control-reference, looping, sequences, side-effects, frontmatter-properties, and getting-started pages to describe actual grammar and `loop.*`, with compact examples. Task setup/teardown remain stacks.
  - Remove looping's `state.loop` and one-release alias claims. Document canonical dictionary order, branch validation versus selected execution, both `no_error` homes, retry overlay lifetime/precedence, audio publication completion, and delivery tracking.
  - Correct planned `break` parameter documentation to include `code`, and update stale defer diagnostic wording from "requeue" while retaining its planned status. Mark all unbuilt behavior as planned; topic docs must not refer to feature/fix snapshots.
  - Sweep `.claude/skills/claudine/` outside `timeline.md` for removed grammar and update architecture/workflow guidance. Review behavior-changing symbol docs/comments in touched code; record detected drift and its correction. Dependencies docs change only if a crate dependency actually changes.
  - Use a Mermaid lifecycle/branch-flow diagram where it makes execution and event termination easier for a new developer to understand (AC13/AC14).

The live-prompt owner owns research/template Markdown; the docs owner owns getting-started and explanatory pages; the frozen-fixture owner owns Claudine test sources/fixtures. Darkmatter Rust/editor fixtures stay with the Wave 4 owner. Source prompt completion precedes hash refresh within this wave; shared test registrations and log merges belong to the integration owner.

**Checkpoint:** migrated artifact parsers report zero grammar errors; refreshed hashes pass without update mode; docs facet lint passes; frozen fixtures still prove their original behavior.

## Phase 5 — Acceptance evidence and review handoff

### Wave 7 — Scoped verification (parallel)

- [x] **Verify Claudine behavior** — **parallel**, Claudine validation owner
  - Run `just test`, `just test-l2`, and `just lint` in `claudine/`, including `just lint-lifecycle-doc-facets`. Use recipe-managed L2 harnesses with no focus acquisition; record backend identities and any skips rather than counting skipped coverage as execution.
  - Confirm AC1–AC9/AC17 results, removed-diagnostic paths, branch approval and initialization shell rejection, audio isolation, message draining, and retry/resume/proxy invariants. Run required compile checks for affected declared target kinds through existing recipes/plans.
- [x] **Verify Darkmatter behavior** — **parallel**, Darkmatter validation owner
  - Run `just test` and `just lint` in `darkmatter/`; the existing recipe includes DMLS. Confirm extension loading/inventory, classifier/pointer regressions, identifier behavior, and mapping corpus evidence (AC12).
  - Keep any re-bless reason and before/after diagnostic interpretation available to reviewers; missing validation cannot be accepted as a snapshot update.

### Wave 8 — Final acceptance audit

- [ ] **Inspect prompt results**
  - After Wave 7 and hash refresh, run root `just test-prompts` and inspect its reported results; it exits zero even on failures. Require the migrated lifecycle corpus, representative execution, and route-drift checks to pass. Record unrelated draft-prompt failures separately without expanding scope (AC10/AC11/AC15).
  - Verify live prompt tests remain opt-in and ordinary behavior tests read frozen copies; no CI scheduling changes are needed.
- [ ] **Audit migration residue**
  - Run AC16's `rg` pattern `^\s+stack:|say_first|^\s+- action:|_loop_(count|is_first|is_last|last_output|last_exit_code)` over `prompts/`, `claudine/docs/`, `claudine/lib/`, `claudine/cli/`, `darkmatter/docs/`, `darkmatter/dmls/`, `darkmatter/lib/`, and `.claude/skills/claudine/`.
  - Also inspect old action discriminator maps and communication `message`/`sound`/`route` parameters by context. List every remaining hit and reason in the implementation log; only historical/snapshot records, removed-grammar tests/diagnostics, and literal-data coverage may remain intentionally.
  - Verify AC13/AC14 guidance and AC9's forbidden-reference guard. Review source comments with behavior changes and confirm the spec/spike snapshots were preserved.
- [ ] **Assemble review evidence**
  - Fill the AC1–AC17 ledger with exact test locations and observed outcomes. Record commands, environments, failures resolved, harness skips, evidence reuse, and departures from the spec in `implementation-log.md`.
  - Preserve portability to macOS, Linux, native Windows, and WSL2. Use qualifying evidence per environment and existing required-cell scheduling; load the OS skill before declaring an environment unavailable. Do not add speculative environment runs or gates. If a push is separately authorized, review `just ci-local --plan` then.
  - Review the final diff for scope, fixture behavior, public docs, and accidental changes. Mark completed plan tasks and phase metadata accurately, and report implementation complete, ready for review; leave lifecycle closure to the author.

**Final checkpoint:** all AC1–AC17 have passing required evidence or an explicitly reported unresolved blocker; unrelated informational prompt failures are distinguished; no new approval flow, commit, or feature-directory move is performed.

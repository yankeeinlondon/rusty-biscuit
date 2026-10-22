---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-27-darkmatter-lifecycle-ownership/spec.md"
plan: "claudine/features/2026-10-27-darkmatter-lifecycle-ownership/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/lifecycle_traces.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/fixtures/lifecycle_traces/
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/mod.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/engine.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/profiles.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/publisher.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/protocol_tests.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
    - darkmatter
    - claudine
    - darkmatter-cli
    - dmls
human_review: false
source_files_during_phase_2:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/prep_context.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-null.md
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-null.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-present.md
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-present.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-preflight-blocked.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-preflight-blocked.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-start.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-start.trace
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/error_guards.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/lifecycle_traces.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/composition/coordinator/handoff.rs
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/binding_checks.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/cancellation.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/effects.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/diagnostics/error_kind.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/diagnostics/restored.rs
    - claudine/lib/src/diagnostics/restored/tests.rs
    - claudine/lib/src/diagnostics/snapshot.rs
    - claudine/lib/src/harness/runtime.rs
    - claudine/lib/src/harness/timeout.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/mcp/import.rs
    - claudine/lib/src/reporting/error.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/lib/tests/l1/path_lookup_guard.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/lifecycle/action_shape.rs
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/lifecycle/actions/tests.rs
    - darkmatter/lib/src/lifecycle/authored_order.rs
    - darkmatter/lib/src/lifecycle/cancellation.rs
    - darkmatter/lib/src/lifecycle/cancellation/tests.rs
    - darkmatter/lib/src/lifecycle/context.rs
    - darkmatter/lib/src/lifecycle/context/tests.rs
    - darkmatter/lib/src/lifecycle/control.rs
    - darkmatter/lib/src/lifecycle/control/tests.rs
    - darkmatter/lib/src/lifecycle/duration.rs
    - darkmatter/lib/src/lifecycle/effects.rs
    - darkmatter/lib/src/lifecycle/effects/tests.rs
    - darkmatter/lib/src/lifecycle/envelope.rs
    - darkmatter/lib/src/lifecycle/envelope/tests.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/executor/outcome.rs
    - darkmatter/lib/src/lifecycle/executor/shell.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/run_guard.rs
    - darkmatter/lib/src/lifecycle/run_guard/tests.rs
    - darkmatter/lib/src/lifecycle/run.rs
    - darkmatter/lib/src/lifecycle/run/protocol.rs
    - darkmatter/lib/src/lifecycle/runtime.rs
    - darkmatter/lib/src/lifecycle/runtime/tests.rs
    - darkmatter/lib/src/lifecycle/services.rs
    - darkmatter/lib/src/lifecycle/services/tests.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/tests.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/markdown/compose/context/merge.rs
    - darkmatter/lib/tests/l1/dependency_direction_guard.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/engine.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/mod.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/profiles.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/protocol_tests.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs
    - darkmatter/lib/tests/l1/lifecycle_run_protocol.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_lookup_guard.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/signal-handling.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/validations-and-handlers.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document-unauthored.md
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document-unauthored.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document.md
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-sequence.md
    - claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-sequence.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/defer-in-preflight-blocked.md
    - claudine/cli/tests/fixtures/lifecycle_traces/defer-in-preflight-blocked.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit-after-launch.md
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit-after-launch.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit.hop.template
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit.md
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-in-initialize.md
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-in-initialize.target.md
    - claudine/cli/tests/fixtures/lifecycle_traces/proxy-in-initialize.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/resume-in-initialize.md
    - claudine/cli/tests/fixtures/lifecycle_traces/resume-in-initialize.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/resume-in-preflight-blocked.md
    - claudine/cli/tests/fixtures/lifecycle_traces/resume-in-preflight-blocked.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-group-leg.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-group-leg.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-root-sequence.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-root-sequence.trace
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/level1_approval_order_pty.rs
    - claudine/cli/tests/l1/lifecycle_initialize_zero_attempts.rs
    - claudine/cli/tests/l1/lifecycle_one_implementation_guard.rs
    - claudine/cli/tests/l1/lifecycle_renamed_events.rs
    - claudine/cli/tests/l1/lifecycle_root_preparation.rs
    - claudine/cli/tests/l1/lifecycle_traces.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/composition/coordinator/invocation.rs
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/lifecycle_preflight.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/guarded_shell_sites.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/checks.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/tests/common/lifecycle_ownership_scan.rs
    - claudine/lib/tests/l1/diagnostic_detail_conformance.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/lib/src/lifecycle/control.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/policy.rs
    - darkmatter/lib/src/lifecycle/policy/tests.rs
    - darkmatter/lib/src/lifecycle/preflight.rs
    - darkmatter/lib/src/lifecycle/preflight/candidates.rs
    - darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
    - darkmatter/lib/src/lifecycle/preflight/tests.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/registry/tests.rs
    - darkmatter/lib/src/lifecycle/run_guard.rs
    - darkmatter/lib/src/lifecycle/run_guard/tests.rs
    - darkmatter/lib/src/lifecycle/run.rs
    - darkmatter/lib/src/lifecycle/run/protocol.rs
    - darkmatter/lib/src/lifecycle/runtime.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/lifecycle/tests.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/engine.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/mod.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/profiles.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs
    - darkmatter/lib/tests/l1/lifecycle_run_protocol.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/flow-control/lifecycle.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/lifecycle.md
source_files_during_phase_4:
    - Cargo.lock
    - claudine/cli/Cargo.toml
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/composition/schema/globals.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_trigger_selection_parity.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/schemas/claudine.yaml
    - claudine/schemas/globals.yaml
    - claudine/schemas/inline-compose.yaml
    - claudine/schemas/sequence.yaml
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/cli/tests/l1/clean_schema.rs
    - darkmatter/cli/tests/l1/compose_dasherized_identifiers.rs
    - darkmatter/cli/tests/l1/compose_lifecycle.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/cli/tests/l1/schema_trigger_rename.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate_baseline_selection.rs
    - darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/standalone_baseline.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/tests/l1/content_policy_baseline.rs
    - darkmatter/dmls/tests/l1/host_globals.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
    - darkmatter/dmls/tests/l1/startup_log_lines.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/dmls/tests/l1/strict_mode_recovery_spike.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/lib/src/lifecycle/envelope.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/md_host/tests.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/registry/tests.rs
    - darkmatter/lib/src/lifecycle/runtime.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/content_policy_disk_baseline.rs
    - darkmatter/lib/tests/l1/content_policy_embedded_baseline.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/trigger_match_grammar.rs
    - darkmatter/lib/tests/l1/trigger_selection_parity.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/schemas/darkmatter.yaml
    - darkmatter/schemas/partials/context-variables.yaml
    - darkmatter/schemas/partials/error.yaml
    - darkmatter/schemas/partials/fm.yaml
    - darkmatter/schemas/partials/style.yaml
    - darkmatter/schemas/partials/timing.yaml
    - darkmatter/schemas/partials/tracking.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/inline-compose.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/sequence.trigger.yaml
    - docs/drafts/memory.yaml
    - schemas/memory.yaml
docs_updated_during_phase_4:
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/state-management/context-variables.md
    - claudine/docs/topics/state-management/drift.md
    - claudine/schemas/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - darkmatter/cli/README.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/inline/fm-shell-expansion.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/schemas/darkmatter-schema.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/schema-targeting.md
    - darkmatter/docs/topics/state-management/context-variables.md
    - darkmatter/docs/topics/state-management/index.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/os/windows-paths.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
    Phase 4 is done; read the "## Phase 4" log sections before Phase 5. Key facts:
    - md host: `darkmatter/lib/src/lifecycle/md_host.rs` (profile, `MdStackHost`, `MdShellRunner`, `MdDiagnostics`, `drive` over `Run`) and CLI `darkmatter/cli/src/commands/compose/lifecycle.rs`. Only a root document authoring a core event key takes the lifecycle path. Tolerance is `UnknownVerbPolicy::IgnoreUnprovided(UNPROVIDED_VERBS)` (inert `Ignored` nodes, warned once per reached occurrence); Claudine stays `Reject`. CLI tests: `darkmatter/cli/tests/l1/compose_lifecycle.rs` (27). `md compose` has no `--dry-run` flag; "dry runs fire nothing" is proven only for report-only `--shell`.
    - Global catalog: `darkmatter/schemas/darkmatter.yaml` (+ partials incl. new `timing.yaml`) is embedded via `embedded.rs::SHIPPED`; loader `darkmatter/lib/src/markdown/schemas/catalog.rs` (`GlobalCatalog`, `is_global_catalog`, `host_catalogs`). `darkmatter/docs/schemas/darkmatter.yaml` is deleted. Claudine host catalog: `claudine/schemas/globals.yaml`, embedded by `claudine/lib/src/composition/schema/globals.rs`.
    - Editor projection still to do: DMLS uses the baseline `BindingView` for `seq`/`loop`/`group` availability (no per-event availability yet), and DMLS silently drops a host catalog that fails to load.
    - Triggers: `kind: schema-trigger`, top-level list AND, `group:` OR, `any:`/`trigger-schema` are `SchemaError::TriggerRenamed`; a trigger with no `$schema` now loads as matcher-only. `--no-trigger-schemas` is rejected in `darkmatter/cli/src/main.rs::reject_renamed_flags`. Spelling guard: `darkmatter/lib/tests/l1/trigger_spelling_guard.rs` (allowlist fails when stale). Phase 6 docs must update its allowlisted `definition.md`/skill text if they change.
    - Schema projection (Phase 5) is not started: Darkmatter's baseline does not yet type `initialize`/`composed`/... with md action unions.
    - Known load-sensitive tests: dmls `searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`, claudine `prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done`. Both pass alone.
---

# Implementation Log for 2026-10-27-darkmatter-lifecycle-ownership (6 phases)

## Phase 1

### Baseline inventory (Wave 1)

- **Starting revision:** `f3381eb5cb98102e26159cef5b690b5a5939ba27` on `feat/pipeline` (2026-10-20). This is the behavioral baseline for every trace captured in this phase.
- **Prerequisite gate — passed.** `2026-10-26-task-inputs-and-providers` closed in `2db75c81b` and sits in `_completed`; its transitive prerequisites `2026-10-25-loops-and-flow-control`, `2026-10-24-sequence-containers`, `2026-10-20-lifecycle-grammar`, `2026-10-23-shell-template-approval`, `2026-10-21-claudine-schemas`, and `2026-10-18-fm-namespace` are all in `claudine/features/_completed/`. The running code exposes the flat grammar (`when`/`then`/`else`), `cancelled` and container lifecycles, `seq`/`loop`/`group` roots, `break`/`prep`/task `when:`, task `with:`/per-task providers, schema action unions, and typed shell templates (all exercised by the existing L1 suites listed below).
- **Finding — `2026-10-16-type-narrowing` has NOT landed.** The spec lists it in `depends-on` (section 4's `TypeScope` placeholder audit and acceptance criterion 23). It is still `status: draft-spec`, `implemented: false` in `claudine/fixes/`, and no `TypeScope` type exists in `darkmatter/lib/src` or `claudine/lib/src`. It is not a transitive prerequisite of the task-inputs gate the plan names, so Phase 1 proceeds; Phase 3 (preflight `TypeScope` reads) and criterion 23 cannot be satisfied until it lands. Raised for human review.
- **Lifecycle code at the baseline:** about 50,000 lines across `claudine/lib/src/composition/lifecycle/` (10,787 in its top-level `.rs` files), `claudine/lib/src/composition/looping/`, and `claudine/cli/src/commands/wrap/harness_orch/loop_control/`.

#### Lifecycle caller inventory (production paths that drive the engine)

| Entry symbol | Callers (excluding tests) | Path(s) |
|---|---|---|
| `LifecycleRunGuard::new` | `cli/.../wrap/composition/pipeline.rs`, `cli/.../wrap/wrapper_stages.rs`, `cli/.../compose/prep.rs`, `cli/.../wrap/sequence/container.rs`, `lib/.../looping/engine.rs`, `lib/.../sequence/task/{group,sequence}.rs` | direct + inline compose, wrapper stages, `prep`, root sequence container, loop engine, nested group/sequence |
| `LifecycleCatchProtocol::new` | `cli/.../wrap/composition/{pipeline,preflight}.rs`, `cli/.../loop_control/error_routing.rs`, `lib/.../looping/engine.rs` | compose preflight catch, harness error routing, loop catch |
| `decide_lifecycle_transition` | `cli/.../wrap/composition/{pipeline,preflight}.rs`, `cli/.../harness_orch/loop_control{,/control_dispatch}.rs`, `lib/.../lifecycle/runtime.rs` | terminal recovery decisions |
| `drive_terminal_recovery` | `cli/.../harness_orch/loop_control{,/control_dispatch}.rs` | retry/resume/proxy dispatch (before finalize) |
| `decide_control` | `lib/.../lifecycle/{control,runtime}.rs` | retry/resume admission |
| `run_event_stack` | `cli/.../wrap/composition/preflight.rs`, `cli/.../loop_control/{error_routing,lifecycle_events}.rs`, `lib/.../lifecycle/runtime.rs`, `lib/.../looping/engine.rs` | every event stack run |
| `ContainerRunner` | `cli/.../wrap/sequence/container.rs`, `lib/.../looping/container.rs`, `lib/.../sequence/task/{group,sequence}.rs` | group/sequence container events |
| `execute_loop_with_lifecycle` | `cli/.../compose/loop_run.rs`, `lib/.../looping/engine.rs` | document loops |

Task paths (`cli/src/commands/wrap/sequence/task_run.rs`: `run_step_task`, `open_member_run`, `open_leg_run`, `run_looping_document`) and recovery paths (proxy adoption in `loop_control`, retry/resume reread in `wrap/composition/pipeline.rs`) reach the engine through the symbols above.

#### Acceptance-criterion ownership map

| AC | Owner phase / task | Evidence (fixture or test) |
|---|---|---|
| 1 Stage 1 gate | P1 trace capture + prototype + verdict | `claudine/cli/tests/l1/lifecycle_traces.rs`, `darkmatter/lib/tests/l1/lifecycle_boundary/`, verdict below |
| 2 No Claudine behavior change | P2 bridge, P3 parity | existing Claudine lib/CLI suites unchanged + Stage 1 trace replay |
| 3 Dependency direction | P2 Claudine bridge | dependency guard over Darkmatter lib/CLI/DMLS manifests and imports |
| 4 One implementation | P3 driver migration | removed-symbol source guard |
| 5 Host names | P3 host registry | registry + parse tests (`composed:`/`failed:` in Claudine docs) |
| 6 Registration checks | P3 host registry (prototyped P1) | registry unit tests; prototype `protocol_tests.rs` |
| 7 Prototype host | P1 boundary prototype | `lifecycle_boundary/publisher.rs` |
| 8 Shell invariants | P3 preflight integration, P5 invocation | both-host shell tests; trace `shell-*` scenarios |
| 9 Preflight host checks | P3 preflight integration | `blocked-preflight` trace + existing approval-order tests |
| 10 Section 8 behavior | P2 engine, P3 parity | Stage 1 traces (`blocked-refused-handoff`, `resume-incompatible`, `skip`, `retry-in-finalize`, …) under the new owner |
| 11 `md compose` events | P4 compose host, P5 invocation | darkmatter CLI L1 |
| 12 Passive surfaces | P5 editor projection | recording fake services |
| 13 Editor typing | P5 editor + schema projection | DMLS tests |
| 14 Global catalog | P4 global catalog | catalog tests + matrix |
| 15 Host globals | P4 global catalog, P5 editor | host catalog + BindingView parity |
| 16 Trigger kind | P4 trigger migration | trigger scan/DMLS tests + matrix |
| 17 Trigger flag | P4 trigger migration | CLI flag tests |
| 18 AND/OR | P4 trigger migration | matcher tests + migrated-file selection parity |
| 19 Spike tests | P2 bridge, P6 final | both `lifecycle_control_flow_spike` tests |
| 20 Docs | P6 documentation | Stage 5 pages |
| 21 Boundaries / output failures | P4 compose host, P5 invocation | darkmatter CLI L1 with failing writers / fake cancellation |
| 22 Approval / tolerant parsing | P3 preflight, P4 compose host, P5 schema projection | approval + ignored-action tests |
| 23 Narrowing survives | blocked on `2026-10-16-type-narrowing` | — (see finding above) |

### Trace capture (Wave 2)

- **Boundary:** the whole `claudine` binary (F3, L1 via `CliProcessFixture`), driven by `claudine compose` / `claudine sequence` with the Unix `launch_recorder` stub provider. These are production paths, not the historical model.
- **Files:** `claudine/cli/tests/l1/lifecycle_traces.rs` (declared `#[cfg(unix)] mod lifecycle_traces;` in `claudine/cli/tests/l1/main.rs`), fixtures in `claudine/cli/tests/fixtures/lifecycle_traces/` (30 `<id>.md` + `<id>.trace`, plus `target`/`step` helper documents).
- **Trace channel:** every lifecycle stack appends to one `trace.log` with the `append_line` side effect (`event <name>`, `err <code | uncoded: msg | none>`, `state <k>=<v>`); lifecycle `shell` actions append `shell <event>` only when shells are unlocked; the stub provider appends `work <fresh|resume> <status>`; sequence `shell:` tasks append `task <name> …`; the test appends `exit <code>`. Container subjects are prefixed (`seq:`, group/leg names, `target:`, `step:`). Each `.trace` carries `# given:` host facts used by the replay.
- **Tests:** one `#[test]` per scenario from `macro_rules!` (fixture `include_str!` literals inside each test fn for CI test-input mapping), plus `every_trace_fixture_has_exactly_one_test`. 31 tests, each 0.02–2.0 s. A mutation check (`skip.trace` edited to `exit 7`) failed as expected and was restored byte-identical.
- **Scenarios (30):** success; blocked-preflight; blocked-refused-handoff; catch-{initialize,start,blocked,success,failure,finalize,loop}; downgrade-{success,blocked}; finalize-error; retry-{admitted,exhausted,in-finalize}; resume-{admitted,no-session,incompatible}; proxy-adopted; proxy-target-bootstrap-failed; loop-break; cancelled-{with-handler,without-handler,not-opted-in}; skip; sequence-nested-group; shell-locked-blocked; shell-unlocked-blocked-after-start; shell-relocked-after-proxy. Cancellation uses deterministic sibling-failure cancellation (`fail_fast` group, marker-file synchronization, no real signals); "without handler" is a nested container that opted in with `cancelled: []` but has no stack.
- **Coverage gap:** the module is Unix-only, like its siblings `loop_break` and `lifecycle_directive_matrix`, because `launch_recorder` is Unix-only and the shells are POSIX. Native Windows and WSL2 parity of these traces is not covered by Phase 1. The darkmatter replay reads the trace text on every OS.

#### Departures from spec section 8 (the code wins; fixtures keep observed behavior)

| # | Scenario | Spec reading | Observed at `f3381eb5c` |
|---|---|---|---|
| D1 | catch-initialize | pre-composition failure → `blocked` | an evaluation error in `initialize` fires `failure` then `finalize` (setup catch goes to the failed outcome alternative) |
| D2 | catch-blocked | catch-only cleanup, unspecified step | a raising guard in `blocked` continues to `failure` then `finalize`; the later error replaces the earlier |
| D3 | catch-success | only explicit `error:` covered | an evaluation error in `success` does **not** run `failure`; `finalize` sees the error; exit 1 (explicit `error:` does downgrade) |
| D4 | retry-in-finalize | "finalize runs once when the lifecycle ends" | finalize runs for attempt 1, then the retried attempt runs its own `start`…`finalize`; never a second finalize for the old attempt |
| D5 | resume-no-session | refused recovery still finalizes | `resume` in `start` before launch raises `ResumeWithoutSession`, exits 1, runs neither `failure` nor `finalize` |
| D6 | resume-incompatible | — (routing matches) | routing is `start` → `failure` → `finalize`; `err` is `LifecycleResumeIncompatible`, whose code falls through to the generic `composition.failed` (`compose_failed_code` / render default) |
| D7 | proxy-target-bootstrap-failed | refused handoff after launch → `failure` | a target that fails to parse (shell in its `initialize`) ends the run, exit 1, with no further events: no source `finalize`, no target events |
| D8 | shell-locked-blocked | refused before execution | a locked shell in early `blocked` raises like an evaluation error → `failure`; a locked shell in early `finalize` ends `finalize` silently, exit 1 |

Also observed: catch-loop (raising gate guard after `finalize`) runs no catch event and exits 1. Current Claudine docs (`claudine/docs/topics/flow-control/lifecycle.md` evaluation-error rules, `flow-control-reference.md` resume/proxy sections) already describe this behavior, so no doc correction was needed in this phase. The spec is left unchanged as the decision record. Section 8's text should be corrected in the Darkmatter lifecycle docs written in Phase 6.

### Boundary prototype (Wave 2)

- **Location:** test-only module `darkmatter/lib/tests/l1/lifecycle_boundary/` (`engine.rs`, `profiles.rs`, `publisher.rs`, `protocol_tests.rs`, `replay.rs`), declared in `darkmatter/lib/tests/l1/main.rs`. There is no production module, Cargo change, or export, because the spec forbids two production engines. `engine.rs` uses Darkmatter's real `BindingView`/`Availability` and `is_reserved_namespace`. Claudine names appear only in `profiles.rs::claudine_profile()`.
- **API decided (input to Phase 2):**
  - descriptors are `EventDescriptor { name, position, role, globals: BindingView, actions }` over `CoreEvent` × `Position` (`before-composition`/`after-composition`/`after-finalize`) × `Role` (`notification`/`terminal-outcome`/`iteration-gate`);
  - `Registry` with `rename`, `register`, `register_outcome_pair(OutcomePair::new(s, f).downgrade(from, to))`, `declare_core_actions`, `set_cancellation_error`, typed `RegistrationError`, and immutability while a run holds it;
  - owned `Request { ticket(run, document, attempt, iteration, generation), kind }` with kinds `EvaluateEvent` (mode `NormalRecovery` / `FatalEvaluationCleanup` / `RefusalCleanup` / `CancellationCleanup`), `Prepare{entry}`, `PerformOperation{entry}`, `AwaitCompletion`, `Recover`, and `Done`;
  - typed replies `Event(EventOutcome)`, `Prepared(Ready{epoch} | Failed(HostFailure{stage, entry, error}))`, `Operation(Started | Completed{outcome, session} | NotStarted)`, and `Recovery(Admitted | Exhausted | Refused | Adopted | Aborted)`;
  - a single shell owner, `run.shell_permission(&ticket)`;
  - `cancel()` invalidates outstanding generations before cleanup.
- **Non-agent host (`publisher.rs`):** registers a before-composition notification `configure`, renames `composed` → `ready`, and pairs `published`/`rejected` with downgrade `published → rejected`. It proves notification order, a `set` visible later, typed failure as `err`, unavailable `err` failing with the view's reason (never `fm.err`), refused shell in `initialize` and early cleanup, and outcome selection/downgrade.
- **Protocol tests:** registration (duplicates, Darkmatter names, reserved roots, fixed events, second name, role/position, outcome-pair completeness, downgrade target, immutability); the core-only `md` profile (no pair; only `failed` holds the slot; write failure and `composed` error → `finalize` without `failed`); rename diagnostics; request binding and rising generations; stale/duplicate/foreign/wrong-kind/late replies leaving the run unchanged (`run == before`); cancellation with and without `cancelled`, during `initialize`, late replies after cancel, cancel during cleanup; the shell owner from `initialize` to proxy adoption; all seven failure-stage rows; downgrade; skip; retry/resume/proxy finalize exceptions; recovery in `finalize`; `SetupRecoveryUnsupported`; the iteration gate.
- **Criterion 6 note:** "handler emitting an event" and "out-of-position emission" cannot be expressed against this API. It has no emit operation, and replies name outcomes rather than events. Phase 3 must keep that property (or add the checked emission seam and test it).

### Prototype verdict (Wave 3) — **PASS**

- `darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs` replays all 30 production traces through the prototype with a scripted Claudine-profile host (one `#[test]` per trace, `include_str!` of each `.trace` inside its test, and a guard `every_recorded_trace_has_a_replay` that reads the directory via `manifest_dir!()`). **30/30 reproduce exactly**, line for line including `exit`.
- **Gate criteria:** the registry API was sufficient for the Claudine profile and the publisher host. Shell permission has one owner (the host writes `shell <event>` only after `shell_permission` returns `Ok`). No reply mutates engine accounting directly (terminal slot, finalize, shell lock, and generation are engine-internal). The Claudine coordinator and provider harness were not moved: budgets, sessions, target parsing, loop `until`/`max`, and container scheduling stay in the scripted host.
- **Engine changes the traces forced (none keyed on Claudine names):** (1) a new `RecoveryReply::Aborted(HostFailure)` ends the run without further events (D5, D7; only stage `Setup` for proxy and `RecoveryRefused` for retry/resume, anything else is `StageNotApplicable`); (2) `composed` may request `resume` (host answers `Aborted` without a session); (3) `Registry::set_cancellation_error` (`composition.cancelled` in the Claudine profile) is selected when cancellation cleanup starts. Two prototype cancellation tests gained the `error composition.cancelled` transcript line. A probe that made `Aborted` finalize failed exactly the D5/D7 replays, and the engine was restored.
- **How D1–D8 are expressed:** D1 is an engine rule (the outcome pair's failed alternative catches setup errors). D2/D3 are role-keyed engine rules (setup versus terminal outcome). D4 is an engine rule. D5/D7 use the host's typed `Aborted` reply. D6 is a host fact (stage + code). D8 is the host turning `ShellRefusal::Locked` into its uncoded evaluation error with Claudine's wording. The cancellation `err` is a profile setting.
- **Open, not covered by any trace (Phase 2 must add a trace before relying on it):** production's `retry` in `initialize` (and possibly recovery from `blocked`) is a hard error without `finalize`, while the prototype runs refusal cleanup; `retry` from `start` is unverified; a cancelled root run (exit 130) is not traced. The historical `spike/model.rs` and `spike/scenarios.rs` are untouched.

### Gates and test evidence

- **Baseline test identities:** `cargo nextest list` at this revision: `claudine` lib 5,499 tests (`/tmp`-only listing, not committed); `claudine-cli --features test-fixtures` 4,649 tests, of which 31 are the new `lifecycle_traces` tests (4,618 pre-existing). Both spike tests (`claudine/lib/tests/l1/lifecycle_control_flow_spike.rs`, `darkmatter/lib/tests/l1/lifecycle_control_flow_spike.rs`) pass unchanged.
- `cd claudine && just test`: **10,479 run, 10,479 passed** (12 slow, 9 skipped), 224.8 s. No pre-existing failures. Phase 1 adds only tests, so this is also the baseline result for the existing suite.
- `cd claudine && just lint`: pass. `just check-tier-coverage claudine`: pass, 0 stranded.
- `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` (the only Darkmatter binary this phase touches): **1,693 passed**. `cd darkmatter && just lint`: pass. `just check-tier-coverage darkmatter`: 0 stranded.
- **Dependents:** no public API changed (test-only additions), so no downstream package tests apply.
- **Cross-OS:** no `just cross-check` run. The changes are test-only. The Darkmatter replay is portable: it trims `\r`, and the repository's `.gitattributes` forces `eol=lf`. The Claudine trace tests are `#[cfg(unix)]` by necessity (see the trace capture coverage gap).
- **Scope calculator:** not used to skip anything. Source files changed, so both gates ran.

#### Requirement → test mapping

| Requirement (plan Phase 1) | Tests |
|---|---|
| Baseline suite + spike tests | `just test` (claudine), both `lifecycle_control_flow_spike` tests |
| Traces through production paths, every Stage 1 scenario | `claudine-cli::l1 lifecycle_traces::<scenario>` ×30 + `every_trace_fixture_has_exactly_one_test` |
| Prototype: before-composition notification, rename, outcome pair, `set`, typed failure, unavailable `err`, refused `initialize` shell | `darkmatter::l1 lifecycle_boundary::publisher::*` (6) |
| Registration validation, owned requests, stale/duplicate/foreign/wrong-kind/cancelled replies, cleanup modes, cancellation, shell owner, failure stages | `darkmatter::l1 lifecycle_boundary::protocol_tests::*` (26) |
| Prototype verdict: replay every trace | `darkmatter::l1 lifecycle_boundary::replay::<scenario>` ×30 + `every_recorded_trace_has_a_replay` |

The Input Robustness Matrix does not apply to Phase 1: no catalog or trigger loader was changed. Its proposed outcomes still need the author rulings the plan requires before Phase 4 (see `human_review_items`).


## Phase 2

- **Starting revision:** `563326f84` on `feat/pipeline` (2026-10-20), Phase 1 committed. **Superseded:** Phase 2 was paused here, before any extraction code, so `2026-10-16-type-narrowing` could be built first (the author's ruling on the former Phase 1 review item, 2026-10-20). Phase 2 resumes from the revision that closes that fix; the plan's Phase 2 entry gate records it and refreshes the Phase 1 baseline.
- **Criterion 23 is no longer blocked:** owner P3 preflight integration (the audit reads `TypeScope`), with evidence from P5 invocation coverage (a guarded lifecycle shell under `md compose` behaves as under Claudine) and the narrowing conformance matrix passing unedited after every phase.

### Extraction design (decided before Wave 4)

- **Placement:** a new `darkmatter::lifecycle` module in `darkmatter/lib/src/lifecycle/`. No new crate: Darkmatter already depends on every crate the moved code uses (`biscuit-terminal`, `biscuit-file`, `indexmap`, `serde_json`, `thiserror`, `tracing`).
- **Errors:** the parser and model raise `darkmatter::lifecycle::LifecycleError`, which holds the lifecycle authoring variants moved out of Claudine's `CompositionError` (same fields, same `Display` text). Claudine keeps its same-named `CompositionError` variants for now, because its diagnostic code catalog, block renderer, and every existing test assertion are keyed on them, and maps each `LifecycleError` variant onto its twin with one exhaustive `From` impl. A Claudine test checks that the two `Display` texts agree for every variant. This keeps the error *type* in two places while the parser exists once; Phase 3 (the "delete superseded" task) can fold the twins together.
- **Existing tests stay where their assertions are typed.** Claudine's lifecycle tests assert on `CompositionError` variants through `claudine::composition::parse_lifecycle_config` and friends. They stay in Claudine, unchanged, and now exercise the moved parser through the bridge, which is the strongest evidence for acceptance criterion 2. Moving them would have meant renaming the error type inside each assertion. Tests that live inside a moved file and assert only on moved types move with it. Darkmatter gets new tests against its own public API.
- **Re-exports:** Claudine re-exports every moved item at its old path (`claudine::composition::{LifecycleSignal, LifecycleConfig, …}`), so callers and tests keep compiling. The re-exports delegate to the one implementation; there is no copy.

### Entry gate (resumed 2026-10-21)

#### Prerequisite revision

- `2026-10-16-type-narrowing` is closed: `implemented: true` on its plan and
  spec, a recorded Phase 6 close (documentation and final verification, all
  gates green), and `review-1.md` with `ready: true`, no findings, and
  `human_review: false`.
- Its closing commits: `3c05cfe60` (structural type narrowing),
  `2a58d0617` (scoped shell template refinements), `e3eb8e7ef` (guard scopes
  through composition consumers), `1b10bb732` and `d91e916af` (phase closes),
  then the review-to-implement repairs `13cbc87c6`, `8fd0addf1`, `3ec767019`,
  and `86d97771f`. The later strict-equality fix (`b59ba28e0`, `455eb05e4`,
  `96ccc207b`, `faef73cda`) also landed in between.
- **New starting revision: `d48d7dcf0`** on `feat/pipeline` (2026-10-21). It
  replaces Phase 1's `f3381eb5c` as this feature's behavioral baseline.

#### Baseline replay

All results at `d48d7dcf0`, before any extraction code:

- `cd claudine && just test`: **10,606 run, 10,606 passed** (30 slow, 9
  skipped), 288.8 s. This includes the 31 `lifecycle_traces` tests. All 30
  traces pass unedited, so the prerequisite changed no recorded trace and none
  was recaptured.
- Darkmatter `lifecycle_boundary` (6 publisher + 26 protocol + 31 replay) and
  its `lifecycle_control_flow_spike` tests: **78 passed**
  (`cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(lifecycle_boundary) | test(lifecycle_control_flow_spike)'`).
- Claudine `lifecycle_control_flow_spike`: **4 passed**.

These counts replace Phase 1's (10,479 / 1,693-binary).

#### Inventory refresh

What the fix added or changed in the lifecycle code
(`git diff f3381eb5c..d48d7dcf0`), with its owner under the spec's ownership
table:

| Code | What it does | Owner |
|---|---|---|
| `lifecycle/actions.rs::walk_scoped`, `action_writes` | the lifecycle walk's `TypeScope` extension (`when:` → `then:`/`else:` via `assume_at`, `join`), and the writes that end a fact | **Darkmatter**: moves with the parser slice in Wave 4 |
| `lifecycle/shell_template.rs` (`lifecycle_declared_types`, `placeholder_guard`, `typed_template`, `retime_guarded`, `HoleTiming::Guarded` sites) | typed lifecycle templates and the presence decision | **Darkmatter**: names only Darkmatter types; `actions.rs::ShellAction.template` holds a `LifecycleShellTemplate`, so it moves with the slice |
| `composition/preflight.rs` (`resolve_shell_text`, `guarded_template`, `sequence_lifecycle_template`, `missing_value`) | the scope-carrying lifecycle pre-flight walk | Darkmatter per the table, but **Phase 3** ("Preflight integration"); stays in Claudine this phase |
| `sequence/preflight/mod.rs` (`site_types`, `prepare_site`, `record_prompt_reference`, `resolve_approval_time`) | the sequence task `when:` extension | **Claudine** (stays) |
| `executor/tests/{guarded_shell_sites,scope_site_conformance}.rs`, `sequence/preflight/tests.rs` | the narrowing conformance matrix and guarded-site tests | tests stay with their assertions; paths updated only if a file moves |
| guarded-hole filling at a reached action | `LifecycleShellTemplate` filled through Darkmatter's shared template API (`fill.rs`) by the executor | **Darkmatter**, with the executor (Wave 5) |

The executor itself (`executor.rs`) did not change in the fix.

Files the fix's guard and allowlists name (`claudine/lib/tests/l1/type_scope_ownership_guard.rs`, which scans `claudine/lib/src` only):

- `composition/preflight.rs`: `resolve_lifecycle_shell_commands_in_sequence` (`Unguarded` ×1), decision sites `resolve_shell_text`, `guarded_template`, `sequence_lifecycle_template`, missing-value constructor `missing_value`
- `composition/sequence/preflight/mod.rs`: `site_types` (`Unguarded` ×1, `Assume` ×1), `prepare_site` (`DeclaredTypes` ×1), `record_prompt_reference` (`DeclaredTypes` ×1), decision sites `prepare_site`, `resolve_approval_time`
- `composition/lifecycle/shell_template.rs`: `lifecycle_declared_types` (`DeclaredTypes` ×1), `placeholder_guard` (`PresenceLookup` ×2)
- `composition/lifecycle/actions.rs`: `walk_scoped` (`Assume` ×2)

When `actions.rs` and `shell_template.rs` move, their four entries leave
Claudine's guard (which only scans Claudine source) and join Darkmatter's own
`darkmatter/lib/tests/l1/type_scope_ownership_guard.rs` allowlist with the
same function, count, and reason. That is the "path only" change for a guard
whose scan root is per crate.

#### Guarded-shell trace

- Two new scenarios, `guarded-shell-present` and `guarded-shell-null`
  (`claudine/cli/tests/fixtures/lifecycle_traces/`), driven like the others
  but with a `.darkmatter-shell-whitelist` holding only
  `template trace-shell start {{ string }}`, so the guarded shell is approved
  by shape and runs as its template (the `--yolo` first draft approved the
  resolved bytes, so the null case ran `hello` from pre-flight. That is
  exact-mode behavior, not the guarded hole). Template mode refuses `>>`, so
  the trace test now installs a `trace-shell` helper in the fixture `bin` for
  these two scenarios' commands.
- Present: `event initialize`, `event start`, `shell start hello`,
  `work fresh 0`, `event success`, `event finalize`, `err none`, `exit 0`.
- Null (`set: { note: null }` inside the selected `then:`): `event start`,
  `state note=`, then **no shell**; the action fails with "reads 'note',
  which is missing" (`MissingValue`), routed `failure` → `finalize` with
  `err composition.failed`, exit 1 (the same routing as `catch-start`).
- Darkmatter replay (`lifecycle_boundary/replay.rs`): a `{"template_shell": key}`
  step that checks `shell_permission`, then fills the placeholder from the
  host's live state; a `null` is the host fact "missing value" raised with
  Claudine's generic code. No engine rule is keyed on names. A truthy
  string guard condition and `null`-as-empty rendering were added to the
  replay vocabulary. **33 trace tests and 32 replays pass.**
- No trace was recaptured; Wave 4 starts from these 32 traces.

### Wave 4 — Model extraction (done)

- **Moved** to `darkmatter/lib/src/lifecycle/` (history-preserving renames):
  `parse.rs`, `action_shape.rs`, `actions.rs` (+ `actions/tests.rs`),
  `signatures.rs`, `validate.rs`, `source_map.rs`, `shell_template.rs`, and
  `composition/authored_order.rs`. **New:** `lifecycle/mod.rs` (`LifecycleSignal`,
  `LifecycleConfig`, `LifecycleStacks`, `LIFECYCLE_EVENT_KEYS`,
  `LifecycleParseOptions`, the neutral parse entry points, and the four
  `LifecycleSignal` methods that used to live in Claudine's `audio.rs`, because
  Rust allows inherent methods only on a crate's own types), `lifecycle/error.rs`
  (`LifecycleError`, `ActionExprError`), and `lifecycle/tests.rs`. Darkmatter's
  `json_type_name` became `pub(crate)` so the parser reuses it rather than a
  copy.
- **Errors:** `LifecycleError` has the 26 authoring variants the moved parser
  raises, with the same names, fields, and `Display` text as their
  `CompositionError` twins. Claudine keeps the twins (its diagnostic catalog,
  renderer, and test assertions are keyed on them) and maps with one exhaustive
  `From` impl in `claudine/lib/src/composition/error/lifecycle_authoring.rs`.
  Parity tests there: `every_lifecycle_error_converts_to_its_same_named_twin_with_the_same_display`
  and `a_typed_cause_survives_the_conversion`. `ActionExprError` moved and is
  re-exported at its old path.
- **Claudine bridge (for the slice):** Claudine's `lifecycle/mod.rs` re-exports
  every moved item at its old path (`actions` and `shell_template` as module
  re-exports) and keeps its public wrappers' signatures (`parse_lifecycle_config`,
  `parse_loop_gate`, `parse_task_action_stack`, `parse_single_action`, …),
  which pass Claudine's Playa effect catalog and `UnknownVerbPolicy::Reject`
  and map errors. **Two validators stay in Claudine** in the new
  `lifecycle/binding_checks.rs`: `validate_no_err_in_no_error_events` and
  `find_removed_loop_read` read Claudine's binding catalog (`err` and `loop`
  availability rules are host policy until Phase 3's registry), so they use
  Darkmatter's walkers. `LifecycleErrNotAvailable` therefore stays a
  Claudine-only error.
- **Unknown verbs:** `LifecycleParseOptions.unknown_verbs: UnknownVerbPolicy`
  (`Reject`, the default and Claudine's, or `Ignore`). Under `Ignore` an
  unknown verb is `LifecycleActionKind::Ignored(IgnoredAction { verb, path })`,
  and its operand is never parsed, evaluated, or scanned for shells, spans, or
  `err` reads; known verbs and `when`/`then`/`else` structure still validate.
  Claudine's executor treats `Ignored` as a commented no-op (its profile never
  produces one). Tests: `darkmatter` `lifecycle::tests::{reject_refuses_an_unknown_verb_with_the_closed_grammar_error,
  reject_is_the_default_policy, ignore_keeps_an_unknown_verb_as_an_opaque_action_with_its_name_and_path,
  ignore_never_interprets_an_unknown_verbs_operand, ignore_still_rejects_a_malformed_known_verb,
  ignore_still_validates_when_then_else_structure}`.
- **Guards (path-only updates):**
  - `claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs` reads the six slice
    files at their Darkmatter paths. Its assertions and forbidden list are
    unchanged, and `claudine/lib/Cargo.toml` lists them as CI `source-inputs`.
  - `claudine/lib/tests/l1/type_scope_ownership_guard.rs`: Darkmatter's own
    guard only reviews `TypeScope::unguarded`, so moving the three entries
    there would have dropped the `DeclaredTypes`/`Assume`/`PresenceLookup`
    reviews. Instead Claudine's guard now also scans
    `darkmatter/lib/src/lifecycle/`, where Claudine's lifecycle walk now lives,
    and keeps the three entries with their function, count, and reason
    unchanged and their paths now `darkmatter/lib/src/lifecycle/{actions,shell_template}.rs`.
    All 7 guard tests pass.
  - `claudine/cli/tests/l1/error_guards/transport-allow.toml`: the
    `actions.rs::type_with_value` entry was removed because that guard scans
    Claudine source only and fails on a stale entry. The code is unchanged, but
    no error-text guard covers it while it lives in Darkmatter.
- Scoped results: darkmatter `--lib` lifecycle 43/43; darkmatter `l1` lifecycle +
  guard 91/91; claudine `--lib` lifecycle/error/looping/preflight 1331/1331;
  claudine `l1` lifecycle + guard 41/41 (7/7 after the guard repair);
  claudine-cli `l1` traces/directive matrix/guarded shells 94/94; doctests 6 + 1;
  `just lint` darkmatter and claudine: pass.

### Engine and service extraction design (Waves 5–6)

Decided before Wave 5, from reading the code at `d48d7dcf0`:

- **What Phase 2 moves is the running implementation**, not the prototype.
  The prototype stays test-only. The executor (`StackExecutionContext` and its
  action dispatch), the run guard (`LifecycleRunGuard`: initialize/start/
  terminal/finalize accounting, the one terminal slot, shell lock, proxy
  reset), the catch protocol and transition decision (`runtime.rs`), control
  admission (`control.rs`), and the `err`/`timing` envelope (`context.rs`)
  move to `darkmatter::lifecycle`. Each Claudine dependency becomes a neutral
  service seam that Claudine implements. Rewiring every driver onto a
  request/reply `Run` is Phase 3's "Driver migration": doing it here would
  leave the prototype-shaped `Run` and the moved guard as two engines for the
  length of the phase, which the spec forbids. Recovery and cleanup modes
  become named types where today's code passes booleans.
- **Services** (`darkmatter::lifecycle` traits, Claudine adapters): effects and
  communication (the moved emitter trait has no messaging or TTS parameters,
  and Claudine's adapter holds `RuntimeMessagingSettings` and `GlobalSettings`
  and keeps its existing configuration-carrying emitter trait, so the current
  test doubles and their assertions stay); shell running (the `ShellRunner`
  trait moves, while `SystemShellRunner`, `DocumentShellRunner`, and structured
  execution stay with Claudine's process code); runtime-state writes, loop
  values and the `break` slot, the `prep` runner, and cancellation probes, each
  through a trait. A missing required service is a typed error.
- **Error envelope:** the neutral envelope (code, category, disposition,
  origin, severity, detail, message, one-level cause, source location) is
  Darkmatter data. Claudine's effective-diagnostic selection projects into it
  (constructors that take `ClaudineError`, `HarnessError`, `CompositionError`,
  or `Diagnostic` stay in Claudine as extension traits), so rendering, machine
  output, and `err` keep reading one value.

### Wave 5a — Service adapters (done)

- **Error envelope** moved to `darkmatter/lib/src/lifecycle/envelope.rs`
  (`DiagnosticSnapshot`, `DiagnosticCause`, `DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION`,
  `concise_message`/`concise_message_with_suffix`/`CONCISE_MESSAGE_MAX_CHARS`,
  the `DiagnosticCatalog` service) and `lifecycle/context.rs`
  (`LifecycleErrorInfo`, `LifecycleCause`, `LifecycleTiming`,
  `LifecycleEvaluationReason`); `LifecycleExprError` moved to
  `lifecycle/error.rs`. All are re-exported at their old Claudine paths. Field
  names, serde attributes, `Display`, and `to_value` output are unchanged; the
  one `to_value` projection lives in Darkmatter.
- **Catalog-dependent constructors** stay in Claudine as extension traits:
  `DiagnosticSnapshotExt` (`from_diagnostic`, `select`, `from_code`),
  `DiagnosticCauseExt`, `LifecycleErrorInfoExt` (`from_claudine_error`,
  `from_harness_error`, `from_composition_error`, `from_proxy_commit_error`,
  `from_error_or_action`, `from_expr_error`, `from_action_failure`), and
  `CancellationCauseExt::error`. Each delegates to a Darkmatter constructor
  that takes a `&dyn DiagnosticCatalog` (`*_with`), passing
  `ClaudineDiagnostics`, so the projection logic exists once. Call sites gained
  only a trait import.
- **`frontmatter_excerpt`** is now `Option<SourceExcerpt>`: an opaque,
  never-serialized `Arc<dyn SourceAppendix>` with value equality. Claudine's
  `FrontmatterExcerpt` stays in Claudine (its line location reads Claudine's
  property paths) and implements `SourceAppendix`; `RestoredDiagnostic`
  renders it unchanged.
- **`concise_message`** hygiene moved with the envelope; Claudine's
  `harness::runtime::sanitize_message` delegates to it, so provider failures
  and typed errors still share one rule.
- **Effects:** `darkmatter::lifecycle::effects::LifecycleEffects` (text only:
  `stdout`, `stderr(signal, …)`, `info`, `warn`, `success`, `message`,
  `speech`, `effect`, `notification`), `dispatch_communication`, and the render
  helpers `render_prose_line`/`render_status_line`. Claudine's
  `LifecycleEmitter` and `DefaultLifecycleEmitter` keep their signatures and
  now render through those helpers. `lifecycle/effects.rs`'s
  `ClaudineLifecycleEffects` adapter resolves the TTS configuration (once per
  adapter) and the messaging route; the executor's `emit_communication` and
  `on_error` warnings and the run guard's `emit_signal` all go through it.
- **Services:** `LifecycleServices { effects, diagnostics }` returns
  `MissingServiceError { service: LifecycleService }` for an absent service.
  Nothing in Claudine constructs it yet; it is the bundle the moved executor
  takes.
- **Cancellation:** moved, not wrapped. `CancellationToken`,
  `CancellationScope`, `CleanupScope`, `current()`, and `CancellationCause`
  live in `darkmatter::lifecycle::cancellation` with the same thread-locals;
  the probes are `cause_with(interrupted)`/`requested_with(interrupted)`, and
  Claudine's `cancellation::{cause, requested}` pass `crate::interrupt::interrupted`.
  The probe is an argument rather than a registered global, so no
  initialization order can drop a user interrupt.
- No guard needed a path update: the error guards, lift-ready guard, and
  type-scope guard scan nothing that moved.

### Wave 5b — Executor extraction (done)

- **Moved** to `darkmatter/lib/src/lifecycle/executor.rs` (engine) with
  `executor/{outcome,shell,host}.rs`: the whole action dispatch (ordered
  nodes, `when`/`then`/`else`, `set` snapshot semantics and shell values,
  deferred spans, event-time interpolation, control directives, `no_error`,
  `on_error`, `proxy`/`retry` `with:`, `break`, `prep`, side effects through
  `EffectEngine`, template-mode shells), `StackControl`,
  `LifecycleEventOutcome`, `ActionLocation`, `ShellRunError`, `ShellRunner`,
  `ShellValueRequest`, `execute_set_shell_values`, `DisabledShellRunner`,
  `compose_request`, `evaluate_operand_in`, `is_template_operand`. All are
  re-exported at their old Claudine paths (`ActionLocation` from
  `coordinator/handoff.rs`, `compose_request` from `composition/mod.rs`).
- **Darkmatter API:** `StackInputs` (per-event neutral inputs) and
  `StackExecutor::{new(inputs, LifecycleServices) -> Result<_, MissingServiceError>,
  with_services}`, with `execute_event`, `execute_action_stack`,
  `evaluate_guard`, `dispatch_task_side_effect`, `eval_expr`,
  `resolve_string_value`, `resolution_context`. Effects and diagnostics are
  both required; a missing one is a `MissingServiceError` at construction.
- **Host seam** `LifecycleHost`: `bindings` (the whole availability decision
  stays Claudine's binding catalog), `name_coercion_keys`,
  `resolution_context`, `capture_overlay_context`, `persisted_data`,
  `commit_set`, `in_loop`/`record_break`, `prep` (`PrepRequest` →
  `PrepError::{Unavailable, Failed}`), `user_blacklist`, `interrupted` (fed to
  `cancellation::requested_with`), `is_known_effect`, and `failure`, which
  classifies a `LifecycleRunFailure` (seven variants mirroring the
  `CompositionError` twins the executor raised) in the host's catalog.
- **`ShellRunner::for_child_environment`** now takes Darkmatter's
  `ShellChildEnvironment`; the one Claudine call site passes
  `env_baseline().shell_child_environment()` and Claudine's runners rebuild an
  `EnvBaseline` from its entries.
- **Claudine adapter:** `StackExecutionContext` keeps every field, so its 45
  construction sites and every test are unchanged. It implements
  `LifecycleHost`, builds one `ClaudineLifecycleEffects` per run (the TTS
  configuration is now resolved once per run rather than once per action;
  settings are immutable, so the result is the same), and delegates every
  dispatch method. `SystemShellRunner`, `DocumentShellRunner`, structured
  execution, `TaskGuardFailure` (parsed in the adapter, evaluated by
  `evaluate_guard`), `PrepCall`/`PrepRunner`, and `LoopBreak` stay in Claudine.
- **Departure from the plan sketch:** call sites were not rewritten onto the
  Darkmatter type; the adapter is the "constructor/helper" and keeps the
  driver rewiring for Phase 3. The `capture_proxy_with_fallback` test probe
  stays in Claudine behind `LifecycleHost::capture_overlay_context`.
- **Guards:** `claudine/cli/tests/l1/composition_seams.rs` now also scans
  `darkmatter/lib/src/lifecycle/` (root-anchored join); its
  `lifecycle::executor::{interpolate_text_in, early_binding_context}` entries
  are unchanged because the module identity is the same. The type-scope guard
  already scanned that directory. The four executor files are `source-inputs`
  of `claudine` and `claudine-cli` (`RealWorkspaceTestInputTests` 7/7).
  Test edits: `executor/tests/mod.rs` gained imports only.

### Wave 5c — Run guard, catch protocol, control admission (done)

- **Moved** to `darkmatter/lib/src/lifecycle/`: `control.rs` (`ControlDispatch`,
  `decide_control`, `control_budget_for`, `compute_backoff_delay`,
  `MAX_PROXY_HOPS`, `proxy_path_identity`, `proxy_handoff_allowed`,
  `parse_delay`); `runtime.rs` (`LifecycleTransition{Input,Error,Abort,Decision}`,
  `decide_lifecycle_transition`, `LifecycleCatch{Execution,Step,Result,State,Protocol}`,
  `ContainerEventDecision`, `ContainerEventResult`, `ContainerRunner`); and
  `run_guard.rs` (`LifecycleRunGuard` with its accounting, terminal slot,
  provider-launched fact, shell lock, resets, `redesignate_terminal_to_failure`,
  `run_event_stack`/`execute_event`, `emit_*_once`, `defuse`, run-prepared
  context, `Drop`, and `cancel_if_requested`). **New:** `duration.rs`
  (`parse_duration_secs`, `DurationSyntaxError`), the one `{number}{unit}`
  grammar; Claudine's `harness::parse_timeout{,_allow_zero}` map its error to
  `HarnessError::InvalidTimeout` with the same `detail` text.
- **Darkmatter API:** `LifecycleRunHost` (`source_path`, `prepared_context`,
  `effects`, `diagnostics`, `interrupted`, `enter_terminal_event` →
  `TerminalScope`, `failure`); `GuardedEventStack::run_guarded(GuardedEvent {
  signal, config, shell_runner, error })`, where the guard passes
  `DisabledShellRunner` until `start` and the cancellation as `err`;
  `LifecycleRunGuard::with_host`/`host`; `LifecycleCatchProtocol::with_catalog`;
  `ContainerRunner<'a, H>::mode`; `LifecycleRunFailure::CancellationRecovery`;
  `EvaluationMode { NormalRecovery, FatalEvaluationCleanup, RefusalCleanup,
  CancellationCleanup }`, carried as `LifecycleCatchStep::mode` and used for
  cancellation cleanup in the guard and `ContainerRunner` (the refused-recovery
  and explicit-`error` warnings are one `cleanup_warnings` for both).
- **Claudine adapters:** `LifecycleRunGuard<'a>` and `ContainerRunner<'a>` are
  aliases over `ClaudineRunHost` (runtime context, emitter, one
  `ClaudineLifecycleEffects` per run, `ClaudineDiagnostics`,
  `TerminalLifecycleScope`, `executor::classify_run_failure`, now shared with
  `StackExecutionContext::failure`). `LifecycleRunGuardExt` keeps `new(config,
  &ctx, emitter)`, `context()`, and `emitter()`; `LifecycleCatchProtocolExt`
  keeps `new(origin, state, prior, outcome)`. Call sites gained only those trait
  imports. `StackExecutionContext` implements `GuardedEventStack` through
  `with_signal` (a new event starts a new `current` memo) and `with_error`.
  `LifecycleRuntimeContext`, `resolve_proxy_target` (it returns `HarnessError`),
  and `IterationSummarySignals` stay in Claudine: the last is provider facts the
  CLI loop consumes, not an engine input, so the engine no longer imports
  `stream::summary`. Everything is re-exported at its old path.
- **Behavior notes:** the drop safety net's `emit_*` now reuses the run's one
  effects adapter instead of building one per emission (settings are
  immutable, so the TTS configuration is the same); `cancel_if_requested` warns
  through that adapter, whose terminal is the runtime context's (every caller
  builds its event context from the same one).
- **Guards:** `path_lookup_guard`'s `proxy_handoff_allowed` exception
  (`std::fs::canonicalize`, 2, same-file verdict) moved from Claudine's guard to
  `darkmatter/lib/tests/l1/path_lookup_guard.rs` with item, operation, count,
  and reason unchanged; Claudine's guard scans only `claudine/lib`, and
  Darkmatter's already scans the new home. `control.rs`, `run_guard.rs`, and
  `runtime.rs` are `source-inputs` of `claudine` and `claudine-cli`. No other
  guard needed an edit.
- **Tests:** the spike imports the moved types from `darkmatter::lifecycle::…`
  plus `LifecycleCatchProtocolExt`/`LifecycleRunGuardExt`; assertions unchanged.
  New Darkmatter tests with a fake host: `run_guard::tests` (6),
  `runtime::tests` (3), `control::tests` (3).
- Scoped results: darkmatter `--lib` lifecycle 86/86; darkmatter `l1` lifecycle
  84/84 (+ guard 62/62); claudine `--lib` 1720/1720; claudine `l1` 47/47;
  claudine-cli `l1` 1426/1426; claudine-cli bin 1370/1370; doctests 10 + 1;
  `just lint` darkmatter and claudine: pass.

### Wave 5d — Gated request/reply `Run` (done)

- **New:** `darkmatter/lib/src/lifecycle/run.rs` (`Run`, `RunSettings`,
  `RunSnapshot`) and `run/protocol.rs` (`Ticket`, `RunId`, `Request`,
  `RequestKind` {`EvaluateEvent(EventRequest)`, `Prepare`, `PerformOperation`,
  `AwaitCompletion`, `Recover(RecoveryRequest)`, `Done(Finish)`}, `Reply`
  {`Event(LifecycleEventOutcome)`, `Gate { outcome, iterate }`, `Prepared`,
  `Operation`, `Recovery`}, `Preparation`, `OperationReport`,
  `CompletionVerdict`, `RecoveryReply` {`Budget { ceiling }`, `Adopted`,
  `Refused`, `Aborted`}, `HostFailure { stage, entry, error }`,
  `FailureStage`, `EntryReason`, `ApprovalEpoch`, `SessionAvailability`,
  `RunError`, `ReplyError`, `CancelError`, `ShellRefusal`, `AuthoredEvents`,
  `TranscriptEntry`). `Run::{start, pending, reply, answer, cancel,
  shell_permission, finish, selected_error, terminal, accounting, snapshot,
  transcript}`. The cancellation error is a `RunSettings` value; the catalog
  that classifies an explicit `error(...)` is the other. Events are the fixed
  `LifecycleSignal` set; nothing is keyed on a host string.
- **One copy of the logic:** the guard's six flags moved into
  `run_guard::RunAccounting` (record once, the one terminal slot, redesignate,
  provider launched, defuse, both resets, the owed safety-net signal);
  `LifecycleRunGuard` holds one and delegates with an unchanged API, and `Run`
  holds one. Routing is `decide_lifecycle_transition` (catch trigger, control
  admission through `decide_control`, the finish verdict) and
  `LifecycleCatchProtocol` (setup catch, cleanup modes, evaluation
  precedence); modes are `EvaluationMode`. The explicit-error default text is
  one `runtime::explicit_error`, used by the catch protocol and the success
  downgrade. Engine rules that are new code: request sequencing, the stage
  table that picks `blocked`/`failure` for a host failure, which events admit
  which recovery, and ticket/cancellation checks.
- **Host facts, not engine state:** document state and `set`, stacks, the
  retry/resume ceiling (`RecoveryReply::Budget`, which the run feeds to
  `decide_control`; it replaces the prototype's `Admitted`/`Exhausted`),
  proxy targets and path identity, the `loop` verdict, and `err` binding
  availability. A resume without a session is now the run's decision
  (`RunError::Aborted(ResumeWithoutSession)`) once the host's budget admits
  it, as in production; the host no longer answers `Aborted` for it.
- **Replay:** `lifecycle_boundary/replay.rs` drives the production `Run`.
  **32/32 traces reproduce line for line**, plus
  `every_recorded_trace_has_a_replay`; no trace or fixture changed.
- **Protocol tests:** 17 prototype tests ported to
  `darkmatter/lib/tests/l1/lifecycle_run_protocol.rs` (the downgrade test
  split in two, so 18), plus
  `a_resume_without_a_session_ends_the_run_without_finalize` and
  `concurrent_runs_share_no_state` (20). Facts that could not be ported: a
  request's stack and state, `set` mutations, and the `goal` state of an
  adopted document (host-owned now); the adopted document's rename check and
  per-event action types (registry phase; kept as prototype tests
  `an_adopted_document_is_split_by_the_registry` and
  `registered_action_types_refine_shell_permission`).
- **Finding (untraced):** the prototype let `failure` recover after an
  explicit `error(...)` in a preflight `blocked`. Production's catch protocol
  re-designates the slot and runs `failure` as `RefusalCleanup`, so its
  recovery is not honored; the ported test is now
  `an_explicit_error_in_preflight_blocked_is_caught_by_failure_as_cleanup`.
  Two smaller production rules replace prototype behavior no test asserted:
  an action error in a terminal-phase event leaves the selected error alone,
  and a failure during cancellation cleanup does not replace the cancellation.
- **Prototype retired:** the engine's cancellation, retry/resume, iteration,
  host-refusal, and await-completion paths and their types, the Ticket
  accessors, and `Registry::set_cancellation_error`. **Kept** (registry
  phase): `engine.rs` registry and its minimal run, `profiles.rs`,
  `publisher.rs`, and the registration, rename, `md`-profile, `err`-view, and
  action-type tests in `protocol_tests.rs`; `mod.rs` says which is which.
- **Claudine:** no driver uses `Run`; behavior unchanged. `run.rs` and
  `run/protocol.rs` joined the `source-inputs` of `claudine` and
  `claudine-cli` (their guards scan `darkmatter/lib/src/lifecycle/`).
- Scoped results: darkmatter `--lib` lifecycle 86/86; darkmatter `l1`
  lifecycle 89/89; claudine `--lib` lifecycle/looping/sequence 1550/1550;
  claudine `l1` lifecycle/guard/spike 47/47; claudine-cli `l1`
  `lifecycle_traces` 33/33; `just lint` darkmatter and claudine: pass.

### Wave 6 — Claudine bridge (done)

- **Dependency-direction guard (criterion 3):**
  `darkmatter/lib/tests/l1/dependency_direction_guard.rs` (declared in
  `tests/l1/main.rs`). From `darkmatter`, `darkmatter-cli`, and `dmls`, it
  walks every dependency table (`dependencies`, `dev-dependencies`,
  `build-dependencies`, `target.*`), resolves `workspace = true` through the
  root `[workspace.dependencies]` (none exists today), follows `path`
  dependencies to their manifests, and fails on a Claudine package name
  (`claudine`, `claudine-cli`, `claudine-contract`, `claudine-gen`,
  `claudine-catalog-types`, `rendezvous-*`, including a `package =` rename) or a
  manifest under `claudine/`. A transitively reached package contributes only
  the tables Cargo builds for a dependency (not its `dev-dependencies`). It
  also scans the three crates' production sources (`source_scan`, comments,
  literals, and test code blanked) for a Claudine crate in a path, a `use`, or
  an `extern crate`. Self-tests inject a Claudine dependency (a renamed
  `cfg(unix)` path dependency one hop away, and a renamed workspace entry) and
  a `use claudine::…;`, and check that each is reported. The real walk asserts
  that it reached `biscuit-file/lib/Cargo.toml`. It uses the existing `toml`
  dev-dependency; no dependency was added. The manifests and `source_scan.rs`
  are `include_str!` literals, so CI's test-input index maps a manifest edit to
  the guard. 4 tests, 0.5 s in total (the source scan).
- **Untraced recovery routes, now traced** (`claudine/cli/tests/l1/lifecycle_traces.rs`,
  35 scenarios; Phase 1 called them open):
  - `retry-in-initialize`: `event initialize`, `exit 1`. The retry raises
    `LifecycleSetupPhaseRecoveryUnsupported` (`initialize`, `retry`) and the
    run ends with no catch event and no `finalize`.
  - `retry-in-preflight-blocked` (shell audit denied): `event initialize`,
    `event blocked`, `err composition.shell_approval`, `event finalize`,
    `err composition.shell_approval`, `exit 1`. `finalize` runs reading the
    blocking error; afterwards the retry becomes
    `LifecycleSetupPhaseRecoveryUnsupported` (`blocked`, `retry`), which is
    the reported error.
  - `retry-in-start` (before launch): `event initialize`, `event start`,
    `event start`, `work fresh 0`, `event success`, `event finalize`,
    `err none`, `exit 0`. The retry re-runs preflight (attempt 2) without
    `initialize` or `finalize`. The second `start`'s retry finds the budget
    spent and the run continues to launch.
  - Against spec section 8 and the prototype: the prototype ran refusal
    cleanup (`finalize`) after a retry in `initialize`; production does not.
- **`Run` changed** (`run.rs::admits_recovery`/`request_recovery`; new
  request-sequencing code, not Claudine knowledge). All three replays failed
  first: the run finalized after an `initialize` retry, showed
  `SetupRecoveryUnsupported` to a preflight `blocked`'s `finalize`, and refused
  a `start` retry. Now `start` admits every recovery, an `initialize` retry
  ends the run without `finalize`, and a preflight `blocked`'s `finalize`
  reads the blocking error (`fire_reading`), while the selected error is still
  `SetupRecoveryUnsupported`, so `blocked_reached_from_preflight_cannot_recover`
  passes unchanged. `initialize` also admits resume (refused without a
  session) and hand-off, following Claudine's `initialize` dispatch in
  `pipeline.rs`. Those two rows are **untraced**. The
  `EventRequest::error` doc was updated. New protocol test
  `setup_phase_recovery_follows_production`. **35/35 replays** reproduce, and
  `every_recorded_trace_has_a_replay` passes.
- **Still divergent (untraced, unchanged):** production reports `resume` in a
  preflight `blocked` as `LifecycleResumeWithoutSession`. `Run` reports
  `SetupRecoveryUnsupported`, which an existing protocol test asserts.
- Scoped results: darkmatter `l1` guard + lifecycle 97/97; darkmatter `--lib`
  lifecycle 86/86; claudine-cli `l1` `lifecycle_traces` 36/36; `just lint`
  darkmatter and claudine: pass; `just check-tier-coverage darkmatter`: pass.

### Phase 2 checkpoint and gates

All gates ran once, sequentially, after every subagent finished and with
nothing else running.

- `cd claudine && just test`: **10,587 run, 10,587 passed** (49 slow, 9
  skipped), 331.7 s. Against the entry gate's 10,606: 26 tests moved to
  Darkmatter with their files (`actions/tests.rs` 21, `shell_template.rs` 3,
  `authored_order.rs` 2) and 7 were added (2 guarded-shell traces, 3
  retry-route traces, 2 error-parity tests): 10,606 − 26 + 7 = 10,587. All 35
  `lifecycle_traces` pass, as do the type-narrowing conformance matrix
  (`executor/tests/scope_site_conformance.rs`, `sequence/preflight/tests.rs`)
  and `type_scope_ownership_guard`, with their assertions unedited.
- `cd claudine && just lint`: pass. `just check-tier-coverage claudine`: 0 stranded.
- `cd darkmatter && just test`: **10,889 run, 10,889 passed** (10 skipped)
  on the third run. The first two runs each failed one test,
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  ("the watch set changed, so it is registered again", left 3, right 2). It is
  **pre-existing and load-sensitive**: it passed alone 3/3, and in a
  temporary worktree at `HEAD` (`d48d7dcf0`, without this phase's changes,
  separate target dir) the DMLS binary failed it in 2 of 7 runs. Nothing in
  this phase touches DMLS. The worktree was removed afterward.
- `cd darkmatter && just lint`: pass. `just check-tier-coverage darkmatter`: 0 stranded.
- **Dependents.** Darkmatter's public API changes are additive (the new
  `lifecycle` module, plus `compose_request`, which moved into it), and
  Claudine's old lifecycle paths are re-exported.
  `cargo check --all-targets` of every direct dependent of `darkmatter` or
  `claudine` (`biscuit-icon-cli`, `biscuit-speaks-cli`, `messenger`,
  `darkmatter-cli`, `claudine-gen`, `playa-cli`, `messenger-cli`, `dmls`,
  `sniff-cli`, `research`, `claudine-contract`): clean.
  `cargo nextest run -p claudine-contract`: 48/48. `claudine-cli`, `dmls`, and
  `darkmatter-cli` ran inside the two area suites.
- **Spike tests (criterion 19):** both `lifecycle_control_flow_spike` tests
  pass. Claudine's imports point at `darkmatter::lifecycle` for moved types;
  `spike/model.rs` and `spike/scenarios.rs` did not move.

#### Checkpoint criteria

| Criterion | Evidence |
|---|---|
| 2 (no Claudine behavior change) | every pre-existing Claudine assertion passes unedited (only imports and include paths changed in tests); 35/35 traces unchanged; 35/35 replays through the production `Run` |
| 3 (dependency direction) | `darkmatter::l1 dependency_direction_guard` (manifests of lib, CLI, DMLS plus path-dependency closure; sources; self-tests) |
| 10 (section 8 behavior under the new owner) | the D1–D8 trace rows replay through Darkmatter's `Run`, and Claudine runs them through Darkmatter's `LifecycleRunGuard`/`decide_lifecycle_transition`/`LifecycleCatchProtocol` |
| 19 (spike tests) | both spike tests pass; imports repointed |
| "no moved module imports a Claudine type" | `dependency_direction_guard` source scan; `lifecycle_lift_ready_guard` over the slice |

#### Requirement → test mapping (Phase 2)

| Requirement | Tests |
|---|---|
| Entry gate: guarded hole present / nulled at a reached shell | `claudine-cli::l1 lifecycle_traces::{guarded_shell_present, guarded_shell_null}`; `darkmatter::l1 lifecycle_boundary::replay::{guarded_shell_present, guarded_shell_null}` |
| Model extraction: one parser, same errors | Claudine's unchanged lifecycle suites through the wrappers; `composition::error::lifecycle_authoring::{every_lifecycle_error_converts_to_its_same_named_twin_with_the_same_display, a_typed_cause_survives_the_conversion}` |
| Opaque ignored action (`md` profile) | `darkmatter lifecycle::tests::{reject_*, ignore_*}` (6) |
| Engine extraction: executor, run guard, catch, admission, modes | `darkmatter lifecycle::{executor,run_guard,runtime,control}::tests` (fake hosts), plus every Claudine lifecycle/looping/sequence/CLI test through the adapters |
| Gated request/reply: stale/duplicate/foreign/wrong-kind/late replies, cancellation invalidation, per-subject isolation | `darkmatter::l1 lifecycle_run_protocol` (21, including `concurrent_runs_share_no_state`, `setup_phase_recovery_follows_production`) |
| Guarded hole filled by the moved executor, `MissingValue` before launch | `guarded_shell_null` trace; `guarded_shell_values` CLI tests; `executor/tests/guarded_shell_sites.rs` |
| Services: missing service is typed; envelope and render parity | `darkmatter lifecycle::{services,envelope,context,effects,cancellation}::tests` |
| Untraced recovery routes | `lifecycle_traces::{retry_in_initialize, retry_in_preflight_blocked, retry_in_start}` + replays |
| Dependency guard | `darkmatter::l1 dependency_direction_guard` (4) |

Input Robustness Matrix: not applicable to Phase 2. No catalog or trigger
loader changed; the lifecycle text grammar is excluded by the plan.

#### Decisions and departures recorded this phase

- **Request/reply driving is built, but Claudine's drivers do not use it
  yet.** The plan's Wave 5 asks for the gated API, and it exists as
  `darkmatter::lifecycle::run::Run`, sharing `RunAccounting` and the routing
  functions with `LifecycleRunGuard`, so there is one engine. Claudine's
  drivers still call the guard directly; rewiring them is Phase 3's "Driver
  migration", as the plan schedules it.
- **Error twins:** `LifecycleError` (Darkmatter) and Claudine's same-named
  `CompositionError` variants coexist, joined by one exhaustive `From` and a
  `Display`-parity test. Phase 3's "delete superseded" task can fold them.
- **Kept in Claudine, behind seams:** the binding catalog's availability
  rules (`err`, `loop`, and the `seq`/`group` roots, read through
  `LifecycleHost`), `validate_no_err_in_no_error_events` and
  `find_removed_loop_read` (`binding_checks.rs`), `LoopBreak`/`LoopBreakSlot`,
  `PrepRunner`, `RuntimeState`, `FrontmatterExcerpt`, `IterationSummarySignals`,
  `resolve_proxy_target` (returns `HarnessError`), process and shell execution,
  and the lifecycle pre-flight walk (`composition/preflight.rs`, Phase 3).
- **Guard coverage moves:** `type_scope_ownership_guard` and
  `composition_seams` now also scan `darkmatter/lib/src/lifecycle/`; the
  `path_lookup_guard` exception for `proxy_handoff_allowed` moved to
  Darkmatter's `path_lookup_guard` unchanged. The `transport-allow.toml` entry
  for `actions.rs::type_with_value` was removed, so no error-text guard covers
  that site in Darkmatter.
- **Open, untraced divergence:** production reports a `resume` in a
  preflight-reached `blocked` as `LifecycleResumeWithoutSession`, while `Run`
  reports `SetupRecoveryUnsupported` (an existing protocol test asserts the
  latter). `initialize` resume/hand-off rows in `Run` follow `pipeline.rs`
  but are untraced. A cancelled root run (exit 130) is still untraced.

#### Cross-OS check (Phase 2)

Moving code between crates carries the classic `-D warnings` risk of an import
or item used only under one `cfg`. Claudine's executor adapter keeps the
`cfg(windows)`/`cfg(not(windows))` system-shell launch code. So the phase ran
the native Windows rig (`build-win-native`) through `just cross-check`:

- `claudine` (lib, all L1): **5,519/5,519 passed**.
- `darkmatter`, filtered to `lifecycle dependency_direction path_lookup_guard type_scope`
  (every test this phase added or moved, plus the guards it edited): **228/228 passed**.
- `claudine-cli` (all L1 and bin): **3,673 passed, 9 failed**. All 9 are
  **pre-existing**: from a temporary worktree at `HEAD` (`d48d7dcf0`, without
  this phase's changes; removed afterward), the same rig ran exactly these
  tests and **all 9 failed the same way** (0/9 passed). None involves code
  this phase changed; the system-shell launch code is byte-identical to
  `HEAD`. All 9 tests arrived on 2026-10-20 with the type-narrowing review
  repairs and strict-equality fix (`13cbc87c6`, `455eb05e4`), after the
  last Windows evidence for this area:
  - `lifecycle_shell_templates::a_lifecycle_exact_key_keeps_a_quoted_empty_argument`
    and `sequence_shell_templates::sequence_task_exact_keys_keep_quoted_empty_arguments`:
    an exact-approved command runs its resolved bytes through the system
    shell, and `cmd.exe` passes `''` literally (`["-m", "", "-s", "''"]`
    instead of `""`). How an exact approval's empty argument should be
    parsed on Windows is the open author item that the type-narrowing review
    (`review-1.md`) records ("Earlier exact-approval handling of an empty
    argument remains an author item").
  - `implement_plan_commit_message::*` (7): the recorder installed as `git`
    records an extra `["--exec-path"]` first. `gix-path` runs
    `git --exec-path` on Windows to find the system Git configuration unless
    `EXEPATH` names a Git for Windows layout. The fixture's `git` is the
    recorder, so the probe lands in the recording, which the module doc says
    must fail the exact comparison. The `worktree` crate's workaround
    (`name_installation_for_gix`) needs a real launcher layout and would not
    apply here. Setting `EXEPATH` in the fixture would hide a real Windows
    `git` call that the test exists to catch.
- **No code fix was attempted.** The first family needs the author's
  exact-approval decision; the second needs either a production change to
  how Claudine's in-process Git discovery reads system configuration on
  Windows or a ruling on what the test may record. Neither belongs to this
  feature, and a test-side workaround would weaken a passing assertion.
  The spec carries `os_failing: windows` so the pipeline routes them for
  Windows-side repair.

## Phase 3

- **Starting revision:** `8a53d7db3` on `feat/pipeline` (2026-10-21), Phase 2 committed.
- **Orchestration:** the three Wave 7/8 code tasks touch overlapping files
  (`darkmatter/lib/src/lifecycle/{mod,parse,error}.rs`, Claudine's
  `composition/lifecycle/`, the CLI pipeline), and in one shared worktree a
  half-finished edit by one agent breaks every other agent's build. So code
  tasks run **one at a time**. Read-only design work runs beside them: the
  registry implementation ran together with the preflight and
  driver-migration design spikes.

### Design decisions (made before code, from two read-only design reviews)

#### Driver migration: one policy kernel, not a request/reply rewrite

Three options were checked against the code at `8a53d7db3`:

- **(A) Rewrite every Claudine driver as a request/reply loop over `Run`.**
  Rejected under criterion 2. About 200 existing test call sites drive
  helpers that take `&mut LifecycleRunGuard`, starting from states `Run`
  cannot reach without walking its whole protocol. For example,
  `loop_control/tests/lifecycle_ordering.rs` marks the provider launched
  with no `initialize`/`start`, then runs a terminal event. Those helpers
  also make decisions, so keeping their signatures while `Run` sequences
  above them would leave two decision-makers. The rewrite would cover about
  15,000 driver lines.
- **(B) Make `LifecycleRunGuard` an adapter that owns a `Run`.** Rejected.
  The guard's direct-call API permits orders that `Run`'s gating refuses,
  such as recording `success` with no preparation reply
  (`guard_runtime.rs`). Supporting them would need a free-order mode in
  `Run`, which is a third state machine in disguise.
- **(C+) Chosen: a policy kernel.** Accounting (`RunAccounting`), routing
  (`decide_lifecycle_transition`), the catch protocol, and control admission
  are already single-sourced. The duplication is the policy above them:
  which recoveries an event admits, the stage table that picks
  `blocked`/`failure`, the setup-phase refusals, resume without a session,
  and the proxy-hop count. Today that policy is decided again in
  `pipeline.rs::route_initialize`, `looping/engine.rs` (twice),
  `wrap/composition/preflight.rs`, `error_routing.rs`, `sequence/mod.rs`,
  `ContainerRunner::record_outcome`, and `Run`. It moves into pure functions
  in one Darkmatter module, which `Run`, `LifecycleRunGuard`,
  `ContainerRunner`, and every Claudine driver call. A source guard rejects
  any decision logic outside that module. `Run` stays as the request/reply
  shell, which the `md` host needs in Phase 4. The guard and
  `ContainerRunner` stay as direct-call shells over the same kernel.
- **Departure from the plan's wording.** The plan says to "wire …
  document/task/group/sequence/loop runs to the engine". Under C+, every
  decision, all accounting, and the routing run through one implementation,
  but Claudine's drivers call it directly rather than through `Run`'s
  request/reply protocol. This is raised for the author's review.
- **Divergences to trace before unifying** (production is the authority,
  spec section 8):
  - `resume` in a `blocked` reached from preflight: production reports
    `LifecycleResumeWithoutSession`, `Run` reports `SetupRecoveryUnsupported`.
  - `defer` in that same preflight `blocked`.
  - `resume` and `proxy` in `initialize`.
  - A cancelled root document and a cancelled root sequence (exit 130).
    Production fills the terminal slot with `Cancelled` even when no
    `cancelled` stack is authored; `Run::cancel` fills it only when one is.
  - **New:** the proxy-hop budget. Claudine counts `ledger.chain().len()`,
    which includes the origin document; `Run` counts adoptions only. This is
    an off-by-one at `MAX_PROXY_HOPS`.
  - **New:** a setup-phase recovery refusal in a container. A group runs
    `failure` then `finalize`; the root sequence runs `finalize` only.
- **Error twins stay, documented.** A type alias cannot alias enum variants,
  and folding the twins into `CompositionError::Lifecycle(LifecycleError)`
  would edit 234 references in 39 test files, against criterion 2. The twins
  remain Claudine's catalog projection of `LifecycleError`, reached only
  through `From`, and the source guard rejects direct construction of a
  parser twin in Claudine.

#### Preflight integration

- **Design:** add a Darkmatter `lifecycle::preflight` module: an ordered,
  host-registered check plan (`PreflightPlan` → `PreflightRun::check`). A
  check is either `BeforeRun` or `InRun`, and an `InRun` failure becomes a
  `HostFailure { stage: Preflight }`. Claudine's checks keep their own error
  types, so diagnostics stay byte-identical. The approval-free half of the
  lifecycle shell walk (candidate collection, template stamping, guarded
  templates, missing values) moves to Darkmatter behind a bindings seam.
  Claudine keeps the prompting approval, the shell policy, and the sequence
  inventory.
- **Already satisfied by today's code** (each with an existing test, listed
  in the design review): shell-free bootstrap; `initialize` before body
  discovery; initialization mutations feeding the first composition; the
  lifecycle-only audit when body shells are disabled; separate compose and
  lifecycle shell permission, unlocked at `start` and relocked on adoption;
  refusal in dead branches, catch routes, and whole-value `set`; the
  exact/template check at execution after mutations.
- **Gaps:** reuse of the approved discovery graph and the remote-fetch
  runtime (Claudine composes again after `compose_preflight`), and a test
  that a parent's root stack keeps its identity when a child authoring the
  same event is merged in.
- **Criterion 9 versus criterion 2.** Agent resolution runs before
  `initialize`, and the eager schema verdict for a document without
  `initialize` fires no lifecycle event today. Criterion 9 says a failed
  check "fires `blocked` then `finalize`". Section 7 and criterion 2 forbid
  changing what Claudine does, and
  `compose_schema_cli::a_required_property_violation_stops_before_lifecycle_actions_under_existing_recovery_policy`
  asserts no events. So the check plan has a `BeforeRun` stage that fires
  nothing. This is recorded as a departure and raised for the author.

### Wave 7 — Host registry (done)

- **New:** `darkmatter/lib/src/lifecycle/registry.rs` (+ `registry/tests.rs`),
  promoted from the test-only prototype. Types: `CoreEvent`, `Position` and
  `Role` (`FromStr` → `DescriptorParseError`), `ActionType(s)`, `EventId`,
  `EventDescriptor`, `OutcomePair`, `RegistrationError`, `KeyResolution`,
  `RunPoint`, `Emitter`, `EmissionError`, `EventRegistry`. `EventRegistry::md()`
  is core-only, with no renames and no outcome pair. Registration fails with
  `RunStarted` while a run shares the registry (`Arc`).
- **Engine slots:** `LifecycleSignal` keeps its variant names; criterion 2
  forbids editing assertions that name them. The registry maps each event to
  its slot: `initialize`→`Initialize`, `composed`→`Start`, `failed`→`Blocked`,
  `cancelled`→`Cancelled`, `finalize`→`Finalize`, the outcome pair's
  succeeded/failed alternatives→`Success`/`Failure`, and the iteration
  gate→`Loop`. A notification event has no slot. Authored names come only
  from the registry (`name_of`, `resolve_key`, `authored_event_keys`).
- **Claudine registration:** `claudine/lib/src/composition/lifecycle/registry.rs`
  (`claudine_registry()`, built once). It renames `start`/`blocked`, registers
  the `success`/`failure` outcome pair with downgrades `success`→`failure` and
  `blocked`→`failure`, and `loop` as the gate after `finalize`. `err`
  availability follows the old catalog rule (`can_carry_error`), `timing` is
  available in every event, and host globals are `loop` and `group`.
- **Renames:** `LifecycleParseOptions.registry`. A Darkmatter spelling under a
  renaming host raises `LifecycleError::LifecycleRenamedEvent` before any stack
  parses. Its Claudine twin `CompositionError::LifecycleRenamedEvent` uses code
  `composition.lifecycle_invalid`, detail `property`, and a hint naming the
  replacement. Under `md`, host-only keys are plain data. A repository search
  found no Claudine document, fixture, or prompt using `composed:`/`failed:`
  as a top-level key, so no existing behavior changes.
- **Hand-maintained lists deleted:** `LIFECYCLE_EVENT_KEYS` (consumers call
  `claudine_registry().authored_event_keys()`; the executor's `with:` check
  calls the new `LifecycleHost::registry()`) and `LIFECYCLE_HOST_GLOBALS`
  (`lifecycle_declared_types(schema, registry)`). The derived lists equal the
  old ones exactly, order included (tested).
- **Prototype:** `lifecycle_boundary/engine.rs` lost its registry types and
  re-exports the production ones under the old names.
  `protocol_tests.rs`, `publisher.rs`, and `replay.rs` are unchanged and pass
  against production (criterion 7: the publisher's before-composition event,
  its outcome pair, and its downgrade).
- **Departures:** registration errors carry no source location, because
  registration is code-only and the failing call is its location.
  `check_emission` is API-only until the driver migration calls it. Per-event
  action types are declared but not enforced by the parser; the existing
  placement rules still apply. `validate.rs`/`source_map.rs` still look up an
  authored record under `LifecycleSignal::property_name`, which matches
  Claudine but not `md`'s names. **Phase 4 must route those through the
  registry.** The error-parity test's variant count rose 26→27, a count
  assertion in a Phase 2 test.
- **Tests added:** darkmatter
  `lifecycle::registry::tests::{every_registration_error_is_typed,
  an_unknown_position_or_role_spelling_is_a_typed_error,
  only_the_engine_fires_an_event_and_only_at_its_registered_point,
  md_registers_only_the_core_events_under_their_own_names,
  a_renaming_host_maps_every_event_to_its_engine_slot_in_a_fixed_key_order}`,
  `lifecycle::tests::{a_renaming_host_refuses_the_darkmatter_spelling_before_any_stack_parses,
  md_parses_its_own_spellings_and_leaves_host_only_keys_as_data}`; claudine
  `composition::lifecycle::registry::tests::{claudine_registry_builds_and_derives_the_previous_lifecycle_keys_and_host_globals,
  claudine_registry_maps_every_slot_to_its_claudine_name,
  claudine_registry_err_availability_agrees_with_the_binding_catalog,
  initialize_alone_excludes_shell_actions}`,
  `composition::lifecycle::tests::parse_config::{darkmatter_spellings_of_renamed_events_fail_naming_claudines_spelling,
  claudines_own_spelling_of_a_renamed_event_parses}`,
  `composition::error::tests::lifecycle_renamed_event_is_a_lifecycle_authoring_error_naming_the_replacement`;
  claudine-cli `l1 lifecycle_renamed_events::{compose_dry_run_refuses_a_composed_key_naming_start,
  compose_refuses_a_failed_key_before_initialize_runs_or_a_provider_launches}`
  (the second is Unix-only because it uses `launch_recorder`; its `blocked:`
  control succeeds).
- **Scoped results:** darkmatter `--lib` lifecycle 93/93; darkmatter `l1`
  lifecycle + dependency guard + feature_review_incident 100/100; claudine
  `--lib` lifecycle/preflight/prepare/error 1539/1539; claudine `l1`
  lifecycle/type_scope/guard/diagnostic_detail 51/51; claudine-cli `l1`
  traces/renamed/sequence/seams/guard 1133/1133; claudine-cli bins sequence
  76/76; doctests pass; `RealWorkspaceTestInputTests` 7/7; `just lint`
  darkmatter and claudine: pass.

### Wave 7 — Preflight integration (done, with two recorded gaps)

- **New:** `darkmatter/lib/src/lifecycle/preflight.rs` with
  `preflight/{shell_walk,candidates,tests}.rs`:
  - The check plan: `CheckStage {BeforeRun, InRun}`, `CheckSpec`,
    `PreflightPlan::{new, register, names, begin}`,
    `PreflightRun::{entry, advance, check, skip, ready}`, and
    `PreflightFailure<E>::host_failure`. An `InRun` failure is
    `HostFailure { stage: Preflight, entry }`; a `BeforeRun` failure is no
    host failure.
  - Errors: `PreflightRegistrationError`, `PreflightOrderError`, and
    `PreflightError` (three variants, each converting to an existing
    `CompositionError` variant, with a Display/source parity test).
  - The moved, approval-free shell walk: `LifecycleShellSites::stamp` (the
    old `resolve_lifecycle_shell_commands_in_sequence` with `resolve_shell_text`,
    `guarded_template`, `sequence_lifecycle_template`, `missing_value`), the
    host seam `ShellAuditBindings` with Claudine's binding-catalog answers,
    `ShellWalkOptions`, the condition-blind `collect_lifecycle_shell_audit`,
    `lifecycle_shell_candidates`, `settle_lifecycle_shell_modes`,
    `lifecycle_document_roots`, and `unavailable_root`.
- **Claudine:** `composition/preflight/checks.rs` registers
  `agent_resolution`, `launch_schema`, and `lifecycle_shell_audit` in today's
  run order (`claudine_preflight()`). The sequence plan
  (`sequence_preflight()`) keeps the sequence's order: graph shell approval,
  then provider selection. Each call site runs its unchanged body inside
  `check(...)`, so prompts and diagnostics keep their order and text.
  Approval prompting, the shell policy, closed approvals, the sequence
  inventory, and `binding_checks.rs` stay in Claudine.
  `composition/preflight.rs` keeps every public signature as a thin wrapper.
- **Stages:**
  - `BeforeRun` (no lifecycle event, as today): agent resolution in
    `prep.rs` and `pipeline.rs::resolve_execution_target`, and the eager
    schema verdict.
  - `InRun`: the staged reread and verdict, every lifecycle shell audit,
    the per-attempt provider rebuild, and both sequence checks.
- **Criterion 9 departure** (see "Design decisions"): `BeforeRun` failures
  keep firing no `blocked`. `compose_schema_cli` and every sequence and
  `wrap_compose` suite pass unedited.
- **Gaps recorded:**
  - **Remote-fetch reuse:** not applicable. Claudine never enables remote
    reads (nothing in `claudine/{lib,cli}/src` configures remote
    transclusion or calls `with_shared_remote_fetch`), so there is no fetch
    to share. Enabling one would be a behavior change.
  - **Discovery-graph reuse (`with_preflight_graph`):** not done. Claudine's
    preflight and canonical compose options differ (for example
    `with_deferred_shell_value_validation`), and sharing them risks moving
    `ctx_per_run`'s observation counts. Composition still runs twice, as
    before this feature. This is carried forward as a performance follow-up,
    not a behavior gap.
  - **Each call site opens its own one-check run.** The checks sit in
    different functions, so order is enforced for real only in the sequence,
    where both checks share one run. Threading a single run through a whole
    execution is left to the driver work.
- **Path-only test edits:** `composition_seams.rs` (the site's new module
  path); Claudine's `type_scope_ownership_guard.rs` (allowlist, decision
  sites, and missing-value constructor point at
  `darkmatter/lib/src/lifecycle/preflight/shell_walk.rs`, and the self-test
  `a_decision_site_that_stops_consulting_the_scope_is_rejected` names the new
  path in its snippet and expected message); `preflight/tests.rs` (two
  imports). Darkmatter's `type_scope_ownership_guard.rs` gained the
  `LifecycleShellSites::stamp` `UNGUARDED` entry, with the reason unchanged.
- **Tests added:**
  - darkmatter `lifecycle::preflight::tests::{a_check_name_registers_once,
  a_run_reaches_checks_in_registration_order_and_may_pass_over_one,
  a_check_run_out_of_order_fails_a_debug_assertion,
  a_passing_walk_is_ready_at_its_epoch,
  a_failure_before_the_run_is_not_a_host_failure,
  a_failure_inside_the_run_is_a_preflight_host_failure_of_its_entry,
  the_audit_collects_every_lifecycle_shell_condition_blind,
  the_audit_of_selected_events_collects_only_theirs,
  a_placeholder_a_guard_proves_present_is_approved_as_a_typed_template,
  an_unguarded_missing_placeholder_fails_preflight_naming_its_property}`,
  plus a doctest.
  - claudine
  `composition::error::lifecycle_preflight::tests::every_preflight_error_converts_to_its_same_named_twin_with_the_same_display_and_source`,
  `composition::preflight::checks::tests::{claudine_registers_its_checks_in_todays_run_order,
  a_check_returns_its_bodys_own_error}`,
  `composition::prepare::tests::a_transcluded_childs_start_stack_never_replaces_the_parents`,
  and l1 `lifecycle_lift_ready_guard::{preflight_names_no_runtime_type,
  preflight_shell_walk_names_no_runtime_type,
  preflight_candidates_names_no_runtime_type}`.
  - claudine-cli l1
  `ctx_per_run::an_included_files_start_stack_neither_replaces_nor_joins_the_parents`
  (the gap the design review found).
- **Scoped results:** darkmatter `--lib` lifecycle 103/103; darkmatter `l1`
  guards + lifecycle 105/105; claudine `--lib` 1255/1255; claudine `l1`
  guards + spike 28/28; claudine-cli `l1` 1261/1261 + 318/318; claudine-cli
  bins 389/389; 35/35 traces unchanged; `just lint` darkmatter and claudine:
  pass.

### Wave 8, step 1 — Divergence traces (done)

Eleven new production traces were captured from the real binary, each with a
replay through Darkmatter's `Run`. No production code changed and no existing
`.trace` was edited.

- **Harness additions** (`claudine/cli/tests/l1/lifecycle_traces.rs`):
  - `INTERRUPT = 130`: a provider launch with this status sends `SIGINT` to
    its `claudine` parent, after checking with `ps` that the parent really is
    `claudine`, so no window or terminal is involved.
  - `write_proxy_hops` generates hop documents in the test's temp dir.
  - An optional `generate:` field in `scenarios!`.

  The trace module was already `#[cfg(unix)]`. Each cancellation test takes
  about 1.0–1.5 s and each hop-limit test about 1.7 s. Five runs in a row
  showed no flakes.
- **Replay additions** (`darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs`):
  - New vocabulary: `defer`, an interrupted launch, an interrupting task, a
    per-scenario cancellation error, and a `reported <label>` check of the
    run's final error.
  - A `KNOWN_DIVERGENT` list, where each listed scenario asserts that it
    still differs from production, plus `known_divergences_still_diverge`,
    which fails once `Run` matches a listed scenario. So the list cannot go
    stale.

| Scenario | Production | `Run` |
|---|---|---|
| `resume-in-preflight-blocked` | `initialize`, `blocked` (`err composition.shell_approval`), `finalize` (same `err`), exit 1; reports `LifecycleResumeWithoutSession` | **diverges**: same events, reports `SetupRecoveryUnsupported` |
| `defer-in-preflight-blocked` | same events; reports `LifecycleDeferNotImplemented` | **diverges**: aborts with no `finalize` |
| `resume-in-initialize` | `initialize`, exit 1 (`LifecycleResumeWithoutSession`, no catch event, no `finalize`) | matches |
| `proxy-in-initialize` | `initialize`, then the target's `initialize`, `start`, work, `success`, `finalize`; exit 0 | matches |
| `cancelled-root-document` | `initialize`, `start`, work 130, `cancelled` (`err provider.interrupted`), `finalize`, exit 130 | matches |
| `cancelled-root-document-unauthored` | as above without `cancelled` | matches |
| `cancelled-root-sequence` | `seq:initialize`, `seq:start`, interrupting task, `seq:cancelled`, `seq:finalize` (`err provider.interrupted`), exit 130 | matches |
| `proxy-hop-limit` (hand-offs from `start`) | adopts hop-01 … hop-15, then exit 1 with `LifecycleProxyCycle` ("limit of 16"), no `finalize` | **diverges**: `Run` also adopts hop-16 |
| `proxy-hop-limit-after-launch` (hop-15 hands off from `success`) | … hop-15 `success`, hop-15 `finalize` (`err none`), exit 1 | **diverges**: `Run` adopts hop-16 and skips hop-15's `finalize` |
| `retry-in-initialize-root-sequence` | `seq:initialize`, exit 1 | matches |
| `retry-in-initialize-group-leg` | leg `initialize`, leg `failure` (`err composition.failed`), leg `finalize`, then `checks` and `seq` `failure`/`finalize`, exit 1 | **diverges**: `Run` runs nothing for the leg after `leg:initialize` |

- **Corrections to the design notes:**
  - A root sequence's `initialize` retry runs **no** `finalize`, as a root
    document's does not, and `Run` already agrees. Only a group leg (a
    nested subject) routes a setup-phase refusal to `failure` and then
    `finalize`.
  - An interrupted root reports `provider.interrupted` (the interrupted
    work's error), not `composition.cancelled`.
  - The claim that the slot fills with `Cancelled` with no stack authored
    cannot be seen in a trace; the events match either way.
- **Proxy-hop rule (production):** the chain counts the origin document, so
  the 16th hand-off is refused. Refused before launch, it runs no
  `finalize`. Refused after launch, the refusing document's `finalize` runs
  first.
- **Scoped results:** claudine-cli `l1 lifecycle_traces` 47/47; darkmatter
  `l1 lifecycle_boundary | lifecycle_run_protocol` 86/86; `just lint` in both
  areas: pass.

### Wave 8, steps 2–3 — Policy kernel, with `Run`, `ContainerRunner`, and the Claudine drivers on it (done)

- **New:** `darkmatter/lib/src/lifecycle/policy.rs` (+ `policy/tests.rs`, 14
  tests, each citing the trace or old driver it encodes). It is re-exported
  as `claudine::composition::lifecycle::policy`.
  - `SubjectKind {Document, RootSequence, Sequence, Group}`. There are four
    kinds rather than `Container { root }`, because a group and a nested
    sequence route refusals differently in production.
  - `recovery_verdict(subject, signal, &StackControl) -> Option<RecoveryVerdict {Admit, Ignore, Refuse(Refusal)}>`.
  - `refusal_route(subject, &Refusal) -> RefusalRoute {FinalizeIfOwed, FinalizeIfOwedReadingRefusal, FailureThenFinalize}`.
  - `failure_event(FailureStage, launched)`.
  - `proxy_budget_exhausted(documents_in_chain)`, the one hop-count
    definition. The chain includes the origin, so the 16th hand-off is
    refused. `decide_lifecycle_transition`, `proxy_handoff_allowed`, and the
    ledger's `approve_hop` all use it.
- **Decision code replaced by kernel calls:**
  - In `Run`: `admits_recovery`, the refusal branch of `request_recovery`,
    `abort`, the defer arm, the stage table in `refuse`, and the
    `proxy_hops` counter.
  - In `ContainerRunner::record_outcome`: the per-control arms and the
    inline budget, which now uses `control_budget_for`.
  - In Claudine:
    - the `initialize` table in `pipeline.rs::route_initialize`;
    - both tables in `looping/engine.rs` (`initialize` and the loop gate);
    - `wrap/composition/preflight.rs` (`setup_phase_deferred` deleted;
      `preflight_blocked_control_error` keeps its signature);
    - the `blocked`/`failure` choice in `error_routing.rs` and
      `sequence/mod.rs`.
  - The three container decision-to-error mappings (`wrap/sequence/container.rs`,
    `sequence/task/sequence.rs`, `sequence/task/group.rs`) are now one
    function, `CompositionError::from_lifecycle_refusal`, routed through
    `ContainerRunner::refusal_route`.
- **All five known divergences are fixed;** `KNOWN_DIVERGENT` is `&[]`. The
  group leg is replayed as its own run of kind `SubjectKind::Sequence`
  (`RunSettings::with_subject`).
- **Production rules the kernel now states once:**
  - A document's preflight `blocked` ignores `retry`/`resume` with
    `max_attempts: 0`; a container's `blocked` refuses it.
  - A group runs `failure` then `finalize` for a setup refusal, but only
    `finalize` (reading the refusal) for a refused resume or defer.
  - A nested sequence runs `failure` then `finalize` for both.
  - The root sequence and a root document run only a `finalize` they
    already owe.
  - A container's `finalize` ignores every recovery directive.
- **Behavior changes in untraced cases:**
  - `retry`/`resume` with `max_attempts: 0` in a document's `initialize`
    used to reach `unreachable!` in `route_initialize` and panic. It now
    reports `SetupRecoveryUnsupported`/`ResumeWithoutSession`, as the loop
    engine already did. A regression test is due in the next step.
  - `ContainerRunner::record_outcome` called with `loop`, or with
    `cancelled` outside cancellation, now refuses a recovery. Production
    never calls it that way.
  - `Run` only: a refused `defer` or proxy budget from `success`/`failure`
    runs the owed `finalize` first, which matches `drive_terminal_recovery`;
    a refused `retry`/`resume` with zero attempts in a preflight `blocked` is
    ignored.
- **Test edits beyond imports:**
  - Darkmatter's Phase 2 `lifecycle_run_protocol` tests:
    `blocked_reached_from_preflight_cannot_recover` now expects
    `Aborted(ResumeWithoutSession)` for a resume (per the trace), and
    `setup_phase_recovery_follows_production` no longer replies with a budget
    after an `initialize` resume (refused outright, per `resume-in-initialize`).
  - `replay.rs` renders a `Run`-raised error as `composition.failed`, the
    code Claudine's mapping gives it.
  - **No Claudine assertion changed.**
- **Scoped results:** darkmatter `--lib` lifecycle 117/117; darkmatter `l1`
  boundary/protocol/spike/dependency guard 105/105; claudine `--lib`
  sequence/looping/lifecycle 1443/1443; claudine `l1` lifecycle/guard/spike
  50/50; claudine-cli bins 481/481; claudine-cli `l1` scoped 1347/1347;
  `just lint` in both areas: pass.

### Wave 8, step 4 — Superseded code deleted, one-implementation guard (done)

- **Audit:** three private copies of kernel rules were left in Claudine, and
  all now go through `darkmatter::lifecycle::policy` with identical output:
  - `loop_control/error_routing.rs::run_cancelled_prompt_cleanup`: a copy of
    the run guard's `refused_recovery`. It is now
    `policy::refused_recovery(EvaluationMode::CancellationCleanup, …)`,
    moved from `run_guard.rs` and made public.
  - `loop_control/proxy.rs::run_target_initialize`: now
    `recovery_verdict(Document, Initialize, …)`. It keeps reporting the
    existing `LifecycleTransitionUnownedAtStage` for an adopted target,
    because changing that would change behavior.
  - `loop_control/control_dispatch.rs`: built `LifecycleResumeWithoutSession`
    and `LifecycleDeferNotImplemented` directly; it now uses
    `from_lifecycle_refusal`.

  Every hop check now uses `proxy_budget_exhausted`/`proxy_handoff_allowed`;
  `MAX_PROXY_HOPS` survives only as the reported `limit:` value.
- **Guard** `claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs`
  (criterion 4):
  - **Scope:** it reads the module trees of `claudine/lib/src/lib.rs` and
    `claudine/cli/src/main.rs`, with comments, literals, and `#[cfg(test)]`
    blanked by the shared `source_scan.rs`.
  - **It fails on:**
    - a Claudine definition of a Darkmatter-owned name (parser entry points,
      `StackExecutor`, `RunAccounting`, `LifecycleCatchProtocol`,
      `decide_control`, `decide_lifecycle_transition`, every kernel
      function, `EventRegistry`, `PreflightPlan`, the moved shell-walk
      items, `setup_phase_deferred`, …);
    - a `MAX_PROXY_HOPS` comparison;
    - direct construction of a refusal error outside
      `from_lifecycle_refusal`;
    - construction of a parser twin outside its `From`;
    - an unreviewed implementation of a Darkmatter lifecycle trait or `*Ext`
      extension;
    - a registry or preflight-plan registration outside Claudine's one
      registration.
  - **Allowlist:** each entry is exact (file, item, reason), and a stale or
    reasonless entry fails. It covers the `LifecycleRunGuard`/`ContainerRunner`
    aliases, the error-converting wrappers, the thin preflight wrappers, the
    host seams (`ClaudineRunHost`, `ClaudineLifecycleEffects`,
    `StackExecutionContext`, the shell runners, `ClaudineShellAuditBindings`,
    `FrontmatterExcerpt`, `ClaudineDiagnostics`, the `*Ext` traits), and
    Claudine's own grammar errors (`RemovedGrammar` in loop/sequence/binding
    grammar, `LifecycleInvalid` in the shell-free bootstrap).
  - **Self-tests:** an injected kernel function, a re-export, an allowed
    alias, a stale entry, hop-limit comparison versus report, a refusal
    error versus a pattern, a parser twin, and an unreviewed seam.
- **Error twins:** kept, as decided. The module doc of
  `error/lifecycle_authoring.rs` states they are Claudine's catalog
  projection of `LifecycleError`/`PreflightError`, reached only through
  `From`.
- **Regression tests for the two former panics:**
  `claudine-cli::l1 lifecycle_initialize_zero_attempts::{a_zero_attempt_retry_in_initialize_reports_the_setup_refusal_without_finalize,
  a_zero_attempt_resume_in_initialize_reports_no_session_without_finalize}`
  (Unix; exit 1, typed error, no launch, only `event initialize`). Both fail
  against the old panic path.
- **Open, handed to parity verification:** CI only runs the guard when
  `claudine` lib or a declared `source-inputs` file changes, so a change to
  `claudine/cli/src` alone would not run its CLI half.
- **Scoped results:** claudine `l1` guards + spike 36/36; claudine `--lib`
  1577/1577; claudine-cli bins 1374/1374; claudine-cli `l1` scoped 154/154;
  darkmatter `--lib` lifecycle 116/116; darkmatter `l1` 150/150; `just lint`
  in both areas and `just check-tier-coverage claudine`: pass.

### Wave 8 — Parity verification (done)

**Requirement → test map.** Prefixes: DM-lib = `darkmatter --lib lifecycle::`,
RP = `darkmatter::l1 lifecycle_run_protocol::`, TR/RT = the trace and its
replay, CLI-bin = `claudine-cli --bin claudine …::loop_control::tests::`.
**NEW** marks a test added in this phase.

| Requirement | Tests |
|---|---|
| §8 failure-stage rows | DM-lib `policy::tests::a_preparation_failure_before_launch_is_blocked_and_any_other_is_failure`; RP `the_failure_stage_and_launch_state_select_blocked_or_failure`; TR/RT `blocked_preflight`, `blocked_refused_handoff`, `resume_incompatible`, `catch_failure`, `retry_exhausted`, **NEW** `proxy_hop_limit_after_launch`; CLI-bin `terminal_routing::*`, `lifecycle_ordering::a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize`, `unowned_handoff::*`, `retry_resume::a_refused_resume_routes_through_failure_then_finalize_with_err` |
| `finalize` rules | RP `skip_ends_the_run_from_initialize_without_finalize`, `retry_or_resume_from_a_terminal_event_skips_that_attempts_finalize`, `proxy_from_a_terminal_event_never_runs_the_source_finalize`, `a_recovery_chosen_in_finalize_is_never_followed_by_a_second_finalize_for_that_attempt`, `cancel_invalidates_outstanding_work_then_runs_cancelled_and_finalize`; TR `skip`, `retry_admitted`, `resume_admitted`, `proxy_adopted`, `retry_in_finalize`, `finalize_error`, `loop_break`, **NEW** `cancelled_root_*` |
| Recovery and cleanup modes | **NEW** DM-lib `policy::tests::only_a_cleanup_mode_refuses_a_recovery_and_names_its_verb`; DM-lib `runtime::tests::*`, `run_guard::tests::cancellation_cleanup_reports_a_recovery_instead_of_running_it`; RP `an_evaluation_failure_in_start_is_caught_by_failure_and_finalize_as_cleanup_only` |
| `blocked` routes | RP `an_explicit_error_downgrades_success_to_failure_whose_recovery_stays_eligible`, `an_explicit_error_in_preflight_blocked_is_caught_by_failure_as_cleanup`, `blocked_reached_from_preflight_cannot_recover`; TR `downgrade_success`, `downgrade_blocked`, **NEW** `{resume,defer}_in_preflight_blocked` |
| Concurrent isolation, cancellation generations | RP `concurrent_runs_share_no_state`, `late_replies_after_cancellation_cannot_emit_an_outcome_reopen_shells_or_finalize_again`, `a_cancel_during_cleanup_lets_that_cleanup_finish_without_a_second_pass`, `stale_duplicate_foreign_wrong_kind_and_late_replies_change_nothing` |
| Shell invariants (criterion 8, Claudine) | RP `the_run_alone_answers_shell_permission_from_initialize_to_proxy_adoption`; claudine `grammar::{initialize_refuses_shells_in_either_branch, initialize_refuses_a_shell_valued_set_in_a_dead_branch}`, `set_shell_values::initialize_refuses_a_shell_assignment_in_any_item`, `action_dispatch::programmatic_initialize_shell_is_rejected_even_with_no_error`; claudine-cli `compose_initialize_staged_boot::{initialize_shells_are_forbidden_even_with_yolo_across_entry_paths, early_catch_shells_cannot_run_or_enable_another_catch_shell}`, `preflight_execution_parity::*`; TR `shell_locked_blocked`, `shell_relocked_after_proxy` |
| Templates (criterion 22, Claudine) | claudine-cli `lifecycle_shell_templates::{a_lifecycle_exact_approval_refuses_iteration_two, an_exact_approval_of_the_first_iteration_refuses_the_second}`; DM-lib `preflight::tests::a_placeholder_a_guard_proves_present_is_approved_as_a_typed_template`; darkmatter `shell_template_policy::*` |
| Approval prompt order, no post-launch prompt (criterion 9, section 4) | **NEW** `claudine-cli::l1 level1_approval_order_pty::{level1_pty_a_documents_approvals_all_precede_its_launch, level1_pty_a_sequence_front_loads_every_approval_in_order}`. Both also pass against a build of the baseline `d48d7dcf0` (temporary worktree, removed afterward). |
| Criteria 5, 6, 7, 19 | listed under Wave 7 (renames, registration); `lifecycle_boundary::publisher::*`; both `lifecycle_control_flow_spike` suites |
| Criterion 21 (Claudine parts) | **NEW** `claudine-cli::l1 lifecycle_root_preparation::{a_root_without_read_permission_runs_nothing, a_root_that_is_not_valid_utf8_runs_no_stack, malformed_frontmatter_runs_none_of_its_stacks, a_malformed_stack_keeps_the_well_formed_cleanup_stacks_beside_it_from_running, the_same_stacks_run_from_a_well_formed_root}` (the last is the control); `compose_initialize_creates_a_file_the_body_includes`; **NEW** `ctx_per_run::an_included_files_start_stack_neither_replaces_nor_joins_the_parents`; `compose_initialize_acceptance::an_include_initialize_rewrites_reaches_the_prompt_with_its_new_content`; CLI-bin `lifecycle_ordering::finalize_evaluation_error_aborts_without_reentry`; RP `cancel_without_a_cancelled_stack_runs_only_finalize_even_during_initialize` |

- **Production facts confirmed:**
  - §8 row 7: after launch, a refused hand-off finds `success` already in
    the terminal slot, so only the owed `finalize` runs and the run ends
    failed. The kernel's `failure_event(HandoffRefused, true)` is `Failure`
    for the case where the slot is still free.
  - The hop-limit refusal before launch ends with no `blocked` and no
    `finalize`.
- **Guard split (CI blind spot closed):** the rules and scanner live once in
  `claudine/lib/tests/common/lifecycle_ownership_scan.rs`, included by both
  guards. The lib guard scans `lib/src`. The new
  `claudine-cli::l1 lifecycle_one_implementation_guard::the_claudine_binary_keeps_no_second_lifecycle_implementation`
  scans `cli/src` and has an empty allowlist. `claudine/cli/Cargo.toml`
  lists the shared scan in `source-inputs`. `error/lifecycle_authoring.rs` is
  deliberately not listed: other CLI guards already read `claudine/lib/src`,
  so listing it would schedule the whole CLI L1 binary on every edit to it.
  The reason is in a `Cargo.toml` comment. A mutation check (an injected
  `fn decide_control` in `pipeline.rs`) made the CLI guard fail.
- **Unix-only new tests:** the launch recorder and PTY are Unix-only. The
  no-read-permission test needs a non-root runner, asserts that up front,
  and fails loudly otherwise.
- **Scoped results:** darkmatter `--lib` lifecycle 118/118; darkmatter `l1`
  lifecycle + spike 105/105; claudine `l1` guard + spike 37/37; claudine-cli
  `l1` traces/guard/new 149/149 (each new test ≤ 2.9 s); `just lint` in both
  areas, `just check-tier-coverage claudine`, and `validate_source_inputs`:
  pass.

### Phase 3 checkpoint and gates

All gates ran once, sequentially, after every subagent had finished.

- **`cd claudine && just test`:** **10,634 run, 10,634 passed** (165 slow, 9
  skipped), 418 s, on the third run. The first two runs each failed one test,
  and both causes were in this phase's own fixtures:
  - `shipped_prompts::shipped_lifecycle_artifacts_use_mapping_only_set`
    parses every `.md` under the CLI fixtures, and the new hop template
    `proxy-hop-limit.hop.md` held raw `@BEFORE@` placeholders, which are not
    valid YAML. It was renamed `proxy-hop-limit.hop.template`; it is a
    generator input, not a document.
  - `lifecycle_traces::every_trace_fixture_has_exactly_one_test`, a Phase 1
    test of this feature, then reported the template as an orphan. Its
    companion rule now accepts a `.template` beside a scenario as well as a
    `.md`.
- **`cd claudine && just lint`:** pass. **`just check-tier-coverage claudine`:**
  0 stranded.
- **`cd darkmatter && just test`:** **10,933 run, 10,933 passed** (10 skipped).
  **`just lint`:** pass, including the zed-dmls wasm check.
  **`just check-tier-coverage darkmatter`:** 0 stranded.
- **Dependents:** `cargo check --all-targets` of every direct dependent of
  `darkmatter` or `claudine` (`biscuit-icon-cli`, `biscuit-speaks-cli`,
  `messenger`, `darkmatter-cli`, `claudine-gen`, `playa-cli`,
  `messenger-cli`, `dmls`, `sniff-cli`, `research`, `claudine-contract`):
  clean. Removing `LIFECYCLE_EVENT_KEYS`/`LIFECYCLE_HOST_GLOBALS` broke no
  dependent. `cargo nextest run -p claudine-contract`: 48/48.
- **Cross-OS (native Windows, `build-win-native`).** The phase moved code
  between crates and added `cfg(unix)` test modules, so the risk was notable.
  - `claudine`: **5,542/5,542 passed**.
  - `claudine-cli`: **3,676 passed, 9 failed**. The 9 are exactly the
    pre-existing ones recorded in "Cross-OS check (Phase 2)", which also fail
    at `d48d7dcf0`: `implement_plan_commit_message::*` (7),
    `lifecycle_shell_templates::a_lifecycle_exact_key_keeps_a_quoted_empty_argument`,
    and `sequence_shell_templates::sequence_task_exact_keys_keep_quoted_empty_arguments`.
    No new failure. The spec's `os_failing: windows` stays.
  - `darkmatter`, filtered to
    `lifecycle dependency_direction path_lookup_guard type_scope spike`:
    **272/272 passed**.
  - The traces, the PTY approval-order tests, and the root-preparation tests
    are Unix-only, for the reasons recorded with each.

#### Checkpoint criteria

| Criterion | Evidence |
|---|---|
| 2 (no Claudine behavior change) | every pre-existing Claudine assertion passes unedited (Phase 3's test edits are imports, guard paths, and this feature's own Phase 1/2 tests, each listed above); the 35 earlier traces are unchanged; 46/46 replays through `Run`; two former `unreachable!` panics now raise typed errors, the one intended change, with regression tests |
| 4 (one implementation) | `lifecycle_one_implementation_guard` (claudine lib + claudine-cli) over one shared scanner; the policy kernel is the only decision site |
| 5 (host names) | Wave 7 registry tests and `lifecycle_renamed_events` (CLI) |
| 6 (registration checks) | `registry::tests::*` |
| 7 (prototype host) | `lifecycle_boundary::publisher::*` against the production registry |
| 8 (shell invariants, Claudine) | see the parity table |
| 9 (preflight host checks) | `InRun` checks route `blocked` → `finalize` with unchanged diagnostics (traces `blocked-preflight`, `*-in-preflight-blocked`); sequence approval order is pinned by `level1_approval_order_pty::*` and matches the baseline binary. **Departure:** agent resolution and the eager schema verdict are `BeforeRun` and fire no event, as before (criterion 2 wins) |
| 10 (§8 behavior) | see the parity table; all §8 rows, `finalize` rules, and cleanup modes have tests under the Darkmatter owner |
| 19 (spike tests) | both `lifecycle_control_flow_spike` suites pass |
| 21 (Claudine parts) | `lifecycle_root_preparation::*`, staged-boot, `ctx_per_run`, and `Run` cancellation tests; the `md` output and writer parts are Phase 4 |
| 22 (Claudine parts) | template and approval tests in the parity table; the `md` tolerant-parsing parts are Phase 4 |

#### Decisions and departures recorded this phase

- **Driver migration is a policy kernel, not a request/reply rewrite.**
  Claudine's drivers keep the direct-call `LifecycleRunGuard`, now a thin
  shell over the same accounting and the same kernel as `Run`. A full move
  onto `Run`'s request/reply protocol needs edits to about 200 test call
  sites, which criterion 2 forbids. The author may want to revisit this once
  those tests can be rewritten.
- **Criterion 9:** `BeforeRun` checks fire no `blocked`, as before.
- **Error twins kept,** documented, and guarded (criterion 4 allows
  adapters; folding them would edit 234 assertions).
- **Not done:** reuse of the discovery graph between preflight and canonical
  composition (a performance item; behavior is unchanged). Remote-fetch reuse
  does not apply to Claudine.
- **Phase 4 must:** route `md` names through the registry in
  `validate.rs`/`source_map.rs` and in `LifecycleSignal::as_str()`
  diagnostics; call `check_emission`; enforce per-event action types in the
  `md` profile.
- **Test edits to this feature's own earlier tests** (not pre-existing
  Claudine assertions): `lifecycle_run_protocol` (two tests, per the traces),
  `replay.rs` (`KNOWN_DIVERGENT` emptied, subject kind, error rendering), the
  error-parity count (26→27), and `every_trace_fixture_has_exactly_one_test`
  (accepts a `.template` input).

#### CI scope

The gates were run, not skipped: this phase changed source in `darkmatter`,
`claudine`, and `claudine-cli`.

## Phase 4

- **Starting revision:** `8a53d7db3` on `feat/pipeline` (2026-10-21) with
  Phase 3's changes still uncommitted in the working tree.
- **Orchestration:** the three Wave 9 strands (compose host, global catalog,
  trigger migration) run as parallel foreground subagents in the shared
  worktree. Worktree isolation was not used, because each isolated tree would
  start from `HEAD` and lose the uncommitted Phase 3 work. Each strand owns a
  disjoint file set; shared files take only small, compile-safe edits, and
  the lead integrates anything that crosses strands afterwards. Subagents
  verify only their own slice; the area gates run once, by the lead.
- **`git mv` for `schemas/memory.yaml`:** the plan asks for `git mv`, but this
  session must not stage files, so the file is moved with a plain `mv`; Git
  detects the rename when the move is committed.

### Wave 9 — Compose host (done)

- **Library:** new `darkmatter/lib/src/lifecycle/md_host.rs` (+ `md_host/tests.rs`,
  24 tests). It holds the md profile (`registry()` = `EventRegistry::md()`,
  parse options), the shell-audit bindings (`err`, `timing`, `current` are
  refused at preflight), the lifecycle shell sources, `MdStackHost`
  (`LifecycleHost`), `MdShellRunner` (it checks shell permission, then exact
  approval per chain segment, then both blacklists, under the md timeout),
  `MdDiagnostics`, and `drive`, the request/reply loop over `Run`. Nothing in
  it runs unless `drive` is called, so the library compose API stays passive.
- **Tolerant parsing:** new `UnknownVerbPolicy::IgnoreUnprovided(UNPROVIDED_VERBS)`.
  Another host's verb (`say`, `retry`, `notify`, …) or an unknown verb becomes
  an inert `Ignored` node: its operand is never parsed, evaluated, or scanned
  for shells. The executor warns once per occurrence it reaches. Claudine
  keeps `Reject`. After parsing, each action's type is checked against what
  the registry allows for its event, and `initialize` excludes `shell` under
  every host, whatever the host declares.
- **Names through the registry:** `validate.rs` and `source_map.rs` gained
  registry-aware `*_in` variants, and executor diagnostics take the event name
  from the registry. Lead integration: `DiagnosticCatalog::event_name`
  (it defaults to Claudine's spelling) now names the event in a bare
  `error`'s default text, so `md` says `lifecycle composed error`, not
  `lifecycle start error` (test
  `compose_lifecycle::a_bare_error_in_composed_names_the_md_event`).
- **CLI:** new `darkmatter/cli/src/commands/compose/lifecycle.rs`. It writes
  terminal output and installs the Ctrl-C handler (the first press cancels,
  the second exits). It ignores `SIGPIPE`, so a broken stdout pipe becomes a
  write error. It also runs the shell-free bootstrap projection and the
  prepare step: references, stamping, initialize mutations passed as data
  overrides, batched approval (lifecycle shells included), then composition.
  It reuses the one `ComposeRequest`. `compose.rs` was split into helpers.
  Only a root document that authors a core event key takes the lifecycle
  path; every other document runs as before. `--shell` report mode also lists
  lifecycle commands. New dependency `ctrlc = "3.4"`, recorded in
  `darkmatter/docs/dependencies.md`.
- **Lead integration:** a lifecycle run refused for unresolved references
  exits 2, the same as the non-lifecycle path (the agent's first version
  exited 1). It exits 1 instead when a cleanup error has replaced that
  refusal as the selected error.
- **Tests:** `darkmatter/cli/tests/l1/compose_lifecycle.rs`, 27 tests, each
  under 0.3 s (mapping below).
- **Departures, recorded for review:**
  1. Under `Run`'s routing an `initialize` error goes straight to `finalize`
     (Claudine skips `blocked` there). To get the spec's md behavior
     (`failed` then `finalize`), the md driver reports an initialize error as
     the preparation's failure. `Run` itself is unchanged.
  2. `md compose` has no `--dry-run` flag, so "dry runs fire nothing" is
     proven only for report-only `--shell`. No flag was added.
  3. `check_emission` is not called. `md` registers no host events, so the
     check could never fail.
  4. Lifecycle shell output is captured and then written to stdout/stderr in
     run order; it is not inherited from the terminal.
  5. Composition cannot be stopped midway: an interrupt during composition
     drops the result once it returns (the reply is refused as outdated). A
     lifecycle command that is already starting when Ctrl-C lands runs to
     completion; a second Ctrl-C force-exits.
  6. A lifecycle run does not print `--perf` or the verbose compose summary.
  7. Lifecycle shells are stamped against the bootstrap frontmatter plus the
     initialize mutations, so a frontmatter `$( … )` value is still
     unexpanded there.

### Wave 9 — Global catalog (done)

- `darkmatter/schemas/darkmatter.yaml` declares `fm`, `ctx`, `current`, `env`,
  `err`, and `timing` with the spec §9 text. `tracking` became `timing`: the
  new `partials/timing.yaml` declares `document_ms`, `total_ms`, and
  `step_ms` as optional integers, and `partials/tracking.yaml` was deleted.
  `error.yaml` declares the fields `LifecycleErrorInfo::to_value` produces.
- **Baseline copied first:** the running baseline's definitions were copied
  into `fm.yaml`, `style.yaml`, and `context-variables.yaml` before the
  switch. A dump taken before and after shows the baseline JSON
  byte-identical and the `ctx.*` descriptors identical. `_cache` was dropped
  from `fm.yaml`: the running baseline never had it, and keeping it would
  have changed validation.
- **Embedding:** `embedded.rs::SHIPPED` holds the catalog, its partials, and
  the Content Policy schema under repo-relative virtual paths. The baseline
  is the catalog's resolved `fm`. `darkmatter/docs/schemas/darkmatter.yaml`
  was deleted, and every reference to it was repointed.
- **Loader:** new `darkmatter/lib/src/markdown/schemas/catalog.rs`
  (`GlobalCatalog::{darkmatter, from_documents, from_file, from_host_file}`,
  `host_catalogs`, `is_global_catalog`). Errors are field-located
  `CatalogError { field, reason }`. The loader is strict: duplicate keys and
  trailing documents fail while the source bytes are parsed.
- **Never a document schema:** `standalone.rs` refuses a global catalog
  (`kind: schema` with a `$schema` mapping and no `types`), by content, on
  every path that reaches it. An explicit registration (`BASELINE_SCHEMA` or
  `with_baseline_from_file` pointed at a catalog) uses the catalog's `fm`.
- **Readers fed from the catalog:** the baseline, the `ctx.*` descriptors
  (`md schema about` and DMLS `ctx` hover), `md schema validate`, and DMLS
  host-global hover and completion.
- **Host catalog:** new `claudine/schemas/globals.yaml` declares `seq`,
  `loop`, and `group`. It is embedded by
  `claudine/lib/src/composition/schema/globals.rs`, and Claudine's
  `bindings.rs` and `registry.rs` take the global names from it. The
  `BindingView` descriptions stay in code, because an inline mapping in the
  catalog cannot carry a top-level description.
- **Lead integration:** registry unit test
  `every_generic_late_global_is_declared_by_the_global_catalog` ties
  `GENERIC_GLOBALS` (`err`, `timing`) to the catalog. The descriptions were
  not switched, so no rendered string changes.
- **Changed assertions:** these follow the plan's final matrix ruling or the
  removal of the old file, and none is a pre-existing Claudine assertion.
  - The Content Policy `$schema` mutation column now edits the catalog, and
    "empty mapping" now expects a load failure.
  - `meta_schema_phase1`: the file count went from 5 to 4.
  - `meta_schema_phase6`: the envelope is "Tagged", since it now reads
    `style.yaml`.
  - The DMLS phase-7 hover test now expects "Declares: **object**".
  - The corpus inline-body minimum went from 10 to 9.
- **Not done (Phase 5, Editor projection):** DMLS does not yet show per-event
  availability for `seq`/`loop`/`group`; it uses the baseline `BindingView`.
  DMLS also silently drops a host catalog that fails to load.

### Wave 9 — Trigger migration (done)

- **Parity captured first:** `darkmatter/lib/tests/l1/trigger_selection_parity.rs`
  (the fixture triggers against 4 fixture documents plus 64 synthetic ones)
  and `claudine/lib/tests/l1/claudine_trigger_selection_parity.rs` (the
  shipped bundle against 115 documents). Both expected tables were recorded
  from the unmigrated code, and both pass unchanged after migration.
- **Kind:** `kind: schema-trigger` loads. `kind: trigger-schema` is the typed
  `SchemaError::TriggerRenamed` error, even when the file is not valid YAML.
  A null or non-string `kind` is now an error. An absent `kind`, or any other
  string including `""`, is still ordinary data.
- **Grammar:** `MatchConditions` ANDs a top-level list. `MatchExpr::Group`
  (`group:`) is OR, and `any:` is a `TriggerRenamed` error naming `group:`.
  An anonymous nested list is rejected at its location. Vacuity is checked
  once over the whole expression (`TriggerVacuousMatch`). Grammar errors name
  their field (`match[4].group[1]`).
- **Matcher-only triggers:** a trigger with no `$schema` now loads and layers
  nothing, where it used to be a load error. The plan's matrix ruling
  ("Absent → matcher-only trigger, no payload") requires this, and the
  existing expectations in `assemble.rs`,
  `schema_embedded_trigger_bundles.rs`, `claudine_schemas_matrix.rs`, and a
  DMLS logging test were updated to match.
- **Flag:** `--no-schema-triggers` on `md compose`, `md schema validate`, and
  `md clean`. The old flag is rejected before clap parses arguments
  (`reject_renamed_flags` in `darkmatter/cli/src/main.rs`): it is a usage
  error, exit 2, that names the new flag. A hidden clap argument was tried
  first and rejected, because fish completion still offered it. Library
  names (`ComposeOptions::with_trigger_schemas`, the DMLS
  `trigger_schemas=` log field) keep the "trigger schemas" concept name.
- **Migration:** three triggers in `claudine/schemas/` and three fixtures
  under `darkmatter/tests/fixtures/schema-triggers/schemas/` (kind rename,
  and `any:` → `group:` where it was used). No top-level OR list existed, so
  none needed rewriting. `schemas/memory.yaml` moved to
  `docs/drafts/memory.yaml`, which no schema root reaches. Active docs and
  skills were updated for spelling.
- **Guards:** `trigger_spelling_guard.rs` checks that no active file uses an
  old spelling. Its allowlist is the rejecting code, the negative tests,
  `definition.md`, and the darkmatter `schema.md` skill, and the guard fails
  on a stale entry.
- **Other new tests:** `trigger_match_grammar.rs` (lib),
  `cli/tests/l1/schema_trigger_rename.rs` (flags, completions, field
  locations, nothing composed), and a DMLS test in
  `workspace_schema_scopes.rs` (an old-kind trigger gives one failed scope
  and a diagnostic naming `schema-trigger`).
- **Known limit:** in the older JSON-flow matrix, an unparseable flow mapping
  is still not claimed as a trigger. This is the documented claim rule: a
  broken file claims the envelope only through a column-0 `kind:` line.

### Input robustness matrices (Matrix evidence, done)

- **Global catalog:** 8 columns × 9 rows = 72 cells.
- **Host catalog:** 5 columns × 9 rows = 45 cells (`kind`, `$schema`, `seq`,
  `loop`, `group`).
- **Trigger envelope and conditions:** all 99 cells of the plan's table, plus
  13 extras: the old kind (parseable and not), `any:`, an anonymous nested
  list, `kind: []`, `match: {}`, a payload reference list, `$path: []`, and
  `min-match` counts of 0, above the candidate count, and −1.
- **How each cell is built:** one generated `#[test]` (`macro_rules!`),
  starting from an unedited shipped or fixture file with one edit. Each
  matrix has a guard test that compares its generated list with the plan's
  table. Results are read only through public results (resolved globals and
  baseline, hover, the trigger scan, the effective schema). Duplicate-key
  and trailing-YAML cells are edits to the source bytes. Every outcome
  matches the plan's table; N/A cells substitute a collection and fail.
- **Code-smell review:** `serde(default)`, null-collapsing `Option`,
  `filter_map(as_str)`, `unwrap_or_default()`, and `.ok()` were checked on
  these fields. Two load-bearing defects were fixed: a null or non-string
  trigger `kind` was read as "not a trigger", and `$schema: null` passed
  through as a payload. The catalog fields had none.

### Behavior → test mapping (Phase 4)

| Behavior (criterion) | Tests |
|---|---|
| md success order; `failed`→`finalize` on shell denial and missing include; `skip`; `stdout` order; `say`/`retry`/unknown verb warned once; unselected branch silent; host-only keys as data (11) | `darkmatter-cli::l1 compose_lifecycle::*` (`a_successful_run_fires_…`, `a_shell_audit_denial_…`, `a_missing_include_…`, `skip_writes_no_document_…`, `stdout_actions_frame_the_document_…`, `say_/retry_/an_unknown_verb_is_skipped_…`, `an_unselected_branch_warns_nothing`, `host_only_keys_are_plain_frontmatter`) |
| Interrupt: `cancelled`+`finalize` / `finalize` alone, 130; cancellation during initialize/prepare/composed/write; late replies refused (11, 21) | `lifecycle::md_host::tests::*` interrupt cases (`late_replies == 1`); `compose_lifecycle::ctrl_c_runs_cancelled_then_finalize_and_exits_130` (Unix, real signal) |
| Report-only `--shell` runs nothing (11, 22) | `compose_lifecycle::report_only_shell_mode_lists_lifecycle_commands_and_runs_nothing` |
| Unreadable root, malformed lifecycle, malformed/forbidden supported action fail before effects (21, 22) | `compose_lifecycle::{an_unreadable_root_…, malformed_lifecycle_yaml_shape_…, a_malformed_provided_action_…, shell_in_initialize_…, skip_in_composed_…}` |
| Initialize creates an include; initialize mutation reaches composition; child stacks never fire; `stop` keeps output and `finalize` (21) | `compose_lifecycle::{initialize_can_create_the_file_…, an_initialize_set_reaches_…, a_childs_lifecycle_keys_never_fire, stop_ends_only_its_stack_…}` |
| `composed` error and closed stdout → `finalize`, nonzero, no `failed`; cleanup error not recursive (21) | `compose_lifecycle::{a_composed_error_runs_finalize_…, a_closed_stdout_runs_finalize_…, an_error_in_failed_replaces_the_error_…}`; md_host write-failure test |
| Ignored action's operand/payload inert; lifecycle-only shell audited; shells unlock at `composed` (8, 22) | `compose_lifecycle::{an_ignored_actions_operand_and_payload_cause_no_effect, an_approved_lifecycle_shell_runs_once_…}`; `md_host::tests` shell-lock cases |
| md event names in diagnostics | `compose_lifecycle::a_bare_error_in_composed_names_the_md_event` |
| Global catalog (14) | `darkmatter::l1 global_catalog::*` (transitive imports, embedded = disk parity, baseline is `fm`, ctx = current, timing/err fields = runtime, `fm.ctx` independent, never auto-applies via any path, unavailable `err` reason, old file gone); registry `every_generic_late_global_is_declared_by_the_global_catalog` |
| Host globals (15) | `claudine::l1 host_catalog::*`; `dmls::l1 host_globals::*` (hover/completion from the file). Per-event DMLS availability: Phase 5 |
| Trigger kind / flag / AND-OR / parity (16–18) | `darkmatter::l1 {trigger_selection_parity, trigger_match_grammar, trigger_input_matrix, trigger_spelling_guard}`; `claudine::l1 claudine_trigger_selection_parity`; `darkmatter-cli::l1 schema_trigger_rename::*`; `dmls::l1 workspace_schema_scopes` old-kind case |
| Catalog / host catalog / trigger matrices | `global_catalog` and `host_catalog` matrix cells and guards; `trigger_input_matrix` (99 + 13) |

### Phase 4 checkpoint and gates

All gates ran once, sequentially, after every subagent had finished.

- **`cd darkmatter && just test`:** first run 11,089 passed, 1 failed:
  `dmls searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`.
  This is the load-sensitive test recorded in Phase 3; this phase changed one
  line in its file. Alone it passed 3/3. Because the first run stopped early,
  the suite was rerun in full: **11,211/11,211 passed** (10 skipped).
  **`just lint`:** pass (rerun after the last test edit: pass).
  **`just check-tier-coverage darkmatter`:** 0 stranded.
- **`cd claudine && just test`:** first run failed early on
  `composition::lifecycle::prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done`.
  This phase did not touch it, and alone it passed 3/3, so it is recorded
  as load-sensitive. Full rerun: **10,685/10,685 passed** (166 slow,
  9 skipped), 425 s. **`just lint`:** pass.
  **`just check-tier-coverage claudine`:** 0 stranded.
- **Dependents:** `cargo check --all-targets` of every direct dependent of
  `darkmatter`, `claudine`, `darkmatter-cli`, or `dmls` (`biscuit-icon-cli`,
  `biscuit-speaks-cli`, `messenger`, `claudine-gen`, `playa-cli`,
  `messenger-cli`, `sniff-cli`, `research`, `claudine-contract`): clean.
  `cargo nextest run -p claudine-contract`: 48/48.
- **Cross-OS (native Windows, `build-win-native`):** this phase adds a Ctrl-C
  handler, `SIGPIPE` handling, and lifecycle shells to `md compose`, so the
  risk is notable.
  - `darkmatter-cli` attempt 1: 1179/1180. The failure was
    `schema_trigger_rename::compose_reports_an_invalid_trigger_field_by_location_and_composes_nothing`:
    the error box wraps the long Windows temp path and splits
    `trigger-schema` across two rows.
  - Attempt 2 joined the rows and failed the same way. The rows also begin
    with SGR styling, which `md` keeps under `NO_COLOR=1` and the
    cross-check log strips.
  - Attempt 3 strips escape sequences before joining the rows:
    **1180/1180**. This trap is now recorded in the `os` skill
    (`windows-paths.md`, item 20).
  - `darkmatter`, filtered to `catalog trigger md_host lifecycle baseline schema`:
    **2444/2444**.
  - `dmls`: **1278/1278**.
  - `claudine`, filtered to `host_catalog trigger schema binding registry`:
    **474/474**.
  - The spec's `os_failing: windows` stays, for the 9 pre-existing
    `claudine-cli` Windows failures recorded in Phases 2 and 3.

#### Checkpoint criteria

| Criterion | Status |
|---|---|
| 11 (md compose events) | covered by `compose_lifecycle` and `md_host` tests; `md compose` has no `--dry-run`, so "dry runs fire nothing" is proven only for report-only `--shell` (departure 2 above) |
| 12 (passive surfaces) | `drive` is the only entry point that runs stacks; `md read`/`schema validate`/DMLS never call it. Recording-fake proof for every passive surface is Phase 5 ("Editor projection") |
| 14 (global catalog) | done |
| 15 (host globals) | catalog, hover, and completion done; per-event DMLS availability is Phase 5 |
| 16, 17, 18 (triggers) | done; selection parity unchanged for every migrated file |
| 21, 22 (md parts) | done (see the mapping) |
| Baseline validation assertions | unchanged, except for the five matrix and file-count assertions listed under Global catalog, each forced by the plan's matrix ruling or by deleting the old file |

#### CI scope

The gates were run, not skipped: this phase changed source in `darkmatter`,
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

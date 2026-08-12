---
total_phases: 12
created: 2026-07-14
source_review: review.md
source_files_during_phase_1:
  - cli/tests/wrap_perf.rs
  - gen/Cargo.toml
  - gen/tests/drift.rs
  - gen/tests/generate_ux.rs
  - lib/src/composition/error/tests.rs
  - lib/src/composition/lifecycle/runtime.rs
docs_updated_during_phase_1:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_1:
  - reviews/2026-07-14-module-assessment/generated-artifact-baseline.json
  - reviews/2026-07-14-module-assessment/phase-1-baseline.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - cli/src/commands/wrap/composition/preflight.rs
  - cli/src/commands/wrap/harness_orch/loop_control.rs
  - cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
  - lib/src/composition/lifecycle/runtime.rs
  - lib/src/composition/mod.rs
docs_updated_during_phase_2:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_3:
  - cli/src/commands/wrap/composition/mod.rs
  - cli/src/commands/wrap/composition/pipeline.rs
  - cli/src/commands/wrap/composition/runner.rs
  - cli/tests/dispatch_inventory.rs
docs_updated_during_phase_3:
  - docs/providers/dispatch-inventory.json
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_4:
  - cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_4:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_5:
  - cli/tests/test_placement.rs
docs_updated_during_phase_5:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_6:
  - cli/src/commands/compose/prep.rs
  - cli/src/commands/compose/prep/tests.rs
  - cli/src/commands/context_render.rs
  - cli/src/commands/context_render/tests.rs
  - cli/src/commands/schema_interactive/mod.rs
  - cli/src/commands/schema_interactive/tests.rs
  - cli/src/commands/wrap/composition/dry_run.rs
  - cli/src/commands/wrap/composition/dry_run/tests.rs
  - cli/src/commands/wrap/composition/pipeline.rs
  - cli/src/commands/wrap/composition/pipeline/tests.rs
  - cli/src/commands/wrap/exec/spawn.rs
  - cli/src/commands/wrap/exec/spawn/tests.rs
  - cli/src/commands/wrap/exec/termination.rs
  - cli/src/commands/wrap/exec/termination/tests.rs
  - cli/src/commands/wrap/exec/timeouts.rs
  - cli/src/commands/wrap/exec/timeouts/tests.rs
  - cli/src/commands/wrap/flags.rs
  - cli/src/commands/wrap/flags/tests.rs
  - cli/src/commands/wrap/repo_home.rs
  - cli/src/commands/wrap/repo_home/tests.rs
  - cli/src/commands/wrap/session_report.rs
  - cli/src/commands/wrap/session_report/tests.rs
  - cli/src/commands/wrap/system_prompt.rs
  - cli/src/commands/wrap/system_prompt/tests.rs
  - cli/src/completion/bootstrap.rs
  - cli/src/completion/bootstrap/tests.rs
  - cli/src/completion/composition/mod.rs
  - cli/src/completion/composition/tests.rs
  - cli/src/completion/engine/mod.rs
  - cli/src/completion/engine/tests.rs
  - cli/src/completion/scopes.rs
  - cli/src/completion/scopes/tests.rs
  - cli/src/completion/setter_value.rs
  - cli/src/completion/setter_value/tests.rs
  - cli/src/completion/walker.rs
  - cli/src/completion/walker/tests.rs
  - cli/src/output/error_walker.rs
  - cli/src/output/error_walker/tests.rs
  - cli/tests/test_placement.rs
  - contract/src/tests.rs
  - contract/src/tests/tracing_capture.rs
  - gen/src/agent_errors_check.rs
  - gen/src/agent_errors_check/tests.rs
  - gen/src/emit.rs
  - gen/src/emit/tests.rs
  - gen/src/generate.rs
  - gen/src/generate/tests.rs
  - gen/src/registry.rs
  - gen/src/registry/tests.rs
  - gen/src/vocabulary.rs
  - gen/src/vocabulary/tests.rs
  - lib/src/actions/hook_action.rs
  - lib/src/actions/hook_action/tests.rs
  - lib/src/composition/closure.rs
  - lib/src/composition/closure/tests.rs
  - lib/src/composition/lifecycle/actions.rs
  - lib/src/composition/lifecycle/actions/tests.rs
  - lib/src/composition/lifecycle/context.rs
  - lib/src/composition/lifecycle/context/tests.rs
  - lib/src/composition/lifecycle/control.rs
  - lib/src/composition/lifecycle/control/tests.rs
  - lib/src/composition/lifecycle/runtime.rs
  - lib/src/composition/lifecycle/runtime/tests.rs
  - lib/src/composition/looping/actions.rs
  - lib/src/composition/looping/actions/tests.rs
  - lib/src/composition/looping/config.rs
  - lib/src/composition/looping/config/tests.rs
  - lib/src/composition/preflight.rs
  - lib/src/composition/preflight/tests.rs
  - lib/src/composition/prepare.rs
  - lib/src/composition/prepare/tests.rs
  - lib/src/composition/resolve.rs
  - lib/src/composition/resolve/tests.rs
  - lib/src/composition/select.rs
  - lib/src/composition/select/tests.rs
  - lib/src/composition/sequence.rs
  - lib/src/composition/sequence/tests.rs
  - lib/src/composition/types.rs
  - lib/src/composition/types/tests.rs
  - lib/src/config/claude.rs
  - lib/src/config/claude/tests.rs
  - lib/src/config/claudine_config.rs
  - lib/src/config/claudine_config/tests.rs
  - lib/src/config/messaging_block.rs
  - lib/src/config/messaging_block/tests.rs
  - lib/src/dispatch/expression.rs
  - lib/src/dispatch/expression/tests.rs
  - lib/src/dispatch/loader.rs
  - lib/src/dispatch/loader/tests.rs
  - lib/src/dispatch/matcher.rs
  - lib/src/dispatch/matcher/tests.rs
  - lib/src/dispatch/mod.rs
  - lib/src/dispatch/runner/mod.rs
  - lib/src/dispatch/runner/tests.rs
  - lib/src/dispatch/template.rs
  - lib/src/dispatch/template/tests.rs
  - lib/src/dispatch/tests.rs
  - lib/src/harness/runtime.rs
  - lib/src/harness/runtime/tests.rs
  - lib/src/linking/compatibility/mod.rs
  - lib/src/linking/compatibility/tests.rs
  - lib/src/linking/skills/portable.rs
  - lib/src/linking/skills/portable/tests.rs
  - lib/src/mcp/import.rs
  - lib/src/mcp/import/tests.rs
  - lib/src/messaging/config.rs
  - lib/src/messaging/config/tests.rs
  - lib/src/messaging/send.rs
  - lib/src/messaging/send/tests.rs
  - lib/src/model_catalog/service.rs
  - lib/src/model_catalog/service/tests.rs
  - lib/src/permissions/engine.rs
  - lib/src/permissions/engine/tests.rs
  - lib/src/permissions/providers/claude.rs
  - lib/src/permissions/providers/claude/tests.rs
  - lib/src/permissions/providers/codex.rs
  - lib/src/permissions/providers/codex/tests.rs
  - lib/src/permissions/providers/gemini.rs
  - lib/src/permissions/providers/gemini/tests.rs
  - lib/src/permissions/providers/qwen.rs
  - lib/src/permissions/providers/qwen/tests.rs
  - lib/src/permissions/query.rs
  - lib/src/permissions/query/tests.rs
  - lib/src/protect/catalog.rs
  - lib/src/protect/catalog/tests.rs
  - lib/src/protect/observe.rs
  - lib/src/protect/observe/tests.rs
  - lib/src/protect/service.rs
  - lib/src/protect/service/tests.rs
  - lib/src/provider/methods.rs
  - lib/src/provider/methods/tests.rs
  - lib/src/render/prompt/system.rs
  - lib/src/render/prompt/system/tests.rs
  - lib/src/reporting/ingest.rs
  - lib/src/reporting/ingest/tests.rs
  - lib/src/runaway/config.rs
  - lib/src/runaway/config/tests.rs
  - lib/src/runaway/detector.rs
  - lib/src/runaway/detector/tests.rs
  - lib/src/signals/bespoke.rs
  - lib/src/signals/bespoke/tests.rs
  - lib/src/stream/badges.rs
  - lib/src/stream/badges/tests.rs
  - lib/src/stream/logs/opencode/events.rs
  - lib/src/stream/logs/opencode/events/tests.rs
  - lib/src/stream/progress.rs
  - lib/src/stream/progress/tests.rs
  - lib/src/stream/protocol/claude.rs
  - lib/src/stream/protocol/claude/tests.rs
  - lib/src/stream/protocol/codex.rs
  - lib/src/stream/protocol/codex/tests.rs
  - lib/src/stream/protocol/kimi.rs
  - lib/src/stream/protocol/kimi/tests.rs
  - lib/src/stream/protocol/opencode.rs
  - lib/src/stream/protocol/opencode/tests.rs
  - lib/src/stream/providers/claude.rs
  - lib/src/stream/providers/claude/tests.rs
  - lib/src/stream/providers/codex.rs
  - lib/src/stream/providers/codex/tests.rs
  - lib/src/stream/providers/gemini.rs
  - lib/src/stream/providers/gemini/tests.rs
  - lib/src/stream/providers/kimi.rs
  - lib/src/stream/providers/kimi/tests.rs
  - lib/src/stream/providers/opencode.rs
  - lib/src/stream/providers/opencode/tests.rs
  - lib/src/stream/providers/pi.rs
  - lib/src/stream/providers/pi/tests.rs
  - lib/src/stream/reporting.rs
  - lib/src/stream/reporting/tests.rs
  - lib/src/stream/semantic.rs
  - lib/src/stream/semantic/tests.rs
  - lib/src/stream/stderr.rs
  - lib/src/stream/stderr/tests.rs
  - lib/src/stream/tool_display.rs
  - lib/src/stream/tool_display/from_event_tests.rs
  - lib/src/stream/tool_display/humanize_tests.rs
  - lib/src/stream/tool_display/summary_tests.rs
  - lib/src/stream/tool_display/tests.rs
  - lib/src/system_prompt/prepare.rs
  - lib/src/system_prompt/prepare/tests.rs
  - lib/src/system_prompt/resolve.rs
  - lib/src/system_prompt/resolve/tests.rs
  - rendezvous/daemon/src/peers.rs
  - rendezvous/daemon/src/peers/tests.rs
  - rendezvous/daemon/src/register.rs
  - rendezvous/daemon/src/register/tests.rs
  - rendezvous/daemon/src/service.rs
  - rendezvous/daemon/src/service/tests.rs
  - rendezvous/daemon/src/sync.rs
  - rendezvous/daemon/src/sync/tests.rs
docs_updated_during_phase_6:
  - docs/providers/dispatch-inventory.json
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_7:
  - cli/src/commands/wrap/harness_orch/loop_control/tests.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
  - lib/src/composition/lifecycle/executor/tests.rs
  - lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
  - lib/src/composition/lifecycle/executor/tests/conditions_control.rs
  - lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
  - lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - lib/src/composition/lifecycle/executor/tests/mod.rs
  - lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
  - lib/src/composition/lifecycle/tests.rs
  - lib/src/composition/lifecycle/tests/action_shape_control.rs
  - lib/src/composition/lifecycle/tests/audio_emission.rs
  - lib/src/composition/lifecycle/tests/diagnostics.rs
  - lib/src/composition/lifecycle/tests/guard_runtime.rs
  - lib/src/composition/lifecycle/tests/mod.rs
  - lib/src/composition/lifecycle/tests/parse_config.rs
  - lib/src/composition/lifecycle/tests/validation.rs
  - lib/src/composition/looping/engine/tests.rs
  - lib/src/composition/looping/engine/tests/iteration_actions.rs
  - lib/src/composition/looping/engine/tests/lifecycle_control.rs
  - lib/src/composition/looping/engine/tests/mod.rs
  - lib/src/composition/looping/engine/tests/rate_limits.rs
  - lib/src/composition/looping/engine/tests/seed_state.rs
  - lib/src/stream/logs/opencode/bridge/tests.rs
  - lib/src/stream/logs/opencode/bridge/tests/ingest_classification.rs
  - lib/src/stream/logs/opencode/bridge/tests/mod.rs
  - lib/src/stream/logs/opencode/bridge/tests/session_lifecycle.rs
  - lib/src/stream/logs/opencode/bridge/tests/signal_projection.rs
  - lib/src/stream/logs/opencode/bridge/tests/stalled_generation_progress.rs
  - lib/src/stream/logs/opencode/bridge/tests/stdout_stderr_coordination.rs
  - lib/src/stream/logs/opencode/bridge/tests/usage_retry_guards.rs
  - rendezvous/daemon/src/session_log/tests.rs
  - rendezvous/daemon/src/session_log/tests/append_rotation.rs
  - rendezvous/daemon/src/session_log/tests/durability.rs
  - rendezvous/daemon/src/session_log/tests/mod.rs
  - rendezvous/daemon/src/session_log/tests/remote_validation.rs
  - rendezvous/daemon/src/session_log/tests/replace_update.rs
  - rendezvous/daemon/src/session_log/tests/replay_rehydration.rs
docs_updated_during_phase_7:
  - docs/providers/dispatch-inventory.json
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
  - rendezvous/daemon/src/session_log/mod.rs
  - rendezvous/daemon/src/session_log/append.rs
  - rendezvous/daemon/src/session_log/staging.rs
  - rendezvous/daemon/src/session_log/rehydrate.rs
  - rendezvous/daemon/src/session_log/validate.rs
  - rendezvous/daemon/src/sync/tests/mod.rs
  - rendezvous/daemon/src/sync/tests/envelope_validation.rs
  - rendezvous/daemon/src/sync/tests/schema_validation.rs
  - rendezvous/daemon/src/sync/tests/snapshot_replace.rs
  - rendezvous/daemon/src/service/tests/mod.rs
  - rendezvous/daemon/src/service/tests/rpc.rs
  - rendezvous/daemon/src/service/tests/session_register.rs
  - rendezvous/daemon/src/service/tests/validation.rs
docs_updated_during_phase_8:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
source_files_during_phase_9:
  - cli/src/commands/wrap/exec/spawn/mod.rs
  - cli/src/commands/wrap/exec/spawn/setup.rs
  - cli/src/commands/wrap/exec/spawn/inherited.rs
  - cli/src/commands/wrap/exec/spawn/captured.rs
  - cli/src/commands/wrap/exec/spawn/semantic.rs
  - cli/src/commands/wrap/exec/spawn/tests/mod.rs
  - cli/src/commands/wrap/exec/spawn/tests/inherited.rs
  - cli/src/commands/wrap/exec/spawn/tests/captured.rs
  - cli/src/commands/wrap/exec/termination/mod.rs
  - cli/src/commands/wrap/exec/termination/reasons.rs
  - cli/src/commands/wrap/exec/termination/summary.rs
  - cli/src/commands/wrap/exec/termination/message.rs
  - cli/src/commands/wrap/exec/termination/unix.rs
  - cli/src/commands/wrap/exec/termination/windows.rs
  - cli/src/commands/wrap/exec/termination/tests/mod.rs
  - cli/src/commands/wrap/exec/termination/tests/projection.rs
  - cli/src/commands/wrap/exec/termination/tests/reasons.rs
  - cli/src/commands/wrap/exec/termination/tests/wait.rs
docs_updated_during_phase_9:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9: []
packages_during_phase_9:
  - claudine-cli
source_files_during_phase_10:
  - gen/Cargo.toml
  - gen/src/lib.rs
  - gen/src/main.rs
  - gen/src/report.rs
  - gen/src/report/tests.rs
  - gen/src/emit/mod.rs
  - gen/src/emit/identity_paths.rs
  - gen/src/emit/execution_prompting.rs
  - gen/src/emit/models_offerings.rs
  - gen/src/emit/event_policy.rs
  - gen/src/emit/linking.rs
  - gen/src/generate.rs
  - gen/src/generate/tests.rs
  - gen/src/generate/coerce/mod.rs
  - gen/src/generate/coerce/identity_paths.rs
  - gen/src/generate/coerce/execution_prompting.rs
  - gen/src/generate/coerce/models_offerings.rs
  - gen/src/generate/coerce/event_policy.rs
  - gen/tests/generate_ux.rs
  - lib/src/provider/mod.rs
docs_updated_during_phase_10:
  - docs/topics/provider-metadata.md
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_10: []
skills_files_updated_during_phase_10: []
packages_during_phase_10:
  - claudine
  - claudine-gen
source_files_during_phase_11:
  - lib/src/composition/error/render/mod.rs
  - lib/src/composition/error/render/lifecycle.rs
  - lib/src/composition/error/render/schema.rs
  - lib/src/composition/error/render/selection.rs
  - lib/src/composition/error/render/sequence_loop.rs
  - lib/src/composition/error/render/provider.rs
  - lib/src/composition/error/tests.rs
docs_updated_during_phase_11:
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_11: []
skills_files_updated_during_phase_11: []
packages_during_phase_11:
  - claudine
source_files_during_phase_12:
  - lib/src/composition/mod.rs
  - lib/src/composition/interpolation_conformance.rs
  - lib/src/composition/looping/actions.rs
docs_updated_during_phase_12:
  - docs/topics/composition.md
  - docs/topics/flow-control/looping.md
  - reviews/2026-07-14-module-assessment/plan.md
docs_created_during_phase_12: []
skills_files_updated_during_phase_12: []
packages_during_phase_12:
  - claudine
source_code:
  - cli/src/commands/compose/prep.rs
  - cli/src/commands/compose/prep/tests.rs
  - cli/src/commands/context_render.rs
  - cli/src/commands/context_render/tests.rs
  - cli/src/commands/schema_interactive/mod.rs
  - cli/src/commands/schema_interactive/tests.rs
  - cli/src/commands/wrap/composition/dry_run.rs
  - cli/src/commands/wrap/composition/dry_run/tests.rs
  - cli/src/commands/wrap/composition/mod.rs
  - cli/src/commands/wrap/composition/pipeline.rs
  - cli/src/commands/wrap/composition/pipeline/tests.rs
  - cli/src/commands/wrap/composition/preflight.rs
  - cli/src/commands/wrap/composition/runner.rs
  - cli/src/commands/wrap/exec/spawn.rs
  - cli/src/commands/wrap/exec/spawn/captured.rs
  - cli/src/commands/wrap/exec/spawn/inherited.rs
  - cli/src/commands/wrap/exec/spawn/mod.rs
  - cli/src/commands/wrap/exec/spawn/semantic.rs
  - cli/src/commands/wrap/exec/spawn/setup.rs
  - cli/src/commands/wrap/exec/spawn/tests.rs
  - cli/src/commands/wrap/exec/spawn/tests/captured.rs
  - cli/src/commands/wrap/exec/spawn/tests/inherited.rs
  - cli/src/commands/wrap/exec/spawn/tests/mod.rs
  - cli/src/commands/wrap/exec/termination.rs
  - cli/src/commands/wrap/exec/termination/message.rs
  - cli/src/commands/wrap/exec/termination/mod.rs
  - cli/src/commands/wrap/exec/termination/reasons.rs
  - cli/src/commands/wrap/exec/termination/summary.rs
  - cli/src/commands/wrap/exec/termination/tests.rs
  - cli/src/commands/wrap/exec/termination/tests/mod.rs
  - cli/src/commands/wrap/exec/termination/tests/projection.rs
  - cli/src/commands/wrap/exec/termination/tests/reasons.rs
  - cli/src/commands/wrap/exec/termination/tests/wait.rs
  - cli/src/commands/wrap/exec/termination/unix.rs
  - cli/src/commands/wrap/exec/termination/windows.rs
  - cli/src/commands/wrap/exec/timeouts.rs
  - cli/src/commands/wrap/exec/timeouts/tests.rs
  - cli/src/commands/wrap/flags.rs
  - cli/src/commands/wrap/flags/tests.rs
  - cli/src/commands/wrap/harness_orch/loop_control.rs
  - cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
  - cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
  - cli/src/commands/wrap/repo_home.rs
  - cli/src/commands/wrap/repo_home/tests.rs
  - cli/src/commands/wrap/session_report.rs
  - cli/src/commands/wrap/session_report/tests.rs
  - cli/src/commands/wrap/system_prompt.rs
  - cli/src/commands/wrap/system_prompt/tests.rs
  - cli/src/completion/bootstrap.rs
  - cli/src/completion/bootstrap/tests.rs
  - cli/src/completion/composition/mod.rs
  - cli/src/completion/composition/tests.rs
  - cli/src/completion/engine/mod.rs
  - cli/src/completion/engine/tests.rs
  - cli/src/completion/scopes.rs
  - cli/src/completion/scopes/tests.rs
  - cli/src/completion/setter_value.rs
  - cli/src/completion/setter_value/tests.rs
  - cli/src/completion/walker.rs
  - cli/src/completion/walker/tests.rs
  - cli/src/output/error_walker.rs
  - cli/src/output/error_walker/tests.rs
  - cli/tests/dispatch_inventory.rs
  - cli/tests/test_placement.rs
  - cli/tests/wrap_perf.rs
  - contract/src/tests.rs
  - contract/src/tests/tracing_capture.rs
  - gen/Cargo.toml
  - gen/src/agent_errors_check.rs
  - gen/src/agent_errors_check/tests.rs
  - gen/src/emit.rs
  - gen/src/emit/event_policy.rs
  - gen/src/emit/execution_prompting.rs
  - gen/src/emit/identity_paths.rs
  - gen/src/emit/linking.rs
  - gen/src/emit/mod.rs
  - gen/src/emit/models_offerings.rs
  - gen/src/emit/tests.rs
  - gen/src/generate.rs
  - gen/src/generate/coerce/event_policy.rs
  - gen/src/generate/coerce/execution_prompting.rs
  - gen/src/generate/coerce/identity_paths.rs
  - gen/src/generate/coerce/mod.rs
  - gen/src/generate/coerce/models_offerings.rs
  - gen/src/generate/tests.rs
  - gen/src/lib.rs
  - gen/src/main.rs
  - gen/src/registry.rs
  - gen/src/registry/tests.rs
  - gen/src/report.rs
  - gen/src/report/tests.rs
  - gen/src/vocabulary.rs
  - gen/src/vocabulary/tests.rs
  - gen/tests/drift.rs
  - gen/tests/generate_ux.rs
  - lib/src/actions/hook_action.rs
  - lib/src/actions/hook_action/tests.rs
  - lib/src/composition/closure.rs
  - lib/src/composition/closure/tests.rs
  - lib/src/composition/error/render/lifecycle.rs
  - lib/src/composition/error/render/mod.rs
  - lib/src/composition/error/render/provider.rs
  - lib/src/composition/error/render/schema.rs
  - lib/src/composition/error/render/selection.rs
  - lib/src/composition/error/render/sequence_loop.rs
  - lib/src/composition/error/tests.rs
  - lib/src/composition/interpolation_conformance.rs
  - lib/src/composition/lifecycle/actions.rs
  - lib/src/composition/lifecycle/actions/tests.rs
  - lib/src/composition/lifecycle/context.rs
  - lib/src/composition/lifecycle/context/tests.rs
  - lib/src/composition/lifecycle/control.rs
  - lib/src/composition/lifecycle/control/tests.rs
  - lib/src/composition/lifecycle/executor/tests.rs
  - lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
  - lib/src/composition/lifecycle/executor/tests/conditions_control.rs
  - lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
  - lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - lib/src/composition/lifecycle/executor/tests/mod.rs
  - lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
  - lib/src/composition/lifecycle/runtime.rs
  - lib/src/composition/lifecycle/runtime/tests.rs
  - lib/src/composition/lifecycle/tests.rs
  - lib/src/composition/lifecycle/tests/action_shape_control.rs
  - lib/src/composition/lifecycle/tests/audio_emission.rs
  - lib/src/composition/lifecycle/tests/diagnostics.rs
  - lib/src/composition/lifecycle/tests/guard_runtime.rs
  - lib/src/composition/lifecycle/tests/mod.rs
  - lib/src/composition/lifecycle/tests/parse_config.rs
  - lib/src/composition/lifecycle/tests/validation.rs
  - lib/src/composition/looping/actions.rs
  - lib/src/composition/looping/actions/tests.rs
  - lib/src/composition/looping/config.rs
  - lib/src/composition/looping/config/tests.rs
  - lib/src/composition/looping/engine/tests.rs
  - lib/src/composition/looping/engine/tests/iteration_actions.rs
  - lib/src/composition/looping/engine/tests/lifecycle_control.rs
  - lib/src/composition/looping/engine/tests/mod.rs
  - lib/src/composition/looping/engine/tests/rate_limits.rs
  - lib/src/composition/looping/engine/tests/seed_state.rs
  - lib/src/composition/mod.rs
  - lib/src/composition/preflight.rs
  - lib/src/composition/preflight/tests.rs
  - lib/src/composition/prepare.rs
  - lib/src/composition/prepare/tests.rs
  - lib/src/composition/resolve.rs
  - lib/src/composition/resolve/tests.rs
  - lib/src/composition/select.rs
  - lib/src/composition/select/tests.rs
  - lib/src/composition/sequence.rs
  - lib/src/composition/sequence/tests.rs
  - lib/src/composition/types.rs
  - lib/src/composition/types/tests.rs
  - lib/src/config/claude.rs
  - lib/src/config/claude/tests.rs
  - lib/src/config/claudine_config.rs
  - lib/src/config/claudine_config/tests.rs
  - lib/src/config/messaging_block.rs
  - lib/src/config/messaging_block/tests.rs
  - lib/src/dispatch/expression.rs
  - lib/src/dispatch/expression/tests.rs
  - lib/src/dispatch/loader.rs
  - lib/src/dispatch/loader/tests.rs
  - lib/src/dispatch/matcher.rs
  - lib/src/dispatch/matcher/tests.rs
  - lib/src/dispatch/mod.rs
  - lib/src/dispatch/runner/mod.rs
  - lib/src/dispatch/runner/tests.rs
  - lib/src/dispatch/template.rs
  - lib/src/dispatch/template/tests.rs
  - lib/src/dispatch/tests.rs
  - lib/src/harness/runtime.rs
  - lib/src/harness/runtime/tests.rs
  - lib/src/linking/compatibility/mod.rs
  - lib/src/linking/compatibility/tests.rs
  - lib/src/linking/skills/portable.rs
  - lib/src/linking/skills/portable/tests.rs
  - lib/src/mcp/import.rs
  - lib/src/mcp/import/tests.rs
  - lib/src/messaging/config.rs
  - lib/src/messaging/config/tests.rs
  - lib/src/messaging/send.rs
  - lib/src/messaging/send/tests.rs
  - lib/src/model_catalog/service.rs
  - lib/src/model_catalog/service/tests.rs
  - lib/src/permissions/engine.rs
  - lib/src/permissions/engine/tests.rs
  - lib/src/permissions/providers/claude.rs
  - lib/src/permissions/providers/claude/tests.rs
  - lib/src/permissions/providers/codex.rs
  - lib/src/permissions/providers/codex/tests.rs
  - lib/src/permissions/providers/gemini.rs
  - lib/src/permissions/providers/gemini/tests.rs
  - lib/src/permissions/providers/qwen.rs
  - lib/src/permissions/providers/qwen/tests.rs
  - lib/src/permissions/query.rs
  - lib/src/permissions/query/tests.rs
  - lib/src/protect/catalog.rs
  - lib/src/protect/catalog/tests.rs
  - lib/src/protect/observe.rs
  - lib/src/protect/observe/tests.rs
  - lib/src/protect/service.rs
  - lib/src/protect/service/tests.rs
  - lib/src/provider/methods.rs
  - lib/src/provider/methods/tests.rs
  - lib/src/provider/mod.rs
  - lib/src/render/prompt/system.rs
  - lib/src/render/prompt/system/tests.rs
  - lib/src/reporting/ingest.rs
  - lib/src/reporting/ingest/tests.rs
  - lib/src/runaway/config.rs
  - lib/src/runaway/config/tests.rs
  - lib/src/runaway/detector.rs
  - lib/src/runaway/detector/tests.rs
  - lib/src/signals/bespoke.rs
  - lib/src/signals/bespoke/tests.rs
  - lib/src/stream/badges.rs
  - lib/src/stream/badges/tests.rs
  - lib/src/stream/logs/opencode/bridge/tests.rs
  - lib/src/stream/logs/opencode/bridge/tests/ingest_classification.rs
  - lib/src/stream/logs/opencode/bridge/tests/mod.rs
  - lib/src/stream/logs/opencode/bridge/tests/session_lifecycle.rs
  - lib/src/stream/logs/opencode/bridge/tests/signal_projection.rs
  - lib/src/stream/logs/opencode/bridge/tests/stalled_generation_progress.rs
  - lib/src/stream/logs/opencode/bridge/tests/stdout_stderr_coordination.rs
  - lib/src/stream/logs/opencode/bridge/tests/usage_retry_guards.rs
  - lib/src/stream/logs/opencode/events.rs
  - lib/src/stream/logs/opencode/events/tests.rs
  - lib/src/stream/progress.rs
  - lib/src/stream/progress/tests.rs
  - lib/src/stream/protocol/claude.rs
  - lib/src/stream/protocol/claude/tests.rs
  - lib/src/stream/protocol/codex.rs
  - lib/src/stream/protocol/codex/tests.rs
  - lib/src/stream/protocol/kimi.rs
  - lib/src/stream/protocol/kimi/tests.rs
  - lib/src/stream/protocol/opencode.rs
  - lib/src/stream/protocol/opencode/tests.rs
  - lib/src/stream/providers/claude.rs
  - lib/src/stream/providers/claude/tests.rs
  - lib/src/stream/providers/codex.rs
  - lib/src/stream/providers/codex/tests.rs
  - lib/src/stream/providers/gemini.rs
  - lib/src/stream/providers/gemini/tests.rs
  - lib/src/stream/providers/kimi.rs
  - lib/src/stream/providers/kimi/tests.rs
  - lib/src/stream/providers/opencode.rs
  - lib/src/stream/providers/opencode/tests.rs
  - lib/src/stream/providers/pi.rs
  - lib/src/stream/providers/pi/tests.rs
  - lib/src/stream/reporting.rs
  - lib/src/stream/reporting/tests.rs
  - lib/src/stream/semantic.rs
  - lib/src/stream/semantic/tests.rs
  - lib/src/stream/stderr.rs
  - lib/src/stream/stderr/tests.rs
  - lib/src/stream/tool_display.rs
  - lib/src/stream/tool_display/from_event_tests.rs
  - lib/src/stream/tool_display/humanize_tests.rs
  - lib/src/stream/tool_display/summary_tests.rs
  - lib/src/stream/tool_display/tests.rs
  - lib/src/system_prompt/prepare.rs
  - lib/src/system_prompt/prepare/tests.rs
  - lib/src/system_prompt/resolve.rs
  - lib/src/system_prompt/resolve/tests.rs
  - rendezvous/daemon/src/peers.rs
  - rendezvous/daemon/src/peers/tests.rs
  - rendezvous/daemon/src/register.rs
  - rendezvous/daemon/src/register/tests.rs
  - rendezvous/daemon/src/service.rs
  - rendezvous/daemon/src/service/tests.rs
  - rendezvous/daemon/src/service/tests/mod.rs
  - rendezvous/daemon/src/service/tests/rpc.rs
  - rendezvous/daemon/src/service/tests/session_register.rs
  - rendezvous/daemon/src/service/tests/validation.rs
  - rendezvous/daemon/src/session_log/append.rs
  - rendezvous/daemon/src/session_log/mod.rs
  - rendezvous/daemon/src/session_log/rehydrate.rs
  - rendezvous/daemon/src/session_log/staging.rs
  - rendezvous/daemon/src/session_log/tests.rs
  - rendezvous/daemon/src/session_log/tests/append_rotation.rs
  - rendezvous/daemon/src/session_log/tests/durability.rs
  - rendezvous/daemon/src/session_log/tests/mod.rs
  - rendezvous/daemon/src/session_log/tests/remote_validation.rs
  - rendezvous/daemon/src/session_log/tests/replace_update.rs
  - rendezvous/daemon/src/session_log/tests/replay_rehydration.rs
  - rendezvous/daemon/src/session_log/validate.rs
  - rendezvous/daemon/src/sync.rs
  - rendezvous/daemon/src/sync/tests.rs
  - rendezvous/daemon/src/sync/tests/envelope_validation.rs
  - rendezvous/daemon/src/sync/tests/mod.rs
  - rendezvous/daemon/src/sync/tests/schema_validation.rs
  - rendezvous/daemon/src/sync/tests/snapshot_replace.rs
documentation:
  - docs/providers/dispatch-inventory.json
  - docs/topics/composition.md
  - docs/topics/flow-control/looping.md
  - docs/topics/provider-metadata.md
  - reviews/2026-07-14-module-assessment/generated-artifact-baseline.json
  - reviews/2026-07-14-module-assessment/phase-1-baseline.md
  - reviews/2026-07-14-module-assessment/plan.md
packages:
  - claudine
  - claudine-cli
  - claudine-contract
  - claudine-gen
  - rendezvous-daemon
---

# Claudine Module-Assessment Implementation Plan

This plan closes every remaining recommendation in [`review.md`](review.md). It
is ordered to preserve behavior at the riskiest boundaries, establish
regression guards before broad mechanical movement, and keep provider-neutral
policy in the library while CLI code retains process and terminal I/O.

## Completion criteria

The work is complete when all of the following are true:

- `run_harness_loop_inner` and
  `execute_composition_request_inner_with_guard` are readable coordinators over
  explicit phase functions and transition results; neither immediately
  destructures a large context into loosely coupled locals.
- Provider-neutral lifecycle transition policy has one owner in the library.
  The CLI supplies process launch, filesystem, messaging, and rendering
  adapters without introducing CLI dependencies into `claudine`.
- A hard structural test enforces the documented inline-test thresholds across
  all Claudine package-area crates, excludes generated sources, and has only
  narrow exceptions with recorded reasons.
- Every current inline-test violation is either migrated or explicitly
  justified, and the six largest extracted test suites are divided by behavior
  rather than retained as monolithic `tests.rs` files.
- Rendezvous is documented as a first-class package-area family, and its
  session-log responsibilities and oversized sync/service tests have coherent
  module owners.
- Wrapper spawning and termination are split by stable responsibility without
  duplicating the Unix/Windows signal ladder or changing interruption,
  timeout, watchdog, and child-reaping semantics.
- `claudine-gen` retains its bootstrap-independent crate boundary, has
  domain-oriented emitter/coercion modules, and renders every human-facing
  output path through `TerminalRenderable`; machine-facing JSON remains raw.
- Composition error rendering delegates by error family while the public
  `CompositionError` vocabulary remains intact.
- Loop/lifecycle interpolation either shares the Darkmatter substrate or has a
  documented intentional divergence backed by common conformance cases.
- The cited provider-metadata and package-architecture documentation drift is
  removed, generated artifacts remain byte-clean, and macOS, Linux, and Windows
  verification gates pass.

## Risk and sequencing constraints

GitNexus reports HIGH upstream risk for `run_harness_loop_inner`,
`run_child_stream_semantic`, and
`wait_with_signal_early_termination_and_completion`. The first affects
composition, lifecycle, wrapper, and execution flows; the latter two affect
the harness and Ctrl+C/termination paths. Before changing any named symbol in
this plan, rerun upstream impact analysis and stop for review if the result is
HIGH or CRITICAL or its direct callers differ from this baseline.

The plan deliberately does not split generated provider data,
`signals/generated.rs`, the declarative `gen/src/registry.rs`,
`catalog-types/src/signal.rs`, the central `CompositionError` enum, or cohesive
render/stream modules solely to reduce line counts.

## Phase 1 — Lock current orchestration and platform behavior

Establish a behavior baseline before changing either state machine or the
cross-platform process layer.

- [x] Add focused characterization cases for lifecycle ordering across
  `initialize`, `start`, `success`, `failure`, `finalize`, and `loop`, including
  evaluation-error precedence, action errors, terminal-slot redesignation,
  and finalize-once behavior.
- [x] Pin recovery behavior for `retry`, `resume`, `proxy`, `stop`, `error`, and
  unsupported setup-phase recovery, including attempt budgets, proxy chains,
  session availability, and the `provider_launched` re-entry distinction.
- [x] Pin composition setup ordering for target selection, launch workspace,
  environment/MCP construction, argv and system-prompt preparation, lifecycle
  setup, initialize routing, and handoff to `run_composition_body`.
- [x] Pin process semantics for inherited, captured, and semantic-stream modes:
  normal completion, user interruption, timeout, watchdog termination,
  completion-triggered termination, exit-summary projection, and child reaping.
- [x] Record byte baselines for every generated provider artifact and stable
  snapshots for human-facing generator reports and composition error blocks.
- [x] Capture the current test-placement inventory and `hug god-files --json
  claudine` result as measurement inputs; generated and test-only files must be
  labeled separately from actionable production code.

Acceptance gates:

- [x] All new cases pass against the unrefactored implementation.
- [x] `just test-library`, `just test-cli`, `just test-gen`, and `just
  test-rendezvous` pass.
- [x] `just test-l2` passes on an available headless terminal backend; Windows
  completion/Ctrl+C cases are confirmed runnable in the Windows CI job.

## Phase 2 — Define the provider-neutral lifecycle transition core

Finish the C4 layering work before decomposing the two CLI callers.

- [x] Extend `lib/src/composition/lifecycle/runtime.rs` with the smallest pure
  transition vocabulary needed by both preflight and the harness loop. Model
  state inputs and decisions explicitly: lifecycle event/slot, launched state,
  prior/evaluation/action error, control action, available session, attempt and
  proxy budgets, and finalize state.
- [x] Return typed transition decisions such as continue/re-enter, finalize,
  terminal success/failure, proxy handoff, or abort. Keep filesystem access,
  process spawning, terminal output, messaging, and provider-specific command
  construction out of the library.
- [x] Replace CLI-side “mirror” helpers only where both callers genuinely share the
  transition contract. Retain CLI adapters for executing the pure decision.
- [x] Add table-driven library tests covering every event/control/error
  combination used by composition preflight and the harness run loop.
- [x] Review and update docs/comments for the changed transition contracts; remove
  prose that still describes one CLI path as mirroring another.

Acceptance gates:

- [x] The library transition matrix is exhaustive and both CLI paths consume it.
- [x] No dependency from `claudine` to `claudine-cli` or CLI-only crates is added.
- [x] Existing Phase 1 ordering and recovery tests remain unchanged and green.
- [x] `just test-library`, `just test-cli`, and `just lint` pass.

## Phase 3 — Decompose composition request setup and initialize routing

Turn `execute_composition_request_inner_with_guard` into a coordinator without
changing its public wrappers or `SingleCompositionOutcome` contract.

- [x] Introduce a cohesive composition-attempt state/context that remains intact
  across the pipeline instead of becoming a large set of locals.
- [x] Extract stable preparation phases with explicit inputs and outputs:
  selection/launch resolution, environment and MCP preparation, argv and
  system-prompt construction, lifecycle runtime construction, initialize
  execution/routing, and provider-run handoff.
- [x] Use a typed phase result for proceed, completed, blocked, or failed outcomes;
  route lifecycle transitions through the Phase 2 library core.
- [x] Keep target selection, command construction, filesystem access, perf
  collection, dry-run rendering, and provider launch in CLI modules.
- [x] Preserve `execute_composition_request`, sequence callers, interactive
  selection, silent/dry-run behavior, and error enrichment at the render
  boundary.

Acceptance gates:

- [x] The root function reads as ordered phase calls with no duplicated error or
  finalize routing and no immediate context destructuring.
- [x] Phase functions have focused unit tests, while Phase 1 end-to-end behavior
  remains byte/sequence equivalent.
- [x] `just test-cli`, `just test-library`, and `just lint` pass.

## Phase 4 — Decompose the harness attempt/recovery state machine

Refactor the HIGH-risk `run_harness_loop_inner` after the shared transition
contract and composition caller have proven the boundary.

- [x] Replace the current loose mutable locals with a `HarnessLoopState` that owns
  attempt count, retry/resume budgets, prompt/session state, proxy tracking,
  cached shell options, lifecycle guard state, and the immutable run context.
- [x] Extract phases for prompt materialization/preflight, attempt execution,
  result classification, lifecycle event execution, terminal recovery,
  requeue/proxy handling, and next-attempt preparation.
- [x] Make each phase return an explicit loop transition such as retry, resume,
  proxy, complete, or abort. Apply the Phase 2 provider-neutral decision in a
  CLI adapter rather than re-encoding event-specific gates.
- [x] Preserve `drive_terminal_recovery` where it remains the single terminal-tail
  executor; do not create a second recovery abstraction with overlapping
  ownership.
- [x] Keep process launch and provider command details in the existing CLI
  attempt/profile modules.

Acceptance gates:

- [x] The loop body exposes phase ordering and re-entry points without reading an
  800-line function, and state ownership is visible from the context type.
- [x] Retry/resume/proxy/finalize tests cover every transition and budget edge.
- [x] The Phase 1 harness characterization suite, `just test-cli`, `just
  test-library`, `just test-l2`, and `just lint` pass.

## Phase 5 — Build the test-placement analyzer

Create an accurate, reusable analyzer before mechanically moving the current
violations.

- [x] Add a structural test under the Claudine CLI integration-test tooling,
  following the existing dispatch-inventory scanner pattern, and scan
  `lib`, `cli`, `contract`, `catalog-types`, `gen`, and
  `rendezvous/{core,client,daemon}`.
- [x] Centralize the documented thresholds: approximately 800 production lines or
  300 lines in an inline `mod tests` body. Count production and test bodies
  separately and handle attributes, comments, strings, raw strings, and nested
  braces so diagnostics are stable.
- [x] Ignore generated files by explicit path/header rules, not by broad directory
  exclusions. Report the file, production-line count, test-line count, and
  threshold exceeded.
- [x] Support a narrow exception table whose entries contain a path and durable
  rationale; reject stale exceptions when a file no longer violates a rule.
- [x] Unit-test the analyzer with portable fixtures. Keep the repository-wide
  assertion in report-only mode until Phase 6 eliminates the current debt, so
  this phase remains green without grandfathering roughly 90 violations.

Acceptance gates:

- [x] Analyzer fixtures cover Unix and Windows newlines and the Rust constructs
  above.
- [x] The report reproduces the review's classes of violation and excludes
  generated provider/signal artifacts.
- [x] `just test-cli` and `just lint` pass.

## Phase 6 — Eliminate inline-test debt and activate the hard gate

Apply the analyzer to the entire package area as a mechanical, behavior-neutral
migration.

- [x] Move every current threshold-violating inline test module to a sibling
  `tests.rs` or `tests/mod.rs`, including the cited rendezvous sync/service,
  wrapper spawn/termination, stream-provider, composition/dispatch, and
  generator files.
- [x] Preserve test names, module visibility, `use super::*`, `cfg` gates, fixtures,
  serial annotations, and platform-specific imports. Do not combine these
  moves with production refactors or formatting churn.
- [x] Review each proposed exception individually. Keep only cases where
  co-location materially clarifies private invariants; record why extraction
  would be worse and require the exception to remain below a separately stated
  ceiling.
- [x] Switch the Phase 5 repository assertion from report-only to a normal Level 1
  test and remove any temporary inventory/baseline entries.
- [x] Update the Claudine architecture Test Placement section to identify the
  enforcing test and exception policy.

Acceptance gates:

- The structural test reports zero unapproved violations across all Claudine
  package-area crates.
- Production diffs in this phase contain only test-module declarations and
  necessary visibility/import adjustments.
- `just test`, `just test-rendezvous`, `just test-gen`, and `just lint` pass.

## Phase 7 — Divide the largest sibling tests by behavior

Resolve the remaining test-navigation hotspots without claiming a Rust
compile-unit optimization.

- [x] Divide `composition/lifecycle/tests.rs` into parse/config, validation,
  action-shape/control, audio/emission, guard/runtime, and diagnostics suites.
- [x] Divide harness `loop_control/tests.rs` into lifecycle ordering, terminal
  routing, retry/resume, proxy, and requeue suites.
- [x] Divide lifecycle executor tests into action dispatch, conditions/control,
  event-time interpolation, mutation visibility, and filesystem/lookup suites.
- [x] Divide OpenCode bridge tests into ingest/classification, session lifecycle,
  usage/retry guards, stalled-generation progress, stdout/stderr coordination,
  and signal projection suites.
- [x] Divide rendezvous session-log tests into append/rotation, durability,
  replay/rehydration, remote validation, and replace/update suites.
- [x] Divide loop-engine tests into seed/state, iteration/actions, rate limits, and
  lifecycle/control suites.
- [x] Put shared fixtures in a small parent `tests/mod.rs`; avoid a new catch-all
  helper module that simply relocates the original hotspot.

Acceptance gates:

- [x] Each test file has one discoverable behavioral responsibility and no copied
  fixtures or assertions.
- [x] Test names and coverage remain stable, and the Phase 6 placement guard stays
  green.
- [x] `just test-library`, `just test-cli`, `just test-rendezvous`, and `just lint`
  pass.

## Phase 8 — Give rendezvous an explicit architecture and session-log boundary

Apply the package area's architectural conventions to
`rendezvous/{core,client,daemon}`.

- [x] Update the Claudine skill overview and architecture document to enumerate
  all three rendezvous crates, their dependency direction, public roles, and
  test commands.
- [x] Keep `SessionLogManager` as the public facade while extracting its existing
  responsibilities into modules for local append/rotation, export/import
  staging, startup replay/rehydration, and remote metadata/schema/append-only
  validation.
- [x] Make shared session state and invariants explicit without exposing internal
  Loro/storage types unnecessarily. Preserve persistence ordering and recovery
  semantics pinned in Phase 1.
- [x] Complete the sync/service test extraction started in Phase 6 and organize
  those suites by protocol framing/timeouts, service RPC behavior, projection,
  and error mapping.
- [x] Review session-log, sync, and service docs/comments for stale responsibility
  descriptions after the moves.

Acceptance gates:

- [x] The facade's public API is unchanged unless a separately reviewed API change
  is required, and startup replay, crash-window, signature, append-only, and
  sync tests remain green.
- [x] `cd claudine/rendezvous && just check && just test && just lint` passes.
- [x] Root `cargo check --workspace --all-targets` still includes all three crates.

## Phase 9 — Split wrapper spawn modes and platform termination

Refactor the HIGH-risk process layer by execution mode and platform boundary.

- [x] Split spawn code into shared command/process setup plus inherited-output,
  captured-output, and semantic-stream execution modules. Share only stable
  setup and wait contracts; keep mode-specific pipe/thread/parser behavior
  local.
- [x] Split termination into provider-neutral termination reasons, summary/guard
  projection, and human-facing rendering, plus `cfg(unix)` and `cfg(windows)`
  wait/escalation implementations behind one internal interface.
- [x] Keep one semantic signal ladder and one early-termination projection. Unix
  process-group signaling and Windows Job Object/console-event behavior must
  remain platform implementations of the same contract, not copied policy.
- [x] Render termination messages through existing terminal components and retain
  stdout/stderr separation.
- [x] Move the extracted platform tests into matching module trees and retain the
  L2/L3 integration coverage.

Acceptance gates:

- Normal exit, capture caps, semantic streaming, interruption feedback,
  timeout, watchdog, completion termination, and reaping cases pass.
- `just test-cli`, `just test-l2`, and `just lint` pass on macOS/Linux-capable
  paths.
- Windows CI runs `cargo check --all-targets` for `claudine-cli` and
  `just test-windows-ctrl-c`; no Unix-only import or path assumption reaches a
  Windows build.

## Phase 10 — Bound generator growth and complete its rendering contract

Preserve the strong generator crate boundary while reducing procedural
concentration and raw terminal output.

- [x] Split `gen/src/emit.rs` by stable catalog domains: identity/paths,
  execution/prompting, models/offerings, event/support policy, and linking
  resources. Keep shared literal/import helpers small and leave
  `emit_data_file` as a thin, visibly ordered assembler.
- [x] Split the `coerce_to_catalog_shape` decision tree along the same domain
  vocabulary so registry entries, coercion, and emission have predictable
  owners. Do not split the declarative registry merely because it is long.
- [x] Introduce typed generator report data and `TerminalRenderable` renderers
  using biscuit-terminal components (`Prose`, `UnorderedList`, `Table`,
  `CodeBlock`, or status components as appropriate). Replace human-facing
  `println!`/`eprintln!` paths for generate, check, provenance, diff, prompt,
  and agent-error reports.
- [x] Preserve raw JSON exclusively for explicit machine-facing modes such as
  mapping/structured reports, with stdout for data and stderr for diagnostics.
  Keep inherited-stdio `claudine providers generate` working.
- [x] Refresh `lib/src/provider/mod.rs` to describe the completed generated-data /
  handwritten-behavior architecture, and update
  `docs/topics/provider-metadata.md` from the stale 18-site statement to the
  authoritative current inventory (19 at assessment time, derived rather than
  hard-coded where practical).

Acceptance gates:

- Generated `data.rs`, catalog, signals, family, and vocabulary artifacts are
  byte-identical before/after the structural split.
- Generator output tests cover stdout/stderr, `NO_COLOR`, `FORCE_COLOR`, and
  plain/non-TTY degradation; machine JSON parses without ANSI or prose.
- `cd claudine/gen && just check && just test && just lint` and `just test-gen`
  pass.
- The dispatch inventory test and generated census agree.

## Phase 11 — Delegate composition error rendering by family

Complete C6 at the actionable rendering boundary while preserving the central
typed error vocabulary.

- [x] Keep `CompositionError` and its public variants in `error/mod.rs`.
- [x] Divide `BlockError::status_block` rendering into focused modules for
  lifecycle, schema/frontmatter, selection/target, sequence/loop, and
  provider/execution/file-reference errors.
- [x] Make the trait method a thin exhaustive dispatcher. Each family renderer
  returns the same `StatusBlock` and reuses shared path/link/code helpers rather
  than copying styles or prose.
- [x] Preserve diagnostic source chains, error codes, frontmatter appendices,
  TTY/color behavior, and the CLI error walker's deepest-typed-error rule.
- [x] Update comments only where responsibility or behavior descriptions moved.

Acceptance gates:

- Exhaustiveness remains compiler-enforced and no public error variant or code
  changes.
- Phase 1 snapshots and family-focused tests prove byte-equivalent terminal and
  plain rendering.
- `just test-library`, `just test-cli`, and `just lint` pass.

## Phase 12 — Resolve interpolation convergence and run the area-wide gate

Make the loop/lifecycle rendering relationship explicit, then close all
documentation and verification work.

- [x] Add one shared conformance matrix for syntax supported by both loop actions
  and lifecycle actions: literal/mixed strings, whole-value typed expansion,
  arrays/objects, namespaces and missing values, functions, escaping,
  malformed expressions, and strict/fail-closed behavior.
- [x] Compare `render_action_value`/`render_string_with_lookup` with lifecycle DM2
  `SubtreeCompose`. If the same input/state can preserve every established loop
  result, migrate loop action rendering to the Darkmatter substrate and remove
  the duplicate renderer. If a required semantic difference remains, keep the
  smaller implementation, document the exact differences and rationale in the
  composition architecture, and require both engines to pass the shared
  overlap matrix. This is an explicit evidence gate, not an open-ended design
  choice.
  - **Evidence-gate outcome: keep the loop renderer.** Both engines already share
    the Darkmatter expression core (`parse`/`evaluate`/`ExpressionFinder`/`scalar_string`)
    over `EvaluationLookup`; the loop is not a parallel *expression* engine. Three
    required semantic differences block migration: (1) the loop re-parses a
    mixed-string result as JSON (documented at `looping.md`), DM2 keeps mixed
    strings as strings; (2) the loop needs contextual `CompositionError::InvalidAction`
    errors (iteration/action index), DM2 returns generic `MarkdownError::Transform`;
    (3) the loop tolerates unknown roots (empty, matching condition evaluation),
    lifecycle DM2 runs strict/fail-closed. Shared overlap matrix:
    `lib/src/composition/interpolation_conformance.rs`.
- [x] Update all READMEs, Claudine skill pages, topic docs, and symbol comments
  affected by the 12 phases. Treat code and generated inventories as authority
  where old prose drifted.
  - Added "Loop vs lifecycle interpolation" to `docs/topics/composition.md`
    (symlinked into the claudine skill; hash re-stamped), cross-linked from
    `docs/topics/flow-control/looping.md`, and added a module-level design
    pointer on `looping/actions.rs`. Prior phases already updated their own
    skill/README/topic docs (see per-phase frontmatter); no residual drift
    surfaced in this sweep.
- [x] Rerun `hug god-files --json claudine` and report actionable production
  changes separately from generated/test files; success is clearer ownership
  and state-machine reviewability, not an indiscriminate line-count target.
  - Census reran (96 production / 96 test / 11 generated over-threshold files).
    The largest remaining production files are the deliberately-excluded central
    `CompositionError` enum (`error/mod.rs`) and the cohesive state machines
    (`pipeline.rs`, `loop_control.rs`, `lifecycle/executor.rs`, `looping/engine.rs`)
    — no actionable split remains beyond what the excluded-file list already
    protects. Ownership and reviewability improved via phases 3/4/8/9/10/11.
- [x] Run GitNexus `detect_changes` against `main` and inspect every affected
  execution flow before any commit. Re-run impact analysis for each changed
  public/shared symbol whose callers changed during implementation.
  - `detect_changes` (medium risk): the only affected execution flows are the
    `gen` codegen paths (`run`/`run_generate`) from Phase 10. Phase 12 itself
    adds only a `#[cfg(test)]` conformance module plus documentation — no
    production callers and no changed public/shared symbol, so no per-symbol
    impact re-analysis is required for phase 12.

Final acceptance gates:

- `just sanity`, `just lint`, `just doctest`, `just test`, `just
  test-rendezvous`, `just signals-check`, and `just test-l2` pass from the
  Claudine area; `cargo fmt --check` is diagnostic only and no write-mode
  formatting is run.
- `cargo check --workspace --all-targets` passes on macOS and Linux CI, and the
  Windows workspace/all-targets plus Ctrl+C gates pass on a Windows host.
- The test-placement guard has zero unapproved violations, generator and
  dispatch drift checks are clean, and no cited documentation drift remains.
- The final diff contains no generated-file hand edits, unrelated cleanup,
  accidental formatting churn, or changes to the areas explicitly excluded
  from line-count-driven splitting.

---
total_phases: 8
start_phase: 1
packages:
  - claudine-catalog-types
  - claudine-gen
  - claudine
  - claudine-cli
  - rendezvous-core
  - rendezvous-daemon
  - rendezvous-client
source_files_during_phase_1:
  - claudine/catalog-types/src/steering.rs
  - claudine/catalog-types/src/lib.rs
  - claudine/catalog-types/Cargo.toml
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/src/steering_check.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/inputs.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/l1/main.rs
  - claudine/gen/tests/l1/drift.rs
  - claudine/gen/tests/l1/generate_ux.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/src/lib.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/cli/tests/l1/dispatch_inventory.rs
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/research/steering/_schema.yaml
docs_updated_during_phase_1:
  - claudine/README.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/research/steering/_fleet.md
  - claudine/docs/research/steering/antigravity.md
  - claudine/docs/research/steering/claude.md
  - claudine/docs/research/steering/codex.md
  - claudine/docs/research/steering/gemini.md
  - claudine/docs/research/steering/goose.md
  - claudine/docs/research/steering/kilo.md
  - claudine/docs/research/steering/kimi.md
  - claudine/docs/research/steering/opencode.md
  - claudine/docs/research/steering/pi.md
  - claudine/docs/research/steering/qwen.md
docs_created_during_phase_1:
  - claudine/docs/topics/steering-activation.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/secrets/mod.rs
  - claudine/lib/src/secrets/tests.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/protect/scrub.rs
  - claudine/lib/src/messaging/send.rs
  - claudine/lib/src/reporting/jsonl.rs
  - claudine/lib/src/reporting/mod.rs
  - claudine/lib/src/reporting/paths.rs
  - claudine/lib/src/dispatch/logging.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/audit.rs
  - claudine/lib/src/steering/audit/tests.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_2:
  - claudine/README.md
  - claudine/docs/topics/messaging.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_2:
  - claudine/docs/topics/secret-recognition.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/catalog-types/src/steering.rs
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/Cargo.toml
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/audit.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/steering/controller/tests.rs
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/rendezvous/core/proto/rendezvous.proto
  - claudine/rendezvous/core/build.rs
  - claudine/rendezvous/core/src/lib.rs
  - claudine/rendezvous/daemon/src/lib.rs
  - claudine/rendezvous/daemon/src/server.rs
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/steering.rs
  - claudine/rendezvous/daemon/src/steering/tests.rs
  - claudine/rendezvous/client/Cargo.toml
  - claudine/rendezvous/client/tests/steering_round_trip.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/main.rs
  - claudine/cli/src/cli_utils.rs
  - claudine/cli/src/budget/run.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/requester.rs
  - claudine/cli/src/steering/wire.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/docs/providers/dispatch-inventory.json
  - Cargo.lock
docs_updated_during_phase_3:
  - claudine/README.md
  - claudine/docs/dependencies.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_3:
  - claudine/docs/topics/steering-routing.md
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/docs/providers/facts/pi.yaml
  - claudine/docs/providers/catalog.json
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/catalog-types/src/steering.rs
  - claudine/catalog-types/src/signal.rs
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/l1/generate_ux.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/src/provider/pi/data.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/lib/src/steering/controller/tests.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/lib/src/stream/protocol/pi.rs
  - claudine/lib/src/stream/protocol/pi/rpc.rs
  - claudine/lib/src/stream/protocol/pi/rpc/tests.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/pi/tests.rs
  - claudine/lib/src/stream/logs/opencode/bridge/mod.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/mod.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/commands.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/executor.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/spawn/descendants.rs
  - claudine/cli/src/commands/wrap/exec/termination/message.rs
  - claudine/cli/src/commands/wrap/exec/termination/reasons.rs
  - claudine/cli/src/commands/wrap/exec/termination/summary.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/pi.rs
  - claudine/cli/src/commands/wrap/profile/tests/pi_managed.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/bin/fake_pi/main.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/pi_managed_rpc.rs
  - claudine/cli/tests/real/main.rs
  - claudine/cli/tests/real/real_pi_managed_rpc.rs
  - claudine/cli/tests/real/real_pi_steering.rs
  - claudine/cli/tests/fixtures/steering/pi-probe.ts
  - claudine/cli/tests/fixtures/steering/pi-bash-cleanup-probe.ts
docs_updated_during_phase_4:
  - claudine/README.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/timeouts.md
docs_created_during_phase_4:
  - claudine/docs/topics/pi-rpc.md
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/rendezvous/core/proto/rendezvous.proto
  - claudine/cli/src/args.rs
  - claudine/cli/src/main.rs
  - claudine/cli/src/telemetry.rs
  - claudine/cli/src/commands/mod.rs
  - claudine/cli/src/commands/help.rs
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/src/commands/steer/service.rs
  - claudine/cli/src/commands/steer/interact.rs
  - claudine/cli/src/commands/steer/render.rs
  - claudine/cli/src/commands/steer/tests.rs
  - claudine/cli/src/completion/root_menu.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/requester.rs
  - claudine/cli/src/steering/wire.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/steer_cli.rs
  - claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_5:
  - claudine/README.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_5:
  - claudine/docs/cli/steer.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/detector/tests.rs
  - claudine/lib/src/runaway/detector/tests/warnings.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/automatic.rs
  - claudine/lib/src/steering/automatic/tests.rs
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/claudine_config/tests.rs
  - claudine/lib/src/config/merge.rs
  - claudine/lib/src/dispatch/runner/speak.rs
  - claudine/lib/src/dispatch/runner/tests.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/automatic.rs
  - claudine/cli/src/steering/automatic/tests.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/src/commands/steer/render.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/cli/src/commands/wrap/runaway_guard.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/automatic_help.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/mod.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/tests/bin/fake_pi/main.rs
  - claudine/cli/tests/l1/pi_managed_rpc.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_6:
  - claudine/README.md
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/steering-routing.md
docs_created_during_phase_6:
  - claudine/docs/topics/automatic-steering.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/src/stream/protocol/codex/app_server.rs
  - claudine/lib/src/stream/protocol/codex/app_server/tests.rs
  - claudine/lib/src/stream/protocol/fixtures/codex-app-server-steer-0.157.1.jsonl
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/codex/tests.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/native/mod.rs
  - claudine/lib/src/steering/native/claude_registry.rs
  - claudine/lib/src/steering/native/claude_registry/tests.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/mod.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/commands.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/launch.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/executor.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/tests.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/profile/pi.rs
  - claudine/cli/src/commands/wrap/profile/tests/pi_managed.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/steer/service.rs
  - claudine/cli/tests/bin/fake_codex/main.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/common/codex_model.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/codex_app_server.rs
  - claudine/cli/tests/l1/steer_cli.rs
  - claudine/cli/tests/l1/compose_system_prompt_lifetime.rs
  - claudine/cli/tests/l1/shipped_prompt_contract.rs
  - claudine/cli/tests/real/main.rs
  - claudine/cli/tests/real/real_codex_app_server.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/catalog.json
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/features/2026-09-08-steering/verification/codex-macos-0.157.1.json
docs_updated_during_phase_7:
  - claudine/README.md
  - claudine/docs/cli/steer.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/automatic-steering.md
  - claudine/docs/topics/timeouts.md
  - claudine/docs/research/steering/codex.md
  - claudine/docs/research/steering/kimi.md
  - claudine/docs/research/non-interactive-sessions/codex.md
  - claudine/features/2026-09-08-steering/verification/README.md
docs_created_during_phase_7:
  - claudine/docs/topics/codex-app-server.md
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/tests/l1/steer_cli.rs
  - claudine/cli/tests/real/real_codex_app_server.rs
  - claudine/cli/tests/level2/level2_lifecycle_control.rs
  - claudine/justfile
docs_updated_during_phase_8:
  - claudine/README.md
  - claudine/docs/cli/steer.md
  - claudine/docs/topics/signal-handling.md
  - claudine/features/2026-09-08-steering/verification/README.md
  - claudine/features/2026-09-08-steering/uncertainties.md
  - claudine/features/2026-09-08-steering/fleet-run.md
docs_created_during_phase_8:
  - claudine/docs/research/summary/steering.md
skills_files_updated_during_phase_8: []
source_code:
  - claudine/catalog-types/src/steering.rs
  - claudine/catalog-types/src/lib.rs
  - claudine/catalog-types/Cargo.toml
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/src/steering_check.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/inputs.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/l1/main.rs
  - claudine/gen/tests/l1/drift.rs
  - claudine/gen/tests/l1/generate_ux.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/src/lib.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/cli/tests/l1/dispatch_inventory.rs
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/research/steering/_schema.yaml
  - claudine/lib/src/secrets/mod.rs
  - claudine/lib/src/secrets/tests.rs
  - claudine/lib/src/protect/scrub.rs
  - claudine/lib/src/messaging/send.rs
  - claudine/lib/src/reporting/jsonl.rs
  - claudine/lib/src/reporting/mod.rs
  - claudine/lib/src/reporting/paths.rs
  - claudine/lib/src/dispatch/logging.rs
  - claudine/lib/src/steering/audit.rs
  - claudine/lib/src/steering/audit/tests.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/lib/Cargo.toml
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/steering/controller/tests.rs
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/rendezvous/core/proto/rendezvous.proto
  - claudine/rendezvous/core/build.rs
  - claudine/rendezvous/core/src/lib.rs
  - claudine/rendezvous/daemon/src/lib.rs
  - claudine/rendezvous/daemon/src/server.rs
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/steering.rs
  - claudine/rendezvous/daemon/src/steering/tests.rs
  - claudine/rendezvous/client/Cargo.toml
  - claudine/rendezvous/client/tests/steering_round_trip.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/main.rs
  - claudine/cli/src/cli_utils.rs
  - claudine/cli/src/budget/run.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/requester.rs
  - claudine/cli/src/steering/wire.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/common/mod.rs
  - Cargo.lock
  - claudine/docs/providers/facts/pi.yaml
  - claudine/docs/providers/catalog.json
  - claudine/catalog-types/src/signal.rs
  - claudine/lib/src/provider/pi/data.rs
  - claudine/lib/src/stream/protocol/pi.rs
  - claudine/lib/src/stream/protocol/pi/rpc.rs
  - claudine/lib/src/stream/protocol/pi/rpc/tests.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/pi/tests.rs
  - claudine/lib/src/stream/logs/opencode/bridge/mod.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/mod.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/commands.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/executor.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/spawn/descendants.rs
  - claudine/cli/src/commands/wrap/exec/termination/message.rs
  - claudine/cli/src/commands/wrap/exec/termination/reasons.rs
  - claudine/cli/src/commands/wrap/exec/termination/summary.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/pi.rs
  - claudine/cli/src/commands/wrap/profile/tests/pi_managed.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/tests/bin/fake_pi/main.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/pi_managed_rpc.rs
  - claudine/cli/tests/real/main.rs
  - claudine/cli/tests/real/real_pi_managed_rpc.rs
  - claudine/cli/tests/real/real_pi_steering.rs
  - claudine/cli/tests/fixtures/steering/pi-probe.ts
  - claudine/cli/tests/fixtures/steering/pi-bash-cleanup-probe.ts
  - claudine/cli/src/args.rs
  - claudine/cli/src/telemetry.rs
  - claudine/cli/src/commands/mod.rs
  - claudine/cli/src/commands/help.rs
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/src/commands/steer/service.rs
  - claudine/cli/src/commands/steer/interact.rs
  - claudine/cli/src/commands/steer/render.rs
  - claudine/cli/src/commands/steer/tests.rs
  - claudine/cli/src/completion/root_menu.rs
  - claudine/cli/tests/l1/steer_cli.rs
  - claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/detector/tests.rs
  - claudine/lib/src/runaway/detector/tests/warnings.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/steering/automatic.rs
  - claudine/lib/src/steering/automatic/tests.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/claudine_config/tests.rs
  - claudine/lib/src/config/merge.rs
  - claudine/lib/src/dispatch/runner/speak.rs
  - claudine/lib/src/dispatch/runner/tests.rs
  - claudine/cli/src/steering/automatic.rs
  - claudine/cli/src/steering/automatic/tests.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/cli/src/commands/wrap/runaway_guard.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/automatic_help.rs
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/src/stream/protocol/codex/app_server.rs
  - claudine/lib/src/stream/protocol/codex/app_server/tests.rs
  - claudine/lib/src/stream/protocol/fixtures/codex-app-server-steer-0.157.1.jsonl
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/codex/tests.rs
  - claudine/lib/src/steering/native/mod.rs
  - claudine/lib/src/steering/native/claude_registry.rs
  - claudine/lib/src/steering/native/claude_registry/tests.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/mod.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/commands.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/launch.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/executor.rs
  - claudine/cli/src/commands/wrap/exec/codex_app_server/tests.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/tests/bin/fake_codex/main.rs
  - claudine/cli/tests/common/codex_model.rs
  - claudine/cli/tests/l1/codex_app_server.rs
  - claudine/cli/tests/l1/compose_system_prompt_lifetime.rs
  - claudine/cli/tests/l1/shipped_prompt_contract.rs
  - claudine/cli/tests/real/real_codex_app_server.rs
  - claudine/features/2026-09-08-steering/verification/codex-macos-0.157.1.json
  - claudine/cli/tests/level2/level2_lifecycle_control.rs
  - claudine/justfile
documentation:
  - claudine/README.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/research/steering/_fleet.md
  - claudine/docs/research/steering/antigravity.md
  - claudine/docs/research/steering/claude.md
  - claudine/docs/research/steering/codex.md
  - claudine/docs/research/steering/gemini.md
  - claudine/docs/research/steering/goose.md
  - claudine/docs/research/steering/kilo.md
  - claudine/docs/research/steering/kimi.md
  - claudine/docs/research/steering/opencode.md
  - claudine/docs/research/steering/pi.md
  - claudine/docs/research/steering/qwen.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/messaging.md
  - claudine/docs/topics/traces-and-logging.md
  - claudine/docs/topics/secret-recognition.md
  - claudine/docs/dependencies.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/pi-rpc.md
  - claudine/docs/cli/steer.md
  - claudine/docs/topics/automatic-steering.md
  - claudine/docs/research/non-interactive-sessions/codex.md
  - claudine/features/2026-09-08-steering/verification/README.md
  - claudine/docs/topics/codex-app-server.md
  - claudine/docs/topics/signal-handling.md
  - claudine/features/2026-09-08-steering/uncertainties.md
  - claudine/features/2026-09-08-steering/fleet-run.md
  - claudine/docs/research/summary/steering.md
completed_phase: 8
implemented: true
---
# Steering implementation plan

Status: Phases 1–8 implemented (2026-09-28); Phase 7 closed with Codex as the
only enabled provider (option A of its human review, assumed). Codex managed
runs are steerable on macOS at Codex 0.157.1; Pi steering stays blocked by a
reviewed policy block. Two Phase 8 documentation steps are blocked (research
summary generation, Claudine skill edit).
Created: 2026-09-08
Specification: [spec.md](spec.md)
Evidence: [fleet run](fleet-run.md), [uncertainty register](uncertainties.md),
[disposable verification](verification/README.md)

## Execution contract

Execute Phase 1 through Phase 8 in order. This plan contains no deferred user
choices. The specification's **Resolved Engineering Contract** supplies concrete
CLI, configuration, control-channel, timeout, recovery, logging, and failure
policies. Earlier pilot recommendations are historical context where superseded.
Do not reopen accepted decisions during execution.

Each phase produces a reviewable change and records its changed files, checks,
results, and remaining external blockers in a progress section appended here.
Do not mark a phase complete while its required implementation or regression
checks are unfinished. A missing provider binary, credential, service, or native
OS runner blocks that capability's activation; it does not justify fabricated
evidence or prevent independent phases from proceeding. Record a phase with such
outstanding required evidence as **implemented; verification blocked**, not complete.

Before editing existing symbols, run GitNexus upstream impact analysis and report
callers, affected processes, and risk. Warn on high/critical risk. If a symbol is
unindexed, refresh or inspect its source/callers and record the coverage limitation;
never describe an unknown result as zero impact. Keep changes within the reviewed
scope, preserve unrelated work, and update behavioral comments with code changes.
Do not commit or run `cargo fmt` without a separate explicit instruction.

Use the Claudine skill throughout, plus the relevant package skills for touched
areas. Use Sniff for host/process discovery and stable user identity, Biscuit-file
for file-reference resolution and data-format conversion, Darkmatter for Markdown
hashes where present, and Biscuit-hash for generated non-Markdown hash baselines.
Use existing library/CLI boundaries and generated provider dispatch; do not add
runtime shell commands copied from research or a parallel capability table.

Runtime human consent for `claudine steer` interruption remains a product feature.
Test it with controlled terminal/input fixtures; the implementation agent must not
stop for a human to operate a test. Research workers use `gpt-5.6-sol` with low
reasoning. Real-provider tests use deliberately created disposable sessions,
prefer deterministic local models, preserve enabled resources, and never focus a
terminal/browser or contact an existing user session.

## Starting baseline

The following work already exists and must be extended rather than repeated:

- All ten roster providers have revision-3 steering research and typed execution
  interface backfills. The full passive fleet and four pilots are complete.
- `claudine providers steering check [slug] [--json]` and the generator equivalent
  check schema relationships, coverage, references, and execution-topic links.
  Generation already gates application of catalog/provider data on these checks.
- Execution-research metadata is already present in the regenerated catalog.
  Runtime steering selection, reviewed production adapters, and the public
  `steer` command are not implemented by that metadata work.
- Pi 0.84.4/macOS has seven scoped verification records. The real-RPC fixtures
  establish useful delivery behavior and important expected failures. They are
  native disposable fixtures, not production-wrapper verification or broad
  extension/profile/OS certification.
- Pi accepts queued text before delivery; duplicate IDs do not suppress duplicate
  messages; pending switches and provider termination can lose accepted text.
  Abort can move queued text into conversation history. Normal abort cleans up
  the tested external process, while killing Pi leaves it alive until fixture
  cleanup. Preserve these regression cases.

No phase may enable a capability merely because a report contains `outcome: passed`.
Required successful-delivery assertions, exact applicability, compatible runtime
interface, and reviewed adapter implementation are separate gates.

## Phase 1 — Typed contracts and generated eligibility

**Dependencies:** Existing research and the resolved specification.

**Work**

- [x] 1. Add shared steering and execution-interface vocabulary to
   [catalog-types](../../catalog-types/src/lib.rs), keeping transport, encoding,
   operation intent, receipt strength, execution state, and delivery state distinct.
   Define typed runtime request/result and identity contracts in the Claudine
   library. Preserve explicit unknown and partial-interruption outcomes.
- [x] 2. Extend [research loading](../../gen/src/inputs.rs),
   [field ownership](../../gen/src/registry.rs), catalog coercion, and
   [Rust emission](../../gen/src/emit/mod.rs). Consume the existing steering
   records and execution selections rather than reauthoring provider facts.
- [x] 3. Extend the maintained checker with deterministic activation applicability:
   exact provider/version/OS/profile/origin/state/operation, adapter revision,
   and required assertion coverage. Represent reviewed adapter bindings by
   implemented identifiers, separate from the generated factual catalog.
   Add typed assertion/adapter references if the current prose-only assertion
   lists cannot support deterministic decisions; preserve historical records as
   evidence without automatically making them activation grants.
- [x] 4. Derive manual and automatic eligibility from operation effects, delivery
   boundaries, access, compatibility, and applicable verification. A next-turn
   follow-up is not active-loop rescue. A terminal-only response cannot satisfy
   prompt return-after-acceptance without separately established early acceptance.
- [x] 5. Regenerate affected catalog/provider artifacts and update their existing drift
   and hash baselines. Keep all unimplemented or insufficiently verified bindings
   unavailable, with an actionable reason.
- [x] Validation: generator/catalog unit tests; all-roster steering check;
   deterministic regeneration; negative activation fixtures.

**Validation:** Generator/catalog unit tests; maintained steering check for all
roster providers; deterministic regeneration; negative fixtures for missing IDs,
wrong OS/profile/origin/version, unreviewed adapter revision, expected-loss-only
records, unsupported mechanisms, and malformed execution references.

**Exit:** Generated facts and deterministic activation policy agree; adding a
source claim or an unrelated passing record cannot enable delivery. Existing
research remains intact and the checker reports specific blocking reasons.

**Failure behavior:** Repair schema/relationship regressions before later phases.
Unresolved provider facts remain unknown. Do not rerun a full fleet to repair a
local contract mapping; use bounded targeted research only where evidence is missing.

## Phase 2 — Shared secret recognition and steering audit

**Dependencies:** Phase 1 request/result contracts.

**Work**

- [x] 1. Introduce `claudine::secrets` for shared text-span and sensitive-key recognition.
   Extract common recognition from [protect scrubbing](../../lib/src/protect/scrub.rs),
   [argument sanitization](../../cli/src/commands/wrap/env/sanitize.rs), and
   [webhook redaction](../../lib/src/messaging/send.rs) where their rules overlap.
   Keep each existing consumer's replacement and privacy policies unchanged.
- [x] 2. Implement steering's `****` replacement over merged UTF-8-safe secret spans.
   Preserve ordinary prose, email addresses, and paths unless a recognized secret
   is present. The delivered message retains its original bytes.
- [x] 3. Add typed steering events to the existing local JSONL logging path: full
   redacted text, opportunity/request/target identities, mechanism/profile,
   timestamps, consent, receipt strength, and separate cancellation/replacement
   results. Redact errors and provider echoes before tracing or rendering.
- [x] 4. Correlate append-only late-result updates without treating them as new sends.
   Audit failures produce content-free diagnostics and never cause replay or
   terminate an otherwise valid agent execution. Keep existing retention policy.
- [x] Validation: recognition fixtures, existing-consumer behavior, and injected
   audit-write failure.

**Validation:** Meaningful fixtures for token prose, assignments, auth headers,
credential-bearing URLs, overlap, Unicode, multiline text, repeated masking,
provider echoes, original-message preservation, and existing-consumer behavior.
Inject log-write failure and confirm the send result and execution remain correct.

**Exit:** Shared recognition has one authoritative implementation; steering logs
contain full masked text and delivery details, while original text reaches only
the intended in-memory delivery path.

**Failure behavior:** Fix redaction leakage before integrating live sends. Do not
solve regressions by deleting required audit text or changing unrelated privacy policy.

## Phase 3 — Session ownership, discovery, and local routing

**Dependencies:** Phases 1–2.

**Work**

- [x] 1. Extend Rendezvous core protobuf, client, and daemon with a local-only managed
   control stream, target listing, and send routing. Reuse
   [local IPC protections](../../docs/rendezvous/local-ipc.md). Do not add the
   daemon/database dependency to ordinary wrapper execution or duplicate its
   Unix/Windows transport implementation.
- [x] 2. Connect an execution-owned controller to the wrapper lifecycle near
   [session reporting](../../cli/src/commands/wrap/session_report.rs). Keep control
   registration distinct from replicated presence and its reporting opt-out.
   The controller exclusively owns provider I/O, correlates requests, and
   serializes mutations while continuously draining output/events.
- [x] 3. Implement per-execution UUIDs, wrapper/process-start identity, provider
   conversation generation, disconnect cleanup, and fresh target checks. Invalidate
   targets on conversation replacement; reject stale IDs rather than retargeting.
- [x] 4. Implement bounded routing: 16 pending requests per execution, one automatic
   request pending/in flight, 10-second manual acceptance and 2-second automatic
   deadlines. Discard expired unsent requests; report unknown after ambiguous
   submission. Never replay on reconnection or duplicate correlation IDs.
- [x] 5. Build a discovery aggregator over managed control registrations and generated
   native-discovery bindings. Use the five-second/four-concurrent-provider bounds,
   exact-identity deduplication, unknown states, partial discovery errors, and
   unavailable reasons specified in the spec. History and mesh presence are hints,
   not proof of local ownership, liveness, or a writable channel.
- [x] 6. Keep direct automatic delivery available inside an owner when Rendezvous is
   absent. Expose missing external routing as unavailable; daemon failure must
   not fail the wrapped task. Native provider discovery remains independent.

- [x] Validation: fake-controller integration tests, Unix socket / named-pipe
   ownership, and no message text in replicated or durable storage.

**Validation:** Fake-controller integration tests for concurrent senders, stale
IDs, reused PIDs, duplicate observations, disconnect/reconnect, missing daemon,
queue saturation, deadlines, late replies, wrong users, and remote mesh exclusion.
Test Unix sockets and native Windows named pipes with their real ownership rules.
No original message text may enter replicated registers or durable retry storage.

**Exit:** A separate local client can route a request to exactly one live owner
and receive only the receipt strength the owner establishes. Target changes and
transport failures cannot redirect or replay work.

**Failure behavior:** Product routing regressions block live integration. Missing
native-OS runners remain explicit verification blockers; do not substitute WSL
for native Windows evidence.

## Phase 4 — Managed Pi RPC execution and adapter

**Dependencies:** Phases 1–3; existing Pi fixtures.

**Work**

- [x] 1. Implement Pi's retained RPC profile in the wrapper and hand-written provider
   behavior, using the generated execution-interface selection. Reconcile
   [Pi facts](../../docs/providers/facts/pi.yaml), overrides, generated data,
   [launch profile](../../cli/src/commands/wrap/profile/pi.rs), and stream parsing.
   RPC is a control interface with structured events, not merely an output flag.
- [x] 2. Preserve extensions, skills, templates, context, MCP, model, sandbox, and
   permission settings. Remove automatically injected resource-disabling flags
   from the selected managed profile. Do not conflate Pi resource trust with tool
   or extension approval and do not broaden existing permission policy.
- [x] 3. Implement initial prompt, active steering, idle prompt, state/identity checks,
   acceptance correlation, and manual abort-then-submit. Preserve pending queues;
   clearing is not part of the default interruption workflow. Verify cancellation
   before revalidating and sending replacement; report partial outcomes honestly.
- [x] 4. Handle unattended requests as specified: existing policy, informational
   responses, documented deny/cancel, or `input_required` failure. Never fabricate
   approval or silently disable the requesting extension.
- [x] 5. Preserve existing semantic output, usage/error reporting, cancellation,
   watchdogs, and execution results. Wait for `agent_settled` before one-shot
   shutdown. Keep stdin open through settlement and retain ownership sufficient
   to clean up tools after provider failure.
- [x] 6. Integrate effective session-change guards. The existing gated-switch test is
   a demonstrated counterexample to snapshot-only safety. If configured extensions
   can bypass ownership coordination, keep that profile unavailable for steering;
   preserve its resources and record the specific blocker.
- [x] 7. Apply pre-submission fallback policy to a verified feature-preserving alternate
   interface. No fallback or initial-task replay after ambiguous submission.
- [x] Validation: wrapper-level real Pi tests and deterministic fake-RPC wrapper
   tests, as listed below.

**Validation:** Extend [real Pi tests](../../cli/tests/real_pi_steering.rs) through
Claudine's actual wrapper, not only direct Pi children. Cover all existing fixture
assertions, working/idle delivery, settlement, resources, unattended requests,
queue retention, switch races, EOF, crash, process cleanup, and ordinary wrapper
output/error parity. Keep the existing expected-failure evidence as regressions.

**Exit:** The managed RPC implementation and wrapper-level deterministic tests
pass. Activate only exact profiles with effective targeting and matching delivery
verification. A Pi profile blocked by independent extension mutation is explicitly
blocked, not advertised as safe because its basic RPC fixture passed.

**Failure behavior:** Fix wrapper regressions before enabling RPC by default.
Missing credentials or unproven guards block the affected profile; continue later
provider-independent work using fake adapters. Do not disable extensions to pass.

## Phase 5 — Manual steering CLI and selection

**Dependencies:** Phases 1–3; Phase 4 supplies the first candidate real adapter.

**Work**

- [x] 1. Add the specified command forms, validation, exit codes, list JSON schema, and
   explicit-ID send JSON. Update CLI dispatch, help, argument normalization, and
   completions using existing conventions. Do not introduce send/list subcommands
   or an interruption-consent bypass flag.
- [x] 2. Render Provider, Session, Directory, State, and Steering with
   `TerminalRenderable` components. Show full IDs and wrapped reasons/details;
   unavailable rows are dim, single-struck, and unselectable. Plain output retains
   clear availability labels. Sort and deduplicate according to the spec.
- [x] 3. Require a selection even with one eligible session; support working and idle
   targets and explain when input starts a turn. Handle no sessions, none eligible,
   unknown state, stale selection, and user cancellation without broadcast.
- [x] 4. Implement interactive interruption consent bound to the selected identity and
   action. Explain running-tool and pending-message effects, revalidate before
   cancellation and replacement, and preserve their separate outcomes. Non-TTY
   and JSON execution never infer consent. If non-interrupting delivery becomes
   unavailable, do not switch to interruption silently.
- [x] 5. Return after provider acceptance and report accepted/queued/delivered only as
   established. Show held as undelivered, explain separate setup, and keep sending
   free of configuration changes or extension installation.
- [x] Validation: parser and JSON-contract tests, simulated picker and confirmation
   input, no-focus rendering tests, and integration tests, as listed below.

**Validation:** Parser and JSON-contract tests; simulated picker/confirmation
input; no-focus terminal tests for disabled styling, narrow layouts, and plain
output; integration tests for exact targeting, stale targets, late replies,
partial interruption, cancellation, and unavailable unattended consent.

**Exit:** Manual selection, explicit targeting, listing, receipts, and failures
work with shared service contracts and verified adapters. Unavailable providers
remain visible with accurate reasons. No interactive test needs a real human.

**Failure behavior:** Block command release on misdelivery, consent bypass,
incorrect receipt upgrades, or leaked text. Provider absence does not make fake
CLI tests inconclusive or justify hiding unsupported rows.

## Phase 6 — Automatic repetition warnings

**Dependencies:** Phases 1–3 and shared delivery exercised in Phase 5.

**Work**

- [x] 1. Extend [ContentDetector](../../lib/src/runaway/detector.rs) with separate pure
   warning/recovery observations while preserving its terminal trip behavior.
   Warn at ceiling-half the stop limit only when repetition is established and
   before termination. A limit of one has no warning opportunity.
- [x] 2. Implement recovery as `max(8, 2 * previous_cycle_length)` nonblank normalized
   semantic-output lines without detected repetition. Freeze that cycle length
   until recovery; blank lines do not advance, detected repetition resets, and
   turn boundaries/acknowledgments/silence do not reset or advance recovery.
- [x] 3. Track three opportunities per agent execution, including unavailable and failed
   attempts. No same-episode retry/refund. After recovery a new episode must reach
   its own warning threshold. Retry/resume executions have distinct budgets even
   when conversation history is reused; ordinary turns retain their allowance.
- [x] 4. Resolve `steering.automatic.enabled` and `CLAUDINE_AUTO_STEER` with explicit-value
   inheritance and the specified parsing/errors. Opt-out removes automatic help
   and unavailable notices, preserving guards and manual steering.
- [x] 5. Wire observations through the existing live semantic sink to the producing
   execution's controller. Deliver the spec's helper text using only eligible
   non-interrupting operations. Do not attach monitoring to discovered native
   sessions or invent early warnings for capture-only paths.
- [x] 6. Enforce hard-stop priority within a chunk and during delivery, bounded work,
   unavailable-notice deduplication, and unchanged timeout clocks. Do not count
   steering control traffic as semantic progress or clear repetition evidence.
- [x] Validation: deterministic detector/runtime fixtures and preserved
   regressions, as listed below.

**Validation:** Deterministic detector/runtime fixtures for odd/small limits,
split chunks, same-chunk warning/stop, single/multiline cycles, new repeated
blocks, brief wording changes, blank lines, recovery, turns, execution restarts,
three-opportunity exhaustion, opt-out/inheritance, unsupported boundaries,
slow/failed sends, and termination during delivery. Preserve existing expression,
volume, silence, wall-clock, and cancellation regressions. Simulate recovery
and continued repetition, proving unchanged hard-stop thresholds.

**Exit:** Early helper delivery is observable before the existing limit when a
verified boundary permits it; continued repetition still stops on the existing
schedule. All automatic outcomes are bounded and never interrupt to deliver.

**Failure behavior:** Any changed hard-stop timing or evidence reset blocks this
phase. Unavailable provider delivery emits the bounded notice and retains the
original failure path; it is not grounds to weaken detection.

## Phase 7 — Remaining provider adapters and native coverage

**Dependencies:** Phases 1–6 shared infrastructure; no dependency on Pi activation.

**Work**

- [x] 1. Visit every eligible entry in [providers.yaml](../../docs/providers.yaml),
   respecting `skip_research`. Use this implementation order for the researched
   candidates: Codex, OpenCode, Claude Code, then remaining roster entries in
   roster order. Pi's managed candidate was addressed in Phase 4; include its
   native discovery/access disposition here. The roster remains authoritative.
- [ ] 2. For each candidate, implement researched discovery and usable delivery/control
   interfaces through shared protocol families and generated bindings. Keep
   ordinary native launches distinct from deliberately managed/server launches.
   Preserve provider features and exact compatibility probes.
- [ ] 3. Codex: test exact thread/expected-turn guards, idle-start separation, retained
   app-server ownership, receipt timing, and cancellation completion.
   OpenCode: test exposed-server versus internal launch access, same-user/session
   routing, early HTTP acceptance versus delivery, and partial interruption.
   Claude Code: test exact registry/socket framing, authentication per OS, inbound
   policy, correlation, and non-interactive held-message behavior.
- [ ] 4. Apply equivalent case-specific work to Goose, Kimi, Qwen, Kilo, Antigravity,
   and any subsequent eligible roster additions. Reuse ACP/HTTP/stdio behavior
   where protocols actually match; do not force every operation into active-turn
   steering or implement an unsupported terminal-keystroke substitute.
   *(Items 2–4: done for Codex (and Claude Code discovery); the rest is
   deferred to follow-up features under option A of the Phase 7 human review,
   assumed when Phase 8 was requested and awaiting the author's confirmation.)*
- [x] 5. Close concrete gaps with versioned-source inspection and bounded disposable
   tests. At most two corrective research passes per gap, with the requested
   model/effort. Preserve existing evidence and activation distinctions.
- [x] 6. Record each case as verified-enabled, implemented but externally blocked,
   evidenced unsupported, or unknown with a concrete next check. Missing access
   or implementation is not proof of provider incapability. Do not leave known
   implementable work unfinished under a generic unknown label.

- [x] Validation: fake-protocol failure tests and disposable exact-profile tests
   for every enabled binding, as listed below.

**Validation:** Fake-protocol failure tests plus disposable exact-profile delivery
and interruption tests for every enabled binding. Include long tools/batches,
stale identity, duplicate IDs, queue/receipt ambiguity, policy holds, process loss,
resource parity, and native/managed distinctions appropriate to each mechanism.

**Exit:** Every eligible provider has an explicit implementation/verification
outcome and discovery coverage. Available mechanisms are implemented and tested;
external prerequisites alone can leave otherwise implemented cases blocked.
Unsupported and unknown cases remain visible and cannot be selected accidentally.

**Failure behavior:** Isolate unavailable credentials/binaries/hosts and continue
other providers. Do not silently change research model, configure real user
providers, or send into existing sessions. Reproducible implementation defects
remain required work, not external blockers.

## Phase 8 — Platform verification, documentation, and release gates

**Dependencies:** Implementation outputs from Phases 1–7.

**Work**

- [x] 1. Run required unit/integration/lint checks for affected packages using `just test`,
   `just test-l2`, and `just lint`; use nextest for targeted runs, never `cargo test`.
   Include catalog types, generator, library, CLI, changed Rendezvous crates, and
   affected redaction consumers. Keep ordinary L1 provider spawning hermetic;
   real-provider targets remain explicit opt-in tests with `real-tests`.
- [x] 2. Verify build and local IPC behavior on macOS, native Linux, and native Windows.
   Run provider delivery tests on each OS/version/profile before enabling that
   combination. Linux/WSL results never stand in for native Windows; compilation
   alone never satisfies a live-delivery assertion.
- [x] 3. Exercise a production-wrapper manual send, an automatic warning followed by
   recovery, and continued repetition reaching its original stop limit. Exercise
   native discovery/delivery independently wherever enabled. Test daemon absence,
   slow routing, controller/provider failure, cleanup, and no accidental replay.
- [x] 4. Run the maintained all-provider steering checker, schema validation, activation
   checks, and deterministic generation/drift checks. Verify that expected-loss
   artifacts and stale adapter revisions cannot activate delivery.
- [ ] 5. Update public README/help, CLI and timeout/signal topics, local IPC docs,
   provider setup/compatibility guidance, and the Claudine skill. Update dependency
   docs only for actual dependency changes. Produce the cross-provider research
   summary through the existing summary/publication workflow.
   *(2026-09-28: docs and help done; the summary prompt is authored but its
   `claudine sequence` run needed approval, and the skill edit was denied —
   both left for the author; see the Phase 8 progress below.)*
- [x] 6. Update this plan's progress, the spec status, uncertainty register, and fleet
   report with final implemented/enabled/blocked coverage. Preserve evidence paths,
   exact versions, OS and launch profiles, and remaining external requirements.
- [x] Validation: package, platform, end-to-end delivery, activation,
   generation, and documentation checks recorded with actual results.

**Validation:** Record the package, platform, end-to-end delivery, activation,
generation, and documentation checks above with actual results. Check public
examples against the implemented parser and JSON schema. Missing external tests
remain explicitly unrun; previous research results do not replace wrapper tests.

**Exit:** Required code checks pass; user-visible behavior and generated metadata
agree; every enabled capability has matching delivery and ownership evidence.
At least one real production-wrapper path demonstrates manual delivery and
non-interrupting automatic help before claiming the feature operational. Do not
claim universal support, complete platform activation, or successful delivery for
cases still externally blocked.

**Failure behavior:** Fix product regressions before declaring implementation
complete. If external resources prevent required verification, finish independent
checks and documentation, retain unavailable defaults, and report the exact blocked
cases. The plan finishes with an honest readiness report, not an interactive
permission question or a fabricated all-green result.

## Progress

| Phase | Status | Evidence |
| --- | --- | --- |
| Phase 1 | Implemented 2026-09-28 | Typed vocabulary, research revision 4, activation policy + checker, generated `lib/src/steering/generated.rs`, runtime eligibility; nothing activated. See implementation-log.md |
| Phase 2 | Implemented 2026-09-28 | `claudine::secrets` (one catalog + key-name recognizer; scrub, webhook, and wrapper sanitization migrated), `steering::audit` typed JSONL records under `~/.claudine/logs/steering/`; no live send path calls it yet. See implementation-log.md |
| Phase 3 | Implemented 2026-09-28 | `steering::controller` (bounded, serialized, audited owner queue), `steering::discovery` aggregator over generated discovery records, in-memory Rendezvous `SteeringControl`/`ListManagedTargets`/`RouteSteering`, wrapper owner link; every managed session registers as unavailable until Phase 4 maps a profile. See implementation-log.md |
| Phase 4 | Implemented 2026-09-28; Pi steering blocked | Managed Pi RPC execution (`exec/control.rs`, `exec/pi_rpc/`) with pre-submission JSON fallback, unattended-request policy, settlement, Unix tool reaping; `pi-rpc` adapter reviewed and implemented; `retained-rpc` blocked by the new policy `blocks` list. Real Pi 0.87.1 (macOS) and fake-Pi (macOS/Linux/Windows) verified. See implementation-log.md |
| Phase 5 | Implemented 2026-09-28 | `claudine steer` (`cli/src/commands/steer/`): list/JSON, explicit and picked sends, interactive consent, revalidation, receipts, exit codes; listings now carry the route's operation and the owner binding. Verified with fake service, scripted input, off-screen rendering, and a real daemon (macOS, Linux, native Windows). See implementation-log.md |
| Phase 6 | Implemented 2026-09-28 | Detector early-warning/recovery signals (`ContentDetector::observe`), `claudine::steering::automatic` (setting, precedence, 3-opportunity budget, helper message), `AutomaticHelp` in the live sink sending to the execution's own controller, hard-stop priority, deduplicated notices, and steering replies excluded from the silence clock. No real session is eligible yet, so the production path shows the unavailable notice; delivery is proven with fixture eligibility. See implementation-log.md |
| Phase 7 | Partially implemented 2026-09-28; human review requested | Codex managed app-server launch + `codex-app-server` adapter, **verified-enabled on macOS at Codex 0.157.1** (steer, idle turn; automatic help with recovery shown through the production wrapper); Claude Code native registry discovery (all OSes); every roster provider has a recorded outcome. OpenCode, Kilo, Gemini, Qwen adapters implementable but deferred; Goose externally blocked; Kimi needs a 2.x research refresh. See implementation-log.md |
| Phase 8 | Implemented 2026-09-28; two documentation steps blocked | Production-wrapper manual send through a real daemon, automatic warning + recovery, and continued repetition to the unchanged stop, all against real Codex 0.157.1 on macOS; all gates run (one pre-existing L1 failure, one load flake in L2); steering suites pass on native Linux, native Windows, and WSL2; docs, help, and evidence records updated. Blocked: the steering research-summary generation run and the Claudine skill edit. See implementation-log.md |

### Phase 1 progress (2026-09-28)

- **Changed:** see this file's `source_files_during_phase_1` and
  `docs_*_during_phase_1` frontmatter; details and the requirement-to-test map
  are in [implementation-log.md](implementation-log.md).
- **Checks:** catalog-types 30/30, `claudine --lib steering` 30/30,
  `claudine-gen` 192/192, `claudine-gen steering check` clean for all ten
  providers at revision 4, `claudine-gen check` clean, `just lint` clean.
  `just test`: 7787 passed, 9 skipped.
- **Departure:** steering is a standalone generated artifact (like the stream
  vocabulary), not a `ProviderInfo` registry field.
- **Remaining blockers:** none for Phase 2. No adapter is implemented and the
  activation policy is empty, so every case is unavailable by design.

### Phase 2 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_2` frontmatter; design decisions
  and the requirement-to-test map are in
  [implementation-log.md](implementation-log.md#phase-2).
- **Checks:** `just test` (claudine area): 7803 passed, 9 skipped, 0 failed.
  `just lint`: clean for all five crates.
- **Departure:** recognition is shared, so scrubbing and wrapper sanitization
  now recognize each other's sensitive key names and every catalog
  credential-token prefix. Replacement tokens and privacy policies (email,
  home path, `<redacted>`, `****`, env stripping) are unchanged.
- **Remaining blockers:** none for Phase 3. `steering::audit::audited_send` is
  the seam Phase 3 routing must call; no caller exists yet.

### Phase 3 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_3` frontmatter; design
  decisions and the requirement-to-test map are in
  [implementation-log.md](implementation-log.md#phase-3).
- **Checks:** `just test` (claudine area): 7840 passed, 9 skipped, 0 failed.
  CLI steering tests with `daemon-tests`: 12/12. Rendezvous area `just test`:
  284 passed; `just lint` clean in both areas. `just cross-check
  rendezvous-client` passed on native Windows (named pipes) and Linux.
- **Departures:** the generator now also projects research `discovery`
  records and a roster-order constant; the daemon closes its steering router
  on shutdown (a defect found by the tests: graceful shutdown waited forever
  on a connected owner); routed requests must be manual.
- **Remaining blockers:** none for Phase 4. Every wrapped execution registers
  with `profile_id: None`, so it is listed as unavailable until Phase 4 maps
  the Pi RPC launch to `retained-rpc` and supplies an executor.

### Phase 4 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_4` frontmatter; protocol facts,
  design decisions, the requirement-to-test map, and the input-robustness
  matrix are in [implementation-log.md](implementation-log.md#phase-4).
- **Checks:** `just test` (claudine area): 7888 passed, 9 skipped, 0 failed.
  `just lint`: clean. Real tier against Pi 0.87.1 on macOS: 11/11 plus the 3
  native regressions. `just cross-check claudine-cli` (affected tests):
  Linux 598/598, native Windows 414/414.
- **Departures:** the reviewed activation policy gained a required `blocks`
  list, used to block Pi's `retained-rpc` profile with its specific reason
  (plan item 6); resume now carries Pi's `--mode` and trust flag; tool
  processes a controlled provider starts outside its group are reaped on Unix.
- **Status:** implemented; Pi steering blocked. The managed launch and the
  `pi-rpc` adapter work against real Pi, but no grant is possible while
  extensions can switch the session unguarded. Real-Pi evidence exists for
  macOS only; the wrapper does not establish the provider version.
- **Remaining blockers:** none for Phase 5, which can use fake adapters (the
  plan's stated fallback when a guard is unproven). An operational real
  delivery (Phase 8's exit) needs either an effective Pi session guard or
  another provider's adapter.

### Phase 5 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_5` frontmatter; design
  decisions, the requirement-to-test map, and the input-robustness matrix are
  in [implementation-log.md](implementation-log.md#phase-5).
- **Checks:** `just test --no-fail-fast` (claudine area): 7929 passed, 9
  skipped, 1 failed — the failure is a compose union test from darkmatter work
  another session merged mid-phase (`25ca47440`), unrelated to steering.
  `just lint`: clean. Daemon-backed steer tests: 41/41 (macOS); `just
  cross-check claudine-cli`: native Windows and Linux 74/74, Windows L1
  `steer_cli` 3/3. Rendezvous area `just test`: 284 passed.
- **Departures:** the listing gained `operation` (JSON and a new proto field,
  `ManagedTargetInfo.operation = 14`), needed because the owner refuses any
  operation other than its current route's and availability alone cannot say
  which operation that is; the list JSON also carries `coverage_gaps`; `steer`
  is exempt from the config/init-wizard check.
- **Status:** implemented. No row is selectable against the shipped policy (Pi
  blocked, everything else unmapped), so the send path is proven end to end
  with fixture eligibility against a real daemon, not against a real provider.
- **Remaining blockers:** none for Phase 6. Native discovery and delivery are
  Phase 7 (`SteeringService` is the seam).

### Phase 6 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_6` frontmatter; design
  decisions, the requirement-to-test map, and the input-robustness matrix are
  in [implementation-log.md](implementation-log.md#phase-6).
- **Checks:** `just test --no-fail-fast` (claudine area): 7978 passed, 9
  skipped, 1 failed — the pre-existing darkmatter union test recorded in
  Phase 5. `just lint`: clean. `just cross-check` (new and affected tests):
  native Windows 56/56, Linux 56/56; claudine lib suites pass on Windows.
- **Departures:** automatic help reads its setting inside the existing guard
  resolution (same files, same pre-launch point); steering control replies no
  longer refresh the stream-silence clock (a defect found while checking
  "unchanged timeout clocks"); the helper message appends observed counts.
- **Status:** implemented. With the shipped policy every session prints the
  bounded "cannot send automatic steering" notice and stops on the unchanged
  schedule (L1-proven through the real wrapper); a confirmed automatic
  delivery is proven only with fixture eligibility.
- **Known gap:** duplicate keys in Claudine config files are last-wins for
  every key (JSON5 → `Value` loader); pinned by the matrix, not fixed here.
- **Remaining blockers:** none for Phase 7. Phase 8's "automatic warning
  followed by recovery" end-to-end needs a provider with an automatic grant.

### Phase 7 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_7` frontmatter; protocol
  facts, design decisions, per-provider outcomes, the requirement-to-test
  map, and the input-robustness matrices are in
  [implementation-log.md](implementation-log.md#phase-7).
- **Checks:** `just test --no-fail-fast` (claudine area): 8031 passed, 1
  failed (the pre-existing darkmatter union test), 9 skipped. `just lint`: clean. Real tier against Codex 0.157.1 on
  macOS: `real_codex_protocol` 8/8, `real_codex_app_server` 4/4. `just
  cross-check` (new and affected tests): Linux and native Windows pass.
- **Enabled:** the first reviewed grants — `app-server-steer` (working) and
  `app-server-turn-start` (idle) for Claudine-managed non-interactive Codex
  runs on macOS at exactly 0.157.1. A real `claudine codex` run that repeats
  itself receives the automatic warning at half the stop limit and recovers.
- **Departures:** the Codex non-interactive research now selects the managed
  app-server with `exec-json` as its fallback (corrective pass with source and
  disposable-test evidence); `StdioControl` gained `launch_args` and
  `stdio_control` receives the child cwd; the controller records a provider
  version the launch establishes (`set_provider_version`); `steer --list`
  without a daemon now succeeds with native rows (spec's partial-discovery
  rule); interrupt-then-start is implemented and verified but not granted.
- **Defects fixed:** Codex whole-item text never formed repetition-detector
  lines (the repetition guard could not trip on Codex agent messages).
- **Not done (human review):** plan items 2–4 are open for OpenCode, Kilo,
  Gemini, Qwen, Goose, Kimi, and Antigravity (see the per-provider outcomes
  table in the log). Pre-existing, outside steering: Claudine's
  non-interactive Kimi launch uses `--wire`, which Kimi Code 2.0.2 rejects.

### Phase 8 progress (2026-09-28)

- **Changed:** see this file's `*_during_phase_8` frontmatter; the full check
  table, requirement-to-test map, and readiness report are in
  [implementation-log.md](implementation-log.md#phase-8).
- **Checks:** `just test --no-fail-fast` (claudine area): 8031 passed, 1
  failed (the pre-existing Darkmatter union test), 9 skipped. `just test-l2`:
  272/273 after fixing 9 Codex-stub failures; the remaining one is a
  tmux Ctrl+C timing test that passes alone. `just lint` clean in both
  areas; Rendezvous `just test` 284 passed. Real tier (macOS): Codex wrapper
  6/6, Codex protocol 8/8, Pi 11/11 + 3 native regressions. Steering checker
  and generation drift clean for all ten providers. `just cross-check`:
  steering suites pass on native Linux, native Windows, and WSL2.
- **Added:** the production-wrapper manual-send test (real daemon, separate
  `claudine steer` process, real Codex) and a real continued-repetition test;
  `test-real` now enables `daemon-tests` for the CLI.
- **Defect fixed:** L2 Codex stubs that modelled `codex exec` hung under the
  managed app-server launch (Phase 7 did not run L2); they now exit on
  `app-server`, like Phase 7's L1 stubs.
- **Docs:** `steer --help` names the daemon prerequisite; `docs/cli/steer.md`
  gained setup and a provider compatibility table; signal handling covers
  controlled launches; README, verification notes, uncertainty register, and
  fleet report carry the final coverage.
- **Readiness:** operational for Codex 0.157.1 on macOS (Claudine-managed
  non-interactive runs), with manual and automatic delivery shown through the
  production wrapper. Not claimed: other Codex versions, Linux or native
  Windows delivery (no real-Codex run there), Pi (blocked), Claude Code
  delivery, or any other provider.
- **Blocked (external):** the `claudine sequence` run that generates
  `docs/research/summary/steering.md` needs approval; edits under
  `.claude/skills/claudine/` were denied (intended text is in the log).

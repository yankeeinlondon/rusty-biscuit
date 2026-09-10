---
total_phases: 10
created: 2026-09-07
phase: 10
agent: claude/default
yolo: "true"
fix: 2026-09-07-faster-claudine-tests
spec: claudine/fixes/2026-09-07-faster-claudine-tests/spec.md
packages:
    - claudine
    - claudine-cli
    - claudine-contract
    - claudine-catalog-types
    - claudine-gen
    - rendezvous-core
    - rendezvous-daemon
    - rendezvous-client
phase_1_status: partial — local gates green; CI tranche blocked on operator merge
source_files_during_phase_1:
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/fixtures/nextest-l1-excerpt.xml
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/expectations.json
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/_completed/2026-08-01-cli-slow-tests/deferred-performance.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/README.md
skills_files_updated_during_phase_1: []
# No Rust package source was touched: Phase 1 is a gate phase, and the plan
# forbids any code change from Phases 4-10 landing before its checkpoint passes.
packages_touched_during_phase_1: []
phase_2_status: complete — reconciler exits 0, checkpoint 2 passed
source_files_during_phase_2:
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory-reconciler.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory-reconciler.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/families.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/enumeration/captures.json
docs_updated_during_phase_2:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
docs_created_during_phase_2:
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
skills_files_updated_during_phase_2: []
# Document-only phase. No Rust package source was touched; Phases 4-10 remain
# blocked on Phase 1's CI checkpoint. The enumeration captures under
# `enumeration/` are evidence artifacts, not source.
packages_touched_during_phase_2: []
phase_3_status: >-
    partial — decisions 1-3 answered with measurements; decision 4 (budgets)
    refused rather than guessed, blocked on Phase 1's CI baseline
source_files_during_phase_3:
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution/launch-cwd-probe.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution/budgets-pending.json
docs_updated_during_phase_3:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
docs_created_during_phase_3:
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.md
skills_files_updated_during_phase_3: []
# Document-only phase, like Phase 2. No Rust package source was touched and no
# package was rebuilt except to run the area's own gates; Phases 4-10 remain
# blocked on Phase 1's CI checkpoint. The measurement artifacts under
# `attribution/` (48 nextest logs, 15 `cargo test` logs, three TSVs) are
# evidence, not source.
packages_touched_during_phase_3: []
phase_4_status: >-
    complete — one environment policy on two command surfaces; area gates green
    except one disclosed host-condition L2 failure; implemented but not
    committed, since Phase 1's checkpoint is still blocked on the operator merge
source_files_during_phase_4:
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/common/host_tools.rs
    - claudine/cli/tests/cli_process_fixture.rs
    - claudine/cli/tests/spawn_site_guard.rs
    - claudine/cli/tests/system_prompt_perf_bench.rs
    - claudine/cli/tests/sequence_magic_reference.rs
    - claudine/cli/tests/loop_cli.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
# Test-only change inside `claudine-cli`. No library, CLI, or production source
# was touched, so the blast radius is the L1/L2 test binaries that compile
# `cli/tests/common/`.
packages_touched_during_phase_4:
    - claudine-cli
phase_5_status: >-
    complete — SPAWN_ALLOWLIST empty, isolation population 37 -> 74 files with
    zero escapes, eight contamination probes added; area gates green except the
    one host-condition L2 failure Phase 4 already disclosed; implemented but not
    committed, since Phase 1's checkpoint is still blocked on the operator merge
source_files_during_phase_5:
    - claudine/cli/tests/contamination_probes.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/spawn_site_guard.rs
    - claudine/cli/tests/compose_cli.rs
    - claudine/cli/tests/compose_interactive_timeout_cli.rs
    - claudine/cli/tests/compose_removed_validation_keys.rs
    - claudine/cli/tests/compose_schema_cli.rs
    - claudine/cli/tests/compose_system_prompt_lifetime.rs
    - claudine/cli/tests/compose_ttff_perf.rs
    - claudine/cli/tests/composition_outputs.rs
    - claudine/cli/tests/inline_compose_cli.rs
    - claudine/cli/tests/sequence_cli.rs
    - claudine/cli/tests/sequence_errors_cli.rs
    - claudine/cli/tests/sequence_groups.rs
    - claudine/cli/tests/sequence_jit.rs
    - claudine/cli/tests/sequence_magic_reference.rs
    - claudine/cli/tests/sequence_prompt_property.rs
    - claudine/cli/tests/sequence_sources_cli.rs
    - claudine/cli/tests/loop_cli.rs
    - claudine/cli/tests/wrap_sequence_composition.rs
    - claudine/cli/tests/context_command.rs
    - claudine/cli/tests/skills_integration.rs
    - claudine/cli/tests/errors_command.rs
    - claudine/cli/tests/effective_diagnostic_render.rs
    - claudine/cli/tests/completion_contract.rs
    - claudine/cli/tests/completion_perf.rs
    - claudine/cli/tests/completion_resolution_round_trip.rs
    - claudine/cli/tests/handle_deadline.rs
    - claudine/cli/tests/handle_blocking_output.rs
    - claudine/cli/tests/level1_structured_error_message.rs
    - claudine/cli/tests/protect_cli.rs
    - claudine/cli/tests/provider_error_finalize.rs
    - claudine/cli/tests/shipped_prompts.rs
    - claudine/cli/tests/wrap_sigint.rs
    - claudine/cli/tests/level1_compose_autocomplete_failure_pty.rs
    - claudine/cli/tests/level1_inline_compose_mismatch_pty.rs
    - claudine/cli/tests/sequence_overlay_pty.rs
    - claudine/cli/tests/wrap_ctrl_c_windows.rs
    - claudine/cli/tests/sequence_ctrl_c_windows.rs
docs_updated_during_phase_5:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
docs_created_during_phase_5: []
# Minimal drift repair only, not Phase 10's sweep: both skills described an
# allow-list that is now empty and omitted the raw-command surface every
# live-child test uses. Phase 10 still owns the full skill/doc review.
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/rust-testing/SKILL.md
# Test-only change inside `claudine-cli`, like Phase 4: 39 L1 test binaries plus
# the shared fixture and the two structural guards. No library, CLI, or
# production source was touched.
packages_touched_during_phase_5:
    - claudine-cli
phase_6_status: >-
    complete — library L1 summed 187.40s -> 102.00s and elapsed 15.96s -> 8.89s;
    eight nextest override blocks removed and none added; four previously
    unreachable identities now run; area gates green except the one
    host-condition L2 failure Phases 4 and 5 already disclosed and four
    host-condition `real_`-tier failures that predate the recipe change;
    implemented but not committed, since Phase 1's checkpoint is still blocked
    on the operator merge
source_files_during_phase_6:
    - .config/nextest.toml
    - claudine/justfile
    - claudine/rendezvous/justfile
    - claudine/catalog-types/Cargo.toml
    - claudine/contract/Cargo.toml
    - claudine/rendezvous/core/Cargo.toml
    - claudine/rendezvous/daemon/Cargo.toml
    - claudine/rendezvous/client/Cargo.toml
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/linking/paths.rs
    - claudine/lib/src/render/event_renderer/mod.rs
    - claudine/lib/src/stream/stderr/tests.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/context_command.rs
    - claudine/cli/tests/wrap_opencode.rs
    - claudine/cli/tests/wrap_sigint.rs
    - claudine/gen/tests/signals_validation.rs
docs_updated_during_phase_6:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
docs_created_during_phase_6: []
# Drift repair only, in the two places this phase's changes falsified a written
# claim; Phase 10 still owns the full skill/doc review. The `claudine` SKILL
# described the rendezvous justfile as four recipes, and `architecture.md` named
# twelve `error_guards` guards a reader would now fail to find as `#[test]`
# functions.
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
# `claudine/lib/src/render/event_renderer/mod.rs` is the only file outside a test
# module, and only its `#[cfg(test)] mod tests` grew — no production behavior
# changed anywhere in this phase. The Cargo manifests gained
# `[package.metadata.ci.tests]` blocks, which are CI policy, not code.
packages_touched_during_phase_6:
    - claudine
    - claudine-cli
    - claudine-catalog-types
    - claudine-contract
    - claudine-gen
    - rendezvous-core
    - rendezvous-daemon
    - rendezvous-client
phase_7_status: >-
    complete — every sleep site dispositioned, two real defects closed (a
    process leak that played audio on the host, and Windows endpoint isolation
    that keyed on the pid alone); area gates green except the one
    host-condition L2 failure Phases 4-6 already disclosed
source_files_during_phase_7:
    - claudine/cli/tests/common/pty.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/cli_process_fixture.rs
    - claudine/cli/tests/sequence_overlay_pty.rs
    - claudine/cli/tests/level1_compose_autocomplete_failure_pty.rs
    - claudine/cli/tests/level1_inline_compose_mismatch_pty.rs
    - claudine/cli/tests/level2_schema_prompt_pty.rs
    - claudine/cli/tests/level2_provided_partial_file_pty.rs
    - claudine/cli/tests/level2_dry_run_pty.rs
    - claudine/cli/tests/level2_pty_tests.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/rendezvous/core/src/local_endpoint/test_support.rs
    - claudine/rendezvous/daemon/tests/pairing_and_sync.rs
docs_updated_during_phase_7:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
docs_created_during_phase_7: []
# Minimal drift repair only, in the two places this phase's changes falsified a
# written claim; Phase 10 still owns the full skill/doc review. Both skills
# described the L1 builder's defaults without the two `PLAYA_*` keys, and
# `rust-testing`'s time-and-ownership contract had no entry for the inverse
# case this phase found — the child blocking on a terminal query the harness
# never answered.
skills_files_updated_during_phase_7:
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/claudine/SKILL.md
# Test-only, except `rendezvous-core`'s `test_support` module — which is gated
# behind the `test-support` feature and so cannot reach a shipped binary. No
# production behavior changed.
packages_touched_during_phase_7:
    - claudine
    - claudine-cli
    - rendezvous-core
    - rendezvous-daemon
phase_8_status: >-
    complete — 55 local runs at two revisions, all green; paired
    candidate ÷ baseline 0.64–0.77 for `just test` in every pair; every
    eliminated-work claim has an lldb work counter or a shim sentinel behind
    it; local numbers are attribution only and the CI tranche (Phase 9) is
    still pending on the operator merge
# Measurement tooling and its recorded evidence. No Rust source was touched.
source_files_during_phase_8:
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement-runner.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/sentinels.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/plan.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/plan-series-2.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/cohorts.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/targets.json
docs_updated_during_phase_8:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
# Generated evidence: provenance, run manifests, gzipped recipe logs, the two
# reports, and the sentinel transcripts under `measurement/`.
docs_created_during_phase_8:
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/report.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/series-2/report.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/sentinels/summary.tsv
# Nothing this phase found falsified a written skill claim; Phase 10 owns the
# skill review and the three candidates are listed in `log.md` § Carried forward.
skills_files_updated_during_phase_8: []
# Measurement only: every package in the blast radius was *run*, none was
# edited. The baseline was measured in a detached worktree with its own build
# directory (`/tmp/rb-baseline-9fc5151a0`), left in place for Phases 9–10.
packages_touched_during_phase_8: []
phase_9_status: >-
    partial, human-gated — the predecessor merged to main (444213eb5) and its
    first CI run is stored, gated and green on all four legs (baseline 1 of
    3); consolidated local validation green (ci-local 147/147 over 73
    packages, check-windows exit 0, L2 236/237 with the known host-condition
    survivor); the gate learned platform exclusions and a within-environment
    comparison; candidate CI runs are 0 of 3 because merging main (13
    conflicts), committing and pushing are operator actions — handoff written
source_files_during_phase_9:
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/expectations.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/expectations.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution/budgets-pending.json
docs_updated_during_phase_9:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/README.md
    - claudine/fixes/_completed/2026-08-01-cli-slow-tests/deferred-performance.md
# Evidence: the two stored baseline runs (four downloaded artifact trees each,
# plus the gate's verbatim output) and the local-gate logs.
docs_created_during_phase_9:
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/README.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/pr-body.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/34173378609/junit-metrics.txt
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/34159725015/junit-metrics.txt
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/ci-local.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/check-windows.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/just-test-l2.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/just-test-l2-claudine-gen.log
# Nothing this phase found falsified a written skill claim; Phase 10 owns the
# skill sweep and the new candidates are in `log.md` § Phase 9 Carried forward.
skills_files_updated_during_phase_9: []
# Validation only: every workspace package was *run* by `ci-local`, none was
# edited. No Rust source changed in this phase.
packages_touched_during_phase_9: []
phase_10_status: >-
    closed for what this session can answer — results.md written with the
    three completion claims separate; seven deferrals given an owner document;
    three skill files corrected where a workflow claim was missing or false;
    two Windows-only unused-import warnings closed with cfg gates (cold
    check-windows: zero warnings); every local gate run or credited; AC1–AC5
    and AC7 verified, AC6 verified locally and pending on CI (candidate runs
    0 of 3, baseline 1 of 3, no budget derivable)
source_files_during_phase_10:
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/compose_caller_file_provenance.rs
docs_updated_during_phase_10:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/spec.md
# Evidence logs for the Phase 10 gate ledger sit beside Phase 9's under
# `candidate/local-gates/`.
docs_created_during_phase_10:
    - claudine/fixes/2026-09-07-faster-claudine-tests/results.md
    - claudine/fixes/_unscheduled/test-suite-residuals/spec.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-test.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-test-leaks-claudine.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-check-windows.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-check-windows-cold.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-lint.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-doctest.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-sniff-just-test.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-sniff-just-lint.log
# Only where a workflow claim was missing (the mingw cross-compile route, the
# measurement method) or false (the Windows console-control row).
skills_files_updated_during_phase_10:
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/rust-testing/test-suite-audits.md
    - .claude/skills/claudine/signal-handling.md
# Two test-file import gates; no library, CLI or production source changed.
packages_touched_during_phase_10:
    - claudine-cli
# Aggregates over Phases 1–10: every source file and every documentation or
# skill file created or updated by the plan.
source_code:
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/fixtures/nextest-l1-excerpt.xml
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/expectations.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory-reconciler.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory-reconciler.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/families.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/enumeration/captures.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution/launch-cwd-probe.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution/budgets-pending.json
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/common/host_tools.rs
    - claudine/cli/tests/cli_process_fixture.rs
    - claudine/cli/tests/spawn_site_guard.rs
    - claudine/cli/tests/system_prompt_perf_bench.rs
    - claudine/cli/tests/sequence_magic_reference.rs
    - claudine/cli/tests/loop_cli.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/cli/tests/contamination_probes.rs
    - claudine/cli/tests/compose_cli.rs
    - claudine/cli/tests/compose_interactive_timeout_cli.rs
    - claudine/cli/tests/compose_removed_validation_keys.rs
    - claudine/cli/tests/compose_schema_cli.rs
    - claudine/cli/tests/compose_system_prompt_lifetime.rs
    - claudine/cli/tests/compose_ttff_perf.rs
    - claudine/cli/tests/composition_outputs.rs
    - claudine/cli/tests/inline_compose_cli.rs
    - claudine/cli/tests/sequence_cli.rs
    - claudine/cli/tests/sequence_errors_cli.rs
    - claudine/cli/tests/sequence_groups.rs
    - claudine/cli/tests/sequence_jit.rs
    - claudine/cli/tests/sequence_prompt_property.rs
    - claudine/cli/tests/sequence_sources_cli.rs
    - claudine/cli/tests/wrap_sequence_composition.rs
    - claudine/cli/tests/context_command.rs
    - claudine/cli/tests/skills_integration.rs
    - claudine/cli/tests/errors_command.rs
    - claudine/cli/tests/effective_diagnostic_render.rs
    - claudine/cli/tests/completion_contract.rs
    - claudine/cli/tests/completion_perf.rs
    - claudine/cli/tests/completion_resolution_round_trip.rs
    - claudine/cli/tests/handle_deadline.rs
    - claudine/cli/tests/handle_blocking_output.rs
    - claudine/cli/tests/level1_structured_error_message.rs
    - claudine/cli/tests/protect_cli.rs
    - claudine/cli/tests/provider_error_finalize.rs
    - claudine/cli/tests/shipped_prompts.rs
    - claudine/cli/tests/wrap_sigint.rs
    - claudine/cli/tests/level1_compose_autocomplete_failure_pty.rs
    - claudine/cli/tests/level1_inline_compose_mismatch_pty.rs
    - claudine/cli/tests/sequence_overlay_pty.rs
    - claudine/cli/tests/wrap_ctrl_c_windows.rs
    - claudine/cli/tests/sequence_ctrl_c_windows.rs
    - .config/nextest.toml
    - claudine/justfile
    - claudine/rendezvous/justfile
    - claudine/catalog-types/Cargo.toml
    - claudine/contract/Cargo.toml
    - claudine/rendezvous/core/Cargo.toml
    - claudine/rendezvous/daemon/Cargo.toml
    - claudine/rendezvous/client/Cargo.toml
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/linking/paths.rs
    - claudine/lib/src/render/event_renderer/mod.rs
    - claudine/lib/src/stream/stderr/tests.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/wrap_opencode.rs
    - claudine/gen/tests/signals_validation.rs
    - claudine/cli/tests/common/pty.rs
    - claudine/cli/tests/level2_schema_prompt_pty.rs
    - claudine/cli/tests/level2_provided_partial_file_pty.rs
    - claudine/cli/tests/level2_dry_run_pty.rs
    - claudine/cli/tests/level2_pty_tests.rs
    - claudine/rendezvous/core/src/local_endpoint/test_support.rs
    - claudine/rendezvous/daemon/tests/pairing_and_sync.rs
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement.test.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement-runner.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/sentinels.ts
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/plan.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/plan-series-2.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/cohorts.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/targets.json
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/expectations.json
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/compose_caller_file_provenance.rs
documentation:
    - claudine/fixes/2026-09-07-faster-claudine-tests/plan.md
    - claudine/fixes/_completed/2026-08-01-cli-slow-tests/deferred-performance.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/log.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/README.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/attribution.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/report.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/series-2/report.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/measurement/sentinels/summary.tsv
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/README.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/pr-body.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/34173378609/junit-metrics.txt
    - claudine/fixes/2026-09-07-faster-claudine-tests/baseline/34159725015/junit-metrics.txt
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/ci-local.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/check-windows.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/just-test-l2.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/just-test-l2-claudine-gen.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/spec.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/results.md
    - claudine/fixes/_unscheduled/test-suite-residuals/spec.md
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-test.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-test-leaks-claudine.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-check-windows.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-check-windows-cold.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-lint.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-just-doctest.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-sniff-just-test.log
    - claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/phase10-sniff-just-lint.log
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/rust-testing/test-suite-audits.md
    - .claude/skills/claudine/signal-handling.md
packages_touched:
    - claudine-cli
    - claudine
    - claudine-catalog-types
    - claudine-contract
    - claudine-gen
    - rendezvous-core
    - rendezvous-daemon
    - rendezvous-client
---

# Execution plan — Faster Claudine tests through complete evaluation and explicit fixtures

Converts [spec.md](spec.md) into ten ordered phases. Required behaviors are
referenced as **RB1**–**RB5** and acceptance criteria as **AC1**–**AC7**, in
the order they appear in the spec.

## Grounding facts (verified on this branch, 2026-09-07)

These were checked against the working tree, not assumed. A phase that
contradicts one of them should stop and re-derive rather than proceed.

- **The predecessor has not landed.** `fix/cli-slow-tests` is 191 commits ahead
  of `main`; `claudine/fixes/2026-08-01-cli-slow-tests/` is tracked and its
  implementation (`cd3a28115`, `3e318802d`, `5b5b92bfb`) sits on this branch
  only. Its `log.md` records finding 2 (AC4 CI evidence) and the compile half of
  finding 3 as **deferred, closing by push**. The spec's sequencing rule
  therefore has teeth: Phase 1 is a real gate, not a formality.
- **The spawn burn-down is 170 sites in 36 allow-listed files**, split
  `outside this fix's scope` = 34 files / 168 sites and
  `needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither`
  = 2 files / 2 sites (`wrap_ctrl_c_windows.rs`, `sequence_ctrl_c_windows.rs`).
  Governed population 83 files. The isolation gate is at 0 escapes across 32
  governed files with an **empty** `ISOLATION_ALLOWLIST`.
- **Per-file raw-spawn census** (grep of the three detected forms; the guard's
  own count is authoritative and must be re-read in Phase 5):

  | file | sites | file | sites |
  |---|---|---|---|
  | `context_command.rs` | 24 | `sequence_groups.rs` | 4 |
  | `skills_integration.rs` | 22 | `sequence_magic_reference.rs` | 4 |
  | `compose_schema_cli.rs` | 20 | `sequence_overlay_pty.rs` | 4 |
  | `sequence_cli.rs` | 20 | `effective_diagnostic_render.rs` | 3 |
  | `loop_cli.rs` | 18 | `sequence_prompt_property.rs` | 3 |
  | `wrap_sequence_composition.rs` | 9 | `completion_contract.rs` | 2 |
  | `compose_interactive_timeout_cli.rs` | 5 | `completion_perf.rs` | 2 |
  | `errors_command.rs` | 5 | `composition_outputs.rs` | 2 |
  | `compose_cli.rs` | 4 | `handle_deadline.rs` | 2 |
  | | | `inline_compose_cli.rs` | 2 |
  | | | `level1_structured_error_message.rs` | 2 |

  Single-site files: `completion_resolution_round_trip.rs`,
  `compose_removed_validation_keys.rs`, `compose_system_prompt_lifetime.rs`,
  `compose_ttff_perf.rs`, `handle_blocking_output.rs`,
  `level1_compose_autocomplete_failure_pty.rs`,
  `level1_inline_compose_mismatch_pty.rs`, `protect_cli.rs`,
  `provider_error_finalize.rs`, `sequence_errors_cli.rs`, `sequence_jit.rs`,
  `sequence_sources_cli.rs`, `shipped_prompts.rs`, `wrap_sigint.rs`, plus the
  two Windows files.
- **The live-child cohort is six files, not two.** `.spawn()` on a raw
  `std::process::Command` appears in `wrap_sigint.rs`, `handle_deadline.rs`,
  `compose_ttff_perf.rs`, `wrap_ctrl_c_windows.rs`,
  `sequence_ctrl_c_windows.rs` and `spawn_inventory.rs`; the two
  `level1_*_pty.rs` binaries plus `sequence_overlay_pty.rs` drive
  `expectrl::session::OsSession` through `common/pty.rs`. The builder's
  `build()` returns `assert_cmd::Command`, which has no `spawn` and no way back
  to the inner `std::process::Command`.
- **The environment policy is applied inline in `ClaudineCommandBuilder::build`**
  (`common/mod.rs:453`) — `env_clear` + `restore_windows_console_variables`,
  then `scrub_inherited_environment`, then eleven `.env`/`.env_remove` calls and
  `path_value()`. It is not extractable as-is: it is written against
  `assert_cmd::Command` receivers.
- **Test population is ≈7,300 attributes** across the eight packages
  (`lib` 4086, `cli` 2728 of which 1036 in `cli/tests`, `rendezvous` 296,
  `gen` 159, `contract` 52, `catalog-types` 21). A one-row-per-test inventory is
  not achievable by hand; RB1's "exactly one row **or explicitly enumerated
  family**" has to be mechanically reconciled.
- **Runner overrides in force** (`.config/nextest.toml`): default profile has 9
  per-test `slow-timeout` overrides plus the `package(claudine-cli) &
  test(/level2_/)` blanket; the CI profile has 9 more plus three `test-group`
  bindings (`claudine-l1` max-threads 4, `claudine-cli-ci-l1` max-threads 1,
  `sniff-windows-l1`). Claudine-scoped ones the spec's AC5 governs:
  `compose_loop_rate_limit_pause_waits_then_continues`,
  `agents_and_commands_route_to_empty_state_messages`,
  `composition::loop_engine::tests::seeded_loop_repro_runs_to_completion_with_live_derived_variable`,
  `every_catalog_variable_survives_ambient_options`,
  `exhausted_remediation_fails_finalize_and_preserves_findings`,
  `context_reports_preserve_all_columns_at_minimum_supported_width`,
  `compose_perf_stdout_matches_non_perf`,
  `inline_compose_perf_stdout_matches_non_perf`, the two `claudine-*-l1` test
  groups, and the L2 blanket.
- **Canonical-recipe gaps already visible.** `claudine/justfile`'s `test-real`
  shells out to `cargo test`, not nextest — the only recipe in the area that
  does. `claudine/rendezvous/justfile` defines 8 recipes of the canonical 12
  (no `sanity`, `test-l3`, `test-browser`, `test-real`, `doctest`, `bench`,
  `coverage`, `fuzz`, `all`). `claudine-contract`, `claudine-catalog-types`,
  and all three rendezvous crates carry **no** `[package.metadata.ci.tests]`,
  so their CI tier/feature route is defaulted rather than declared.
- **Bench entry points exist**: `claudine/lib/benches/` holds `claude_parse`,
  `opencode_parse`, `pre_flight_checks`, `prompt_preparation`,
  `runtime_hot_paths`. No `fuzz/` directory in the area.
- **Known-open residual from the predecessor**: `COLUMNS=44 just test-cli`
  reddens `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub`
  and `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`.
  Both are `SPAWN_ALLOWLIST` files; they close by migration (Phase 5A), which is
  exactly what AC3 asks for.
- **Local reference point, not a target**: the predecessor's final run recorded
  `just test-cli` at 2411 passed / 10 skipped in ~13.5 s and `just test-l2` at
  230 passed in ~53 s on a 16-core Mac.

## Assumptions and stated decisions

1. **Phase 1 is human-gated.** Committing, pushing, and merging are separate
   operator actions per `CLAUDE.md`. Phases 2–3 are document-only and may run
   while Phase 1's CI window is open; **no code change from Phases 4–8 may be
   committed until Phase 1's checkpoint passes**, because the same runs are the
   predecessor's evidence and this fix's baseline.
2. **The inventory is family-based with a mechanical completeness proof.** A
   TypeScript reconciler (house convention — the predecessor shipped
   `junit-metrics.ts`) joins `cargo nextest list` output against the inventory's
   declared families and fails on any unassigned or doubly-assigned identity.
   Hand enumeration of 7,300 identities is not a credible deliverable.
3. **The shared environment policy becomes data, not a receiver method.** The
   only way `assert_cmd::Command` and `std::process::Command` can share "one
   shared implementation" is a computed description — clear flag, ordered
   removes, ordered sets, `current_dir` — applied by two thin adapters. Both
   adapters are three lines; the policy is computed once. This is the design
   Phase 4 implements, and the drift test asserts the two adapters produce the
   same effective environment.
4. **`--dry-run`-style scope discipline on assertions.** Assertion repair and
   test-population changes land in their own commits, separate from fixture
   migration, so a reviewer can read either without the other (spec RB3, and
   `CLAUDE.md` § Scope discipline).
5. **`results.md` and `inventory.md` live in this fix directory.** `inventory.md`
   carries the baseline table, the numeric budgets beside it, and the
   dispositions; `results.md` carries measurements, coverage deltas, residual
   findings and the three separate completion claims.
6. **No new override, retry, tier change, or disabled assertion** may be
   introduced at any point (AC5, AC7). `git diff main -- .config/nextest.toml`
   must be inspected at every checkpoint.

---

## Phase 1 — Land the predecessor and open the attribution window

**Gate. Nothing from Phases 4–10 may be committed until this phase's checkpoint
passes.** The spec fixes this sequencing: interleaving would make the
predecessor's deferred evidence measure this fix's changes.

- [x] Confirm `fix/cli-slow-tests` still carries the predecessor's three
      implementation commits and that `git diff main -- .config/nextest.toml`
      is empty.
- [x] Run the local gates from the `claudine` package area and record the
      output verbatim: `just lint`, `just test`, `just test-cli`, `just test-l2`.
      All four green; tests run before lint to dodge the stale-binary hazard.
      Verbatim output in `baseline/local-gates/`.
- [x] Run `just check-windows` (mingw `x86_64-pc-windows-gnu`, `--tests`) if the
      toolchain is present on the host; record "toolchain absent" explicitly if
      not. This is the predecessor's finding-3 compile half and this fix's AC6.
      Toolchain present; exit 0 in 1 m 37 s after `rustup target add`.
- [x] Run `just ci-local --lint-only` then `just ci-local` at the repo root for
      the branch's affected scope, per the repo's pre-push discipline.
      Scope 27 packages (79 files changed vs `origin/main` a9e88c069);
      lint-only 28/28 gates, full run 55/55 gates, both exit 0.
- [ ] Hand off to the operator for commit, push, and merge of the predecessor
      to `main`. Record the merge SHA.
      **BLOCKED — operator action.** Commits must be OpenPGP-signed and this
      session is non-interactive, so a signed commit would hang rather than
      fail. Nothing staged or committed. Handoff commands in `log.md`.
- [ ] Collect **three consecutive green CI runs** on `main` after the merge,
      one artifact set per configured leg: `ubuntu-latest`, `macos-latest`,
      `windows-latest`, `wsl2-ubuntu`. Record every intervening failed attempt
      with its cause — selecting only the successful attempts is disallowed.
      **BLOCKED** by the handoff above: no post-merge run exists to read. All
      four legs are named *pending*, not assumed.
- [ ] Store the JUnit artifacts under
      `claudine/fixes/2026-09-07-faster-claudine-tests/baseline/<run-id>/`,
      keeping build time, runner elapsed time, and summed test duration as
      three separate columns.
      **BLOCKED** by the collection above. The layout, the collection recipe and
      the three-column gate that reads them are in place and tested
      (`baseline/README.md`, `baseline/expectations.json`, `junit-metrics.ts`);
      only the artifacts are missing.
- [x] Extend or fork the predecessor's `junit-metrics.ts` into this fix
      directory so it reads the four-leg baseline and emits the table that
      `inventory.md` will hold. It must reject malformed reports, missing
      expected artifacts or tests, duplicate identities, invalid durations, and
      failed runs — a script that prints a miss and exits 0 is not a gate.
- [x] Record in the predecessor's `deferred-performance.md` that its finding 2
      and the finding-3 compile half are now closed (or, if a leg is red, that
      they are not). Recorded: finding 2 **open** (no merge, so no run); the
      finding-3 compile half **closed** for `x86_64-pc-windows-gnu`, with the
      MSVC surface and all runtime behavior still open.

**Validation checkpoint 1** — three green runs on four legs exist, their
artifacts are stored, `junit-metrics.ts` reproduces the baseline table from
them, and the predecessor's two deferrals are resolved in writing. Blocked
legs are named as pending rather than assumed.

---

## Phase 2 — Reconciled inventory (RB1, AC1) ‖ runs during Phase 1's CI window

Document-only. No source file changes. Produces `inventory.md`.

- [x] Build the enumeration substrate: for each of the eight packages, capture
      `cargo nextest list --message-format json` under every feature selection
      that CI or a canonical recipe uses — bare, `daemon-tests`,
      `terminal-tests`, `real-tests` — and under each platform the host can
      enumerate. Record the command, revision, toolchain, and features next to
      each capture.
      16 captures in `enumeration/`, declared in `enumeration/captures.json`
      with command/revision/toolchain/nextest/platform; stderr kept as
      `<label>.err`. 7,400 identities across 163 build targets. One platform
      only — this host is `aarch64-apple-darwin`; Windows/Linux enumeration is
      the pending CI tranche's.
- [x] Capture the source-side population separately (attribute scan over
      `#[test]`, `#[tokio::test]`, `#[rstest]`, plus `#[ignore]` and `#[cfg]`
      gates) and diff it against the runner discovery. Every source test that
      the runner never lists is a **cfg/feature exclusion row**, listed
      separately from executed tests with its reason and actual execution route.
      7,455 source attributes; 55 source-only, all platform gates, each with a
      gate and a route. The scan blanks string/char literals and comments (so
      `test_placement.rs`'s raw-string fixture is not counted) and resolves the
      one `macro_rules!` test template to its 7 invocation sites.
- [x] Inventory the doctest population (`just doctest`) and the five
      `lib/benches` entry points; confirm the area has no fuzz targets and
      record that as a finding rather than an omission.
      32 doctests (20 pass / 7 ignored / 3 compile-fail / 2 in contract).
      **Four of the five bench entry points are zero-byte files** — only
      `runtime_hot_paths.rs` holds benchmarks. No fuzz targets.
- [x] Inventory the rendezvous family explicitly through its own recipes
      (`just test` inside `claudine/rendezvous`, or `just test-rendezvous` from
      the parent area) and record that the parent area's `just test` covers only
      the five claudine crates.
      Run and logged at `enumeration/recipes/just-test-rendezvous.log`.
      `rz-daemon-unit` is 107.75 s summed over 153 identities — the highest
      per-test unit cost in the eight packages, in a crate `just test` never
      reaches.
- [x] Inventory shared fixture machinery as first-class rows: `common/mod.rs`
      (builder + scrub + containment guard), `common/wrap.rs`,
      `common/pty.rs`, `common/completion.rs`, `common/source_scan.rs`,
      `cli/tests/error_guards/`, and the `_ensure-md` area pre-build.
      Nine components, with usage counts and `common/mod.rs`'s ordering
      contract recorded.
- [x] Write the per-family rows. Each records: purpose; assertion quality
      (does it distinguish a plausible failure?); shared helpers; required
      effects; CWD / home / cache / environment / tool dependencies; timing
      floor; runner overrides in force; resource ownership; tier, features and
      platforms; canonical recipe; observed cost with provenance; disposition.
- [x] Enumerate family membership explicitly. A family row is valid only when
      its members are listed and share the same setup and proof; anything that
      differs gets its own row.
      50 families. Eleven carve-outs exist because their setup or proof differs
      from the parent: six runner-override targets, `lib-unit-task-shell`,
      `lib-unit-atomic-config`, `lib-unit-model-catalog`,
      `lib-unit-clock-streams`, `cli-unit-child-exec`.
- [x] Write the runner-override census as its own table: all 18 per-test
      `slow-timeout` overrides plus the three `test-group` bindings and the L2
      blanket, each marked *justified* (with the contract its floor expresses)
      or *remove with the cost it hides* (AC5).
      **Corrected: the file holds 17 per-test entries, not 18** (9 default +
      8 CI), inside 24 override blocks. Eleven are Claudine-scoped and carry a
      verdict; seven belong to darkmatter/biscuit-terminal/sniff and are named
      as out of scope — including `every_catalog_variable_survives_ambient_options`,
      which the plan's grounding fact wrongly listed as Claudine's.
      **Two overrides are dead filters**: `test(=composition::loop_engine::…)`
      in both profiles has bound to nothing since the module was renamed
      `loop_engine` → `looping::engine`.
- [x] Record the recipe/metadata reconciliation findings: `test-real` bypassing
      nextest, rendezvous's 8-of-12 canonical recipes, and the five crates with
      no `[package.metadata.ci.tests]`.
- [x] Implement the completeness reconciler (TypeScript, in this fix directory).
      It reads the nextest listings plus the inventory's declared families and
      **fails** on any identity assigned to zero or more than one row. Run it
      and paste its output into `inventory.md`.
      `inventory-reconciler.ts` + `families.json`; 70 tests in
      `inventory-reconciler.test.ts`, all passing; five neuter transcripts in
      `log.md`. It also fails on inventory count drift, an empty or
      unrecognized disposition, a stale family, an undeclared exclusion and a
      stale exclusion. `GATE EXIT=0`.
- [x] State the disposition of every row as one of: satisfactory, remediation in
      this fix, or linked follow-up naming the unmet requirement and the reason
      for deferral. No row may be dispositioned by timing threshold.
      26 satisfactory, 21 remediation in this fix, 3 linked follow-ups (L3 and
      the two real tiers, all pending rather than assumed). The gate rejects a
      disposition outside those three.

**Validation checkpoint 2** — the reconciler exits 0; the inventory covers all
eight packages, all tiers, doctests, benches, excluded/ignored tests and shared
fixtures; every row has a disposition; the override census is complete.

---

## Phase 3 — Attribution and ratified budgets (RB5 first half; resolves the spec's four draft decisions)

Document-only. Depends on Phases 1 and 2.

- [x] Attribute cost by family against the Phase 1 baseline, keeping build,
      elapsed, and summed-duration columns separate. Answer draft decision 1
      — *which non-spawn families account for the remaining execution cost* —
      with numbers, not intuition.
      Done against Phase 1's **local** baseline (the CI half is still blocked),
      through `attribution.ts`, which joins run logs to `families.json` with the
      reconciler's own matcher. The decisive finding is that CI runs
      `claudine-cli` L1 at `max-threads = 1`, so summed duration — not local
      elapsed — is the leg's floor.
- [x] Measure the source-scan families (`error_guards`, `test_placement`,
      `dispatch_inventory`, `spawn_inventory`, `spawn_site_guard`) individually.
      Nextest runs each in its own process, so a process-local cache shares
      nothing across cases; record the per-binary scan cost and the number of
      processes that repeat it. Answer draft decision 2 — *which scans can share
      work without losing independent failure detail*.
      Seven binaries, three repetitions each, plus a shared-process
      counterfactual: `error_guards`' eighteen cases cost 20.23 s across
      eighteen processes and 1.61 s in one. Consolidate those twelve corpus
      cases and nothing else.
- [x] Measure the context/render families (`context_command`,
      `composition_seams`, `every_catalog_variable_survives_ambient_options`,
      the loop pause tests) and the corpus loaders (`shipped_prompts`,
      `shipped_prompt_contract`, `shipped_prompt_route_drift`).
      Done. `every_catalog_variable_survives_ambient_options` is darkmatter's,
      as Phase 2 already corrected, and is out of scope. `context_command`'s
      cost is the launch CWD: 34× per process, 116× at nextest's concurrency.
- [x] Enumerate the live-child cohort and decide, per file, whether it still
      needs an exception once Phase 4 ships a raw-command path. Answer draft
      decision 3 — *which technical exceptions remain necessary*.
      **None.** The cohort is five files plus three PTY binaries, not six —
      `spawn_inventory.rs`'s `.spawn()` sites are fixture text.
- [ ] Ratify per-family numeric budgets **on the existing CI runners**, derived
      from the baseline and written into `inventory.md` **beside the baseline
      table**, so budget review and evidence review are one act. Answer draft
      decision 4. Local timing may attribute cost; it may not set a target.
      **BLOCKED** on Phase 1's CI tranche: with no baseline there is nothing to
      derive from, and deriving from the local tables is what this bullet
      forbids. Instead the refusal is mechanical (`deriveBudgets` exits 1 on
      non-CI provenance, a short run count, or a leg with no measurements) and
      the ratification procedure — worst-of-three per leg, ×1.25 headroom, legs
      never merged — is fixed in `inventory.md` § Budgets before the numbers
      exist.
- [x] Record the timeout/clock semantics inherited from the implemented
      [startup-stall fix](../_completed/2026-08-31-silent-success-and-startup-stall/spec.md):
      every floor and budget recorded here is derived under its spawn-fallback
      silence clock, not the pre-fix first-event grace.
      Recorded, with the five families whose floors *are* that contract and the
      production defaults they parameterise (`interval` 5 s, `kill_grace` 10 s).
      The plan's original link was stale — the fix is archived under
      `_completed/`.
- [x] For any production defect surfaced during attribution, open a linked
      issue/spec with the evidence rather than fixing it here.
      None met the bar. The one candidate — `claudine context --values` at
      672 ms from the monorepo root against 20 ms outside a repository — renders
      733 rows of real repository content on the expensive side, so it is work
      proportional to output rather than a defect. Recorded with its evidence in
      `attribution.md` so a later phase can reopen it with a work counter.

**Validation checkpoint 3** — every draft decision has a written answer backed
by a measurement; budgets are in `inventory.md` next to the baseline; no budget
was derived from a local run alone.

---

## Phase 4 — One environment policy, two command surfaces (RB2 infrastructure)

First code phase. Requires checkpoint 1. Everything downstream of the builder
depends on this, so it lands alone.

- [x] Extract the inline policy in `ClaudineCommandBuilder::build` into a
      computed description — clear flag, Windows console restores, ordered
      removes (`CLAUDINE_*` by prefix, `GIT_PLUMBING_VARS`, the three render
      inputs, `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME`), ordered sets (home
      variables, `PATH`, `CLAUDINE_RENDEZVOUS_REPORT`, `NO_COLOR`), and
      `current_dir`. Preserve the existing ordering contract exactly: scrub
      before defaults, so `CLAUDINE_RENDEZVOUS_REPORT` survives its own
      namespace sweep, and a per-key `.env` after `build()` still wins.
- [x] Add two thin adapters that apply that description — one to
      `assert_cmd::Command`, one to `std::process::Command`. `build()` keeps its
      signature and behavior; the new `command_std()` / `command_builder()
      .build_std()` yields a `std::process::Command` carrying the same policy.
- [x] Add a drift test proving the two surfaces produce the same effective
      environment: run the recording stub through both paths and compare the
      captured key/value sets, including the Windows arm via `cfg!(windows)`
      rather than `#[cfg]` so both arms compile everywhere.
- [x] Keep the named escapes working on the raw path: `fake_only_path()`,
      `host_path()`, `ambient_context(dir)`, `inherit_no_env()`, and the
      `checkout_containment_error` precondition — including after
      canonicalization, so a fixture root can never sit inside the checkout.
- [x] Teach `spawn_site_guard.rs`'s **spawn** gate that the builder's raw path
      is a sanctioned form, and that a raw `std::process::Command::new(
      claudine_bin())` outside it is still a violation. Add detector unit tests
      for both, including negatives (`bin_exe!("md")`, prose, string literals).
- [x] Teach the **isolation** gate the resulting command forms: a
      `.current_dir` / `.env("PATH", …)` / `.env_remove("PATH")` /
      `.env_clear()` / `augmented_path` on a raw fixture command is the same
      escape it already flags on the `assert_cmd` one.
- [x] Widen the isolation gate's governed population from 32 files toward the
      rest of the L1 suite, as each file leaves `SPAWN_ALLOWLIST` in Phase 5.
      Where a governed file legitimately targets a *different* command
      (parent-side `git`, `rustc`, `md`), resolve it with an
      `ISOLATION_ALLOWLIST` entry naming that command — **never** by weakening
      the detector or dropping stale-entry failure.
      The widening needs no edit — deleting a spawn entry is what performs it —
      so what this phase adds is the proof:
      `deleting_a_spawn_entry_is_what_widens_the_isolation_population`.
      **The plan's population numbers are stale**: the gate reads 89 governed
      files / 172 spawn sites and 37 isolation-governed files on this tree, not
      83 / 170 / 32. `ISOLATION_ALLOWLIST` stays empty — no governed file
      targets a non-claudine command today.
- [x] Harden the helper commands too: audit every parent-side
      `Command::new("git")` / `rustc` / `md` invocation in `common/` and the
      fixtures for inherited Git plumbing; isolating only the claudine child
      does not protect those.
      Six `git` sites hardened through the new `common::helper_command`; the
      four `rustc` sites read no `GIT_*` and are left alone; there is no
      parent-side `md` spawn in the L1 suite. Details in `log.md`.
- [x] Prove non-vacuity for each new or changed guard arm: apply a neuter, show
      the named failure, restore, and `diff` the file back to identical.
      Transcribe into the log.
- [x] Compile-verify the Windows-only arms with `just check-windows` where the
      mingw toolchain is present; otherwise name `windows-latest` as the
      authority and record the check as pending.

**Validation checkpoint 4** — `just test-cli` and `just lint` green from the
`claudine` area; `just test-l2` green (Phase 4 touches `common/`, which every
L2 binary compiles); both guards' censuses printed and unchanged except for the
new sanctioned form; `git diff main -- .config/nextest.toml` empty.

**Passed, with one host-condition failure disclosed.** `just test-cli`
(2498 passed / 10 skipped), `just test` (6870 passed / 11 skipped), `just lint`
and `just check-windows` all exit 0; `git diff main -- .config/nextest.toml` is
empty; both censuses are unchanged (172 spawn sites in 36 allow-listed files,
zero isolation escapes). `just test-l2` is 236/237: the survivor is
`level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`, which fails on
this host because Atuin's first-run prompt sits in the spawned WezTerm pane and
swallows the exit marker — reproduced in isolation, unrelated to this diff, and
recorded in `log.md` with the captured frame.

**Sequencing note.** Phase 1's checkpoint is still blocked on the operator
merge, so the plan's rule that no Phase 4–10 *commit* may land first is intact:
this phase is implemented and verified in the working tree, and nothing was
staged or committed.

---

## Phase 5 — L1 spawn burn-down (RB2, AC2)

Four batches. **5A, 5B and 5C are mutually parallelizable** — they touch
disjoint file sets and each ends by deleting its own `SPAWN_ALLOWLIST` entries.
**5D depends on Phase 4's raw-command path** and is the only batch that may
need a new allow-list reason. Each batch ends green before the next merges.

### Phase 5A — compose / composition family (‖ with 5B, 5C)

- [x] Migrate `compose_cli.rs` (4), `compose_schema_cli.rs` (20),
      `compose_removed_validation_keys.rs` (1),
      `compose_system_prompt_lifetime.rs` (1), `composition_outputs.rs` (2),
      `inline_compose_cli.rs` (2) to `CliProcessFixture::command()` /
      `command_builder()`, each escape carrying a call-site comment naming the
      tool or proof it needs.
      **The plan's batch lists omitted `compose_interactive_timeout_cli.rs`
      (5 sites)** — it is on the allow-list and appears in no batch, so 5E's
      "zero generic exemptions" could not have been reached. Migrated here with
      the rest of the compose family.
      One test needed more than a call-site swap:
      `compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area`
      launched from the rusty-biscuit checkout itself. It now copies the two
      shipped artifacts it exercises (`prompts/plan.md` and the
      `shipped_plan_route` spec) into the fixture at their checkout-relative
      paths, so the relative references it proves are preserved, and pins each
      of its two launch directories with `ambient_context`.
- [x] Delete those files' `SPAWN_ALLOWLIST` entries; confirm the stale-entry arm
      would fire if an entry were left behind.
- [x] Re-run the predecessor's inherited-width probe: `COLUMNS=44 just test-cli`
      must no longer redden
      `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub`
      or `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`.
      This is AC3's named case.
      Both pass under `COLUMNS=44`, and both **fail** under `COLUMNS=44` when the
      pre-migration file is restored in place — so the probe is non-vacuous.

### Phase 5B — sequence / loop family (‖ with 5A, 5C)

- [x] Migrate `sequence_cli.rs` (20), `loop_cli.rs` (18),
      `wrap_sequence_composition.rs` (9), `sequence_groups.rs` (4),
      `sequence_magic_reference.rs` (4), `sequence_prompt_property.rs` (3),
      `sequence_errors_cli.rs` (1), `sequence_jit.rs` (1),
      `sequence_sources_cli.rs` (1).
      Three stubs wrote state under `$HOME` that the assertion then read back
      from the workspace root — the same directory before the migration, two
      after it. Each now names the file it means: `sequence_jit`'s
      `::shell wc -l < log.txt` and its document-rewrite stub take the launch
      directory's path, and the `launches.txt` witnesses read from
      `fixture.home()`.
- [x] `sequence_cli.rs` alone carries ~20 `.env("PATH", augmented_path(…))`
      pairs. Replace each with the named escape that expresses its actual need
      (`host_path()` where a real tool is the subject, `fake_only_path()` where
      absence is the assertion, default otherwise) — do not carry the raw pair
      across.
      Every one of them turned out to be the *default*: the tool each names is a
      stub the test writes into the fixture `bin`, so none needed `host_path()`.
      `wrap_sequence_composition.rs` is where `fake_only_path()` was owed — four
      sites whose assertion is that only an `installed` provider counts.
- [x] Delete the batch's `SPAWN_ALLOWLIST` entries.
      Two guard tests keyed off `sequence_cli.rs` still being allow-listed.
      `governs_isolation` is now parameterized by the allow-list
      (`governs_isolation_against`), so the widening rule is proven against a
      synthetic before/after pair and stays provable once the list is empty —
      an assertion written against an empty live list can only say "nothing is
      exempt", which a broken predicate says too.

### Phase 5C — context / errors / completion / resources family (‖ with 5A, 5B)

- [x] Migrate `context_command.rs` (24), `skills_integration.rs` (22),
      `errors_command.rs` (5), `effective_diagnostic_render.rs` (3),
      `completion_contract.rs` (2), `completion_perf.rs` (2),
      `handle_deadline.rs` (2), `level1_structured_error_message.rs` (2),
      `completion_resolution_round_trip.rs` (1), `compose_ttff_perf.rs` (1),
      `handle_blocking_output.rs` (1), `protect_cli.rs` (1),
      `provider_error_finalize.rs` (1), `shipped_prompts.rs` (1).
      `context_command.rs` was the launch-CWD cost Phase 3 attributed: 24 of its
      27 processes launched from the rusty-biscuit checkout and rendered its real
      contents. Each now launches from an empty `git init` the fixture built,
      which proves the same contract (`ctx.repo_root` resolves, no row is
      `null`) against topology the test wrote. `errors_command.rs` needed no
      repository at all — `claudine errors` reads only the code registry.
      `shipped_prompts.rs` keeps the *shipped* `prompts/implement.md` as its
      subject and isolates only the repository it is launched from.
- [x] `handle_deadline.rs` and `compose_ttff_perf.rs` hold a live child — route
      them through Phase 4's raw-command path rather than `assert_cmd`.
      `completion_perf.rs`'s two sites turned out to be the same shape (they
      reached for `cargo_bin(…).get_program()` to hand-roll a raw command, one of
      them for an `expectrl` session) and now use `command_std()` too.
- [x] Delete the batch's `SPAWN_ALLOWLIST` entries.

### Phase 5D — live-child cohort (depends on Phase 4)

- [x] Route `wrap_sigint.rs` (1) through the raw-command path; the call site
      keeps only its subject — signal delivery, output draining, child reaping.
- [x] Route `common/pty.rs`'s session construction through the raw-command path
      so `level1_compose_autocomplete_failure_pty.rs`,
      `level1_inline_compose_mismatch_pty.rs` and `sequence_overlay_pty.rs`
      inherit the policy; verify the `#[cfg(unix)]` gate on `mod pty;` still
      holds and that `expectrl` receives a configured `std::process::Command`.
      **`common/pty.rs` builds no command** — the plan's grounding fact is off by
      one file. Each of the three binaries built its own, so each was migrated
      directly to `command_std()`; `common/pty.rs` is untouched and its
      `#[cfg(unix)]` gate is unchanged.
      Two findings fell out. `level1_inline_compose_mismatch_pty.rs` had been
      passing only because it inherited the developer's own
      `~/.claudine/config.json` — under the fixture home the first-run wizard
      intercepted the PTY, so it now seeds its own config.
      `sequence_overlay_pty.rs`'s stdout-redirect test drives `/bin/sh -c`, whose
      grandchild is claudine, which `build_std()` cannot express; the builder
      gained `apply_policy_to(&mut Command)` for exactly that shape, and the
      claudine path comes from `command_std().get_program()` rather than a
      hand-rolled `cargo_bin`.
- [x] Route `wrap_ctrl_c_windows.rs` and `sequence_ctrl_c_windows.rs` through
      the raw-command path. The call site keeps `CREATE_NEW_PROCESS_GROUP`, the
      targeted console signal, the readiness poll, and the reaping — nothing
      else. **Do not** move these to L3 and **do not** drop Windows coverage.
      This closes the `NEEDS_LIVE_CHILD` reason and the environment hazard the
      predecessor's log recorded but did not fix (an inherited `CLAUDINE_TIMEOUT`
      producing a silent false pass).
      Both stay L1 and stay `#[cfg(windows)]`. Neither now builds its own `PATH`
      or `HOME`; the `CLAUDINE_*` scrub the fixture applies is what closes the
      false-pass hazard, and each call site says so.
- [x] Delete `NEEDS_LIVE_CHILD` and its two entries once no site cites it.
- [x] Compile-verify with `just check-windows` where the mingw toolchain is
      present; name `windows-latest` as the authority otherwise.
      Toolchain present; `just check-windows` exits 0 for
      `x86_64-pc-windows-gnu --tests`. Runtime behavior on Windows stays
      `windows-latest`'s to confirm.

### Phase 5E — burn-down closure

- [x] Assert `SPAWN_ALLOWLIST` contains **zero** generic
      `outside this fix's scope` entries (AC2). Any survivor carries a specific
      technical necessity **and** equivalent isolation proof, written at the
      entry.
      The list is **empty**, not merely free of generic entries.
- [x] Confirm the isolation gate's governed population now covers the migrated
      files and that `ISOLATION_ALLOWLIST` entries, if any, each name the
      non-claudine command their site targets.
      37 governed files at Phase 4 → **74** now, with zero escapes and an
      `ISOLATION_ALLOWLIST` that is still empty. Two `.current_dir` sites the
      widening surfaced were on parent-side `git`, and both were resolved by
      routing through `common::init_git_repo` /
      `CliProcessFixture::initialize_repository` — which own the `.current_dir`
      themselves — rather than by an allow-list entry.
- [x] Keep the negative guard tests and the stale-exemption failure arms; add a
      neuter transcript for any arm whose behavior changed.
      Two arms changed, both because the burn-down reaching zero removed the
      live data they leaned on:
      `deleting_a_spawn_entry_is_what_widens_the_isolation_population` now runs
      against a synthetic before/after allow-list through
      `governs_isolation_against`, and
      `an_empty_allowlist_leaves_every_live_site_unlisted` became
      `the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`,
      which asserts the population size, the zero census, and that the same
      detector still finds a planted raw spawn. Transcripts in `log.md`.
- [x] Add the contamination probes AC3 requires, using **disposable state only**
      — never an edit to the real checkout or the user's configuration:
      application variables (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`), Git
      plumbing (`GIT_DIR`/`GIT_WORK_TREE` pointed at a throwaway repo),
      home/cache relocation, `PATH`, inherited width/color (`COLUMNS=44`,
      `FORCE_COLOR=1`), and a checkout-ancestor `TMPDIR`. Each probe must leave
      unrelated test results unchanged.
      `cli/tests/contamination_probes.rs`, eight probes. Each exports its family
      and then asserts an ordinary `claudine compose` run is unchanged in exit
      status, composed stdout, the `success` line, that line's styling, and the
      home the system prompt was read from.
      Two findings the probes forced, both recorded in `log.md`: **`GIT_DIR` /
      `GIT_WORK_TREE` do not move `ctx.repo_root`** (claudine's discovery walks
      the filesystem), so the Git family's real hazard is the *parent-side*
      helper, which is what the probe exercises; and **a single `compose` run
      consults no `CLAUDINE_*` variable observably**, so that family needed
      `observed_iteration_cap()` — a `loop.max` document whose reported cap a
      leaked `CLAUDINE_MAX_ITERATIONS` replaces.
      Four neuters, each firing exactly the probes it should: dropping the
      inherited scrub (render-width + `CLAUDINE_*`), dropping the `HOME` set
      (home + everything downstream of it), replacing the fixture `PATH` with the
      parent's (`PATH` + downstream), and dropping `helper_command`'s Git scrub
      (Git plumbing). Every file diffed back identical afterwards.

**Validation checkpoint 5** — `just test-cli`, `just test`, `just lint` and
`just test-l2` green from the `claudine` area; the burn-down artifact
(`$STAGE/spawn-site-burn-down.jsonl`) shows zero generic exemptions; every
contamination probe passes; test count reconciles (additions and removals
reported separately, never netted).

**Passed, with the same host-condition L2 failure Phase 4 disclosed.**
`just test-cli` 2506 passed / 10 skipped, `just test` 6878 passed / 11 skipped,
`just lint` and `just check-windows` exit 0. The burn-down artifact reads
`{"kind":"total","gate":"spawn","files":0,"sites":0,"scanned_sites":0,"governed_files":90}`
and the isolation artifact `…"scanned_sites":0,"governed_files":74`.
`git diff main -- .config/nextest.toml` is empty.

`just test-l2` is 236/237: the survivor is
`level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`, reproduced in
isolation with Atuin's "Atuin AI is not yet configured" prompt visible in the
captured pane, swallowing the exit marker. Same host condition as Phase 4, in a
`level2_*` file this phase never touched.

**Test count reconciliation** (not netted). Additions: 8 —
`contamination_probes.rs`'s eight probes. Removals: 0. Renames (neither an
addition nor a removal): 1 — the spawn gate's non-vacuity test. `just test-cli`
2498 → 2506 and `just test` 6870 → 6878 both account for exactly the 8
additions.

**Sequencing note.** Phase 1's checkpoint is still blocked on the operator
merge, so nothing here is staged or committed.

---

## Phase 6 — Non-spawn cost and test quality (RB3) ‖ partially with Phase 5

Touches different files from Phase 5 for the most part; the loop/context items
must land **after** 5A/5B if they touch the same binaries.

- [x] Inspect every consumer of the expensive context/discovery helpers
      (`ComposeContext::capture`, `LaunchContext`, repository discovery).
      Substitute synthetic context where discovery is incidental; **retain real
      discovery against test-built repositories where it is the assertion's
      subject**.
      The dominant consumer is not `ComposeContext::capture` — the composition
      fixtures already hand it a demand-driven context. It is
      `resolve_composition_source`, whose `capture_file_resolution_context()`
      walks the **process CWD's** repository topology before resolution begins:
      0.23 s per call against 0.01 s from outside a repository, once per test
      process. Every fixture document is an absolute path in its own `TempDir`,
      and `ResolvedCompositionSource` carries no context forward, so that walk
      decides nothing. One shared seam — `composition::resolve_fixture_source`,
      `#[cfg(test)]`, anchored on the document's own directory and panicking on
      a relative path — now serves the schema, preflight and task fixtures.
      `linking::paths` was the second consumer: eight tests of the
      capability-derived path table each paid `ProviderSkillPaths::new()`'s
      ambient `resolve_repo_root`. They now build the table from synthetic
      roots; `new_roots_the_table_at_the_process_home_and_the_resolved_repository`
      is the one case that keeps the real discovery, because the wiring is its
      subject.
      Kept on real discovery against test-built repositories: the whole
      `invocation_context` family (work-counter tests over their own repos),
      `cross_repo_task_nested_reference_uses_its_own_repository_context`,
      `resolve_repo_root_*`, and the `make_source_in` / `serial(schema_validation_cwd)`
      tests, whose subject is precisely independence from the process CWD.
      Measured on the full `claudine` L1 suite: **summed 187.40 s → 102.00 s,
      elapsed 15.96 s → 8.89 s.** Per family: `composition::sequence::preflight`
      32.84 s → 2.21 s, `composition::sequence::task` 56.29 s → 20.43 s,
      `linking::paths` 9.31 s → 0.88 s.
- [x] Keep real shipped-prompt coverage through an isolated copy of the corpus
      with relative references preserved — do not replace the shipped artifact
      with a synthetic stand-in.
      No shipped artifact was replaced. `composition::schema`'s two
      `shipped_implement_plan_*` tests keep the **real**
      `prompts/_implement/implement-plan.md` at its shipped path and keep the
      real `ctx.*` capture that goes with it (1.38 s each): the file declares no
      relative references, so relocating it would buy 0.2 s and cost the only
      in-repository end-to-end preparation of a shipped prompt the library has.
      The isolated-copy shape is already in place where it belongs — Phase 5A's
      `compose_eager_spec_setter_…` copies `prompts/plan.md` and the
      `shipped_plan_route` spec into the fixture at their checkout-relative
      paths, and Phase 5C's `shipped_prompts.rs` keeps the shipped
      `prompts/implement.md` as its subject while isolating only the repository
      it launches from.
- [x] Consolidate the repeated source scans **only** where coverage, diagnostic
      attribution, and selective execution survive. Where justified, use the
      established shape: one shared passive corpus binary that scans once and is
      *extended* per regression, rather than a new rescanning process per case,
      with representative end-to-end cases retained through real shipped
      artifacts.
      Decision 2's single recommendation, and nothing else: `error_guards`'
      twelve `scan_production_sources()` cases became named arms of
      `SCAN_GUARDS`, evaluated by one passive corpus test that scans once and
      reports **every** failing arm under the test name it used to carry. The
      six scan-free cases stay independently selectable — four are the
      blindness anchors, and folding those in would make the corpus test its
      own witness. Measured: **20.20 s summed / 18 processes → 1.72 s / 8**,
      elapsed 1.707 s → 1.614 s. The five text scanners are untouched.
- [x] Re-measure the loop pause and context rendering families against Phase 3's
      attribution; report what actually dominated rather than what was assumed.
      **Neither dominated, and what dominated `context_command` was not what
      Phase 3 attributed.** Phase 3 read `context_command` at 27.87 s and named
      the launch CWD; Phase 5C's migration to a fixture-built repository took it
      to 12.52 s, which left the *fixture* as the cost — both width sweeps
      called `repository_fixture()` (a `TempDir` plus a `git init`) **inside**
      their loops, 12 and 7 times. Hoisting it took
      `context_reports_preserve_all_columns_at_minimum_supported_width` from
      2.182 s to 0.897 s and the binary to 11.55 s.
      The loop-pause family did not move and should not: at 4.335 s
      (Phase 3: 4.04 s) it is a real rate-limit wait. `loop_cli` is 15.37 s over
      18 identities, and `compose_loop_rate_limit_pause_waits_then_continues`
      plus `compose_loop_step_timeout_surfaces_as_iteration_failure` are 6.8 s
      of that — both timeout contracts, i.e. Phase 7's, not this phase's.
      Two families Phase 3 never ranked are now above both:
      `sequence_overlay_pty` (18.62 s / 7) and `compose_caller_file_provenance`
      (9.35 s / 15). The first is sleep-bound and belongs to Phase 7; the second
      is fifteen distinct proxy/caller-identity scenarios at the boundary that
      proves them.
- [x] Work through the override census from Phase 2. Each survivor gets a
      justification tying its floor to a real contract; each other one is
      **removed together with the cost it was hiding** — including the 30 s
      entry on `context_reports_preserve_all_columns_at_minimum_supported_width`.
      Adding a new override is disallowed (AC5, AC7).
      Eight blocks removed, none added; `git diff main -- .config/nextest.toml`
      is 16 insertions / 41 deletions and **every inserted line is a comment**.
      Removed with the cost each hid, measured on 2026-09-08:
      `context_reports_preserve_all_columns_at_minimum_supported_width`
      5.21 s → 0.897 s (the entry the spec names),
      `agents_and_commands_route_to_empty_state_messages` 0.05 s,
      `exhausted_remediation_fails_finalize_and_preserves_findings` 0.059 s
      against a comment claiming 21–83 s, `compose_perf_stdout_matches_non_perf`
      0.476 s, `inline_compose_perf_stdout_matches_non_perf` 0.530 s, and the
      dead `test(=composition::loop_engine::…)` filter in both profiles — the
      module is `composition::looping::engine` and the real test runs in
      0.023 s.
      Kept, with the contract each floor expresses recorded at the entry:
      `compose_loop_rate_limit_pause_waits_then_continues` (4.335 s of real
      pause, against a 5 s slow mark), the `package(claudine-cli) &
      test(/level2_/)` blanket (the L2 binaries' own ~40 s deadline sits above
      the 30 s default kill), `test(/level2_/) retries = 0`, and the two
      `test-group` bindings.
      **New finding.** Every per-test `slow-timeout` override in the **ci**
      profile sets `{ period = "30s", terminate-after = 3 }`, which is that
      profile's own default — all eight were no-ops. The five Claudine-scoped
      ones are gone; the three that belong to darkmatter and biscuit-terminal
      are named in the census as out of scope. The profile comment now says so,
      so the next author does not add a ninth.
- [x] Repair tautological assertions and stale test identities **in separately
      reviewable commits**. For each, record the original failing input where
      one exists, the defect the old assertion could not distinguish, and the
      additional failure the replacement now detects.
      Seven, each with its own before/after recorded at the call site:
      1. `wrap_opencode.rs` — `assert_eq!(row["error"], row["error"], "error
         field should carry the rate-limit message")`. A value compared with
         itself; held for a missing, `null`, or generic `error`. Now asserts the
         field is a string containing `Usage limit reached`. Neuter transcript:
         changing the needle to `Quota limit reached` fails and prints the real
         value, `"Usage limit reached for glm-5.1 (zai-coding-plan); resets at
         2026-04-15 21:18:56"`.
      2. `stream::stderr::tests::silent_mode_produces_nothing` —
         `assert_eq!(Verbosity::Silent, Verbosity::Silent)`, true of a build
         with the silent gate deleted. Nothing in that module takes a
         `Verbosity`. Removed; replaced by two tests beside the gate itself
         (`render::event_renderer`): `silent_verbosity_produces_no_render_units`
         (with a Normal-verbosity control, so Silent proves something) and
         `session_start_updates_the_auth_source_even_when_silent`, which pins
         the ordering `render`'s doc comment calls out and nothing tested.
         Neuter: hoisting the gate above the `api_key_source` self-update fails
         the new test with `left: None, right: Some("ANTHROPIC_API_KEY")`.
      3. `linking::paths::repo_scope_target_paths_are_absolute` → renamed
         `…_are_rooted_at_the_repository`. `is_absolute()` was satisfied by a
         bare `/.claude/skills`; the linker's actual contract is that the target
         is the repository root joined with the provider directory, which is now
         asserted by equality.
      4-6. `target_dir_resolves_agent_paths`,
         `resource_dir_resolves_non_markdown_custom_paths`, and
         `opencode_also_reads_from_claude_for_skills` asserted `ends_with(…)` /
         `any(ends_with(…))`, which cannot distinguish a path rooted at the
         wrong root — the exact defect a home/repo argument swap in
         `from_roots` produces. All three now assert equality against the
         synthetic root they were given.
      7. `context_values_renders_non_null_for_canonical_keys` — a second launch
         of `claudine context --values` asking the identical question of one
         more key than its near-namesake. Folded into
         `context_values_renders_canonical_keys_non_null` as a fourth key.
      Plus two stale **tier** identities, which are the same defect in a name:
      `slow_compose_sigint_during_prep_exits_130_with_notice` (0.183 s; the
      `slow_` prefix removed it from every recipe and every CI leg) and
      `real_corpus_builds_deterministically` (0.092 s; `real_` did the same, and
      the "real" it meant was the shipped corpus, not an external resource).
      Both renamed, so two contracts that ran nowhere now run in L1.
- [x] Move exhaustive value/representation combinations to the cheapest boundary
      that proves them, retaining representative real-binary cases. Record every
      test-population change with its replacement coverage.
      The two `context_command` width sweeps are the only exhaustive
      combination in the area, and the CLI **is** their cheapest proving
      boundary: the report renderers write to the process logger
      (`log::data`) rather than returning text, so an in-process width sweep
      would need a capture seam that does not exist and adding one is a
      production change this fix puts out of scope. What was movable was the
      *setup*: nineteen `TempDir` + `git init` fixtures for nineteen launches,
      when the combination under test is width × report mode and the repository
      is not part of it. One fixture per sweep now; every launch and every
      assertion is retained.
      Test-population changes, not netted. Removals: 2 —
      `context_values_renders_non_null_for_canonical_keys` (replacement: its
      key is now asserted by `context_values_renders_canonical_keys_non_null`)
      and `stream::stderr::tests::silent_mode_produces_nothing` (replacement:
      the two `event_renderer` tests above). Additions: 15 —
      `a_failing_scan_backed_guard_is_reported_under_its_own_name`,
      `production_sources_pass_every_scan_backed_guard` (which carries the
      twelve merged arms),
      `linking::paths::new_roots_the_table_at_the_process_home_and_the_resolved_repository`,
      `render::event_renderer::silent_verbosity_produces_no_render_units`,
      `render::event_renderer::session_start_updates_the_auth_source_even_when_silent`,
      and the 12 identities lost when `error_guards`' scan-backed cases merged
      are counted as removals below. Merges: 12 `error_guards` identities → 1
      (each still named in the failure report). Renames (neither addition nor
      removal): 4.
- [x] Reconcile the recipe/metadata findings from Phase 2: bring `test-real`
      onto nextest or record why it cannot be; declare
      `[package.metadata.ci.tests]` for `claudine-contract`,
      `claudine-catalog-types`, and the three rendezvous crates, or record the
      deliberate default; close or link the rendezvous canonical-recipe gap.
      All three closed, none deferred.
      **`test-real` is on nextest.** It was the only recipe in the area using
      `cargo test`; it now goes through `_test_real`, so it stages a JUnit
      report and obeys the slow/leak policy like every other tier. The stated
      reason for the old form — one unavailable provider must not mask the
      other — is preserved by the two invocations and the aggregated exit code
      it already had. Verified: nextest selects the four
      `claudine-contract::real_provider` identities and the one
      `claudine-cli::real_opencode_yolo_subagent` identity. The four
      contract identities **fail on this host** with `Unauthorized` — a live
      provider that is not authenticated here — and fail identically under the
      old `cargo test` form, so this is a disclosed pre-existing host
      condition, not a regression.
      **Five `[package.metadata.ci.tests]` blocks declared**, each stating why
      the route is what it is rather than restating the default. The scope
      calculator validates them (`affected_scope.py --all` exits 0 and still
      rejects a typo'd field, transcript in `log.md`).
      **The rendezvous recipe gap is closed**: `sanity`, `test-l3`,
      `test-browser`, `test-real`, `doctest`, `bench`, `coverage`, `fuzz` and
      `all` added, matching the canonical twelve. The one with teeth is
      `test-real`: `rendezvous-daemon::peer_discovery`'s two `real_*` mDNS
      identities had **no** route at all and now run and pass through it.
- [x] Run `just check-tier-coverage` and `just check-test-interrupts` after any
      tier-marker or recipe change.

**Validation checkpoint 6** — `just test`, `just test-cli`, `just test-gen`,
`just test-contract`, `just doctest`, `just bench` and `just lint` green;
`just test-rendezvous` green; the override census in `inventory.md` shows every
entry justified or removed; `git diff main -- .config/nextest.toml` shows
removals only.

**Passed, with the same host-condition L2 failure Phases 4 and 5 disclosed and
one host-condition `real_`-tier failure this phase surfaced.**

| Gate | Result |
|---|---|
| `just test` | **6871 passed / 9 skipped**, 24.86 s |
| `just test-cli` | green |
| `just test-gen` | green (fleet: 83 records, 0 failures) |
| `just test-contract` | green |
| `just doctest` | green — 25 passed / 7 ignored |
| `just bench` | exit 0 |
| `just lint` | exit 0 (claudine area **and** `claudine/rendezvous`) |
| `just test-rendezvous` | green — core, daemon, client |
| `just check-tier-coverage` | exit 0, **0 stranded** |
| `just check-test-interrupts` | exit 0, 34 areas |
| `just check-windows` | exit 0 |
| `git diff main -- .config/nextest.toml` | 16 insertions / 41 deletions, **every insertion a comment** |

`just bench` needed `BENCH_YES=1`: `_bench_preflight` reads the 1-minute load
average, which on this host sat at 11–33 for the whole session while
`top` showed the CPU idle — the recorded AutoMounter/SMB load-average
inflation, not real contention. The run itself completed with no error and no
panic.

**Test count reconciliation** (not netted). 6878 → 6871 is exactly −7:
`error_guards` 18 → 8 (twelve scan-backed identities merged into one, plus one
new non-vacuity test) = −10; `linking::paths` 10 → 11 = +1;
`render::event_renderer` +2; `stream::stderr` −1 (the tautology, replaced by
those two); `context_command` 27 → 26 = −1 (the duplicate `--values` test);
`wrap_sigint` +1 and `claudine-gen::signals_validation` +1 as the two renamed
tier identities enter L1 selection. Skips 11 → 9 is the same two renames
leaving the filtered-out set.

**`just test-l2` is 236/237**, under `--no-fail-fast`, reproduced twice. The
survivor is `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` —
the identical Atuin-prompt-in-the-WezTerm-pane condition Phases 4 and 5 both
recorded, in a `level2_*` file this phase never touched. Three earlier
fail-fast runs each aborted on a *different* single test
(`level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch`
twice, then a WezTerm `LEAK-FAIL`); each passes in isolation, and the two
`--no-fail-fast` runs that actually execute all 237 do not reproduce them.
Nothing this phase changed reaches an L2 binary: the library edits are all
`#[cfg(test)]` or test modules, which are excluded when `claudine` is built as
`claudine-cli`'s dependency, and no `cli/tests/common/` or `level2_*` file was
touched.

**`just test-real` from the `claudine` area is 1 passed / 4 failed.** The four
`claudine-contract::real_provider` identities report `Unauthorized` — the
provider CLI is not authenticated on this host. They fail identically under the
`cargo test` form this phase retired, so the recipe change is not the cause.
Recorded as a pending host condition, not as passing evidence.

**Sequencing note.** Phase 1's checkpoint is still blocked on the operator
merge, so nothing here is staged or committed.

---

## Phase 7 — Bounded time and resource ownership (RB4) ‖ with Phase 6

- [x] Replace every readiness sleep with bounded observation of the **final
      required condition**, with a deadline. Audit all 7 L1 sleep sites in
      `cli/tests` (`common/pty.rs` ×5, `wrap_sigint.rs`, `completion_perf.rs`,
      and the Windows pair's 3 each) and the lib-side sleeps in
      `composition/sequence/task/tests.rs` (8), `render/assistant_stream.rs`
      (4), `composition/sequence/task/shell.rs` (2), and the singletons in
      `render/thinking_stream.rs`, `model_catalog/`, `config/atomic.rs`,
      `composition/looping/engine.rs`.
      **The plan's L1 count is 7 short**: the two `level1_*_pty.rs` binaries
      carry 2 apiece, so the L1 population is 17 sites across 7 files, not 13.
      Full audit table in `inventory.md` § Sleep sites.
      Eleven of the seventeen were already poll cadences inside deadline loops
      and stay. Four became bounded observation. The decisive finding was not a
      sleep at all: **the PTY harness never answered the OSC 10/11 colour
      queries the child blocks on**, so every claudine PTY child waited out
      biscuit-terminal's 1 s `DEFAULT_TIMEOUT` twice before emitting a byte —
      2.05 s of the 2.4 s each `sequence_overlay_pty` test cost. `common/pty.rs`
      now answers them on observation, exactly as it already answered the DSR
      cursor probe. `sequence_overlay_pty` **17.88 s summed / 3.03 s elapsed →
      4.66 s / 1.11 s**.
      Lib side: five of the six `1600 ms` reap waits became direct observation
      of the descendant's pid (`BackgroundedDescendant`), 1.6 s → ~0.04 s each,
      and the interrupt-readiness sleep became a marker wait, 0.33 s → 0.06 s.
- [x] Retain sleeps that *are* the timeout contract, and justify each budget,
      polling cadence, and shutdown margin in the inventory row — derived under
      the startup-stall fix's spawn-fallback silence clock.
      Thirteen retained rows, each with its budget's derivation, in
      `inventory.md` § Sleep sites. `config/atomic.rs` is the model case: its
      Windows persist backoff is already injected as `S: FnMut(Duration)`, so
      the test never sleeps at all.
- [x] Give daemon / session / IPC tests isolated endpoints, data directories,
      processes and cleanup. Cover `rendezvous-daemon`'s `pairing_and_sync.rs`,
      `peer_discovery.rs`, `phase6_integration.rs`, `rendezvous-client`'s
      `local_round_trip.rs` and `session_log_round_trip.rs`, and the
      `daemon-tests`-gated CLI targets.
      All eight files already own a per-test `TempDir`, an in-memory
      projection, an ephemeral `quic_bind` port, and — in `session_report` —
      a bounded connect-until-accepted readiness poll rather than a sleep.
      One real gap: `private_endpoint` isolated the **Windows** named pipe by
      process id alone, dropping `parent` entirely, so two fixtures asking for
      `alice` in one process shared a pipe. Nextest's process-per-test hid it.
      The fixture root is now folded into the pipe name, and
      `endpoints_are_stable_per_fixture_and_distinct_across_fixtures` asserts
      the contract on whichever transport the build targets.
- [x] Confirm unrelated tests disable reporting (`CLAUDINE_RENDEZVOUS_REPORT`
      is already a builder default — verify it reaches the raw path too).
      It does, structurally — one computed `ChildEnvironment` feeds `build`,
      `build_std` and `apply_policy_to` — but nothing asserted it, and the
      set-equality drift test cannot: two identically enabled surfaces compare
      equal. `both_command_surfaces_disable_rendezvous_reporting_over_an_enabled_parent`
      now pins it against a parent that exported `true`. Absence means
      *enabled* (`Err(_) => true` in both `handle.rs` and `session_report.rs`),
      so the neuter that drops the default yields `[]`, not `[true]` — the
      escape is real either way.
- [x] Prove cleanup on panic, error and cancellation for the process-owning
      cohorts with **nextest's per-test leak policy plus the root
      `just test-leaks` post-run sweep** — inspection alone is not proof.
      The sweep earned its keep: `just test-leaks claudine` reported **two
      orphaned `claudine` processes**, and `sample` traced them to
      `run_audio_worker_if_requested` → `playa::detached::run_scheduler` →
      `delegate_job`, blocked in `wait4`. A lifecycle audio effect makes
      claudine re-exec *itself* as playa's detached spool worker, which
      outlives the enqueuing command by design — so
      `shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target`
      left two processes behind and played a sound through the developer's
      speakers on every run. The builder now defaults `PLAYA_DRY_RUN=1` and a
      fixture-local `PLAYA_SPOOL_DIR`, and sweeps the `PLAYA_*` namespace
      before applying them. `detached_audio.rs` already carried
      `.env_remove("PLAYA_DRY_RUN")` — an escape hatch written for a default
      that had never been added — and is now the one file that opts back in.
- [x] Reduce runner-visible serialization to resources that are actually shared;
      remove `#[serial]` where the resource is per-test. Note that
      `serial_test` is a no-op across nextest processes, so anything genuinely
      shared needs runner-visible coordination instead.
      The `serial(pty)` group was the one provably empty claim: `Session::spawn`
      allocates a fresh `/dev/ptmx` pair and each fixture root is unique, so
      nothing is contended — and nextest was already running all seven
      `sequence_overlay_pty` tests concurrently (18.9 s summed in 3.2 s
      elapsed), which is the proof the annotation enforced nothing. 30
      annotations removed across 7 files; the reasoning is recorded once at
      `common/pty.rs`'s module level rather than 30 times.
      The remaining ~200 `#[serial]` sites guard process-global `env`
      mutation. Under nextest they are inert, but they are not *false*: they
      name a resource that genuinely is shared whenever the tests share a
      process. Left alone.
- [x] Verify no terminal or browser gains focus in any tier touched.
      Nothing this phase changed spawns a terminal emulator or a browser. The
      PTY work is `expectrl` over `/dev/ptmx`, which has no window; the tiers
      touched are L1 (`claudine`, `claudine-cli`) and rendezvous L1. The four
      `level2_*` PTY binaries whose `serial(pty)` was removed are PTY-backed
      too — they were never terminal-harness tests. Confirmed by observation
      across the `just test-l2` run recorded below: no window was created and
      focus never left the invoking terminal.

**Validation checkpoint 7** — `just test-leaks` at the repo root reports no
survivors; `just test-daemon` and `just test-rendezvous` green; the ten reruns
scheduled in Phase 9 have a stable target set recorded.

**Passed, with the same host-condition L2 failure Phases 4-6 disclosed.**

| Gate | Result |
|---|---|
| `just test-leaks claudine` (from the repo root) | **`leak-sweep: no leaked processes detected`**, 7146 passed / 11 skipped — against two orphans before the fix |
| `just test` | 6873 passed / 9 skipped, ~26 s |
| `just test-cli` | 2498 passed / 9 skipped |
| `just test-daemon` | 2503 passed / 9 skipped |
| `just test-rendezvous` | 273 passed / 2 skipped (272 → 273: the new endpoint test) |
| `just doctest` | exit 0 |
| `just lint` (claudine **and** `claudine/rendezvous`) | exit 0 |
| `just check-windows` | exit 0 |
| `just test-l2 --no-fail-fast` | 236 / 237 — see below |

The leak sweep is scoped to `claudine`, which is this phase's whole blast
radius; an unscoped workspace sweep would rebuild 35 packages to re-prove the
other 27 areas' cohorts, which no change here touched.

**`just test-l2` is 236/237.** The survivor is
`level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`, with Atuin's
"Atuin AI is not yet configured" prompt visible in the captured WezTerm pane
swallowing the exit marker — byte-for-byte the condition Phases 4, 5 and 6 each
recorded, on this host, in a file this phase did not touch. Notably the four
`level2_*` PTY binaries this phase *did* touch all pass, so answering the OSC
colour queries did not shift any styling assertion.

**Stability under load.** The converted reap tests are timing-sensitive by
construction, so they were run under representative suite load rather than in
isolation: **16 full `just test` runs**. The first three exposed two genuine
fixture races, both fixed (see the test-count note below); the last **11 were
consecutive and clean**. One earlier run reported a single nextest `LEAK-FAIL`
whose identity was not captured and which did not recur in those 11 — recorded
as unexplained rather than dismissed. Phase 8 owns the ten-rerun evidence
tranche; this is the stability bar for landing, not that evidence.

**Test-count reconciliation** (not netted). Additions: 3 —
`both_command_surfaces_disable_rendezvous_reporting_over_an_enabled_parent`,
`both_command_surfaces_keep_audio_out_of_the_developers_machine`,
`endpoints_are_stable_per_fixture_and_distinct_across_fixtures`. Removals: 0.
`just test` 6871 → 6873 (+2, the two `claudine-cli` fixture tests) and
`just test-rendezvous` 272 → 273 (+1) account for exactly the 3.

**Two fixture races this phase created and closed**, both found by running the
suite rather than the tests:

1. The descendant could be reaped before it published its pid, because for a
   command as short as `printf 'now\n'` the reap lands in milliseconds. The
   command shell now waits for the pid file before completing, which also
   removes the vacuous case where nothing was backgrounded.
2. In the two wait-error tests the injected failure arms on the *first captured
   stdout byte*, so a descendant staged after that byte raced the teardown into
   existence. Both now background before their first `printf`.

**One finding recorded rather than fixed.**
`a_failed_ownership_setup_kills_the_spawned_command` cannot observe a
descendant: the injected failure fires on the statement after `spawn`, so the
kill reaches the shell before it runs its first command and nothing is ever
backgrounded — which means its `!marker.exists()` assertion has always held
vacuously. Converting it to `BackgroundedDescendant` fails on exactly that.
Strengthening it needs the runner to expose the direct child's pid, a
production change this fix puts out of scope; the limit is now stated at the
test instead of implied by its name.

**Sequencing note.** Phase 1's checkpoint is still blocked on the operator
merge. Phases 4-6 were committed to this branch by the separate commit process
partway through this phase; nothing here was staged or committed by it.

---

## Phase 8 — Local measurement (RB5, first evidence tranche)

Requires Phases 5–7 implemented with applicable checks passing; diagnosed
unrelated environment failures remain explicitly pending.

Validation reuse for Phases 8–10: record command, selection, features,
profile, platform, source state (including dirty changes), environment, result,
and artifact link once in `log.md`. Reuse evidence while its relevant inputs
remain unchanged; rerun only affected checks after a change or new failure.
Carry diagnosed unrelated environment failures as pending with a reproduction
link, and revisit them only after relevant code or environment changes.
Document-only updates do not trigger Rust suites. These phases may prepare a
PR and its acceptance report while CI evidence is pending; the dependency map
below describes final evidence completion, not a barrier to PR review.

- [x] Warm the preserved baseline and candidate artifacts, then collect
      **five alternating warm runs per revision** of each required full L1
      population. Extract changed-cohort identities, counts, and summed test
      durations from those same reports; do not also run every cohort in
      isolation. A cohort's summed duration is not its isolated wall time.
      Run a separate cohort experiment only when selection, resource needs,
      or an explicit latency claim cannot be represented by the suite runs.
      Record source state, toolchain, features, profile, platform, concurrency,
      fixture inputs, cache state, and commands. Keep revision-specific build
      directories warm and prevent concurrent edits or competing workloads
      during measurement. Reuse earlier samples only if their provenance and
      alternating sequence match this protocol.
      Done, twice. Baseline is a detached worktree at `9fc5151a0` with its own
      build directory; candidate is this tree. `measurement-runner.ts` ran
      one uncounted warm-up per suite and revision (they absorbed 175 / 28 /
      298 crates of per-invocation feature re-unification; every counted run
      compiled nothing), then `baseline, candidate, …` five times for
      `just test` and for `just test-rendezvous`, then the candidate-only load
      rounds — 45 runs, all exit 0, in `measurement/`. `measurement.ts` reads
      the logs, keeps the three costs apart, and gates on count mismatches,
      unstable identity sets and any non-passing result. Cohorts
      (`measurement/cohorts.json`) come out of those same suite reports; no
      cohort was run in isolation. The host was not quiet — system daemons held
      two to four cores and one baseline run sat at 88.6 s against a 49–55 s
      neighbourhood — so the whole-suite delta did not clear the strict drift
      bracket and a second alternating series of ten `just test` runs was
      taken in a quieter window (`measurement/series-2/`). Series 2, medians:
      runner elapsed **52.30 → 37.89 s**, summed **819.42 → 587.89 s**,
      build/setup 1.5–1.9 s at both; candidate ÷ baseline per adjacent pair
      0.643–0.765 (elapsed) and 0.637–0.758 (summed), improved in all ten pairs
      across both series. Identities 6861 → 6873 (+28 / −16, every one named
      in the report); failures, timeouts, leaks and retries 0 in every run.
      `just test-rendezvous` is unchanged (paired median 0.96), as claimed.
- [x] Execute each **changed** timeout, readiness, or concurrency
      contract ten times under representative suite load. Count compatible
      candidate measurement runs toward those ten executions and run only the
      remaining repetitions with a fixed representative load cohort. Record
      the target set, spread, failures, and leak results. Do not repeat
      unchanged tests separately merely because their file was migrated.
      Target set in `measurement/targets.json`: the five
      `composition::sequence::task` reap / interrupt tests, the three L1 PTY
      binaries (11 identities), the rendezvous endpoint test, and the four L2
      PTY binaries whose `serial(pty)` was removed (19 identities) — 36
      identities, **eleven executions each**: one warm-up, five alternating,
      five load rounds. The load cohort is the full L1 population of the
      target's own package set (`just test`, `just test-rendezvous`), and for
      the L2 binaries the four running together at `-j 8`, the concurrency the
      removed group used to forbid. 0 non-passing, 0 retries, 0 leaks across
      396 executions. Reap tests 0.038–0.106 s; L1 PTY 0.24–1.21 s; one L2
      outlier, `level2_pty_provided_partial_single_match_confirms_and_launches`
      at 4.609 s once against a 0.81 s median, passing. Files that were only
      migrated were not re-run separately.
- [x] Measure any cold-build claim in an isolated build directory — never by
      clearing the developer's working cache.
      No cold-build claim exists in Phases 4–7 (`log.md` contains no such
      claim; the only "cold" in this plan is this bullet). Nothing to measure.
      The baseline worktree's build directory was created for the alternation,
      not for a claim, and the developer's cache was never cleared.
- [x] Prove eliminated discovery and unrelated launches with **work counters or
      sentinel effects**, independently of timing. A timing improvement is not
      evidence that a walk was removed.
      `sentinels.ts`, transcripts in `measurement/sentinels/`. The counters
      are lldb breakpoint hit counts at the entry location of the function
      each claim names, on the binaries the suite ran, at both revisions.
      **The CWD walk** (`capture_file_resolution_context`, one process per
      module): `composition::schema` 73 → 11, `sequence::preflight` 62 → 2,
      `sequence::task` 84 → 0; `resolve_repo_root` in `linking::paths`
      10 → 3. Each survivor is a test Phase 6 named as deliberately kept on
      real discovery. **The repeated scan**: `run_scan` — the `OnceLock`
      initializer behind `scan_production_sources` — fired in 12 of 18
      `error_guards` processes at the baseline and in 1 of 8 on the
      candidate. **The launch origin**: a `git` shim on `PATH` during
      `just test-cli --test context_command` logged 44 `rev-parse
      --show-toplevel` calls from the checkout root and 0 `git init` at the
      baseline, against 34 `git init` under `$TMPDIR`, 0 inside the checkout
      and 0 `show-toplevel` on the candidate — 26 tests, 34 repositories,
      which is the hoisted sweeps building one each instead of 12 and 7.
      `just test-leaks claudine` and the two structural gates were re-run and
      are recorded in `log.md`; the baseline leak sweep was not repeated
      because Phase 7 recorded that it plays audio on the host.
- [x] Keep the three costs separate in every table: build/setup, runner elapsed,
      summed test duration. Track identities, counts, failures, skips, timeouts,
      retries and slow cases alongside speed so lost coverage cannot read as an
      optimization.
      Every table `measurement.ts` emits carries build/setup (wall − elapsed),
      runner elapsed and summed duration as separate columns beside tests,
      passed, failed, timed out, skipped, leaks, retries and slow marks, and
      lists the added and removed identities by name. The recipes themselves
      still do not separate build from run; the runner measures wall time
      around them, which is why the column exists locally at all.
- [x] Record local numbers as **attribution only**; they establish no CI target.
      Stated at the head of the Phase 8 log section and in the report's
      preamble; `attribution.ts`'s budget gate still refuses non-CI provenance
      and nothing from this phase feeds it.

**Validation checkpoint 8** — five alternating runs per revision cover the full L1
populations and changed cohorts through shared reports; ten executions cover
each changed timing/concurrency contract; each eliminated-work
claim has a counter or sentinel behind it.

**Passed.** Five alternating runs per revision of `just test` (twice) and of
`just test-rendezvous`, every cohort read from those reports; eleven
executions of each of the 36 target identities, 0 non-passing; three work
counters and two structural gates behind the three eliminated-work claims.
The one thing the checkpoint did not get is a whole-suite delta that clears
the strict drift bracket — the host's daemons saw to that in both series — and
`log.md` reports it as such beside the paired reading (candidate ÷ baseline
0.64–0.77 in all ten pairs) rather than choosing. Full record in `log.md`
§ Phase 8; every table in `measurement/report.md` and
`measurement/series-2/report.md`.

---

## Phase 9 — CI evidence (RB5, second tranche; AC6)

Human-gated like Phase 1: push and read.

Configured legs include `ubuntu-latest`, `macos-latest`,
`windows-latest`, and WSL2; record the actual selected packages on each.

- [x] Complete one consolidated validation of the affected scope before
      push handoff. Inspect the actual recipe expansion: if `just ci-local`
      already includes lint, run it once without a preceding
      `just ci-local --lint-only`. Credit equivalent current-state checks in
      the validation ledger only where the workflow supports that reuse;
      otherwise run the required gate once. Record scope, features, and any
      checks still missing. This step does not authorize a commit or push.
      `ci-local` runs lint and test by default, so it ran once: **73 packages,
      class=full** (the `.config/nextest.toml` deletions are a non-comment
      global change), **147/147 gates, exit 0** in 44 m 16 s. Nothing
      credited from earlier phases; ledger in `log.md` § Phase 9. Still
      missing: candidate CI itself, L3 (focus), `real` (host auth).
- [x] Run `just check-windows` where the mingw toolchain is present; state the
      limitation explicitly where it is not.
      Toolchain present; exit 0 (warm, 1.3 s) with the two `wrap_basics.rs`
      unused-import warnings Phase 1 disclosed still emitted. mingw, not MSVC;
      the MSVC surface is covered by the `windows-latest` check job on the
      baseline run (`candidate/local-gates/check-windows.log`).
- [ ] Open or hand off the PR once implementation and applicable local
      checks are ready; do not wait for repeated performance CI samples.
      Use the first candidate run on every configured package/environment leg
      for cross-platform correctness review, then accumulate **three
      consecutive green candidate runs per leg** for final performance
      verification. Reuse qualifying normal CI runs; request extra runs only
      for missing samples. Keep source state, workflow definition, runner
      image, and features comparable, and record every intervening failure.
      Baseline gaps and missing samples remain pending; PR readiness does not
      imply merge readiness or completion of the performance criteria.
      **Handed off, not opened — operator action.** `candidate/README.md`
      carries the steps and `candidate/pr-body.md` the description. Blocked
      on a merge of `origin/main` that conflicts in thirteen files (PR #70
      landed after this branch diverged), a signed commit and a push, none of
      which this non-interactive session can do. Candidate runs: **0 of 3**
      per leg. Baseline: the predecessor merged (`444213eb5`) and its first
      `main` run is stored and green on all four legs, **1 of 3**.
- [ ] Compare **matched tests within each environment** against that
      environment's own baseline. Report additions and platform exclusions
      separately; do not require identical cross-platform counts.
      **Tool ready, candidate pending.** `junit-metrics.ts --baseline` matches
      identities within one environment and lists additions and removals
      apart, never across legs; platform exclusions are declared
      (`windows-latest`: eleven `#![cfg(unix)]` tests, 2105 vs 2466
      identities) and reported in their own table. Smoke on real data: the
      tree-identical PR run against the `main` run matches every identity.
- [ ] Run `junit-metrics.ts` as the gate over both baseline and candidate sets;
      it must fail on malformed reports, missing artifacts or tests, duplicate
      identities, invalid durations, and failed runs.
      **Baseline gated (exit 0 on both stored runs); candidate pending.** The
      first pass over real artifacts failed on Windows's eleven `cfg(unix)`
      absences, so the gate gained `platformExclusions` plus a
      `stale-exclusion` violation — stricter, not looser. 57 tests.
- [ ] Compare against the Phase 3 budgets. **Report misses and their causes;
      do not invent a universal speedup percentage** and do not close a miss by
      adjusting the budget after the fact.
      **No budget exists to compare against.** `deriveBudgets` still refuses:
      one run per leg, not three, and no JUnit → family aggregator yet.
      Causes named in `log.md` § Phase 9; no percentage offered.
- [x] Run the affected L2/L3/real tiers through their canonical recipes only
      where the resources exist (`just test-l2`, `just test-l3`,
      `just test-real`); record unavailable runtime evidence as **pending**, not
      as passing.
      `just test-l2`: 236/237, the Phases 4–7 Atuin/WezTerm host-condition
      survivor, plus `claudine-gen`'s 3 run separately (the recipe aborts
      first). `test-l3`: **pending** — steals focus; not run from a
      non-interactive session. `test-real`: **pending, host condition** —
      Phase 6's `Unauthorized` on 4 of 5; not re-run against live providers.

**Validation checkpoint 9 (final performance evidence; not PR opening)**

three consecutive green runs exist per leg, with
failures disclosed; every budget is met or its miss is explained; no override,
retry, tier change, or disabled assertion was used to reach a number.

**Not passed — human-gated.** Candidate runs 0 of 3 per leg (no push);
baseline runs 1 of 3 (`34173378609`, green on all four legs, stored and
gated). Failures disclosed for everything that ran; L3 and `real` are named
pending. No budget exists yet, so none is met or missed. The only expectation
change is the Windows platform-exclusion declaration, which adds a violation
class rather than removing one. Full record in `log.md` § Phase 9.

---

## Phase 10 — Closure: `results.md`, drift, acceptance sweep

- [x] Write `results.md` with: the measurements (baseline and candidate, three
      costs separate, per leg); coverage changes (tests added, removed, moved
      boundary, replacement coverage for each changed assertion); residual
      findings; and **separate** implementation / verified-locally / verified-on-CI
      completion status.
      [`results.md`](results.md). Baseline per leg from `34173378609` with the
      PR-run noise bracket; candidate **none exists** (no push) and says so;
      local Phase 8 tables labelled attribution only; every one of the sixteen
      removed identities named with its replacement; the three claims are the
      first table and are not collapsed.
- [x] Give every deferred finding evidence, a reason, and a linked owner
      document. Generic fixture migration may **not** be deferred (AC4).
      Seven deferrals, each with evidence, reason and closing criteria, in
      [`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md);
      none is a fixture migration (the spawn allow-list is empty). Host
      conditions and recorded observations are listed apart in `results.md`
      § Residual findings so a pending tier cannot read as a deferral.
- [x] Update area docs and the `rust-testing` / `claudine` skills **only where
      workflow or architecture changed** — the raw-command builder surface and
      the widened isolation-gate population are the likely candidates.
      `CLAUDE.md` § Drift Maintenance governs.
      The builder surface and the isolation population were already repaired
      in Phases 5–7 (both skills). Phase 10 changed three skill files and no
      area doc (no `claudine/` doc describes the test workflow):
      `rust-testing/SKILL.md` gained the cross-compile route the predecessor
      lacked (`just check-windows`, mingw vs MSVC, warm-check caveat), the
      one-binary narrowing spelling, and the global-path scope rule;
      `rust-testing/test-suite-audits.md` gained the measurement workflow
      Phases 8–9 used (exact-recipe warm-up, drift bracket + paired ratio,
      lldb entry-location counters and the `PATH` shim, per-leg exclusions
      with stale-exclusion failure, `gh run rerun` for a second sample);
      `claudine/signal-handling.md`'s Windows row said "no recorded green
      Windows runtime run in this repo yet", which run `34173378609` falsified.
      Also repaired: the spec's links to the two archived fixes, which pointed
      at pre-`_completed` paths.
- [x] Sweep the acceptance criteria using one compact status-and-evidence table:
      the table is `results.md` § Acceptance criteria; statuses below.
  - [x] **AC1** — every discovered test/family has a reviewed disposition and a
        reconciled platform/feature/tier route; none omitted by timing threshold
        (reconciler output attached).
        **Verified.** Reconciler exit 0 at `9fc5151a0`, 7400 identities / 163
        targets, 0 unassigned / double-assigned / stale; output reproduced in
        `inventory.md` § Reconciler output.
  - [x] **AC2** — zero generic residual spawn exemptions; live-child and
        ordinary paths share the policy; negative guard tests and Windows proof
        present.
        **Verified.** Allow-list empty (`files:0 sites:0 governed_files:90`);
        one `ChildEnvironment` behind `build` / `build_std` /
        `apply_policy_to` with the drift test; seven neuters; `check-windows`
        exit 0 with zero warnings warm and cold; the two console-control tests
        green on `windows-latest`.
  - [x] **AC3** — the two inherited-width failures are covered; every
        contamination probe cannot alter unrelated results; probes used only
        disposable state.
        **Verified.** `COLUMNS=44` before/after transcripts (`log.md` § Phase
        5); eight probes, four neuters; the one checkout-adjacent probe uses
        gitignored `target/` and cleans up on both paths.
  - [x] **AC4** — shared-setup, cleanup, assertion and reachability findings in
        scope are resolved; deferrals are evidenced and linked.
        **Verified.** Four unreachable identities run; `test-real` on nextest;
        five metadata blocks; seven assertion repairs; leak sweep clean again
        in Phase 10; seven deferrals with an owner document.
  - [x] **AC5** — every pre-existing override is justified in the inventory or
        removed with the cost it hid; none was added.
        **Verified.** `inventory.md` § Runner override census + § Phase 6
        disposition; diff against `main` re-checked in Phase 10: 41 deletions,
        16 comment-only insertions, 0 non-comment insertions.
  - [x] **AC6** — local gates pass, `just check-windows` result recorded,
        platform limitations explicit, budgets have compatible CI evidence.
        **Partially verified; the CI half is pending.** Every local gate this
        host can run is green (ledger in `results.md`); `check-windows` exit 0;
        exclusions declared per leg; L3 and `real` recorded pending with
        cause. **No budget exists and no candidate CI run exists** — not
        claimable from this session.
  - [x] **AC7** — `results.md` complete; docs and skills updated only where
        workflow or architecture changed.
        **Verified.** `results.md` written; skill edits limited to the items
        above; no area doc needed a change.
- [x] Reconcile the final gate ledger instead of restarting all gates.
      Credit passing checks from implementation, measurement, and pre-push
      validation when their relevant source state and environment are still
      applicable. Run only missing or invalidated checks. If a full-suite
      run already covers a package subset with the required features, do not
      repeat that subset just to obtain a second green command. Record known
      environment failures as pending with links; do not retry them without
      a relevant change.
      Phase 10 edited two `claudine-cli` test files (the carried unused-import
      pair, plus a third of the same shape the warm Windows check had hidden),
      which invalidated the `claudine-cli` L1 suite, the area lint and the
      Windows check. Those were re-run; `test-rendezvous` (Phase 8),
      `test-l2` (Phase 9), `bench` (Phase 6), `ci-local` (Phase 9) and the
      TypeScript gates (Phase 9) are credited with the source-state argument
      recorded in `results.md` § Gate ledger. `test-l3` and `test-real`
      stay pending, not retried.
- [x] Prepare `results.md` and the acceptance review while CI runs. Record
      each criterion as verified, pending, or an explicitly permitted
      deferral, with an evidence link; keep the detailed record in its owning
      document. Final closure still requires the specified measurement samples
      and applicable acceptance evidence. Pending evidence does not prevent
      PR review, but it does prevent claiming that verification is complete.
      Done as above. No CI is running for this branch — there is nothing to
      run until the operator pushes — so the review was prepared against the
      stored baseline and the handoff. Verification is **not** claimed
      complete: `results.md`'s first table says so.
- [x] Required final coverage: area `just test` and `just lint`; relevant
      L2 via `just test-l2`; `just doctest`, `just bench`, and
      `just test-rendezvous`; root `just test-leaks`. Credit the CLI,
      generator, and contract selection already covered by area `just test`
      when the features match. Preserve missing higher-tier evidence as pending.
      Run in Phase 10: `just test` 6873 / 9 skipped; `just lint` exit 0;
      `just doctest` 25 / 7 ignored; root `just test-leaks claudine` 7146 /
      11 skipped, no leaks; `just check-windows` exit 0 warm and cold; sniff
      `just test` 2599 / 23 skipped and `just lint` exit 0. Credited:
      `test-l2` 236/237 + 3/3 (Phase 9, same day, same source), `bench`
      (Phase 6), `test-rendezvous` ×11 (Phase 8, and inside the Phase 10
      leak sweep's 7146). Pending: L3, `real`.
- [x] Confirm `git diff main -- .config/nextest.toml` contains removals only.
      Re-checked against `origin/main` (= local `main`, `6504747e2`): **41
      deletions, 16 insertions, and every inserted line is a comment** (the
      retained rate-limit override's contract and the ci-profile no-op rule
      Phase 6 recorded); zero non-comment insertions. Literal removals-only
      it is not; no override, retry, tier change or disabled assertion was
      added, which is what the bullet guards.

**Validation checkpoint 10** — all seven acceptance criteria are answered with
evidence or an explicitly linked deferral; `results.md` keeps the three
completion claims separate; no gate was weakened to close a criterion.

**Passed for what this session can answer; AC6's CI half is pending.** AC1–AC5
and AC7 verified with evidence; AC6 verified locally and pending on CI (no
candidate run, no budget). The three claims are separate in `results.md`. No
gate was weakened: the only test-file edits are two `#[cfg(unix)]` import
gates, and the nextest diff is unchanged from Phase 6.

---

## Parallelism map

| Can run concurrently | Why it is safe |
|---|---|
| Phase 2 ‖ Phase 1's CI window | Phase 2 is document-only; it lands no code, so it cannot contaminate the predecessor's attribution window. |
| Phase 5A ‖ 5B ‖ 5C | Disjoint file sets, each deleting only its own allow-list entries. The guard's stale-entry arm catches a mis-merge. |
| Phase 6 ‖ Phase 7 | Different concerns (cost/quality vs. time/ownership) and largely different files. Serialize only where both touch a binary Phase 5 also touched. |
| Phase 6/7 ‖ Phase 5D | 5D is confined to the live-child cohort plus `common/pty.rs`. |

**Strictly serial**: Phase 1 → Phase 4 (no code lands before the baseline
window closes); Phase 4 → Phase 5D (the raw-command path must exist); Phases
5–7 → Phase 8 → Phase 9 → Phase 10 (evidence follows the change it measures).

## Dependency order (summary)

```text
1 ──┬─→ 4 ─→ 5A ─┐
    │        5B ─┤
    │        5C ─┼→ 5E ─┬→ 8 ─→ 9 ─→ 10
    │        5D ─┘      │
    └─→ 2 ─→ 3 ─────────┴→ 6 ‖ 7 ──┘
```

## Out of scope (guard rails)

Named so a phase does not quietly widen. Anything here that turns out to be
necessary becomes a linked follow-up, not an in-flight expansion:

- production behavior changes, new provider features, wholesale CLI/library
  restructuring, CI runner redesign, generalized cross-package fixture
  frameworks;
- a second mechanical rewrite of the 29 already-migrated binaries — they get
  evaluation and regression verification only;
- moving Windows console-control tests to L3 or dropping Windows coverage to
  close the live-child exemption;
- weakening the isolation detector or dropping stale-entry failure to resolve a
  false positive — the sanctioned resolution is an allow-list entry naming the
  command the site targets;
- any new `slow-timeout` override, retry, tier change, or disabled assertion as
  a substitute for a fix;
- fixing a production defect found during attribution in place of filing it.

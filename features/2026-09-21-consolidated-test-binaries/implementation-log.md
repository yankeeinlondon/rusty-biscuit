---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-21-consolidated-test-binaries/spec.md
plan: features/2026-09-21-consolidated-test-binaries/plan.md
implemented_by: claude/opus
started_phase: "1"
packages:
    - repo-deps
    - claudine-cli
    - claudine
source_files_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/baseline/capture-listings.sh
    - features/2026-09-21-consolidated-test-binaries/baseline/inventory.py
    - features/2026-09-21-consolidated-test-binaries/baseline/consumer-sweep.py
    - features/2026-09-21-consolidated-test-binaries/baseline/deps-census.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s2-scan.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh
docs_updated_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/rulings.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s1-nextest-identity.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s2-crate-globals.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measurement.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s4-snapshots.md
    - features/2026-09-21-consolidated-test-binaries/baseline/inventory.md
    - features/2026-09-21-consolidated-test-binaries/baseline/test-selector-consumers.md
    - features/2026-09-21-consolidated-test-binaries/baseline/guard-scans.md
    - features/2026-09-21-consolidated-test-binaries/baseline/disk-and-ci.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - scripts/ci/test_ci_local.py
    - just/ci-local.just
    - features/2026-09-21-consolidated-test-binaries/selfproof/mutation-check.py
docs_updated_during_phase_2:
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_2:
    - features/2026-09-21-consolidated-test-binaries/selfproof/README.md
    - features/2026-09-21-consolidated-test-binaries/selfproof/noop-comparison.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - .config/nextest.toml
    - claudine/cli/Cargo.toml
    - claudine/cli/src/budget/tests.rs
    - claudine/cli/src/commands/wrap/exec/termination/coordinator/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/recovery_identity.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/agent_cwd.rs
    - claudine/cli/tests/l1/argv_normalization.rs
    - claudine/cli/tests/l1/characterization_error_routes.rs
    - claudine/cli/tests/l1/cli_process_fixture.rs
    - claudine/cli/tests/l1/command_routing.rs
    - claudine/cli/tests/l1/completion_cli.rs
    - claudine/cli/tests/l1/completion_compose.rs
    - claudine/cli/tests/l1/completion_contract.rs
    - claudine/cli/tests/l1/completion_inline_compose.rs
    - claudine/cli/tests/l1/completion_perf.rs
    - claudine/cli/tests/l1/completion_resolution_round_trip.rs
    - claudine/cli/tests/l1/completion_sequence.rs
    - claudine/cli/tests/l1/completion_setter.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_cli.rs
    - claudine/cli/tests/l1/compose_frontmatter_model.rs
    - claudine/cli/tests/l1/compose_header_first.rs
    - claudine/cli/tests/l1/compose_initialize_acceptance.rs
    - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
    - claudine/cli/tests/l1/compose_interactive_timeout_cli.rs
    - claudine/cli/tests/l1/compose_removed_validation_keys.rs
    - claudine/cli/tests/l1/compose_repository_context.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/compose_system_prompt_lifetime.rs
    - claudine/cli/tests/l1/compose_ttff_perf.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/contamination_probes.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/l1/contextual_errors.rs
    - claudine/cli/tests/l1/ctx_launch_anchor.rs
    - claudine/cli/tests/l1/detached_audio.rs
    - claudine/cli/tests/l1/diagnostic_discovery.rs
    - claudine/cli/tests/l1/dispatch_inventory.rs
    - claudine/cli/tests/l1/effective_diagnostic_render.rs
    - claudine/cli/tests/l1/error_guards.rs
    - claudine/cli/tests/l1/error_guards/source_scan.rs
    - claudine/cli/tests/l1/errors_command.rs
    - claudine/cli/tests/l1/handle_blocking_output.rs
    - claudine/cli/tests/l1/handle_deadline.rs
    - claudine/cli/tests/l1/handle_repo_config.rs
    - claudine/cli/tests/l1/hooks_cli.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/inline_compose_cli.rs
    - claudine/cli/tests/l1/inline_compose_hash.rs
    - claudine/cli/tests/l1/inline_compose_sequence_mismatch.rs
    - claudine/cli/tests/l1/level1_compose_autocomplete_failure_pty.rs
    - claudine/cli/tests/l1/level1_dry_run_pty.rs
    - claudine/cli/tests/l1/level1_inline_compose_mismatch_pty.rs
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
    - claudine/cli/tests/l1/level1_provider_overlay_home.rs
    - claudine/cli/tests/l1/level1_pty_wrapper_summary.rs
    - claudine/cli/tests/l1/level1_review_router_partial_pty.rs
    - claudine/cli/tests/l1/level1_schema_prompt_pty.rs
    - claudine/cli/tests/l1/level1_structured_error_message.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_initialize_state.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/mcp_cli.rs
    - claudine/cli/tests/l1/prompt_reporting.rs
    - claudine/cli/tests/l1/propagated_context_fixtures.rs
    - claudine/cli/tests/l1/protect_cli.rs
    - claudine/cli/tests/l1/provider_error_finalize.rs
    - claudine/cli/tests/l1/run_harness_loop_call_sites.rs
    - claudine/cli/tests/l1/sequence_budget.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_ctrl_c_windows.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_initialize_include_preflight.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_prompt_property.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/shipped_prompt_contract.rs
    - claudine/cli/tests/l1/shipped_prompt_route_drift.rs
    - claudine/cli/tests/l1/shipped_prompts.rs
    - claudine/cli/tests/l1/skills_integration.rs
    - claudine/cli/tests/l1/spawn_site_guard.rs
    - claudine/cli/tests/l1/system_prompt_perf_bench.rs
    - claudine/cli/tests/l1/test_placement.rs
    - claudine/cli/tests/l1/wrap_antigravity_exit_signal.rs
    - claudine/cli/tests/l1/wrap_basics.rs
    - claudine/cli/tests/l1/wrap_compose_agent.rs
    - claudine/cli/tests/l1/wrap_compose_exec.rs
    - claudine/cli/tests/l1/wrap_compose_preflight.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/cli/tests/l1/wrap_ctrl_c_windows.rs
    - claudine/cli/tests/l1/wrap_direct_argv.rs
    - claudine/cli/tests/l1/wrap_incomplete_subagents.rs
    - claudine/cli/tests/l1/wrap_inline_compose.rs
    - claudine/cli/tests/l1/wrap_inline_compose_interactive.rs
    - claudine/cli/tests/l1/wrap_opencode.rs
    - claudine/cli/tests/l1/wrap_opencode_models.rs
    - claudine/cli/tests/l1/wrap_perf.rs
    - claudine/cli/tests/l1/wrap_provider_flags.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/cli/tests/l1/wrap_structured_stream.rs
    - claudine/cli/tests/l1/wrap_watchdog_startup_stall.rs
    - claudine/cli/tests/l1/wrap_watchdog_timeout.rs
    - claudine/cli/tests/level2/level2_auto_complete_chooser.rs
    - claudine/cli/tests/level2/level2_auto_complete_operation_file.rs
    - claudine/cli/tests/level2/level2_context_capture.rs
    - claudine/cli/tests/level2/level2_dry_run_approval_capture.rs
    - claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs
    - claudine/cli/tests/level2/level2_explicit_operation_file_miss.rs
    - claudine/cli/tests/level2/level2_file_resolution_capture.rs
    - claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs
    - claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs
    - claudine/cli/tests/level2/level2_inline_compose_mismatch_capture.rs
    - claudine/cli/tests/level2/level2_interrupt_feedback_capture.rs
    - claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs
    - claudine/cli/tests/level2/level2_lifecycle_action_forms.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/cli/tests/level2/level2_lifecycle_dispatch.rs
    - claudine/cli/tests/level2/level2_lifecycle_loop.rs
    - claudine/cli/tests/level2/level2_malformed_frontmatter_capture.rs
    - claudine/cli/tests/level2/level2_perf_capture.rs
    - claudine/cli/tests/level2/level2_prompt_reporting_capture.rs
    - claudine/cli/tests/level2/level2_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_provider_overlay_capture.rs
    - claudine/cli/tests/level2/level2_removed_validation_key_capture.rs
    - claudine/cli/tests/level2/level2_schema_parse_capture.rs
    - claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/level2/level2_stalled_generation_capture.rs
    - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
    - claudine/cli/tests/level2/level2_windows_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_wrap_ctrl_c_loop_wedge_tmux.rs
    - claudine/cli/tests/level2/level2_wrap_ctrl_c_tmux.rs
    - claudine/cli/tests/level2/main.rs
    - claudine/cli/tests/level3/level3_auto_complete_chooser.rs
    - claudine/cli/tests/level3/level3_linux_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_windows_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_wrap_ctrl_c.rs
    - claudine/cli/tests/level3/main.rs
    - claudine/cli/tests/real/main.rs
    - claudine/cli/tests/real/real_inline_write_grant.rs
    - claudine/cli/tests/real/real_opencode_yolo_subagent.rs
    - claudine/cli/tests/real/real_pi_steering.rs
    - claudine/justfile
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/tests.rs
    - features/2026-09-21-consolidated-test-binaries/pilot/apply-move.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh
docs_updated_during_phase_3:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/cli-pre-parsing.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/performance-testing.md
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/topics/signal-handling.md
    - claudine/prompts/create-new-provider.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_3:
    - features/2026-09-21-consolidated-test-binaries/claudine-cli-migration.json
    - features/2026-09-21-consolidated-test-binaries/pilot.md
    - features/2026-09-21-consolidated-test-binaries/pilot/attribute-check.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin-linux.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin-linux.md
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin.md
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-linux.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-linux.md
    - features/2026-09-21-consolidated-test-binaries/pilot/guard-scans-after.md
    - features/2026-09-21-consolidated-test-binaries/pilot/linux-l1-summary.txt
    - features/2026-09-21-consolidated-test-binaries/pilot/snapshot-check.json
    - features/2026-09-21-consolidated-test-binaries/pilot/snapshot-mapping.json
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-test-harness/SKILL.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/argv-normalization.md
    - .claude/skills/claudine/cli-pre-parsing.md
    - .claude/skills/claudine/completions/shell-completions.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/error-architecture.md
    - .claude/skills/os/build-hosts.md
---

# Implementation Log for 2026-09-21-consolidated-test-binaries (8 phases)

## Phase 1

Started and completed 2026-09-21 on macOS (`aarch64-apple-darwin`, rustc
1.98.1, cargo-nextest 0.9.136) at revision `c0f911f5a`.

Phase 1 is evidence only. The one production-file change is the spec
`status: draft-spec → planned` (R7). No package source, manifest, recipe,
config, or skill changed. The three guard files that got temporary probes
were restored with `git checkout --` and verified clean.

### What was produced

| Plan task | Artifact |
|---|---|
| R1–R8 (+ new R9) rulings | `rulings.md` |
| Spike S1 — Nextest identity | `spikes/s1-nextest-identity.md` (scratch crate in system temp) |
| Spike S2 — crate-global scan | `spikes/s2-crate-globals.md`, `spikes/s2-scan.py`, `spikes/s2-scan-output.txt` |
| Spike S3 — measurement dry-run | `spikes/s3-measurement.md`, `spikes/s3-measure.sh`, `spikes/s3/` |
| Spike S4 — snapshot mapping | `spikes/s4-snapshots.md` (scratch crate, insta 1.47.2) |
| Baseline inventory | `baseline/inventory.json` + `.md`, generated by `baseline/inventory.py` |
| `--test` consumer sweep | `baseline/test-selector-consumers.md`, generated by `baseline/consumer-sweep.py` |
| Guard before-scans | `baseline/guard-scans.md` |
| Before-listings | `baseline/listings/*.json.gz` (114) + `SHA256SUMS`, from `baseline/capture-listings.sh` |
| Disk and CI context | `baseline/disk-and-ci.md`, `baseline/deps-census.py` |
| Measurement protocol + before series | `measurements.md` |
| Phase 2 design re-read | "Adjustments from Phase 1 evidence" block added at the top of plan Phase 2 |

### Findings that changed the plan's assumptions

1. **Static identity consumers (R5 amended).** `.config/nextest.toml` holds
   exact-name overrides for two in-scope tests. After the move they stop
   matching silently, and the tests lose their extended `slow-timeout`.
   `claudine/docs/providers/dispatch-inventory.json` persists a
   `--test dispatch_inventory` selector that its test compares against.
2. **One neutral alias is required (R2).** darkmatter-cli
   `level2_harness_integrity` is gated on `terminal-tests`, but its 4 tests are
   L1-tier: CI's darkmatter-cli L1 count is 578 with the feature and 574
   without. As a module named `level2_harness_integrity` they would move to
   L2. The first inventory pass also flagged biscuit-terminal `perf_gate`. That
   was a false positive, because `perf_` is a marker only for
   `worktree-cli`, and the fix is in `inventory.py`.
3. **Identity-sensitive runtime code (R9).** Darkmatter's L2 support re-runs
   its own binary with `--exact public_entry_points::level2_render_probe_entrypoint`.
   `interpolation_literal_pipeline.rs` embeds `current_exe() --list` output,
   whose size grows from one file's tests to the whole L1 binary.
4. **Module-file resolution** (measured on the scratch crate). Bare `mod x;`
   in a non-root module file resolves under `<file-stem>/`. `#[path]` and
   `include_str!` resolve relative to the containing file's directory.
   `#[path = "../common/mod.rs"]` keeps `common`'s children resolving in
   place. An inner `#![cfg]` still works in a module. A crate-root-only
   attribute degrades to a warning.
5. **`slow_` premise (R4).** The four packages' integration targets hold zero
   `slow_` tests. The single difference between darkmatter's two slow-policy
   listings is a **lib unit test**, which is a built-in control.
6. **Spec facts.** `error_snapshots/main.rs` is auto-discovered, not declared.
   The claudine `mod common;` count is 129, not 131; the total of 187 is
   right. The spec's 51 packages / 523 targets are confirmed (74 workspace
   members in all).
7. **Pre-existing, preserved as-is (separate-defect candidates):**
   darkmatter `schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path`
   is `real_`-prefixed in a package with no real tier, so no recipe runs it.
   biscuit-terminal's 1,086 `layout_matrix__*` snapshots belong to an
   `#[ignore]`d test.

### Measurements (S3 before series, loaded host)

Clean build: 280.1 s wall, 3.63 GB peak RSS, 136 test targets, 2.78 GB of
executables (the spec observed 2.67 GB / 136). Edit-to-one-test: median
10.2 s, slowest 15.0 s. The host was **not idle** (1-minute load 10.8 at the
start, 57.1 at the end, from other sessions). Phase 3 must re-measure the
before side alternating with the after side, and needs an edit-only mode in
`s3-measure.sh` to do so (`measurements.md`).

### Requirement-to-evidence mapping

Phase 1 changes no behavior, so there is no regression test to add. Every
phase requirement maps to a re-runnable artifact instead:

| Requirement | Evidence (re-runnable) | Verified how |
|---|---|---|
| Exact Nextest identity shape, including nested modules, ignored, filter-excluded, cfg-absent, feature-gated-absent, and archive remap | S1 scratch crate listings | observed outputs recorded in `s1-nextest-identity.md` |
| Crate-global scan with dispositions | `spikes/s2-scan.py` → `s2-scan-output.txt` | every hit class dispositioned; `fn main` / `#[link]` hits read manually and confirmed to be string fixtures or imports |
| Measurement protocol works | `spikes/s3-measure.sh` | ran end to end; `--no-tests=fail` proves the edit filter selected the test every trial; target count matches the independent 136 |
| Snapshot rule and missed-mapping failure | S4 scratch crate | negative case (unmapped → both tests fail, no `.snap.new`) and positive case (byte-identical move passes under `INSTA_UPDATE=no` and `CI=true`) both observed; `INSTA_REQUIRE_FULL_MATCH` failure recorded and shown unused in the repository |
| Before-listings immutable and complete | `baseline/listings/` + `SHA256SUMS` | 114/114 non-empty; each decompressed file re-hashed and diffed against `SHA256SUMS` (clean) |
| Inventory merges metadata + manifest + listings | `baseline/inventory.py` | regenerated twice, byte-identical; workspace totals match the spec (51 / 523) |
| Consumer sweep complete and classified | `baseline/consumer-sweep.py` | regenerated, byte-identical; 30 active / 8 in-flight / 546 historical |
| Guard before-populations by path | `baseline/guard-scans.md` | probe counts equal each guard's own `governed_files` census (103/88, 42/40); all three suites passed in the same run |

Gates run: `shellcheck` on both shell scripts (clean) and `python3 -m
py_compile` on all four Python scripts (clean). `just test` / `just lint`
were **not run**. This phase changed no Rust source, manifest, or recipe in
any package area, so there is nothing for them to verify. Phase 2 builds
the toolkit with its own pytest suite under `scripts/ci`.

### Skipped or deferred

- **Cross-OS listings:** macOS only. The plan puts Linux/Windows captures in
  Phase 3 Wave 3 (per package, on-host), and platform-absent sets need them
  (S1). Three claudine-cli targets are cfg-absent on macOS. Their alias
  status is provisional, from a source scan.
- **Real-pair snapshot move:** S4 verified the mechanism on a scratch crate,
  because moving the real `claudine-cli` snapshot would be a production
  change. Phase 3 verifies the real pair.

### Unrelated working-tree changes observed (not made by this phase)

While this phase ran, `just/devops.just` (the `status` / `_status_recent`
recipes) and spec files under `tree-hugger/`, `unchained-ai/`, and
`worktree/` were modified by another process (mtime 16:59). They do not touch
`_tier_filter`, so the listings are unaffected. They were left untouched.

## Phase 2

Completed 2026-09-21 on macOS (`aarch64-apple-darwin`, cargo-nextest 0.9.136)
at revision `da6e3847d`. No file in the four package trees changed. This
phase builds and proves the toolkit only.

### Resumed work

An earlier run of this phase wrote `scripts/ci/consolidation.py` (all six
subcommands), `scripts/ci/test_consolidation.py`, `selfproof/mutation-check.py`,
and the first self-proof capture (`selfproof/capture-a/`). It ticked the four
Wave 1 tasks and "Toolkit tests", then stopped before logging anything. This
run re-verified that work instead of trusting the ticks:

- **Toolkit tests**: 62 of 62 pass. `mutation-check.py` reports 8 of 8 oracles
  red on their mutation and green on the original.
- **"Wire into the existing Python suite runner" was not done**, although the
  task was ticked. It is done now (next section).
- Only the second capture, the comparisons, and the records were missing.

### Suite wiring (the one behavior change outside the toolkit)

`test_consolidation.py` now runs in the `just ci-local` ci-infra self-test
loop (`just/ci-local.just`). The two `test_ci_local.py` fixtures that stub
every suite in that loop gained the new name, so the fixtures still model
the recipe exactly.

It is deliberately **not** in `affected_scope.py`'s `SUITE_REGISTRY`. That
registry schedules a CI companion cell on `ubuntu-latest`, and R1 rules that
no workflow runs this toolkit (spec §Out of scope). The comment in the recipe
says so, so a later "every `test_*.py` is a companion" sweep does not add it by
reflex. It costs about 4 s per `ci-local` run. The shipped-artifact tests read
frozen Phase 1 evidence (`baseline/inventory.json`, `baseline/listings/`), so
they do not drift as packages migrate.

Note: R1 and the plan say "pytest suite", but the suite is stdlib `unittest`
like every other `scripts/ci/test_*.py`. It runs as
`python3 scripts/ci/test_consolidation.py`. R1's intent, the house pattern,
holds.

### Self-proof (`selfproof/README.md`)

| Proof | Result |
|---|---|
| `capture` twice, then `compare --require-identical-digests` over all four packages | **identical**: 0 failures and 0 notes across 18 feature-set captures and 404 selector cells, every raw-listing digest equal (`selfproof/noop-comparison.{json,md}`) |
| Capture determinism | The two runs are byte-identical, so the second is kept only as `capture-b.SHA256SUMS` |
| Reproduces the Phase 1 listings | `compare --before baseline/listings --after capture-a --common-selectors --require-identical-digests`: 114 of 114 digests equal (locality fields excluded), zero identity differences |
| Reproduces the Phase 1 inventory | `inventory --listings baseline/listings` equals `baseline/inventory.json` except `generator` |

Platform-absent is `not-evaluated` in every cell. The captures are
single-host, and the tool refuses to report that set as empty.

### Requirement-to-test mapping

All in `scripts/ci/test_consolidation.py` unless noted. "Mutation" marks a
test that `selfproof/mutation-check.py` proves red against a broken tool.

| Requirement | Test(s) |
|---|---|
| Tier expressions come only from `_tier_filter`; every shipped filter parses | `ShippedFilterCorpusTests.test_every_tier_and_override_filter_parses` (passive corpus over the live `_tier_filter` and `.config/nextest.toml`), `…test_include_slow_is_captured_only_where_the_manifest_declares_it` |
| Filterset semantics (anchoring, binary is never the test path, precedence, matchers, refusal of unsupported predicates) | `FilterEvaluatorTests.*` (5) |
| Evaluator must agree with every Nextest verdict before planning | `PlanTests.test_the_evaluator_must_agree_with_every_nextest_verdict` (mutation), `…test_a_capture_taken_under_another_filter_is_refused` |
| Inventory merges metadata and manifest; fails on one-sided targets, undeclared targets, unbuilt nested roots | `InventoryTests.*` (5) |
| Contract grouping, R2 names, neutral alias, collisions, reserved names, R3 target-wide keys | `PlanTests.test_contracts_become_named_targets`, `…marker_name_that_would_move_tests_gets_a_neutral_alias` (mutation), `…alias_that_collides…`, `…reserved_names_custom_harness_and_target_wide_keys_are_refused`, `…unmarked_name_matched_by_an_unanchored_override_is_refused`, `…a_target_no_capture_lists_falls_back_to_a_source_scan` (cfg-absent targets) |
| Exact-name override → rewrite, not alias (R5) | `PlanTests.test_an_exact_name_override_is_a_rewrite_not_an_alias` |
| End to end over the real shipped artifacts | `ShippedArtifactPlanTests.test_darkmatter_cli_needs_exactly_the_one_known_alias` (reproduces `level2_harness_integrity` → `harness_integrity`), `…test_the_ruled_target_sets_for_every_package` |
| Capture folding, locality-free digest, same-build check, package-skipped binaries, empty listing ≠ zero tests, gzip round trip | `CaptureDocumentTests.*` (6; skipped-binary one is a mutation) |
| Four-way compare by exact identity, never counts | `CompareTests.test_identity_loss_masked_by_identity_gain_fails_with_the_identities` (mutation; silent count match), `…tier_move_by_module_name_fails`, `…ignore_flip_fails`, `…absent_consolidated_suite_is_lost_identities_not_zero` |
| Unmapped test / duplicate normalization fail | `CompareTests.test_an_unmapped_module_and_a_duplicate_normalization_fail` (mutation) |
| Platform-absent needs other hosts; single host is `not-evaluated` | `CompareTests.test_platform_absent_needs_other_hosts` (mutation) |
| Missing captures and one-sided selectors, tier-filter change, override rewrite noted, additions, digest requirement, CLI exit codes 0/1/2 | `CompareTests.*` (remaining 6) |
| No-op comparison identical | `CompareTests.test_an_unmigrated_tree_compares_identical_to_itself` plus the real-tree self-proof above |
| Inner attributes anywhere in the leading block; declarations with attributes | `SourceScanTests.*` (2) |
| Attribute checker: cfg moved to the declaration, cfg missing, dropped lint, crate-root-only, missing declaration, crate-global and identity detectors, `crate::`, path review | `CheckAttributesTests.*` (8; the cfg-missing one is a mutation) |
| Snapshot mapper: rule-derived paths, byte-identical move, missing row, disagreeing row, changed bytes / left-behind / `.snap.new`, unmoved changed, Insta path settings, unattributable, `#[ignore]`d readers stated per file, most-specific attribution | `CheckSnapshotsTests.*` (10; the missing-row one is a mutation) |
| Suite wiring is modeled exactly by the recipe fixtures | `scripts/ci/test_ci_local.py` (100 of 100 pass) |

No persisted value is written and read back beyond the capture documents.
Their round trip is `CaptureDocumentTests.test_round_trip_and_raw_listing_directories`,
and on the real tree the byte-identical recapture shows it.

### Gates run

- `python3 scripts/ci/test_consolidation.py`: 62 passed.
- `python3 selfproof/mutation-check.py`: 8 of 8 OK.
- `python3 scripts/ci/test_ci_local.py`: 100 passed (82 s).
- `just _lint repo-deps` (the owning package's lint; `scripts/` belongs to the
  `root` area, whose `just lint` fans out across the whole monorepo): exit 0.
- `just --list` parses. `py_compile` is clean on all changed Python.
- **Not run:** the root-area `just lint`/`just test` fan-out across every
  area. Only Python and a `just` recipe changed, and no Rust source.
- **Cross-OS:** not run in this phase. The toolkit is stdlib Python with
  `pathlib` / POSIX-normalized paths, and Phase 3 exercises it on-host on
  Linux and Windows (plan Phase 3 Wave 3). Capture's `host` is
  `platform.system().lower()` (`darwin`/`linux`/`windows`), which matches the
  baseline's file naming.

### Open for review

- Plan Phase 2 checkpoint "Reviewer sign-off that the manifest format is the
  single authoritative mapping" is left unticked. It is a human reviewer's
  call, and it is raised in the spec's `human_review_items`.

### Unrelated working-tree changes (not made by this phase)

`just/devops.just` (`_status_recent`) and
`darkmatter/features/2026-07-22-explicit-null/spec.md` (deleted) were already
modified when this run started. They were left untouched. Neither affects
`_tier_filter`.

## Phase 3

Worked 2026-09-21/22 on macOS (`aarch64-apple-darwin`, rustc 1.98.1,
cargo-nextest 0.9.136) against base revision `9621882ae`. `claudine/cli` was
unchanged since the Phase 2 capture, so `selfproof/capture-a` is a valid
before side (R8). The review packet is `pilot.md`, and this section records
how it was produced.

### What was done

| Plan task | Result | Artifact |
|---|---|---|
| Execute migration | 139 targets → 4 (`l1` 102 modules, `level2` 29, `level3` 5, `real` 3). `autotests = false`, 4 explicit `[[test]]`. No alias needed. | `claudine-cli-migration.json`, `pilot/apply-move.py` |
| Compile and list on macOS | all 5 feature sets list. `check-attributes`: 0 failures, 1 recorded disposition (a `current_exe`/`--list` false positive) | `pilot/attribute-check.json` |
| Snapshot moves | 3 moved (`tests/l1/snapshots/l1__wrap_basics__*`), 4 unaffected, `INSTA_UPDATE=no` green, 0 `.snap.new` | `pilot/snapshot-{mapping,check}.json` |
| Identity comparison (macOS) | identical, 0 failures across 5 feature sets × 22 selectors | `pilot/comparison-darwin.{json,md}` |
| Test-placement guard | layout gate added inside `l1` (`test_placement.rs`) | 4 new tests, declared as manifest `additions` |
| Area recipe/doc pass | `lint-transport` recipe, area docs, provider prompt, R9 strings, moved-path references | list below |
| macOS full validation | `just test` 7,334 passed; `just test-l2` 231+3 passed; `just lint` exit 0; tier-coverage and canonical pass | — |
| Linux | on-host before/after captures identical. L1: 2,760 passed, 3 pre-existing failures | `pilot/comparison-linux.*`, `pilot/linux-l1-summary.txt` |
| Native Windows | **not done**: cross-compile only (compile evidence) | `pilot.md` §Per-host |
| WSL2 | **pending**, recorded with the exact missing evidence | `pilot.md` §Per-host |
| After measurement series | alternating pair, kache off | `measurements.md`, `pilot/measure/` |
| Skip-baseline check | `.github/ci/ci-baseline.toml` holds only `schema_version = 3`, empty | — |
| Guardrail decision | one binary per contract (+0.30 s median edit latency) | `measurements.md` §Seam decision |
| Pilot report | done | `pilot.md` |

### Decisions made during the move

- **`mod common;` → `use crate::common;`** in each moved file, with any
  attribute or doc line above it kept. Each root declares `common` once through
  `#[path = "../common/mod.rs"]`. `use common::…` statements then resolve
  through the import, with no further edits. Three imports needed the `cfg`
  their uses already had, because an unused import warns where an unused
  `mod` (under `common`'s `#![allow(dead_code)]`) did not. That showed up on
  macOS for `wrap_ctrl_c_windows.rs`, and in the Windows cross-compile for
  `loop_initialize_state.rs` and `shipped_prompts.rs`.
- **Inner `#![cfg]` removed from the module file** and carried only on the
  declaration. `check-attributes` would also accept a duplicate, but "becomes
  an outer attribute" (spec §4) reads as a move.
- **`#[path = "common/…"] mod x;` copies** (`source_scan` in the two guards,
  `host_tools` in the perf bench) became `use crate::common::x;`. That is the
  §3 "replace repeated helper-module declarations" edit. Their comments
  explained why a per-file binary avoided `mod common`, and they were
  rewritten because they no longer applied.
- **`error_guards/` moved beside `l1/error_guards.rs`** (R2). Its `#[path]`
  became a plain `mod source_scan;`, which resolves there, and the two
  area-relative allowlist constants were repaired.
- **The layout gate** (acceptance 12) walks the Rust module graph from the
  `[[test]]` roots `Cargo.toml` declares. It requires every `.rs` under
  `tests/` (except `fixtures/`, `snapshots/`) to be reached. One rule covers a
  stray top-level file, an undeclared `tests/<x>/main.rs`, and an undeclared
  module, including one inside a helper directory. It also requires
  `autotests = false` and an explicit `path` on every `[[test]]`. A
  `cfg`-gated declaration counts as declared, and `mod x;` in prose or a
  string does not.
- **Narrow-test guidance** is `just test-cli <module>::` (run in `claudine/`).
  Verified: nextest intersects a positional filter with `-E`, so
  `-E <L1> error_guards::` lists exactly the 8 `error_guards` tests. For a
  compile-only check of a Windows target, `--test level3` names the new target
  (`signal-handling.md`), which acceptance 11 allows.
- **`dispatch-inventory.json`**: re-blessing through the new command also
  rewrote about 50 `line` fields that were already stale at `HEAD` (the compare
  ignores lines). To honor R9 ("the committed diff must be exactly the
  `regenerate` line"), the file was restored and only that line edited. The
  test passes, and fails with the old line.

### Requirement-to-test mapping

| Changed behavior | Test / check (targeted) | Red-then-green shown |
|---|---|---|
| Every old test keeps one identity, tier, ignore state, and platform presence | `consolidation.py compare` macOS, Linux, macOS+Linux | the toolkit's own oracles (Phase 2 mutation check), identical here |
| Override `test(=…)` still selects its test | `compare` selector `override-ci-5` / `override-default-0` (1 selected before and after) | Phase 2 `test_an_exact_name_override_is_a_rewrite_not_an_alias` |
| Former file-level `cfg` on the declaration | `check-attributes` | Phase 2 mutation |
| Snapshots found at the new path | `INSTA_UPDATE=no just test-cli wrap_basics::` + `check-snapshots` | yes: an unmoved name makes `help_lists_wrapper_subcommands` fail |
| Acceptance 12 layout gate | `test_placement::every_test_source_is_compiled_by_a_declared_target`, `layout_gate_rejects_stray_roots_and_undeclared_modules`, `layout_gate_requires_explicit_targets`, `module_declarations_read_attributes_visibility_and_path` | yes: planted `tests/stray_probe.rs` + `tests/l1/orphan_probe.rs` → red naming both, then green |
| Guards scan the same files | temporary probes, `pilot/guard-scans-after.md` | populations compared by path; the only additions are the 4 roots |
| Guard assertions follow the move | `spawn_site_guard::*` (20), `error_guards::*` (8), `test_placement::*` | the renamed-path assertions are positive (`contains`), so a wrong path fails |
| Persisted `regenerate` selector | `dispatch_inventory::dispatch_inventory_matches_committed_file` | yes: the old line fails it |
| `lint-transport` recipe | `just lint` (runs `just _test claudine-cli error_guards::`: 8 tests) | — |
| L3 / real stay opt-in | L3 and real filters without the opt-in env | every test printed its skip reason |
| Cross-OS module graph | Windows cross-compile of all 4 targets; Linux on-host build and listings | Windows cross-compile warnings found and fixed (2 imports) |

"Passive corpus" and "shipped artifact end-to-end" are covered by the layout
gate reading the real `Cargo.toml` and tree, and by `compare` over real
captures. No persisted value is written and read back.

### Gates run

- `cargo check --tests` and `cargo clippy --all-targets -D warnings` with
  `terminal-tests,real-tests`: clean. `_lint` itself uses no features, so it
  skips the three feature-gated targets. That was also true of their
  per-file predecessors.
- `just lint` (claudine): exit 0. `just test` (claudine): 7,334 passed,
  9 skipped. `just test-l2`: 231 + 3 passed.
- `just check-tier-coverage claudine`, `just check-canonical claudine`: pass.
- `python3 scripts/ci/test_consolidation.py`: 62 passed (unchanged toolkit).
- `shellcheck spikes/s3-measure.sh`: clean.
- GitNexus `detect-changes --scope all`: risk low, 0 affected processes.
- Not run: `just test-l3` and `just test-real`. Both take focus or bill real
  providers, and their opt-in was verified instead. Also not run: the
  root-area fan-out.

### Pre-existing failures (not caused by this phase)

- **Linux:** `sequence_groups::{parallel_body_lines_carry_their_own_tasks_bar_color,
  parallel_group_members_are_attributed_across_both_channels,
  serial_and_parallel_group_frames_share_one_left_edge}` fail on
  `build-linux` on the unmigrated base as well, identically. They pass on
  macOS. Separate-defect candidate.
- **Windows cross-compile warning** `GENERATED_MARKER` never used
  (`sequence_initialize_include_preflight.rs`): the constant is only read by
  a unix-gated test, so the old per-file crate warned too.
- Orphan snapshots and a stale `--test level2_pty_tests` doc line: `pilot.md`
  §Defects.

### Host problems met (recorded, not worked around silently)

- **kache was on in the Phase 1 measurement and in the first Phase 3
  attempts.** `unset RUSTC_WRAPPER` does not beat
  `~/.cargo/config.toml`'s `rustc-wrapper = "kache"`, and `build-linux`'s
  `/usr/local/bin/cargo` shim enables kache on any cold target dir. The first
  after "clean" build took 33.8 s, and the first Linux run hit kache's
  read-only hardlink error. Fixed with an explicitly empty
  `RUSTC_WRAPPER=""` (plus removing the kache `cc` shims locally). Both runs
  were discarded and re-done. Recorded in the `os` skill
  (`build-hosts.md` §Compiler cache) and in `measurements.md` §Correction.
- **`build-linux` cross-check lock** has been held since 2026-09-14 by another
  branch's `nightly-reward-spike`. Linux evidence came from a private
  `--shared` clone plus a bundle, per the `os` skill. The scratch clone was
  deleted afterward, and the lock was left alone.
- **`build-win-native` `W:` has 43 GB free**, under the 50 GiB preflight, and
  **`build-win` (WSL) resets SSH at key exchange**, the `os` skill's full-`W:`
  signature. No Windows build was attempted. Freeing `W:` is the owner's
  call.

### Changed outside the four target directories

`.config/nextest.toml` (R5 override), `claudine/justfile`
(`lint-transport`), `claudine/cli/Cargo.toml`,
`claudine/cli/tests/common/mod.rs` (one stale comment),
`claudine/docs/providers/dispatch-inventory.json` (one line), area docs
(`claudine/docs/topics/*` ×9, `claudine/prompts/create-new-provider.md`),
comment-only path updates in `claudine/cli/src` ×4 and
`claudine/lib/src/provider` ×2, skills (`claudine` ×7 path references,
`biscuit-test-harness` ×1, `os/build-hosts.md`), and this feature's
`spikes/s3-measure.sh` (build/edit modes, kache really off).

Comment-only and doc-only changes should go in a separate commit from the
structural move (AGENTS.md scope discipline). The source-file comment edits
are 1:1 path swaps in `//!`/`///` lines.

### Unrelated working-tree changes

None observed. The tree was clean at `9621882ae` when this phase started.

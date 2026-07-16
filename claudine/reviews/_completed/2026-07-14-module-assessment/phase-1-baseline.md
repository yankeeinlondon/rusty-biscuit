# Phase 1 behavior and measurement baseline

Captured on 2026-07-14 before the orchestration and process-layer refactors in
later phases. This is a measurement input, not an exception list or a claim
that every listed hotspot needs structural change.

## Behavior coverage

The Phase 1 additions pin the boundaries that later phases will change:

- `composition::lifecycle::runtime::tests::phase_1_terminal_routing_matrix_pins_precedence_and_control`
  covers clean/action-error outcomes, evaluation-error precedence, control
  suppression, terminal catch ordering, and loop-gate routing.
- `wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline`
  drives the real composition path and pins target selection, launch workspace,
  environment/MCP construction, argv and system-prompt preparation, initialize
  routing, and provider handoff ordering.
- `drift::committed_generated_artifacts_match_phase_1_byte_baseline` checks the
  14 generated artifacts in `generated-artifact-baseline.json` with
  biscuit-hash XXH64. The existing generator drift tests continue to prove
  regeneration convergence.
- `generate_ux::clean_check_report_summary_matches_phase_1_snapshot` pins the
  human-facing clean report order, while
  `composition::error::tests::phase_1_plain_composition_error_block_snapshots`
  pins representative lifecycle and selection error blocks at 80 columns with
  color disabled.

Existing focused tests provide the rest of the characterization matrix and run
unchanged alongside the Phase 1 additions:

| Contract | Characterization coverage |
|---|---|
| Lifecycle ordering, terminal redesignation, evaluation/action errors, finalize once | `loop_control::tests::terminal_event_tests` and `level2_lifecycle_dispatch` |
| Retry budgets and `provider_launched` re-entry | `dispatch_retry_from_failure_continues_and_resets_guard`, `dispatch_retry_exhausts_after_budget`, `emit_blocked_finalize_pre_launch_runs_blocked_then_finalize_with_err`, `emit_blocked_finalize_post_launch_selects_failure` |
| Resume/session availability | `dispatch_resume_with_session_seeds_prompt_state`, `dispatch_resume_without_session_aborts_typed`, and `level2_lifecycle_control` |
| Proxy chains and reset behavior | `dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run` and the proxy cases in `level2_lifecycle_control` |
| Stop, error, and unsupported setup recovery | `dispatch_stop_falls_through`, `dispatch_error_aborts_without_changing_stop_semantics`, plus initialize retry/resume cases in `wrap_compose_validation` |
| Inherited/captured normal completion and timeout reaping | `exec::spawn::tests` (`run_child_*`, `run_child_capture_*`) |
| Semantic-stream normal completion and summary projection | `wrap_structured_stream` and `exec::termination::tests::early_termination_*` |
| Watchdog and completion termination/reaping | `wrap_watchdog_timeout`, `early_termination_signal_reaps_child_and_reports_timed_out`, `completion_termination_reaps_child_and_reports_completed` |
| User interruption | `level2_wrap_ctrl_c_tmux`, `level2_interrupt_feedback_capture`, and `level3_wrap_ctrl_c` |

The Windows completion/Ctrl+C cases are compile-gated with `cfg(windows)` and
have the package recipe `just test-windows-ctrl-c`; runtime confirmation remains
the Windows CI host's responsibility.

## Test-placement inventory

The assessment's structural scan found at least 90 inline `mod tests` files over
the documented thresholds: 9 exceeded both the approximately 800 production
line and 300 test-line thresholds, 11 exceeded only production, and 70 exceeded
only the inline-test threshold. Generated sources and sibling/integration test
files are measurement categories, not actionable inline-test violations. Phase
5 will replace this assessment-time scanner with the portable enforced analyzer.

Largest extracted test-only inputs at capture time:

| File | Raw lines |
|---|---:|
| `lib/src/composition/lifecycle/tests.rs` | 3,936 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests.rs` | 3,413 |
| `lib/src/composition/lifecycle/executor/tests.rs` | 2,340 |
| `lib/src/stream/logs/opencode/bridge/tests.rs` | 2,283 |
| `rendezvous/daemon/src/session_log/tests.rs` | 2,002 |
| `lib/src/composition/looping/engine/tests.rs` | 1,946 |

## God-file measurement

Command: `hug god-files --json claudine`, run from the repository root.

| Category | High | Moderate | Total |
|---|---:|---:|---:|
| Generated provider/signal sources | 1 | 10 | 11 |
| Test-only files | 14 | 38 | 52 |
| Production/mixed files | 27 | 124 | 151 |
| Total | 42 | 172 | 214 |

The generated category is the ten `provider/<slug>/data.rs` files plus
`signals/generated.rs`. Other generated artifacts are below the `hug` risk
reporting threshold. The production/mixed category intentionally retains files
with large inline test blocks; Phase 5's analyzer will split production and test
counts accurately.

The actionable High-risk production inputs identified by the assessment are
the two orchestration roots, wrapper spawn/termination, rendezvous
sync/service/session-log, generator emission, and composition error rendering.
Protocol-specific and declarative files remain inputs for judgment rather than
automatic split targets.

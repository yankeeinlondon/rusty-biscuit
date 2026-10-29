---
kind: implementation-log
---

# Test audit: 2026-10-24-sequence-containers

## Changes and counts

Added **two genuine coverage test functions/Nextest identities** (gross and
net increase both two), strengthened **zero** existing tests, and modified
**zero** slow tests. No cases were split, removed, ignored, retried, or moved
between tiers. No production behavior changed.

The new normal-CLI tests cover runtime-only `retry.with` across both nested
container implementations:

- `nested_sequence_retry_overlay_is_runtime_only_and_clears_abandoned_results`
  fails after one child has completed, retries with changed values, and checks
  the exact initialize/start/failure/finalize trace and native nested results.
  A retained abandoned child output, repeated initialize/finalize, missing
  overlay, or persistent source modification fails the test.
- `nested_group_retry_overlay_reaches_legs_without_persisting_or_duplicate_results`
  proves the retry overlay reaches a leg's preparation and lifecycle, while
  exactly one final named result is published. It also checks event order and
  unchanged source text through the existing isolated invocation helper.

These complement the existing in-process retry tests and root-sequence CLI
test; the previous coverage did not directly exercise both nested kinds
through normal command dispatch.

Corrected the stale `SequenceSource::is_dynamic` comment: an authored empty
sequence is legal, just as the existing parser and empty-container tests
already establish. Code behavior was preserved.

## Requirement coverage

| Acceptance criteria | Coverage and boundary |
| --- | --- |
| 1, 18: nesting, references, cycles, ordered concurrent legs, empty/skipped containers and collisions | Task/preflight tests, frozen-graph CLI cases, shipped example CLI invocation, and deterministic sibling barriers exercise graph construction and runtime results. |
| 2, 10, 15: removed keys/names/budget commands | Full sequence grammar matrix, CLI error cases, binding rename/property-presence tests; failures assert typed diagnostics and no provider/effect. |
| 3, 4, 12, 13, 16: event ordering, propagation, body ownership, finalization and proxy publication | Existing root CLI traces and sequence/group task contracts assert exact order, once-only cleanup, native arrays/dictionaries, withheld results, and target-only proxy cleanup. |
| 5, 6, 19: fail-fast, timeout versus cancellation, cleanup and state | Deterministic task runner contracts and CLI opted-provider cancellation test exercise admission, state withholding/ordered merge, timeout retry, secondary cleanup failures, and forbidden cancellation recovery. |
| 7: Ctrl-C and settled descendants | Existing tmux terminal test sends the interrupt through the real foreground terminal, checks exit 130, descendant process death, no later work, and exact innermost cleanup order. |
| 8, 9, 11: group/sequence roots, positions, shorthand, lexical restoration | Binding tests and nested task/CLI contracts check root shadowing, native values, run IDs, independent accumulators, and sibling restoration. |
| 14: lift-ready lifecycle parser | Existing library import guard and deliberate-invalid-import control are selected. |
| 17: shipped migration | Normal L1 shipped-container corpus prepares each public plan/preflight graph and validates lifecycle stacks; the shipped container example runs through the real CLI. Earlier pipeline audit S10 also passed the opt-in internal lifecycle/route corpus; this slice changes no shipped prompt. |
| 20: retry retention, adjacency and filesystem rereads | Existing repeated retry/exhaustion contracts, frozen graph CLI tests, and deterministic isolated-memory/shared-filesystem sibling test; new nested CLI tests add normal-dispatch coverage. |
| 21: nested approval positions and closed membership | Preflight all-first-ancestry tests, native-recorder event-time shell tests, and changed-source/root-retry/proxy refusal CLI cases. S13's new persisted-policy/native-argument regression remains in place. |
| 22: Messenger run protection | Seven focused persisted-record/real-process tests cover live lock refusal, finished/interrupted/stopped statuses, elapsed timeout, explicit stale recovery, live cleanup/promotion refusal, legacy exhausted records, and the load-bearing-field matrix. |
| 23: runtime-only container retry overlays | Existing in-process sequence/group contracts, root CLI test, and both new nested normal-CLI tests. |

Internal transformations use F1 tests with independent expected structures;
normal CLI tests exercise actual command dispatch and isolated filesystem
state rather than replacing the container scheduler. Native subprocess
termination and OS-backed fleet locks are observed through real processes.
The terminal interrupt and concurrent rendering checks cross the real tmux
medium at F2. No requirement here promises OS keyboard encoding or live
agent-provider behavior, so no L3/live-provider test was added. All audio is
suppressed with the existing fixture's private spool and absence assertion.

## Verification

From `claudine/`, canonical Nextest-backed L1:

```sh
BISCUIT_TEST_FILTER='test(sequence_containers::) | test(sequence_deep_cancelled::) | test(sequence_frozen_graph::) | test(sequence_errors_cli::) | test(sequence_grammar_matrix::) | test(composition::sequence::task::tests::) | test(composition::sequence::preflight::tests::) | test(composition::lifecycle::bindings::) | test(lifecycle_lift_ready_guard::)' just test
```

**716/716 passed**, maximum **2.214 seconds**, total execution 14.033 seconds.
The two new tests also passed in the narrower 22-test container run at
1.263 and 1.277 seconds. Final log: `/tmp/pipeline-s14-l1-final.log`.

From `messenger/`, before starting L2:

```sh
BISCUIT_TEST_FILTER='test(a_live_runner_refuses_a_second_run) | test(the_runner_records_finished_interrupted_and_stopped) | test(a_stale_active_run_is_recovered_explicitly_and_never_silently) | test(cleanup_and_promotion_are_refused_during_a_live_run) | test(a_ledger_era_exhausted_run_is_readable_resumable_and_never_promotable) | test(the_run_record_matrix_is_typed_for_every_load_bearing_field) | test(the_runner_stops_a_sequence_that_exceeds_the_elapsed_time_limit)' just test
```

**7/7 passed**, maximum **1.590 seconds**. Log:
`/tmp/pipeline-s14-messenger.log`.

After both L1 runs completed, from `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(level2_ctrl_c_cancels_group_descendants_and_finalizes_innermost_first) | test(level2_mega_merge_s10_parallel_proxy_failure_task_integrity) | test(level2_interleaved_shell_streams_arrive_live_and_attributed_in_tmux) | test(level2_zero_step_sequence_renders_a_styled_notice_in_tmux)' just test-l2
```

**Scope correction:** `BISCUIT_TEST_FILTER` overrides L1 only; `_test_l2`
ignored this intended narrow selection. The command therefore ran the full
existing L2 tier: **289 CLI tests and three generator tests passed**. No
selected test was skipped and no test exceeded five seconds. This was a
broader run than intended, not evidence that the filter worked. No additional
full run is needed for this slice.

The maximum was **4.997 seconds** for the existing
`level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop` test.
The specific container boundary checks took:

- Ctrl-C cleanup/descendant termination: **1.437 seconds**.
- Live interleaved attributed shell streams: **3.294 seconds**.
- Parallel proxy/failure task integrity: **3.434 seconds**.
- Empty-sequence terminal notice: **1.366 seconds**.

The CLI tier took 59.218 seconds overall; the generator tier took 0.792
seconds. Log: `/tmp/pipeline-s14-l2.log`. Existing background harness defaults
were retained; no focusing L3 action or live provider was run.

Final result: **723 distinct selected L1 tests and 292 L2 tests passed**.
`git diff --check` passed. No other-OS execution or lint run is claimed by
this slice; the coordinator owns final broader gates. The pre-existing
native-Windows console Ctrl-C capability gap in the Unix tmux test is not
claimed as resolved by a macOS pass.

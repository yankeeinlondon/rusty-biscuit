---
$schema: feature-review.yaml
ready: false
findings:
  - title: Ctrl+C lifecycle behavior lacks verification at the required terminal levels
    priority: high
  - title: Teardown and grace regression tests leave explicit contract branches unprotected
    priority: medium
  - title: Signal fixtures depend on scheduling and leave child processes behind
    priority: medium
human_review: false
reviewed_by: codex/default
created: 2026-09-26T13:31:05-07:00
spec: 2026-09-23-opencode-failure-lifecycle/spec.md
implemented: true
next: 2026-09-23-opencode-failure-lifecycle/review-2.md
implemented_by: claude/default
log: claudine/fixes/2026-09-23-opencode-failure-lifecycle/implementation-log.md
description: A **fix** review of `2026-09-23-opencode-failure-lifecycle/spec.md`
fix: 2026-09-23-opencode-failure-lifecycle/review-1.md
---

# Review 1

The fix is **not production ready**. F1–F3 are implemented in the expected execution paths, and the reference-based error enrichment is appropriately narrow. The blockers are verification gaps and unreliable regression fixtures. No additional design decision or human-only acceptance step is needed to resolve these findings. Cross-OS execution evidence is not a readiness condition for this review.

## Findings

### High — Ctrl+C lifecycle behavior lacks verification at the required terminal levels

The new tests in `claudine/cli/tests/l1/wrap_sigint.rs:153`, `:339`, and `:365` deliver `libc::kill(pid, SIGINT)` directly to a subprocess. They are Level 1: neither a terminal encoder nor foreground process-group delivery participates. They cannot establish F1's keyboard behavior during orphan teardown or F2's repeat-keypress behavior during terminal lifecycle work.

Existing Level-3 coverage in `claudine/cli/tests/level3/level3_wrap_ctrl_c.rs:269` and `:293` exercises one chord while the wrapped provider is still running. It does not enter either new window. Existing Level-2 feedback coverage in `claudine/cli/tests/level2/level2_interrupt_feedback_capture.rs:60` checks the earlier `interrupt received` notice, not the new grace or grace-expired notices.

Add focused Level-3 scenarios for interrupting teardown and for repeat Ctrl+C during short and long terminal lifecycle work, asserting lifecycle completion or bounded exit 130 as appropriate. Capture the new notices through a real terminal at Level 2 or above. Preserve the repository's focus constraints; direct signal delivery must not be relabeled as OS keyboard verification. Keep the deterministic Level-1 tests for the underlying process logic.

### Medium — Teardown and grace regression tests leave explicit contract branches unprotected

`compose_sigint_during_orphan_teardown_runs_failure_lifecycle` sends exactly one signal (`wrap_sigint.rs:220`). The first press always takes `PressRung::Notice`, regardless of `WaitLoopActiveGuard`. Removing that guard from `kill_process_group` would therefore leave this test passing even though the repeated-interrupt defect could return. Its comment claims protection against a repeat press that the test never sends.

The same fixture keeps its orphan alive until interrupted. It does not prove the primary F1 latency fix: after SIGTERM reaches a surviving descendant that then exits, teardown must return promptly without any Ctrl+C. The spec's recorded manual timing is useful historical evidence, but is not an automated regression test for the current code.

F2's subprocess tests cover two presses during `failure` only. They do not verify that no press or one press permits work longer than 500 ms, that later presses cannot postpone the deadline, or that the scope is entered for `success`, `blocked`, and `finalize`. The pure rung tests accept a supplied boolean and cannot prove those lifecycle entry points set it.

Add bounded, synchronized Level-1 tests for early group disappearance and repeat-press deferral, plus representative grace controls for zero/one/additional presses. Verify all four terminal-event scope entries at the cheapest suitable boundary. The teardown guard test should fail if that guard is removed; the empty-group test should fail if polling is replaced by a full grace-period sleep.

### Medium — Signal fixtures depend on scheduling and leave child processes behind

`press_twice_during_failure_stack` (`wrap_sigint.rs:263`) waits for `failure-started`, sleeps a fixed 50 ms between signals, and uses a 200 ms shell sleep for the short case. If the test process is descheduled after observing the marker, the second signal can land after the lifecycle has already finished. The first marker is also written before entering the shell action. These are timing assumptions, not synchronization with the state being tested; `libc::kill` results are ignored.

The long case deliberately redirects output to files because force-exiting the wrapper leaves its five-second shell child alive (`wrap_sigint.rs:288`). The fixture never tracks or terminates that child. Redirecting its output avoids a pipe wait but does not provide process cleanup. A panic in either readiness loop also drops a plain `std::process::Child`, which does not kill the wrapper. In the orphan fixture, the provider exits immediately after spawning the background subshell, without waiting for that subshell to install its TERM trap; teardown can beat trap installation and the marker then never appears.

Use explicit readiness/release barriers for the orphan and lifecycle action, acknowledge the first interrupt before sending the second, and check signal delivery. Own the spawned process trees with cleanup that runs on success and panic, including after a deliberate wrapper force-exit. Measure the grace interval from confirmed delivery while retaining a justified scheduling allowance.

## Requirement-to-verification map

| Requirement | Strongest relevant coverage present | Required verification / assessment |
|---|---|---|
| F1: promptly finish teardown when the group empties | Historical manual OpenCode timing; no dedicated automated no-interrupt case | Level 1 process test missing; finding 2 |
| F1: Ctrl+C cuts teardown short and preserves failure/finalize | Level 1 direct-SIGINT subprocess test, one press | Level 1 repeat-press case and Level 3 keyboard scenario missing; findings 1–2 |
| F2: short terminal work completes after repeat Ctrl+C | Level 1 subprocess test for `failure` | Level 3 scenario missing; fixture timing unreliable; findings 1 and 3 |
| F2: long terminal work exits 130 after 500 ms | Level 1 subprocess test for `failure`, timing bounds 400 ms–3 s | Level 3 scenario missing; child cleanup missing; findings 1 and 3 |
| F2: no deadline for zero/one presses; later presses do not extend it | Rung-selection unit assertions cover only part of this behavior | Level 1 process controls missing; finding 2 |
| F2: mark success, blocked, failure, and finalize while executing | Source wiring; subprocess coverage enters `failure` and subsequently `finalize` | Scope-entry assertions for all four events missing; finding 2 |
| F2: announce grace and expiration | Level 1 stderr substring assertions | Real-terminal capture at Level 2 or above missing; finding 1 |
| F3: extract the stdout reference | Level 1 parser test `a_generic_error_records_its_opencode_ref` | Appropriate level |
| F3: join only a matching stderr cause and set the error class | Level 1 matching/unrelated-reference bridge tests | Appropriate level |
| F3: expose the cause through lifecycle `err.msg`/`err.variant` | Level 1 fake-provider CLI test `opencode_unknown_model_surfaces_its_cause_as_the_failure_error` | Appropriate level; filesystem assertions need no terminal |

## Validation and scope

Reviewed the current implementation, its introducing signal-handling commit, parser/protocol/bridge wiring, lifecycle execution entry point, and relevant L1/L2/L3 test sources. No implementation files were changed.

Both CLI test modules are declared by `cli/tests/l1/main.rs` and compiled by the manifest's `l1` target despite `autotests = false`. The new library tests are connected through their unit-test modules. Their names are selected by L1. Terminal targets declare `terminal-tests`, which CI enables, and live `test-l2`/`test-l3` recipes exist. `just check-tier-coverage claudine` passed with zero stranded tests.

Focused checks on this macOS host:

- `just test-cli wrap_sigint::`: passed all four selected tests, including the three new signal/lifecycle regressions. This passing run does not eliminate the scheduling and coverage issues above.
- `just test-library a_generic_stdout_error`: passed both matching-reference and unrelated-reference tests.
- `just test-cli opencode_unknown_model_surfaces_its_cause_as_the_failure_error`: passed the lifecycle error-projection test.
- Review frontmatter/iteration assertions passed; whitespace checks reported no errors.

All seven selected tests passed. Builds emitted a nonfatal macOS linker warning about the size of `__eh_frame`. No Level-2 or Level-3 tests were executed for this review; the missing scenarios above are established by inspecting the declared suites, not by treating a skipped test as evidence. The existing Level-3 wrapper test explicitly raises its window, so it was not run under this session's no-focus constraint.

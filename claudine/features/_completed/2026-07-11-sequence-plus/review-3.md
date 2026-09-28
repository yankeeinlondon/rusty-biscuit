---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T11:05:33-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-3.md
previous: 2026-07-11-sequence-plus/review-2.md
next: 2026-07-11-sequence-plus/review-4.md
---

# Review 3: Sequence Plus

## Verdict

The feature is **not ready for production**. Review 2's Level-1 compile failure
and formal-document normalization divergence are closed, and the task-stream
repair now has focused Level-1 coverage plus baseline Level-2 shell rendering.
The remaining blockers are release-critical: sequence interruption is disabled
on Windows and the Windows provider wait machinery is not safe for parallel
prompt tasks; the sequence-specific Level-3 keyboard test has never passed; the
new Level-2 suite does not exercise the prompt/provider output paths it was
added to validate; and the required platform/release gates are still red or
unrun.

## Findings

### 1. High — Windows cannot satisfy sequence Ctrl+C or parallel fan-out

The sequence owns one shared `interrupted` flag, and every running shell task
polls it. The only code that can set that flag is compiled under `#[cfg(unix)]`;
the non-Unix branch deliberately installs no handler
(`cli/src/commands/wrap/sequence/mod.rs:158-175`). On Windows, Ctrl+C therefore
cannot stop a running shell task, stop at a step boundary, suppress a later
step, or produce the specified exit `130`.

Prompt tasks do not repair the sequence-level gap. The reachable simple
provider wait path is a blocking `child.wait()` that always labels completion
as `ProcessTermination::Completed`
(`cli/src/commands/wrap/exec/termination/windows.rs:22-34`), selected whenever
the semantic spawn has neither an early-termination receiver nor an enabled
watchdog (`cli/src/commands/wrap/exec/spawn/semantic.rs:498-524`). Because the
child is created in a new Windows process group, the terminal chord does not
reliably terminate it as an incidental side effect.

The advanced Windows wait path also assumes only one wrapped child exists in a
process and stores press/force-kill state in process-global atomics
(`windows.rs:83-97,188-190`). That assumption contradicts the implementation:
a parallel group launches multiple prompt tasks in sibling threads
(`lib/src/composition/sequence/task/group.rs:239-289`), and each prompt runs the
wrapper pipeline in the same Claudine process
(`cli/src/commands/wrap/sequence/task_run.rs:265-372`). Concurrent wait loops
reset the same counters and install/remove the same console handler. On a second
press, the first loop to set `CONSOLE_FORCE_KILL_SENT` prevents siblings from
force-terminating their own Jobs (`windows.rs:249-299`). This cannot provide the
specified all-child fan-out.

Required change: install a real Windows sequence-scoped console handler that
sets the shared scheduler flag, route the simple provider wait through the same
signal-aware machinery, and redesign Windows child registration around one
process-scoped interrupt coordinator that tracks every active child/Job. Add a
Windows-host integration test for a blocking parallel group that verifies every
child dies, later work does not launch, the shell regains control, and the exit
code is `130`.

### 2. High — The sequence-specific OS-keyboard requirement still has no passing Level-3 evidence

`level3_sequence_ctrl_c_fans_out_to_parallel_children` is correctly shaped: it
opens a real WezTerm window, starts SIGINT-immune parallel children, injects a
Quartz Ctrl+C chord with `cliclick`, and checks fan-out, later-step suppression,
shell recovery, and exit `130`. The validation record states that it failed four
consecutive real runs because the chord never reached WezTerm
(`validation-matrix.md:260-284`). The passing substitute uses
`tmux send-keys C-c`, which is Level 2 because it bypasses the terminal
emulator's keyboard encoder.

Under the review rubric, the existence of a red Level-3 test is not Level-3
verification. The strongest passing sequence-specific evidence remains Level 2,
so the user-observable claim "when the user presses Ctrl+C" is still at the
wrong level.

Required change: make the Level-3 harness reliably focus the spawned pane and
record at least one green sequence run through the canonical `just test-l3`
gate. Keep the SIGINT-immune children; they are what prevents a false positive
from terminal process-group delivery. Add equivalent OS-keyboard evidence on
other supported hosts where the platform tooling permits it, especially after
the Windows implementation is corrected.

**Resolution (2026-07-18): CLOSED — green run recorded.** The required change
was made where the finding said it should be: in the harness, not the test.
`focus_spawned_pane` now polls until WezTerm is genuinely the frontmost macOS
application instead of sleeping a fixed 200ms after `AXRaise`, and the Ctrl
chord goes through AppleScript `keystroke … using control down`, which carries
the modifier flag on the same key event rather than racing it. With those in
place `level3_sequence_ctrl_c_fans_out_to_parallel_children` passes through the
canonical `just test-l3` gate: every child interrupted, later step suppressed,
exit `130`. The SIGINT-immune children are unchanged. Both harness changes are
still uncommitted on this branch — see `validation-matrix.md` → "Execution
status".

Follow-on from the same review: the tier now refuses to run unattended.
`just test-l3` exits non-zero without a TTY unless `BISCUIT_L3_TAKE_FOCUS=1`
authorizes it, and a new L1 guard,
`test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files`, keeps
`SpawnVisibility::Foreground` and `focus_spawned_pane` out of any file not named
`level3_*`. Remaining open: equivalent OS-keyboard evidence on Windows, which
still depends on finding 1's console-handler work.

### 3. High — Prompt/provider task attribution is still verified at the wrong terminal level

Review 2 identified three prompt-specific bypasses: assistant idle flush,
post-parser-failure raw fallback, and provider status/reasoning. Each now has a
focused Level-1 test
(`cli/src/commands/wrap/exec/watchdog/spawn.rs:518-556`,
`cli/src/commands/wrap/exec/spawn/semantic.rs:667-696`, and
`cli/src/commands/wrap/stream_io.rs:315-350`). Those tests prove routing to the
decorated sink and preserve stdout/stderr separation.

The new Level-2 suite never drives any of those paths. Its serial and parallel
fixtures contain only `shell: printf ...` tasks
(`cli/tests/level2_sequence_task_stream_capture.rs:60-108`); the remaining
fixture is a zero-step no-op. Consequently the real pane proves the common
bar renderer, shell body colors, geometry, no-color behavior, glyph fallback,
and zero-step styling, but not that a prompt task's assistant/status/fallback
lines retain their task attribution after the provider protocol, stdout/stderr
merge, and terminal rendering.

The specification's promise applies to "every subsequent status/output line,"
and this review requires terminal-visible attribution/color claims at Level 2.
The strongest path-specific evidence for all three repaired provider paths is
still Level 1, so the mismatch remains a high-severity verification gap.

Required change: add a Level-2 parallel prompt fixture using a deterministic
provider stub. Emit identifiable assistant data and status/reasoning events,
force semantic fallback, and hold a partial Markdown frame through idle flush;
then capture the real pane and assert every line retains the owning task's bar
or textual label. Keep the existing Level-1 exact-channel tests.

### 4. Medium — The Windows Job Object is leaked and its kill-on-close contract is false

The advanced Windows wait creates a raw Job Object `HANDLE`
(`cli/src/commands/wrap/exec/termination/windows.rs:192-205`) but never wraps it
in an owning type and never calls `CloseHandle`. The nearby comment claims the
handle closes "by normal Drop," but a bare `HANDLE` has no such behavior.

A long sequence therefore leaks one Job handle per provider child. More
importantly, a provider that exits while leaving descendants behind does not
trigger `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` at that step boundary; the leaked
handle keeps the Job alive until the Claudine process exits. This undermines
the process-isolation/termination contract and can accumulate live descendants
and handles across a sequence.

Required change: own the Job handle with the Windows crate's RAII handle type
or an explicit guard that closes on every return and error path. Add a
Windows-host regression that proves a descendant is terminated when the wait
scope ends, plus a repeated-run handle-count check if practical.

### 5. High — The required release and cross-platform gates are still not green

Acceptance Criteria 8 and 11 require macOS, Windows, and Linux compilation plus
the canonical Claudine `just test`, `just test-l2`, and `just lint` validation.
The checked-in matrix records:

- macOS `just test --no-fail-fast` exited `1` after two timeouts, and
  `just test-l2 --no-fail-fast` exited `100` after three failures; isolated
  evidence makes those failures plausibly unrelated, but the gates themselves
  are not green (`validation-matrix.md:312-336`);
- the sequence Level-3 test has no passing run;
- Linux CLI test targets were not compiled, and the Linux library L1 command
  exited `101` even though its two failures were diagnosed as container
  artifacts (`validation-matrix.md:338-366`);
- Windows production lib/bin cross-compilation passed, but Windows runtime was
  not run, the library test target does not compile, and the CLI test-target
  cross-build is blocked (`validation-matrix.md:368-433`).

Pre-existing and environment-specific failures should not be misreported as
Sequence Plus regressions, but they also cannot be converted into passing
acceptance evidence. The confirmed Windows product defects make the missing
native run material rather than administrative.

Required change: close Findings 1-4, then obtain clean canonical macOS gates,
complete the Linux CLI L1 build/run, and run native Windows compile/runtime
coverage including the sequence Ctrl+C scenario. Record exact commands and
summaries without relabeling nonzero exits as green.

## Requirement Verification Levels

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| State normalization, generated fields, neighbors, and state-name coercion | Level 1 unit and CLI tests | Appropriate. The Level-1 test target now compiles. |
| Dynamic expression/shell/file sources, list classification, offsets, operators, and typed source errors | Level 1 unit and CLI tests | Appropriate for parsing and resolution. |
| Direct and referenced formal documents share template/schema behavior | Level 1 unit and CLI parity tests | Appropriate and now aligned with the spec. |
| Static preflight, approved-byte parity, JIT rereads, runtime `set`, reserved precedence, and `outputs` chaining | Level 1 unit and CLI tests | Appropriate for state and process contracts. |
| Serial/parallel scheduling, caps, snapshots, deterministic merge, and failure policy | Level 1 concurrency and CLI tests | Appropriate except for the platform interrupt defects below. |
| Interactive missing-property collection | Level 1 pseudo-TTY | Appropriate for application prompt behavior. |
| Shell-task colored bars, pane wrapping, invisible-bar geometry, no-color labels, glyph fallback, and styled zero-step notice | Level 2 tmux capture | Appropriate; the five new sequence L2 cases passed in the recorded run. |
| Prompt assistant idle flush, semantic fallback, and provider status/reasoning keep task attribution in a real pane | Level 1 decorated-sink tests | **Gap:** terminal-visible prompt/provider behavior requires Level 2. |
| User Ctrl+C stops a sequence, fans out to every parallel child, suppresses later work, and exits `130` | Level 2 tmux injection; Level-3 test exists but has never passed | **Gap:** an OS-keyboard claim requires passing Level 3. |
| The same interruption contract on Windows | Source audit/cross-compile only | **Gap and functional defect:** the sequence flag has no Windows producer and the wait coordinator is not parallel-safe. |
| macOS, Windows, and Linux release behavior | macOS partial gates, Linux partial L1, Windows production cross-compile | **Gap:** the explicit three-platform and clean-gate acceptance criteria are unmet. |

## Prior Review Closure

- Review 2 Finding 1 — **closed**. The Arc ownership migration is complete;
  `cargo check -p claudine --tests --color=never` succeeds.
- Review 2 Finding 2 — **closed**. Direct and referenced formal documents now
  share `normalize_formal_plan`; scalar shorthand and typed template defaults
  are covered at both library and CLI levels.
- Review 2 Finding 3 — **closed at Level 1, incomplete at Level 2**. All three
  prompt-output bypasses route through the task decorator, but Finding 3 above
  captures the remaining real-terminal evidence gap.
- Review 2 Finding 4 — **partially closed**. Real tmux coverage now proves shell
  task bars, geometry, degradation, and zero-step styling; it does not exercise
  prompt/provider emissions.
- Review 2 Finding 5 — **not closed**. A correctly designed sequence Level-3
  test was added, but it has no passing run.
- Review 2 Finding 6 — **not closed**. Lint and production cross-compiles
  improved, but the required platform and release gates remain incomplete or
  nonzero.

## Verification Performed

- Read the complete specification, Review 2, the Claudine sequence architecture
  and user documentation, the implementation changes since Review 2, the new
  L1/L2/L3 tests, and the acceptance validation matrix.
- Used GitNexus to locate the sequence preflight, task/group, rendering, and
  interruption flows, then inspected the concrete implementations and callers.
- Ran `cargo check -p claudine --tests --color=never`; it passed.
- Ran the two focused library regressions for typed template defaults and scalar
  shorthand through nextest; both passed.
- Attempts to build the focused `claudine-cli` parity tests were stopped after
  the non-interactive command ceiling while the dev-dependency chain compiled
  `libduckdb-sys`; no CLI result is claimed from those interrupted attempts.
- Did not rerun the known-red Level-3 test. Its checked-in record already
  contains four real failures, and repeating a focus-dependent failure would
  not create production evidence.
- Attempted to validate both review frontmatters with `md schema validate`.
  Darkmatter rejected the repository's baseline `schemas/feature-review.yaml`
  as a legacy tagged schema with unsupported root keys, so no schema-pass claim
  is made; the requested properties were verified directly and `git diff
  --check` passes.

## Production Readiness Closure

Production readiness requires all five findings to close. In particular, a
follow-up review needs a concurrency-safe Windows interrupt coordinator, owned
Job handles, real-terminal prompt/provider attribution, a passing
sequence-specific Level-3 keyboard run, and clean native/cross-platform release
evidence.

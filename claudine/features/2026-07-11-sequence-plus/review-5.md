---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T21:10:01-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-5.md
previous: 2026-07-11-sequence-plus/review-4.md
---

# Review 5: Sequence Plus

## Verdict

**Not ready for production.** The state model, dynamic sources, just-in-time
composition, task/group execution, deterministic output merging, prompt stream
framing, and most of review 4's source-level findings are implemented. The
remaining blockers are concentrated in the production shell runner and in the
verification required for keyboard-driven interruption.

## Findings

### 1. High — shell timeout and interruption do not own the process tree

`SystemTaskShell::run` spawns the platform shell, pipes stdout, and kills only
that immediate child on timeout or interruption. It does not place the command
in a Unix process group or a Windows Job Object. It then performs an unbounded
`reader.join()` after the child exits.

A command that starts a descendant which inherits stdout can therefore outlive
the shell and keep the pipe open. Claudine may report that the parent was killed
and then block indefinitely waiting for the descendant to close stdout. The
same design lets descendants survive Ctrl+C and omits the spec's per-task
runaway-output guard because stdout is accumulated in an unbounded `Vec<u8>`.
The existing real-process timeout test covers a direct `sleep`, while the other
timeout cases use a fake runner; neither exercises a background descendant,
pipeline, or inherited pipe.

Required change:

- Give each production shell command the same tree-scoped ownership used by
  provider processes: a process group on Unix and a Job Object on Windows.
- Terminate and reap the entire tree on timeout and sequence interruption.
- Bound both captured output and the reader's shutdown wait so a descendant
  cannot defeat the deadline by retaining a pipe handle.
- Add Level-1 real-process regressions for a background descendant holding
  stdout, a pipeline, timeout, interrupt, and output-volume overflow. Run the
  Windows cases on a native Windows host; cross-compilation is not execution.

### 2. High — parallel shell output is neither live nor fully synchronized

The spec requires concurrent task output to be line-interleaved in arrival
order, task-attributed in color and no-color modes, and written through one
synchronized sink. The production shell runner instead inherits stderr
directly and buffers stdout with `read_to_end`. `run_shell` calls `emit_live`
only after the whole command returns.

Consequences:

- stderr bypasses `TaskStreamSink`, so it has no task bar or text label and can
  tear or interleave with other terminal writers;
- stdout from a long-running command is silent until completion, so concurrent
  shell output is grouped by completion rather than line arrival;
- the Level-2 shell fixture verifies one stdout line rendered after completion,
  but does not verify delayed alternating output, attributed stderr, or the
  no-color path.

Required change:

- Stream stdout and stderr incrementally through the synchronized live-output
  seam, preserving stdout as task output and treating stderr as status only.
- Preserve line arrival order at the sink without allowing partial writes to
  tear.
- Add Level-1 channel/order tests and a Level-2 real-terminal test with two
  simultaneous shell tasks that alternate delayed stdout and stderr. Capture
  both color and no-color rendering and assert textual attribution, SGR
  framing, ordering, and absence of torn lines.

### 3. High — Ctrl+C behavior lacks the required current Level-3 evidence

The requirement is phrased as user keyboard behavior: pressing Ctrl+C while a
parallel group is active must interrupt every task, prevent the next sequence
step, and exit 130. Under the review's rigor rules, only OS keyboard injection
exercises the terminal's input encoder and can verify that contract.

The recorded macOS Level-3 pass predates the current review tree, and the
current non-interactive gate correctly refused to take desktop focus. The
Windows Level-2 fixture uses `GenerateConsoleCtrlEvent`, which exercises the
application's signal path but bypasses the terminal encoder and a user's key
press. It has only been type-checked, never run. Linux likewise has no recorded
Level-3 sequence interruption run.

Required change:

- Run the current-tree macOS Level-3 sequence interruption test with its focus
  opt-in from an attended terminal.
- Add and run native Windows and Linux Level-3 coverage that injects an OS
  Ctrl+C key event into a supported terminal, while retaining the lower-level
  signal and process-tree tests for diagnosis.
- Record pane output, exit code, absence of the later-step marker, and
  descendant-process cleanup for each supported OS path.

### 4. Medium — the canonical Level-1 gate fails under `NO_COLOR=1`

The current `just test --no-fail-fast` run stopped after the library suite with
3,774 passed and one failure:
`a_parallel_group_gives_each_task_its_own_palette_entry`. The test constructs a
default terminal and requires three distinct ANSI-colored bars. This review
environment validly exports `NO_COLOR=1`, so the renderer removes color and all
three visible bar prefixes are identical. Rerunning that test with `NO_COLOR`
unset passes.

This is a test-isolation defect, not evidence that no-color rendering is
wrong. A canonical gate must not depend on ambient color policy.

Required change:

- Give the palette test an explicit color-capable terminal instead of
  inheriting process environment.
- Keep a separate explicit no-color assertion for textual task attribution.
- Rerun the complete Level-1 area gate in both color-capable and no-color
  environments.

### 5. Medium — the preflight progress status is emitted after preflight

`execute_sequence` calls `run_phase_1c_with_schema`, which performs sequence
validation and shell approval, before rendering “Starting pre-flight checks.”
The next status immediately says that shell commands are approved. Slow or
interactive approval therefore occurs without the promised early progress
feedback, and the wording describes work that has already completed.

Move the starting status before Phase 1c and update the Level-2 capture helpers
that currently use it as a post-preflight frame boundary.

### 6. Low — the validation record still contains stale branch-state claims

The validation matrix says the two Level-3 harness fixes are “still
uncommitted,” but the harness fix is present in branch history. The matrix also
mixes historical green runs with current-tree evidence without consistently
distinguishing them. Update it after the blockers above are addressed so each
claim names the tested revision, host, verification level, command, and result.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence found | Assessment |
|---|---|---|
| State shape, navigation fields, string coercion, outputs, and JIT composition | Level 1 | Appropriate for in-process data and evaluation semantics. |
| Dynamic file/list resolution and suffix operators | Level 1, including filesystem fixtures | Appropriate for resolution semantics; platform path cases are cross-checked but still need native Windows execution in the release gate. |
| Serial/parallel scheduling and deterministic output merge | Level 1 | Appropriate for scheduling and returned data. |
| Prompt task attribution, idle flush, styled frames, narrow panes, and no-color labels | Level 2 in tmux | Appropriate real-terminal rendering evidence is recorded. |
| Parallel shell stdout/stderr shown live and in arrival order | Partial Level 2: one buffered stdout line after completion | **Wrong/incomplete level coverage; finding 2.** |
| Colored task bars and no-color textual attribution | Level 2 recorded, but current Level-1 palette test is environment-dependent | Rendering level is appropriate; gate reliability is not. |
| User presses Ctrl+C and all parallel work stops | Historical macOS Level 3; Windows synthetic Level 2 only; no Linux Level 3 | **Insufficient current and cross-platform evidence; finding 3.** |
| Timeout/interrupt kills each shell task and descendants | Level 1 direct-child/fake-runner coverage | **Implementation and coverage gap; finding 1.** |
| Prompt stdout contributes to outputs while status stays on stderr | Level 1 plus Level 2 pane capture | Appropriate. |

## Review 4 Closure

| Review 4 finding | Review 5 status |
|---|---|
| Windows runtime unverified and Windows tests absent from gates | Tests now type-check in `just check-windows`; native execution remains open and keyboard behavior still lacks Level 3. |
| Prompt interruption inferred only from exit 130 | Closed: the explicit shared interruption signal is authoritative, with non-130 regression cases. |
| Required gates not rerun | Partially closed: lint and Windows cross-check pass, but the current Level-1 gate is red under `NO_COLOR=1`; current Level 3 did not run. |
| Validation matrix stale | Improved, but stale branch-state text remains. |
| Non-Unix compilation guard hid Windows source | Closed. |
| Dead `StreamParseError::Fatal` contract | Closed; the parser surface is now infallible. |
| Direct Windows provider feedback bypassed task rendering | Closed in the provider path. Finding 2 is a separate production shell-runner path. |
| Windows composed-sequence Ctrl+C was a no-op | Source path and cross-check are present; native runtime and Level-3 key evidence remain open. |

## Verification Performed

- Read the specification, review 4, validation matrix, gate record, task
  executor, system shell runner, stream renderer, interruption paths, and their
  Level-1/Level-2/Level-3 tests.
- Used the repository code index to inspect the `execute_sequence` and task
  execution flows before reviewing source details.
- `just lint`: passed. One unrelated leaked-handle guard retried and then
  passed.
- `just test --no-fail-fast`: failed in the Claudine library suite with 3,774
  passed, one failed, and seven skipped; the area recipe did not proceed past
  that failure.
- Targeted palette diagnostic with `NO_COLOR` unset: passed.
- Targeted non-130 prompt interruption regression: passed.
- `just check-windows`: passed for Windows library, binary, and test
  type-checking; nothing Windows was executed.
- Level 2 was not rerun in this review; its checked-in current-branch gate
  record and the test/source contract were inspected.
- Level 3 was not run because this session is non-interactive and the focus
  guard requires explicit attended authorization.

## Production Readiness

`ready: false` is required. Findings 1 and 2 are production implementation
defects in shell tasks, finding 3 is a mandatory verification-level gap for a
keyboard UX contract, and finding 4 leaves the current canonical Level-1 gate
red in a supported terminal mode.

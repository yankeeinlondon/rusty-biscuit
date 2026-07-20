---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T14:48:50-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-7.md
previous: 2026-07-11-sequence-plus/review-6.md
next: 2026-07-11-sequence-plus/review-8.md
---

# Review 7: Sequence Plus

## Verdict

**Not ready for production.** Review 6's ASCII fallback and runaway-detail
findings are closed, and the macOS Level-1 plus focused Level-2 evidence is
green. Three high-severity release blockers remain: the current Ctrl+C path
still has no Level-3 keyboard evidence, the rewritten process-tree contract has
no current Linux execution and no native Windows execution, and a wait failure
can return while task-stream readers are still able to write after the footer.

## Findings

### 1. High — the current Ctrl+C contract still has no Level-3 evidence

The specification promises that pressing Ctrl+C during a parallel group fans
out to every running child and descendant, suppresses later sequence steps, and
exits `130`. This is explicitly a keyboard UX contract. Level-1 flag/signal
tests and Level-2 byte injection begin downstream of the terminal encoder, so
they cannot prove what happens when the user presses the chord.

The three platform fixtures now assert descendant cleanup, but the checked-in
runbook and validation matrix state that none has run against the current
process-tree implementation. The only macOS green is from R-2, before both
process-tree rewrites and before the descendant assertion; the Linux and
Windows fixtures have never run on any host. Adding and type-checking a Level-3
fixture does not satisfy a Level-3 requirement.

Required change:

- Run the current revision's Level-3 fixture in attended native sessions on
  macOS, Linux, and Windows.
- Record the exact revision, OS, terminal, injected keyboard event, captured
  pane, exit `130`, absence of the later-step marker, and descendant cleanup.
- Keep the lower-tier signal tests for diagnosis, but do not use them as
  production-readiness evidence for the keyboard path.

### 2. High — the process-tree rewrite is unverified on current Linux and native Windows

`SystemTaskShell` now relies on platform-specific ownership primitives for
user-observable guarantees: Unix process groups, or a suspended Windows child
assigned to a kill-on-close Job Object before resume. Those paths determine
whether successful background work, timeouts, interrupts, runaway commands,
and error exits leave descendants behind or retain the task's output pipes.

Current real-process tests executed on macOS and passed. The matrix's Linux run
is R-2 and predates the rewritten ownership contract. Windows only
cross-type-checks: Job creation/configuration, suspended assignment, thread
discovery/resume, termination, and the Windows-specific tests have never
executed. That is not enough evidence for behavior required on all three
supported operating systems.

Required change:

- Run the process-tree regressions on a current native Linux host and a native
  Windows host, including success-path descendant cleanup, timeout, interrupt,
  runaway output, ownership-establishment failure, inherited-pipe closure, and
  Job assignment/resume.
- Record executed results separately from cross-compilation. A type-check may
  remain a portability gate but must not be labeled runtime verification.

### 3. High — wait errors bypass reader settlement and can render after the task footer

`SystemTaskShell::run` starts detached stdout/stderr reader threads, but a
`child.try_wait()?` error returns directly from the polling loop. That early
return skips the normal sequence at the end of `run`: explicit tree drop,
bounded reader settlement, handle drop, live-stream flush, and capture
collection. Automatic `ProcessTree::drop` kills the tree, but it does not wait
for the detached readers to drain bytes already in the pipes.

The group scheduler closes the task stream immediately after `member.run()`
returns. A reader that drains buffered data after the wait error can therefore
append a body frame after the failure footer, violating the synchronized
header/body/footer contract. The same shape exists if the post-termination
`child.wait()?` fails. The new `an_early_wait_error_still_reaps_the_whole_tree`
test checks only that a delayed marker file is absent; it supplies no live sink
and cannot detect post-return frames or a missing flush.

The error is also misclassified. `TaskShellError::Io` intentionally covers both
spawn and wait failures, but `TaskExecution::run_shell` maps every `Io` to
`SequenceTaskShellSpawn`, telling the user that a command which already ran and
may have emitted output "failed to run."

Required change:

- Route every post-spawn exit through one cleanup epilogue that terminates the
  tree, settles readers within the bounded grace, flushes the live stream, and
  only then returns the preserved wait error.
- Split spawn and wait failures, or otherwise preserve the operation stage in
  the typed diagnostic.
- Add a Level-1 real-process regression combining injected wait failure,
  inherited/buffered output, a live task stream, and a footer; assert that no
  frame arrives after `run` returns or after the footer.

### 4. Medium — the validation matrix still does not identify a reproducible current revision

The matrix calls R1 (`a7bfdd7a7` plus an uncommitted tree with 17 modified and
six untracked paths) the current tree. The reviewed branch is now
`237b86a41`; the three review-6 implementation commits and the documentation
commit are part of history. Although the recorded dirty snapshot appears to
correspond to those changes, it is not a revision another developer can check
out, and the statements that R1 is current and uncommitted are now false.

Refresh the evidence against an exact committed revision after the remaining
fixes. Each gate claim should identify that revision, host, verification level,
command, result, and any skips. Historical dirty-tree evidence can remain, but
must not be presented as the current release candidate.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence on the reviewed tree | Assessment |
|---|---|---|
| State shape/navigation, source grammar and formats, strict/lenient normalization, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Level 1 library plus CLI process tests | Appropriate; the 3,804-test library suite and 146 filtered CLI sequence tests passed in this review. |
| Live attributed shell stdout/stderr, channel separation, arrival order, and no-color labels | Level 2 tmux | Appropriate; both focused interleaved-shell captures passed. |
| ASCII header fallback under a non-UTF-8 locale | Level 2 tmux | Appropriate; the focused current-tree capture passed. Review 6 finding 1 is closed. |
| Other task-stream bars, wrapping, prompt frames, idle flush, and zero-step styling | Level 2 tmux recorded at R1 | Appropriate level; feature-owned cases are recorded green, but the evidence artifact needs a reproducible revision. |
| Ctrl+C key press fans out, suppresses later work, cleans descendants, and exits `130` | Historical pre-rewrite macOS Level 3; current fixtures compile only | **Wrong/stale evidence for the current behavior; finding 1.** |
| Unix process-tree cleanup on success, timeout, runaway, and ordinary interruption | Current macOS Level-1 real-process tests | Appropriate for the macOS/Unix execution path; Linux current-tree execution is absent. |
| Native Windows Job ownership, suspended assignment/resume, and cleanup | Cross-type-check only | **No behavioral verification; finding 2.** |
| No task body frames after an error footer | Level 1 for normal completion only | **The wait-error branch is unverified and bypasses cleanup; finding 3.** |

## Review 6 Closure

| Review 6 finding | Review 7 status |
|---|---|
| ASCII task-header fallback failed in a real terminal | Closed: locale-derived Unicode support is preserved through forced-color/plain terminal construction, and the current focused Level-2 test passes. |
| Shell process-tree ownership was fail-open and OS-divergent | Partially closed: fail-closed ownership and uniform success cleanup are implemented and macOS tests pass; current Linux/native Windows evidence is absent, and the wait-error stream cleanup is incomplete. |
| Ctrl+C lacked current Level-3 evidence | Open: fixtures and runbook improved, but no current attended run exists. |
| Validation record was stale | Partially closed then stale again: R1 records the dirty implementation snapshot, not an exact current revision. |
| Runaway diagnostics discarded trip counters | Closed: the diagnostic now includes limit kind, observed count, and configured limit. |

## Verification Performed

- Read the specification, review 6, sequence architecture/topic docs,
  validation matrix, Level-3 runbook, shell runner, task/group framing, locale
  capability selection, and the relevant Level-1/Level-2/Level-3 tests.
- Used the repository code index for the shell runner, locale capability path,
  and Level-3 fixture, then confirmed the findings against source.
- `just test --no-fail-fast`: `claudine-catalog-types` passed 21/21 and the
  `claudine` library passed 3,804/3,804 with seven skipped. The area command was
  stopped during the later `claudine-contract` build after crossing this
  non-interactive session's subprocess limit, so the full five-package gate is
  incomplete rather than green.
- `just test-cli sequence`: 146/146 passed with 2,193 skipped.
- `just test-l2 non_utf8`: 1/1 passed.
- `just test-l2 interleaved_shell`: 2/2 passed.
- Level 3 was not run. This non-interactive session is not authorized to take
  desktop focus or inject OS keyboard input.
- No native Linux or Windows runtime was available. The checked-in Linux
  evidence is stale and Windows evidence remains type-check-only.
- `just lint` was not rerun in this review.

## Production Readiness

`ready: false` is required. The Level-3 mismatch alone prevents a production
verdict under the mandated testing rubric. Native cross-platform process-tree
behavior also lacks current execution, and the wait-error path has a current
stream-ordering defect. Production readiness requires findings 1–3 to close
and the acceptance record to point at the exact reviewed revision.

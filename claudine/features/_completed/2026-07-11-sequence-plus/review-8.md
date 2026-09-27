---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T16:22:11-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-8.md
previous: 2026-07-11-sequence-plus/review-7.md
next: 2026-07-11-sequence-plus/review-9.md
---

# Review 8: Sequence Plus

## Verdict

**Not ready for production.** Review 7's wait-error cleanup defect and current
Linux process-tree verification gap are closed. Two high-severity release
blockers remain: the current Ctrl+C path still has no Level-3 keyboard evidence
on any supported platform, and the native Windows Job Object and console
interrupt paths have never executed. The acceptance record also still labels
an obsolete dirty snapshot as the current tree instead of the committed
revision under review.

## Findings

### 1. High — the current Ctrl+C contract still has no Level-3 evidence

The specification promises that pressing Ctrl+C during a parallel group fans
out to every running child and descendant, suppresses later sequence steps,
marks the group interrupted, and exits `130`. This is an OS-keyboard behavior.
Level-1 flag/signal tests and Level-2 terminal-CLI byte injection start after
the terminal encoder and cannot verify what the terminal emits for the actual
key press.

The macOS fixture's only recorded green run is R-2, before the process-tree
rewrites, descendant-cleanup assertion, and wait-error epilogue. The current
macOS, Linux/X11, and Windows fixtures compile, but none has run against the
reviewed behavior; the Linux and Windows fixtures have never run on any
revision. This is the exact wrong-level mismatch the review rubric identifies
as a production blocker.

Required change:

- Run the current committed revision's Level-3 fixture in attended native
  sessions on macOS, Linux, and Windows.
- Record the exact revision, OS, terminal, injected keyboard event, captured
  pane, exit `130`, absence of the later-step marker, and descendant cleanup.
- Retain the lower-level signal and byte-injection tests for diagnosis, but do
  not present them as verification of OS keyboard encoding.

### 2. High — native Windows process ownership and interruption remain unverified

The Windows implementation depends on a platform-specific sequence with real
failure modes: create the child suspended, create and configure a kill-on-close
Job Object, assign the child, discover and resume its primary thread, deliver
the console interrupt ladder, and terminate the Job on cleanup. Cross-target
type-checking proves that these APIs and tests compile; it proves none of their
runtime behavior.

The new Linux gate closes the Unix half of review 7 finding 2: the current code
ran inside a real Linux kernel and passed the sequence task and process-tree
regressions. macOS real-process coverage is also current and green. Windows is
still type-check-only: its Job ownership, assignment/resume, inherited-pipe
closure, timeout, interrupt, runaway-output, and success cleanup paths have
never executed on a Windows host. The specification and repository contract
require behavior on Windows, not merely compilability.

Required change:

- Run the Windows-host runtime gate and the Windows process-tree regressions on
  an attached native console at the exact release-candidate revision.
- Include success, ordinary failure, timeout, Ctrl+Break interruption, runaway
  output, ownership-establishment failure, inherited-pipe closure, Job
  assignment/resume, and descendant cleanup.
- Record runtime execution separately from `just check-windows` type-checking.

### 3. Medium — the acceptance record is stale and not reproducible

`validation-matrix.md` still declares R2 to be the current tree and defines it
as `237b86a41` plus an uncommitted review-7 fix. That fix is now committed as
`b814bf031`, and the reviewed HEAD is `baba83844`. The matrix therefore still
describes the release candidate as a dirty snapshot that cannot be checked out,
while its own promise to refresh after the finding-3 commit has become due.

Refresh the matrix with a new anchor for the exact committed candidate. For
each gate, record the revision, host, level, command, result, and skips. The
current macOS Level-1/lint results, Linux gate record, focused Level-2 results,
Windows runtime result, and attended Level-3 results should all point to that
same candidate or be identified explicitly as historical.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence | Assessment |
|---|---|---|
| State shape/navigation, source grammar and formats, strict/lenient normalization, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Current Level 1 library and CLI process tests | Appropriate; the full macOS area suite passed in this review. |
| Shell wait-error classification, descendant cleanup, reader settlement, flush, and no body frames after the footer | Current Level 1 real-process tests with an injected wait failure and live sink | Appropriate; review 7 finding 3 is closed. |
| Live attributed shell/prompt streams, channel separation, ordering, task-bar colors, narrow-width wrapping, no-color labels, locale fallback, and zero-step styling | Level 2 tmux pane capture | Appropriate level. Eight current focused cases passed; the intentionally approximately 80-second idle-flush case was aborted at this session's 60-second subprocess ceiling without an assertion failure, and its checked-in Level-2 test is unchanged from the recorded green revision. |
| Ctrl+C key press fans out, suppresses later work, cleans descendants, and exits `130` | Historical pre-rewrite macOS Level 3; current fixtures compile only | **Wrong/stale evidence for the reviewed behavior; finding 1.** |
| Unix process-tree cleanup on success, failure, timeout, runaway, and interruption | Current macOS Level 1 plus recorded current-code Linux real-kernel execution | Appropriate for both Unix implementations. |
| Native Windows Job ownership, suspended assignment/resume, console interruption, and cleanup | Cross-target type-check only | **No behavioral verification; finding 2.** |

No paste, IME, mouse, or other keyboard interaction is specified by this
feature, so no additional Level-3 input requirements were identified.

## Review 7 Closure

| Review 7 finding | Review 8 status |
|---|---|
| Current Ctrl+C lacked Level-3 evidence | Open: the fixtures and runbook exist, but no current attended run exists on any platform. |
| Process-tree rewrite lacked current Linux and native Windows execution | Partially closed: the current Linux real-kernel gate is green; native Windows execution is still absent. |
| Wait errors bypassed reader settlement and were mislabeled as spawn errors | Closed: post-spawn exits share the cleanup epilogue, wait failures have a distinct typed error, and live-stream regressions prove settlement and footer ordering. |
| Validation matrix did not identify a reproducible current revision | Open: the implementation fix is now committed, but the matrix still names the obsolete dirty R2 snapshot as current. |

## Verification Performed

- Read the specification, review 7, sequence architecture/topic docs,
  validation matrix, Linux gate record, Level-3 runbook, shell runner, task
  framing, and the relevant Level-1/Level-2/Level-3 tests.
- Used the repository code index to trace sequence execution and inspect
  `SystemTaskShell::run`, then confirmed the review-7 fix and its callers in
  source. The index was current at `baba83844`.
- `just test` from `claudine/` passed: catalog types 21/21, library 3,806/3,806
  with seven skipped, contract 47/47 with five skipped, CLI 2,165/2,165 with
  174 skipped, and generator 152/152 with four skipped. Two CLI cases retried
  after initial leaked-handle detections and then passed.
- `just lint` from `claudine/` completed successfully for every area crate.
- The focused Level-2 nextest expression started nine tmux pane-capture tests.
  Eight passed. The idle-flush test documents an expected runtime of about 80
  seconds; it was interrupted at 68 seconds to honor the non-interactive
  session ceiling, so this was an incomplete run rather than an assertion
  failure.
- The checked-in current-code Linux gate reports all 96 sequence task tests
  green, including the 34 shell/process-tree cases. No native Windows runtime
  evidence exists.
- Level 3 was not run. This non-interactive session cannot take desktop focus
  and inject OS keyboard events safely.

## Production Readiness

`ready: false` is required. The Ctrl+C requirement is verified at the wrong
level for the current implementation, which independently bars a production
verdict under the mandated rubric. Native Windows behavior is also entirely
unexecuted. Production readiness requires findings 1 and 2 to close and the
acceptance matrix to identify the exact reviewed release candidate.

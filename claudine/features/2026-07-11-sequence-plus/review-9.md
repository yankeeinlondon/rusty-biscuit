---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T19:03:13-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: false
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-9.md
previous: 2026-07-11-sequence-plus/review-8.md
---

# Review 9: Sequence Plus

## Verdict

**Not ready for production.** The new Linux XTEST run is valid Level-3
evidence for the Linux keyboard-to-terminal path and closes that part of review
8. It does not supply current macOS evidence, any Windows keyboard evidence, or
any native Windows execution of the Job Object and console-interrupt paths.
The claimed reproducible revision also depends on uncommitted test-harness
modules, so a clean checkout cannot compile the Linux and Windows Level-3
fixtures that the acceptance record says were checked.

## Findings

### 1. High — the Ctrl+C keyboard contract still lacks current Level-3 evidence on macOS and Windows

The specification promises that a user pressing Ctrl+C while a parallel group
is active interrupts every task and descendant, suppresses later sequence
steps, and exits `130`. This is explicitly an OS-keyboard requirement. Level-1
signal/flag tests and Level-2 terminal-CLI injection begin downstream of the
terminal input encoder and cannot discharge it.

The new Linux gate is correctly classified as Level 3: XTEST delivered a real
Ctrl+C key event through Xvfb into WezTerm, whose encoder produced the terminal
input seen by Claudine. It passed the current Linux fixture and verified exit
`130`, later-step suppression, and descendant cleanup. However:

- the macOS fixture's only green run predates the process-tree ownership
  rewrites, descendant-cleanup assertion, and wait-error cleanup path;
- the Windows Level-3 fixture has never executed on any host; and
- review 8's additional attended-native-desktop run requirement remains
  unfulfilled on every supported OS, including Linux.

Required change:

- Run the current, reproducible release candidate's Level-3 fixture in an
  attended native session on macOS and Windows, and retain the attended Linux
  run required by review 8.
- Record the exact revision and clean/dirty state, OS, terminal, injected
  event, captured pane, exit `130`, absence of the later-step marker, and
  descendant cleanup.
- Keep the lower-level signal and terminal-CLI tests as diagnostic coverage;
  do not promote them to keyboard-path evidence.

### 2. High — native Windows process ownership and interruption still have no runtime evidence

The Windows implementation creates the task suspended, creates and configures
a kill-on-close Job Object, assigns the task, discovers and resumes its primary
thread, delivers the console interrupt ladder, and terminates the Job during
cleanup. `just check-windows` type-checks these paths; it does not link or
execute them.

The new Windows gate record explicitly says that nothing Windows has ever
executed. Consequently there is still no behavioral evidence for Job
assignment/resume, inherited-pipe closure, success cleanup, ordinary failure,
timeout, Ctrl+Break interruption, runaway-output cleanup, ownership failure,
or descendant cleanup. This violates the repository's macOS/Windows/Linux
support contract independently of finding 1.

Required change:

- Run the Windows Level-1 process-tree tests, the attached-console interrupt
  gate, and the Windows Level-2 sequence Ctrl+C test on a native Windows host
  at the exact release-candidate revision.
- Record success, failure, timeout, runaway output, ownership-establishment
  failure, inherited-pipe closure, suspended assignment/resume, and descendant
  cleanup separately from cross-target type-checking.

### 3. High — the checked-in candidate is not the reproducible cross-platform test target the matrix claims

`validation-matrix.md` calls `baba83844` a reproducible R3 anchor while also
stating that the Linux and Windows Level-3 fixtures compile only when the
uncommitted `biscuit-test-harness/src/xdotool.rs`,
`biscuit-test-harness/src/win_input.rs`, and their `lib.rs` declarations are
present. The current checked-in fixtures import those modules, but `git show
HEAD:biscuit-test-harness/src/lib.rs` contains neither export. A clean checkout
therefore cannot reproduce the recorded Linux gate or the Windows `--tests`
type-check; on the corresponding target it has unresolved imports.

The matrix is also anchored to `baba83844` even though the current HEAD is
`23c05d16e`, the documentation commit that added the R3 records. This repeats
review 8's reproducibility defect rather than closing it.

Required change:

- Include the two injector modules and their public module declarations in the
  release-candidate change set.
- Refresh the matrix to the exact candidate revision after that source is
  present, then rerun the Linux Level-3 gate and Windows cross-target check at
  that same revision.
- Do not label an anchor reproducible when load-bearing source exists only in a
  dirty working tree.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence | Assessment |
|---|---|---|
| State shape/navigation, source grammar and formats, strict/lenient normalization, `FileReference` resolution, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Current Level 1 library tests; the focused `composition::sequence` run in this review passed 245/245 | Appropriate level. |
| CLI fail-fast, dry-run, error presentation, task execution, output capture, and channel contracts | Level 1 CLI process/PTY tests recorded green at the same production-source revision | Appropriate level; no terminal encoder or renderer dependency. |
| Attributed parallel streams, real glyph widths, colors, wrapping, no-color labels, locale fallback, and zero-step styling | Level 2 tmux pane capture | Appropriate level for real-terminal rendering. |
| User presses Ctrl+C and the sequence fans out, suppresses later work, cleans descendants, and exits `130` | Current Linux Level 3 in headless X11; stale macOS Level 3; no Windows Level 3 | **Insufficient across supported platforms; finding 1.** |
| Native Windows Job ownership, suspended assignment/resume, console interruption, and cleanup | Cross-target type-check only | **No behavioral verification; finding 2.** |

No paste, IME, mouse, or other keyboard interaction is specified by this
feature, so no additional Level-3 input requirement was identified.

## Review 8 Closure

| Review 8 finding | Review 9 status |
|---|---|
| Current Ctrl+C lacked Level-3 evidence | Partially closed: current Linux XTEST evidence is valid Level 3; current macOS and Windows evidence remains absent, and the attended-run condition remains open. |
| Native Windows process ownership and interruption were unverified | Open: the new gate is explicitly type-check-only. |
| Acceptance record was stale and not reproducible | Open: the new anchor still depends on uncommitted, load-bearing harness source and is already behind current HEAD. |

## Verification Performed

- Read the specification, review 8, the sequence architecture/topic docs, the
  validation matrix, both new gate records, the Level-3 runbook, the three
  OS-specific Level-3 fixtures, and the new harness injector modules.
- Used the repository code index to locate the sequence interruption flows and
  test fixtures. The index was current at `23c05d16e`.
- `just test-library composition::sequence` passed 245/245 current Level-1
  tests on macOS.
- A full `just test` attempt was capped at 55 seconds during a cold build. The
  catalog-types package passed 21/21 before the cap; the area result is
  inconclusive, not a test failure.
- A current `just check-windows` attempt was likewise capped during a cold
  cross-target dependency build. The checked-in R3 record reports the same
  command green only with the uncommitted injector modules present; it remains
  type-check evidence, not runtime evidence.
- Level 2 was not rerun because the follow-up contains no production or
  rendering change. Level 3 was not run because this non-interactive session
  may not take desktop focus or inject host keyboard events.

## Production Readiness

`ready: false` is required. The Ctrl+C user interaction lacks current
appropriate-level evidence on macOS and Windows, native Windows process control
has never executed, and the recorded candidate is not reproducible from its
revision. All three findings must close before this feature can be considered
production ready.

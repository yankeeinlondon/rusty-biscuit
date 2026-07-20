---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T19:25:14-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: false
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-10.md
previous: 2026-07-11-sequence-plus/review-9.md
---

# Review 10: Sequence Plus

## Verdict

**Not ready for production.** The functional Sequence Plus implementation
continues to pass its focused Level-1 suite, and the newly committed Linux and
Windows keyboard-injector modules make a clean checkout compilable. The
review-9 release blockers were not all implemented, however: macOS and Windows
still lack current Level-3 evidence for the user Ctrl+C contract, no native
Windows process-control path has executed, and the validation record still
describes an old dirty revision as the current tree without rerunning the Linux
Level-3 gate at the checked-in candidate.

## Findings

### 1. High — the Ctrl+C keyboard contract still lacks current Level-3 evidence on macOS and Windows

The specification promises that a user pressing Ctrl+C while a parallel group
is active interrupts every task and descendant, prevents later sequence steps,
and exits `130`. This is an OS-keyboard requirement, so manufactured signals,
cooperative-flag tests, and terminal-CLI byte injection are diagnostic coverage,
not substitutes for Level 3.

The Linux Xvfb/WezTerm/XTEST run remains valid Level-3 evidence for the Linux
X11 encoder path. It does not exercise the macOS Quartz/cliclick path or the
Windows foreground-window/SendKeys/console path. The current matrix still says
the macOS fixture's last green run predates the process-tree ownership rewrite
and descendant-cleanup assertion, while the Windows fixture has never run
(`validation-matrix.md:187-189`). The Level-3 runbook likewise records the
macOS result as stale and Windows as never executed
(`l3-ctrl-c-runbook.md:3-15`).

Required change:

- Run the current release candidate's macOS and Windows Level-3 sequence
  fixtures in attended native sessions, with missing backends configured to
  fail rather than skip.
- Retain the pane, exit-`130`, later-step suppression, and descendant-cleanup
  observations required by the runbook.
- Keep the current Level-1 and Level-2 interrupt tests as lower-level
  diagnostics; do not promote them to keyboard-path evidence.

### 2. High — native Windows process ownership and interruption still have no runtime verification

The Windows sequence path uses distinct platform code: suspended child
creation, Job Object assignment and kill-on-close ownership, primary-thread
resume, console-control delivery, bounded pipe-reader settlement, and Job
termination. The current `just check-windows` run proves that these paths and
their tests type-check; it neither links nor executes them.

The checked-in Windows gate record explicitly says that nothing Windows has
ever executed and lists the still-owed native cases
(`gate-run-2026-07-19-windows.md:114-138`). That leaves success cleanup,
ordinary failure, timeout, runaway output, ownership-establishment failure,
inherited-pipe closure, Job assignment/resume, Ctrl+Break, and descendant
cleanup without behavioral evidence on a supported operating system.

Required change:

- On a native Windows host, run the Windows Level-1 process-tree/Job tests, the
  attached-console Ctrl+Break gate, and the Windows Level-2 sequence fan-out
  fixture at the exact release-candidate revision.
- Record each exit path separately, including descendant cleanup and
  inherited-pipe closure. The Windows Level-3 keyboard fixture required by
  finding 1 remains a separate gate.

### 3. High — the release evidence is still not reproducible from the revision it calls current

Review 9 required the injector modules to be checked in, the validation matrix
to be re-anchored, and the Linux Level-3 plus Windows cross-target gates to be
rerun at that one candidate. Only the first part landed. The injectors are now
committed in `b965938e2`, and this review's Windows cross-target check passes at
current HEAD `96679f516`, but the matrix still calls `baba83844` the current
tree (`validation-matrix.md:18,25`), says the injectors are untracked/staged
(`validation-matrix.md:41-57`), and cites the old dirty-tree Linux run.

The current source therefore has no checked-in acceptance record tying the
Linux Level-3 result to the reproducible candidate. This is not merely stale
wording: the matrix itself says the old Level-3 rows must not be cited again
until those reruns occur (`validation-matrix.md:54-57`).

Required change:

- Re-anchor the matrix to the exact release-candidate commit containing the
  injector modules.
- Rerun the Linux Level-3 fixture at that revision and record the result.
- Record the current Windows cross-target check at the same revision, while
  continuing to label it compile-only.
- Remove the obsolete dirty-tree/current-tree narrative instead of layering a
  seventh historical anchor on top of it.

### 4. Low — the test-harness documentation contradicts the newly exported Level-3 helpers

`biscuit-test-harness/src/lib.rs:15-16` says Level 3 is not covered by the
harness, even though the crate now exports `cliclick`, `xdotool`, and
`win_input`. The README also still says Linux `xdotool` support is "not
implemented here" (`biscuit-test-harness/README.md:366-369`). This is direct
comment/documentation drift introduced by the review-9 repair.

Required change:

- Update the crate-level testing vocabulary and README to document the three
  platform injector modules and their availability/skip contracts.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence | Assessment |
|---|---|---|
| State/navigation shape, source grammar and formats, strict/lenient normalization, `FileReference` resolution, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Current Level 1; focused `composition::sequence` run passed 245/245 | Appropriate level. |
| CLI fail-fast, dry-run, typed error presentation, task execution, output capture, and stdout/stderr contracts | Level 1 CLI process/PTY tests in the acceptance record | Appropriate level; no terminal input encoder or real renderer is required. |
| Parallel stream glyph widths, colors, wrapping, no-color labels, channel separation, and zero-step styling | Level 2 tmux capture; feature-owned L2 cases passed in the latest recorded L2 run | Appropriate level, though the area-wide recorded run is not current and remained red for three unrelated context tests. |
| Linux user Ctrl+C reaches a parallel sequence through the terminal encoder | Level 3 XTEST through Xvfb + WezTerm at the old R3 dirty snapshot | Correct level for Linux/X11, but not rerun at the reproducible current candidate; finding 3. |
| macOS user Ctrl+C interrupts all children/descendants, suppresses later work, and exits `130` | Stale Level 3 predating the current process-tree contract; current lower-level tests only | **Insufficient current evidence; finding 1.** |
| Windows user Ctrl+C interrupts all children/descendants, suppresses later work, and exits `130` | Fixture type-check only; no Level-3 execution | **Wrong verification strength; finding 1.** |
| Native Windows Job ownership, suspended assignment/resume, console interruption, pipe closure, and cleanup | Cross-target type-check only | **No behavioral verification; finding 2.** |

No paste, IME, mouse, or additional keybinding behavior is specified by this
feature, so no other Level-3 interaction requirement was identified.

## Review 9 Closure

| Review 9 finding | Review 10 status |
|---|---|
| Current Ctrl+C lacked Level-3 evidence on macOS and Windows | Open. No new native execution record exists. |
| Native Windows process ownership and interruption were unverified | Open. The current cross-target check passes, but no Windows code ran. |
| The candidate depended on uncommitted injector modules and was not reproducible | Partially closed. The modules are committed and compile, but the matrix was not re-anchored and the Linux Level-3 gate was not rerun at the checked-in candidate. |

## Verification Performed

- Read the full specification, review 9, the current validation matrix, both R3
  gate records, the Level-3 runbook, the three OS-specific sequence Ctrl+C
  fixtures, and the newly committed `xdotool`/Windows input helpers.
- Used the current code index to locate the sequence entry point, preflight
  graph, task execution modules, and related tests. The worktree index was
  current at `96679f516`.
- `just test-library composition::sequence` passed 245/245 on macOS.
- `just check-windows` passed at `96679f516` in 27.44 seconds. This is
  cross-target type-check evidence only.
- The macOS Level-3 test binaries compiled with
  `cargo nextest run --color=never -p claudine-cli --no-run -E 'test(/level3_/)'`.
- Level 2 was not rerun because the review-9 implementation changed only the
  Level-3 harness and review documentation, not rendering or sequence
  production code. Level 3 was not run because this non-interactive session may
  not take desktop focus or inject host keyboard events.

## Production Readiness

`ready: false` is required. The feature cannot be called production-ready while
its macOS and Windows user-keyboard behavior lacks current Level-3 evidence,
the native Windows process-control implementation has never run, and the
acceptance record does not describe or reproduce the checked-in candidate.

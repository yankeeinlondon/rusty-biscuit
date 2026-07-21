---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T20:49:18-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-11.md
previous: 2026-07-11-sequence-plus/review-10.md
---

# Review 11: Sequence Plus

## Verdict

**Not ready for production.** The current macOS functional and real-terminal
checks are green, and the review-10 documentation-drift finding is addressed in
the working tree. The three high-severity release blockers remain open: the
current macOS and Windows user-keyboard paths lack Level-3 execution, native
Windows process ownership and interruption have never executed, and the
acceptance matrix still describes an obsolete dirty R3 snapshot instead of the
checked-in candidate.

## Findings

### 1. High — current macOS and Windows Ctrl+C behavior is still unverified at Level 3

The specification promises that pressing Ctrl+C while a parallel group is
active interrupts every task and descendant, suppresses the following step, and
exits `130`. That is explicitly a user-keyboard contract, so only OS keyboard
injection through the terminal's input encoder is sufficient verification.

The checked-in runbook still says the macOS result predates the process-tree
rewrite and descendant-cleanup assertion, while the Windows fixture has never
run (`l3-ctrl-c-runbook.md:3-15`). The acceptance matrix makes the same status
explicit (`validation-matrix.md:1123-1125`). Current Level-1 signal tests and
Level-2 terminal-CLI injection remain valuable diagnostics, but they begin
downstream of the keyboard encoder and cannot close this requirement.

Required change:

- Run the macOS and Windows Level-3 sequence fixtures on the exact release
  candidate in attended native sessions, with required backends configured to
  fail rather than skip.
- Record the pane evidence, exit `130`, later-step suppression, and descendant
  cleanup defined by `l3-ctrl-c-runbook.md`.

### 2. High — native Windows process ownership and interruption still have no behavioral execution

DECISION: everything that can be reliable built for the Windows platform outside of actually running it on Windows is required but we are currently using a macOS host so any limitations imposed by this host's OS will not be considered a blocker for achieving "production readiness".

The Windows implementation has a materially distinct runtime path: suspended
child creation, Job Object assignment, thread resume, console-control delivery,
bounded reader settlement, Job termination, and kill-on-close ownership. The
updated Windows gate proves that this source and its test targets type-check,
but states that no Windows runtime behavior has executed
(`gate-run-2026-07-19-windows.md:146-180`).

Consequently success cleanup, ordinary failure, timeout, runaway output,
ownership-establishment failure, inherited-pipe closure, Job assignment/resume,
Ctrl+Break, and descendant cleanup remain unverified on one of the feature's
supported operating systems. Cross-target `cargo check --tests` is not a
substitute because it neither links nor runs the platform code.

Required change:

- On a native Windows host, run the Windows Level-1 process-tree and Job Object
  tests, the attached-console Ctrl+Break gate, and the Windows Level-2 sequence
  fan-out fixture at the release-candidate revision.
- Record each exit path separately, including descendant cleanup and inherited
  pipe closure. The Windows Level-3 keyboard fixture remains the separate gate
  in finding 1.

### 3. High — the acceptance record still cannot reproduce its current Level-3 claim

DECISION: this will be considered non-blocking for now!

The injector modules are now committed and the Windows compile-only gate was
re-anchored to `96679f516`, closing part of review 10. The canonical validation
matrix was not updated: it still calls `baba83844` plus load-bearing untracked
injectors the current R3 tree and explicitly says its Linux Level-3 evidence
must not be cited until the gate is rerun after those injectors are committed
(`validation-matrix.md:18-57`). Its current-first gate table repeats the old
revision and result (`validation-matrix.md:1188-1199`).

The Linux Xvfb/WezTerm/XTEST fixture is the correct verification level and its
historical run is useful. It is not a checked-in acceptance record for the
candidate the matrix now needs to certify. The Windows gate file alone cannot
repair the matrix's contradictory current-tree narrative.

Required change:

- Re-anchor `validation-matrix.md` to one exact release-candidate revision.
- Rerun the Linux Level-3 fixture and Windows cross-target check at that
  revision; keep the Windows result labeled compile-only.
- Replace the obsolete dirty-R3/current-tree sections rather than appending
  another competing anchor.

## Requirement Verification Levels

| User-facing requirement | Strongest current evidence | Assessment |
|---|---|---|
| State/navigation shape, source grammar and formats, strict/lenient normalization, `FileReference` resolution, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Level 1; current `composition::sequence` run passed 245/245 | Appropriate level. |
| CLI fail-fast, dry-run, typed diagnostics, task/group execution, output capture, and stdout/stderr contracts | Level 1; current CLI `sequence` filter passed 146/146 | Appropriate level. |
| Parallel glyphs, colors, wrapping, no-color labels, channel separation, idle flush, and zero-step styling | Level 2 tmux capture; current focused run passed 9/9 | Appropriate level. The idle-flush fixture's ~78s duration is intentional and documented. |
| Linux user Ctrl+C reaches a parallel sequence through the terminal encoder | Level 3 XTEST through Xvfb + WezTerm at obsolete R3 | Correct level, but not anchored to the checked-in candidate; finding 3. |
| macOS user Ctrl+C interrupts all children/descendants, suppresses later work, and exits `130` | Stale Level 3 predating the current process-tree contract | Insufficient current evidence; finding 1. |
| Windows user Ctrl+C interrupts all children/descendants, suppresses later work, and exits `130` | Fixture type-check only | Wrong verification strength; finding 1. |
| Native Windows Job ownership, suspended assignment/resume, console interruption, pipe closure, and cleanup | Cross-target type-check only | No behavioral verification; finding 2. |

No paste, IME, mouse, or other keybinding behavior is specified by this
feature, so no additional Level-3 requirement was identified.

## Review 10 Closure

| Review 10 finding | Review 11 status |
|---|---|
| Current macOS and Windows Ctrl+C lacked Level-3 evidence | Open. No new native execution record exists. |
| Native Windows process ownership and interruption were unverified | Open. The updated gate remains explicitly compile-only. |
| The release evidence was not reproducible from the revision it called current | Partially addressed. Injectors are committed and the Windows gate is re-anchored, but the validation matrix and Linux Level-3 result remain at obsolete R3. |
| Test-harness documentation contradicted exported Level-3 helpers | Closed in the current working tree. The crate docs, README, and skill now enumerate the macOS, Linux/X11, and Windows injectors and their availability contracts. |

## Verification Performed

- Read the full specification, review 10, sequence user contract, implementation
  architecture, validation matrix, Level-3 runbook, and Windows gate record.
- Used the current code index to trace `execute_sequence` into preflight, JIT
  execution, task/group handling, and the corresponding test suites.
- `just test-library composition::sequence` passed 245/245 on macOS.
- `just test-cli sequence` passed 146/146 on macOS.
- A focused `just test-l2` run passed all nine Sequence Plus task-stream capture
  cases in tmux. The idle-flush case passed in 77.884s; its source documents an
  expected ~80s runtime because it must cross two 30s idle ticks.
- `just lint` was started but stopped with exit `130` after exceeding the
  non-interactive session's command-duration limit. No lint error appeared
  before termination, but this review does not claim a completed lint gate.
- Level 3 was not run because this non-interactive session is not authorized to
  take desktop focus or inject host keyboard events. Native Windows execution
  is unavailable from this macOS host.

## Production Readiness

`ready: false` is required. Functional L1 and rendering L2 behavior are green,
but the feature cannot be production-ready until its macOS and Windows
keyboard contract has current Level-3 evidence, the native Windows process
control paths execute successfully, and the acceptance matrix describes one
reproducible release candidate.

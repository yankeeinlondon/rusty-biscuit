---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T13:40:32-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-12.md
previous: 2026-07-11-sequence-plus/review-11.md
---

# Review 12: Sequence Plus

## Verdict

**Not ready for production.** The implementation and its Level-1 and Level-2
coverage are strong: the review baseline passed 247 sequence-library tests, 146
sequence CLI tests, all 148 then-existing Level-2 real-terminal tests, lint, and
the Windows cross-target type-check. A subsequent macOS Level-3 run passed the
Sequence Plus Ctrl+C fixture, and investigation of that run's unrelated
autocomplete failures moved six chooser interactions to Level 2, where the
focused suites pass. Review 11's stale acceptance-record finding is closed.
The release remains blocked because Linux and Windows lack candidate-current
Level-3 Ctrl+C results, and the distinct native Windows process and
console-control paths have never executed.

## Findings

### 1. High — Ctrl+C Level-3 acceptance is incomplete across supported OSes

The specification promises that pressing Ctrl+C during a parallel group fans
out interruption to every task and descendant, suppresses later work, and exits
`130`. This is a user-keyboard requirement. Level-1 signal manufacture and
Level-2 terminal-CLI byte injection do not exercise a terminal emulator's input
encoder, so they cannot verify it.

The user-provided macOS run against the current working tree passed
`level3_sequence_ctrl_c_fans_out_to_parallel_children` and both wrapped-child
Ctrl+C cases. Its eight failures were autocomplete chooser tests, not Sequence
Plus interruption tests. Investigation showed that the operation-file and YAML
selection cases exercise PTY-visible chooser behavior adequately at Level 2;
their apparent YAML acceptance defect was also an invalid formal-sequence
fixture with no usable composed prompt. Six interaction cases now pass at Level
2, the eight redundant operation-file Level-3 cases were removed, and one
focused macOS keyboard-encoder smoke remains for autocomplete.

This supplies current working-tree macOS evidence for the Sequence Plus
keyboard path, but it is not yet anchored to a clean release candidate. Linux
still has only the obsolete R3 result, and Windows has never run.

Required change:

- Record the successful macOS run against the exact release-candidate revision,
  rerunning it if the candidate differs from the tested working tree.
- Run the platform Level-3 fixture against the same candidate on Linux and
  Windows, with `BISCUIT_TEST_LEVEL_REQUIRED=3` so a missing backend fails.
- Record the injected key path, pane evidence, exit `130`, later-step
  suppression, and descendant cleanup. Do not claim the keyboard contract ready
  on an OS whose fixture has not run on the candidate.

### 2. High — native Windows process ownership and interruption remain unexecuted

Windows uses a separate implementation for suspended process creation, Job
Object assignment, thread resume, console-control delivery, reader settlement,
Job termination, and kill-on-close ownership. `just check-windows` passed on
the current tree, but it only type-checks; it does not link or execute these
paths. The Windows Level-1 Job/process-tree tests, attached-console gate, and
Level-2 sequence fan-out fixture have never run on a Windows host.

This leaves success cleanup, ordinary failure, timeout, runaway output,
ownership-establishment failure, inherited-pipe closure, Ctrl+Break delivery,
and descendant cleanup behaviorally unverified on a supported platform.

Required change:

- On native Windows, run the Windows Level-1 process-tree and Job Object tests,
  `just test-windows-ctrl-c`, and the Windows Level-2 sequence fixture at the
  release-candidate revision.
- Record every exit path separately. Keep the Level-3 keyboard run from finding
  1 as a distinct gate.

### 3. Medium — the acceptance matrix is reproducible but no longer anchored to the current tree

Review 11's obsolete-dirty-tree problem is fixed: `validation-matrix.md` now
names clean commit C11 and accurately separates executed evidence from
type-check evidence. Since then, the branch advanced to `1ac55f860`, including
Darkmatter file-resolution changes consumed by sequence composition. C11 is
therefore no longer the release candidate for this review.

This review initially reran the affected sequence suites, the complete
then-existing Level-2 tier, lint, and the Windows cross-target type-check on the
current tree, all green. The autocomplete test-tier correction subsequently
added six Level-2 cases. Both affected Level-2 binaries pass in isolation, but
the complete expanded Level-2 tier has not been rerun. The durable acceptance
artifact should still be re-anchored so future readers do not have to
reconstruct review 12's evidence from this file.

Required change:

- Re-anchor `validation-matrix.md` to one clean release-candidate commit that
  contains the final review-12 changes.
- Rerun and record the complete expanded Level-2 tier alongside the current
  247/247 library, 146/146 CLI, lint, and Windows compile-only results.
- Record the current macOS Sequence Plus Level-3 pass separately from the stale
  Linux result and the missing Windows result.

## Requirement Verification Levels

| User-facing requirement | Strongest current evidence | Assessment |
|---|---|---|
| State/navigation shape, source grammar and formats, strict/lenient normalization, `FileReference` resolution, preflight, JIT layering, schemas, lifecycle ordering, deterministic merge, and `outputs` | Level 1; 247/247 focused library tests and 146/146 focused CLI tests passed | Appropriate level. |
| CLI fail-fast, dry-run, typed diagnostics, task/group execution, output capture, and stdout/stderr contracts | Level 1 CLI execution and PTY tests | Appropriate level. |
| Parallel glyphs, colors, wrapping, no-color labels, channel separation, idle flush, zero-step styling, and synchronized writes | Level 2; the review baseline passed 148/148, including all nine Sequence Plus captures | Appropriate level; the later autocomplete additions do not change these feature cases. |
| Linux user Ctrl+C reaches a parallel sequence through the terminal encoder | Historical Level 3 at obsolete R3 only | Wrong candidate; finding 1. |
| macOS user Ctrl+C interrupts tasks and descendants, suppresses later work, and exits `130` | Current working-tree Level 3 passed | Correct level, but the result still needs a clean candidate anchor; findings 1 and 3. |
| Windows user Ctrl+C interrupts tasks and descendants, suppresses later work, and exits `130` | Fixture type-check only | Wrong verification level; finding 1. |
| Native Windows Job ownership, console interruption, pipe closure, and cleanup | Cross-target type-check only | No behavioral verification; finding 2. |

No paste, IME, mouse, or other keybinding behavior is specified by this
feature, so no additional Sequence Plus Level-3 requirement was identified.
The corrected autocomplete chooser coverage is outside this feature and does
not block its readiness independently.

## Review 11 Closure

| Review 11 finding | Review 12 status |
|---|---|
| Current macOS and Windows Ctrl+C lacked Level-3 evidence | Partially closed: macOS now passes on the current working tree; Linux is stale and Windows remains unexecuted. |
| Native Windows process ownership and interruption were unverified | Open; current type-check is green but no Windows runtime executed. |
| Acceptance record described an obsolete dirty snapshot | Closed for C11. The matrix now has a clean reproducible anchor, but finding 3 tracks normal re-anchoring to the newer review-12 tree. |

## Verification Performed

- Read the complete specification, review 11, validation matrix, Level-3
  runbook, platform gate records, sequence documentation, implementation, and
  sequence test inventory.
- `just test-library composition::sequence`: **247/247 passed**.
- `just test-cli sequence`: **146/146 passed**.
- Before the autocomplete test-tier correction, `just test-l2 level2_`:
  **148/148 passed**. All nine Sequence Plus captures passed; the intentional
  idle-flush case completed in 80.838 seconds.
- User-provided macOS Level-3 run: **15 tests executed, 7 passed and 8 unrelated
  autocomplete tests failed after four attempts**. The Sequence Plus parallel
  Ctrl+C fixture and both wrapped-child Ctrl+C fixtures passed.
- After correcting the autocomplete test tiers,
  `just test-l2 --test level2_auto_complete_operation_file`: **19/19 passed**,
  including Markdown operation-file and YAML sequence Enter, `y`, and arrow
  navigation behavior.
- `just test-l2 --test level2_auto_complete_chooser`: **6/6 passed**.
- `cargo check -p claudine-cli --tests`: **passed**.
- The focus-stealing API placement guard passed. The retained Level-3
  autocomplete test is a single macOS keyboard-encoder smoke, while chooser
  product behavior is covered at Level 2.
- `just lint`: **passed**, including the 18/18 diagnostic guard suite and all
  five Claudine package lint phases.
- `just check-windows`: **passed** for production and test targets. This is
  compile/type-check evidence only; nothing Windows executed.
- The first unconstrained `just test` attempt was inconclusive under severe
  host contention: the library completed 3,840/3,840, but four unrelated CLI
  header tests exhausted their 30-second retries and nextest fail-fast canceled
  the remainder. A timing-based sequence overlap test also exceeded its wall
  threshold while many process-spawning tests competed, then the isolated
  sequence suites passed. No product defect is inferred from that run, and it
  is not claimed as a green full-suite gate.
- This review-update session did not rerun Level 3 because it is non-interactive
  and must not take desktop focus or inject host keyboard events. The complete
  expanded Level-2 tier was also not rerun after the focused corrections.
  Native Windows execution is unavailable from this macOS host.

## Production Readiness

`ready: false` is required. Functional Level-1 and real-terminal Level-2
behavior are green in the recorded baseline and affected focused runs, the
macOS Sequence Plus Level-3 fixture is green on the current working tree, and
no new implementation defect was found. Production readiness still requires a
clean evidence anchor, candidate-current Linux and Windows Level-3 keyboard
evidence, and native Windows execution at the levels identified above.

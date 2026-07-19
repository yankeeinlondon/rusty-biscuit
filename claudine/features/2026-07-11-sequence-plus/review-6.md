---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T12:11:21-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-6.md
previous: 2026-07-11-sequence-plus/review-5.md
next: 2026-07-11-sequence-plus/review-7.md
---

# Review 6: Sequence Plus

## Verdict

**Not ready for production.** Review 5's live shell streaming, no-color
isolation, and preflight-ordering findings are closed, and the complete
Level-1 area gate is green. Three high-severity blockers remain: a real
Level-2 task-stream rendering test fails, shell process-tree ownership is not
fail-closed or cross-platform consistent, and the keyboard-driven interruption
contract still has no current Level-3 evidence.

## Findings

### 1. High — the required ASCII task-header fallback fails in a real terminal

`level2_non_utf8_locale_uses_the_ascii_header_glyph_in_tmux` sets `LC_ALL`,
`LANG`, and `LC_CTYPE` to `C`, runs a parallel group in tmux, and requires the
task headers to use `>`. All four nextest attempts rendered `▶` instead:

```text
│ ▶ fetch-data
│ ▶ render-page
```

This is the correct verification level for a glyph/capability claim: unlike a
constructed `Terminal`, the test exercises the capability selection inherited
by the real process and captures what the pane rendered. The checked-in
validation matrix calls this a cold-server artifact and then states that no
Sequence Plus Level-2 test failed. This review reproduced the failure on four
consecutive attempts, so neither statement is acceptable release evidence.

Required change:

- Trace the terminal construction used by sequence task streams and preserve
  locale-derived `supports_unicode` even when color or geometry is forced.
- Make the fixture independent of tmux server lifetime and ambient environment;
  explicitly control every capability override that can select an optimistic
  terminal.
- Rerun the cold- and warm-server Level-2 cases and require the canonical gate
  to return zero. A passing unit test with `supports_unicode(false)` is useful
  diagnosis but cannot replace this Level-2 assertion.

### 2. High — shell process-tree ownership remains fail-open and differs by OS

`SystemTaskShell::run` now creates a Unix process group or a Windows Job Object
and terminates it on timeout, interruption, or output overflow. The ownership
contract is still incomplete on other exits:

- On Unix, `ProcessTree` has no `Drop` cleanup. If the direct shell exits
  normally—or `try_wait`/`wait` returns an error—a background descendant is
  left alive. After the two-second reader grace, the reader thread is detached
  while it still owns the live-output handle. A descendant retaining a pipe can
  therefore emit frames after the task footer or during a later task.
- On Windows, Job Object creation, configuration, and assignment errors are
  discarded and represented as `job: None`; later termination then kills only
  the direct child. That silently reintroduces review 5's defect on a supported
  OS.
- The Windows child is spawned normally and assigned to the Job afterward. It
  is not created suspended, so the comment claiming assignment occurs before
  the child can create descendants is not an enforceable invariant. A fast
  child can create an unowned descendant in that interval.
- A successful command that backgrounds work is reaped at Job drop on Windows
  but deliberately survives on Unix. The same authored sequence therefore has
  different process-lifetime semantics across supported platforms.

Required change:

- Choose one success-path policy and enforce it on macOS, Linux, and Windows.
  For an owned per-task process tree, the least surprising policy is to reap
  remaining descendants when the command completes.
- Make ownership establishment fallible and abort the task with a typed error
  if isolation cannot be guaranteed; do not silently degrade to direct-child
  cleanup.
- Establish Windows Job membership without a spawn/assignment race, such as a
  suspended creation/assignment/resume sequence or another documented
  race-free design.
- Add real-process regressions for normal-success background descendants,
  early wait errors, late writes through inherited pipes, and failed ownership
  setup. Run the Windows cases on native Windows; type-checking is not runtime
  evidence.

### 3. High — Ctrl+C still lacks the required current Level-3 evidence

The specification says that pressing Ctrl+C during a parallel group fans out
to every running child, records interruption, suppresses later steps, and exits
130. That is a keyboard UX contract, so only OS keyboard injection verifies the
terminal encoder and the complete signal path.

The macOS Level-3 pass is from the pre-review-4 tree and predates the task-shell
process-tree rewrite. The Linux and Windows Level-3 fixtures have never run on
any host. Level-1 flag/process tests and Level-2 byte injection remain valuable
diagnostics, but they cannot promote this requirement to production-ready.

Required change:

- From attended native sessions, run the current revision's Level-3 fixture on
  macOS, Linux, and Windows.
- Record the revision, host, terminal, injected keyboard event, pane capture,
  exit `130`, absence of the later-step marker, and descendant cleanup.
- Keep the lower-tier tests so a Level-3 failure can be localized without
  weakening the acceptance level.

### 4. Medium — the validation record is stale and overstates current evidence

The matrix still defines R0 as `be2d100a6 + uncommitted working tree`, says that
review 5 produced no commits, and reports approximately 6,156 Level-1 passes.
The current reviewed revision is `a7bfdd7a7`, the review-5 work is committed,
and this run contains 3,792 library tests alone. More importantly, the matrix
classifies the failing ASCII task-stream capture as a pre-existing artifact
while claiming that no Sequence Plus Level-2 test failed.

Refresh the matrix after the implementation blockers are fixed. Every green
claim must name the exact revision, host, level, command, and result; a gate
with a feature-owned failure must be red, even if it sometimes passes under a
different tmux-server state.

### 5. Low — runaway errors discard the useful trip counters

The capture guard detects whether the byte or line limit tripped, but the
sequence shell result reduces that to one `aborted` boolean. The resulting
`SequenceTaskShellRunaway` diagnostic names the task and command without the
observed line count, byte count, or configured limit. Preserve the guard's trip
details in `ShellCommandOutput` and the typed diagnostic so users can distinguish
a slightly oversized command from an uncontrolled flood.

## Requirement Verification Levels

| User-facing requirement | Strongest evidence on the reviewed tree | Assessment |
|---|---|---|
| State shape, navigation, JIT composition, expression-backed sources, and deterministic merge semantics | Level 1 | Appropriate for in-process data and evaluation semantics; current area gate is green. |
| Serial and bounded-parallel scheduling, lifecycle ordering, and positional outputs | Level 1 plus CLI process tests | Appropriate for scheduling and returned-data behavior. |
| Live, attributed shell stdout/stderr with preserved arrival order | Level 1 channel/order tests plus Level 2 tmux in color and no-color | Appropriate; both interleaved shell captures passed in this review. |
| Task bars, textual attribution, wrapping, zero-step notice, prompt frames, and idle flush | Level 2 tmux | Appropriate real-terminal level; the reviewed cases passed. |
| ASCII task header under a non-UTF-8 locale | Level 2 tmux, failing four attempts | **Correct level, failing behavior; finding 1.** |
| Ctrl+C key press interrupts all parallel work, suppresses later steps, and exits 130 | Historical macOS Level 3; Linux and Windows never run | **Wrong/stale level evidence; finding 3.** |
| Timeout, interruption, and runaway limits terminate the whole shell tree | Level 1 real-process tests on macOS; Windows type-check only | Abort branches have useful Level-1 evidence, but ownership is incomplete and native Windows behavior is unverified; finding 2. |
| Cross-platform process lifetime and native Windows Job behavior | macOS Level 1 plus Windows source audit/type-check | **No native Windows runtime evidence, and the implementations disagree; finding 2.** |

## Review 5 Closure

| Review 5 finding | Review 6 status |
|---|---|
| Shell timeout/interruption killed only the direct child and capture was unbounded | Partially closed: abort-path tree termination, bounded capture, and bounded reader settling are implemented, but finding 2 identifies fail-open ownership and divergent normal-exit semantics. |
| Parallel shell output was buffered and stderr bypassed attribution | Closed: Level-1 channel tests and both current Level-2 interleaving captures verify live attributed stdout/stderr in color and no-color modes. |
| Ctrl+C lacked current Level-3 evidence | Open: fixtures exist, but macOS is stale and Linux/Windows have never run. |
| Level-1 palette test depended on ambient `NO_COLOR` | Closed: the full Level-1 gate passes with the current environment, and explicit color/no-color helpers isolate the rendering contracts. |
| Preflight status appeared after preflight | Closed: source ordering and the Level-2 captures show the status before Phase 1c completion. |
| Validation record contained stale provenance | Open/regressed: finding 4. |

## Verification Performed

- Read the specification, review 5, validation matrix, Level-3 runbook,
  sequence/task architecture, production shell runner, task-stream renderer,
  CLI terminal selection, and relevant Level-1/Level-2/Level-3 tests.
- Used the repository code index to inspect the task-shell execution flow and
  blast radius before the source audit.
- `just test --no-fail-fast`: passed across all five Claudine packages. The
  library reported 3,792 passed and seven skipped; `claudine-gen` reported 152
  passed and four skipped.
- `just test-l2 --no-fail-fast`: 146 run, 142 passed, four failed. Three failures
  are the unrelated `level2_context_*_at_140_fills_cap_in_tmux` cases. The
  feature-owned ASCII fallback test failed all four attempts. The two new live
  shell interleaving captures passed, including no-color attribution.
- `just lint`: the error-guard tests passed, and the library/contract checks
  completed; the full gate was stopped during the CLI dependency build after
  exceeding this non-interactive session's approximately 60-second subprocess
  limit. It is therefore **incomplete**, not green.
- Level 3 was not run. The session is non-interactive, and the repository's
  focus guard correctly reserves OS keyboard injection for an attended run.
- Native Windows tests were not run; checked-in cross-target evidence remains
  type-check-only.

## Production Readiness

`ready: false` is required. Findings 1 and 2 are current implementation defects,
and finding 3 is a mandatory verification-level gap for the feature's keyboard
contract. The feature should not be marked production-ready until all three are
closed and the canonical Level-2 gate passes at the reviewed revision.

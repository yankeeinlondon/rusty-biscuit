---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T17:13:20-08:00
spec: 2026-11-14-foreground-tree-handoff/spec.md
implemented: false
description: "A **fix** review of `2026-11-14-foreground-tree-handoff/spec.md`"
fix: 2026-11-14-foreground-tree-handoff/review-2.md
previous: 2026-11-14-foreground-tree-handoff/review-1.md
---

# Review 2: foreground tree handoff

The fix is **production ready** under this review's criteria. The previous
finding is verified fixed across every required instance. No partial repair,
regression, new sibling defect, or new finding remains. No human decision is
needed.

## Previous Finding Verification

Read `review-1.md` and the repair log's `Finding Ledger` before inspecting
the implementation. There are no reviews before iteration 1 or human rulings
in this directory. The table includes the seven original instances and the four
additional sites enumerated by the repair ledger. A clean site is recorded
as verified fixed when its stated invariant rules the defect out; it does
not imply that the repair changed that site.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: terminal capability probing stops background Claudine jobs | R1-F1.1–R1-F1.11 below | verified fixed | In biscuit-terminal, `RawModeGuard::new` checks foreground ownership before `tcgetattr` or `tcsetattr`. An unknown or different group returns an error. The sole production construction in `tty_query/unix.rs::open_port` propagates it before any query write or read. Focused L1 and actual-terminal L2 runs pass. | Nothing |
| R1-F1.1: inherited output | Background Claudine with both streams inherited | verified fixed | `a_background_claudine_with_terminal_output_completes` passes at L2. The lifecycle command and fake provider record background groups; the shell retains foreground ownership and its settings. | Nothing |
| R1-F1.2: stdout redirected | Background Claudine with stdout captured | verified fixed | `a_background_claudine_with_stdout_redirected_completes` passes with the same execution, ownership, settings, and descendant-cleanup assertions. | Nothing |
| R1-F1.3: stderr redirected | Background Claudine with stderr captured | verified fixed | `a_background_claudine_with_stderr_redirected_completes` passes with the same assertions. | Nothing |
| R1-F1.4: both streams redirected | Original background control | verified fixed | `a_background_claudine_leaves_the_foreground_alone` passes; provider-group assertions now supplement command-group, descendant-cleanup, and terminal-restoration checks. | Nothing |
| R1-F1.5: foreground launch | Positive control for normal command execution and probing | verified fixed | Claudine's foreground success, failure, structured-command, nested-command, and cancellation L2 cases pass. Biscuit-terminal's foreground raw-mode test and L1 manufactured-reply query tests pass, proving queries still execute in the foreground. | Nothing |
| R1-F1.6: no lifecycle actions | Background document without `start` | verified fixed | `a_background_claudine_without_lifecycle_actions_completes` passes; the provider runs in the background and the shell retains its terminal and settings. | Nothing |
| R1-F1.7: background handoff | Sniff preparation must stay inactive | verified fixed | `a_background_host_is_inactive_and_mutates_nothing` passes; its scripted operations record no transfer or restoration and its signal mask stays unchanged. Claudine's background L2 cases prove the caller wiring. | Nothing |
| R1-F1.8: stdin convenience guard | Same foreground check through `RawModeGuard::stdin()` | verified fixed | Source inspection: directly delegates to `new(STDIN_FILENO)`, with no alternate mutation path or production caller. | Nothing |
| R1-F1.9: cell-size ioctl | Separate `/dev/tty` opener | verified fixed | `fonts/window_size.rs::cell_size_ioctl` only performs `TIOCGWINSZ`; it does not read terminal input or change settings. Its escape-query fallback uses the guarded shared exchange. | Nothing |
| R1-F1.10: native Windows query port | Platform boundary | verified fixed | The new guard is Unix-only; native Windows retains its separate console port and has no Unix foreground-group job control. This is source verification, not a claim of Windows runtime evidence. | Nothing |
| R1-F1.11: deliberate interactive raw mode | biscuit-tui standalone and Claudine configuration/autocomplete prompts | verified fixed | These enter raw mode for an explicitly interactive operation, rather than optional capability discovery. They are outside the reported defect class and require no change. | Nothing |

The new background cases use a fresh document and scripts in an isolated
workspace, launched by the shipped Claudine binary inside detached tmux
panes. They verify successful command/provider execution, background group
identity, unchanged shell terminal settings, foreground ownership, subsequent
shell input, and cleanup of the lifecycle command's background child. The
terminal-query guard is not replaced by a fake in these cases.

The two new raw-mode L1 tests separately verify refusal when foreground
ownership cannot be read, and foreground success followed by background
refusal on a controlling PTY. The background child is observed with
`waitpid(WUNTRACED)`, so a job-control stop fails the test rather than being
mistaken for completion.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification levels

The review re-read the specification, plan, implementation record, shared
guard, both caller integrations, terminal-query repair, and relevant tests
and documentation. The two author rulings remain satisfied: one shared
handoff implementation, used by both callers.

The central sites are Sniff's
[ForegroundHandoff](/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/lib/src/process/foreground.rs)
(owns terminal state), Claudine's
[run_owned_lifecycle_command](/Volumes/coding/wt/rusty-biscuit/fix-sequences/claudine/lib/src/composition/sequence/task/shell.rs:892)
(settles each lifecycle command's tree before restoring the terminal),
Darkmatter's
[LaunchPlan](/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/lib/src/editor/launch.rs)
(launches resolved editors), and biscuit-terminal's
[RawModeGuard](/Volumes/coding/wt/rusty-biscuit/fix-sequences/biscuit-terminal/lib/src/discovery/raw_mode.rs)
(now refuses optional terminal probes from background groups).

| User-facing requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Own a fresh child group and restore the host's foreground and settings after success or failure | Sniff L1 scripted operation ordering and failures; Sniff L2 typed input, nonzero exit, failed spawn after transfer, and unconfigured-child refusal | Met |
| Restore during panic unwind | Sniff L1 drop-order/claim-release test and L2 unwind test with an actual owned tree | Met |
| Lifecycle cancellation stops the command and its background child, then restores the terminal | Claudine L2 typed Ctrl-C and host-directed SIGINT cases check root/descendant death, cancellation/finalization events, no later normal action or provider, and restored shell input/settings | Met |
| Exact-byte and structured lifecycle commands share ownership and handoff | Both routes call Claudine's shared lifecycle runner; structured and exact-byte L2 cases exercise it | Met |
| Darkmatter uses the same handoff and retains editor behavior | `LaunchPlan` uses Sniff's guard before spawn, settles the tree before restoration, and preserves primary errors; both existing editor L2 tests pass and remain unchanged against the pre-handoff commit | Met |
| Native Windows policy stays unchanged; WSL2 follows Unix | Inactive non-Unix guard; unchanged Windows process-tree configuration; Unix foreground implementation used on Linux/WSL2 | No policy regression found; cross-OS runtime proof remains CI's responsibility |
| Canonical process-tree page documents the handoff | Page covers API order, inactive cases, contention, cleanup/restoration, keyboard signal delivery, and platform limits; obsolete foreground exception removed | Met |
| Foreground lifecycle reads a line, starts a background child, is cancelled, and returns the terminal without taking focus | Detached-tmux L2 tests inject the line and Ctrl-C into their own panes and observe groups/settings/process death | Met at the spec's requested L2 level |

The keyboard behavior here depends on the terminal driver's delivery of
bytes and signals, rather than an application-specific key encoder. The
spec explicitly asks for L2; these detached-terminal tests supply it without
claiming L3 OS keyboard injection or taking window focus. L1 tests alone
are not used as proof of the actual foreground transfer.

No file-format or configuration reader changed, so the input robustness
matrix does not apply.

## Verification performed

Independent runs in this review, on macOS:

- `cd biscuit-terminal && just test raw_mode tty_query level1_`: 62 passed;
  longest 1.278 s; no slow L1 test. Includes the new refusal tests and the
  foreground query controls.
- Root `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests,test-fixtures level2_lifecycle_foreground_handoff level2_lifecycle_ctrl_c_tmux level2_edit_action level2_edit_interactive_capture`:
  27 passed; backend proof `tmux run=27 skip=0 panic=0`. All twelve lifecycle
  handoff cases ran, including all five background cases.
- `cd sniff && just test process::foreground`: 12 passed; longest 0.018 s;
  no slow L1 test.
- `cd claudine && just test lifecycle_tests interrupt_stops`: 16 passed;
  longest 0.194 s; no slow L1 test. Covers wait-error cleanup, cancellation,
  structured execution, interrupt classification, and cleanup-event policy.
- `cd sniff && BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_foreground_handoff`:
  five passed; longest 0.588 s; backend proof `tmux run=5 skip=0 panic=0`.
- `cd darkmatter && BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_edit_action`:
  both darkmatter-cli tests passed (0.860 s and 0.900 s). The library leg
  and language-server leg selected no tests. The final backend counter was
  `tmux run=7 skip=0 panic=0`, including the five overlapping Sniff tests;
  this is evidence of two Darkmatter tests, not seven.
- `just check-tier-coverage biscuit-terminal claudine sniff darkmatter`:
  zero stranded tests. Source checks confirm consolidated module declarations,
  live tier recipes, and enabled fixture/terminal features.
- `git diff --check` on the repaired source and L2 test: clean. Existing
  Darkmatter and Claudine `level2_edit_action.rs` files have no diff against
  `bc0870378`, the pre-handoff shared-tree commit.

The repair log's broader macOS L1/lint and Linux evidence is recorded
evidence, distinct from the independent runs above. Its full L1 runs report
no slow tests. The original implementation's 8.5 s network outlier was
already cleared by the isolated and full-suite remeasurements accepted in
review 1; no new timing issue was found. No cross-OS execution claim is
added by this review.

## Implementation record accuracy

No statement was verified false in either implementation record. The
original implementation log's account of leaving the background-probe
defect unfixed describes the earlier implementation phase; the later repair
log records its repair and the new controls. Those are consistent historical
records, not contradictory claims about the current code. No factual
correction or human review item is needed.

## Recurrence

None. R1-F1 is closed, and this review introduces no finding in its class.
The additional sibling sites listed by the repair were checked and remain
clean. Accepted decisions from review 1 are unchanged and are not reopened.

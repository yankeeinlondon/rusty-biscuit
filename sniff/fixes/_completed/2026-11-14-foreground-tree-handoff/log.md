---
spec: 2026-11-14-foreground-tree-handoff
review: 2026-11-14-foreground-tree-handoff/review-1.md
implementation_1: "2026-11-16T16:50:11-08:00"
---

# Review Fix Log: foreground tree handoff

## Implementation of Review Findings #1

> **started at:** 2026-11-16T16:50:11-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-foreground-tree-handoff/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- the review has one parent finding (R1-F1), no `human_in_the_loop` items, no blockers, and recurrence `false`

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Terminal capability probing stops background Claudine jobs; optional probes must fall back when the caller does not own the terminal foreground | fixed | `RawModeGuard::new` refuses (via `ensure_foreground`) when `tcgetpgrp(fd)` fails or differs from `getpgrp()`, before any settings change or query I/O; every child row below is fixed or clean; L1 `raw_mode` tests (2) and 12 foreground-handoff L2 tests pass |
| R1-F1.1 | Background Claudine, both output streams inherited: action and provider run | fixed | `level2_lifecycle_foreground_handoff_a_background_claudine_with_terminal_output_completes` PASS (0.84 s); before the fix it failed with status 150, `Stopped(SIGTTOU)` |
| R1-F1.2 | Background Claudine, stdout redirected: action and provider run | fixed | `..._a_background_claudine_with_stdout_redirected_completes` PASS (0.85 s); before the fix: status 150, SIGTTOU |
| R1-F1.3 | Background Claudine, stderr redirected: action and provider run | fixed | `..._a_background_claudine_with_stderr_redirected_completes` PASS (0.85 s); before the fix: status 150, SIGTTOU |
| R1-F1.4 | Background Claudine, both streams redirected (control): action and provider run | clean | control `..._a_background_claudine_leaves_the_foreground_alone` passes before and after; now also asserts the provider ran outside the foreground group; stale SIGTTOU comment removed |
| R1-F1.5 | Foreground launch (control): action and provider run | clean | the eight foreground-launch L2 tests in the same file pass; biscuit-terminal `level1_` PTY probe tests (foreground session leader) pass, so the foreground query path still enters raw mode |
| R1-F1.6 | Background launch with terminal output and no `start` field: completes without `SIGTTOU` | fixed | `..._a_background_claudine_without_lifecycle_actions_completes` PASS (0.79 s); before the fix: status 150, SIGTTOU |
| R1-F1.7 | Sniff foreground preparation for a background host: inactive, mutates nothing | clean | Sniff untouched; `process::foreground::tests::unix::a_background_host_is_inactive_and_mutates_nothing` PASS (0.021 s) |
| R1-F1.8 | Sibling: `RawModeGuard::stdin()` (no production caller) | fixed | delegates to `new()`, so inherits the foreground check |
| R1-F1.9 | Sibling: `fonts/window_size.rs::cell_size_ioctl` opens `/dev/tty` | clean | only a read-only `TIOCGWINSZ` ioctl; it neither changes settings nor reads input, so raises no job-control signal |
| R1-F1.10 | Sibling: `tty_query/windows.rs` port | clean | native Windows has no job control or foreground process group |
| R1-F1.11 | Sibling: `enable_raw_mode` in biscuit-tui standalone, claudine `config_tui`, `completion/autocomplete_ui.rs` | clean | deliberate interactive prompts the user asked for, not optional discovery; outside the defect class |

### R1-F1 work

- starting the work on 'R1-F1' at 16:51:14
        - root cause confirmed: `RawModeGuard::new` (biscuit-terminal/lib/src/discovery/raw_mode.rs) ran `tcsetattr` on `/dev/tty` with no foreground check; `tty_query/unix.rs::open_port` builds that guard before every exchange, so a background job was stopped by `SIGTTOU` on the first startup probe
        - repair: `RawModeGuard::new` now calls `ensure_foreground(fd)` before `tcgetattr`; it returns `Err` when `tcgetpgrp(fd)` fails or differs from `getpgrp()`, which `open_port` maps to `TtyQueryError::Io`, so every caller takes its existing fallback; no query bytes are written or read; `SIGTTOU` is neither ignored nor blocked; `stdin()` inherits the check
        - no process-wide short-circuit was added: a background job can be brought to the foreground with `fg`, so caching "background" would be wrong, and the check costs one `open` plus one `tcgetpgrp` per query
        - guard `## Errors` docs updated; `biscuit-terminal/docs/discovery/terminal-queries.md` (Platforms) and `.claude/skills/biscuit-terminal/discovery.md` now describe the background refusal
- sweep (defect class: optional terminal discovery that mutates or reads a terminal from a background process)
        - `tty_query/unix.rs::open_port`: the only production `RawModeGuard` construction; fixed through the guard
        - `RawModeGuard::stdin()`: no production caller; inherits the check
        - `fonts/window_size.rs::cell_size_ioctl`: opens `/dev/tty` for a read-only `TIOCGWINSZ` ioctl, which raises no job-control signal; clean
        - `tty_query/windows.rs`: native Windows has no job control; clean
        - no other `tcsetattr`, `/dev/tty`, stdin read, `tcflush`/`tcdrain`, or `tcsetpgrp` in `biscuit-terminal/lib/src` (`detection/versions.rs` spawns children with `Stdio::null()`); clean
        - `enable_raw_mode` in biscuit-tui standalone, claudine `config_tui`, and `completion/autocomplete_ui.rs`: deliberate interactive prompts, not optional discovery; out of class
- pre-fix evidence (the `ensure_foreground` call disabled temporarily; restored afterwards)
        - L1 `raw_mode_is_entered_from_the_foreground_and_refused_from_the_background`: FAIL, child code 22, meaning the background grandchild was stopped by `SIGTTOU`
        - L1 `a_terminal_whose_foreground_cannot_be_read_is_refused_unchanged`: FAIL, raw mode was entered on a pty that is not the controlling terminal
        - L2 with terminal output, stdout redirected, stderr redirected, and no `start` field: all FAIL, status 150; the pane shows `[1]+ Stopped(SIGTTOU) sh -c '... exec claudine compose --goose --yolo doc.md'` after "Preflight: shell commands approved"
        - L2 redirected-output control `a_background_claudine_leaves_the_foreground_alone`: PASS
- post-fix results
        - `cd biscuit-terminal && just test raw_mode`: 2 passed (0.011 s, 0.012 s)
        - `cd biscuit-terminal && just test tty_query`: 10 passed in 0.198 s
        - `cd biscuit-terminal && just test level1_`: 52 passed in 2.159 s; the PTY probe tests run `discovery_probe` as the pty's session leader, so they cover the foreground positive path end to end; none at or above 5 s
        - `cd biscuit-terminal && just lint`: clean
        - root `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests,test-fixtures level2_lifecycle_foreground_handoff`: 12 passed (0.84-1.41 s each), backend proof `tmux run=12 skip=0 panic=0`
- work completed for 'R1-F1' at 17:01:47

### Orchestrator Reconciliation

- reviewed the combined diff: the guard check sits before `tcgetattr`, returns `Err` without touching the terminal, and is mapped to `TtyQueryError::Io` by `open_port`, so every query (startup probes, OSC, cursor position, window size) takes its existing fallback; `SIGTTOU` is neither ignored nor blocked
- decision (ledger only): background ownership is not cached process-wide, because a job resumed with `fg` should probe normally; the check costs one `open` and one `tcgetpgrp` per query
- accepted residual: a job moved to the background between `tcgetpgrp` and `tcsetattr` (a microsecond window) is stopped exactly as any terminal program would be; closing that would require blocking `SIGTTOU`, which the review forbids
- unrelated concurrent author edits left untouched: `claudine/docs/topics/config-files.md` (deleted) and `claudine/docs/topics/configuration/` (new), `darkmatter/fixes/2026-11-16-warn-or-error/`, and the pre-existing `spec.md` edit

### Completion Report

- **outcome: complete**; elapsed 22 minutes (16:50:11 to 17:12)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 11 (fixed 5: R1-F1.1, R1-F1.2, R1-F1.3, R1-F1.6, R1-F1.8; clean 6: R1-F1.4, R1-F1.5, R1-F1.7, R1-F1.9, R1-F1.10, R1-F1.11); none open
- open findings: none
- changed files
        - `biscuit-terminal/lib/src/discovery/raw_mode.rs`: `ensure_foreground` check, `## Errors` docs, two L1 pty tests (R1-F1, R1-F1.8)
        - `claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs`: four inherited-output and no-lifecycle background tests, the provider records its groups, the control's stale comment is replaced (R1-F1.1–R1-F1.4, R1-F1.6)
        - `biscuit-terminal/docs/discovery/terminal-queries.md` and `.claude/skills/biscuit-terminal/discovery.md`: a background process sends no terminal queries
- final verification (macOS unless noted, load average 5–9 on 16 cores)
        - `cd biscuit-terminal && just test`: 4026 passed, 55 skipped; none at or above 5 s (R1-F1, R1-F1.5, R1-F1.8)
        - `cd biscuit-terminal && just lint`: exit 0
        - `cd claudine && just test`: 12261 passed, 15 skipped; none at or above 5 s
        - `cd claudine && just lint`: exit 0
        - root `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests,test-fixtures level2_lifecycle_foreground_handoff level2_lifecycle_ctrl_c_tmux level2_edit_action level2_edit_interactive_capture`: 27 passed, backend proof `tmux run=27 skip=0 panic=0` (R1-F1.1–R1-F1.6; existing edit tests unchanged)
        - `cd sniff && just test foreground`: 12 passed, including `a_background_host_is_inactive_and_mutates_nothing` (R1-F1.7)
        - `just check-tier-coverage biscuit-terminal claudine sniff`: zero stranded
        - `just cross-check biscuit-terminal --os linux`: 3639 passed, none slow (new `openpty`/`TIOCSCTTY` tests compile and pass on Linux)
        - the Windows cross-check was not run: all changed code and tests are `#[cfg(unix)]` and the Windows query port is unchanged

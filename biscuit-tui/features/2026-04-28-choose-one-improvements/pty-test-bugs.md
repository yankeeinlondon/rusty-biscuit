---
ready: false
status: open
scope: cli/tests/choose_cli.rs::pty (and equivalent harnesses)
related:
  - biscuit-tui/cli/tests/choose_cli.rs
  - biscuit-tui/cli/tests/keyboard_protocol.rs
  - biscuit-tui/lib/src/core/standalone.rs
---
# PTY Integration Test Harness Defects

While diagnosing the `just test` hang reported on 2026-04-30, two distinct defects in the
`mod pty` integration tests were uncovered. Both manifest **only when**
`QUESTION_INTERACTIVE_PTY=1` is set in the environment, so they have not been
seen on default CI runs.

Both defects are in the **test harness**, not in the `question` binary itself.
The binary handles `Ctrl+C`, `Esc`, and `Enter` correctly under a real PTY —
verified by an out-of-tree probe — but the harness shape is incompatible with
the runner's terminal-protocol contract that the recent fix-plan introduced
(kitty keyboard enhancement push + alt-screen + raw mode).

---

## Bug 1 — Child blocks on its own exit-time writes (PTY back-pressure)

### Symptom

`pty::ctrl_c_exits_with_code_130` and (intermittently) the other PTY tests
hang for the full 5-second timeout, then panic with
`child did not exit within 5s`. Before the fix-plan harness changes there was
no timeout at all, so the same condition presented as an indefinite hang
(matching the original `just test` report).

### Root cause

`run_standalone_with_chrome` (`lib/src/core/standalone.rs:356-403`) defers
terminal cleanup through a `TerminalGuard` whose `restore_terminal` writes
three CSI sequences just before the process returns:

| Sequence       | Source                                |
|----------------|----------------------------------------|
| `\x1b[<1u`     | `PopKeyboardEnhancementFlags`          |
| `\x1b[?1049l`  | `LeaveAlternateScreen`                 |
| `\x1b[?25h`    | `Show` (cursor) — emitted by ratatui's `Terminal::draw` final flush |

These bytes are written to the slave PTY's stdout. The harness allocates a
PTY pair via `expectrl::Session::spawn` but never reads from the master while
waiting for exit:

```rust
fn wait_exit_code(session: &OsSession) -> i32 {
    match session.get_process().wait().expect("wait for child") {
        WaitStatus::Exited(_, code) => code,
        ...
    }
}
```

After `Ctrl+C` is delivered, the runner enters `restore_terminal`, attempts
to write the CSI bytes, and **blocks** because the slave→master ring buffer
is already full (the ~168-byte initial render was never drained). With the
slave's stdout `write` blocked, the binary cannot reach `exit`, so the test's
blocking `process.wait()` deadlocks. macOS PTY ring buffer is small and
fills quickly with the alt-screen entry + initial render; once full,
back-pressure is permanent until something reads.

### Reproduction (out-of-tree probe)

```text
$ probe choose-one alpha beta gamma   (sleep 200ms, send \x03, poll)
TIMEOUT

$ probe choose-one alpha beta gamma   (sleep 200ms, drain, send \x03, drain+poll)
exited: Ok(Exited(Pid(N), 130))   in ~6µs
```

The only difference is whether the master FD is drained while waiting.

### Fix applied

`cli/tests/choose_cli.rs::pty::wait_exit_code_within` now drains the master
FD on every poll iteration so the child's exit-time writes always have
buffer space:

```rust
loop {
    match session.try_read(&mut scratch) {
        Ok(0) => break,
        Ok(_) => continue,
        Err(_) => break,
    }
}
match session.get_process().status() { ... }
```

Combined with the earlier change replacing the unbounded `process.wait()`
with a deadline-guarded `status()` poll, this restores correct exit-detection
for `pty::ctrl_c_exits_with_code_130`,
`pty::esc_restores_initial_and_exits_with_code_0`, and
`pty::choose_many_ctrl_a_then_submit_writes_all_values`.

### Why this affects the new fix-plan path specifically

Pre-fix-plan, `prepare_terminal` did **not** push keyboard enhancement
flags, so `restore_terminal` did **not** write `\x1b[<1u`. The slave→master
buffer pressure was lower and tests sometimes squeaked through. After the
fix-plan added `PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES |
DISAMBIGUATE_ESCAPE_CODES)` (Phase 3), the symmetric `Pop…` write on exit
became the straw that broke the harness — and explains why the user only
started seeing hangs in the latest worktree.

### Recommendation

Treat this as the canonical pattern for any future PTY harness in this repo:

1. Always poll `process.status()` (`WaitStatus::StillAlive`) — never
   `process.wait()` — so the test thread can never deadlock against a
   wedged child.
2. Always drain the master FD between polls so exit-time CSI writes flush.
3. Always enforce a deadline; on expiry, send `SIGKILL` and *poll for reap*
   (also non-blocking) before panicking.

The same pattern should be backported into `cli/tests/keyboard_protocol.rs`
and `cli/tests/completions_shell.rs`, which use the same risky shape today.

---

## Bug 2 — Inline mode hangs on DSR cursor-position query that the harness never answers

### Symptom

`pty::choose_one_height_100_percent_runs_end_to_end` fails with exit code 1
(ABORTED) instead of 0 (Submitted), even after Bug 1 is fixed:

```text
assertion `left == right` failed: --height 100% must submit the active option with code 0
  left: 1   right: 0
```

### Root cause

`--height 100%` selects ratatui's **Inline viewport** mode. To position the
inline view correctly, ratatui's `CrosstermBackend` issues a DSR cursor
position query (`\x1b[6n`) during `Terminal::with_options` initialisation
and **synchronously reads the response** before returning. The expected
reply is `\x1b[<row>;<col>R`.

Captured initial output from the binary under the harness:

```text
9 bytes: "\x1b[>3u\x1b[6n"
         └────────┘ └─────┘
         kbd flags  DSR query
```

Only 9 bytes. No prompt rendered. The binary is parked waiting for the DSR
reply that never comes — the harness writes `\r` (Enter) but `\r` does not
match the CSI cursor-position reply grammar, so the parser keeps waiting.
Eventually crossterm gives up, returns an `io::Error`, and
`run_standalone_with_chrome` propagates it. The CLI maps the error to
`ABORTED_KIND` → exit code 1.

### Reproduction (probe)

```text
$ probe --height 100%       (no DSR responder)
initial (9 bytes): "\x1b[>3u\x1b[6n"
TIMEOUT

$ probe --height 100%       (DSR responder writes "\x1b[1;1R" before \r)
initial (9 bytes): "\x1b[>3u\x1b[6n"
got DSR query, responding with row=1 col=1
after DSR response (178 bytes): "...▶ alpha ... beta ... gamma ... Enter=Submit ..."
exited: Ok(Exited(Pid(N), 0))   ✅
```

A DSR responder is mandatory for any Inline-viewport test.

### Fix not yet applied

The keyboard-protocol tests (`cli/tests/keyboard_protocol.rs:55-82`) already
contain an `answer_cursor_position_request` helper that does this correctly.
The choose-cli PTY harness does not. Two paths forward:

**Option A — Add a DSR responder to the choose-cli harness.** Promote
`answer_cursor_position_request` to a shared helper (e.g. in a new
`cli/tests/common/pty.rs` module) and call it from `spawn_question` whenever
the spawned binary may use Inline viewport (`--height` is set).

**Option B — Restrict the choose-cli PTY tests to fullscreen.** Drop the
`--height 100%` test (or move it under `keyboard_protocol.rs` where the
DSR responder already exists), since the geometry-resolver math is already
covered by `lib::core::frame::tests::height_spec_percent_*`.

Option A is the right long-term fix because the harness will eventually
need to support `--height 50%` and other inline cases. Option B is the
expedient unblocker.

### Why this affects the new fix-plan path specifically

Same as Bug 1: before the fix-plan, `prepare_terminal` did not push the
`\x1b[>3u` sequence, so the initial output was zero bytes (not 9) and the
DSR query was the *only* thing in the buffer. The harness still had no
responder, so the test was always broken — but the back-pressure profile
was different and the failure presented as a different timeout.

### Recommendation

Adopt Option A. Concretely:

1. Extract `answer_cursor_position_request` from `keyboard_protocol.rs` into
   `cli/tests/common/pty.rs`.
2. Have `choose_cli::pty::spawn_question` call it when any of `args`
   contains `--height` or `-h <value>`.
3. Document in the harness module that any Inline-viewport prompt **must**
   answer the DSR query before sending input, or the binary will block
   during initialisation.

---

## Cross-cutting observations

- The fix-plan's runner changes (Phase 3) introduce two new
  CSI write points (`Push`/`Pop` keyboard enhancement flags) that the
  existing harness shape does not tolerate. Any future PTY-driven test
  must drain the master FD on every wait iteration.
- The `expectrl::Session::spawn(Command)` form is the correct entry point;
  the prior string-concatenation form (`spawn(&format!("{} \"…\"", bin))`)
  silently mis-quotes args and was the root cause of the earlier exit-2
  failures.
- The `process.wait()` (blocking) call is dangerous in any test that holds
  an open PTY master FD. Switch to deadline-bounded `process.status()`
  polling everywhere.

## Status

- [x] Bug 1 fix-applied in `cli/tests/choose_cli.rs::pty::wait_exit_code_within`
      (drain-while-poll + deadline-bounded status check)
- [x] Bug 1 verified: 3 of 4 PTY tests pass after fix
- [ ] Bug 2 fix pending: choose `Option A` (shared DSR responder helper)
      or `Option B` (drop the inline-viewport PTY test)
- [ ] Backport drain-while-poll pattern into `keyboard_protocol.rs` and
      `completions_shell.rs` (the harness shape there has the same risk)

---
ready: false
agent: codex/default
created: 2026-06-26T16:07:36
implemented: true
---

# Review 3

The latest iteration improves the evidence around the previous findings, but I
do not consider the feature production-ready. The remaining blockers are both
verification-level mismatches for user-observable terminal behavior.

## Findings

### High: Relaxed Ctrl/Alt+Shift hotkey behavior is still not verified at Level 3

Spec requirement: `CONTROL | SHIFT` plus a mapped Ctrl hotkey and `ALT | SHIFT`
plus a mapped Alt hotkey must activate the hotkey, `CONTROL | ALT` must match
neither map, and these semantics apply to both `choose-one` and `choose-many`.

Strongest verification present: Level 2 for `choose-one` only, plus Level 1
manufactured `KeyEvent` tests for both components. The new tests in
`biscuit-tui/cli/tests/real_terminal_render.rs:648` and
`biscuit-tui/cli/tests/real_terminal_render.rs:706` inject bytes through tmux /
WezTerm APIs. The Ctrl+Shift path explicitly uses `send_text` with a kitty CSI-u
escape sequence at `real_terminal_render.rs:726`, bypassing the OS keyboard and
the terminal emulator's physical keyboard encoder. There is no `level3_*` test
in the package.

Why this is a gap: the review instructions require Level 3 for requirements of
the form "when the user presses key X, Y happens." Level 2 proves the app can
decode injected bytes once delivered, but it does not prove a real Ctrl+Shift or
Alt+Shift physical keypress from a supported terminal emits the bytes this code
expects. The new real-terminal tests also only run `choose-one`; `choose-many`
remains at Level 1 for this behavior (`choose_many/tests.rs:82` and
`choose_many/tests.rs:101`).

Recommended fix: add `level3_*` OS-keyboard-injection coverage for the physical
Ctrl+Shift and Alt+Shift chords in at least one supported GUI terminal, and
cover both `choose-one` and `choose-many` behavior. Keep the Level 1 reducer
tests and Level 2 byte-decoder tests as useful lower-level contracts.

### High: Windows captured-stdout behavior has only opt-in/manual boundary coverage

Spec requirement: on Windows, `question` with stdout captured and a console
attached must render the prompt to the console while captured stdout receives
only the submitted value, with no TUI/ANSI bytes.

Strongest guaranteed verification present: compile-only for the new Windows
integration test, plus earlier Level 1 in-process handle tests. The added test
is `#[cfg(windows)]`, but it returns early unless
`BISCUIT_TUI_WINDOWS_CONSOLE_TEST=1` is set (`windows_captured_stdout.rs:53`).
The file documents that its "primary guaranteed-here deliverable" is compilation
and that it "never runs" without opt-in (`windows_captured_stdout.rs:34`). The
paired reproduction recipe is manual/opt-in (`windows-captured-stdout-repro.md:56`).

Why this is a gap: F2 is a user-observable terminal/console behavior at the CLI
process boundary. An opt-in manual test and recipe are useful, but they do not
make the normal test suite or CI matrix fail when the Windows captured-stdout
contract regresses. Without recorded CI evidence or a non-interactive Windows
runner path, this remains below the required verification level for production
readiness.

Recommended fix: add a Windows CI/manual-run artifact that is actually executed
for the feature, or build a non-interactive Windows console harness that can run
the spawned `question` process with stdout piped, console output attached, and a
deterministic submitted input. The assertion should fail if captured stdout
contains ESC/TUI bytes or omits the submitted value.

## Verification Level Matrix

| Requirement | Strongest observed verification | Result |
|---|---:|---|
| F1: failed terminal setup unwinds raw mode/alt-screen state | Level 1 fault-injected unit tests | Acceptable |
| F2: Windows captured stdout renders prompt to console and value to captured stream | Level 1 handle tests + compile-only/opt-in manual CLI boundary test | Gap, needs executed Windows boundary verification |
| F3: `try_new` typed errors for invalid table rows, including missing column ids | Level 1 public API unit tests | Acceptable |
| F4: strict `input-table` JSON validation | Level 1 CLI/parser tests | Acceptable |
| F5: relaxed Ctrl/Alt+Shift hotkey matching for `choose-one` and `choose-many` | Level 2 byte injection for `choose-one`; Level 1 for `choose-many`; no Level 3 | Gap, needs Level 3 physical-key verification |

## Notes

The review-2 `MissingColumnId` gap appears fixed: public
`InputTableState::try_new` now delegates under-length rows to `validate_row`,
and the targeted public test passes. The implementation also type-checks the
new Windows-only boundary test for `x86_64-pc-windows-gnu` from this macOS host.

## Checks Run

- `cargo test --color=never -p biscuit-tui input_table::table::tests::try_new_returns_missing_column_id_with_context --lib` — passed.
- `cargo test --color=never -p biscuit-tui-cli --test real_terminal_render --no-run` — passed.
- `cargo test --color=never -p biscuit-tui-cli --test windows_captured_stdout --no-run` — passed on macOS host profile.
- `cargo check --color=never -p biscuit-tui-cli --target x86_64-pc-windows-gnu --test windows_captured_stdout` — passed.

I did not run `just test-l2`, `just test-l3`, `just lint`, or the opt-in
Windows console reproduction during this review.

# F2 — Windows captured-stdout reproduction recipe

This recipe closes the High review finding "Windows captured-stdout behavior is
not verified at the required level" (see [`review-2.md`](./review-2.md)) by
documenting how to verify the F2 contract at the real `question` CLI boundary on
Windows.

**F2 contract (from [`spec.md`](./spec.md)):** when `question` runs with stdout
captured (the shell command-substitution shape) while a console stays attached,
the prompt must render to the console and the captured stream must receive ONLY
the submitted value — no TUI chrome, no ANSI/escape (`0x1b`) bytes.

In-process handle tests in `lib/src/core/standalone/tests.rs` prove the
`CONOUT$` / `SetStdHandle` strategy, but cannot prove the real process boundary.
This recipe is the human/CI-runner path for that boundary, paired with the
opt-in gated test `cli/tests/windows_captured_stdout.rs`.

## Why this is manual / opt-in

A console must be attached for the prompt to render into, and submitting a value
drives the crossterm event loop, which reads physical key events. Under nextest,
stderr is captured, which trips the standalone headless guard (both stdout and
stderr non-tty → `no interactive terminal available`) and prevents the real
shape. So the spawn in `windows_captured_stdout.rs` is gated behind both
`cfg(windows)` and the env var `BISCUIT_TUI_WINDOWS_CONSOLE_TEST=1`, and skips
cleanly otherwise.

## A. Reproduce the captured-stdout shape interactively (PowerShell)

Run from a real Windows console (Windows Terminal, conhost, or PowerShell host)
where `question` is on `PATH` (`just install`, or use the built binary path).
PowerShell command substitution `$( ... )` captures stdout while leaving the
console attached for the prompt:

```powershell
$v = $(question choose-one Red Green Blue)
# The prompt renders to the console. Use arrow keys / fuzzy filter to highlight
# an option, then press Enter to submit.
$v                       # -> exactly one of: Red | Green | Blue
$v -match "`e"           # -> False  (no ESC / 0x1b byte captured)
[int[]][char[]]$v        # inspect bytes: no 27 (0x1b) present
```

cmd.exe analog (capture via a `for /f` loop):

```cmd
for /f "delims=" %v in ('question choose-one Red Green Blue') do set RESULT=%v
echo %RESULT%
```

**Expected PASS observation:** the prompt renders to the console; after Enter,
the captured variable holds ONLY the submitted option value (`Red`, `Green`, or
`Blue`) with no escape sequences. If TUI/ANSI bytes leak into the captured
variable, F2 is violated.

## B. Run the gated boundary test on a Windows runner

The gated test spawns `question` with stdout piped and stderr inherited (console
attached), then asserts the captured stream has no `0x1b` byte and holds exactly
the submitted value. It requires an attached console and a live Enter keypress,
so set the opt-in env var and run it with the package's nextest:

```powershell
$env:BISCUIT_TUI_WINDOWS_CONSOLE_TEST = "1"
cargo nextest run -p biscuit-tui-cli --no-capture `
  -E 'test(captured_stdout_receives_only_value_no_tui_bytes)'
```

`--no-capture` is required so the prompt reaches the console and your keystroke
reaches the prompt. Highlight an option and press Enter when the prompt appears.

**Expected PASS observation:** the test prints the prompt to the console, you
submit a value, and the assertions pass (no ESC bytes; captured value is one of
`Red | Green | Blue`).

Without the env var (the normal CI/local shape, including macOS/Linux where the
file is not even compiled), the test returns early and passes as a clean skip.

## C. CI matrix linkage

Cross-compilation alone is verified continuously: this branch was checked with
`cargo check -p biscuit-tui-cli --target x86_64-pc-windows-gnu` from a macOS
host, proving the gated test and crate compile for Windows. The full
`windows-latest` runner coverage belongs to the CI matrix described in
[`features/2026-06-07-matrix-testing/spec.md`](../../../features/2026-06-07-matrix-testing/spec.md);
the interactive parts of this recipe (A and B) are the operator/manual path that
a maintainer runs on a real Windows console when validating an F2-affecting
change.

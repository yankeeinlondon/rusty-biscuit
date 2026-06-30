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
This recipe pairs with the executable boundary test
`cli/tests/windows_captured_stdout.rs`: section A is the human PowerShell sanity
check, sections B and C are the automated Windows-host gates.

## Why a console is still required (but no human keypress is)

A console must be attached for the prompt to render into. Under nextest, stderr
is captured, which trips the standalone headless guard (both stdout and stderr
non-tty → `no interactive terminal available`) and prevents the real shape — so
the test needs a real attached console. The submit keystroke, however, is no
longer manual: the test injects a deterministic **Enter** into the console input
buffer with `WriteConsoleInputW`, so it runs unattended. Following the claudine
precedent, `windows_captured_stdout.rs` is `#[cfg(windows)]` + `#[ignore]`d (not
early-returning): it COMPILES on every Windows target and RUNS only when invoked
with `--ignored` on a real Windows console.

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

## B. Run the executable boundary test on a Windows host

The boundary test spawns `question` with stdout piped and stderr inherited
(console attached), injects a deterministic Enter via `WriteConsoleInputW` to
submit the default-highlighted first option, then asserts the captured stream has
no `0x1b` byte and holds exactly the submitted value. It requires an attached
console but **no human keypress**. Run it on a Windows host with:

```powershell
just test-windows-captured-stdout
```

or directly (the `just` recipe wraps this exact invocation):

```powershell
cargo test -p biscuit-tui-cli --test windows_captured_stdout `
  -- --ignored captured_stdout_receives_only_value_no_tui_bytes --nocapture
```

`--ignored` selects the `#[ignore]`d test; `--nocapture` lets the prompt reach
the console so the injected Enter lands on a live event loop.

**Expected PASS observation:** the prompt renders to the console, the injected
Enter submits the highlighted option, and the assertions pass (no ESC bytes;
captured value is one of `Red | Green | Blue`).

On macOS/Linux the file is not compiled (`#![cfg(windows)]`); on a Windows run
that does not pass `--ignored`, the test is skipped (still listed, not run).

## C. CI gate and verification status

The path-filtered workflow
[`.github/workflows/biscuit-tui-windows-captured-stdout.yml`](../../../.github/workflows/biscuit-tui-windows-captured-stdout.yml)
runs this test on `windows-latest` (`shell: cmd`, attached console) for changes
under `biscuit-tui/**`, `just/**`, `Cargo.toml`, `Cargo.lock`, or the workflow
file, and is also `workflow_dispatch`-runnable.

From the macOS dev host the maximum honest verification is cross-compilation:
`cargo check -p biscuit-tui-cli --target x86_64-pc-windows-gnu --test windows_captured_stdout`,
which proves the rewritten test and its `windows` dev-dependency compile for
Windows. Until a green run is recorded on the Windows workflow / a Windows host,
the path is **compile-checked, not yet runtime-confirmed**. The interactive
section A remains the operator sanity check a maintainer can run on a real
Windows console.

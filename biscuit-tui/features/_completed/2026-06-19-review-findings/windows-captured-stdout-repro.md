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

## Why a console is required — and why the test PROVES it has one

A console must be attached for the prompt to render into. Under a bare
`cargo test` on GitHub Actions (and under nextest), the test process's
stdout/stderr are pipes, not a console — so a spawned `question` would inherit a
non-console stderr and take the headless bail path (both stdout and stderr
non-tty → `no interactive terminal available`), and a green run would prove
nothing. `--nocapture` does not change this: it disables only the Rust harness
buffer, not the Actions pipe.

So the test does not *assume* a console — it **establishes and proves** one. It
calls `AllocConsole` (a benign no-op when one already exists), rewires the
process std handles onto `CONOUT$`/`CONIN$` via `CreateFileW` + `SetStdHandle`
so the inherited child stderr/stdin are real console buffers, and then asserts
the precondition (`stderr.is_terminal()` AND `CONOUT$` openable), **panicking
with a specific diagnostic** if either fails. Because the test is `#[ignore]`d
and run with `--ignored`, that panic is a visible CI failure. The submit
keystroke is also unattended: a deterministic **Enter** is injected into the
console input buffer with `WriteConsoleInputW`. Following the claudine
precedent, `windows_captured_stdout.rs` is `#[cfg(windows)]` + `#[ignore]`d (not
early-returning): it COMPILES on every Windows target and RUNS only when invoked
with `--ignored` on a Windows host.

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

The boundary test first **self-establishes and proves** the console
precondition: it attaches a console (`AllocConsole`), rewires the std handles
onto `CONOUT$`/`CONIN$`, and asserts `stderr.is_terminal()` AND `CONOUT$` is
openable — panicking with a clear diagnostic (and thus failing the run) if it
cannot. It then prints a `F2 precondition HELD: ...` line so the log records
that the precondition genuinely held. Only then does it spawn `question` with
stdout piped and stderr/stdin inherited (now real console buffers), inject a
deterministic Enter via `WriteConsoleInputW` to submit the default-highlighted
first option, and assert the captured stream has no `0x1b` byte and holds
exactly the submitted value — all with **no human keypress**. Run it on a
Windows host with:

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

**Expected PASS observation:** the log shows `F2 precondition HELD: ...`, the
prompt renders to the console, the injected Enter submits the highlighted
option, and the assertions pass (no ESC bytes; captured value is one of
`Red | Green | Blue`). A `F2 precondition NOT met` panic instead means the run
could not establish a console and therefore did NOT verify the contract — treat
it as a failure, not a skip.

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
the path is **compile-checked, not yet runtime-confirmed**.

A green Windows-host run, however, IS now reliable evidence — not merely "it
compiled". Because the test attaches its own console, proves
`stderr.is_terminal()` + `CONOUT$` usability before spawning, and fails loudly
if it cannot, a pass means the attached-console captured-stdout contract was
genuinely exercised (the Actions log carries the `F2 precondition HELD` line as
the record). A run that could not establish a console fails instead of silently
passing, so a non-console CI environment can no longer masquerade as a pass. The
interactive section A remains the operator sanity check a maintainer can run on a
real Windows console.

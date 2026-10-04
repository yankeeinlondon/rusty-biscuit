# Windows: Consoles, Terminals, and PowerShell

Load before writing a Windows Level 2 test, driving a console or
pseudoconsole, using WezTerm on `build-win`, or capturing a native command's
output from PowerShell. Path spelling is in [windows-paths.md](windows-paths.md);
processes and the environment in [windows.md](windows.md).

## WezTerm on `build-win`

- `~/.wezterm.lua` there `dofile`s the shared config from `X:` and then a UNC
  path. `X:` is a per-logon mapped drive that no SSH, nextest, or mux-server
  process has, so every config *evaluation* from those contexts blocks for
  the SMB connect timeout — measured at 21.2–21.3 s, deterministic, on each
  `wezterm cli spawn`; Windows negative-caches the failure for well under a
  minute, so a second spawn seconds later is 0.1 s. `wezterm cli list` does
  not evaluate the config at all, which is why the harness's availability
  gate passes and only the spawn times out (15 s `SPAWN_TIMEOUT`).
  `biscuit-test-harness` now runs every `wezterm` client under an empty
  `WEZTERM_CONFIG_FILE` unless the caller set one; the mux server keeps its
  own config. The empty config is a private, uniquely created temporary file
  retained until that client exits and then removed; creation/write errors
  propagate rather than selecting an existing shared pathname. Do not "fix"
  this by raising the timeout.
- A headless `wezterm-mux-server` reachable through `WEZTERM_UNIX_SOCKET`
  (`C:\Users\ken\.local\share\wezterm\sock`) is all the Level 2 tier needs
  there; the twin in `level2_windows_provided_partial_file_capture.rs` passes
  against it in ~4 s. Never blanket-stop mux servers on that host — one of
  them may be carrying the session the developer is working in.
- `cross-check --os windows` runs in an SSH session with no
  `WEZTERM_UNIX_SOCKET`, so a WezTerm Level 2 test **skips there and nextest
  prints PASS in ~0.02 s**. Read the duration, or set
  `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` so a missing backend fails.

## PowerShell

- **A PowerShell function's output stream is not its return value.** Every
  native command inside a function writes its stdout into that function's
  output, so `$code = Invoke-Thing` binds an *array* whose first element is
  some program's chatter, and `exit $code` reports that. Symptom: a remote run
  whose tier exited 1 is summarized as a pass (`cross-check --os windows`,
  build-win-native, 2026-09-14). Assign a `$script:`-scoped variable at every
  failure point and call the function as `Invoke-Thing | Out-Host`, which keeps
  the log on the console without binding it. `$LASTEXITCODE` after each native
  command is still the right check — the bug is in how the result leaves the
  function, not in how it is read.
- **Windows PowerShell 5.1 re-encodes a native command's captured stdout** with
  `[Console]::OutputEncoding`, which is the OEM code page (IBM437 on
  build-win-native). A UTF-8 `café-ü日` arrived as `caf├⌐-├╝µùÑ`. A wrapper
  that captures a Rust binary's output (`$out = & tool.exe`) must set
  `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)` around
  the call and restore it in `finally`. stdin and stderr stay TTYs while
  stdout is captured. Measured 2026-09-24 under `ssh -tt`
  (`worktree/fixes/2026-09-24-ux-improvements/spike-s3.md`).

## A real console without a window: ConPTY

Measured on `build-win-native` on 2026-09-25 through `just cross-check`
(`worktree/cli/tests/level2_powershell_remove.rs`).

- **A pseudoconsole is a Level 2 console that needs no backend.** `xpty`
  (already in `Cargo.lock` for `unchained-ai`) opens ConPTY from inside the SSH
  session's nextest process, with no window and no focus change. Interactive
  Windows PowerShell 5.1 runs in it with PSReadLine, keystrokes are plain bytes
  (`\r` for Enter), and a Rust prompt (inquire) sees a terminal. `xpty` leaves
  out `PSEUDOCONSOLE_INHERIT_CURSOR`, so conhost sends no DSR that the test
  would have to answer.
- **The output stream is a repaint, not the text.** PSReadLine's echo of one
  typed line arrived as `. 'C:\…\s. 'C:\…\sc. 'C:\…\sce…`. Wait for a single
  word in the stream at most; assert on the console's own screen buffer,
  which the session can dump with `$Host.UI.RawUI.GetBufferContents` (the
  Windows counterpart of `tmux capture-pane`).
- **Index that buffer with `GetValue`.** In a script, `$cells[$y, $x]` on the
  `BufferCell[,]` it returns failed to parse ("Missing ']' after array index
  expression"); `$cells.GetValue($y, $x).Character` works.
- **The screen buffer hard-wraps at the column width, mid-word.** A long
  error line (a temp path) split as `…': P` / `ermission denied`. Join a
  full-width row to the next without a space before matching a phrase
  (`unwrapped` in that test, 2026-09-25).
- **`build-win-native` checks files out with CRLF** (`core.autocrlf`), so a
  test that wrote `"guide\n"`, committed it, and read the file back from a
  `git worktree add` checkout got `"guide\r\n"`. Normalize line endings
  before comparing checked-out content (2026-09-25).
- **Ctrl+C typed into a pseudoconsole is ignored unless the child inherits
  Ctrl+C processing.** Writing ETX (`\x03`) to the ConPTY input makes conhost
  raise `CTRL_C_EVENT`, as a terminal key press does, but "ignore Ctrl+C"
  (`SetConsoleCtrlHandler(NULL, TRUE)`) is a process attribute that children
  inherit, and a process chain under sshd and nextest can carry it. The child
  then never sees the event, its registered handler is never called, and it
  runs to completion; `CTRL_BREAK_EVENT` is unaffected, which is why the
  group-targeted Ctrl+Break tests pass on the same host. Call
  `SetConsoleCtrlHandler(None, FALSE)` in the test process before spawning,
  as a terminal launching a shell effectively does
  (`claudine/cli/tests/l1/lifecycle_message_drain_console_windows.rs`,
  `build-win-native`, 2026-09-27).
- **`xpty::CommandBuilder` starts from the parent's environment**, like
  `std::process::Command`. When copying a `Command` built by a fixture that
  inherits and only overrides, apply its `get_envs()` on top without
  `env_clear()`: clearing dropped `PATHEXT`, so `which("claude")` no longer
  matched `claude.cmd` (2026-09-27).
- The process's working directory is the one passed to `CommandBuilder::cwd`,
  so this is also how a test reproduces "a window launched inside the
  directory" for the current-directory lock in [windows.md](windows.md).

## Attaching a console inside a nextest process

`biscuit-tui/cli/tests/level2/windows_captured_stdout.rs` is ordinary `windows-latest`
**L1** evidence inside `biscuit-tui-cli`'s own cell — not an `#[ignore]`d test
behind a hand-invoked recipe or workflow. It compiles into the `level2` binary
because it needs `terminal-tests`, but its test name has no tier marker, so the
L1 filter selects it there (2026-09-22-consolidated-test-binaries-wave-2, R4).
Everything below was measured on
`build-win-native` at the CI thread count (`--test-threads 4`), 2026-09-14.

- **Process-wide handle rewiring is safe only because nextest gives each test
  its own process.** `AllocConsole` + `SetStdHandle` mutate process state; under
  `cargo test`'s shared harness they would corrupt every sibling test in the
  binary. Say so in the test's `//!` docs — it is the reason the tier is L1
  rather than a serialized L3.
- **`AllocConsole` returning `ERROR_ACCESS_DENIED` (0x80070005) is the normal
  path, not a failure.** A console is usually already present, and the API
  reports that as access denied. Treat "already present or failed" as one state
  and assert the *precondition you actually need* — `stderr.is_terminal()` and
  `CONOUT$` openable — instead of the call's return value.
- **Redirecting a std handle to `CONOUT$` makes everything printed afterwards
  invisible to nextest.** The line goes to the attached console, not to the
  harness pipe. Two consequences, both found the hard way: a success diagnostic
  printed after the redirect never reaches the log (`grep -c` returns 0), and —
  worse — an assertion that panics *after* the redirect leaves nextest reporting
  `FAIL` with an empty message. Redirect only the handle the contract requires
  (stderr here; the stdout redirect was deleted as unnecessary), capture the
  original handle before redirecting, and restore it the moment the child exits
  so later failures are reported through the pipe.
- **The console input buffer queues injected records**, so a written input
  record survives the child not having started its event loop yet. The 750 ms /
  250 ms fixed sleeps this test shipped with were covering a measured
  requirement of **0 ms**: the test's real work is ~45 ms and the sleep *was*
  its 0.78 s runtime. A bounded readiness loop — 2 s deadline, 25 ms poll,
  re-inject at 500 ms — replaced them; no passing run has needed the second
  injection. Keep the loop anyway: it converts a timing assumption into an
  assertion that fails loudly at its own deadline rather than at nextest's 90 s
  `ci` termination ceiling, and it kills and reaps the child so the cell reports
  `FAIL` rather than `LEAK`.
- Six consecutive clean runs, zero flakes (392 run / 392 passed / 7 skipped).
  Runs that died in `git fetch` with `ssh: connect to host github.com port 22`
  are a build-host network fault, not a test result — exclude them rather than
  counting them as failures.

# Windows: Consoles, Terminals, and PowerShell

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Load before writing a Windows Level 2 test, driving a console or
pseudoconsole, using WezTerm on `build-win`, capturing a native command's
output from PowerShell, or writing escape sequences from a Windows SSH session. Path spelling is in [windows-paths.md](windows-paths.md);
processes and the environment in [windows.md](windows.md).

## WezTerm on `build-win`

- **A spawn times out while `wezterm cli list` works.** That host's
  `~/.wezterm.lua` loads config from a per-logon mapped drive that SSH,
  nextest, and mux-server processes lack, so every config *evaluation* waits
  out the SMB connect timeout; `list` evaluates no config. The harness runs
  clients with `--skip-config` (see the `biscuit-test-harness` skill); do not
  raise the timeout.
- A headless `wezterm-mux-server` reachable through `WEZTERM_UNIX_SOCKET` is
  all Level 2 needs there. Never blanket-stop mux servers on that host; one may
  carry the developer's session.
- **`cross-check --os windows` has no `WEZTERM_UNIX_SOCKET`,** so a WezTerm
  Level 2 test skips and nextest prints a near-instant PASS. Read the duration,
  or set `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` so a missing backend fails.

## PowerShell

- **A function's output stream is not its return value.** Every native
  command's stdout inside a function joins its output, so
  `$code = Invoke-Thing` binds an array led by some program's chatter, and
  `exit $code` can report a failed run as a pass. Set a `$script:`-scoped
  result at each failure point and call `Invoke-Thing | Out-Host`.
  `$LASTEXITCODE` after each native command is still the right check.
- **Windows PowerShell 5.1 re-encodes captured native stdout** with
  `[Console]::OutputEncoding` (the OEM code page), turning UTF-8 into mojibake
  (`café` becomes `caf├⌐`). A wrapper that captures a Rust binary's output
  must set `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)`
  around the call and restore it in `finally`. stdin and stderr stay TTYs
  while stdout is captured.

## A real console without a window: ConPTY

Reference tests: `worktree/cli/tests/level2_powershell_remove.rs` and
`claudine/cli/tests/l1/lifecycle_message_drain_console_windows.rs`.

- **A pseudoconsole is a Level 2 console that needs no backend.** `xpty` opens
  ConPTY from inside an SSH session's nextest process with no window or focus
  change. Interactive PowerShell runs in it with PSReadLine, keystrokes are
  plain bytes (`\r` for Enter), and a Rust prompt sees a terminal. `xpty` omits
  `PSEUDOCONSOLE_INHERIT_CURSOR`, so conhost sends no cursor DSR to answer.
- **Answer the Device Attributes query or the child starts about 3 s late.**
  ConPTY writes `ESC [ c` first and holds the child until a reply arrives or
  the wait expires. Reply from the output-reader thread with Windows
  Terminal's answer (`ESC [ ? 61;4;6;7;14;21;22;23;24;28;32;42 c`); a bare
  `ESC [ ? 1;0 c` does not release it. Every test that reads an `xpty`
  pseudoconsole should do this.
- **The output stream is a repaint, not the text.** PSReadLine re-echoes a
  typed line incrementally. Wait for a single word in the stream at most;
  assert on the screen buffer, dumped in-session with
  `$Host.UI.RawUI.GetBufferContents` (the counterpart of `tmux capture-pane`).
  - Index it with `$cells.GetValue($y, $x).Character`; `$cells[$y, $x]` fails
    to parse in a script.
  - It hard-wraps at the column width mid-word; join a full-width row to the
    next without a space before matching a phrase.
- **Ctrl+C typed as ETX is ignored if the child inherits "ignore Ctrl+C".**
  `SetConsoleCtrlHandler(NULL, TRUE)` is inherited, and an sshd/nextest chain
  can carry it, so the child never sees `CTRL_C_EVENT` and runs to completion;
  `CTRL_BREAK_EVENT` is unaffected. Call `SetConsoleCtrlHandler(None, FALSE)`
  in the test process before spawning, as a terminal does.
- **`xpty::CommandBuilder` inherits the parent environment** like
  `std::process::Command`. When copying a fixture's `Command`, apply its
  `get_envs()` on top without `env_clear()`; clearing drops `PATHEXT`, and
  `which("claude")` stops matching `claude.cmd`.
- **Checked-out text may be CRLF** on a host with `core.autocrlf`, so content
  read back from a `git worktree add` checkout gains `\r\n`. Normalize line
  endings before comparing.
- **Use `xpty`, not `expectrl`, when the test plays the terminal for a
  child's queries.** Measured 2026-10-21 on `build-win-native` (build 26300,
  inbox console host) with `biscuit-terminal`'s `discovery_probe`. Under
  `xpty`'s pseudoconsole, the child's OSC 10/11 queries, the Kitty APC query,
  and its DA1 arrived verbatim in the output stream, and the replies written
  to the input pipe reached the child's `ReadConsoleInputW` as key-down
  records carrying the reply's characters. Under `expectrl`'s pseudoconsole
  (the `conpty` crate, `CreatePseudoConsole` flags `0`), none of those
  queries reached the stream, and the console host answered DA1 itself
  without Sixel's attribute `4`. Unlike the SSH server's console, this one
  passes APC through. The reference is
  `biscuit-terminal/lib/tests/level2/level2_windows_console_query.rs`.
- **Read a child's printed results from a file, not the stream:**
  `cmd /d /c <exe> > out.txt 2>&1` with `cwd` set to a temp directory. A
  program that opens `CONIN$`/`CONOUT$` by name still reaches the console.
  The child's own `IsTerminal` check then reports false.
- **`WriteConsoleInputW` puts typed keys back.** Key records consumed by
  `ReadConsoleInputW` and written back, key-down plus key-up, are read by a
  later line-mode `ReadConsoleW` (Rust's `stdin().read_line`) as typed text.
  They are appended, so a key typed after the last read lands ahead of
  them.
- `CommandBuilder::cwd` sets the child's working directory, so a ConPTY child
  reproduces "a window launched inside the directory" for the
  current-directory lock in [windows.md](windows.md#current-directory-locks).

## Attaching a console inside a nextest process

`biscuit-tui/cli/tests/level2/windows_captured_stdout.rs` is ordinary
`windows-latest` **L1** evidence in `biscuit-tui-cli`'s cell: it lives in the
`level2` binary because it needs `terminal-tests`, but its name carries no tier
marker, so the L1 filter selects it.

- **Process-wide handle rewiring is safe only because nextest gives each test
  its own process.** `AllocConsole` and `SetStdHandle` mutate process state and
  would corrupt sibling tests under `cargo test`. Say so in the test's `//!`
  docs; it is why the tier is L1 rather than a serialized L3.
- **`AllocConsole` returning `ERROR_ACCESS_DENIED` (`0x80070005`) is normal**:
  a console is usually already present. Assert the precondition you need
  (`stderr.is_terminal()`, `CONOUT$` openable), not the call's result.
- **Output after redirecting a std handle to `CONOUT$` is invisible to
  nextest.** Diagnostics vanish, and a panic after the redirect makes nextest
  report `FAIL` with an empty message. Redirect only the handle the contract
  needs, save the original first, and restore it as soon as the child exits.
- **The console input buffer queues injected records,** so input written
  before the child's event loop starts is not lost and fixed sleeps are
  unnecessary. Use a bounded readiness loop (deadline, short poll, one
  re-inject) that fails at its own deadline rather than nextest's termination
  ceiling, and kill and reap the child so the cell reports `FAIL`, not `LEAK`.
- A run that dies in `git fetch` with `ssh: connect to host github.com port
  22` is a build-host network fault, not a test result; exclude it.

## Escape sequences through the Windows OpenSSH server

Measured 2026-10-20 on `build-win-native`: the bytes a Windows process wrote
inside an `ssh -tt` session were captured at the client.

- **The session's console is the system `C:\WINDOWS\system32\conhost.exe`,**
  and it filters by sequence type, not by content:

  | Sent by the Windows process | Arrived at the client |
  | --- | --- |
  | APC (Kitty graphics) | **Removed** |
  | DCS (Sixel, and anything wrapped for tmux passthrough) | **Removed** |
  | OSC (iTerm2 `OSC 1337;File=`, BEL or ST terminated) | Intact, even at 2 MB, byte count exact |
  | OSC 8 hyperlink | Passes; conhost adds an `id=` parameter |

- **WezTerm's multiplexer works on the same host** because WezTerm ships its
  own `conpty.dll` and `OpenConsole.exe` (`C:\Program Files\WezTerm\`) and
  never uses the system console host. A WezTerm-mux result is not evidence for
  a plain SSH session.
- **Consequence in code:** `biscuit-terminal`'s graphics route never selects
  Kitty or Sixel when `SSH_CONNECTION` is set on native Windows (a forced
  `BISCUIT_GRAPHICS=kitty`/`sixel` is reported as a blocked transport); it
  selects the iTerm2 format only when the client terminal renders it. See
  `biscuit-terminal/docs/discovery/graphics-routes.md`.
- A test that asserts on OSC 8 output read back through this path must accept
  the added `id=`.

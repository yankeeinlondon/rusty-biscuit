# Harness API

Part of the `biscuit-test-harness` skill; [SKILL.md](SKILL.md) is the index.

## The `TerminalHarness` contract

| Method | Use |
|---|---|
| `spawn_shell()` | Default. A shell with `PATH` assembled, color forced, prompt waited for. |
| `spawn_program(prog, args)` | Only when the test must own the process tree: no shell, no `PATH`, no color forcing. |
| `send_text(&[u8])` | Raw bytes to the pane (escape sequences, Kitty keyboard protocol). |
| `send_command_with_env(cmd, env)` | One command line with scoped `KEY=value` prefixes, then a fixed `settle()` sleep. |
| `capture()` | `CapturedFrame { raw, plain }`: assert styling on `raw`, visible text on `plain`. |

Prefer `send_command_settled(&mut harness, cmd, env)` to `send_command_with_env`
followed by a capture: it waits until the pane shows the command instead of
sleeping a fixed `settle()`. When the command's output must be found by a
marker, use `run_to_sentinel`: it waits on a temp file the line touches, so
it does not poll captures while the command runs. Every capture is a terminal
client call, and parallel test processes queue for those (WezTerm admits two
at a time), so polling one pane slows every other test.

## The pane shell

Every backend runs an outer `-l` login shell, which assembles `PATH`, and then
`exec`s an interactive shell with its rc files suppressed (`bash --norc -i`,
`zsh -f -i`, `unset ENV`). Interactive rc files are where prompt and history
tools install themselves, and their pickers swallow sent input. On Windows
bash also runs `--noediting`, because readline under ConPTY makes typing
superlinear in line length. A test that needs readline (Tab completion)
spawns its own shell with `spawn_program`.

- Pane `PATH` is login-profile only. Pass an absolute path or an explicit
  `PATH` env pair rather than relying on an rc edit.
- The prompt is the shell's stock `PS1`; never match a literal it contains.
- The invocation is built once in `lib.rs` (`login_shell_script`,
  `login_shell_argv`, `login_shell_command_line`) so backends cannot drift.
  tmux needs `-e` to set pane environment; an env var on the `tmux` client
  does not reach a session on an already-running server.
- The tmux server is the harness's own (`tmux -L biscuit-test-harness -f
  /dev/null`): no developer config, same defaults as CI. A test's own `tmux`
  call against a harness session uses `tmux::tmux_command()`.

## Reading a capture

- **Bracket the command with markers** printed by `printf` so the echo never
  spells them, and read only between them. tmux's bash shortens a long echoed
  command line instead of wrapping it, so the echo is not a reliable anchor.
  Example: `biscuit-terminal/cli/tests/common/output_region.rs`.
- **A shared pane still holds earlier output**; a `clear` is not proof. Scope
  every assertion to the command under test. Search for the *last* sentinel
  row, and match it after `strip_ansi`: in a raw capture the row can lead
  with an attribute reset carried over from the output above it.
- **WezTerm re-emits only changed attributes too**: a code block's padding
  row arrives as a bare `\e[K` under a background set rows earlier, so a
  per-row "is it blank" check must replay SGR across rows.
- **OSC 8 parameters (`line=3:column=1`) come back in hash-map order** from
  WezTerm; the harness sorts them in every capture so frames compare equal.
- **tmux re-emits only changed attributes**, carried across rows, so read a
  cell's style by replaying SGR from the capture's start. Example:
  `tree-hugger/cli/tests/support/terminal_report.rs`.

## Helpers worth knowing

| Item | Purpose |
|---|---|
| `shell_path(&Path)` / `shell_quote(&str)` | One pane-shell word. Every path in a typed command goes through `shell_path`: it quotes it and, on Windows, spells it with `/`, which Cygwin bash needs for a command word. |
| `CLEAR_SCREEN` | Clears screen and scrollback with the `printf` builtin. Prefix it to the command line rather than running `clear` as its own round trip. |
| `run_to_sentinel(harness, cmd, env, sentinel, timeout)` | Runs `cmd`, prints `sentinel` on its own row, returns a settled capture showing it. |
| `strip_ansi(&str)` | ECMA-48-aware stripper for CSI, OSC, SGR, and charset designations. |
| `skip_with_reason(&str)` | Prints `skipping: requires <X>` and returns `true`. |
| `wait_for_prompt(&mut harness)` | Polls until the shell prompt appears. |
| `apply_color_forcing_env(&mut Command)` | Forces color env and clears `NO_COLOR`. |
| `bin_exe!("name")` | Workspace binary path resolved at run time, so archived CI runs find it. |
| `run_with_timeout` / `run_with_stdin_timeout` | Wall-clock timeout with deadlock-safe pipe draining. |
| `cargo_bin_dir(name)`, `detect_shell()` | `PATH` augmentation and POSIX shell choice. |

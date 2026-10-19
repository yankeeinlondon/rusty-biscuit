# Availability and Backend Reachability

Part of the `biscuit-test-harness` skill; [SKILL.md](SKILL.md) is the index.

Every L2/L3 test checks its harness's `available()` and skips cleanly when it
returns `false`. Gate through `require_level!` with a `Backend` so
`BISCUIT_TEST_REQUIRED_BACKENDS` can require and prove that backend; the gating
rules and the backend-proof bracket are in the `rust-testing` skill's
`tiers-and-gating.md`.

## What `available()` requires

| Harness | Requires |
|---|---|
| `TmuxHarness` | `tmux` on `PATH`. |
| `WezTermHarness` | `wezterm` on `PATH`, `WEZTERM_UNIX_SOCKET` set, and `wezterm cli list` answering promptly. A slow answer counts only when `wezterm` is a required backend. |
| `KittyHarness` | `kitty` on `PATH` and `KITTY_LISTEN_ON` set. |
| `KittyInstance` (macOS) | `can_launch()`: `kitty`, `open`, `screencapture`. No host Kitty needed. |
| `AppleTerminalHarness` | macOS, not CI, and `osascript` can reach Terminal.app. |
| `cliclick` (L3, macOS) | `cliclick` on `PATH` only; also gate on `cliclick::accessibility_trusted()`, which checks platform and permission. |
| `xdotool` (L3, Linux) | Linux, `xdotool`, and `DISPLAY` set; Wayland skips. |
| `win_input` (L3, Windows) | Windows and a working `powershell`. |
| `XvfbKitty` (L3, Linux) | `can_launch()`: `Xvfb` and `kitty` on `PATH`. |

## The host-terminal trap

`WEZTERM_UNIX_SOCKET` and `KITTY_LISTEN_ON` are exported only to processes the
terminal itself launched. Run the suite from WezTerm and the Kitty tests skip;
run it from Kitty and the WezTerm tests skip. tmux is the only L2 backend
immune to this.

To reach the other terminal:

- **WezTerm:** its mux server outlives windows, so point at its socket:
  `export WEZTERM_UNIX_SOCKET="$(ls "$HOME/.local/share/wezterm/gui-sock-"* | head -1)"`.
- **Kitty, per test (macOS):** `KittyInstance::launch(cols, lines)` starts a
  private, unfocused Kitty at that cell size, and `screenshot()` captures it.
  Kitty draws nothing while its window is hidden or covered, so a pixel test
  must fail or skip on an empty capture, never pass. Detail:
  `biscuit-test-harness/README.md` ("A private Kitty per test"); example:
  `worktree/cli/tests/level2_graph_in_kitty.rs`.
- **Kitty, host instance:** start one with remote control
  (`kitty -o allow_remote_control=yes --listen-on unix:/tmp/kitty-l2 --start-as=minimized`)
  and set `KITTY_LISTEN_ON` to that address.

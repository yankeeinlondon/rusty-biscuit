---
name: biscuit-test-harness
description: |
  Shared real-terminal test harness crate (L2/L3): backend selection
  (tmux/WezTerm/Kitty/Apple Terminal/OS key injectors), the
  `TerminalHarness` API, `available()`, `SpawnVisibility`, sharing panes,
  parallel L2 runs, and focus safety. Load when picking a backend or using
  the harness API; for tiers, gating, and assertion patterns against captured
  frames, load the `rust-testing` skill instead.
---

# biscuit-test-harness

`biscuit-test-harness/` is a `publish = false` dev-dependency that runs a
binary inside a *real* terminal emulator and captures what it rendered. This
skill covers the harness itself. Tiers, `require_level!` gating, and
`BISCUIT_TEST_REQUIRED_BACKENDS` belong to the `rust-testing` skill
(`tiers-and-gating.md`); assertion patterns against `frame.raw` are in its
`wezterm-harness-pitfalls.md`.

## Pick a backend

```text
Must the test prove what the terminal's *input encoder* emits for a key?
├── YES → Level 3: OS key injection (level3.md).
└── NO  → Level 2:
    ├── default                                  → TmuxHarness
    ├── graphics, inline images, Kitty keyboard  → KittyHarness or WezTermHarness
    ├── truecolor/OSC8 fidelity, pane_size()     → WezTermHarness
    └── degradation on a weak terminal           → AppleTerminalHarness
```

| Harness | Headless | Can verify | Cannot verify |
|---|---|---|---|
| `TmuxHarness` | yes | SGR, OSC8, text; chords via `send_key` | Kitty keyboard protocol, graphics, truecolor nuance |
| `WezTermHarness` | needs a running WezTerm | SGR, OSC8, images, truecolor, `pane_size()` | anything when WezTerm is not reachable |
| `KittyHarness` / `KittyInstance` | needs Kitty | SGR, OSC8, Kitty graphics, `pane_cols()` | anything when not pointed at a Kitty |
| `AppleTerminalHarness` | macOS GUI | low capability: no images, no OSC8 | byte-level negatives (`raw == plain`) |

Prefer tmux: it is headless, on `PATH` everywhere, and immune to the
host-terminal trap below.

## Rules that fail silently when broken

1. **Check `available()` first and skip cleanly**; never `#[ignore]`. A skip
   because a terminal is not the host is usually not a defect.
   → [availability.md](availability.md)
2. **WezTerm and Kitty are reachable only from processes they launched**
   (`WEZTERM_UNIX_SOCKET`, `KITTY_LISTEN_ON`). Run from the other terminal and
   its tests skip. → [availability.md](availability.md#the-host-terminal-trap)
3. **The pane shell reads login files, not rc files.** `PATH` edits in
   `~/.bashrc` don't reach it, and the prompt is the stock one.
   → [harness-api.md](harness-api.md#the-pane-shell)
4. **Find a command's output by printed markers, never by its echo** or the
   prompt. → [harness-api.md](harness-api.md#reading-a-capture)
5. **Talk to harness sessions only through the harness** (`tmux::tmux_command()`,
   the WezTerm client turns). A bare `tmux` or `wezterm cli` call reaches the
   wrong server or bypasses throttling. → [sharing-and-parallel-runs.md](sharing-and-parallel-runs.md)
6. **Never steal focus outside a `level3_` file**, and never set
   `BISCUIT_L3_TAKE_FOCUS` for a human. → [level3.md](level3.md#never-steal-focus-outside-a-level3_-file)
7. **Never close a window or pane the harness did not create**; cleanup reaps
   only tagged resources whose owner is dead.
   → [sharing-and-parallel-runs.md](sharing-and-parallel-runs.md#cleanup-and-ownership)

## Topic pages

| Open when you are… | File |
|---|---|
| spawning a shell or program, sending input, reading a capture, using a helper | [harness-api.md](harness-api.md) |
| gating on a backend, or a backend skips when you expected it to run | [availability.md](availability.md) |
| sharing a pane, running L2 in parallel or serial, debugging a cleanup or WezTerm stall | [sharing-and-parallel-runs.md](sharing-and-parallel-runs.md) |
| injecting real OS key presses, or testing a key press without taking focus | [level3.md](level3.md) |

`biscuit-test-harness/README.md` holds the full trait surface and a quick
start. Apple Terminal's own traps are in the `rust-testing` skill's
`apple-terminal-harness-pitfalls.md`.

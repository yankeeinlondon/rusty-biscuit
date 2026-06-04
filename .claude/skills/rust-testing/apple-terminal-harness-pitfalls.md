# Apple Terminal L2 Harness Pitfalls

`AppleTerminalHarness` (`biscuit-test-harness/src/apple_terminal.rs`) drives the
real macOS Terminal.app through `osascript`. It is the lowest-capability L2
backend (no images, no OSC8, single underline) and the most fragile, because it
automates a GUI app that has global, shared, focus-sensitive state. These are
the failure modes learned the hard way — read this before touching the harness
or debugging an `level2_apple_terminal_*` failure.

## Rule 0 — never run L2 tests outside `just test-l2`

`level2_*` tests spawn **real terminal windows**. The `just test-l2` recipe is
the *only* supported way to run them: it pre-spawns **one shared pane per
backend** via `biscuit-harness-broker`, exports `BISCUIT_SHARED_*` ids, and runs
nextest with **`-j 1`**. Running them directly —
`cargo test` / `cargo nextest run -E 'test(/level2_/)'` — bypasses all of that:

- Tests spawn windows **in parallel**, racing on Terminal's global window state.
- `osascript` "window 1" / window-id lookups become ambiguous with many windows
  open, producing `Can't get selected tab of window … (-1728)` at `send_text`.
- Timeouts/panics **leak windows** (Drop never runs), and they accumulate across
  runs into dozens of orphans, making every later run worse.

If you must scope a single L2 test, spawn the shared window yourself and set the
env var, still single-threaded:

```bash
BROKER="$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)/debug/biscuit-harness-broker"
WID="$("$BROKER" spawn apple-terminal)"
BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID="$WID" \
  cargo nextest run -p biscuit-terminal-cli -E 'test(level2_apple_terminal_link_fallback_visible)' -j 1
"$BROKER" kill apple-terminal "$WID"
```

To reset after orphans accumulate, `killall Terminal` (safe iff Terminal.app is
not your interactive terminal — most developers use iTerm/WezTerm/Ghostty).

## Pitfall 1 — `do script` reuses the idle front window

Terminal's `do script "cmd"` **with no target window** does *not* reliably create
a new window. When an idle Terminal window is frontmost, `do script`
**reuses it** — nondeterministically, depending on focus and timing. The
returned `id of front window` is then that *pre-existing* window.

Why this is dangerous: a test that spawns its **own** window (e.g.
`level2_apple_terminal_harness_lifecycle`, which exercises Drop cleanup) can
capture the **shared broker window's** id, and its Drop / `close` then destroys
that window's only tab. The shared window survives with **0 tabs**, so every
subsequent attach-based test fails at its first `send_text("clear\n")` with:

```
osascript failed: execution error: Terminal got an error:
Can't get selected tab of window 1 whose id = NNNNN. (-1728)
```

Diagnosis recipe (proves it in seconds): spawn the shared window, record its
`count of tabs` (1), run only `harness_lifecycle`, re-check — it is now **0**.

### The implemented fix (focus-free; two parts)

Forcing a deterministic new window is impossible without `⌘N`
(`activate` + `System Events keystroke`), which **steals foreground focus** —
a hard no. So the fix avoids forcing a new window and instead makes reuse
*harmless*, in two focus-free parts:

1. **Ownership guard in `spawn_shell`.** Snapshot the window-id set before
   `do script`; the AppleScript returns `winId\tisNew` where `isNew` is "the id
   is not in the pre-snapshot". When `isNew` is false (reuse), Rust clears
   `owned`, so `Drop` **never closes a window the harness did not create**. No
   `activate`, no keystroke — spawning stays focus-free.
2. **`harness_lifecycle` skips under a shared broker window.** It is the only
   Apple-Terminal test that spawns its *own* window (to exercise `Drop`), so it
   is the only one that can reuse — and thereby pollute — the shared window.
   When `BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID` is set it early-returns
   (skip-clean); its `Drop`/cleanup coverage runs in the non-broker context
   where there is no shared window to corrupt.

Verified: under a dedicated broker window with `-j 1`, all attach tests pass,
`harness_lifecycle` skips in 0.04 s, the shared window keeps its tab (no
destruction), and the run leaks zero windows — deterministically across repeats.

Two invariants this enforces and that any future change must keep: **never
`close` a window the harness did not create**, and **spawning must never steal
focus**. Do not "improve" this by forcing a new window via `⌘N`/keystroke — that
reintroduces focus-steal. (Orphan-leak *identification* — Pitfall 2 — is still
open and tracked separately.)

## Pitfall 2 — the orphan reaper can't see leaked windows

`cleanup_stale_apple_terminal_windows()` finds harness windows by a custom title
prefix (`biscuit-test-terminal-<pid>`). But an interactive **shell prompt
overwrites the window title** via its own title escape sequences
(zsh `precmd` / bash `PROMPT_COMMAND`). Leaked windows therefore show
`title = "Terminal"`, the reaper never matches them, and they accumulate.

Implications and best practices:

- Leak identification must be **title-independent** (e.g. a tracked-id registry
  file the reaper consumes, or matching on a marker env/arg visible in the tab's
  process list) — titles are not load-bearing.
- macOS "**reopen windows when reopening an app**" can *restore* windows after
  `killall Terminal`, re-leaking them on next launch. Don't assume `killall`
  fully resets state.
- Drop is best-effort: timeouts/panics skip it. The reaper must run **at spawn
  time** so each run cleans the previous run's orphans.

## Pitfall 3 — capture is plain-text only

Terminal.app's scripting interface exposes only the **visible plain text** of a
tab; there is no SGR/byte stream. `CapturedFrame::raw == CapturedFrame::plain`,
so you can only assert "no literal escape garbage is visible", never "no OSC8
bytes were emitted". Byte-level negative assertions belong in L1 PTY tests; this
backend covers real-display *visibility* only. (For the WezTerm backend's
opposite problem — SGR collapsing in capture — see `wezterm-harness-pitfalls.md`.)

## Pitfall 4 — environment viability is per-backend

A green `level2_*_in_kitty` / `_apple_terminal` does **not** imply
`_in_wezterm` will run: each backend needs its own emulator actually installed
and scriptable. In a sandbox without a working WezTerm, every `*_in_wezterm`
test fails (not skips) once the broker hands out a dead/absent shared pane.
Treat a wall of single-backend failures as an **environment** signal, not a code
regression — confirm the same test passes on an available backend before
suspecting the renderer.

## Checklist before touching this harness

- [ ] Run only via `just test-l2` (or a single-test broker invocation, `-j 1`).
- [ ] No `activate` / `System Events keystroke` on the spawn path (no focus-steal).
- [ ] Never `close`/Drop a window not created by this harness instance.
- [ ] Don't rely on window titles for identification (shell overwrites them).
- [ ] Reset orphans with `killall Terminal`; expect macOS window-restore.
- [ ] Don't debug by opening/closing real windows in a live dev session.

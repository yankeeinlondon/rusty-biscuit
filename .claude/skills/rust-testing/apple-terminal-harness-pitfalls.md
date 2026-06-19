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
reintroduces focus-steal. (Orphan-leak cleanup — Pitfall 2 — is resolved via the
registry, self-healing sweep, and quit-relaunch husk recovery.)

## Pitfall 2 — resolved: orphan-window leaks (registry + self-healing sweep + quit-relaunch)

Leaked Terminal.app windows piled up (dozens of idle login-shell windows)
because **four** compounding problems defeated cleanup. The original plan blamed
only the first; the deeper three are what actually broke it. If you touch
`apple_terminal.rs`, know all four:

1. **Title is not load-bearing.** An interactive shell prompt can overwrite the
   `biscuit-test-terminal-<pid>` custom title, so a title-only reaper misses
   leaked windows. → window-id **registry** (below).
2. **The idle-shell predicate matched the wrong shell.**
   `looks_like_harness_window` built its process allowlist from
   `detect_shell()` (which prefers `bash`), but Terminal windows run the user's
   login shell — `-zsh`. The predicate therefore matched **nothing** and closed
   nothing. It now accepts any common login shell (`login`/`bash`/`zsh`/`sh`/
   `dash`/`fish`, with or without the `-` login marker).
3. **AppleScript `-1700` on every window.** `set w to item 1 of (every window
   whose id …)` + `repeat with p in (processes of t)` builds a nested
   element-reference chain that fails `p as text` with error `-1700`. So the
   predicate **errored on every window** — both the registry reap and the sweep
   gate it, so nothing was ever closed. Use a direct `first window whose id …`
   reference and materialize `processes of t` into a list variable first.
4. **`close` can't destroy a window — it leaves an invisible husk.** On current
   macOS, `close … saving no` leaves a `visible false`, zero-tab **husk** that
   no further `close` removes (the Drop path's polling-close just times out
   against it). Husks accumulate ~1 per spawn/kill and slow every full-window
   scan. The only reliable recovery is to **quit** the app.

### Layer 1 — title-independent window-id registry

Each owned spawn appends `{window_id, owner_pid, seq}` to
`${TMPDIR}/biscuit-test-terminal-registry.jsonl`; the reaper closes entries
whose owner is dead, then prunes. Mutations that can race with a rewrite are
guarded by a sidecar lock. Every close is gated by `looks_like_harness_window`
(now correct per #2/#3) so a recycled window id hosting real work is never
closed. Note `${TMPDIR}` varies by launch context (e.g. Claude Code overrides
it), so a registry written under one `$TMPDIR` is invisible to a run under
another — the sweep (Layer 3) is the backstop for that.

### Layer 3 — self-healing sweep (mirrors WezTerm)

The registry cannot catch windows restored by macOS (new ids), pre-registry
leaks, or cross-`$TMPDIR` leaks. The sweep tests every window against the strict
predicate (not busy, only login-shell processes, default 80×24, empty/
`"Terminal"`/harness-tag title) and closes matches. It runs when the open-window
count exceeds `LEGACY_WINDOW_LIMIT` **or** `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE=1`.

The count auto-trigger is the key parity with WezTerm (`background_count >
LEGACY_BACKGROUND_PANE_LIMIT`) that the original plan **deliberately omitted** —
which is why Apple Terminal never self-healed. WezTerm can sweep freely inside
its isolated `biscuit-bg` workspace; Terminal.app has no isolation, so the count
gate + narrow predicate are what keep a developer's real windows safe.

### Layer 4 — quit-relaunch (husk recovery)

Because `close` cannot remove husks (#4), when the window list past the limit
contains **only disposable** windows — invisible husks plus idle
harness-signature windows — cleanup quits Terminal.app and lets it relaunch
clean on the next spawn. It **refuses to quit** if any window looks like real
work (visible + not harness-signature), so a developer's window is never lost.
Verified: an all-husk pile-up quits + relaunches (husks cleared); a window with
a custom title blocks the quit and survives.

### macOS window-restoration mitigation

macOS "Reopen windows when reopening an app" can restore closed windows with
new ids and fresh login shells. To stop restoration globally:

```bash
defaults write com.apple.Terminal NSQuitAlwaysKeepsWindows -bool false
```

The harness does **not** mutate the developer's defaults.

### Best practices

- Drop is best-effort: timeouts/panics skip it. Cleanup runs **at spawn time**
  so each run cleans the previous run's orphans.
- `killall Terminal` / quit is the only thing that clears invisible husks;
  individual `close` cannot.
- Titles are not load-bearing for identification; rely on the registry and the
  idle-shell predicate.
- When debugging "nothing gets reaped", test `looks_like_harness_window`'s
  AppleScript directly against a real window id — a `-1700` or shell-name
  mismatch silently disables the whole reaper.

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

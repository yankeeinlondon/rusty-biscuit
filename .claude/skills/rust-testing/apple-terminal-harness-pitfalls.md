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

### The fix: spawn without `do script`

Forcing a new window with `⌘N` (`activate` + `System Events keystroke`)
**steals foreground focus**, which is not allowed. The harness avoids the
problem in a focus-free way:

1. **`spawn_shell` never uses `do script` to create a window.** It writes a
   `.terminal` settings file named after the window tag and opens it with
   `open -g -a Terminal`. Opening the file always creates a new window and
   leaves Terminal.app in the background, and the spawn finds the window by
   its settings-set name, so it can never adopt someone else's window. (From
   2026-10-10. The earlier fix diffed window ids around `do script` and gave
   up ownership on reuse.) Pitfall 7 covers the settings file's own rules.
2. **`harness_lifecycle` skips under a shared broker window.** It is the only
   Apple-Terminal test that spawns its *own* window (to exercise `Drop`).
   When `BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID` is set it early-returns
   (skip-clean); its `Drop`/cleanup coverage runs in the non-broker context
   where there is no shared window to corrupt.

`send_text` still uses `do script … in selected tab of` the harness's own
window, which targets that tab and cannot reuse another window.

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

## Pitfall 5 — wait for the printed sentinel, and not by its bare name

`settle()` on this backend is a fixed 400 ms sleep, and `bt`'s first launch in
a freshly spawned window runs its real capability probes against Terminal.app
(`preserve_capabilities(true)`), which took about 1.6 s on the development Mac
under the pre-push hook. A test that sleeps and takes one capture sees the
opening sentinel and nothing else, and fails a renderer that is working. Poll
with a deadline instead (`common::capture_until` in `biscuit-terminal-cli`'s
tests), as the rust-testing contract already requires.

The predicate has a trap of its own: the shell **echoes the command line**, and
that echo contains both sentinel names. A wait on `plain.contains("__BT_END__")`
is satisfied by the echo and returns before `bt` has printed anything. The
printed sentinel differs from the echo only in having a real newline before it
where the echo has the two literal characters `\n`, so match on the newline
form — `split("__BT_START__\n")` then `split_once("\n__BT_END__")` — and use
the same function for the wait and for the assertion's bounded region.

## Pitfall 6 — `osascript` costs, and two traps that cost more

A fresh `osascript` process spends about 100 ms loading Terminal.app's
scripting dictionary at its first `tell`, so a one-shot call costs 150–300 ms.
The harness therefore runs its AppleScript through one long-lived helper per
test process (`osa_helper` in `apple_terminal.rs`), where a call after the
first takes 10–20 ms. Route new harness scripts through `run_script`. A script
must return text: a list result reads as empty. JXA quirks the helper works
around: `NSData.length` arrives as a string (compare `Number(…)`), and
`executeAndReturnError` crashes when given an error out-pointer (the helper
wraps each script in an AppleScript `try` instead).

Two traps cost more than any call, and one of them fails without any error:

- **System Events `whose frontmost` costs about 2 s.** On the development Mac
  (2026-10-10) `name of first process whose frontmost is true` took 2.1 s
  each time, and timed out with `-1712` when the `System Events` process was
  left stopped (`ps` state `T`). Read the frontmost application with
  StandardAdditions' `path to frontmost application`, which takes a few
  milliseconds, and call System Events only when something actually needs
  restoring.
- **`tab` inside `tell application "Terminal"` is Terminal's tab class, not
  the tab character.** A script that builds `id & tab & title` inside that
  block writes the literal word `tab`, so a Rust-side `split('\t')` matches
  nothing and the code that reads it does nothing. Bind
  `set sep to (character id 9)` outside the `tell` block. Until 2026-10-10
  this bug meant the cleanup's title pass never closed a window.

A window made with `do script` first starts the developer's own interactive
login shell, rc files included, and only then runs the harness's `exec` line.
That took 1.5–2.5 s per window on the development Mac. AppleScript has no
property for a window's command, which is why the spawn opens a settings file
instead (Pitfall 7).

## Pitfall 7 — the spawn's settings file

The spawn writes `<tag>.terminal` with `CommandString` and `RunCommandAsShell`,
and opens it with `open -g -a Terminal`. Terminal.app then runs the command as
the window's shell, still under `login -pf` and with its own `TERM_PROGRAM`,
`TERM`, and `LANG`, and the bash prompt is up about 0.4 s after the window
appears. `.command` and `.tool` files do not help: Terminal types their path
into the profile's shell, so the rc files still run.

- **Opening the file imports a settings set**, persisted in
  `com.apple.Terminal`'s `Window Settings`. The spawn deletes it with
  `delete (every settings set whose name is "<tag>")` as soon as the window
  exists; the live window keeps running on the default settings. Cleanup
  deletes harness-tagged sets whose owning process is dead.
- **`CommandString` is split on whitespace, not parsed by a shell.** Put the
  invocation in a launcher script, and keep the script's path free of
  whitespace.
- **The command must name the shell the tab ends up running.** Terminal
  counts a tab as idle only while its process carries the command's name. A
  launcher run directly (`/tmp/x.sh`) that `exec`s `bash` leaves the tab
  `busy`, and closing a busy tab raises a "terminate running processes?"
  sheet. Terminal's own scripting cannot dismiss that sheet (the sheet's
  window has no id and does not understand `close`), and with System Events
  wedged nothing automated can, so it stays on the developer's screen.
  `/bin/bash /tmp/x.sh` keeps the tab idle. Check `busy of selected tab` on
  a probe window before you ever close one in a live session.

## Checklist before touching this harness

- [ ] Run only via `just test-l2` (or a single-test broker invocation, `-j 1`).
- [ ] No `activate` / `System Events keystroke` on the spawn path (no focus-steal).
- [ ] Never `close`/Drop a window not created by this harness instance.
- [ ] Don't rely on window titles for identification (shell overwrites them).
- [ ] Reset orphans with `killall Terminal`; expect macOS window-restore.
- [ ] Don't debug by opening/closing real windows in a live dev session.

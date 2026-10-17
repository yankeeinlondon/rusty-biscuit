# Apple Terminal L2 Harness Pitfalls

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

`AppleTerminalHarness` (`biscuit-test-harness/src/apple_terminal.rs`) drives
Terminal.app through `osascript`. It is the lowest-capability L2 backend (no
images, no OSC8, single underline) and the most fragile: it automates a GUI app
whose window list, focus, and scripting state are global and shared with the
developer. Read this before touching the harness or debugging a
`level2_apple_terminal_*` failure.

## Hard invariants

- **Never close a window the harness did not create.**
- **Spawning never steals focus.** No `activate`, no System Events keystroke,
  no `⌘N` on the spawn path.
- **Titles are not identity.** A shell prompt can overwrite the window title;
  identify windows by the registry and the idle-shell predicate.

## Run only through `just test-l2`

Running `level2_*` directly (`cargo test`, a bare nextest filter) races windows
in parallel, makes "window 1"/id lookups ambiguous (`Can't get selected tab of
window … (-1728)`), and leaks windows on timeout or panic. The recipe and the
`apple-terminal-l2` nextest test group (`max-threads = 1`) keep at most one
Terminal window open; see [l2-tests.md](l2-tests.md). To scope one test, spawn
the shared window yourself and stay single-threaded:

```bash
BROKER="$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)/debug/biscuit-harness-broker"
WID="$("$BROKER" spawn apple-terminal)"
BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID="$WID" \
  cargo nextest run -p biscuit-terminal-cli -E 'test(level2_apple_terminal_link_fallback_visible)' -j 1
"$BROKER" kill apple-terminal "$WID"
```

`killall Terminal` resets accumulated orphans (safe only when Terminal.app is not
your own interactive terminal). Do not debug by opening and closing real windows
in a live session.

## Spawn through a settings file, never `do script`

`do script` with no target window reuses an idle frontmost window
nondeterministically, so a test that owns its window can capture someone else's
id and its `Drop` then closes that window's only tab (every later
`send_text` fails with `-1728`). `do script` also runs the developer's
interactive rc files first, which is slow. `spawn_shell` instead writes
`<tag>.terminal` (`CommandString` + `RunCommandAsShell`), opens it with
`open -g -a Terminal`, and finds the window by settings-set name. Rules for that
file:

- **Opening it imports a settings set** into `com.apple.Terminal`; the spawn
  deletes it once the window exists, and cleanup deletes harness-tagged sets
  whose owner is dead.
- **`CommandString` is split on whitespace**, not parsed by a shell. Put the
  invocation in a launcher script whose path has no whitespace.
- **The command must name the shell the tab ends up running**
  (`/bin/bash /tmp/x.sh`, not `/tmp/x.sh` that `exec`s bash). Otherwise the tab
  stays `busy`, and closing it raises a "terminate running processes?" sheet that
  nothing automated can dismiss. Check `busy of selected tab` before closing.

`send_text` uses `do script … in selected tab of` the harness's own
window, which cannot reuse another window.
`level2_apple_terminal_harness_lifecycle`
(`biscuit-terminal/cli/tests/level2/level2_apple_terminal_prose.rs`), the only
test that owns its window to exercise `Drop`, skips when
`BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID` is set.

## Orphan cleanup runs at spawn time, in layers

`Drop` is best-effort (timeouts and panics skip it), so
`cleanup_stale_apple_terminal_windows` runs at each spawn and cleans the previous
run's orphans:

1. **Registry** — each owned spawn appends `{window_id, owner_pid, seq}` to
   `${TMPDIR}/biscuit-test-terminal-registry.jsonl` (lock-guarded);
   `reap_registered_windows` closes entries whose owner is dead. `$TMPDIR`
   differs across launch contexts, so a registry can be invisible to another run.
2. **Sweep** — when the window count exceeds `LEGACY_WINDOW_LIMIT` or
   `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE=1`, close every window matching the
   strict predicate (not busy, login-shell processes only, default 80×24,
   empty/`Terminal`/harness-tag title). Terminal.app has no isolated workspace,
   so the count gate and narrow predicate are what protect real windows.
3. **Quit-relaunch** — `close … saving no` leaves an invisible zero-tab husk that
   no further `close` removes. When every window past the limit is disposable
   (husk or idle harness window), `quit_terminal_if_only_disposable` quits
   Terminal.app; any window that looks like real work blocks the quit.

Every close is gated by `looks_like_harness_window`. If "nothing gets reaped",
run its AppleScript against a real window id: it must accept any login shell
(`login`/`bash`/`zsh`/`sh`/`dash`/`fish`, with or without the leading `-`), and
it must use a direct `first window whose id …` reference and copy
`processes of t` into a list first, or `p as text` fails with `-1700` and the
predicate silently rejects every window.

macOS window restoration can reopen closed windows with new ids;
`defaults write com.apple.Terminal NSQuitAlwaysKeepsWindows -bool false` stops it.
The harness never changes the developer's defaults.

## AppleScript traps

- **Use the long-lived helper.** A fresh `osascript` pays a large first-`tell`
  cost; route scripts through `run_script` (`osa_helper` module). Scripts must
  return text (a list reads as empty).
- **`tab` inside `tell application "Terminal"` is Terminal's tab class**, not the
  tab character. Bind `set sep to (character id 9)` outside the `tell` block, or
  a Rust-side `split('\t')` silently matches nothing.
- **Avoid System Events `whose frontmost`**: it is slow and times out (`-1712`)
  when System Events is stopped. Read the frontmost app with
  `path to frontmost application`, and call System Events only to restore focus.

## Capture and assertions

- **Capture is plain text only** (`CapturedFrame::raw == plain`). Assert "no
  escape garbage is visible", never "no OSC8 bytes were emitted"; byte-level
  negatives belong in L1 PTY tests. For WezTerm's opposite problem see
  [wezterm-harness-pitfalls.md](wezterm-harness-pitfalls.md).
- **Poll, do not sleep.** `settle()` is a short fixed sleep, while a first `bt`
  launch runs real capability probes. Wait with `capture_until`
  (`biscuit-terminal/cli/tests/common/mod.rs`).
- **Match the printed sentinel, not its name.** The shell echoes the command
  line, which contains both sentinel names. Match the newline-delimited form
  (`__BT_START__\n` … `\n__BT_END__`) and use the same function for the wait and
  the assertion region, as `bounded_output` in `level2_apple_terminal_prose.rs`
  does.
- **Failures are per-backend.** A wall of failures on one backend is usually an
  environment signal; confirm the test on another backend before suspecting the
  renderer.

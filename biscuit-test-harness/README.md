# biscuit-test-harness

Shared **real-terminal test harness** for the rusty-biscuit monorepo.

It lets a `cargo test` run spawn the workspace's CLI binaries (`bt`,
`question`, …) inside a *real* terminal emulator or multiplexer, drive
them with synthetic input, and capture the rendered pane text — so
glyph-width, SGR, OSC8, scroll-handling, and graphics-protocol
regressions become observable in assertions.

It is a `publish = false` workspace member, consumed as a
`[dev-dependencies]` entry by CLI test suites (e.g. `biscuit-terminal-cli`,
`biscuit-tui-cli`, `darkmatter-cli`).

## Testing vocabulary

The whole monorepo uses one Level 1 / 2 / 3 taxonomy. This crate covers
Levels 2 and 3.

| Level | What it is | Needs this crate? |
|-------|------------|-------------------|
| **Level 1** | PTY-based tests. The test generates input bytes; the binary parses them. No real terminal involved. Use `expectrl` directly. | No |
| **Level 2** | Run-in-real-terminal with IPC. The binary renders through a real terminal's display path; input is injected as bytes via the terminal's own CLI. | **Yes** — the `TerminalHarness` implementations. |
| **Level 3** | OS-level keyboard injection. Real `CGEvent` / X11 XTEST / `SendInput` key presses, so the *terminal's input encoder* fires. | **Yes** — one injector module per platform: [`cliclick`](src/cliclick.rs) (macOS), [`xdotool`](src/xdotool.rs) (Linux/X11), [`win_input`](src/win_input.rs) (Windows). |

**Why Level 2/3 exist:** a Level 1 test can never catch a bug in the
*terminal's* encoder or display path, because the test itself produces
the bytes the binary parses — the terminal is never exercised. A
requirement of the form "when the user presses X, behaviour Y happens"
is only truly verified at Level 2 (kitty bytes piped through the
terminal's CLI) or Level 3 (real key injection).

## Quick start

```rust
use biscuit_test_harness::{TerminalHarness, skip_with_reason};
use biscuit_test_harness::wezterm::WezTermHarness;

#[test]
fn level2_prose_emits_sgr() {
    // Skip-clean: never fail when the required tooling is absent.
    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    // `new()` defaults to SpawnVisibility::Background — the window
    // never grabs focus or appears on the active desktop.
    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell");

    // Shell-model: drive the binary by typing a command line.
    harness
        .send_command_with_env(
            "bt prose --force-color \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env");

    let frame = harness.capture().expect("capture");
    assert!(frame.raw.contains("\x1b[31m"));   // SGR-bearing capture
    assert!(frame.plain.contains('x'));        // escapes stripped
}
```

## The `TerminalHarness` trait

Every backend implements one **shell-model-first** contract:

| Method | Purpose |
|--------|---------|
| `spawn_shell()` | Open a fresh login shell in a new pane/window, wait for its prompt. Prepends the cargo target dir to `PATH` and forces color env vars. |
| `send_text(&[u8])` | Write **raw bytes** to the pane's stdin (escape sequences, kitty keyboard-protocol bytes, …). |
| `send_command_with_env(cmd, env)` | Type one command line with scoped `KEY=value` prefixes. Portable, single-quote-escaped. *(provided)* |
| `capture()` | Capture the pane's current rendered text as a [`CapturedFrame`]. |
| `settle()` | Sleep long enough for a redraw (default 200 ms). *(provided)* |
| `spawn_program(prog, args)` | Escape hatch: launch a binary directly, no shell. Defaults to `Unsupported`. *(provided)* |

### `spawn_shell` vs `spawn_program`

- **`spawn_shell` (recommended).** Spawns a POSIX shell (`bash`/`sh` —
  see `detect_shell`), augments `PATH`, forces color, waits for the
  prompt. Drive the binary with `send_text(b"bt …\n")` or
  `send_command_with_env`.
- **`spawn_program` (escape hatch).** Launches a program directly — no
  shell, no `PATH` augmentation, no prompt-readiness wait, no
  color-forcing. Only WezTerm, Kitty, and tmux override it; callers
  supply absolute paths and handle their own settling.

### Sending input — pick the right channel

| Channel | Use it for |
|---------|-----------|
| `send_command_with_env(cmd, env)` | The normal case: run a command, optionally with env vars scoped to just that command. |
| `send_text(&[u8])` | Raw escape sequences / kitty keyboard-protocol bytes — anything that must reach the pane's stdin un-interpreted. |
| `TmuxHarness::send_key("C-space")` | Symbolic chord names routed through tmux's key-translation layer. Needed for chords containing bytes (NUL) that `Command::arg` rejects. tmux-only. |

### Capturing — `CapturedFrame`

`capture()` returns a `CapturedFrame { raw, plain }`:

- `raw` — pane text **including** ANSI/SGR/OSC sequences (when the
  backend can return them).
- `plain` — `raw` with escape sequences stripped via `strip_ansi`.

Assert styling/links on `raw`; assert visible text on `plain`.

## Default L2 backend: prefer `TmuxHarness`

**Pick `TmuxHarness` first.** It is fully headless, requires only `tmux`
on `$PATH`, and runs on any CI runner without a GUI. It covers the
common Level-2 capability surface (plain text, SGR, OSC8) and is the
only backend immune to the "which terminal am I running inside" gotcha
described below.

Reach for `WezTermHarness` or `KittyHarness` **only** when you need a
capability tmux cannot deliver: graphics protocols, inline images,
truecolor display nuances, Kitty keyboard-protocol bytes, or a
`pane_size()`/`pane_cols()` geometry query against a real emulator.
Tests that require those capabilities should document the dependency
inline. Use `AppleTerminalHarness` exclusively for graceful-degradation
checks on a deliberately low-capability terminal.

## Harness variants — when to use which

All four implement `TerminalHarness`. Choose by the capability you need
to verify and the tooling available on the host.

| Harness | Headless? | Capability profile | Best for | Cannot verify |
|---------|-----------|--------------------|----------|---------------|
| **`TmuxHarness`** | ✅ fully headless | Multiplexer; portable | Default Level-2 choice. Chords via `send_key`. Runs anywhere `tmux` is installed. | Kitty keyboard-protocol bytes (tmux strips them); graphics protocols; truecolor display nuances. |
| **`WezTermHarness`** | ⚠️ needs a running WezTerm | Full: SGR, OSC8, images, truecolor | SGR/OSC8/image/diagram rendering against a high-capability emulator. `pane_size()` geometry. | Anything when WezTerm isn't the host terminal (see *Environment prerequisites*). |
| **`KittyHarness`** | ⚠️ needs a running Kitty | Full: SGR, OSC8, Kitty graphics protocol | Kitty graphics-protocol image tests; SGR/OSC8 cross-checks. `pane_cols()` geometry. | Anything when not run inside / pointed at a Kitty session. |
| **`AppleTerminalHarness`** | ❌ macOS GUI app | **Low**: no images, no OSC8, single underline only | Verifying `Prose` *graceful degradation* on a low-capability terminal. | Byte-level negative assertions — capture is plain-text only (`raw == plain`). |

Rules of thumb:

- **Verifying rendering works on a capable terminal?** Prefer
  `TmuxHarness` (most portable); reach for `WezTermHarness` /
  `KittyHarness` when you need images, true OSC8, or graphics-protocol
  bytes.
- **Verifying a feature *degrades* on a weak terminal?** Use
  `AppleTerminalHarness` with `.preserve_capabilities(true)` so `bt`
  runs its real detection instead of the force-color "everything
  supported" path.
- **Verifying chords / key handling?** `TmuxHarness::send_key` for
  portable chords; Level 3 (`cliclick` / `xdotool` / `win_input`) for
  true OS key injection.

### `SpawnVisibility` — background vs foreground

`WezTermHarness` and `KittyHarness` accept a `SpawnVisibility`. The
**default is `Background`** so a Level-2 run never interrupts a
developer working on the same machine.

| Visibility | WezTerm | Kitty | When to use |
|------------|---------|-------|-------------|
| `Background` *(default)* | Spawns into a dedicated workspace (`biscuit-bg`) — created but not on the active workspace. | Passes `--keep-focus` to `kitty @ launch` — the new window never steals focus. | Almost always. Any test that only sends bytes via the terminal's CLI. |
| `Foreground` | No workspace override; window appears on the active desktop. | No `--keep-focus`; standard launch. | **Only** Level-3 tests — OS keyboard injection can only target a window that is actually focused. Pair with `WezTermHarness::focus_spawned_pane()`. |

Select a non-default visibility with the builder:

```rust
use biscuit_test_harness::SpawnVisibility;
let harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
```

`TmuxHarness` has no `SpawnVisibility` — tmux sessions are detached, so
visibility is irrelevant. `AppleTerminalHarness` always spawns behind
the developer's frontmost app (it snapshots and restores focus around
the `do script` call).

## Sharing a harness across tests — `SharedHarness<T>`

Spawning a real terminal costs 2–3 s. Test binaries that hold many
`#[serial]` tests against the same backend should share a single
harness via [`shared::SharedHarness`]. The helper wraps the
`Mutex<Option<T>>` + `libc::atexit` cleanup pattern so the harness's
`Drop` impl runs at process exit (Rust does not run `Drop` on `static`
values, which would otherwise leak the pane).

```rust,ignore
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;

static SHARED: SharedHarness<WezTermHarness> = SharedHarness::new();

fn run_in_pane() {
    let mut guard = SHARED.get_or_init(|| {
        let mut h = WezTermHarness::new();
        h.spawn_shell().expect("spawn_shell");
        h
    });
    let harness = guard.as_mut().expect("harness present");
    // ... drive the harness ...
}
```

## Defensive cleanup

The harness defensively removes stale resources before opening new
real-terminal sessions. WezTerm panes, tmux sessions, and Terminal.app
windows are tagged with the creating process id; later runs close only
tagged resources whose process is no longer alive, so active concurrent
test runs are left alone.

`cleanup_stale_terminal_harness_resources()` is available for explicit
suite setup or local maintenance. Backend spawns also call their own
cleanup path once per process. For historical WezTerm leaks created
before tagging existed, the WezTerm cleanup sweeps untagged panes in the
`biscuit-bg` workspace when that workspace grows past a conservative
limit, or when `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM=1` is set.

### Apple Terminal: known gaps and invariants

Terminal.app is GUI-automated via `osascript` and is the most fragile
backend. Several issues bite anyone editing `apple_terminal.rs`:

1. **Window identity (title-independent registry).** Spawned windows get a
   custom title (`biscuit-test-terminal-<pid>`), but it is not load-bearing:
   an interactive shell prompt can overwrite the window title, and identity
   must survive that. Every owned spawn records its window id in
   `${TMPDIR}/biscuit-test-terminal-registry.jsonl`; the reaper closes
   registry windows whose owner process is dead, regardless of title. The
   registry is pruned after each reaper run, and mutations that can race with
   a rewrite use a sidecar lock.
2. **The idle-shell predicate must match the real login shell.** The reaper
   guards every close with `looks_like_harness_window` (idle, default 80×24,
   empty/"Terminal"/harness-tag title, only login-shell processes). Two bugs
   made it match **nothing**, so leaked windows piled up unbounded: (a) it
   built its process allowlist from `detect_shell()` (which prefers `bash`)
   while Terminal windows actually run the user's login shell (`-zsh`); (b)
   `item 1 of (every window …)` + `processes of t` produces a reference chain
   that fails `as text` coercion with AppleScript error `-1700` on *every*
   window. The predicate now accepts any common login shell and uses a direct
   `first window whose id …` reference with a materialized process list.
3. **`do script` reuses the idle front window.** `do script "cmd"` with no
   target nondeterministically runs in the frontmost idle window instead of
   creating a new one. A self-spawning test (`level2_apple_terminal_harness_lifecycle`)
   could thereby capture and close the *shared* test window. **Fixed** (focus-free):
   `spawn_shell` reports whether the window was genuinely new (id-diff) and
   clears `owned` on reuse so `Drop` never closes a reused window; the
   self-spawning lifecycle test skips when a shared broker window is present.
4. **`close` cannot destroy a window — it leaves an invisible husk.** On
   current macOS, `close … saving no` does not remove a Terminal.app window;
   it leaves a `visible false`, zero-tab **husk** that no further `close`
   removes (even the Drop path's polling-close just times out against it).
   Husks accumulate ~1 per spawn/kill cycle and eventually slow every
   full-window scan. The only reliable recovery is to **quit** the app — see
   the self-healing cleanup below.

Two **invariants** any spawn change must preserve: spawning must **never
steal foreground focus** (no `activate` / `System Events keystroke`), and
the harness must **never close a window it did not create** (track
ownership by window-id diff, not by `id of front window`).

#### Self-healing cleanup (sweep + quit-relaunch)

Mirroring the WezTerm sweep, cleanup self-heals when leaks pile up — no env
var required:

- **Sweep.** When the open-window count exceeds `LEGACY_WINDOW_LIMIT` (or
  `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE=1` is set), every window is tested
  against the strict `looks_like_harness_window` predicate and matches are
  closed. This catches leaks the registry cannot see (macOS-restored windows
  with new ids, pre-registry leaks, leaks recorded under a different
  `$TMPDIR`).
- **Quit-relaunch (husk recovery).** Because `close` cannot remove invisible
  husks, when the window list has degenerated into **only** disposable
  windows — invisible husks plus idle harness-signature windows — past the
  same limit, cleanup quits Terminal.app (the one thing that clears husks)
  and lets it relaunch clean on the next spawn. It **refuses to quit** if any
  window looks like real work (visible, non-default, titled, or running a
  command), so a developer's window is never lost.

`LEGACY_WINDOW_LIMIT` is conservative because Terminal.app has no workspace
isolation (unlike WezTerm's quarantined `biscuit-bg` workspace); the narrow
predicate is what keeps the sweep and quit safe on a machine where someone
uses Terminal.app interactively.

#### macOS window-restoration mitigation

macOS "Reopen windows when reopening an app" can restore closed Terminal
windows on the next launch with **new** window ids and fresh login shells,
re-leaking them. To stop restoration globally:

```bash
defaults write com.apple.Terminal NSQuitAlwaysKeepsWindows -bool false
```

The harness does **not** mutate the developer's defaults. See
`.claude/skills/rust-testing/apple-terminal-harness-pitfalls.md`.

## Environment prerequisites

`available()` gates every harness. It returns `false` cleanly when the
required tooling is missing — the test then skips rather than fails.

| Harness | `available()` requires |
|---------|------------------------|
| `TmuxHarness` | `tmux` on `$PATH`. **Nothing else** — fully self-contained. |
| `WezTermHarness` | `wezterm` on `$PATH` **and** `WEZTERM_UNIX_SOCKET` set. |
| `KittyHarness` | `kitty` on `$PATH` **and** `KITTY_LISTEN_ON` set. |
| `AppleTerminalHarness` | macOS, `CI != 1`, and `osascript` can address Terminal.app. |
| `cliclick` (Level 3, macOS) | `cliclick` on `$PATH`. Gate *additionally* on `cliclick::accessibility_trusted()` — cliclick can be installed yet have every event dropped by the WindowServer when the runner lacks macOS Accessibility trust. |
| `xdotool` (Level 3, Linux) | Linux, `xdotool` on `$PATH`, **and** `DISPLAY` set. Wayland has no reachable XTEST equivalent, so a Wayland session reports unavailable and skips. |
| `win_input` (Level 3, Windows) | Windows and a working `powershell`. |

### The "which terminal am I running inside" gotcha

`WEZTERM_UNIX_SOCKET` and `KITTY_LISTEN_ON` are exported **automatically
only to processes launched inside that terminal**. This is symmetric:

- Run `cargo test` from a **WezTerm** window → WezTerm tests run, Kitty
  tests skip.
- Run it from a **Kitty** window (with remote control enabled) → Kitty
  tests run, WezTerm tests skip.

A skipped Kitty/WezTerm test is therefore usually *not* a defect — it
means that terminal wasn't the host. `TmuxHarness` is the only harness
immune to this, which is why it's the most portable Level-2 choice.

### Exercising the *other* terminal — cold-start recipes

To run a harness for a terminal you are **not** currently inside, launch
that terminal yourself with an explicit control socket, then export the
matching env var before `cargo test`.

**WezTerm** (its mux server persists independently of any GUI window):

```bash
export WEZTERM_UNIX_SOCKET="$(ls "$HOME/.local/share/wezterm/gui-sock-"* | head -1)"
cargo test -p biscuit-terminal-cli --test level2_prose_styling
```

**Kitty** (no daemon — you must start an instance with remote control):

```bash
# `--start-as=minimized` keeps the bootstrap window out of the way.
kitty -o allow_remote_control=yes --listen-on unix:/tmp/kitty-l2 --start-as=minimized &
sleep 2
KITTY_LISTEN_ON=unix:/tmp/kitty-l2 cargo test -p biscuit-terminal-cli --test level2_prose_styling
```

Once the instance exists, the harness's `SpawnVisibility::Background`
(`--keep-focus`) handles the *per-test* windows — the bootstrap step
above only brings the terminal itself into existence.

## Level 3 — OS keyboard injection

Unlike `send_text` (which writes bytes straight to the pane's stdin and
bypasses the terminal's input encoder), a Level-3 injector emits OS-level
key presses that the *terminal itself* must encode and forward — the only
way to verify "what bytes does the terminal emit when the user presses
Ctrl+C?". There is one module per platform; each exposes its own
`available()` and skips cleanly off its platform.

| Module | Platform | Mechanism |
|--------|----------|-----------|
| [`cliclick`](src/cliclick.rs) | macOS | The `cliclick` Homebrew utility (`CGEventCreateKeyboardEvent`), plus `osascript` / System Events for the cases cliclick cannot express. |
| [`xdotool`](src/xdotool.rs) | Linux / X11 | `xdotool key` via the X11 **XTEST** extension — the same server input pipeline a physical keyboard uses. |
| [`win_input`](src/win_input.rs) | Windows | A PowerShell driver over `SendKeys.SendWait` (`keybd_event`/`SendInput`). |

### `cliclick` (macOS)

Free functions: `available`, `accessibility_trusted`, `key_down`,
`key_up`, `press`, `hold_modifier`, `type_text`, `move_to`, `click_at`,
`click_then_keys`, `click_then_text`, `click_then_move`,
`click_then_press`, `click_then_ctrl_chord`, `click_then_alt_chord`,
`system_events_key_down` / `_up`, `activate_app`,
`activate_process_window`, `process_id_for_window`.

`available()` only proves the binary is on `$PATH`. Fold
`accessibility_trusted()` into the gate as well: without macOS
Accessibility trust the WindowServer silently drops every injected event,
which would red-fail as though the *product* were broken.

Modified chords (`click_then_ctrl_chord`, `click_then_alt_chord`) route
through System Events rather than cliclick — cliclick cannot carry a
modifier flag on the same event as a letter, so the flag races the letter
and the terminal intermittently receives the unmodified character.

### `xdotool` (Linux / X11)

Free functions: `available`, `window_id_for_title`, `activate_window`,
`chord`, `focus_then_ctrl_chord`.

`available()` requires the binary **and** a live `DISPLAY`; Wayland
sessions report unavailable and skip.

`--window` is deliberately never passed: it switches `xdotool` from XTEST
to `XSendEvent`, whose events carry the `send_event` flag that terminals
ignore as untrusted input. That would inject nothing and then fail as
though the product were broken. Focus the window first, inject globally.

### `win_input` (Windows)

Free functions: `available`, `focus_then_ctrl_chord`.

`GenerateConsoleCtrlEvent` is *not* Level 3 — it posts a console-control
notification downstream of both the keyboard and the terminal's encoder.
It remains useful as the lower-level diagnostic that isolates the signal
path when a Level-3 test fails.

`focus_then_ctrl_chord` taps ALT immediately before `SetForegroundWindow`
because Windows refuses foreground activation from a process that does
not already own it; without the tap, activation is silently downgraded to
a flashing taskbar button and the chord lands in the user's real window.

### Level-3 practices

- The spawned window must be **focused** — use
  `SpawnVisibility::Foreground` and `focus_spawned_pane()`.
- Window selection is by **unique** title match on all three platforms.
  Zero or several matches is an error rather than a first-match guess:
  injecting Ctrl+C into the wrong window looks identical to a broken
  product.
- Focus is a shared global resource — gate Level-3 tests behind an env
  flag (convention: `RUN_LEVEL3=1`) and run them with `--test-threads=1`
  so parallel windows don't steal focus mid-injection.
- **Bare-modifier press** events are structurally unreliable on macOS via
  `cliclick`. *Chord* injection (`Ctrl+R`) works reliably; bare modifiers
  need a Level-2 raw-kitty-bytes test instead.

## Utilities

Re-exported from the crate root:

| Item | Purpose |
|------|---------|
| `CapturedFrame` | `raw` + `plain` capture result. |
| `SpawnVisibility` | `Background` (default) / `Foreground`. |
| `strip_ansi(&str)` | Remove CSI/OSC/SGR/charset-designation escapes (ECMA-48 §5.4 aware). |
| `skip_with_reason(&str)` | Print `skipping: requires <X>` to stderr; returns `true`. |
| `cleanup_stale_terminal_harness_resources()` | Best-effort cleanup for tagged stale WezTerm, tmux, and Terminal.app resources. |
| `detect_shell()` | Pick a POSIX shell (`bash` → `sh` → `$SHELL`). |
| `cargo_bin_dir(name)` | Locate the directory of a built cargo binary, for `PATH` augmentation. |
| `apply_color_forcing_env(&mut Command)` | Set `FORCE_COLOR`/`CLICOLOR_FORCE`/`TERM`/`COLORTERM`, clear `NO_COLOR`. |
| `wait_for_prompt(&mut harness)` | Poll `capture()` until a shell prompt (`$`/`#`/`%`) appears (5 s cap). |
| `run_with_timeout` / `run_with_stdin_timeout` | Run a `Command` with a wall-clock timeout and pipe-deadlock-safe draining. |
| `SPAWN_TIMEOUT`, `SEND_TIMEOUT`, `CAPTURE_TIMEOUT`, `QUERY_TIMEOUT`, `CLEANUP_TIMEOUT` | Default timeout constants. |

## Skip-clean contract

Every Level-2/3 test **must** check `available()` first and early-return
via `skip_with_reason` when it returns `false`. No `#[ignore]` markers.
On GitHub-hosted CI runners — which lack WezTerm and Kitty, and offer no
focusable window for a Level-3 injector to target — the tests print
`skipping: requires <X>` and exit `ok`, keeping CI green
while still acting as a local regression net. (Level-2 tests are not a
PR gate; that would need a self-hosted runner with the emulators
installed.)

## Concurrency

Real-terminal tests share global OS resources (focus, terminal window
lists). Serialize them with `serial_test` — e.g.
`#[serial(level2_terminal)]` — so two harnesses don't race on window
indices or focus. `AppleTerminalHarness` in particular **requires**
serialization: Terminal.app exposes a single global AppleScript state.

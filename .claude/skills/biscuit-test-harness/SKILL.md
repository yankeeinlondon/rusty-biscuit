---
name: biscuit-test-harness
description: |
  Shared real-terminal test harness crate (L2/L3): backend selection
  (tmux/WezTerm/Kitty/Apple Terminal/cliclick), `SpawnVisibility`,
  `SharedHarness`, `available()` gating, and the
  "which terminal am I running inside" gotcha. Load when picking a
  backend or using the harness API; for *assertion patterns* against
  captured frames, load the `rust-testing` skill instead.
---

# biscuit-test-harness

`publish = false` workspace member under `biscuit-test-harness/`. Consumed
as a `[dev-dependencies]` entry by CLI test suites
(`biscuit-terminal-cli`, `biscuit-tui-cli`, `darkmatter-cli`, ...) to run
binaries inside a *real* terminal emulator and capture rendered output.

This skill covers the **harness API** — backend choice, spawn
visibility, sharing, env gates. For how to *write assertions* against
`CapturedFrame.raw` so SGR re-emission variance does not break tests,
load the `rust-testing` skill via the Skill tool and open its
`wezterm-harness-pitfalls.md` topic page.

## Pick a backend

Decision order (use the first that satisfies your requirement):

```
Do you need to verify the *terminal's* input encoder (e.g. what bytes
does bare Ctrl emit)?
├── YES → Level 3. Use `cliclick` (macOS, RUN_LEVEL3=1, foreground).
└── NO  → Level 2. Pick the backend:
    ├── Default → TmuxHarness (fully headless, portable, no GUI).
    ├── Graphics protocol / inline images / Kitty keyboard-protocol →
    │   KittyHarness or WezTermHarness.
    ├── Truecolor display nuance / OSC8 fidelity / `pane_size()` →
    │   WezTermHarness.
    └── Graceful-degradation on a weak terminal →
        AppleTerminalHarness (`.preserve_capabilities(true)`).
```

**Why tmux first.** Headless, on `$PATH` everywhere, immune to the
host-terminal gotcha below. Cannot deliver Kitty keyboard-protocol bytes
(tmux strips them), graphics protocols, or truecolor display fidelity.

## Backend matrix

| Harness | Headless | Capabilities | Best for | Cannot verify |
|---------|----------|--------------|----------|---------------|
| `TmuxHarness` | yes | SGR, OSC8, plain text; chords via `send_key` | Default L2 | Kitty kb-protocol; graphics; truecolor nuance |
| `WezTermHarness` | needs running WezTerm | SGR, OSC8, images, truecolor, `pane_size()` | SGR/OSC8/image rendering | Anything when WezTerm is not the host (see gotcha) |
| `KittyHarness` | needs running Kitty | SGR, OSC8, Kitty graphics protocol, `pane_cols()` | Kitty graphics tests | Anything when not pointed at a Kitty session |
| `AppleTerminalHarness` | macOS GUI | Low: no images, no OSC8, single underline | Degradation on weak terminal | Byte-level negatives (`raw == plain`) |

## The `TerminalHarness` contract

| Method | Purpose |
|--------|---------|
| `spawn_shell()` | Fresh login shell, augments `PATH`, forces color, waits for prompt. |
| `send_text(&[u8])` | Raw bytes to pane stdin (escape sequences, kitty kb-protocol). |
| `send_command_with_env(cmd, env)` | Type one command line with scoped `KEY=value` prefixes (provided). |
| `capture()` | `CapturedFrame { raw, plain }`. `raw` keeps escapes; `plain` is stripped. |
| `settle()` | Sleep for one redraw (default 200 ms, provided). |
| `spawn_program(prog, args)` | Escape hatch — no shell, no `PATH`, no color forcing. |

**Use `spawn_shell` by default.** `spawn_program` is for tests that must
control the process tree (no shell wrapper, absolute path required).

Assert *styling* on `raw`; assert *visible text* on `plain`.

## `available()` and skip-clean

Every L2/L3 test **must** check `Harness::available()` first and early-return
via `skip_with_reason("<X>")` when it returns `false`. No `#[ignore]`.

| Harness | `available()` requires |
|---------|------------------------|
| `TmuxHarness` | `tmux` on `$PATH`. Nothing else. |
| `WezTermHarness` | `wezterm` on `$PATH` **and** `WEZTERM_UNIX_SOCKET` set. |
| `KittyHarness` | `kitty` on `$PATH` **and** `KITTY_LISTEN_ON` set. |
| `AppleTerminalHarness` | macOS, `CI != 1`, `osascript` can reach Terminal.app. |
| `cliclick` (L3) | `cliclick` on `$PATH` (macOS only). |

`BISCUIT_TEST_LEVEL_REQUIRED=2` (or `3`) flips skips into hard failures — set
on hosts where the tooling *must* be present.

## The "which terminal am I running inside" gotcha

`WEZTERM_UNIX_SOCKET` and `KITTY_LISTEN_ON` are exported by their
terminal **only to processes the terminal itself launches**. Symmetric
consequence:

- `cargo test` from **WezTerm** → WezTerm tests run, Kitty tests skip.
- `cargo test` from **Kitty** → Kitty tests run, WezTerm tests skip.

A skip is usually *not a defect* — that terminal was not the host.
`TmuxHarness` is the only L2 backend immune to this.

### Cold-start the *other* terminal

WezTerm (mux server persists independent of any window):

```bash
export WEZTERM_UNIX_SOCKET="$(ls "$HOME/.local/share/wezterm/gui-sock-"* | head -1)"
cargo test -p biscuit-terminal-cli --test level2_prose_styling
```

Kitty (no daemon; start an instance with remote control):

```bash
kitty -o allow_remote_control=yes --listen-on unix:/tmp/kitty-l2 --start-as=minimized &
sleep 2
KITTY_LISTEN_ON=unix:/tmp/kitty-l2 cargo test -p biscuit-terminal-cli --test level2_prose_styling
```

## `SpawnVisibility` — default is `Background`

Applies to `WezTermHarness` and `KittyHarness` only.

| Visibility | WezTerm | Kitty | When to use |
|------------|---------|-------|-------------|
| `Background` *(default)* | Spawns into the `biscuit-bg` workspace (off-screen). | Passes `--keep-focus`. | Almost always. Any test that drives via the terminal's CLI. |
| `Foreground` | No workspace override. | No `--keep-focus`. | **Only L3** — OS keyboard injection needs the window focused. Pair with `WezTermHarness::focus_spawned_pane()`. |

```rust
use biscuit_test_harness::SpawnVisibility;
let harness = WezTermHarness::new()
    .with_spawn_visibility(SpawnVisibility::Foreground);
```

`TmuxHarness` has no visibility knob (detached sessions). `AppleTerminalHarness`
always snapshot-and-restores frontmost focus around its `do script` call.

## `SharedHarness<T>` — amortise the 2-3 s spawn cost

A test binary with many `#[serial]` tests against the same backend
should share one harness:

```rust
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
    // ... drive harness ...
}
```

The helper wraps `Mutex<Option<T>>` + `libc::atexit` so `Drop` runs at
process exit (Rust does not run `Drop` on `static` values — otherwise
the pane leaks).

### `just test-l2` and `biscuit-harness-broker`

`just test-l2` pre-spawns one shared pane per backend via the
`biscuit-harness-broker` binary, exports `BISCUIT_SHARED_<BACKEND>_ID`,
runs nextest with `-j 1`, and tears the panes down in a trap. Tests use
`<Backend>Harness::shared_or_spawn()` to attach to that pane and fall
back to per-process spawning when the env var is missing.

**Parallel self-spawn mode (`BISCUIT_L2_THREADS`).** The shared-pane +
`-j 1` model serializes the *entire* tier for the sake of whichever tests
attach to the shared pane. An area whose L2 suite is dominated by
self-isolating tests (each spawns its OWN session/PTY — e.g. claudine)
sets `BISCUIT_L2_THREADS=N` (claudine uses `min(cores, 8)`): `_test_l2`
then skips the broker, exports no `BISCUIT_SHARED_*`, and runs nextest at
`-j N`. With the env var unset, every `shared_or_spawn()` takes its
fallback branch and spawns an *owned*, `Drop`-cleaned pane, so there is
no shared resource to contend for. Default (`1`/unset) keeps the serial
shared-pane path. Backstop: claudine-cli L2 carries a 1 s leak-grace
override in `.config/nextest.toml` for concurrent child teardown.

## Level 3 — `cliclick`

`src/cliclick.rs` wraps the `cliclick` Homebrew utility to synthesize
real macOS Quartz keyboard events — the **only** way to verify
"what bytes does the terminal emit when the user presses bare Ctrl?".

Rules:

- Window must be **focused** — `SpawnVisibility::Foreground` +
  `focus_spawned_pane()`.
- macOS focus is a global resource — gate behind `RUN_LEVEL3=1` and run
  with `--test-threads=1`.
- **Bare-modifier press** is structurally unreliable via `cliclick`.
  Chord injection (`Ctrl+R`) works; bare modifiers need an L2
  raw-kitty-bytes test instead.
- Linux callers would need `xdotool` (not implemented).

### Never steal focus outside a `level3_` file

`SpawnVisibility::Foreground` and `focus_spawned_pane()` raise a GUI
terminal over whatever the user is doing. The `level3_` filename prefix
is what keeps them behind the `level3_` nextest filterset **and** the
`RUN_LEVEL3=1` opt-in — `just test` and `just test-l2` exclude
`test(/level3_/)`, so a focus-stealing test in any other file runs
unannounced on someone's desktop.

Claudine enforces this at L1 via
`cli/tests/test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files`,
which scans comment-stripped test sources. Copy that guard into other
areas that grow L3 coverage.

Corollary trap: **a test *name* must not contain `level3_`** unless the
test really is L3 — the filtersets match by substring, so a name like
`focus_apis_stay_in_level3_files` silently excludes itself from every
recipe that would run it.

`just test-l3` also **refuses to start unattended**: with no TTY it
exits non-zero unless `BISCUIT_L3_TAKE_FOCUS=1` authorizes the run, so
an agent, hook, or CI job cannot hijack an active desktop session. From
a terminal it prompts first. Never set that variable on an agent's
behalf — it exists for a human who knows the machine is free.

If a focus-stealing test cannot be made to pass reliably, fix the
*harness*, not the test. Two changes turned Claudine's L3 sequence
Ctrl+C test from never-green to reliable, and both are the kind of
thing to reach for first:

- poll until the terminal is genuinely frontmost (`AXRaise` returns
  before the WindowServer has made the window key) rather than sleeping
  a fixed interval and hoping;
- send modified chords via AppleScript `keystroke … using <mod> down`,
  which carries the flag on the same key event. cliclick's
  `kd:ctrl t:c ku:ctrl` emits the modifier and the letter as *separate*
  events, so they race — measured 7/10 delivery versus 24/24.

## Defensive cleanup

Panes/sessions/windows are tagged with the creating pid; later runs
close only tagged resources whose process is no longer alive — active
concurrent runs are left alone.

- `cleanup_stale_terminal_harness_resources()` is callable for explicit
  suite setup or local maintenance.
- Each backend spawn calls its own cleanup path once per process.
- Legacy untagged WezTerm panes in `biscuit-bg` are swept when that
  workspace grows past a conservative limit, or when
  `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM=1`.

## Concurrency

Real-terminal tests share OS-global resources (focus, window lists).
Serialise with `serial_test` — e.g. `#[serial(level2_terminal)]`.
`AppleTerminalHarness` **requires** serialisation (single global
AppleScript state).

## Re-exports worth knowing

| Item | Purpose |
|------|---------|
| `CapturedFrame` | `raw` + `plain`. |
| `SpawnVisibility` | `Background` (default) / `Foreground`. |
| `strip_ansi(&str)` | ECMA-48 §5.4-aware CSI/OSC/SGR/charset-designation stripper. |
| `skip_with_reason(&str)` | `skipping: requires <X>` to stderr; returns `true`. |
| `cleanup_stale_terminal_harness_resources()` | Best-effort tagged-resource sweep. |
| `detect_shell()` | POSIX shell pick (`bash` → `sh` → `$SHELL`). |
| `cargo_bin_dir(name)` | Locate a built cargo binary's directory for `PATH` augmentation. |
| `apply_color_forcing_env(&mut Command)` | Set `FORCE_COLOR`/`CLICOLOR_FORCE`/`TERM`/`COLORTERM`, clear `NO_COLOR`. |
| `wait_for_prompt(&mut harness)` | Poll `capture()` until `$`/`#`/`%` (5 s cap). |
| `run_with_timeout` / `run_with_stdin_timeout` | Wall-clock timeout with pipe-deadlock-safe draining. |
| `SPAWN_TIMEOUT`, `SEND_TIMEOUT`, `CAPTURE_TIMEOUT`, `QUERY_TIMEOUT`, `CLEANUP_TIMEOUT` | Default timeouts. |

## Cross-references

- `biscuit-test-harness/README.md` — full prose, including quick-start
  example and the complete `TerminalHarness` trait surface.
- `rust-testing` skill, `wezterm-harness-pitfalls.md` — assertion
  patterns against captured `frame.raw` (SGR re-emission collapses;
  semicolon vs ITU colon form). Load via the Skill tool.
- `rust-testing` skill, SKILL.md — L1/L2/L3 taxonomy, `require_level!`
  gating, canonical `just` recipes, `test-l2` mechanics.

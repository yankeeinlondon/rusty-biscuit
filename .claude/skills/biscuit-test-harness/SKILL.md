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
├── YES → Level 3 (RUN_LEVEL3=1). Linux: `xvfb::XvfbKitty` (private display,
│         never takes the user's focus). macOS: `cliclick` (foreground).
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

**The tmux server is the harness's own.** Sessions run on
`tmux -L biscuit-test-harness -f /dev/null`: no developer `tmux.conf`
(a plugin-manager config cost 2.1 s per server start), same defaults as
CI. A test's own `tmux` call against a harness session must use
`tmux::tmux_command()`; a bare `tmux` asks the default server.

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

### The pane's shell reads login files, not interactive ones

**Every** backend spawns an outer `-l` login shell (which is where
`PATH` is assembled) that `exec`s an interactive shell with its **rc
files suppressed** — `bash --norc -i`, `zsh -f -i`, `unset ENV` for
POSIX `sh`. The interactive rc is where Atuin, starship, fzf, and zoxide
install themselves; an Atuin first-run picker rendered into the pane
swallowed a sent command line and stalled a claudine L2 test until its
exit marker timed out. Consequences:

- Pane `PATH` is **login-profile only**. A `~/.bashrc` PATH edit does
  not reach the pane — pass an absolute path or an explicit `PATH` env
  pair.
- The prompt is the shell's stock `PS1` (`bash-5.3$`, `host%`). Never
  grep a captured frame for a literal the stock prompt contains.
- Never locate a command's output from its echo. tmux's bash shortens a
  long command line with a leading `<` instead of wrapping it, so the
  full echo (or a `bt <subcommand>` anchor) is missing. Print begin/end
  markers around the command, assembled by `printf` so the echo never
  spells them out; `biscuit-terminal/cli/tests/common/output_region.rs`
  is the worked example.
- The outer shell still runs the login profile, so a profile that
  sources an rc file unconditionally still executes it — but
  non-interactively, in a shell that is then replaced. Anything guarded
  on `$-` containing `i` never reaches the pane.
- The invocation is built once in `lib.rs` (`login_shell_script`,
  `login_shell_argv`, `login_shell_command_line`) so backends cannot
  drift. tmux needs `-e` to set pane environment: a variable set on the
  `tmux` client does not reach a session created against an
  already-running server.

Assert *styling* on `raw`; assert *visible text* on `plain`.

A shared pane's capture still holds earlier commands' output (a `clear` is
not proof), and tmux `capture-pane -e` re-emits only the attributes that
changed, carrying them across rows: a bold row under a bold heading may show
just `\e[39m`. Scope assertions to the command under test (begin/end markers)
and read a cell's style by replaying SGR from the capture's start; worked
example: `tree-hugger/cli/tests/support/terminal_report.rs`.

## `available()` and skip-clean

Every L2/L3 test **must** check `Harness::available()` first and early-return
via `skip_with_reason("<X>")` when it returns `false`. No `#[ignore]`.

| Harness | `available()` requires |
|---------|------------------------|
| `TmuxHarness` | `tmux` on `$PATH`. Nothing else. |
| `WezTermHarness` | `wezterm` on `$PATH`, `WEZTERM_UNIX_SOCKET` set, **and** `wezterm cli list` answering within 5 s. A later answer (a busy or hung GUI) counts as available only when `BISCUIT_TEST_REQUIRED_BACKENDS` lists `wezterm`, where "unavailable" would fail the test anyway. |
| `KittyHarness` | `kitty` on `$PATH` **and** `KITTY_LISTEN_ON` set. |
| `KittyInstance` (macOS) | `can_launch()`: macOS with `kitty`, `open`, `screencapture`. No host Kitty session needed. |
| `AppleTerminalHarness` | macOS, `CI` not truthy, `osascript` can reach Terminal.app. |
| `cliclick` (L3, macOS) | `cliclick` on `$PATH` — checks neither platform nor permission. Gate *additionally* on `cliclick::accessibility_trusted()`, which covers both. |
| `xdotool` (L3, Linux) | Linux, `xdotool` on `$PATH`, **and** `DISPLAY` set. Wayland reports unavailable and skips. |
| `win_input` (L3, Windows) | Windows and a working `powershell`. |
| `XvfbKitty` (L3, Linux) | `can_launch()`: `Xvfb` and `kitty` on `$PATH`. No `DISPLAY` or window manager. |

`BISCUIT_TEST_LEVEL_REQUIRED=2` (or `3`) flips skips into hard failures, but
all-or-nothing: it also panics the GUI backends a headless runner cannot host.
Prefer **`BISCUIT_TEST_REQUIRED_BACKENDS`** for L2 — a comma-separated,
case-insensitive list of the stable identifiers `tmux`, `wezterm`, `kitty`,
`apple-terminal`. Named backends hard-fail when unavailable; every other backend
still skips cleanly. Identifiers match exactly (`wez` is an error, not a
near-miss), and the same spellings appear in each package's
`[package.metadata.ci.tests]` `l2-backends`.

```bash
BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2   # tmux fatal, GUI backends skip
```

Naming a backend requires the gate to carry that backend's identity, so pass a
`Backend` rather than a bare label:

```rust
use test_toolkit::{require_level, Backend, Level};

require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
```

A plain string label still works, and remains correct for composite or
non-backend requirements — `"PTY (/dev/ptmx)"`, `"WezTerm + cliclick"` — where
no single backend applies. Those gates contribute no execution evidence by
design and can neither satisfy nor block the variable.

**Availability is not execution.** A host with `tmux` installed and zero
tmux-backed tests selected still exits 0, having verified nothing. When
`BISCUIT_TEST_REQUIRED_BACKENDS` is set, each gate therefore appends a
`{backend, test, decision}` record to `$STAGE/backend-executions.jsonl`, and
`just test-l2` brackets the tier with `backend-proof reset` / `backend-proof
verify` (`tools/test-toolkit`, `--features backend-proof`) so a required backend
that ran no test fails the tier. Recording is off — no file I/O at all — when
the variable is unset. See the `rust-testing` skill for the full contract.

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
cargo test -p biscuit-terminal-cli --features terminal-tests --test level2 level2_prose_styling::
```

Kitty, per test and without any host session: `KittyInstance::launch(cols,
lines)` (macOS) starts a private Kitty through `open -g` at exactly that
cell size, never focused, and `screenshot()` captures what it drew via
`screencapture -l`. Its window is visible (a `--start-as=hidden` window is
never drawn: black screenshots), and Kitty does not render it while another
window covers it: empty screenshots, so a pixel test must skip visibly or
fail on an empty capture, never pass. `open` hands the pane the test's env, so
unset the host's `TERM_PROGRAM` before detection-sensitive programs, and
`get-text` returns soft-wrapped lines whole. Details and the pixel geometry:
`biscuit-test-harness/README.md`, "A private Kitty per test"; worked example:
`worktree/cli/tests/level2_graph_in_kitty.rs`.

Kitty against the host's own instance (no daemon; start one with remote control):

```bash
kitty -o allow_remote_control=yes --listen-on unix:/tmp/kitty-l2 --start-as=minimized &
sleep 2
KITTY_LISTEN_ON=unix:/tmp/kitty-l2 cargo test -p biscuit-terminal-cli --features terminal-tests --test level2 level2_prose_styling::
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
opens its window with `open -g` and restores frontmost focus if the spawn
displaced it.

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

**The Apple Terminal window is spawned only on request.** Terminal.app has
no background mode, so its shared window sits visibly on the developer's
desktop for the whole run. `_test_l2` spawns it only when the package's
`[package.metadata.ci.tests] l2-backends` lists `apple-terminal` (today:
`biscuit-terminal-cli`). Before 2026-10-04 every package's L2 run, including
every pre-push `ci-local` L2 cell, opened one, which developers saw as "tests
popping up Terminal windows". A package that adds an Apple Terminal test must
declare the backend, or each test self-spawns its own window.

**Parallel self-spawn mode is the default.** `_test_l2` skips the broker,
exports no `BISCUIT_SHARED_*`, and runs nextest at `-j N`, so every
`shared_or_spawn()` takes its fallback branch and spawns an *owned*,
`Drop`-cleaned pane: no pane is shared, but the WezTerm mux is (see "WezTerm
clients take turns" below).
N is `_test_l2_threads`: `max(1, floor(logical_cores / 2))` on every host, CI
runners included, because every worker drives a real terminal and one WezTerm
mux server serves them all; at 14 workers on a 16-core Mac its spawn and
capture calls timed out until the harness made WezTerm clients take turns. The
count follows the host, not the OS: that Mac's
pre-push run gets 8, a GitHub-hosted `ubuntu-latest` runner (4 cores) 2, and a
GitHub-hosted `macos-latest` runner (3 cores) 1. It controls worker count, not
CPU affinity or a guaranteed reservation.

**Serial is the rare exception and must say why.** A package whose L2 tests
genuinely contend for a resource declares it in its manifest, and the CI
planner rejects the marker without a non-empty reason:

```toml
[package.metadata.ci.tests]
runner-tools = ["l2-serial"]
l2-serial-reason = "every test reads and writes the one system clipboard"
```

That package gets the shared-pane `-j 1` path above. `BISCUIT_L2_THREADS`
overrides both (`1` forces serial). Backstop: claudine-cli L2 carries a 1 s
leak-grace override in `.config/nextest.toml` for concurrent child teardown.

### WezTerm clients take turns

The WezTerm GUI serves every `wezterm cli` client on its main thread and,
while one is connected, pushes it render changes for every busy pane. Many
clients at once collapse it: 16 parallel tests' clients took 13–40 s each
(2026-10-10), surfacing as `WezTerm not available` or a 15 s spawn timeout
on a *different set of tests each run*, each of which passes alone. A sample
of `wezterm-gui` (`sample <pid> 5`) during the stall shows the main thread in
`GetPaneRenderChangesResponse` encoding. The harness therefore:

- runs at most **two** clients per mux at a time, across all processes and
  worktrees, in arrival order (lock files
  `biscuit-wezterm-cli-<socket>-{0,1}.lock` and a `-queue` directory in the
  temporary directory; random order left a 1–2 s tail wait that pushed
  single tests past 5 s). A waiter holds an OS lock on its queue entry, so
  a crashed or killed waiter's entry is detected and removed on every OS;
- runs every client with `wezterm --skip-config` (the GUI exports
  `WEZTERM_CONFIG_FILE` into its panes, so honoring it evaluated the
  developer's whole config on every `spawn`);
- reuses an answered `available()` probe for 2 s and sweeps stale panes at
  most once per 30 s per mux, since `list` is the heaviest call.

A test that shells out to `wezterm cli` itself bypasses the turns; go
through the harness. Never raise a WezTerm timeout to "fix" this symptom.

## Level 3 — OS keyboard injection

One injector module per platform, each with its own `available()` that
skips cleanly off its platform. Level 3 is the **only** way to verify
"what bytes does the terminal emit when the user presses this chord?".

| Module | Platform | Mechanism |
|--------|----------|-----------|
| `src/cliclick.rs` | macOS | `cliclick` (Quartz `CGEvent`), plus System Events for modified chords |
| `src/xdotool.rs` | Linux / X11 | `xdotool key` via the XTEST extension |
| `src/win_input.rs` | Windows | PowerShell driving `SendKeys.SendWait` |
| `src/xvfb.rs` | Linux | XTEST via `x11rb` on a **private** `Xvfb` display with its own kitty |

Window selection is by **unique** title match on all three: zero or
several matches is an error, never a first-match guess — injecting
Ctrl+C into the wrong window looks identical to a broken product.

Rules:

- Window must be **focused** — `SpawnVisibility::Foreground` +
  `focus_spawned_pane()`.
- macOS focus is a global resource — gate behind `RUN_LEVEL3=1` and run
  with `--test-threads=1`.
- **Bare-modifier press** is structurally unreliable via `cliclick`.
  Chord injection (`Ctrl+R`) works; bare modifiers need an L2
  raw-kitty-bytes test instead.
- `xdotool` must never be passed `--window` — that switches it from
  XTEST to `XSendEvent`, whose flagged events terminals ignore as
  untrusted. Focus the window first, then inject globally.

### A key press without the user's focus

On Linux, `xvfb::XvfbKitty` gives a real OS key press without touching
the user's desktop: it starts its own `Xvfb`, runs kitty there, focuses
kitty *on that display* (`SetInputFocus`; no window manager needed), and
presses through XTEST. `harness()` returns a `KittyHarness` for typing and
capture; `press_ctrl('c')` and `press_escape()` are the keys. It needs no `BISCUIT_L3_TAKE_FOCUS`,
but it is still L3 (`level3_` name, `RUN_LEVEL3=1`) because the press is
an OS key event. Example: `claudine/cli/tests/level3/level3_drain_ctrl_c.rs`.

macOS and Windows have no such private desktop. macOS hands key events
to the key window of the one login session only; a Ctrl+C posted to an
unfocused kitty's pid with `CGEventPostToPid` never reached its pane
(measured 2026-09-27). Windows `SendInput` reaches only the input
desktop, the one the user sees. There, the closest no-focus press is
`KittyHarness::send_key("ctrl+c")` on a `KittyInstance`: kitty encodes the
key for the program's keyboard mode, as it does a physical press, and
never raises the window. The OS input layer ahead of kitty is skipped, so
that is **Level 2** evidence and belongs in a `level2_` file. Example:
`claudine/cli/tests/level2/level2_drain_ctrl_c_kitty.rs`.

### Never steal focus outside a `level3_` file

`SpawnVisibility::Foreground` and `focus_spawned_pane()` raise a GUI
terminal over whatever the user is doing. The `level3_` filename prefix
is what keeps them behind the `level3_` nextest filterset **and** the
`RUN_LEVEL3=1` opt-in — `just test` and `just test-l2` exclude
`test(/level3_/)`, so a focus-stealing test in any other file runs
unannounced on someone's desktop.

Claudine enforces this at L1 via
`cli/tests/l1/test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files`,
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

Panes/sessions/windows are tagged with an owning pid; later runs close
only tagged resources whose process is no longer alive — active
concurrent runs are left alone.

The owning pid is the spawning process unless `BISCUIT_HARNESS_OWNER_PID`
names a live process, in which case that pid is stamped instead. `_test_l2`
exports its own pid before the broker spawns: the broker exits as soon as it
has spawned, so a pane tagged with the broker's pid is "dead-owner" from the
first test on, and any reaper on the host — a second L2 run in another
worktree, say — killed the shared session under the tests. The symptom was
`tmux send-keys failed` on the first send of a run that passes when rerun
alone (2026-09-18, sniff-detection-cli). The env-var guard on the shared session name
still applies inside the run; the owner pid is what protects it from
everyone else.

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
| `bin_exe!("name")` | `PathBuf` of a workspace binary, resolved at run time so an archived run (the `wsl2-ubuntu` leg) finds it. Never spawn `env!("CARGO_BIN_EXE_…")` directly. |
| `apply_color_forcing_env(&mut Command)` | Set `FORCE_COLOR`/`CLICOLOR_FORCE`/`TERM`/`COLORTERM`, clear `NO_COLOR`. |
| `wait_for_prompt(&mut harness)` | Poll `capture()` until `$`/`#`/`%` (5 s cap). |
| `send_command_settled(&mut harness, cmd, env)` | Type a command line, wait for the pane to show it, return the settled frame. Use instead of `send_command_with_env` + `capture_settled`, which sleeps a fixed `settle()` first. |
| `run_with_timeout` / `run_with_stdin_timeout` | Wall-clock timeout with pipe-deadlock-safe draining. |
| `SPAWN_TIMEOUT`, `SEND_TIMEOUT`, `CAPTURE_TIMEOUT`, `QUERY_TIMEOUT`, `CLEANUP_TIMEOUT` | Default timeouts. |

## Cross-references

- `biscuit-test-harness/README.md` — full prose, including quick-start
  example and the complete `TerminalHarness` trait surface.
- `rust-testing` skill, `wezterm-harness-pitfalls.md` — assertion
  patterns against captured `frame.raw` (SGR re-emission collapses;
  semicolon vs ITU colon form). Load via the Skill tool.
- `rust-testing` skill, SKILL.md — L1/L2/L3 taxonomy, `require_level!`
  gating, canonical `just` recipes; `l2-tests.md` for `test-l2` mechanics.

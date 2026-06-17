//! `biscuit-harness-broker` — pre-spawn shared terminal harnesses for
//! the `_test_l2` recipe.
//!
//! `cargo nextest run` executes each test in its own child process, so
//! a process-local `SharedHarness` static cannot amortize the 2-3 s
//! cost of spawning a real terminal pane across tests. This binary
//! moves the spawn / teardown out to the recipe layer:
//!
//! 1. The `_test_l2` recipe runs `biscuit-harness-broker spawn <backend>`
//!    before nextest. The broker spawns one pane / window / session in
//!    the requested backend, prints the resource id to stdout, then
//!    leaks the harness (via [`std::mem::forget`]) so [`Drop`] does not
//!    tear the resource down when the broker process exits.
//! 2. The recipe captures the printed id into an environment variable
//!    (`BISCUIT_SHARED_WEZTERM_PANE_ID`, etc.) and runs nextest with
//!    that env exported. Each nextest child sees the env var and uses
//!    `<Backend>Harness::shared_or_spawn()` to attach to the existing
//!    pane instead of spawning a new one. Attach-mode harnesses do
//!    nothing in `Drop`.
//! 3. After nextest exits (success or failure), the recipe runs
//!    `biscuit-harness-broker kill <backend> <id>` to tear the pane
//!    down via the backend's `*_by_id` cleanup helper.
//!
//! Exit codes:
//!
//! - `0` — spawn or kill succeeded.
//! - `2` — backend tooling unavailable on this host (e.g. no WezTerm
//!   socket). Caller should silently skip the backend; tests fall back
//!   to per-process spawning, which themselves skip cleanly via
//!   `require_level!` when the same tooling is missing.
//! - `1` — invocation error (bad args, unknown backend, spawn failed
//!   despite tooling being available).

use std::env;
use std::process::ExitCode;

use biscuit_test_harness::SpawnVisibility;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::apple_terminal::{self, AppleTerminalHarness};
use biscuit_test_harness::kitty::{self, KittyHarness};
use biscuit_test_harness::tmux::{self, TmuxHarness};
use biscuit_test_harness::wezterm::{self, WezTermHarness};

const USAGE: &str = "\
biscuit-harness-broker — pre-spawn shared terminal harnesses

USAGE:
  biscuit-harness-broker spawn <backend>
  biscuit-harness-broker kill  <backend> <id>

BACKENDS:
  wezterm          Spawn / kill a WezTerm pane (BISCUIT_SHARED_WEZTERM_PANE_ID).
  kitty            Spawn / kill a kitty window (BISCUIT_SHARED_KITTY_WINDOW_ID).
  tmux             Spawn / kill a tmux session  (BISCUIT_SHARED_TMUX_SESSION).
  apple-terminal   Spawn / kill a Terminal.app window
                   (BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID).

EXIT CODES:
  0   success
  2   backend unavailable (caller should skip)
  1   invocation error\n";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprint!("{USAGE}");
        return ExitCode::from(1);
    }
    match args[0].as_str() {
        "spawn" if args.len() == 2 => spawn(&args[1]),
        "kill" if args.len() == 3 => kill(&args[1], &args[2]),
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(1)
        }
    }
}

fn spawn(backend: &str) -> ExitCode {
    match backend {
        "wezterm" => spawn_wezterm(),
        "kitty" => spawn_kitty(),
        "tmux" => spawn_tmux(),
        "apple-terminal" => spawn_apple_terminal(),
        other => {
            eprintln!("biscuit-harness-broker: unknown backend {other:?}");
            ExitCode::from(1)
        }
    }
}

fn kill(backend: &str, id: &str) -> ExitCode {
    match backend {
        "wezterm" => {
            wezterm::kill_pane_by_id(id);
            ExitCode::SUCCESS
        }
        "kitty" => {
            kitty::close_window_by_id(id);
            ExitCode::SUCCESS
        }
        "tmux" => {
            tmux::kill_session_by_name(id);
            ExitCode::SUCCESS
        }
        "apple-terminal" => match id.parse::<i64>() {
            Ok(parsed) => {
                apple_terminal::close_window_by_id(parsed);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("biscuit-harness-broker: apple-terminal id {id:?} is not an i64: {e}");
                ExitCode::from(1)
            }
        },
        other => {
            eprintln!("biscuit-harness-broker: unknown backend {other:?}");
            ExitCode::from(1)
        }
    }
}

fn spawn_wezterm() -> ExitCode {
    if !WezTermHarness::available() {
        eprintln!("biscuit-harness-broker: wezterm unavailable (no WEZTERM_UNIX_SOCKET or binary)");
        return ExitCode::from(2);
    }
    // INVARIANT: broker-spawned panes MUST be background. Test runs
    // routinely happen while a developer is working in another window,
    // and a foreground spawn would steal focus / interrupt the active
    // desktop. L3 tests that need foreground spawn their own
    // (per-test) harness; the shared L2 pane stays out of the way.
    let mut h = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Background);
    if let Err(e) = h.spawn_shell() {
        eprintln!("biscuit-harness-broker: wezterm spawn_shell failed: {e}");
        return ExitCode::from(1);
    }
    println!("{}", h.pane_id_str());
    std::mem::forget(h);
    ExitCode::SUCCESS
}

fn spawn_kitty() -> ExitCode {
    if !KittyHarness::available() {
        eprintln!("biscuit-harness-broker: kitty unavailable (no KITTY_LISTEN_ON or binary)");
        return ExitCode::from(2);
    }
    // INVARIANT: see `spawn_wezterm` — broker spawns are always
    // background so the developer's foreground app keeps focus.
    let mut h = KittyHarness::new().with_spawn_visibility(SpawnVisibility::Background);
    if let Err(e) = h.spawn_shell() {
        eprintln!("biscuit-harness-broker: kitty spawn_shell failed: {e}");
        return ExitCode::from(1);
    }
    println!("{}", h.window_id_str());
    std::mem::forget(h);
    ExitCode::SUCCESS
}

fn spawn_tmux() -> ExitCode {
    if !TmuxHarness::available() {
        eprintln!("biscuit-harness-broker: tmux unavailable (binary missing from PATH)");
        return ExitCode::from(2);
    }
    let mut h = TmuxHarness::new();
    if let Err(e) = h.spawn_shell() {
        eprintln!("biscuit-harness-broker: tmux spawn_shell failed: {e}");
        return ExitCode::from(1);
    }
    println!("{}", h.session_name());
    std::mem::forget(h);
    ExitCode::SUCCESS
}

fn spawn_apple_terminal() -> ExitCode {
    if !AppleTerminalHarness::available() {
        eprintln!(
            "biscuit-harness-broker: apple-terminal unavailable (not macOS, in CI, or osascript missing)"
        );
        return ExitCode::from(2);
    }
    // `preserve_capabilities(true)` matches the only current consumer
    // (`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`),
    // which exercises Prose graceful-degradation paths and therefore
    // wants `bt`'s natural capability detection against Terminal.app's
    // unforced colour profile. Image / colour tests that need
    // FORCE_COLOR can scope it per command via
    // `TerminalHarness::send_command_with_env`.
    let mut h = AppleTerminalHarness::new().preserve_capabilities(true);
    if let Err(e) = h.spawn_shell() {
        eprintln!("biscuit-harness-broker: apple-terminal spawn_shell failed: {e}");
        return ExitCode::from(1);
    }
    let id = h.spawned_window_id().expect("spawn_shell set window id");
    println!("{id}");
    std::mem::forget(h);
    ExitCode::SUCCESS
}

//! Real-terminal rendering tests.
//!
//! The harness contract, the Level-1 / Level-2 / Level-3 vocabulary, the
//! harness variants (and when to use each), and the environment each
//! requires are documented in the shared harness crate's README:
//! `biscuit-test-harness/README.md`. `cli/tests/common/real_terminal/mod.rs`
//! is a thin re-export shim over that crate. The tests in this file are
//! deliberately written so that:
//!
//! - they **skip cleanly** (printing `skipping: requires <tool>`) when
//!   the host lacks the required terminal — no `#[ignore]` markers,
//!   no spurious failures on CI or contributor laptops;
//! - they **actually run** when the prerequisite IS available, so
//!   `cargo test` on a developer machine that has WezTerm/Kitty/tmux
//!   installed exercises the real path. This is what the user
//!   asked for: "if the host has the terminal then the test is run."
//!
//! ## Shared harness reuse
//!
//! Level-2 tests in this file reuse a single shell pane per terminal
//! backend. Each test sends `clear` followed by the `question` binary
//! as a shell command, captures the rendered output (and/or sends
//! keys), then terminates the question process with `Ctrl+C` so the
//! shell returns to a prompt before the next test runs.
//!
//! Spawning a fresh WezTerm/Kitty/tmux pane costs 2–3 seconds. Under
//! `cargo test` (single-process, libtest), the per-binary
//! [`SharedHarness`] static amortizes that cost across every test in
//! this file. Under `just test-l2`/`cargo nextest run` (process-per-
//! test), `biscuit-harness-broker spawn` pre-spawns one pane per
//! backend before nextest starts, exports its id via
//! `BISCUIT_SHARED_*_ID`, and the per-test
//! `<Backend>Harness::shared_or_spawn()` calls attach to the existing
//! pane instead of spawning fresh. The `#[serial(level2)]` attribute
//! serialises tests within a single process so they never contend for
//! the shared pane; the recipe runs nextest with `-j 1` for the same
//! reason across processes.
//!
//! Every test here spawns in the background ([`SpawnVisibility::Background`],
//! the harness default) so a test run never steals desktop focus. These
//! headless byte-injection tests are the always-on contract: keyboard-driven
//! behavior (navigation, Ctrl/Alt chord selection, relaxed Ctrl/Alt+Shift
//! matching) is verified by feeding the equivalent terminal byte sequences into
//! a headless tmux/WezTerm pane — see the keyboard-driven tests near the end of
//! this file.
//!
//! Physical-keypress (OS keyboard injection / cliclick) proof of the relaxed
//! Ctrl+Shift / Alt+Shift chords lives in `level3_chord_select.rs`. Those
//! Level-3 tests require a focused GUI window and steal desktop focus, so they
//! are gated behind `RUN_LEVEL3=1` (via `just test-l3`) and never run in normal
//! CI/dev. This file's L2 byte-injection tests prove the binary decodes the
//! chord bytes; the L3 file proves a real terminal emits those bytes for a
//! physical chord.

#![cfg(unix)]

#[path = "common/mod.rs"]
mod common;

use std::time::Duration;

use biscuit_test_harness::shared::SharedHarness;
use common::real_terminal::{
    TerminalHarness, kitty::KittyHarness, tmux::TmuxHarness, wezterm::WezTermHarness,
};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

fn question_binary() -> String {
    assert_cmd::cargo::cargo_bin!("question")
        .to_str()
        .expect("question binary path is utf-8")
        .to_string()
}

/// Quotes a single argument for safe inclusion in a `sh`/`bash`
/// command line. Wraps in single quotes and escapes any embedded
/// single-quote characters.
fn sh_quote(arg: &str) -> String {
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('\'');
    for ch in arg.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Builds a shell command string for the `question` binary plus
/// arguments, terminating with a newline so the shell executes it.
fn question_command(args: &[&str]) -> String {
    let mut cmd = sh_quote(&question_binary());
    for a in args {
        cmd.push(' ');
        cmd.push_str(&sh_quote(a));
    }
    cmd.push('\n');
    cmd
}

/// How long to wait after launching the `question` binary in a shared
/// pane before capturing. The TUI takes a few hundred milliseconds to
/// alt-screen and render its first frame.
const QUESTION_RENDER_MS: u64 = 800;

/// Sends `Ctrl+C` to whatever is currently running in `harness` so the
/// shared pane returns to a shell prompt before the next test runs.
///
/// `byte 0x03` is the literal Ctrl-C control byte; both WezTerm and
/// Kitty accept it through `send_text`. For tmux, [`tmux_cleanup`]
/// routes through `send-keys C-c` to avoid argument-encoding issues.
fn cleanup_via_ctrl_c<H: TerminalHarness>(harness: &mut H) {
    let _ = harness.send_text(b"\x03");
    std::thread::sleep(Duration::from_millis(250));
}

/// Like [`cleanup_via_ctrl_c`] but uses tmux's symbolic key path so the
/// chord is routed through tmux's translator (tmux's `send-keys -l`
/// path does not accept raw control bytes).
fn tmux_cleanup(harness: &mut TmuxHarness) {
    let _ = harness.send_key("C-c");
    std::thread::sleep(Duration::from_millis(250));
}

// ---------------------------------------------------------------------------
// Process-shared shell panes — one per backend.
//
// `SharedHarness` constructs each pane lazily on first use and registers
// an atexit hook so the underlying WezTerm/Kitty/tmux session is torn
// down when the test binary exits.
// ---------------------------------------------------------------------------

static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

// ---------------------------------------------------------------------------
// Level 2 — render-in-real-terminal smoke tests
//
// Each launches `question` in the shared shell pane for its backend,
// captures the rendered output, asserts that option labels appear, and
// terminates the binary so the next test starts from a clean prompt.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
fn level2_wezterm_renders_option_labels() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(question_command(&["choose-one", "Red", "Green", "Blue"]).as_bytes())
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    let frame = harness.capture().expect("capture wezterm pane");
    cleanup_via_ctrl_c(harness);

    assert!(
        frame.plain.contains("Red")
            && frame.plain.contains("Green")
            && frame.plain.contains("Blue"),
        "expected option labels in wezterm pane; got: {:?}",
        frame.plain
    );
}

#[test]
#[serial(level2)]
fn level2_kitty_renders_option_labels() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(question_command(&["choose-one", "Red", "Green", "Blue"]).as_bytes())
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    let frame = harness.capture().expect("capture kitty window");
    cleanup_via_ctrl_c(harness);

    assert!(
        frame.plain.contains("Red")
            && frame.plain.contains("Green")
            && frame.plain.contains("Blue"),
        "expected option labels in kitty window; got: {:?}",
        frame.plain
    );
}

#[test]
#[serial(level2)]
fn level2_tmux_renders_option_labels() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(question_command(&["choose-one", "Red", "Green", "Blue"]).as_bytes())
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    let frame = harness.capture().expect("capture tmux pane");
    tmux_cleanup(harness);

    assert!(
        frame.plain.contains("Red")
            && frame.plain.contains("Green")
            && frame.plain.contains("Blue"),
        "expected option labels in tmux pane; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — `Ctrl+Space` portable badge toggle
//
// Verifies that sending the `Ctrl+Space` chord (as raw bytes) into the
// pane causes the renderer to emit `^R/^G/^B` badges. tmux is the
// most portable harness here — it handles chord input cleanly without
// depending on kitty keyboard protocol.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
fn level2_tmux_ctrl_space_reveals_badges() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    // Options with explicit hotkey prefixes — only options that opt
    // in via `[CTRL+x]` get badges.
    harness
        .send_text(
            question_command(&[
                "choose-one",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ])
            .as_bytes(),
        )
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys C-space` routes through tmux's key-name
    // translation. Sending the raw NUL byte via `send_text -l` is
    // rejected by `Command::arg` (no nul in process arguments).
    harness.send_key("C-space").expect("send Ctrl+Space");
    std::thread::sleep(Duration::from_millis(300));

    let frame = harness.capture().expect("capture tmux pane");
    tmux_cleanup(harness);

    let has_badge =
        frame.plain.contains("^R") || frame.plain.contains("^G") || frame.plain.contains("^B");
    assert!(
        has_badge,
        "Ctrl+Space MUST reveal at least one ^R/^G/^B badge in tmux pane; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — badge SGR styling (orange BG / black FG / bold for held Ctrl)
//
// Forces the Ctrl badge family into the held state via `--hotkey-badges
// ctrl`, captures the pane WITH escape sequences, and asserts the
// styling immediately preceding `^R` carries:
//   * palette-208 background (the held Ctrl orange)
//   * SGR `30` (black foreground) — the spec mandates black; white-on-
//     yellow is illegible, and we want a single consistent FG across
//     both badge families
//   * SGR `1` (bold) — the held badge's distinguishing attribute
//
// This is the strongest end-to-end colour verification we have. The
// PTY suite captures escapes too, but no other test asserts that the
// badge family + weight survives ratatui's diff renderer and the
// terminal's re-emission unchanged.
// ---------------------------------------------------------------------------

/// Returns true when `param` appears as a complete SGR token inside any
/// `\x1b[…m` sequence in `haystack`.
///
/// SGR parameters are `;`-separated, bounded by `[` (start) and `m`
/// (terminator). A loose substring match would falsely accept e.g. `1`
/// inside `38;5;1` (red palette index), so we require token boundaries.
fn sgr_param_present(haystack: &str, param: &str) -> bool {
    haystack.contains(&format!("\x1b[{param}m"))
        || haystack.contains(&format!("\x1b[{param};"))
        || haystack.contains(&format!(";{param}m"))
        || haystack.contains(&format!(";{param};"))
}

#[test]
#[serial(level2)]
fn level2_tmux_ctrl_held_badge_uses_orange_bold_black_sgr() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(
            question_command(&[
                "choose-one",
                "--hotkey-badges",
                "ctrl",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ])
            .as_bytes(),
        )
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    let frame = harness.capture().expect("capture tmux pane");
    tmux_cleanup(harness);

    // Guardrail: the badge text must actually be on screen before we
    // try to inspect its styling. If this fails the rest of the test's
    // diagnostics would be misleading.
    let badge_pos = frame.raw.find("^R").unwrap_or_else(|| {
        panic!(
            "expected `^R` Ctrl badge in pane after --hotkey-badges ctrl; \
             plain={:?}\nraw={:?}",
            frame.plain, frame.raw
        )
    });

    // Look at the styling window immediately before the badge text.
    // 200 bytes is generous: ratatui may emit fg/bg/bold as separate
    // CSIs, and tmux's capture-pane -e may interleave a charset
    // designator or two.
    let window_start = badge_pos.saturating_sub(200);
    let window = &frame.raw[window_start..badge_pos];

    assert!(
        window.contains("48;5;208"),
        "Ctrl-held badge `^R` must use palette-208 (orange) background. \
         window before badge={:?}; full raw={:?}",
        window,
        frame.raw,
    );
    assert!(
        sgr_param_present(window, "30") || window.contains("38;5;0"),
        "Ctrl-held badge `^R` must use BLACK foreground (SGR 30 or 38;5;0); \
         spec mandates black on both family colours. window={:?}",
        window,
    );
    assert!(
        sgr_param_present(window, "1"),
        "Ctrl-held badge `^R` must use bold (SGR 1). window={:?}",
        window,
    );
}

#[test]
#[serial(level2)]
fn level2_tmux_ctrl_c_exits_130() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    // Run the binary inline so the parent shell sees `$?` after it
    // exits via Ctrl+C. The shell prints `EXIT:<code>` on its own
    // line which we then assert on. We do NOT sleep after the printf
    // — the shared shell returns to its prompt naturally and the
    // next test starts with a `clear`.
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "{bin} choose-one Red Green Blue; printf '\\nEXIT:%s\\n' \"$?\"\n"
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    harness.send_key("C-c").expect("send Ctrl+C");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");
    // No further cleanup required: question exited from Ctrl+C and
    // the shell printed EXIT:130 then returned to a prompt.

    assert!(
        frame.plain.contains("EXIT:130"),
        "Ctrl+C from tmux MUST make question choose-one exit 130; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — bare-Ctrl kitty-byte handler (NOT a user-keypress proof)
//
// This test verifies a narrow internal contract: given the literal
// kitty escape sequence that WezTerm *would* emit for a bare-Ctrl
// press/release (with `REPORT_EVENT_TYPES` + `REPORT_ALL_KEYS_AS_ESCAPE_CODES`
// pushed), the binary correctly transitions between `CtrlHeld` /
// `Hidden` and surfaces the `^R` / `^G` / `^B` badge. We deliver the
// bytes through `wezterm cli send-text`, bypassing the OS keyboard
// entirely.
//
// What this Level-2 test proves: the binary's modifier-only handler
// and badge renderer behave correctly when fed the expected bytes.
//
// What this Level-2 test does NOT prove: that WezTerm emits those bytes
// when a user physically holds bare Ctrl. That path depends on (a) macOS
// dispatching a `flagsChanged` event, (b) WezTerm's keymap not intercepting
// Ctrl, and (c) `enable_kitty_keyboard = true` in the user's `wezterm.lua`.
// No userspace tool on macOS can synthesise the `flagsChanged` event needed
// to drive that path, so it cannot be verified by automated OS keyboard
// injection without flaky, focus-stealing results — see the
// **Production readiness** note in `biscuit-tui/docs/components/choose_one.md`.
// The byte-level contract this test asserts is biscuit-tui's actual
// responsibility; the terminal's physical-key encoder is not.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
#[cfg(target_os = "macos")]
fn level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(
            question_command(&[
                "choose-one",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ])
            .as_bytes(),
        )
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // Kitty keyboard-protocol bytes for "Left Ctrl press" — exactly
    // what WezTerm should emit when bare Ctrl is held under our flags.
    harness
        .send_text(b"\x1b[57442;1u")
        .expect("send kitty bare-Ctrl press");
    std::thread::sleep(Duration::from_millis(200));

    let frame = harness.capture().expect("capture during Ctrl 'hold'");

    // Send the release so the binary returns to Hidden state cleanly,
    // then Ctrl+C to exit the binary so the next test sees a clean
    // shell prompt.
    let _ = harness.send_text(b"\x1b[57442;1:3u");
    cleanup_via_ctrl_c(harness);

    let has_badge =
        frame.plain.contains("^R") || frame.plain.contains("^G") || frame.plain.contains("^B");
    if !has_badge {
        eprintln!("=== Level-2 capture (raw, with escapes) ===");
        eprintln!("{:?}", frame.raw);
        eprintln!("=== Level-2 capture (plain) ===");
        eprintln!("{:?}", frame.plain);
    }
    assert!(
        has_badge,
        "kitty bare-Ctrl bytes piped into a real WezTerm pane MUST surface \
         a ^R/^G/^B badge. If this fails, the binary's modifier-only handler \
         is broken or `REPORT_ALL_KEYS_AS_ESCAPE_CODES` regressed."
    );
}

// ---------------------------------------------------------------------------
// Level 2 — keyboard-driven navigation and chord selection (tmux, headless)
//
// These replace the former Level-3 cliclick tests. Arrow-down, Ctrl+R, and
// Alt+R are *legacy* terminal byte sequences (CSI `B`, the 0x12 control
// byte, and `ESC r` respectively) — a terminal emits them for those keys
// without any kitty keyboard-protocol push, so tmux's `send-keys`
// translator delivers them faithfully into a fully headless pane. That lets
// us verify the binary's end-to-end input handling (navigation moves the
// active marker; a chord selects + submits its option) WITHOUT spawning a
// focused GUI window. Only *bare* modifier presses (Ctrl held with no
// letter) need the kitty protocol, and that path is covered separately by
// `level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`.
//
// What is intentionally NOT covered here: whether a *physical* key press
// makes the terminal emit these bytes — that is the terminal's input
// encoder, not biscuit-tui's code. Verifying it requires OS keyboard
// injection (cliclick), which can only reach a focused window and so
// steals desktop focus and yields flaky results. Per the testing rubric we
// do not pay that cost to test a third party's encoder; the byte-level
// contract above is biscuit-tui's actual responsibility.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
fn level2_tmux_arrow_down_moves_active_marker() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    harness
        .send_text(question_command(&["choose-one", "Red", "Green", "Blue"]).as_bytes())
        .expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys Down` routes through tmux's key-name translation and
    // emits the plain cursor-down sequence the binary's navigation handler
    // consumes — no kitty protocol, no focus required.
    harness.send_key("Down").expect("send arrow-down");
    std::thread::sleep(Duration::from_millis(300));

    let frame = harness.capture().expect("capture tmux pane");
    tmux_cleanup(harness);

    let active_on_green = frame
        .plain
        .lines()
        .any(|l| l.contains('▶') && l.contains("Green"));
    assert!(
        active_on_green,
        "arrow-down MUST move the active marker from Red to Green in tmux; got: {:?}",
        frame.plain
    );
}

#[test]
#[serial(level2)]
fn level2_tmux_ctrl_r_chord_selects_red() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    // Capture the submitted value via command substitution. `question`
    // renders its TUI to stderr (still the tmux pane / a tty), so the
    // headless guard — which only trips when BOTH stdout and stderr are
    // piped — lets the prompt run while stdout is captured into `$out`.
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-one {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[CTRL+r] Red"),
        g = sh_quote("[CTRL+g] Green"),
        b = sh_quote("[CTRL+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys C-r` emits the 0x12 control byte — the legacy Ctrl+R
    // chord. The binary maps it to the `[CTRL+r]` hotkey, selects Red, and
    // submits; the shell then prints `PICK:Red` and returns to a prompt.
    harness.send_key("C-r").expect("send Ctrl+R");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");
    // No Ctrl+C cleanup: the chord submitted, so question already exited and
    // the shell is back at a prompt for the next test's `clear`.

    assert!(
        frame.plain.contains("PICK:Red"),
        "Ctrl+R MUST select + submit the [CTRL+r] Red option in tmux; got: {:?}",
        frame.plain
    );
}

#[test]
#[serial(level2)]
fn level2_tmux_alt_r_chord_selects_red() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-one {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[ALT+r] Red"),
        g = sh_quote("[ALT+g] Green"),
        b = sh_quote("[ALT+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys M-r` emits the `ESC r` Alt/Meta chord. The binary
    // maps it to the `[ALT+r]` hotkey, selects Red, and submits.
    harness.send_key("M-r").expect("send Alt+R");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");

    assert!(
        frame.plain.contains("PICK:Red"),
        "Alt+R MUST select + submit the [ALT+r] Red option in tmux; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — relaxed Ctrl/Alt+Shift chord matching on real terminal bytes
//
// Spec F5 / review finding F2 (hotkey): the matcher uses
// `modifiers.contains(...)` plus `c.to_ascii_lowercase()`, so a benign extra
// SHIFT bit on an uppercase chord must not suppress an otherwise-valid Ctrl/Alt
// hotkey. The L1 reducer tests
// (`choose_one/tests.rs::{ctrl_shift_chord_matches_ctrl_hotkey,
// alt_shift_chord_matches_alt_hotkey}`) prove the reducer once crossterm has
// already produced `CONTROL|SHIFT` / `ALT|SHIFT`. These two tests prove the
// stronger end-to-end claim: real terminal bytes for the physical chord decode
// into the relaxed-matched selection.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
fn level2_tmux_alt_shift_r_chord_selects_red() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-one {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[ALT+r] Red"),
        g = sh_quote("[ALT+g] Green"),
        b = sh_quote("[ALT+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys M-R` (capital R) emits the `ESC R` Alt/Meta chord — the
    // legacy byte sequence for Alt+Shift+r. crossterm decodes it as
    // `KeyCode::Char('R')` with the ALT modifier set; the matcher lowercases
    // 'R' -> 'r' and looks it up in the alt-hotkey map, so the extra SHIFT
    // (capital letter) does not suppress the `[ALT+r]` hotkey. This is the
    // clean legacy-byte proof of relaxed ALT|SHIFT matching.
    harness.send_key("M-R").expect("send Alt+Shift+R");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");

    assert!(
        frame.plain.contains("PICK:Red"),
        "Alt+Shift+R MUST relaxed-match + submit the [ALT+r] Red option in tmux; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// CONTROL|SHIFT requires the kitty keyboard protocol.
//
// Legacy terminals collapse Ctrl+R and Ctrl+Shift+R to the same 0x12 byte, so
// `tmux send-keys C-r` cannot carry a *distinct* CONTROL|SHIFT payload — the
// L1 reducer test stays the authoritative CONTROL|SHIFT contract for the
// legacy path. To inject a true CONTROL|SHIFT chord we use the kitty CSI-u
// encoding `\x1b[<codepoint>;<modifiers>u`, where
// modifiers = 1 + (shift=1) + (alt=2) + (ctrl=4). For Ctrl+Shift+`r`
// (codepoint 114): modifiers = 1+1+4 = 6, i.e. `\x1b[114;6u`. The binary
// pushes `DISAMBIGUATE_ESCAPE_CODES | REPORT_ALL_KEYS_AS_ESCAPE_CODES` on a
// kitty-aware terminal, so WezTerm reports the chord as kitty bytes which
// crossterm decodes to `KeyCode::Char('r')` + `CONTROL|SHIFT`. The relaxed
// matcher then selects the `[CTRL+r]` option. We deliver the bytes via
// `wezterm cli send-text`, bypassing the OS keyboard entirely (see the
// bare-Ctrl test above for why OS injection is intentionally avoided).
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
#[cfg(target_os = "macos")]
fn level2_wezterm_ctrl_shift_r_kitty_bytes_select_red() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-one {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[CTRL+r] Red"),
        g = sh_quote("[CTRL+g] Green"),
        b = sh_quote("[CTRL+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // Kitty CSI-u bytes for Ctrl+Shift+r: codepoint 114, modifiers 6 (ctrl+shift).
    harness
        .send_text(b"\x1b[114;6u")
        .expect("send kitty Ctrl+Shift+r");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture wezterm pane");
    // The chord submits, so question exits and the shell returns to a prompt;
    // no Ctrl+C cleanup is needed. If for any reason it did not submit, a
    // best-effort Ctrl+C keeps the shared pane usable for the next test.
    if !frame.plain.contains("PICK:Red") {
        cleanup_via_ctrl_c(harness);
        eprintln!("=== Level-2 capture (raw, with escapes) ===");
        eprintln!("{:?}", frame.raw);
        eprintln!("=== Level-2 capture (plain) ===");
        eprintln!("{:?}", frame.plain);
    }

    assert!(
        frame.plain.contains("PICK:Red"),
        "kitty Ctrl+Shift+r bytes piped into a real WezTerm pane MUST relaxed-match \
         + submit the [CTRL+r] Red option; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — relaxed Ctrl/Alt+Shift chord matching for choose-many
//
// The choose-many reducer shares the relaxed matcher with choose-one
// (`modifiers.contains(...)` + `c.to_ascii_lowercase()`), but the user-observable
// path differs: a choose-many hotkey *toggles* a selection and does NOT submit.
// These tests mirror the choose-one relaxed-match tests above and then submit
// with Enter so the captured value reflects the toggled selection. A single
// toggled option submits as just its label (`Red`), so we assert `PICK:Red`.
// Together with `choose_many/tests.rs::{ctrl_shift_chord_matches_ctrl_hotkey,
// alt_shift_chord_matches_alt_hotkey}` (L1 reducer) and the L3 physical-key
// tests in `level3_chord_select.rs`, this closes the choose-many parity gap
// review-3 flagged.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
fn level2_tmux_alt_shift_r_chord_selects_red_choose_many() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-many {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[ALT+r] Red"),
        g = sh_quote("[ALT+g] Green"),
        b = sh_quote("[ALT+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // `tmux send-keys M-R` (capital R) emits the `ESC R` Alt/Meta chord — the
    // legacy byte sequence for Alt+Shift+r. The matcher lowercases 'R' -> 'r'
    // and toggles the `[ALT+r]` option, so the extra SHIFT does not suppress it.
    // choose-many toggles without submitting, so we then press Enter to submit.
    harness.send_key("M-R").expect("send Alt+Shift+R");
    std::thread::sleep(Duration::from_millis(300));
    harness.send_key("Enter").expect("submit selection");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");

    assert!(
        frame.plain.contains("PICK:Red"),
        "Alt+Shift+R MUST relaxed-match + toggle the [ALT+r] Red option in choose-many \
         (submitted via Enter); got: {:?}",
        frame.plain
    );
}

// CONTROL|SHIFT needs the kitty keyboard protocol (legacy terminals collapse
// Ctrl+R and Ctrl+Shift+R to the same 0x12 byte), so we inject the kitty CSI-u
// encoding through WezTerm exactly as the choose-one test does. See the
// `level2_wezterm_ctrl_shift_r_kitty_bytes_select_red` comment block above for
// the modifier-bit derivation (`\x1b[114;6u` = Ctrl+Shift+`r`).
#[test]
#[serial(level2)]
#[cfg(target_os = "macos")]
fn level2_wezterm_ctrl_shift_r_kitty_bytes_select_red_choose_many() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    let bin = sh_quote(&question_binary());
    let cmd = format!(
        "out=$({bin} choose-many {r} {g} {b}); printf '\\nPICK:%s\\n' \"$out\"\n",
        r = sh_quote("[CTRL+r] Red"),
        g = sh_quote("[CTRL+g] Green"),
        b = sh_quote("[CTRL+b] Blue"),
    );
    harness.send_text(cmd.as_bytes()).expect("launch question");
    std::thread::sleep(Duration::from_millis(QUESTION_RENDER_MS));

    // Kitty CSI-u bytes for Ctrl+Shift+r toggle the `[CTRL+r]` option; choose-many
    // does not submit on toggle, so submit with a carriage return afterward.
    harness
        .send_text(b"\x1b[114;6u")
        .expect("send kitty Ctrl+Shift+r");
    std::thread::sleep(Duration::from_millis(300));
    harness.send_text(b"\r").expect("submit selection");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture wezterm pane");
    // The Enter submit returns the shell to a prompt; if for any reason it did
    // not, a best-effort Ctrl+C keeps the shared pane usable for the next test.
    if !frame.plain.contains("PICK:Red") {
        cleanup_via_ctrl_c(harness);
        eprintln!("=== Level-2 capture (raw, with escapes) ===");
        eprintln!("{:?}", frame.raw);
        eprintln!("=== Level-2 capture (plain) ===");
        eprintln!("{:?}", frame.plain);
    }

    assert!(
        frame.plain.contains("PICK:Red"),
        "kitty Ctrl+Shift+r bytes piped into a real WezTerm pane MUST relaxed-match \
         + toggle the [CTRL+r] Red option in choose-many (submitted via Enter); got: {:?}",
        frame.plain
    );
}

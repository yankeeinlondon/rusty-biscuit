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
//! Level-3 tests (OS keyboard injection via cliclick) still own their
//! own foreground-visible harness because `focus_spawned_pane` / AXRaise
//! requires the spawn to be on the active workspace.

#![cfg(unix)]

#[path = "common/mod.rs"]
mod common;

use std::time::Duration;

use biscuit_test_harness::shared::SharedHarness;
use common::real_terminal::{
    SpawnVisibility, TerminalHarness, cliclick, kitty::KittyHarness, tmux::TmuxHarness,
    wezterm::WezTermHarness,
};
use serial_test::serial;
use test_toolkit::{Level, require_level};

fn question_binary() -> String {
    assert_cmd::cargo::cargo_bin("question")
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
    require_level!(Level::L2, WezTermHarness::available(), "WezTerm");

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
    require_level!(Level::L2, KittyHarness::available(), "kitty");

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
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
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
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
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
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
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
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
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    harness.send_text(b"clear\n").expect("clear");
    harness.settle();
    // Run the binary inline so the parent shell sees `$?` after it
    // exits via Ctrl+C. The shell prints `EXIT:<code>` on its own
    // line which we then assert on. We do NOT sleep after the printf
    // — the shared shell returns to its prompt naturally and the
    // next test starts with a `clear`.
    let bin = sh_quote(&question_binary());
    let cmd = format!("{bin} choose-one Red Green Blue; printf '\\nEXIT:%s\\n' \"$?\"\n");
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
// What this Level-2 test does NOT prove (and is **not** a substitute
// for Level 3): that WezTerm emits those bytes when a user physically
// holds bare Ctrl. That path depends on (a) macOS dispatching a
// `flagsChanged` event, (b) WezTerm's keymap not intercepting Ctrl,
// and (c) `enable_kitty_keyboard = true` in the user's `wezterm.lua`.
// Per the testing-best-practices rubric, the user-observable claim
// "press Ctrl → badges appear" requires Level-3 OS keyboard injection,
// and no userspace tool on macOS can synthesise the `flagsChanged`
// event needed to drive that path. See `level3_wezterm_bare_ctrl_reveals_badges`
// below — it is `#[ignore]` for exactly that reason — and the
// **Production readiness** note in `biscuit-tui/docs/components/choose_one.md`.
// ---------------------------------------------------------------------------

#[test]
#[serial(level2)]
#[cfg(target_os = "macos")]
fn level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges() {
    require_level!(Level::L2, WezTermHarness::available(), "WezTerm");

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
// Level 3 — bare-Ctrl via cliclick (macOS)
//
// **Known limitation.** cliclick uses `CGEventCreateKeyboardEvent` to
// synthesise key events, but on macOS modifier keys propagate through
// `flagsChanged` events at the AppKit layer. cliclick's synthetic
// modifier events do not always travel that path — the chord test
// works (the modifier flag rides along with the letter keyDown, which
// IS a normal CGEvent), but a *bare* modifier press tends to be lost
// before it reaches WezTerm's flagsChanged handler. As a result this
// test cannot reliably verify the bare-modifier path even when
// everything in the binary is correct.
//
// Why we keep the test: a future cliclick / OS / WezTerm improvement
// may close the gap. The test itself is not broken — it correctly
// asserts the right end-state — and `RUN_LEVEL3=1` lets a developer
// run it interactively.
//
// Level-3 tests do NOT share the level-2 pane: they require
// `SpawnVisibility::Foreground` so AXRaise can route OS events to the
// pane. The level-2 shared pane is background-spawned and would be
// invisible to AXRaise.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Level 3 — basic-keypress diagnostic
//
// Localises whether ANY OS-level keyboard injection reaches the
// spawned binary. Sends a single arrow-down via cliclick and checks
// whether the active option marker moves from "Red" to "Green".
//
// If THIS test passes but the bare-Ctrl / chord tests still fail,
// the problem isn't focus or event delivery — WezTerm's keymap is
// intercepting Ctrl-based input before forwarding to the pane (e.g.
// the user's `wezterm.lua` binds Ctrl+R to a WezTerm action) AND/OR
// `enable_kitty_keyboard` is disabled in the user's config.
// ---------------------------------------------------------------------------

#[test]
#[cfg(target_os = "macos")]
fn level3_wezterm_arrow_down_moves_active_marker() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );
    let bin = question_binary();
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    harness
        .spawn_program(&bin, &["choose-one", "Red", "Green", "Blue"])
        .expect("spawn question in wezterm");
    std::thread::sleep(Duration::from_millis(400));

    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned wezterm pane")
        .expect("AXRaise yielded no window coords");

    // Single batched cliclick: click to focus, then arrow-down. No
    // modifier — pure plain key. If even this doesn't reach the
    // binary the problem is upstream of the binary entirely.
    let _ = std::process::Command::new("cliclick")
        .args([
            "-m",
            "verbose",
            "-w",
            "100",
            &format!("c:{},{}", coords.0, coords.1),
            "kp:arrow-down",
        ])
        .output()
        .map(|o| {
            eprintln!(
                "[cliclick stdout] {}",
                String::from_utf8_lossy(&o.stdout).trim()
            );
        });

    std::thread::sleep(Duration::from_millis(400));
    let frame = harness.capture().expect("capture after arrow-down");
    eprintln!("=== After arrow-down (plain) ===");
    eprintln!("{:?}", frame.plain);

    // Find which option line carries the active marker `▶`.
    let active_on_green = frame
        .plain
        .lines()
        .any(|l| l.contains("▶") && l.contains("Green"));
    let active_on_red = frame
        .plain
        .lines()
        .any(|l| l.contains("▶") && l.contains("Red"));
    if !active_on_green {
        eprintln!(
            "DIAGNOSTIC: arrow-down did NOT reach the binary. \
             active_on_red={active_on_red}, active_on_green={active_on_green}. \
             This means OS-level keyboard injection is not flowing to \
             the spawned WezTerm pane at all — even plain keys without \
             modifiers don't arrive."
        );
    } else {
        eprintln!(
            "DIAGNOSTIC: arrow-down DID reach the binary (active marker \
             moved to Green). The Ctrl-based test failures are caused \
             by either WezTerm intercepting the chord (check wezterm.lua \
             for Ctrl+R bindings) or the user's wezterm.lua not setting \
             `enable_kitty_keyboard = true` (which is required for bare \
             modifier reporting)."
        );
    }
    assert!(
        active_on_green,
        "arrow-down should move the active marker from Red to Green"
    );
}

// ---------------------------------------------------------------------------
// Level 3 — bare Ctrl via cliclick (macOS) — KNOWN-LIMITED, IGNORED
//
// This test is `#[ignore]` because there is no userspace path on
// macOS that synthesises a real `flagsChanged` event — and that's
// the AppKit event type bare-modifier presses arrive through. We
// have verified through this session that:
//
//   * `cliclick kd:ctrl` uses CGEventCreateKeyboardEvent, which
//     dispatches a regular keyDown event. AppKit apps observing
//     `flagsChanged` (which WezTerm does for bare modifiers) don't
//     see anything happen.
//   * AppleScript `tell application "System Events" to key down
//     control` shares the same underlying CGEvent path — same
//     limitation.
//   * The chord case (`kd:ctrl t:r ku:ctrl`) WORKS because the Ctrl
//     modifier rides along with the letter keyDown event as a
//     normal CGEvent flag, never needing a flagsChanged dispatch.
//
// To make this test pass would require a custom Rust binary built on
// the `core_graphics` crate that constructs a CGEvent with type
// `kCGEventFlagsChanged` and the Control flag set, then posts it via
// `kCGHIDEventTap`. That is a meaningful piece of new code, not a
// test-harness tweak.
//
// Verification-level honesty: per the testing-best-practices rubric,
// "press Ctrl → badges appear" is a user-observable keyboard claim
// and therefore requires Level-3 OS keyboard injection. The Level-2
// raw-bytes test (`level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`)
// proves the binary's *internal* kitty-byte handler is correct but
// is NOT a substitute for Level-3 verification of the end-to-end UX.
// Bare-modifier press visibility is therefore documented as
// **best-effort / not production-verified on macOS** in
// `biscuit-tui/docs/components/choose_one.md` until a flagsChanged-capable
// injector is wired up and this `#[ignore]` is removed.
//
// We keep this test in source (rather than deleting it) because the
// supporting harness is sound — the only missing piece is OS event
// synthesis. Anyone who wires up a flagsChanged-capable injector can
// remove `#[ignore]` and the rest just works.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "macOS userspace can't synthesise flagsChanged events; see comment block above and run \
            level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges for canonical verification"]
#[cfg(target_os = "macos")]
fn level3_wezterm_bare_ctrl_reveals_badges() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );
    let bin = question_binary();
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    // Options carry explicit `[CTRL+x]` prefixes — that's the only
    // way to get a hotkey. Plain options have no badge regardless
    // of modifier state, so we need explicit prefixes to assert
    // that holding bare Ctrl surfaces a `^R` badge.
    harness
        .spawn_program(
            &bin,
            &[
                "choose-one",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ],
        )
        .expect("spawn question in wezterm");

    // Capture once after spawn so we can prove the binary is
    // rendering before we attempt to inject keys.
    std::thread::sleep(Duration::from_millis(400));
    let baseline = harness.capture().expect("baseline capture");
    assert!(
        baseline.plain.contains("Red"),
        "spawned binary did not render before key injection; baseline plain: {:?}",
        baseline.plain
    );

    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned wezterm pane")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");

    // cliclick CAN synthesise the click (mouse events dispatch
    // cleanly), but it CANNOT reliably synthesise bare-modifier
    // presses — macOS routes those through AppKit's flagsChanged
    // event type, which cliclick's CGEventCreateKeyboardEvent path
    // does not fire. We click via cliclick (focus transfer), then
    // hold Ctrl via osascript / System Events, which DOES dispatch
    // through the AppKit path because System Events is itself an
    // AppKit consumer.
    cliclick::click_at(coords.0, coords.1).expect("click to focus pane");
    std::thread::sleep(Duration::from_millis(150));
    cliclick::system_events_key_down("control").expect("System Events key down control");

    // Right after click+kd, ask macOS who actually has keyboard
    // focus. This is the definitive diagnostic — if frontmost is
    // anything other than WezTerm we know the click didn't activate
    // it and cliclick events are landing in the wrong app.
    if let Ok(out) = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events"
                   set frontApp to name of first application process whose frontmost is true
                   set focusedTitle to "<no AXFocusedWindow>"
                   set mainTitle to "<no AXMain>"
                   try
                       tell first process whose name contains "wezterm"
                           try
                               set focusedTitle to title of (value of attribute "AXFocusedWindow")
                           end try
                           try
                               set mainTitle to title of (first window whose value of attribute "AXMain" is true)
                           end try
                       end tell
                   end try
                   return frontApp & " | focused: " & focusedTitle & " | main: " & mainTitle
               end tell"#,
        ])
        .output()
    {
        eprintln!(
            "[post-click] {}",
            String::from_utf8_lossy(&out.stdout).trim()
        );
    }
    // Give WezTerm + the binary time to process the press event and
    // the renderer to draw the badges.
    std::thread::sleep(Duration::from_millis(300));
    let frame = harness
        .capture()
        .expect("capture wezterm pane during Ctrl hold");
    // Always release before any assertion can panic — otherwise a
    // stuck Ctrl modifier would mess with the rest of the test run.
    // Symmetric with the System Events key down: the press path used
    // System Events / AppKit flagsChanged, so the release MUST go
    // through the same path or the OS modifier state gets out of sync.
    let _ = cliclick::system_events_key_up("control");

    let has_badge =
        frame.plain.contains("^R") || frame.plain.contains("^G") || frame.plain.contains("^B");
    if !has_badge {
        eprintln!("=== Level-3 capture (raw, with escapes) ===");
        eprintln!("{:?}", frame.raw);
        eprintln!("=== Level-3 capture (plain) ===");
        eprintln!("{:?}", frame.plain);
    }
    assert!(
        has_badge,
        "bare Ctrl held in real WezTerm MUST reveal a ^R/^G/^B badge \
         next to options that have explicit Ctrl hotkeys.\n\
         \n\
         If the diagnostic above shows `focused: question` (which it \
         should after our AXRaise + click + activate machinery), then \
         OS focus is correct and the failure is in WezTerm's keyboard \
         protocol handling. Specifically:\n\
         \n\
         WezTerm requires `config.enable_kitty_keyboard = true` in your \
         wezterm.lua to forward bare-modifier presses (Ctrl held without \
         a chord) to the running pane. Without this, WezTerm silently \
         drops the protocol push the binary issues at startup and bare \
         Ctrl never reaches `question`.\n\
         \n\
         Add to ~/.config/wezterm/wezterm.lua:\n\
             config.enable_kitty_keyboard = true\n\
         then restart any WezTerm windows the test will spawn into.\n\
         \n\
         The companion `level3_wezterm_arrow_down_moves_active_marker` \
         test confirms basic key delivery; if THAT also fails, the \
         issue is upstream of WezTerm config."
    );
}

// ---------------------------------------------------------------------------
// Level 3 — chord injection (Ctrl+R) via cliclick (macOS)
//
// A weaker-but-still-useful Level-3 test: inject the *chord* `Ctrl+R`
// and verify the binary selects the `Red` option (which has the
// default `Ctrl+R` hotkey). Unlike bare-Ctrl, chord events flow via
// the terminal's standard input path on every terminal — no kitty
// flag required — so this test verifies cliclick → WezTerm → binary
// connectivity even on configurations where bare-modifier reporting
// is broken or absent.
//
// Same env gate (`RUN_LEVEL3=1`) for the same focus-stability reason.
// ---------------------------------------------------------------------------

#[test]
#[cfg(target_os = "macos")]
fn level3_wezterm_ctrl_r_chord_selects_red() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );
    let bin = question_binary();
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    // Options MUST carry explicit `[CTRL+x]` prefixes — a plain
    // option string ("Red") has no hotkey binding, so pressing
    // Ctrl+R against it does nothing. This was a long-standing bug
    // in this test that masked itself as a focus failure: events
    // reached the binary fine, but the binary had nothing bound to
    // Ctrl+R so the prompt didn't submit.
    harness
        .spawn_program(
            &bin,
            &[
                "choose-one",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ],
        )
        .expect("spawn question in wezterm");
    std::thread::sleep(Duration::from_millis(400));

    // Baseline: prove the prompt is up before injection.
    let baseline = harness.capture().expect("baseline capture");
    assert!(
        baseline.plain.contains("Red"),
        "spawned binary did not render before chord injection"
    );

    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned wezterm pane")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");

    cliclick::click_then_ctrl_chord(coords.0, coords.1, "r").expect("invoke cliclick click+chord");

    if let Ok(out) = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events"
                   set frontApp to name of first application process whose frontmost is true
                   set focusedTitle to "<no AXFocusedWindow>"
                   set mainTitle to "<no AXMain>"
                   try
                       tell first process whose name contains "wezterm"
                           try
                               set focusedTitle to title of (value of attribute "AXFocusedWindow")
                           end try
                           try
                               set mainTitle to title of (first window whose value of attribute "AXMain" is true)
                           end try
                       end tell
                   end try
                   return frontApp & " | focused: " & focusedTitle & " | main: " & mainTitle
               end tell"#,
        ])
        .output()
    {
        eprintln!(
            "[post-chord] {}",
            String::from_utf8_lossy(&out.stdout).trim()
        );
    }

    std::thread::sleep(Duration::from_millis(500));

    // Successful submission tears down the pane — `wezterm cli
    // get-text` will return "no such pane <id>". Either the
    // capture errors with that signature OR (on a slower system)
    // the pane is still alive but no longer renders the prompt.
    // Both are success.
    match harness.capture() {
        Err(e) if format!("{e}").contains("no such pane") => {
            // Pane destroyed → submission completed cleanly.
        }
        Ok(frame) => {
            assert!(
                !frame.plain.contains("Enter=Submit"),
                "Ctrl+R should have submitted; prompt still visible: {:?}",
                frame.plain
            );
        }
        Err(other) => panic!("unexpected capture error: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Level 3 — Alt chord injection (Alt+R) via cliclick (macOS)
//
// Companion to `level3_wezterm_ctrl_r_chord_selects_red`: same
// click+chord shape, but with the Alt/Option modifier. This proves
// the OS → WezTerm → binary path for Alt-letter hotkeys, which the
// docs claim to support alongside Ctrl chords.
//
// Like the Ctrl chord, an Alt chord rides along with the letter
// keyDown as a normal CGEvent (modifier flag set on the same event),
// so cliclick can synthesise it without needing a flagsChanged
// dispatch. This is structurally identical to the Ctrl test and
// avoids the `#[ignore]` fate of the bare-Alt path.
//
// Options carry explicit `[ALT+x]` prefixes; without them, Alt+R
// has nothing bound and the prompt would not submit.
// ---------------------------------------------------------------------------

#[test]
#[cfg(target_os = "macos")]
fn level3_wezterm_alt_r_chord_selects_red() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );
    let bin = question_binary();
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    harness
        .spawn_program(
            &bin,
            &["choose-one", "[ALT+r] Red", "[ALT+g] Green", "[ALT+b] Blue"],
        )
        .expect("spawn question in wezterm");
    std::thread::sleep(Duration::from_millis(400));

    // Baseline: prove the prompt is up before injection.
    let baseline = harness.capture().expect("baseline capture");
    assert!(
        baseline.plain.contains("Red"),
        "spawned binary did not render before chord injection"
    );

    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned wezterm pane")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");

    cliclick::click_then_alt_chord(coords.0, coords.1, "r")
        .expect("invoke cliclick click+alt-chord");

    if let Ok(out) = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events"
                   set frontApp to name of first application process whose frontmost is true
                   set focusedTitle to "<no AXFocusedWindow>"
                   set mainTitle to "<no AXMain>"
                   try
                       tell first process whose name contains "wezterm"
                           try
                               set focusedTitle to title of (value of attribute "AXFocusedWindow")
                           end try
                           try
                               set mainTitle to title of (first window whose value of attribute "AXMain" is true)
                           end try
                       end tell
                   end try
                   return frontApp & " | focused: " & focusedTitle & " | main: " & mainTitle
               end tell"#,
        ])
        .output()
    {
        eprintln!(
            "[post-chord] {}",
            String::from_utf8_lossy(&out.stdout).trim()
        );
    }

    std::thread::sleep(Duration::from_millis(500));

    // Successful submission tears down the pane — same shape as the
    // Ctrl+R companion test.
    match harness.capture() {
        Err(e) if format!("{e}").contains("no such pane") => {
            // Pane destroyed → submission completed cleanly.
        }
        Ok(frame) => {
            assert!(
                !frame.plain.contains("Enter=Submit"),
                "Alt+R should have submitted; prompt still visible: {:?}",
                frame.plain
            );
        }
        Err(other) => panic!("unexpected capture error: {other}"),
    }
}

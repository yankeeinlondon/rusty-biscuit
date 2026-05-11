//! Real-terminal rendering tests.
//!
//! See `cli/tests/common/real_terminal/mod.rs` for the harness contract
//! and Level-1 / Level-2 / Level-3 vocabulary. The tests in this file
//! are deliberately written so that:
//!
//! - they **skip cleanly** (printing `skipping: requires <tool>`) when
//!   the host lacks the required terminal — no `#[ignore]` markers,
//!   no spurious failures on CI or contributor laptops;
//! - they **actually run** when the prerequisite IS available, so
//!   `cargo test` on a developer machine that has WezTerm/Kitty/tmux
//!   installed exercises the real path. This is what the user
//!   asked for: "if the host has the terminal then the test is run."

#![cfg(unix)]

#[path = "common/mod.rs"]
mod common;

use std::time::Duration;

use common::real_terminal::{
    SpawnVisibility, TerminalHarness, cliclick, kitty::KittyHarness, tmux::TmuxHarness,
    wezterm::WezTermHarness,
};

fn question_binary() -> String {
    assert_cmd::cargo::cargo_bin("question")
        .to_str()
        .expect("question binary path is utf-8")
        .to_string()
}

// ---------------------------------------------------------------------------
// Level 2 — render-in-real-terminal smoke tests
//
// Each spawns the binary in a real terminal/multiplexer, captures the
// rendered pane text, and asserts that option labels appear. The PTY
// suite cannot prove glyph-width / SGR / scroll behaviour matches what
// a user actually sees; these can.
// ---------------------------------------------------------------------------

#[test]
fn level2_wezterm_renders_option_labels() {
    if !WezTermHarness::available() {
        eprintln!("skipping: requires WezTerm (set WEZTERM_UNIX_SOCKET + wezterm in PATH)");
        return;
    }
    let bin = question_binary();
    let mut harness = WezTermHarness::new();
    harness
        .spawn_program(&bin, &["choose-one", "Red", "Green", "Blue"])
        .expect("spawn question in wezterm");
    std::thread::sleep(Duration::from_millis(400));
    let frame = harness.capture().expect("capture wezterm pane");
    assert!(
        frame.plain.contains("Red")
            && frame.plain.contains("Green")
            && frame.plain.contains("Blue"),
        "expected option labels in wezterm pane; got: {:?}",
        frame.plain
    );
}

#[test]
fn level2_kitty_renders_option_labels() {
    if !KittyHarness::available() {
        eprintln!("skipping: requires kitty (KITTY_LISTEN_ON + remote_control enabled)");
        return;
    }
    let bin = question_binary();
    let mut harness = KittyHarness::new();
    harness
        .spawn_program(&bin, &["choose-one", "Red", "Green", "Blue"])
        .expect("spawn question in kitty");
    std::thread::sleep(Duration::from_millis(400));
    let frame = harness.capture().expect("capture kitty window");
    assert!(
        frame.plain.contains("Red")
            && frame.plain.contains("Green")
            && frame.plain.contains("Blue"),
        "expected option labels in kitty window; got: {:?}",
        frame.plain
    );
}

#[test]
fn level2_tmux_renders_option_labels() {
    if !TmuxHarness::available() {
        eprintln!("skipping: requires tmux on PATH");
        return;
    }
    let bin = question_binary();
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program(&bin, &["choose-one", "Red", "Green", "Blue"])
        .expect("spawn question in tmux");
    std::thread::sleep(Duration::from_millis(400));
    let frame = harness.capture().expect("capture tmux pane");
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
fn level2_tmux_ctrl_space_reveals_badges() {
    if !TmuxHarness::available() {
        eprintln!("skipping: requires tmux on PATH");
        return;
    }
    let bin = question_binary();
    let mut harness = TmuxHarness::new();
    // Options with explicit hotkey prefixes — only options that opt
    // in via `[CTRL+x]` get badges.
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
        .expect("spawn question in tmux");
    std::thread::sleep(Duration::from_millis(400));
    // `tmux send-keys C-space` routes through tmux's key-name
    // translation. Sending the raw NUL byte via `send_text -l` is
    // rejected by `Command::arg` (no nul in process arguments).
    harness.send_key("C-space").expect("send Ctrl+Space");
    std::thread::sleep(Duration::from_millis(300));
    let frame = harness.capture().expect("capture tmux pane");
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
fn level2_tmux_ctrl_held_badge_uses_orange_bold_black_sgr() {
    if !TmuxHarness::available() {
        eprintln!("skipping: requires tmux on PATH");
        return;
    }
    let bin = question_binary();
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program(
            &bin,
            &[
                "choose-one",
                "--hotkey-badges",
                "ctrl",
                "[CTRL+r] Red",
                "[CTRL+g] Green",
                "[CTRL+b] Blue",
            ],
        )
        .expect("spawn question in tmux");
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture tmux pane");

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
fn level2_tmux_ctrl_c_exits_130() {
    if !TmuxHarness::available() {
        eprintln!("skipping: requires tmux on PATH");
        return;
    }
    let bin = question_binary();
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program(
            "sh",
            &[
                "-c",
                r#""$0" choose-one Red Green Blue; code=$?; printf '\nEXIT:%s\n' "$code"; sleep 2"#,
                &bin,
            ],
        )
        .expect("spawn question in tmux");
    std::thread::sleep(Duration::from_millis(400));

    harness.send_key("C-c").expect("send Ctrl+C");
    std::thread::sleep(Duration::from_millis(500));

    let frame = harness.capture().expect("capture tmux pane");
    assert!(
        frame.plain.contains("EXIT:130"),
        "Ctrl+C from tmux MUST make question choose-one exit 130; got: {:?}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Level 2 — bare Ctrl via raw kitty bytes through `wezterm cli send-text`
//
// This is the *most reliable* way to verify that the binary correctly
// handles WezTerm's bare-modifier kitty encoding inside a real
// terminal pane. We send the literal escape sequence WezTerm would
// emit for a Ctrl press / release through `wezterm cli send-text`,
// capture the rendered pane, and assert badges appear during the
// "press" window. This bypasses macOS keyboard simulation entirely.
//
// What it proves: with `REPORT_EVENT_TYPES` +
// `REPORT_ALL_KEYS_AS_ESCAPE_CODES` pushed, the binary turns the
// kitty press/release sequences into the `CtrlHeld` / `Hidden` state
// transitions that drive the badge renderer.
//
// What it does NOT prove: that WezTerm itself actually *emits* those
// bytes when the user physically presses bare Ctrl. That's the
// terminal's responsibility and depends on `enable_kitty_keyboard`
// in the user's `wezterm.lua`. See the Level-3 cliclick test below
// for the (limited) attempt to verify that path.
// ---------------------------------------------------------------------------

#[test]
#[cfg(target_os = "macos")]
fn level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges() {
    if !WezTermHarness::available() {
        eprintln!("skipping: requires WezTerm");
        return;
    }
    let bin = question_binary();
    let mut harness = WezTermHarness::new();
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

    // Kitty keyboard-protocol bytes for "Left Ctrl press" — exactly
    // what WezTerm should emit when bare Ctrl is held under our flags.
    harness
        .send_text(b"\x1b[57442;1u")
        .expect("send kitty bare-Ctrl press");
    std::thread::sleep(Duration::from_millis(200));

    let frame = harness.capture().expect("capture during Ctrl 'hold'");

    // Send the release so the binary returns to Hidden state cleanly
    // before the next test runs.
    let _ = harness.send_text(b"\x1b[57442;1:3u");

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
    if !level3_enabled() {
        eprintln!("skipping: set RUN_LEVEL3=1 to enable Level-3 OS keyboard injection");
        return;
    }
    if !WezTermHarness::available() {
        eprintln!("skipping: requires WezTerm");
        return;
    }
    if !cliclick::available() {
        eprintln!("skipping: requires cliclick");
        return;
    }
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

fn level3_enabled() -> bool {
    std::env::var("RUN_LEVEL3").as_deref() == Ok("1")
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
// Until/unless that gets built, the **canonical** verification for
// the binary's bare-Ctrl handling is the Level-2 raw-bytes test
// `level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`, which pipes
// the literal kitty escape `\x1b[57442;1u` directly into a real
// WezTerm pane via `wezterm cli send-text` and asserts the badge
// appears. That test runs deterministically on every CI host with
// WezTerm available.
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
    if !level3_enabled() {
        eprintln!(
            "skipping: set RUN_LEVEL3=1 to enable Level-3 OS keyboard injection \
             (requires focus stability — the spawned WezTerm window must stay \
             frontmost during the test)"
        );
        return;
    }
    if !WezTermHarness::available() {
        eprintln!("skipping: requires WezTerm (set WEZTERM_UNIX_SOCKET + wezterm in PATH)");
        return;
    }
    if !cliclick::available() {
        eprintln!("skipping: requires cliclick (brew install cliclick)");
        return;
    }
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
    if !level3_enabled() {
        eprintln!("skipping: set RUN_LEVEL3=1 to enable Level-3 OS keyboard injection");
        return;
    }
    if !WezTermHarness::available() {
        eprintln!("skipping: requires WezTerm");
        return;
    }
    if !cliclick::available() {
        eprintln!("skipping: requires cliclick");
        return;
    }
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

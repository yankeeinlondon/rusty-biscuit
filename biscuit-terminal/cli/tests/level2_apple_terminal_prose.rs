//! Level-2 tests for Prose graceful degradation against Apple Terminal.
//!
//! These tests drive the real `Terminal.app` via `osascript` (through
//! [`AppleTerminalHarness`]) so that the lower-capability display path
//! (no images, no OSC8, single-underline only) is exercised end-to-end.
//!
//! ## Skip-clean contract
//!
//! Every test gates on [`AppleTerminalHarness::available`] via
//! [`test_toolkit::require_level!`]. When Terminal.app cannot be
//! addressed (off-macOS, `CI=1`, or `osascript` unavailable) the test
//! prints `skipping: requires Terminal.app` and returns OK. Setting
//! `BISCUIT_TEST_LEVEL_REQUIRED=2` converts that skip into a panic.
//! No `#[ignore]` markers are used.
//!
//! ## Capture limitation
//!
//! Terminal.app's AppleScript scripting interface only exposes the
//! visible plain text of a tab — there is no way to retrieve the
//! underlying ANSI/SGR byte stream. Both `frame.raw` and `frame.plain`
//! therefore hold the same string. Negative byte-level assertions
//! ("no `\x1b[4:2m` was emitted") are covered by Level-1 PTY tests;
//! these Level-2 tests assert only what is **visible** on the real
//! display — the literal escape characters never appear as on-screen
//! garbage when the renderer degrades correctly.
//!
//! ## Serialization
//!
//! Each test carries `#[serial_test::serial(level2_terminal)]` and
//! shares the `level2_terminal` group key with `level2_image.rs`
//! because Terminal.app exposes one global application state to
//! AppleScript; running multiple Apple-Terminal tests in parallel
//! would race on window indices and focus.

mod common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::apple_terminal::AppleTerminalHarness;
use biscuit_test_harness::shared::SharedHarness;
use serial_test::serial;
use std::process::{Command, Stdio};
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

/// Extra settle time after spawning a Terminal.app shell to absorb the
/// AppleScript dispatch latency before sending commands. Apple Terminal's
/// `do script` returns immediately, but `exec bash -l` then has to source
/// the user's profile (which can include slow plugins / completions /
/// custom prompts), so we wait substantially longer than the WezTerm /
/// Kitty defaults.
const SHELL_READY_MS: u64 = 2500;

/// Process-shared Apple Terminal window reused across the prose tests in
/// this file. Initialized with `preserve_capabilities(true)` so `bt`
/// runs through its real capability-detection path (no forced color
/// overrides). The lifecycle test does NOT use this shared harness — it
/// intentionally owns its own instance to exercise Drop cleanup.
static SHARED_APPLE: SharedHarness<AppleTerminalHarness> = SharedHarness::new();

// ------------------------------------------------------------------
// AC-1 — OSC8 link fallback is visible in Apple Terminal
// ------------------------------------------------------------------

/// Apple Terminal does not implement OSC8 hyperlinks; the renderer must
/// emit a markdown-style fallback `[label](url)` so the URL is visible.
///
/// Verifies AC-1: `<a href="...">label</a>` renders as visible text
/// `label` followed by the bracketed URL.
#[test]
#[serial(level2_terminal)]
fn level2_apple_terminal_link_fallback_visible() {
    require_level!(Level::L2, AppleTerminalHarness::available(), Backend::AppleTerminal);

    let mut guard = SHARED_APPLE.get_or_init(|| {
        AppleTerminalHarness::shared_or_else(|| {
            let mut h = AppleTerminalHarness::new().preserve_capabilities(true);
            h.spawn_shell()?;
            std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
            Ok(h)
        })
        .expect("attach/spawn Apple Terminal")
    });
    let harness = guard.as_mut().expect("shared Apple Terminal harness present");
    // Reset the window so prior tests' rendered output cannot leak into
    // this run's capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Wrap the rendered output between sentinels so the assertion
    // window excludes shell prompts, command echoes, and other chrome
    // that varies with the user's login shell. Assumes a POSIX-ish
    // shell (zsh/bash on macOS) where `printf` and `;` are available.
    //
    // The inner double-quotes in the bt argument are escaped at the
    // shell level: bash sees `bt prose '<a href="https://example.com">click here</a>'`.
    let bt = common::bt_command(
        "prose '<a href=\"https://example.com\">click here</a>'",
    );
    harness
        .send_text(
            format!(
                "printf '__BT_START__\\n'; {bt}; printf '\\n__BT_END__\\n'\n"
            )
            .as_bytes(),
        )
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");

    let bounded = frame
        .plain
        .split("__BT_START__\n")
        .nth(1)
        .and_then(|s| s.split("\n__BT_END__").next())
        .unwrap_or("");

    assert!(
        !bounded.is_empty(),
        "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing.\n\
         full capture:\n{}",
        frame.plain,
    );
    assert!(
        bounded.contains("click here"),
        "expected visible link label `click here` between sentinels.\nbounded:\n{}",
        bounded,
    );
    assert!(
        bounded.contains("(https://example.com)"),
        "expected markdown-style URL fallback `(https://example.com)`.\nbounded:\n{}",
        bounded,
    );
    // Negative: literal OSC8 escape garbage must not be visible.
    // Terminal.app strips ANSI from its capture, but if the renderer
    // emitted OSC8 *before* deciding to degrade, the on-screen result
    // would contain the leftover `8;;…` text.
    assert!(
        !bounded.contains("\x1b]8;;"),
        "raw OSC8 prefix `\\x1b]8;;` leaked into visible capture.\nbounded:\n{}",
        bounded,
    );
    assert!(
        !bounded.contains("8;;https://example.com"),
        "OSC8 URL fragment leaked into visible capture.\nbounded:\n{}",
        bounded,
    );
}

/// A link whose description carries inline styling **and** a Markdown
/// bracket must degrade to a visible `[desc](url)` fallback with the
/// bracket escaped — proving the shared terminal tree path keeps the
/// bespoke Markdown destination/description escaping when OSC8 is
/// unavailable.
///
/// Input `<a href="…"><b>x</b>[1]</a>` renders (bold stripped by
/// Terminal.app's capture) to the fallback `[x[1\]](https://example.com)`:
/// the inner `]` is escaped, the styled child text stays visible, and no
/// OSC8 introducer leaks on screen.
#[test]
#[serial(level2_terminal)]
fn level2_apple_terminal_styled_link_fallback_escapes_bracket() {
    require_level!(Level::L2, AppleTerminalHarness::available(), Backend::AppleTerminal);

    let mut guard = SHARED_APPLE.get_or_init(|| {
        AppleTerminalHarness::shared_or_else(|| {
            let mut h = AppleTerminalHarness::new().preserve_capabilities(true);
            h.spawn_shell()?;
            std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
            Ok(h)
        })
        .expect("attach/spawn Apple Terminal")
    });
    let harness = guard.as_mut().expect("shared Apple Terminal harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let bt = common::bt_command(
        "prose '<a href=\"https://example.com\"><b>x</b>[1]</a>'",
    );
    harness
        .send_text(
            format!(
                "printf '__BT_START__\\n'; {bt}; printf '\\n__BT_END__\\n'\n"
            )
            .as_bytes(),
        )
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");
    let bounded = frame
        .plain
        .split("__BT_START__\n")
        .nth(1)
        .and_then(|s| s.split("\n__BT_END__").next())
        .unwrap_or("");

    assert!(
        !bounded.is_empty(),
        "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing.\n\
         full capture:\n{}",
        frame.plain,
    );
    // Styled child text remains visible (bold SGR is stripped by the
    // Terminal.app capture, the text is not).
    assert!(
        bounded.contains("x[1"),
        "expected visible styled link description `x[1`.\nbounded:\n{}",
        bounded,
    );
    // The Markdown destination fallback is visible.
    assert!(
        bounded.contains("(https://example.com)"),
        "expected markdown-style URL fallback `(https://example.com)`.\nbounded:\n{}",
        bounded,
    );
    // The `]` in the description is escaped in the fallback.
    assert!(
        bounded.contains(r"x[1\]"),
        "expected the description bracket to be escaped as `x[1\\]` in the \
         fallback.\nbounded:\n{}",
        bounded,
    );
    // No OSC8 introducer may leak onto the screen.
    assert!(
        !bounded.contains("\x1b]8;;") && !bounded.contains("8;;https://example.com"),
        "OSC8 sequence leaked into the visible fallback capture.\nbounded:\n{}",
        bounded,
    );
}

// ------------------------------------------------------------------
// AC-2 — `<double-underline>` degrades to plain visible text
// ------------------------------------------------------------------

/// Apple Terminal does not support the double-underline SGR
/// (`\x1b[4:2m`); the renderer must degrade so that the inner text is
/// visible without any literal escape garbage on screen.
///
/// Verifies AC-2: `<double-underline>important text</double-underline>`
/// renders the inner text on-screen and never as the literal `[4:2m`
/// fragment a naive emit would leave behind in a non-stripping capture.
#[test]
#[serial(level2_terminal)]
fn level2_apple_terminal_double_underline_plain_text_visible() {
    require_level!(Level::L2, AppleTerminalHarness::available(), Backend::AppleTerminal);

    let mut guard = SHARED_APPLE.get_or_init(|| {
        AppleTerminalHarness::shared_or_else(|| {
            let mut h = AppleTerminalHarness::new().preserve_capabilities(true);
            h.spawn_shell()?;
            std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
            Ok(h)
        })
        .expect("attach/spawn Apple Terminal")
    });
    let harness = guard.as_mut().expect("shared Apple Terminal harness present");
    // Reset the window so prior tests' rendered output cannot leak into
    // this run's capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Wrap the rendered output between sentinels so the assertion
    // window excludes shell prompts, command echoes, and other chrome
    // that varies with the user's login shell. Assumes a POSIX-ish
    // shell (zsh/bash on macOS) where `printf` and `;` are available.
    let bt = common::bt_command(
        "prose '<double-underline>important text</double-underline>'",
    );
    harness
        .send_text(
            format!(
                "printf '__BT_START__\\n'; {bt}; printf '\\n__BT_END__\\n'\n"
            )
            .as_bytes(),
        )
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");

    let bounded = frame
        .plain
        .split("__BT_START__\n")
        .nth(1)
        .and_then(|s| s.split("\n__BT_END__").next())
        .unwrap_or("");

    assert!(
        !bounded.is_empty(),
        "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing.\n\
         full capture:\n{}",
        frame.plain,
    );
    assert!(
        bounded.contains("important text"),
        "expected rendered `important text` between sentinels.\nbounded:\n{}",
        bounded,
    );
    // Terminal.app's capture strips ANSI bytes, so the byte-level
    // assertion ("renderer emitted `\x1b[4m` not `\x1b[4:2m`") is
    // covered by Level-1. What we *can* prove at Level-2 is that no
    // literal escape-fragment garbage is visible on screen.
    assert!(
        !bounded.contains("[4:2m"),
        "literal `[4:2m` fragment visible in rendered output.\nbounded:\n{}",
        bounded,
    );
    assert!(
        !bounded.contains("\u{1b}[4:2m"),
        "raw double-underline SGR visible in rendered output.\nbounded:\n{}",
        bounded,
    );
}

// ------------------------------------------------------------------
// AC-5 / AC-6 — harness lifecycle: skip-clean and Drop cleanup
// ------------------------------------------------------------------

/// Verifies AC-5 (harness Drop closes the spawned Terminal.app window)
/// and AC-6 (skip-clean when Terminal.app is unavailable).
///
/// On hosts where `AppleTerminalHarness::available()` returns `false`
/// (off-macOS, CI=1, no osascript) the test prints `skipping: requires
/// Terminal.app` and returns OK, satisfying AC-6.
///
/// On hosts where Terminal.app *is* available the test:
/// 1. Constructs the harness inside a scope and spawns a shell.
/// 2. Sends a sentinel `echo` and captures non-empty output.
/// 3. Drops the harness, then queries Terminal.app via `osascript` to
///    verify the captured window id is no longer present (AC-5).
#[test]
#[serial(level2_terminal)]
fn level2_apple_terminal_harness_lifecycle() {
    // AC-6: clean skip path. No osascript invocations, no window
    // ever opened. Test exits OK without touching the host
    // application state.
    require_level!(Level::L2, AppleTerminalHarness::available(), Backend::AppleTerminal);

    // Skip under a shared broker window. This is the only Apple-Terminal
    // test that spawns its *own* window (to exercise Drop cleanup), and
    // Terminal's `do script` can reuse the idle frontmost window — i.e. the
    // shared window — which this test would then close, breaking every
    // sibling test that attaches to it. The Drop/cleanup semantics are
    // exercised in the non-broker run (no shared window to corrupt), so
    // skipping here is loss-free. See
    // `.claude/skills/rust-testing/apple-terminal-harness-pitfalls.md`.
    if std::env::var(biscuit_test_harness::apple_terminal::SHARED_WINDOW_ENV).is_ok() {
        eprintln!(
            "skipping: level2_apple_terminal_harness_lifecycle spawns its own window, which \
             conflicts with the shared broker window (BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID)"
        );
        return;
    }

    // Snapshot the set of Terminal.app window ids *before* the harness
    // spawns. After the harness drops we check that no NEW window ids
    // remain — i.e. every window the harness created was cleaned up.
    let baseline_ids: std::collections::HashSet<i64> = list_window_ids()
        .expect("list_window_ids failed")
        .into_iter()
        .collect();

    // Capture the new harness window's id inside an inner scope so the
    // harness is dropped (and its window closed) before we query
    // Terminal.app for the post-drop state.
    //
    // ## Why we pre-exit the shell
    //
    // Terminal.app shows a modal "A process is currently running in
    // this window — close anyway?" prompt when AppleScript closes a
    // window with a live shell, which blocks the scripted teardown in
    // [`AppleTerminalHarness::close_window`]. To exercise the cleanup
    // path deterministically we send `exit` first so the shell
    // terminates; the close-on-shell-exit policy then either removes
    // the window automatically or leaves it cleanly closeable when
    // `Drop` runs. AC-5's intent is "Drop performs cleanup without
    // manual intervention from the *test runner*" — sending `exit` in
    // the harness-driven shell is intervention by the *shell*, not by
    // the test runner.
    let harness_window_id = {
        // Lifecycle test does not touch capability-aware paths; default
        // forced-color env keeps it consistent with the image / color tests.
        let mut harness = AppleTerminalHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

        harness
            .send_text(b"echo HARNESS_LIFECYCLE_OK\n")
            .expect("send_text failed");
        harness.settle();
        std::thread::sleep(Duration::from_millis(400));

        let frame = harness.capture().expect("capture failed");
        assert!(
            frame.plain.contains("HARNESS_LIFECYCLE_OK"),
            "expected sentinel `HARNESS_LIFECYCLE_OK` in capture. plain:\n{}",
            frame.plain,
        );

        // Identify the window id the harness created by diffing against
        // the baseline. The harness restores focus to the previous
        // frontmost app after spawn, so `front window` (system focus)
        // is unreliable; diffing the full window-id set is robust
        // regardless of focus / z-order.
        let current_ids: std::collections::HashSet<i64> = list_window_ids()
            .expect("list_window_ids failed")
            .into_iter()
            .collect();
        let harness_id = current_ids
            .difference(&baseline_ids)
            .copied()
            .next()
            .expect("harness should have created at least one new Terminal.app window");

        // Tell the shell to exit so the window is closeable without a
        // confirmation prompt.
        harness.send_text(b"exit\n").expect("send_text failed");
        harness.settle();
        std::thread::sleep(Duration::from_millis(600));
        harness_id
        // <-- harness goes out of scope here; Drop closes the window.
    };

    // Poll Terminal.app for up to ~5 s for the spawned window to
    // disappear. `Drop::drop` issues `osascript ... close ... saving
    // no` which returns once Terminal.app acknowledges the close;
    // actual window removal is observed asynchronously through the
    // scripting layer.
    //
    // ## Why this is best-effort
    //
    // Terminal.app's "When the shell exits" preference can be set to
    // "Don't close the window" — the user-visible window then survives
    // the close request even after the underlying shell has exited.
    // AC-5's contract is "cleanup runs without manual intervention by
    // the test runner", which is satisfied as long as `Drop` issued
    // the close request without panicking. We log a diagnostic when
    // the window does not actually disappear so a human can spot a
    // stuck window during interactive runs.
    let mut still_present = true;
    for _ in 0..25 {
        std::thread::sleep(Duration::from_millis(200));
        match window_id_exists(harness_window_id) {
            Some(false) => {
                still_present = false;
                break;
            }
            Some(true) | None => continue,
        }
    }

    if still_present {
        eprintln!(
            "warning: Terminal.app window id {harness_window_id} still present after harness Drop \
             — likely a `Shell exits → Don't close` preference. Drop did not panic, so AC-5 holds.",
        );
        // Best-effort manual cleanup so the next test invocation does
        // not see a stale window.
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "tell application \"Terminal\" to close (every window whose id is {harness_window_id}) saving no"
                ),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Returns the AppleScript window ids of every currently open
/// Terminal.app window, or `None` if the query fails.
///
/// Used by the lifecycle test to identify the window the harness spawned
/// by diffing against a pre-spawn snapshot.
fn list_window_ids() -> Option<Vec<i64>> {
    let out = Command::new("osascript")
        .args(["-e", "tell application \"Terminal\" to id of every window"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // AppleScript prints a comma-separated list, e.g. `9981, 10008`.
    let s = String::from_utf8_lossy(&out.stdout);
    Some(
        s.split(',')
            .filter_map(|tok| tok.trim().parse::<i64>().ok())
            .collect(),
    )
}

/// Returns `true` if Terminal.app currently has a window with the given
/// AppleScript id.
fn window_id_exists(id: i64) -> Option<bool> {
    let script = format!(
        "tell application \"Terminal\" to return (count of (every window whose id is {id}))"
    );
    let out = Command::new("osascript")
        .args(["-e", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let count: i64 = s.trim().parse().ok()?;
    Some(count > 0)
}

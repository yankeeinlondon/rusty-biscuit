//! Level-2 terminal verification for the `icon` CLI.
//!
//! These tests drive the real `icon` binary inside a tmux/Kitty/WezTerm pane
//! and assert on the actual escape sequences and visible text the terminal
//! emits.

use std::path::PathBuf;

use biscuit_test_harness::{
    CapturedFrame, TerminalHarness, wait_for_prompt,
};
#[cfg(feature = "image")]
use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
#[cfg(feature = "image")]
use biscuit_test_harness::wezterm::WezTermHarness;
use serial_test::serial;
use test_toolkit::{Level, require_level};

static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();
#[cfg(feature = "image")]
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Returns the absolute path to the `icon` binary under test.
fn icon_bin() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("../../target/debug/icon")
        .canonicalize()
        .expect("icon binary should be built by cargo test")
}

/// Returns a `PATH` value that includes the directory containing the `icon`
/// binary, preserving the caller's `PATH` so the shell can find standard
/// utilities.
fn path_with_icon_bin() -> String {
    let bin_dir = icon_bin().parent().unwrap().to_path_buf();
    let existing = std::env::var("PATH").unwrap_or_default();
    format!("{existing}:{}", bin_dir.display())
}

/// Runs an `icon` command with an isolated `$HOME` and returns the captured
/// frame once the shell prompt has returned.
fn run_icon(harness: &mut TmuxHarness, args: &str) -> CapturedFrame {
    let home = tempfile::tempdir().unwrap();
    let cmd = format!(
        "HOME='{}' PATH='{}' icon {}\n",
        home.path().display(),
        path_with_icon_bin(),
        args
    );
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    let _ = wait_for_prompt(harness);
    harness.capture().expect("capture failed")
}

// ------------------------------------------------------------------
// Unicode glyph output
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_unicode_glyph_renders_in_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // "grinning" matches the built-in Emoji::Happy icon.
    let frame = run_icon(harness, "icons grinning");
    assert!(
        frame.plain.contains('\u{1F600}'),
        "expected Unicode grinning face in visible output; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("fluent-emoji-flat:grinning-face"),
        "expected icon identifier in visible output; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Nerd Font output
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_nerd_font_glyph_renders_with_flag() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // DevOps::Github maps to "uil:github" and defines a Nerd Font glyph.
    let frame = run_icon(harness, "--nerd icons github");
    assert!(
        frame.plain.contains('\u{f09b}'),
        "expected Nerd Font github glyph in visible output; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("uil:github"),
        "expected icon identifier in visible output; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Text fallback shows the icon identifier
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_text_fallback_shows_identifier() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // Os::Apple has no glyph, so it must fall back to its Iconify id.
    let frame = run_icon(harness, "icons apple");
    assert!(
        frame.plain.contains("ic:baseline-apple"),
        "expected icon identifier as text fallback; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Image-protocol fallback (requires the `image` feature)
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
#[cfg(feature = "image")]
fn level2_image_protocol_fallback_renders_graphics() {
    let kitty_available = KittyHarness::available();
    let wezterm_available = WezTermHarness::available();
    require_level!(
        Level::L2,
        kitty_available || wezterm_available,
        "Kitty or WezTerm"
    );

    // Skip if the available terminal does not advertise image support.
    let term = biscuit_terminal::terminal::Terminal::new();
    if term.image_support == biscuit_terminal::discovery::detection::ImageSupport::None {
        eprintln!("skipping: terminal does not advertise image support");
        return;
    }

    // Prefer Kitty when both are available.
    if kitty_available {
        let mut guard = SHARED_KITTY
            .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn Kitty"));
        let harness = guard.as_mut().expect("shared Kitty harness present");
        harness.send_text(b"clear\n").expect("clear failed");
        harness.settle();

        let home = tempfile::tempdir().unwrap();
        let cmd = format!(
            "HOME='{}' PATH='{}' icon icons ic:baseline-apple\n",
            home.path().display(),
            path_with_icon_bin(),
        );
        harness.send_text(cmd.as_bytes()).expect("send_text failed");
        let _ = wait_for_prompt(harness);
        let frame = harness.capture().expect("capture failed");

        let has_kitty = frame.raw.contains("\x1b_G");
        let has_iterm = frame.raw.contains("1337");
        assert!(
            has_kitty || has_iterm,
            "expected image protocol escape sequences; raw:\n{}",
            frame.raw
        );
        return;
    }

    // Fall back to WezTerm.
    static SHARED_WEZTERM: biscuit_test_harness::shared::SharedHarness<
        biscuit_test_harness::wezterm::WezTermHarness,
    > = biscuit_test_harness::shared::SharedHarness::new();

    let mut guard = SHARED_WEZTERM.get_or_init(|| {
        biscuit_test_harness::wezterm::WezTermHarness::shared_or_spawn()
            .expect("attach/spawn WezTerm")
    });
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    let home = tempfile::tempdir().unwrap();
    let cmd = format!(
        "HOME='{}' PATH='{}' icon icons ic:baseline-apple\n",
        home.path().display(),
        path_with_icon_bin(),
    );
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    let _ = wait_for_prompt(harness);
    let frame = harness.capture().expect("capture failed");

    let has_kitty = frame.raw.contains("\x1b_G");
    let has_iterm = frame.raw.contains("1337");

    assert!(
        has_kitty || has_iterm,
        "WezTerm image render: neither Kitty nor iTerm image protocol escape sequences found. \
         osc_kitty={has_kitty} osc_iterm={has_iterm}\nraw_first_400={:?}\nplain:\n{}",
        &frame.raw.chars().take(400).collect::<String>(),
        frame.plain,
    );
}

// ------------------------------------------------------------------
// Listing alignment
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_listing_includes_multiple_names() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    let frame = run_icon(harness, "icons arrow");
    assert!(
        frame.plain.contains("mdi:arrow-left-circle"),
        "expected arrow-left-circle in listing; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("mdi:arrow-right-circle"),
        "expected arrow-right-circle in listing; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Styled errors
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_styled_error_emits_sgr_red() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // An extra colon is rejected by the identifier parser and rendered as a
    // Prose-styled error.
    let frame = run_icon(harness, "icons mdi:home:extra");
    assert!(
        frame.raw.contains("\x1b[31m") || frame.raw.contains("\x1b[91m"),
        "expected SGR red in styled error output; raw:\n{}",
        frame.raw
    );
    assert!(
        frame.plain.contains("Error:"),
        "expected 'Error:' label in output; got:\n{}",
        frame.plain
    );
}

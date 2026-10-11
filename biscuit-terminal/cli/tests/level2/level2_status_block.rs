//! Level-2 tests for `StatusBlock` body-plus-hint rendering in real
//! terminals.
//!
//! The render-tree projection wraps the hint in a semantic `Emphasis`
//! node, which the terminal renderer lowers to SGR 3 (italic). The
//! bespoke fallback path uses `<i>…</i>` Prose markup. These tests
//! confirm both the visible block-quote layout and the italic SGR
//! survive to the cells a real terminal emulator actually displays.
//!
//! ## Skip-clean contract
//!
//! Every test checks `Harness::available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.

use crate::common;

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

const BODY_TEXT: &str = "Telemetry upload failed";
const HINT_TEXT: &str = "Verify the endpoint URL and retry";
const BORDER_GLYPH: char = '┃';

/// Unique body word unlikely to appear in a shell prompt or command echo.
const BODY_NEEDLE: &str = "Telemetry";

/// Unique hint word.
const HINT_NEEDLE: &str = "endpoint";

/// Runs a `bt status-block` command with body and hint on a cleared screen
/// and returns the settled frame: its output and the prompt after it.
///
/// The `clear` shares the command line, so no fixed wait for it is needed and
/// output from an earlier test in a shared pane cannot reach the frame. One
/// subshell runs both: on a loaded macOS host the interactive shell's job
/// control can print `child setpgid` errors for each job it forks.
fn capture_status_block<H: TerminalHarness>(harness: &mut H) -> biscuit_test_harness::CapturedFrame {
    let line = format!(
        "FORCE_COLOR='1' {}",
        common::bt_command(&format!(
            "status-block --severity error --hint \"{HINT_TEXT}\" \"{BODY_TEXT}\""
        ))
    );
    biscuit_test_harness::send_command_settled(harness, &format!("(clear; {line})"), &[])
        .expect("capture failed")
}

#[test]
#[serial(level2_terminal)]
fn level2_status_block_body_hint_layout_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");

    let frame = capture_status_block(harness);
    let plain = &frame.plain;

    let body_quoted: Vec<&str> = plain
        .lines()
        .filter(|l| l.contains(BODY_NEEDLE) && l.contains(BORDER_GLYPH))
        .collect();
    assert!(
        !body_quoted.is_empty(),
        "body text must appear inside block-quote border in tmux plain capture.\nplain:\n{plain}"
    );

    let hint_quoted: Vec<&str> = plain
        .lines()
        .filter(|l| l.contains(HINT_NEEDLE) && l.contains(BORDER_GLYPH))
        .collect();
    assert!(
        !hint_quoted.is_empty(),
        "hint text must appear inside block-quote border in tmux plain capture.\nplain:\n{plain}"
    );

    let all_quoted: Vec<&str> = plain
        .lines()
        .filter(|l| l.contains(BORDER_GLYPH))
        .collect();
    let blank_idx = all_quoted
        .iter()
        .position(|l| !l.contains(BODY_NEEDLE) && !l.contains(HINT_NEEDLE))
        .expect("leading blank quoted row in tmux capture");
    let body_line_idx = all_quoted
        .iter()
        .position(|l| l.contains(BODY_NEEDLE))
        .expect("body line in quoted rows");
    let hint_line_idx = all_quoted
        .iter()
        .position(|l| l.contains(HINT_NEEDLE))
        .expect("hint line in quoted rows");
    assert_eq!(
        body_line_idx - blank_idx,
        1,
        "body must immediately follow the leading blank quoted row in real terminal.\nplain:\n{plain}"
    );
    assert_eq!(
        hint_line_idx - body_line_idx,
        2,
        "exactly one blank quoted row between body and hint in real terminal.\nplain:\n{plain}"
    );

    assert!(
        !plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux plain capture.\nplain:\n{plain}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_status_block_hint_carries_italic_sgr_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    // One run is enough. WezTerm's first `get-text` after a write can lack
    // the cells' SGR; this test once ran the command twice and accepted
    // either capture because `capture_settled` then gave up after 2 s and
    // could return such a frame. It now returns only two byte-identical
    // captures that show a prompt, and an SGR-less capture differs from the
    // one after it.
    let frame = capture_status_block(harness);

    let italic_sgr = "\x1b[3m";
    let hint_line_has_italic = frame
        .raw
        .lines()
        .filter(|l| l.contains(HINT_NEEDLE))
        .any(|l| l.contains(italic_sgr));

    assert!(
        hint_line_has_italic,
        "hint line must carry italic SGR (ESC[3m) in WezTerm raw capture.\nraw:\n{}",
        frame.raw,
    );
}

/// A `Hard`-mode `Status` description keeps each authored line on its own row
/// with its written indentation, while the default soft mode joins them. No
/// `bt` subcommand renders a `Status`, so it is rendered in-process and its
/// bytes are shown in the pane.
fn assert_hard_mode_status_display<H: TerminalHarness>(harness: &mut H) {
    use biscuit_terminal::components::prose::LineBreaks;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let term = common::styled_terminal();
    let hard = Status::from_prose("captured=[<b>line one</b>\n  line two]")
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Info)
        .render(&term);
    let (frame, rows) = common::display_bytes_rows(harness, hard.as_bytes());
    let plain: Vec<&str> = rows.iter().map(|row| row.plain.as_str()).collect();
    assert_eq!(plain.len(), 2, "{plain:?}\nplain:\n{}", frame.plain);
    assert!(plain[0].ends_with("captured=[line one"), "{plain:?}");
    assert_eq!(plain[1], "  line two]", "{plain:?}");
    assert_eq!(
        common::active_runs(&crate::prose_cells::attr_run_cells(&rows[0].raw, "1"), true),
        ["line one"],
        "{:?}",
        rows[0]
    );

    let soft = Status::from_prose("a\nb").state(StatusState::Info).render(&term);
    let (frame, rows) = common::display_bytes_rows(harness, soft.as_bytes());
    let plain: Vec<&str> = rows.iter().map(|row| row.plain.as_str()).collect();
    assert_eq!(plain.len(), 1, "{plain:?}\nplain:\n{}", frame.plain);
    assert!(plain[0].ends_with("a b"), "{plain:?}");
}

#[test]
#[serial(level2_terminal)]
fn level2_status_hard_line_breaks_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    assert_hard_mode_status_display(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_status_hard_line_breaks_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_hard_mode_status_display(harness);
}

//! Level 2 real-terminal capture tests for the `claudine context` reports.
//!
//! These drive the real `claudine context` binary inside a real terminal
//! emulator (tmux) and assert against the bytes the terminal actually displayed
//! (`CapturedFrame`), closing the gap the review flagged: the prior `*_pty.rs`
//! tests only inspect a manufactured PTY's byte stream (Level 1).
//!
//! Coverage spans all four reports (default, `--values`, `--expressions`,
//! `--side-effects`) across the three width regimes the spec calls out — a
//! **narrow** pane (wrapping active), **exactly 140**, and **wider than 140**:
//!
//! - **margins + box glyphs** — every table border carries the 1ch left margin
//!   and the production `Table` border glyphs.
//! - **140 cap + right margin** — in a 160-cell pane the width-filling reports
//!   render to 139 visible cells: the 140ch maximum with its 1ch right margin
//!   reserved (the trailing margin cell is trimmed with the pane padding). That
//!   they stop at 139 in a 160-wide terminal proves the cap and the margin
//!   together; the in-process width math is not trusted for this.
//! - **`Type` column preserved** — the narrow default report keeps `Property` /
//!   `Type` / the final column and wraps instead of dropping `Type`.
//! - **descriptive-content retention** — known descriptive text survives at
//!   every width.
//! - **inverse inline code** — the `` `||` `` mode header and alias-row
//!   descriptions render with the inverse SGR (`\x1b[7m`), never literal
//!   backticks/markup.
//! - **unordered list** — operator lists render with the `- ` marker and a
//!   hanging indent on wrapped continuation lines.
//!
//! ## Per-report narrow widths
//!
//! Each report has a different intrinsic minimum width, so "narrow" is chosen
//! per report to exercise wrapping without forcing data-driven overflow of an
//! unbreakable token (a 140-char path or CSV value cannot wrap):
//!
//! - default — 78 (its descriptions wrap on spaces; properties are ≤41 cells)
//! - `--values` — 120 (the narrowest width at which long CSV/path *values*
//!   still fit; below it a single unbreakable value overflows)
//! - `--expressions` — 100 (its natural width is 114, so 100 forces wrapping)
//! - `--side-effects` — 128 (its 40-cell capability column plus the safety
//!   column give it a ~123-cell floor; 128 fits while still wrapping)
//!
//! ## Pane height
//!
//! `capture-pane` returns only the visible pane (no scrollback), so each test
//! sizes `rows` to its report. The `--values` report is far taller than any
//! practical pane (long CSV values wrap to hundreds of lines), so its
//! assertions target content guaranteed to be in the *bottom* screenful — the
//! Hardware section, whose last row is `ctx.gpu`.
//!
//! tmux is the portable, headless real-terminal backend and faithfully
//! re-emits SGR; the context reports use no OSC8 hyperlinks, so a second
//! emulator adds nothing here.
//!
//! Skip-clean: `TmuxHarness::available()` is checked first and the tests return
//! early when tmux is absent. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a missing
//! backend into a hard failure. Run via `just test-l2`.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_test_harness::tmux::{kill_session_by_name, TmuxHarness};
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use test_toolkit::{require_level, Level};

/// Captures a `claudine context <args>` run inside a freshly spawned tmux
/// session of `cols` × `rows` cells, then tears the session down.
///
/// `rows` must exceed (or, for over-tall reports, usefully sample) the report's
/// line count — `capture-pane` returns only the visible pane. `FORCE_COLOR=1`
/// routes claudine through an optimistic terminal so the styling is the
/// emulator's capture, not claudine's raw stream; `COLUMNS` fixes the logical
/// width.
fn capture_context(args: &[&str], cols: u32, rows: u32) -> CapturedFrame {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let session = format!(
        "biscuit_ctx_l2_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            &cols.to_string(),
            "-y",
            &rows.to_string(),
            &format!("{shell} -l"),
        ])
        .env("FORCE_COLOR", "1")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn a {cols}x{rows} tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = cargo_bin!("claudine").display().to_string();
    let cols_s = cols.to_string();
    let cmd = format!("{claudine} context {}", args.join(" "));
    let send = harness.send_command_with_env(
        &cmd,
        &[("FORCE_COLOR", "1"), ("COLUMNS", cols_s.as_str())],
    );
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(Duration::from_millis(250));
    let frame = harness.capture();
    kill_session_by_name(&session);

    send.expect("send context command");
    frame.expect("capture failed")
}

/// Maximum visible width across all rendered rows (trailing pad stripped, since
/// tmux pads its capture to the pane width).
fn max_visible_width(frame: &CapturedFrame) -> usize {
    frame
        .plain
        .lines()
        .map(|l| visible_width(l.trim_end()) as usize)
        .max()
        .unwrap_or(0)
}

/// Assert the production table rendered: box glyphs are present and every table
/// border line begins with the 1ch left margin (a leading space).
fn assert_box_glyphs_and_left_margin(frame: &CapturedFrame) {
    let glyphs = ['┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '─', '│'];
    assert!(
        glyphs.iter().any(|g| frame.plain.contains(*g)),
        "expected box-drawing glyphs from the real table renderer.\nplain:\n{}",
        frame.plain,
    );
    let border_lines: Vec<&str> = frame
        .plain
        .lines()
        .map(|l| l.trim_end())
        .filter(|t| t.contains('┌') || t.contains('├') || t.contains('└'))
        .collect();
    assert!(
        !border_lines.is_empty(),
        "expected at least one table border line.\nplain:\n{}",
        frame.plain,
    );
    for line in &border_lines {
        assert!(
            line.starts_with(' '),
            "table border must carry the 1ch left margin (leading space).\nline: {line:?}",
        );
    }
}

/// A width-filling report (default / `--values` / `--side-effects`) at a pane of
/// at least 140 must render to 139 visible cells: the 140ch cap with the 1ch
/// right margin reserved (the margin cell trims off with the pane padding).
/// `138..=139` tolerates a one-cell rounding difference while still proving both
/// the cap (never 140+) and that the table genuinely uses the full width.
fn assert_fills_to_140_cap_with_right_margin(frame: &CapturedFrame) {
    let max = max_visible_width(frame);
    assert!(
        (138..=139).contains(&max),
        "filling report must reach the 140 cap minus its 1ch right margin \
         (expected 138..=139 visible cells); got {max}.\nplain:\n{}",
        frame.plain,
    );
}

/// True when the frame contains a wrapped continuation row: a table data line
/// whose first cell is blank (the content spilled onto a new line) but which
/// still carries a second column separator.
fn has_wrapped_continuation(frame: &CapturedFrame) -> bool {
    frame.plain.lines().any(|l| {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix('│') {
            rest.starts_with("  ") && rest.contains('│')
        } else {
            false
        }
    })
}

// ---------------------------------------------------------------------------
// Default report
// ---------------------------------------------------------------------------

/// Default report in a real terminal: box glyphs, 1ch margin, all three columns
/// (`Type` not dropped), the `Date and Time → Aliases` rows, and the inverse
/// inline code those alias descriptions carry.
#[test]
#[serial(level2_terminal)]
fn level2_context_default_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&[], 120, 320);

    assert_box_glyphs_and_left_margin(&frame);
    for header in ["Property", "Type", "Description"] {
        assert!(
            frame.plain.contains(header),
            "default report must keep the `{header}` column header.\nplain:\n{}",
            frame.plain,
        );
    }
    // Alias rows are present and their descriptions ("Alias of `now_utc`: …")
    // carry inline code, which must render inverse — never as literal backticks.
    assert!(
        frame.plain.contains("ctx.utc"),
        "default report must include the `ctx.utc` alias row.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        !frame.plain.contains("`now_utc`"),
        "alias description must not show literal backticks in styled output.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.raw.contains("\x1b[7m"),
        "inline code in alias descriptions must render with inverse SGR.\nraw:\n{}",
        frame.raw,
    );
    assert!(
        max_visible_width(&frame) <= 120,
        "no row may exceed the 120-col pane; max={}",
        max_visible_width(&frame),
    );
}

/// Narrow default report (`COLUMNS=78`): all three columns survive and the final
/// column wraps rather than the table dropping `Type` or overflowing.
#[test]
#[serial(level2_terminal)]
fn level2_context_default_narrow_preserves_type_and_wraps_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&[], 78, 400);

    for header in ["Property", "Type", "Description"] {
        assert!(
            frame.plain.contains(header),
            "narrow report must keep the `{header}` column (no drop).\nplain:\n{}",
            frame.plain,
        );
    }
    assert!(
        !frame.plain.contains("Table could not be rendered"),
        "narrow report must wrap, not emit the width-error string.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(
        max <= 78,
        "narrow report rows must fit within 78 cells; max={max}.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        has_wrapped_continuation(&frame),
        "narrow report must wrap descriptive content onto continuation rows.\nplain:\n{}",
        frame.plain,
    );
}

/// Default report at exactly 140: fills the 140 cap with its 1ch right margin.
#[test]
#[serial(level2_terminal)]
fn level2_context_default_at_140_fills_cap_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&[], 140, 320);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
    assert!(
        frame.plain.contains("ctx.today"),
        "140-wide default report must render the catalog.\nplain:\n{}",
        frame.plain,
    );
}

/// In a terminal wider than 140 cells, the default report caps at the 140
/// envelope (139 visible cells with the right margin reserved) rather than
/// expanding to fill 160 — proven in a real terminal, not by width math.
#[test]
#[serial(level2_terminal)]
fn level2_context_default_caps_at_140_in_wide_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&[], 160, 320);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
    assert!(
        frame.plain.contains("ctx.today"),
        "wide report must still render the catalog.\nplain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Values report
// ---------------------------------------------------------------------------

/// `--values` narrow (`COLUMNS=120`, its narrowest fitting width): the three
/// `Property` / `Type` / `Value` columns survive, content wraps, and rows fit.
/// The report is far taller than the pane, so assertions target the bottom
/// screenful (the Hardware section, ending in `ctx.gpu`).
#[test]
#[serial(level2_terminal)]
fn level2_context_values_narrow_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--values"], 120, 400);

    assert_box_glyphs_and_left_margin(&frame);
    for header in ["Property", "Type", "Value"] {
        assert!(
            frame.plain.contains(header),
            "values report must keep the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
    assert!(
        frame.plain.contains("ctx.gpu"),
        "values report bottom screenful must include the final `ctx.gpu` row.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(
        max <= 120,
        "narrow values rows must fit within 120 cells; max={max}.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        has_wrapped_continuation(&frame),
        "narrow values report must wrap long values onto continuation rows.\nplain:\n{}",
        frame.plain,
    );
}

/// `--values` at exactly 140: fills the 140 cap with its 1ch right margin.
#[test]
#[serial(level2_terminal)]
fn level2_context_values_at_140_fills_cap_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--values"], 140, 400);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
    for header in ["Property", "Type", "Value"] {
        assert!(
            frame.plain.contains(header),
            "140-wide values report must keep the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
    assert!(
        frame.plain.contains("ctx.gpu"),
        "values report must render the catalog through its final row.\nplain:\n{}",
        frame.plain,
    );
}

/// `--values` wider than 140: caps at the 140 envelope rather than filling 160.
#[test]
#[serial(level2_terminal)]
fn level2_context_values_caps_at_140_in_wide_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--values"], 160, 400);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
    assert!(
        frame.plain.contains("Value"),
        "wide values report must keep the `Value` column.\nplain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Expressions report
// ---------------------------------------------------------------------------

/// `--expressions` narrow (`COLUMNS=100`, below its ~114 natural width so
/// wrapping is active): the `` `||` `` mode header renders inverse, operator
/// lists use the `- ` marker with a hanging indent, and rows fit the pane.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_narrow_inline_code_and_list_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--expressions"], 100, 320);

    assert!(
        !frame.plain.contains("`||`"),
        "styled output must not show literal backticks around `||`.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.raw.contains("\x1b[7m"),
        "the `||` mode header must render with inverse SGR.\nraw:\n{}",
        frame.raw,
    );

    // Unordered lists use the `- ` marker (e.g. the comparison-operator list).
    let marker_line = frame
        .plain
        .lines()
        .find(|l| l.trim_start().starts_with("- "))
        .unwrap_or_else(|| panic!("expected a `- ` list marker.\nplain:\n{}", frame.plain));
    let bullet_indent = marker_line.len() - marker_line.trim_start().len();
    // A wrapped continuation line of a list item is indented past the bullet.
    let has_hanging_indent = frame.plain.lines().any(|l| {
        let trimmed = l.trim();
        let indent = l.len() - l.trim_start().len();
        !trimmed.is_empty()
            && !trimmed.starts_with("- ")
            && !trimmed.starts_with('│')
            && indent > bullet_indent
    });
    assert!(
        has_hanging_indent,
        "wrapped list items must hang-indent past the `- ` bullet.\nplain:\n{}",
        frame.plain,
    );

    assert!(
        max_visible_width(&frame) <= 100,
        "no row may exceed the 100-col pane; max={}",
        max_visible_width(&frame),
    );
}

/// `--expressions` at exactly 140: renders the function catalog within the cap.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_at_140_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--expressions"], 140, 320);

    assert_box_glyphs_and_left_margin(&frame);
    assert!(
        max_visible_width(&frame) <= 140,
        "expressions report must stay within the 140 cap; max={}",
        max_visible_width(&frame),
    );
    assert!(
        frame.plain.contains("min(a, b)"),
        "expressions report must list the function catalog.\nplain:\n{}",
        frame.plain,
    );
}

/// `--expressions` wider than 140: content-hugging tables still never bleed past
/// the 140 cap in a 160-cell pane.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_caps_at_140_in_wide_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--expressions"], 160, 320);

    assert!(
        max_visible_width(&frame) <= 140,
        "in a 160-col terminal, expressions rows must stay within 140; max={}.\nplain:\n{}",
        max_visible_width(&frame),
        frame.plain,
    );
    assert!(
        frame.plain.contains("Functions"),
        "wide expressions report must render the function section.\nplain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Side-effects report
// ---------------------------------------------------------------------------

/// `--side-effects` narrow (`COLUMNS=128`, just above its ~123-cell floor):
/// capability tables render with glyphs, the three documented columns, wrapped
/// descriptions, and rows that fit the pane.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_narrow_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--side-effects"], 128, 200);

    assert_box_glyphs_and_left_margin(&frame);
    for header in ["Capability", "Description", "Safety"] {
        assert!(
            frame.plain.contains(header),
            "side-effects report must show the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
    assert!(
        max_visible_width(&frame) <= 128,
        "narrow side-effects rows must fit within 128 cells; max={}.\nplain:\n{}",
        max_visible_width(&frame),
        frame.plain,
    );
    assert!(
        has_wrapped_continuation(&frame),
        "narrow side-effects report must wrap descriptions onto continuation rows.\nplain:\n{}",
        frame.plain,
    );
}

/// `--side-effects` at exactly 140: fills the 140 cap with its 1ch right margin.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_at_140_fills_cap_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--side-effects"], 140, 200);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
    for header in ["Capability", "Description", "Safety"] {
        assert!(
            frame.plain.contains(header),
            "140-wide side-effects report must show the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
}

/// `--side-effects` wider than 140: caps at the 140 envelope rather than filling
/// 160.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_caps_at_140_in_wide_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--side-effects"], 160, 200);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
}

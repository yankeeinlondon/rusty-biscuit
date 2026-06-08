//! Level 2 real-terminal capture tests for the `claudine context` reports.
//!
//! These drive the real `claudine context` binary inside a real terminal
//! emulator (tmux) and assert against the bytes the terminal actually displayed
//! (`CapturedFrame`), closing the gap the review flagged: the prior `*_pty.rs`
//! tests only inspect a manufactured PTY's byte stream (Level 1).
//!
//! Coverage (per the context spec's shared-rendering requirements):
//!
//! - **margins + box glyphs** — every table border carries the 1ch left margin
//!   and the production `Table` border glyphs.
//! - **`Type` column preserved** — the narrow report keeps `Property` / `Type` /
//!   the final column and wraps content instead of dropping `Type`.
//! - **visible-width contract** — no rendered row exceeds the terminal width,
//!   and a wider-than-140 terminal caps rows at 140 visible cells.
//! - **inverse inline code** — the `` `||` `` mode header and the alias-row
//!   descriptions render with the inverse SGR (`\x1b[7m`), never literal
//!   backticks/markup.
//! - **unordered list** — operator lists render with the `- ` marker and a
//!   hanging indent on wrapped continuation lines.
//!
//! Each test spawns a **dedicated, tall** tmux session sized to hold the whole
//! report — the harness captures only the visible pane (no scrollback), and the
//! reports are far taller than a default pane, so the section under test would
//! otherwise scroll away. tmux is the portable, headless real-terminal backend
//! and faithfully re-emits SGR; the context reports use no OSC8 hyperlinks, so a
//! second emulator adds nothing here.
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
/// `rows` must exceed the report's line count so the full report is on the
/// (detached) pane when captured — `capture-pane` returns only the visible
/// pane. `FORCE_COLOR=1` routes claudine through an optimistic terminal so the
/// styling is the emulator's capture, not claudine's raw stream; `COLUMNS`
/// fixes the logical width.
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

/// Narrow report (`COLUMNS=78`): all three columns survive and the final column
/// wraps rather than the table dropping `Type` or overflowing.
#[test]
#[serial(level2_terminal)]
fn level2_context_narrow_preserves_type_and_wraps_in_tmux() {
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
    // Wrapping produces continuation rows: a data line whose first cell is blank
    // (the description spilled onto a new line) but which still has a second
    // column separator.
    let wrapped = frame.plain.lines().any(|l| {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix('│') {
            rest.starts_with("  ") && rest.contains('│')
        } else {
            false
        }
    });
    assert!(
        wrapped,
        "narrow report must wrap descriptive content onto continuation rows.\nplain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Expressions report
// ---------------------------------------------------------------------------

/// `--expressions`: the `` `||` `` mode header renders inverse, and operator
/// lists use the `- ` marker with a hanging indent on wrapped lines.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_inline_code_and_list_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--expressions"], 120, 320);

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
        !trimmed.is_empty() && !trimmed.starts_with("- ") && !trimmed.starts_with('│') && indent > bullet_indent
    });
    assert!(
        has_hanging_indent,
        "wrapped list items must hang-indent past the `- ` bullet.\nplain:\n{}",
        frame.plain,
    );

    assert!(
        max_visible_width(&frame) <= 120,
        "no row may exceed the 120-col pane; max={}",
        max_visible_width(&frame),
    );
}

// ---------------------------------------------------------------------------
// Side-effects report
// ---------------------------------------------------------------------------

/// `--side-effects`: capability tables render with glyphs, the 1ch margin, the
/// three documented columns, and rows that fit the pane.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&["--side-effects"], 120, 120);

    assert_box_glyphs_and_left_margin(&frame);
    for header in ["Capability", "Description", "Safety"] {
        assert!(
            frame.plain.contains(header),
            "side-effects report must show the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
    assert!(
        max_visible_width(&frame) <= 120,
        "no row may exceed the 120-col pane; max={}",
        max_visible_width(&frame),
    );
}

// ---------------------------------------------------------------------------
// 140-cap in a wider-than-140 terminal
// ---------------------------------------------------------------------------

/// In a terminal wider than 140 cells, every rendered row caps at 140 visible
/// cells (the report's maximum-width contract), proven in a real terminal
/// rather than by the in-process width math.
#[test]
#[serial(level2_terminal)]
fn level2_context_caps_rows_at_140_in_wide_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let frame = capture_context(&[], 160, 320);

    let max = max_visible_width(&frame);
    assert!(
        max <= 140,
        "in a 160-col terminal, rows must cap at 140 visible cells; max={max}.\nplain:\n{}",
        frame.plain,
    );
    // The report still rendered fully (not blank / not scrolled away).
    assert!(
        frame.plain.contains("ctx.today"),
        "wide report must still render the catalog.\nplain:\n{}",
        frame.plain,
    );
}

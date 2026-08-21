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
//! - **unordered list** — the modes consequence list renders with the `- `
//!   marker and a hanging indent on wrapped continuation lines, surrounded by
//!   exactly one blank line on each side, and reserves a 1ch right margin so
//!   list lines never reach the full pane width.
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
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{CliProcessFixture, clear_no_color};

/// Captures a `claudine context <args>` run inside a freshly spawned tmux
/// session of `cols` × `rows` cells, then tears the session down.
///
/// `rows` must exceed (or, for over-tall reports, usefully sample) the report's
/// line count — `capture-pane` returns only the visible pane. `FORCE_COLOR=1`
/// routes claudine through an optimistic terminal so the styling is the
/// emulator's capture, not claudine's raw stream; `COLUMNS` fixes the logical
/// width.
fn capture_context(args: &[&str], cols: u32, rows: u32) -> CapturedFrame {
    let fixture = CliProcessFixture::named("claudine-context-l2");
    fixture.seed_user_config();
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let session = format!(
        "biscuit_ctx_l2_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell (bash/sh), not the developer's `$SHELL`: a custom login
    // prompt (e.g. Starship's `❯`) never ends in `$`/`#`/`%`, so
    // `wait_for_prompt` would never match and burn its full timeout twice.
    let shell = biscuit_test_harness::detect_shell();
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
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);

    let claudine = cargo_bin("claudine").display().to_string();
    let home = fixture.home().display().to_string().replace('\'', "'\\''");
    let cols_s = cols.to_string();
    // The environment is bound through `env` rather than the harness' inline
    // `KEY='v' cmd` prefix so the line can open with the `REPORT_SENTINEL`
    // print — see `report_plain_slice` for why the sentinel has to precede
    // claudine's first byte of output.
    let cmd = format!(
        "printf '{REPORT_SENTINEL}\\n'; env HOME='{home}' CLAUDINE_RENDEZVOUS_REPORT=false FORCE_COLOR=1 COLUMNS={cols_s} {claudine} context {}",
        args.join(" ")
    );
    let send = harness.send_command_with_env(&cmd, &[]);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(Duration::from_millis(250));
    let frame = harness.capture();
    kill_session_by_name(&session);

    send.expect("send context command");
    frame.expect("capture failed")
}

/// Printed by the shell immediately before claudine runs, so everything after
/// it in the captured pane is claudine's own output.
const REPORT_SENTINEL: &str = "__CTX_L2_REPORT_BEGIN__";

/// The captured pane text with the leading shell noise removed.
///
/// The pane also holds the shell's echo of the typed command, which carries the
/// absolute binary path and readily exceeds the report's width envelope. That
/// echo is a *single logical line*, so the terminal wraps it into as many
/// physical rows as it needs — and every wrapped row but the last is exactly
/// pane-width. Filtering rows by a substring of the command cannot remove them:
/// the wrap lands wherever the prompt length puts it, frequently mid-token, so
/// the marker is split across two rows and matches neither. That is a false
/// "report is too wide" of exactly `cols` cells, and it is why the command is
/// preceded by `REPORT_SENTINEL`. Anchoring on the *last* occurrence picks the
/// printed sentinel over the echoed one.
///
/// Falls back to the whole frame when the sentinel is absent, which happens when
/// an over-tall report has scrolled it off the pane — in which case the echo has
/// scrolled off with it and there is no shell noise left to drop.
fn report_plain_slice(frame: &CapturedFrame) -> &str {
    let Some(idx) = frame.plain.rfind(REPORT_SENTINEL) else {
        return &frame.plain;
    };
    let rest = &frame.plain[idx..];
    rest.find('\n').map_or(rest, |nl| &rest[nl + 1..])
}

/// Maximum visible width across the rendered report rows (trailing pad
/// stripped, since tmux pads its capture to the pane width).
fn max_visible_width(frame: &CapturedFrame) -> usize {
    report_plain_slice(frame)
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

fn context_report_plain(frame: &CapturedFrame) -> &str {
    frame
        .plain
        .find("Darkmatter Expression Engine")
        .map(|idx| &frame.plain[idx..])
        .unwrap_or(&frame.plain)
}

/// Asserts the unordered list bracketed by the `heading` line and the
/// `next_heading` line is surrounded by exactly one blank line on each side.
///
/// The spec requires exactly one blank line before and after every unordered
/// list. Earlier iterations double-spaced these lists because the caller emitted
/// its own `log::data("")` on top of the blank line the list helper already
/// owns. Counting blanks from the bytes the emulator actually displayed guards
/// that contract: the list must have exactly one blank line immediately before
/// its first marker and exactly one blank line immediately after its last
/// content line.
///
/// The operator sections that once rendered as lists are now descriptor tables,
/// so the only unordered list left in the `--expressions` report is the modes
/// consequence list that closes the `Interpolation vs. Condition Mode` section
/// and precedes `Null Propagation Summary`. That list does not open directly
/// under its own heading (an intro paragraph and the modes table sit between),
/// so this searches forward from `heading` for the first `- ` marker rather
/// than assuming it sits on the line directly below.
fn assert_list_single_blank_lines(plain: &str, heading: &str, next_heading: &str) {
    let lines: Vec<&str> = plain.lines().map(|l| l.trim_end()).collect();
    let find = |label: &str| {
        lines
            .iter()
            .position(|l| l.trim() == label)
            .unwrap_or_else(|| panic!("heading `{label}` not found.\nplain:\n{plain}"))
    };
    let h = find(heading);
    let n = find(next_heading);
    let list_start = lines[h + 1..n]
        .iter()
        .position(|l| l.trim_start().starts_with("- "))
        .map(|offset| h + 1 + offset)
        .unwrap_or_else(|| {
            panic!("expected a list between `{heading}` and `{next_heading}`.\nplain:\n{plain}")
        });
    let list_end = lines[list_start + 1..n]
        .iter()
        .position(|l| l.is_empty())
        .map(|offset| list_start + 1 + offset)
        .unwrap_or(n);
    assert!(
        list_end > list_start,
        "expected a list between `{heading}` and `{next_heading}`.\nplain:\n{}",
        plain,
    );

    // Exactly one blank line before the list.
    assert!(
        list_start > 0 && lines[list_start - 1].is_empty(),
        "expected one blank line directly before the `{heading}` list.\nplain:\n{}",
        plain,
    );
    assert!(
        list_start == 1 || !lines[list_start - 2].is_empty(),
        "expected exactly one blank before the `{heading}` list.\nplain:\n{}",
        plain,
    );

    // Exactly one blank line after the list.
    assert!(
        lines[list_end].is_empty(),
        "expected one blank line directly after the `{heading}` list.\nplain:\n{}",
        plain,
    );
    assert!(
        list_end + 1 >= lines.len() || !lines[list_end + 1].is_empty(),
        "expected exactly one blank after the list preceding `{next_heading}`.\nplain:\n{}",
        plain,
    );
}

/// Asserts every line belonging to an unordered list fits within
/// `pane_width - 1` visible cells — the 1ch right margin the spec reserves for
/// list content (spec.md "Unordered Lists").
///
/// A frame-wide `max_visible_width <= pane_width` check cannot prove this: a
/// list line that consumed the reserved margin cell (reaching the full pane
/// width) would still pass it. This isolates the list lines and holds them to
/// the tighter `pane_width - 1` bound.
///
/// List lines are the contiguous block opened by a `- ` marker and closed by
/// the next blank line — markers and their hanging-indent continuations, but
/// not the surrounding prose paragraphs, which carry no right margin. The
/// caller must pick a width at which the list genuinely wraps so the bound is
/// exercised rather than trivially satisfied; the assertion requires at least
/// one wrapped continuation line as a guard.
fn assert_list_lines_reserve_right_margin(frame: &CapturedFrame, pane_width: u32) {
    let limit = pane_width as usize - 1;
    let mut in_list = false;
    let mut markers = 0usize;
    let mut continuations = 0usize;
    for raw in frame.plain.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ") {
            in_list = true;
            markers += 1;
        } else if trimmed.is_empty() {
            in_list = false;
        }
        if in_list {
            if !trimmed.starts_with("- ") {
                continuations += 1;
            }
            let width = visible_width(line) as usize;
            assert!(
                width <= limit,
                "list line must fit within pane_width - 1 ({limit}) to reserve the \
                 1ch right margin; got {width}.\nline: {line:?}\nplain:\n{}",
                frame.plain,
            );
        }
    }
    assert!(
        markers > 0,
        "expected at least one `- ` list marker.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        continuations > 0,
        "expected the list to wrap (a hanging-indent continuation line) so the \
         right-margin bound is exercised, not trivially met.\nplain:\n{}",
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
/// wrapping is active): the `` `||` `` mode header renders inverse, the modes
/// consequence list uses the `- ` marker with a hanging indent and is surrounded
/// by exactly one blank line, and rows fit the pane.
///
/// At 100 columns the `Example` column is kept (100 >= the 70-cell threshold),
/// so the function catalog wraps the report to ~533 lines. The `||` mode header
/// and the modes consequence list anchor assertions past the descriptor-table
/// operator sections, so the pane must hold the whole report — `capture-pane`
/// has no scrollback. 700 rows fit it with headroom.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_narrow_inline_code_and_list_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--expressions"], 100, 700);
    let report_plain = context_report_plain(&frame);

    assert!(
        !report_plain.contains("`||`"),
        "styled output must not show literal backticks around `||`.\nplain:\n{}",
        report_plain,
    );
    assert!(
        frame.raw.contains("\x1b[7m"),
        "the `||` mode header must render with inverse SGR.\nraw:\n{}",
        frame.raw,
    );

    // The modes consequence list uses the `- ` marker.
    let marker_line = frame
        .plain
        .lines()
        .skip_while(|l| !l.contains("Darkmatter Expression Engine"))
        .find(|l| l.trim_start().starts_with("- "))
        .unwrap_or_else(|| panic!("expected a `- ` list marker.\nplain:\n{}", report_plain));
    let bullet_indent = marker_line.len() - marker_line.trim_start().len();
    // A wrapped continuation line of a list item is indented past the bullet.
    let has_hanging_indent = report_plain.lines().any(|l| {
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
        report_plain,
    );

    // The modes consequence list (the only unordered list in this report — the
    // operator sections are now descriptor tables) closes the
    // `Interpolation vs. Condition Mode` section and precedes
    // `Null Propagation Summary`, and must carry exactly one blank line on each
    // side (spec: one blank before and after every unordered list).
    assert_list_single_blank_lines(
        report_plain,
        "Interpolation vs. Condition Mode",
        "Null Propagation Summary",
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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

/// `--expressions` at 65 columns: every unordered-list line stays within the
/// pane's `width - 1` envelope, proving the 1ch right margin the spec reserves
/// for list content. 65 is chosen deliberately: one operator item
/// (`+ performs string concatenation when either operand is a string`) is
/// exactly 65 cells wide, so only the reserved margin forces it to wrap. A
/// regression dropping the margin would let it fill the full 65-cell pane and
/// fail the bound — the discrimination a frame-wide `<= width` check lacks.
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_list_reserves_right_margin_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    // At 65 columns the `--expressions` report wraps to ~772 lines. The only
    // `- ` list is the modes consequence list, the 7th of 9 sections; the
    // Functions catalog below it must not push that list above the captured
    // pane (`capture-pane` has no scrollback). 820 rows fit the full report
    // with headroom for the prompt echo and footer.
    let frame = capture_context(&["--expressions"], 65, 820);

    assert_list_lines_reserve_right_margin(&frame, 65);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--side-effects"], 160, 200);

    assert_box_glyphs_and_left_margin(&frame);
    assert_fills_to_140_cap_with_right_margin(&frame);
}

/// `--side-effects` at 60 columns: every constraint-list line stays within the
/// pane's `width - 1` envelope, proving the 1ch right margin the spec reserves
/// for list content. 60 is chosen deliberately: one constraint item
/// (`Markdown mutations honor Darkmatter's auto-rehash behavior`) is exactly 60
/// cells wide, so only the reserved margin forces it to wrap. A regression
/// dropping the margin would let it fill the full 60-cell pane and fail the
/// bound. 60 also clears the report's 53-cell table floor.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_list_reserves_right_margin_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--side-effects"], 60, 200);

    assert_list_lines_reserve_right_margin(&frame, 60);
}

// ---------------------------------------------------------------------------
// Minimum supported width — required-column preservation (review-4 finding 1)
// ---------------------------------------------------------------------------
//
// The minimum supported terminal width is 53 cells (see the spec's "Narrow
// Terminals" clause). At that floor the binding default and values reports must
// preserve every required column by wrapping — never drop a column and never
// emit the planner width-error string. The expression report holds narrower
// unbreakable tokens and preserves its columns below the floor (exercised at
// 50); the side-effect report's `MarkdownMutation` safety token binds it to the
// shared 53 floor, so it is exercised there. Each case captures a real terminal
// and asserts box glyphs, the 1ch left margin, no planner diagnostic, that all
// required headers survive, representative content renders, and rows fit the
// pane.

/// The minimum supported terminal width, in cells, at which every report keeps
/// all required columns. Mirrors `MIN_SUPPORTED_REPORT_WIDTH` in the renderer.
const MIN_SUPPORTED_WIDTH: u32 = 53;

/// Asserts the visible frame carries no table planner width-error diagnostic.
fn assert_no_planner_diagnostic(frame: &CapturedFrame) {
    assert!(
        !frame.plain.contains("could not be rendered"),
        "constrained-width report must wrap, not emit the planner width-error.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts every required column header survives in the captured frame.
fn assert_headers_present(frame: &CapturedFrame, headers: &[&str]) {
    for header in headers {
        assert!(
            frame.plain.contains(header),
            "constrained report must keep the `{header}` column.\nplain:\n{}",
            frame.plain,
        );
    }
}

/// Default report at the minimum supported width (53 columns): wraps to fit with
/// box glyphs, keeps all three columns (`Property`/`Type`/`Description`), shows
/// no planner diagnostic, and still renders catalog content (`ctx.gpu`).
#[test]
#[serial(level2_terminal)]
fn level2_context_default_preserves_columns_at_min_width_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&[], MIN_SUPPORTED_WIDTH, 400);

    assert_box_glyphs_and_left_margin(&frame);
    assert_no_planner_diagnostic(&frame);
    assert_headers_present(&frame, &["Property", "Type", "Description"]);
    assert!(
        frame.plain.contains("ctx.gpu"),
        "constrained default report must still render catalog rows.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        has_wrapped_continuation(&frame),
        "constrained default report must wrap onto continuation rows.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(
        max <= MIN_SUPPORTED_WIDTH as usize,
        "rows must fit the {MIN_SUPPORTED_WIDTH}-col pane; max={max}.\nplain:\n{}",
        frame.plain,
    );
}

/// `--values` at the minimum supported width (53 columns): wraps, no planner
/// diagnostic, keeps all three columns (`Property`/`Type`/`Value`), and renders
/// the bottom `ctx.gpu` row. The spec binds the default *and* values reports to
/// the same 53-cell floor, so this report is exercised at that floor — not a
/// looser per-report width.
#[test]
#[serial(level2_terminal)]
fn level2_context_values_preserves_columns_at_min_width_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--values"], MIN_SUPPORTED_WIDTH, 400);

    assert_box_glyphs_and_left_margin(&frame);
    assert_no_planner_diagnostic(&frame);
    assert_headers_present(&frame, &["Property", "Type", "Value"]);
    assert!(
        frame.plain.contains("ctx.gpu"),
        "constrained values report must render the bottom `ctx.gpu` row.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(
        max <= MIN_SUPPORTED_WIDTH as usize,
        "rows must fit the {MIN_SUPPORTED_WIDTH}-col pane; max={max}.\nplain:\n{}",
        frame.plain,
    );
}

/// `--expressions` at 50 columns (below the shared floor — its narrower
/// unbreakable tokens preserve all columns there): wraps, no planner diagnostic,
/// keeps both columns (`Function`/`Description`), and still renders the function
/// catalog (the bottom `validate_schema` Filesystem group).
#[test]
#[serial(level2_terminal)]
fn level2_context_expressions_constrained_50_wraps_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--expressions"], 50, 400);

    assert_box_glyphs_and_left_margin(&frame);
    assert_no_planner_diagnostic(&frame);
    assert_headers_present(&frame, &["Function", "Description"]);
    assert!(
        frame.plain.contains("validate_schema"),
        "constrained expressions report must render the function catalog.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(max <= 50, "rows must fit the 50-col pane; max={max}.\nplain:\n{}", frame.plain);
}

/// `--side-effects` at the minimum supported width (53 columns): wraps, no
/// planner diagnostic, keeps all three columns
/// (`Capability`/`Description`/`Safety`), and still renders the capability
/// catalog (the bottom `http_post` Network group). The `Frontmatter Mutations`
/// section's `MarkdownMutation` safety token is the binding constraint here, so
/// this report is exercised at the shared floor rather than below it.
#[test]
#[serial(level2_terminal)]
fn level2_context_side_effects_preserves_columns_at_min_width_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_context(&["--side-effects"], MIN_SUPPORTED_WIDTH, 200);

    assert_box_glyphs_and_left_margin(&frame);
    assert_no_planner_diagnostic(&frame);
    assert_headers_present(&frame, &["Capability", "Description", "Safety"]);
    assert!(
        frame.plain.contains("http_post"),
        "constrained side-effects report must render the capability catalog.\nplain:\n{}",
        frame.plain,
    );
    let max = max_visible_width(&frame);
    assert!(
        max <= MIN_SUPPORTED_WIDTH as usize,
        "rows must fit the {MIN_SUPPORTED_WIDTH}-col pane; max={max}.\nplain:\n{}",
        frame.plain,
    );
}

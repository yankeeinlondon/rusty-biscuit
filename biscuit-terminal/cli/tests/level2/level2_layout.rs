//! Level-2 (real-terminal) tests for `bt columns` (the `TwoColumn` layout).
//!
//! These tests run the `bt` CLI inside a real WezTerm pane via the shared
//! `biscuit-test-harness`, so the rendered two-column layout is validated
//! against the actual terminal's display path — visible column widths, the
//! gutter between columns, narrow-column word wrapping, and left margins.
//!
//! `bt columns` renders through the bespoke `TwoColumn` renderer. The
//! `render_comparison` drift suite (`biscuit-terminal/lib/tests`) proves the
//! render-tree `TwoColumn` path is byte-equivalent to the bespoke path for
//! every layout-matrix cell, so real-terminal coverage of the bespoke output
//! here transitively covers the tree renderer's `TwoColumn` output too.
//!
//! ## Skip-clean contract
//!
//! Every test checks `WezTermHarness::available()` before spawning. When
//! WezTerm is absent the test prints `skipping: requires <X>` to stderr and
//! returns immediately. No `#[ignore]` markers are used.

use crate::common;

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, LevelDecision, decide_harness};

/// The WezTerm pane a test attaches to: `just test-l2`'s shared pane in
/// serial mode, or one the test spawns. `common::run_bt_rows` clears the
/// screen and reads only the rows between its own markers, so an earlier
/// test's output cannot reach an assertion.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Runs a `bt` command in the WezTerm pane and returns the rendered output
/// lines (ANSI stripped), with the shell command echo excluded.
///
/// Returns `None` when WezTerm is unavailable.
fn run_bt(args: &str) -> Option<Vec<String>> {
    // Evaluate at the helper boundary so callers stay `let Some(lines) = …`;
    // mirrors the `require_level!` macro but returns `None` on skip so the
    // `Option`-returning helper signature can stand.
    match decide_harness!(Level::L2, WezTermHarness::available(), Backend::WezTerm) {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    let (_, rows) = common::run_bt_rows(harness, args, &[]);
    Some(rows.into_iter().map(|row| row.plain).collect())
}

/// Counts leading spaces on `line`.
fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

#[test]
#[serial(level2_terminal)]
fn level2_columns_long_content_wraps_within_columns() {
    // Each column gets ~half the pane; this content is well over half a pane
    // wide, so a correct renderer must flow it onto multiple rows rather than
    // overrunning the gutter.
    let left = "LEFTSTART alpha bravo charlie delta echo foxtrot golf hotel LEFTEND";
    let right = "RIGHTSTART one two three four five six seven eight nine ten RIGHTEND";
    let Some(lines) = run_bt(&format!("columns \"{left}\" \"{right}\"")) else {
        return;
    };
    let pane = lines.join("\n");

    // All four sentinels survive the round-trip through the real terminal.
    for needle in ["LEFTSTART", "LEFTEND", "RIGHTSTART", "RIGHTEND"] {
        assert!(
            lines.iter().any(|l| l.contains(needle)),
            "expected sentinel {needle:?} in rendered pane:\n{pane}"
        );
    }

    let start_row = lines
        .iter()
        .position(|l| l.contains("LEFTSTART"))
        .expect("LEFTSTART row");
    let end_row = lines
        .iter()
        .position(|l| l.contains("LEFTEND"))
        .expect("LEFTEND row");
    // The left column wrapped: its first and last words land on different
    // rows.
    assert!(
        end_row > start_row,
        "left column should wrap onto a later row: start={start_row}, end={end_row}\n{pane}"
    );
    // The right column's first word shares the first content row with the
    // left column's first word — the columns render side by side.
    assert!(
        lines[start_row].contains("RIGHTSTART"),
        "first content row should carry both columns:\n{}",
        lines[start_row]
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_columns_left_margin_shifts_block() {
    // A left margin must indent every rendered row by that many columns.
    let Some(lines) = run_bt("columns --ml 8 \"MARGINLEFT body\" \"MARGINRIGHT body\"") else {
        return;
    };

    let row = lines
        .iter()
        .find(|l| l.contains("MARGINLEFT"))
        .unwrap_or_else(|| panic!("MARGINLEFT row missing in pane:\n{}", lines.join("\n")));
    assert!(
        leading_spaces(row) >= 8,
        "left margin 8 should indent the row by >=8 spaces, got {}: {row:?}",
        leading_spaces(row)
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_columns_gap_separates_columns() {
    // With an explicit wide gap the two single-word columns are separated by
    // at least that many spaces on the rendered row.
    let Some(lines) = run_bt("columns --gap 7 \"GAPLEFT\" \"GAPRIGHT\"") else {
        return;
    };

    let row = lines
        .iter()
        .find(|l| l.contains("GAPLEFT") && l.contains("GAPRIGHT"))
        .unwrap_or_else(|| panic!("expected both columns on one row:\n{}", lines.join("\n")));
    let between = row
        .split_once("GAPLEFT")
        .and_then(|(_, rest)| rest.split_once("GAPRIGHT"))
        .map(|(mid, _)| mid.chars().filter(|c| *c == ' ').count())
        .expect("gutter slice");
    assert!(
        between >= 7,
        "gap 7 should leave >=7 spaces between the columns, got {between}: {row:?}"
    );
}

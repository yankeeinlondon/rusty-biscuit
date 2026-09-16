//! Level 2 tests for the `--perf` metrics trees rendered in a real terminal.
//!
//! The `--perf` report is two `MetricsTree` renderings: a timing hierarchy with
//! a unit-aligned value column, a right-aligned share column, a single styled
//! HOT marker and an italic overlap note, followed by a separate counter tree.
//! Level 1 pins the projected strings and the CLI's stream routing, but it
//! reaches none of the contract that only an emulator can decide: the pane width
//! a real terminal reports (a piped run never truncates at all), the display
//! columns the glyphs actually occupy, the SGR sequences the markup lowers to,
//! and whether any row wrapped.
//!
//! These tests run the real `sniff` binary inside a tmux pane — the portable,
//! headless backend — at a width this test owns, and assert on the captured
//! grid.
//!
//! ## Why owned panes rather than the shared broker pane
//!
//! Pane geometry *is* the contract here: truncation, column alignment, and
//! wrapping are all functions of the width. A test that asserts them has to set
//! that width, and the broker's shared pane is not this test's to resize. Each
//! test therefore spawns its own tmux session, which `Drop` kills; tmux sessions
//! are headless and carry no global OS state, so nothing is shared and nothing
//! leaks.
//!
//! ## Why these helpers rather than `cli.rs`'s
//!
//! The Level 1 perf helpers (`performance_section` / `metric_row` /
//! `metric_value` / `metric_offset`) live in another test binary and are not
//! reachable from here, but the deeper reason for re-deriving them is that they
//! locate rows with `str::find`, which yields a *byte* offset. A pane carrying
//! `├─`, `│`, `…`, `µ`, and `—` has byte offsets that are not display columns,
//! and display columns are exactly what a real-terminal alignment assertion has
//! to compare. The three portability rules they encode are kept: a label is
//! matched as a whole whitespace cell, a value is read as the cell *after* the
//! label (never the last cell — that is the share, which folds from an em dash
//! to a hyphen without Unicode), and hierarchy is expressed as an offset
//! ordering rather than by naming a connector glyph.
//!
//! Skip-clean: when tmux is unavailable the test prints a skip notice and
//! passes, so a runner without the tooling stays green.
//!
//! Gated behind `test-fixtures`, which is what pulls in `biscuit-test-harness`
//! and the shared `common::capture_until` poller. The `test-l2` recipe passes
//! `--features test-fixtures`.
#![cfg(feature = "test-fixtures")]

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::CapturedFrame;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

mod common;

const RENDER_DEADLINE: Duration = Duration::from_secs(15);

/// Tall enough that the whole report plus its trailing marker stays on screen
/// at the narrowest swept width, where the host report wraps hardest.
const PANE_ROWS: u32 = 70;

/// Comfortably wider than the longest production label plus the value, share,
/// and HOT-marker columns, and wider than the 98-column overlap note, so this
/// pane exercises the untruncated layout.
const WIDE_PANE_COLS: u32 = 100;

/// The swept narrow widths. The component derives its label column from the
/// pane width, so consecutive widths slide the truncation point one character
/// at a time across every label — which is what walks it onto an underscore.
///
/// The span is the budget: each width costs one `sniff` run plus the harness's
/// settle and poll (~0.45 s; 9.5 s for the sweep on a 16-core Mac, 2026-09-15),
/// against a 30 s local and 90 s CI termination ceiling. A shorter sweep would
/// cover fewer cut positions and could miss the boundary on a host whose stage
/// names differ from this one's.
const NARROW_PANE_COLS: std::ops::RangeInclusive<u32> = 30..=50;

/// The tail of the overlap note, used to locate its rendered line.
const OVERLAP_NOTE_HEAD: &str = "Concurrent, nested, and repeated";

// ============================================================================
// Pane-grid parsing
// ============================================================================

/// One rendered metrics-tree row, located by pane display column.
struct PaneRow {
    label: String,
    label_col: usize,
    value: String,
    value_col: usize,
    share: String,
    share_col: usize,
}

impl PaneRow {
    /// The column the value mantissa ends at. The mantissa is right-aligned and
    /// its unit suffix left-aligned against it, so this is the unit boundary the
    /// whole tree aligns on — the one column that must be identical on every
    /// row.
    fn mantissa_end_col(&self) -> usize {
        self.value_col
            + self
                .value
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .count()
    }

    /// The column the share cell ends at. The share is right-aligned, so its
    /// *right* edge is the invariant — its left edge moves with the cell width
    /// (`100%` versus `—`).
    fn share_end_col(&self) -> usize {
        self.share_col + self.share.chars().count()
    }
}

/// Whitespace-separated cells paired with the display column each starts at.
///
/// Columns are counted in characters because every glyph the component emits —
/// box connectors, `…`, `µ`, `—` — occupies one terminal cell.
fn cells(line: &str) -> Vec<(String, usize)> {
    let mut collected = Vec::new();
    let mut current = String::new();
    let mut start = 0usize;
    for (column, character) in line.chars().enumerate() {
        if character.is_whitespace() {
            if !current.is_empty() {
                collected.push((std::mem::take(&mut current), start));
            }
        } else {
            if current.is_empty() {
                start = column;
            }
            current.push(character);
        }
    }
    if !current.is_empty() {
        collected.push((current, start));
    }
    collected
}

/// A value cell: a mantissa of digits and dots, then an optional unit suffix.
/// A share (`100%`, `<1%`, `—`) never satisfies this, so the two are never
/// confused when scanning a row.
fn is_value_cell(cell: &str) -> bool {
    let mut characters = cell.chars().peekable();
    let mut digits = 0usize;
    while characters
        .peek()
        .is_some_and(|c| c.is_ascii_digit() || *c == '.')
    {
        if characters.next().is_some_and(|c| c.is_ascii_digit()) {
            digits += 1;
        }
    }
    digits > 0 && characters.all(char::is_alphabetic)
}

/// A share cell in either glyph set: a percentage, the sub-percent sliver, or
/// the unknown-share dash (em dash with Unicode, hyphen without).
fn is_share_cell(cell: &str) -> bool {
    if cell == "—" || cell == "-" || cell == "<1%" {
        return true;
    }
    cell.len() > 1
        && cell.ends_with('%')
        && cell
            .trim_end_matches('%')
            .chars()
            .all(|c| c.is_ascii_digit())
}

/// Parse a rendered row as the `label value share` triple the component
/// promises. A row that wrapped loses that triple on both of its fragments,
/// which is what makes `None` the wrap detector.
fn parse_row(line: &str) -> Option<PaneRow> {
    let cells = cells(line);
    (1..cells.len().saturating_sub(1)).find_map(|index| {
        (is_value_cell(&cells[index].0) && is_share_cell(&cells[index + 1].0)).then(|| PaneRow {
            label: cells[index - 1].0.clone(),
            label_col: cells[index - 1].1,
            value: cells[index].0.clone(),
            value_col: cells[index].1,
            share: cells[index + 1].0.clone(),
            share_col: cells[index + 1].1,
        })
    })
}

/// The line index of a tree root, found by its label owning the first cell of
/// the line.
fn root_line(lines: &[&str], from: usize, root: &str) -> Option<usize> {
    (from..lines.len()).find(|index| lines[*index].split_whitespace().next() == Some(root))
}

/// The contiguous non-blank lines a tree occupies, as `(first index, lines)`.
/// The component separates the tree, its notes, and the next tree with blank
/// rows, so a blank line is the block terminator.
fn tree_block<'a>(lines: &[&'a str], from: usize, root: &str) -> (usize, Vec<&'a str>) {
    let start = root_line(lines, from, root)
        .unwrap_or_else(|| panic!("captured pane must carry a `{root}` tree root"));
    let block = lines[start..]
        .iter()
        .take_while(|line| !line.trim().is_empty())
        .copied()
        .collect();
    (start, block)
}

/// Every line of a block, parsed. A line that does not parse is a wrapped or
/// corrupted row, which is the failure this whole file exists to catch.
fn parsed_rows(block: &[&str], tree: &str, pane: &str) -> Vec<PaneRow> {
    block
        .iter()
        .map(|line| {
            parse_row(line).unwrap_or_else(|| {
                panic!(
                    "the `{tree}` tree wrapped or corrupted a row: `{line}` carries no \
                     label/value/share triple.\npane:\n{pane}"
                )
            })
        })
        .collect()
}

/// The value and share columns must land on the same display column for every
/// row of a tree — the alignment contract, measured as the emulator laid it out
/// rather than as a string length.
fn assert_columns_align(rows: &[PaneRow], tree: &str, pane: &str) {
    let first = &rows[0];
    for row in rows {
        assert_eq!(
            row.mantissa_end_col(),
            first.mantissa_end_col(),
            "`{tree}` row `{}` breaks the unit-aligned value column \
             (`{}` ends at {}, `{}` at {}).\npane:\n{pane}",
            row.label,
            row.value,
            row.mantissa_end_col(),
            first.value,
            first.mantissa_end_col(),
        );
        assert_eq!(
            row.share_end_col(),
            first.share_end_col(),
            "`{tree}` row `{}` breaks the right-aligned share column.\npane:\n{pane}",
            row.label,
        );
    }
}

/// Italic is the signature of the truncation defect this fix closed: a label cut
/// immediately after an underscore opened a Prose emphasis span that a later
/// underscore closed, eating visible columns. Tree rows carry bold, red, dim,
/// and grey — never italic, which belongs to the trailing note alone.
fn assert_no_italic_rows(raw_lines: &[&str], block_start: usize, block_len: usize, pane: &str) {
    for (offset, raw) in raw_lines
        .iter()
        .skip(block_start)
        .take(block_len)
        .enumerate()
    {
        assert!(
            !raw.contains("[3m") && !raw.contains(";3m"),
            "row {offset} acquired an italic SGR — a label truncation opened an \
             unescaped markup span.\nraw row: {raw:?}\npane:\n{pane}"
        );
    }
}

/// A truncated label whose cut landed immediately after an underscore, in
/// either glyph set. This is the exact input that used to corrupt the row.
fn cuts_after_underscore(label: &str) -> bool {
    label.ends_with("_…") || label.ends_with("_...")
}

// ============================================================================
// Driving the pane
// ============================================================================

/// Spawn an owned tmux session sized for this test and return it.
fn owned_pane(cols: u32) -> TmuxHarness {
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux spawn_shell failed");
    harness.resize(cols, PANE_ROWS).expect("tmux resize failed");
    harness
}

/// Run `sniff <args> --perf` in the pane and return the completed frame.
///
/// The command ends with a `printf` whose *output* (`PERF-DONE-<nonce>`) cannot
/// appear in the echoed command line, so the predicate cannot be satisfied by a
/// stale frame left over from the previous width. Its leading blank line keeps
/// the marker out of the counter tree's block, which ends at the first blank
/// row. The environment prefix is written into the command string rather than
/// passed to `send_command_with_env`, which would bind it to `clear` instead of
/// `sniff`.
fn render_perf(harness: &mut TmuxHarness, nonce: &str) -> CapturedFrame {
    let binary = cargo_bin("sniff").display().to_string();
    let marker = format!("PERF-DONE-{nonce}");
    harness
        .send_command_with_env(
            &format!(
                "clear; FORCE_COLOR=1 {binary} os --perf; printf '\\nPERF-DONE-%s\\n' {nonce}"
            ),
            &[],
        )
        .expect("send_command_with_env failed");

    common::capture_until(harness, RENDER_DEADLINE, |frame| {
        let lines: Vec<&str> = frame.plain.lines().collect();
        let Some(heading) = lines
            .iter()
            .position(|line| line.contains("## Performance"))
        else {
            return false;
        };
        root_line(&lines, heading, "Total").is_some()
            && root_line(&lines, heading, "Counters").is_some()
            && lines.iter().any(|line| line.trim() == marker)
    })
}

// ============================================================================
// Tests
// ============================================================================

/// The whole `--perf` visual contract at a width where nothing truncates:
/// hierarchy, a separate counter tree, the styled HOT row, aligned value and
/// share columns, an italic note, and no wrapped row.
#[test]
fn level2_perf_trees_render_aligned_and_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = owned_pane(WIDE_PANE_COLS);
    let frame = render_perf(&mut harness, "wide");
    let pane = frame.plain.clone();
    let lines: Vec<&str> = frame.plain.lines().collect();
    let raw_lines: Vec<&str> = frame.raw.lines().collect();

    let heading = lines
        .iter()
        .position(|line| line.contains("## Performance"))
        .unwrap_or_else(|| panic!("captured pane must carry the performance heading:\n{pane}"));

    // --- The timing tree ---------------------------------------------------
    let (timing_start, timing_block) = tree_block(&lines, heading, "Total");
    let timing = parsed_rows(&timing_block, "Total", &pane);
    assert!(
        timing.len() >= 3,
        "the timing tree must render its root and nested stages:\n{pane}"
    );
    assert_eq!(
        timing[0].label_col, 0,
        "the synthetic root owns column zero:\n{pane}"
    );
    assert_eq!(
        timing[0].share, "100%",
        "the synthetic root owns the full share:\n{pane}"
    );

    // Hierarchy, expressed as an offset ordering rather than by naming a
    // connector glyph: `detect.os` must render as an `os` row indented past its
    // `detect` parent, which itself sits past the root.
    let column_of = |label: &str| {
        timing
            .iter()
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("timing tree must carry a `{label}` row:\n{pane}"))
            .label_col
    };
    assert!(
        column_of("Total") < column_of("detect") && column_of("detect") < column_of("os"),
        "the timing tree must nest `os` below `detect` below the root:\n{pane}"
    );
    let depths: std::collections::BTreeSet<usize> =
        timing.iter().map(|row| row.label_col).collect();
    assert!(
        depths.len() >= 3,
        "the timing tree must render at least three depths:\n{pane}"
    );

    // Alignment as the emulator measured it. A piped Level 1 run compares
    // string lengths; this compares the columns the pane actually painted,
    // with `µ`, `…`, and the box connectors counted as the cells they occupy.
    assert_columns_align(&timing, "Total", &pane);
    assert_no_italic_rows(&raw_lines, timing_start, timing_block.len(), &pane);

    // --- The HOT row -------------------------------------------------------
    let hot_rows: Vec<usize> = timing_block
        .iter()
        .enumerate()
        .filter(|(_, line)| line.split_whitespace().any(|cell| cell == "HOT"))
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(
        hot_rows.len(),
        1,
        "exactly one row carries the HOT marker:\n{pane}"
    );
    let hot = hot_rows[0];
    assert!(
        timing[hot].label_col > 0,
        "the HOT marker belongs to a measured stage, not the synthetic root:\n{pane}"
    );
    // Interpreted SGR, which no Level 1 test reaches: the marker is bold and the
    // hot row's value is red. Matched precisely so `[31m` cannot satisfy the
    // bold check.
    let hot_raw = raw_lines[timing_start + hot];
    assert!(
        hot_raw.contains("[1m") || hot_raw.contains(";1m"),
        "the HOT marker must render bold in the pane.\nraw row: {hot_raw:?}\npane:\n{pane}"
    );
    assert!(
        hot_raw.contains("[31m") || hot_raw.contains(";31m"),
        "the HOT row's value must render red in the pane.\nraw row: {hot_raw:?}\npane:\n{pane}"
    );

    // The value units recede: dim for coarse units, grey for microseconds.
    let timing_raw = raw_lines[timing_start..timing_start + timing_block.len()].join("\n");
    assert!(
        timing_raw.contains("[2m")
            || timing_raw.contains(";2m")
            || timing_raw.contains("38;2;")
            || timing_raw.contains("38;5;"),
        "the value units must render dim or grey in the pane.\nraw:\n{timing_raw}"
    );

    // --- The overlap note --------------------------------------------------
    let note = lines
        .iter()
        .position(|line| line.contains(OVERLAP_NOTE_HEAD))
        .unwrap_or_else(|| panic!("the overlap note travels with the section:\n{pane}"));
    assert!(
        note > timing_start + timing_block.len(),
        "the note follows the timing tree:\n{pane}"
    );
    assert!(
        raw_lines[note].contains("[3m") || raw_lines[note].contains(";3m"),
        "the overlap note must render italic in the pane.\nraw row: {:?}\npane:\n{pane}",
        raw_lines[note],
    );

    // --- The separate counter tree -----------------------------------------
    let (counter_start, counter_block) = tree_block(&lines, note, "Counters");
    assert!(
        counter_start > note,
        "the counter tree is a separate tree rendered after the timing tree and \
         its note:\n{pane}"
    );
    let counters = parsed_rows(&counter_block, "Counters", &pane);
    assert!(
        counters.len() >= 2,
        "the counter tree must render its root and at least one counter:\n{pane}"
    );
    assert_eq!(
        counters[0].label_col, 0,
        "the counter root owns column zero:\n{pane}"
    );
    for row in &counters {
        assert!(
            row.value.chars().all(|c| c.is_ascii_digit()),
            "a counter row carries a unitless count, not a duration: `{}` = `{}`\n{pane}",
            row.label,
            row.value,
        );
    }
    assert_columns_align(&counters, "Counters", &pane);
    assert_no_italic_rows(&raw_lines, counter_start, counter_block.len(), &pane);
}

/// Narrow panes truncate labels, and a cut that lands immediately after an
/// underscore used to open an unescaped Prose emphasis span that consumed
/// visible columns and sheared the value column off the grid.
///
/// Sweeping consecutive widths slides the cut one character at a time across
/// every label, so the sweep walks the truncation point onto the underscores of
/// the host's own production stage and counter names. Every swept width must
/// render intact rows, and at least one must actually reach the underscore
/// boundary — otherwise the sweep proved nothing about it.
#[test]
fn level2_perf_trees_survive_narrow_pane_truncation_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = owned_pane(*NARROW_PANE_COLS.start());
    let mut underscore_boundaries: Vec<(u32, String)> = Vec::new();

    for cols in NARROW_PANE_COLS {
        harness.resize(cols, PANE_ROWS).expect("tmux resize failed");
        let frame = render_perf(&mut harness, &cols.to_string());
        let pane = frame.plain.clone();
        let lines: Vec<&str> = frame.plain.lines().collect();
        let raw_lines: Vec<&str> = frame.raw.lines().collect();

        let heading = lines
            .iter()
            .position(|line| line.contains("## Performance"))
            .unwrap_or_else(|| panic!("pane at {cols} columns lost the heading:\n{pane}"));

        for root in ["Total", "Counters"] {
            let (start, block) = tree_block(&lines, heading, root);
            let rows = parsed_rows(&block, root, &pane);
            assert_columns_align(&rows, root, &pane);
            assert_no_italic_rows(&raw_lines, start, block.len(), &pane);

            for row in &rows {
                // Prose consumes the escaping backslashes; one reaching the pane
                // would mean the escape was applied to the wrong text.
                assert!(
                    !row.label.contains('\\'),
                    "row `{}` at {cols} columns leaked an escape character:\n{pane}",
                    row.label,
                );
                if cuts_after_underscore(&row.label) {
                    underscore_boundaries.push((cols, row.label.clone()));
                }
            }
        }
    }

    assert!(
        !underscore_boundaries.is_empty(),
        "no swept width ({:?}) truncated a label immediately after an underscore, \
         so the sweep never reached the boundary it exists to exercise",
        NARROW_PANE_COLS,
    );
    eprintln!("underscore truncation boundaries exercised: {underscore_boundaries:?}");
}

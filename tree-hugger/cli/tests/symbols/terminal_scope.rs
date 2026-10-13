//! The marker-scoped report reader shared by the `level2_*` terminal tests
//! (`tests/support/terminal_report.rs`), on synthetic pane captures: the
//! report under test, never an earlier one still in the pane's history,
//! decides every check.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::terminal_report::{Report, ReportMarkers, ScopeError, style_at};

const HEADING: &str = "tools/long.py";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// The pane text a shell shows for one wrapped command and its output: the
/// colored prompt with the typed line, the markers, then the next prompt.
fn shown(markers: &ReportMarkers, rows: &[String]) -> String {
    let typed = markers.wrap("hug symbols --group-by-file");
    let mut pane = format!("\x1b[32mbash-5.3$\x1b[0m {typed}{}\n", markers.begin());
    for row in rows {
        pane.push_str(row);
        pane.push('\n');
    }
    pane.push_str(&markers.end());
    pane.push('\n');
    pane
}

fn heading(style: &str) -> String {
    format!("{style}{HEADING}{RESET}")
}

/// Two earlier reports whose headings carry `earlier` styles, then the
/// current report with a `current` heading, then an idle prompt.
fn history(earlier: [&str; 2], current: &str) -> (String, ReportMarkers) {
    let mut pane = String::new();
    for (index, style) in earlier.into_iter().enumerate() {
        let markers = ReportMarkers::with_id(&format!("7-{index}"));
        pane.push_str(&shown(&markers, &[heading(style), "- parameter old [1:1]".into()]));
    }
    let markers = ReportMarkers::with_id("7-2");
    pane.push_str(&shown(
        &markers,
        &[heading(current), "- parameter regional_breakdown [5:44]".into()],
    ));
    pane.push_str("\x1b[32mbash-5.3$\x1b[0m \n");
    (pane, markers)
}

fn scope(pane: &str, markers: &ReportMarkers) -> Report {
    match Report::scope(pane, markers) {
        Ok(report) => report,
        Err(error) => panic!("expected a scoped report, got {error:?}"),
    }
}

fn panics(check: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(check)).is_err()
}

#[test]
fn a_broken_current_heading_fails_although_an_earlier_report_is_correct() {
    let (pane, markers) = history([BOLD, DIM], DIM);

    let report = scope(&pane, &markers);

    // Control: the first escaped occurrence in the whole pane is the earlier
    // correct heading, which is the evidence the unscoped check accepted.
    assert_eq!(style_at(&pane, HEADING), Some(vec!["1".to_string()]));
    assert_eq!(
        report.style_in_row(HEADING, HEADING),
        Some(vec!["2".to_string()])
    );
    assert!(panics(|| report.assert_styled(HEADING, HEADING, &["1"], "a bold heading")));
}

#[test]
fn a_correct_current_heading_passes_although_an_earlier_report_differs() {
    let (pane, markers) = history([DIM, BOLD], BOLD);

    let report = scope(&pane, &markers);

    // Control: the whole pane's first occurrence is the earlier dim heading.
    assert_eq!(style_at(&pane, HEADING), Some(vec!["2".to_string()]));
    report.assert_styled(HEADING, HEADING, &["1"], "a bold heading");
}

#[test]
fn rows_present_only_in_an_earlier_report_are_not_found() {
    let (pane, markers) = history([BOLD, DIM], BOLD);

    let report = scope(&pane, &markers);

    assert!(!report.has_row("- parameter old [1:1]"));
    assert!(panics(|| report.assert_rows_in_order(&["- parameter old"])));
    assert!(panics(|| {
        report.row("- parameter old");
    }));
    report.assert_rows_in_order(&[HEADING, "- parameter regional_breakdown"]);
    assert_eq!(
        report.plain(),
        "tools/long.py\n- parameter regional_breakdown [5:44]\n"
    );
}

#[test]
fn a_row_needle_matching_several_rows_is_rejected() {
    let markers = ReportMarkers::with_id("7-0");
    let pane = shown(&markers, &[heading(BOLD), heading(DIM)]);

    let report = scope(&pane, &markers);

    assert!(panics(|| {
        report.row(HEADING);
    }));
}

#[test]
fn the_typed_command_line_is_not_a_marker() {
    let markers = ReportMarkers::with_id("7-0");
    let typed_only = format!("bash-5.3$ {}", markers.wrap("hug functions"));

    assert!(typed_only.contains("HUG-BEGIN 7-0"));
    assert!(matches!(
        Report::scope(&typed_only, &markers),
        Err(ScopeError::Unfinished)
    ));
}

#[test]
fn markers_that_cannot_delimit_one_report_are_errors() {
    let markers = ReportMarkers::with_id("7-0");
    let (begin, end) = (markers.begin(), markers.end());
    let malformed = [
        ("begin scrolled out of history", format!("{HEADING}\n{end}\n")),
        ("begin repeated", format!("{begin}\n{begin}\n{HEADING}\n{end}\n")),
        ("end repeated", format!("{begin}\n{HEADING}\n{end}\n{end}\n")),
        ("end before begin", format!("{end}\n{HEADING}\n{begin}\n")),
    ];

    for (case, pane) in malformed {
        assert!(
            matches!(
                Report::scope(&pane, &markers),
                Err(ScopeError::Malformed(_))
            ),
            "{case}: expected a malformed-marker error"
        );
    }
    assert!(matches!(
        Report::scope(&format!("{begin}\n{HEADING}\n"), &markers),
        Err(ScopeError::Unfinished)
    ));
}

#[test]
fn soft_wrapped_rows_are_rejoined_inside_the_report() {
    let markers = ReportMarkers::with_id("7-0");
    // A 20-column pane: a full row joins its continuation directly, a row one
    // column short gets the trimmed space back.
    let pane = format!(
        "{}\n- abcdefghijklmnopq\nrs tuvwxyz0123456789\nAB\n{}\n",
        markers.begin(),
        markers.end()
    );

    let report = scope(&pane, &markers);

    assert!(report.has_logical_row("- abcdefghijklmnopq rs tuvwxyz0123456789AB", 20));
    assert!(!report.has_row("- abcdefghijklmnopq rs tuvwxyz0123456789AB"));
}

/// WezTerm pads the last piece of a soft-wrapped line to the pane width; that
/// piece still ends the line, so the next row is not joined to it.
#[test]
fn a_padded_last_piece_ends_its_logical_row() {
    let markers = ReportMarkers::with_id("7-0");
    let pane = format!(
        "{}\nabcdefghijklmnopqrst\nuv{}\nnext row\n{}\n",
        markers.begin(),
        " ".repeat(18),
        markers.end()
    );

    let report = scope(&pane, &markers);

    assert!(report.has_logical_row("abcdefghijklmnopqrstuv", 20));
    assert!(report.has_logical_row("next row", 20));
}

#[test]
fn a_style_carried_over_from_the_previous_row_is_the_cells_style() {
    // tmux re-emits only what changed: the file heading under a bold blue
    // package heading resets the color and keeps the bold.
    let markers = ReportMarkers::with_id("7-0");
    let pane = shown(
        &markers,
        &[
            "\x1b[1m\x1b[34mtools".to_string(),
            format!("\x1b[39m{HEADING}"),
            format!("\x1b[0m- function f() {DIM}{HEADING}:5:1"),
        ],
    );

    let report = scope(&pane, &markers);

    assert_eq!(report.style_in_row("tools", "tools"), Some(vec!["1".into(), "34".into()]));
    assert_eq!(report.style_in_row(HEADING, HEADING), Some(vec!["1".to_string()]));
    assert_eq!(
        report.style_in_row("- function f()", HEADING),
        Some(vec!["2".to_string()])
    );
}

#[test]
fn styles_spelled_differently_by_backends_read_the_same() {
    let spellings = [
        "\x1b[0;1;34mtools\x1b[0m",
        "\x1b[1m\x1b(B\x1b[34mtools",
        "\x1b[1;38:5:4m\x1b[34mtools",
        "\x1b]8;;file:///tools\x1b\\\x1b[1m\x1b[34mtools\x1b]8;;\x1b\\",
    ];

    for raw in spellings {
        assert_eq!(
            style_at(raw, "tools"),
            Some(vec!["1".to_string(), "34".to_string()]),
            "{raw:?}"
        );
    }
    // Text inside a hyperlink's URL is not drawn, so it is never the match.
    assert_eq!(
        style_at("\x1b]8;;file:///x/tools\x1b\\\x1b[2mtools", "tools"),
        Some(vec!["2".to_string()])
    );
}

#[test]
fn hyperlink_labels_survive_terminal_sgr_reemission() {
    for style in ["", "\x1b[m", "\x1b[22;2m", "\x1b[38:2:20:30:40m\x1b[2m"] {
        let raw = format!("\x1b]8;;file:///tmp/geometry.ts#L14\x1b\\{style}geometry.ts:14:3\x1b]8;;\x1b\\");
        assert!(crate::terminal_report::has_hyperlink(&raw, "/geometry.ts#L14", "geometry.ts:14:3"));
        assert!(!crate::terminal_report::has_hyperlink(&raw, "/geometry.ts#L10", "geometry.ts:14:3"));
        assert!(!crate::terminal_report::has_hyperlink(&raw, "/geometry.ts#L14", "geometry.ts:10"));
    }
}

#[test]
fn unlinked_text_cannot_borrow_an_earlier_destination() {
    let raw = "\x1b]8;;file:///tmp/geometry.ts#L14\x1b\\wrong label\x1b]8;;\x1b\\geometry.ts:14:3";
    assert!(!crate::terminal_report::has_hyperlink(raw, "/geometry.ts#L14", "geometry.ts:14:3"));
    let broken = "\x1b]8;;file:///tmp/geometry.ts#L14\x1b\\\x1b[2Jgeometry.ts:14:3";
    assert!(!crate::terminal_report::has_hyperlink(broken, "/geometry.ts#L14", "geometry.ts:14:3"));
}

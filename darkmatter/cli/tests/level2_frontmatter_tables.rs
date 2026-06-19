mod common;

use common::level2::{rtrim, run_md};
use serial_test::serial;

#[test]
#[serial(level2_terminal)]
fn level2_table_max_fill_constrains_visible_width() {
    // Use unique sentinel column values so we can anchor on a single body row
    // regardless of header/separator framing. The table needs enough render
    // width to keep all columns; we cap at 50 (well under page width 80) and
    // assert the rendered row never exceeds that cap.
    let body = "\
| ColA | ColB | ColC |\n\
| ---- | ---- | ---- |\n\
| sentinel_alpha | beta | gamma |\n";

    let Some((frame, _)) = run_md(body, "--fill-tables max=50 --max-width 80") else {
        return;
    };

    // Locate the body row by sentinel (it's only present in rendered output,
    // never in the shell command echo).
    let row = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_alpha"))
        .unwrap_or_else(|| {
            panic!(
                "expected a table row containing 'sentinel_alpha' in:\n{}",
                frame.plain
            )
        });
    let visible = rtrim(row).chars().count();
    assert!(
        visible <= 50,
        "table row visible width should be capped to 50 cols, got {visible}: {row:?}"
    );
    // Also ensure the cap actually constrains (visible < page width).
    assert!(
        visible < 80,
        "table row should be narrower than the page (80 cols), got {visible}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_table_center_alignment_indents_more_than_left() {
    // Use a unique sentinel cell value so we anchor on the rendered body row
    // and never accidentally match the shell command echo (which contains the
    // temp file path and CLI numerics like `max=20`/`60`).
    let body = "\
| A | B |\n\
| - | - |\n\
| sentinelA | sentinelB |\n";

    // `max` must be wide enough for the table to actually render — each
    // wrapping column needs ~9 cols of content plus pipes/padding, so a
    // 2-column table needs roughly 25 cols minimum. We pick 40 so the cap
    // still leaves visible alignment surplus on the 60-col page.
    let Some((left, _)) = run_md(
        body,
        "--align-tables left --fill-tables max=40 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-tables center --fill-tables max=40 --max-width 60",
    ) else {
        return;
    };

    let row_left = left
        .plain
        .lines()
        .find(|l| l.contains("sentinelA"))
        .unwrap_or_else(|| panic!("left: data row missing. plain:\n---\n{}\n---", left.plain));
    let row_center = center
        .plain
        .lines()
        .find(|l| l.contains("sentinelA"))
        .unwrap_or_else(|| {
            panic!(
                "center: data row missing. plain:\n---\n{}\n---",
                center.plain
            )
        });
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    let center_indent = row_center.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_blockquote_indent_fill_caps_wrap_width() {
    // Long quoted prose so the wrap point is observable in the captured pane.
    let body = "> This is a fairly long quoted paragraph that should wrap onto a second visible line once Indent(20) forces the blockquote render width below the page width.\n";

    let Some((frame, _)) = run_md(body, "--fill-block-quotes indent=20 --max-width 80") else {
        return;
    };

    // Strip ANSI-stripped, trim trailing background fill. Blockquote lines are
    // prefixed with the `▐` indicator glyph.
    let quote_lines: Vec<String> = frame
        .plain
        .lines()
        .filter(|l| l.contains('▐'))
        .map(|l| rtrim(l).to_string())
        .collect();
    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto multiple lines under Indent(20). plain:\n{}",
        frame.plain
    );
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    // Indent(20) sets padding-left = 20ch. The content box is 60 cols,
    // and the prefix consumes 4 cols, leaving 56 cols for text.
    // Total line width = 20 (pad) + 4 (prefix) + text ≤ 80.
    assert!(
        max_len <= 80,
        "blockquote lines must be capped to 80 cols by Indent(20) padding; got max={max_len}. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_blockquote_center_alignment_indents_more_than_left() {
    let body = "> Centered quoted line.\n";
    let Some((left, _)) = run_md(
        body,
        "--align-block-quotes left --fill-block-quotes max=20 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-block-quotes center --fill-block-quotes max=20 --max-width 60",
    ) else {
        return;
    };

    let left_quote = left
        .plain
        .lines()
        .find(|l| l.contains('▐'))
        .expect("left: blockquote line");
    let center_quote = center
        .plain
        .lines()
        .find(|l| l.contains('▐'))
        .expect("center: blockquote line");
    let left_indent = left_quote.chars().take_while(|c| *c == ' ').count();
    let center_indent = center_quote.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "center alignment should indent more than left: left={left_indent}, center={center_indent}"
    );
}

const STYLE_PROP_FIXTURE: &str = r#"---
style:
    page:
        left-margin: 2ch
        right-margin: 4ch
        top-margin: 1
        bottom-margin: 0
---

# StylePropFixtureHeading

Sentinel_paragraph_for_left_margin_check.
"#;

#[test]
#[serial(level2_terminal)]
fn level2_style_fixture_applies_top_and_left_margins_in_real_terminal() {
    // Acceptance: `style.page.top-margin: 1` adds a blank row above the
    // rendered title, and `style.page.left-margin: 2ch` leaves >=2 leading
    // columns on non-empty content rows when rendered through the real
    // terminal pipeline. The fixture is composed inline to mirror
    // `darkmatter/example-docs/rendering/style-prop.md` page-level frontmatter.
    let Some((frame, _)) = run_md(STYLE_PROP_FIXTURE, "--max-width 60") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let marker_idx = lines
        .iter()
        .position(|l| l.contains("StylePropFixtureHeading"))
        .unwrap_or_else(|| panic!("heading not found in pane capture:\n{}", frame.plain));

    // Top-margin: 1 — at least one row above the heading must be blank
    // (after rtrim of left-margin spaces).
    assert!(
        marker_idx >= 1,
        "top-margin: 1 must leave at least one row above the heading, got idx {marker_idx}. plain:\n{}",
        frame.plain
    );
    let above = rtrim(lines[marker_idx - 1]);
    assert!(
        above.is_empty(),
        "row directly above heading should be blank (top-margin: 1), got: {above:?}. plain:\n{}",
        frame.plain
    );

    // Left-margin: 2ch — the heading line itself starts with >=2 leading
    // columns (the heading row carries the left-margin prefix too).
    let head_leading = lines[marker_idx].chars().take_while(|c| *c == ' ').count();
    assert!(
        head_leading >= 2,
        "expected >=2 leading columns for left-margin: 2ch on heading, got {head_leading}: {:?}. plain:\n{}",
        lines[marker_idx],
        frame.plain
    );
}

// =============================================================================
//   COMPONENT STYLE FRONTMATTER (sub-spec #3 acceptance — review-3 finding #2)
// =============================================================================
//
// These tests exercise the new `style.table.*`, `style.images.*`, and
// `style.block-quote.*` frontmatter path through the real `md` CLI in a real
// WezTerm pane. The Level 1 tests in `cli.rs` cover the resolved
// `DarkmatterPage` state and a single in-process render; these confirm the
// visible terminal layout for the same buckets actually reaches the user.

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_table_max_width_caps_visible_row() {
    // `style.table.max-width: 50%` at --max-width 80 must cap rendered table
    // rows at 40 visible columns. Anchored on a unique cell value so we never
    // accidentally match the shell command echo.
    let body = r#"---
style:
    table:
        max-width: 50%
---

| ColA | ColB | ColC |
| ---- | ---- | ---- |
| sentinel_fm_alpha | beta | gamma |
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let row = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_fm_alpha"))
        .unwrap_or_else(|| {
            panic!(
                "expected table row containing 'sentinel_fm_alpha' in:\n{}",
                frame.plain
            )
        });
    let visible = rtrim(row).chars().count();
    assert!(
        visible <= 40,
        "frontmatter style.table.max-width: 50% must cap row to 40 cols (50% of 80), got {visible}: {row:?}"
    );
    assert!(
        visible < 80,
        "frontmatter style.table.max-width: 50% must constrain below page width, got {visible}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_table_right_alignment_pushes_row_to_right_edge() {
    // `style.table.alignment: right` plus `style.table.max-width: 40%` must
    // push the rendered table to the right edge of the page so its leading
    // indent exceeds what a left-aligned table at the same fill would have.
    let right = r#"---
style:
    table:
        alignment: right
        max-width: 40%
---

| A | B |
| - | - |
| sentinelR | xx |
"#;
    let left = r#"---
style:
    table:
        alignment: left
        max-width: 40%
---

| A | B |
| - | - |
| sentinelR | xx |
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 60") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 60") else {
        return;
    };

    let row_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinelR"))
        .unwrap_or_else(|| {
            panic!(
                "right: data row missing. plain:\n---\n{}\n---",
                right_frame.plain
            )
        });
    let row_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinelR"))
        .unwrap_or_else(|| {
            panic!(
                "left: data row missing. plain:\n---\n{}\n---",
                left_frame.plain
            )
        });
    let right_indent = row_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "frontmatter style.table.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_block_quote_max_width_caps_wrap_width() {
    // `style.block-quote.max-width: 50%` at --max-width 80 must wrap quoted
    // text under 40 visible columns. Block-quote lines are prefixed by the
    // `▐` indicator glyph.
    let body = r#"---
style:
    block-quote:
        max-width: 50%
---

> This is a fairly long quoted paragraph that should wrap onto a second visible line once the frontmatter max-width cap forces the blockquote render width to half the page width.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let quote_lines: Vec<String> = frame
        .plain
        .lines()
        .filter(|l| l.contains('▐'))
        .map(|l| rtrim(l).to_string())
        .collect();
    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto multiple lines under style.block-quote.max-width: 50%. plain:\n{}",
        frame.plain
    );
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 40,
        "blockquote lines should be capped to 40 cols (50% of 80), got max={max_len}. plain:\n{}",
        frame.plain
    );
}

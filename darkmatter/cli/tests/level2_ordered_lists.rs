mod common;

use common::level2::{foreground_at_text, rtrim, run_md};
use serial_test::serial;

#[test]
#[serial(level2_terminal)]
fn level2_list_max_fill_caps_wrap_width() {
    // Long bullet so wrap is observable.
    let body = "- This is a notably long list item that has to wrap to a second visible row once Max(40) constrains the list render width to forty columns.\n- Follow-up.\n";

    let Some((frame, _)) = run_md(body, "--fill-lists max=40 --max-width 80") else {
        return;
    };

    // Locate the start of list output via a stable anchor, then collect the
    // list region until the next blank line. This avoids measuring shell
    // prompt + command-echo lines which span the full pane width.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("- This is"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();
    let max_len = list_region
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_len <= 40,
        "list lines should be capped to 40 cols, got max={max_len}. region:\n{}\nfull plain:\n{}",
        list_region.join("\n"),
        frame.plain
    );
    // Sanity-check: confirm wrap actually occurred (>= 3 list rows: long line
    // wraps to multiple rows + the Follow-up item).
    assert!(
        list_region.len() >= 3,
        "expected list to wrap onto multiple rows, got {} rows:\n{}",
        list_region.len(),
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_list_center_alignment_indents_more_than_left() {
    // Use short items so we can compare the marker line's leading indent.
    // Per sub-spec #4, when `style.li.alignment` (or the broadcast `--align-lists`
    // that writes Li alignment) shifts the body, the body becomes its own block
    // on a fresh line. We anchor on the body sentinel `xy` so the test works
    // regardless of whether the marker and body share a line.
    let body = "- xy\n- ab\n";
    let Some((left, _)) = run_md(
        body,
        "--align-lists left --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-lists center --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };

    let find_body_indent = |plain: &str, label: &str| -> usize {
        let line = plain
            .lines()
            .find(|l| l.trim_start() == "xy" || l.trim_start().starts_with("- xy"))
            .unwrap_or_else(|| panic!("{label}: list item not found. plain:\n{plain}"));
        line.chars().take_while(|c| *c == ' ').count()
    };
    let left_indent = find_body_indent(&left.plain, "left");
    let center_indent = find_body_indent(&center.plain, "center");
    assert!(
        center_indent > left_indent,
        "list center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_left_margin_offsets_bullet_in_real_terminal() {
    // `style.ul.left-margin: 4ch` must offset the bullet marker 4 columns from
    // the left when rendered through the real terminal pipeline.
    let body = r#"---
style:
    ul:
        left-margin: 4ch
---

- sentinel_ul_lm item
"#;

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let list_line = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ul_lm"))
        .unwrap_or_else(|| {
            panic!(
                "expected list item with sentinel in pane capture:\n{}",
                frame.plain
            )
        });
    let leading = list_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "style.ul.left-margin: 4ch must offset the marker by >=4 cols, got {leading}: {list_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_max_width_caps_wrap_width_in_real_terminal() {
    // `style.ul.max-width: 40` must wrap the bullet body at no more than 40
    // visible columns through the real terminal pipeline.
    let body = r#"---
style:
    ul:
        max-width: 40
---

- sentinel_ul_mw This is a notably long bullet item that has to wrap to a second visible row once the frontmatter cap constrains the list render width to forty columns.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("sentinel_ul_mw"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();
    let max_len = list_region
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_len <= 40,
        "style.ul.max-width: 40 must cap list lines at 40 cols, got max={max_len}. region:\n{}",
        list_region.join("\n")
    );
    assert!(
        list_region.len() >= 2,
        "expected list to wrap onto multiple rows, got {}:\n{}",
        list_region.len(),
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_left_margin_plus_max_width_stack_in_real_terminal() {
    // `style.ul.left-margin: 4ch` plus `style.ul.max-width: 40` must stack
    // correctly: 4-cell offset outside the body, body wrapping at <= 40 cols.
    let body = r#"---
style:
    ul:
        left-margin: 4ch
        max-width: 40
---

- sentinel_ul_stack This is a notably long bullet item that has to wrap to a second visible row once the frontmatter max-width cap constrains the list render width to forty columns total.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("sentinel_ul_stack"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();

    // First line must carry the 4-cell left margin.
    let first = list_region[0];
    let leading = first.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "first list line must carry >=4ch left margin, got {leading}: {first:?}"
    );
    // Body (after the 4-cell margin) must wrap at <= 40 cols.
    let max_body = list_region
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let trimmed = l.strip_prefix("    ").unwrap_or(l);
            rtrim(trimmed).chars().count()
        })
        .max()
        .unwrap_or(0);
    assert!(
        max_body <= 40,
        "body (after 4ch margin) must wrap at <= 40 cols, got max body={max_body}. region:\n{}",
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ol_alignment_right_indents_more_than_left_in_real_terminal() {
    // `style.ol.alignment: right` plus `style.ol.max-width: 40` must push the
    // ordered list to the right edge versus the left-aligned baseline.
    let right = r#"---
style:
    ol:
        alignment: right
        max-width: 40
---

1. sentinel_ol_align item
"#;
    let left = r#"---
style:
    ol:
        alignment: left
        max-width: 40
---

1. sentinel_ol_align item
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 80") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 80") else {
        return;
    };

    let row_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ol_align"))
        .unwrap_or_else(|| {
            panic!(
                "right: ordered list row missing. plain:\n{}",
                right_frame.plain
            )
        });
    let row_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ol_align"))
        .unwrap_or_else(|| {
            panic!(
                "left: ordered list row missing. plain:\n{}",
                left_frame.plain
            )
        });
    let right_indent = row_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "style.ol.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_li_alignment_right_aligns_body_in_real_terminal() {
    // Per spec, `style.li.alignment: right` affects the item body only — the
    // marker stays at the column dictated by the containing Ul. The body
    // becomes a block on its own line that is right-aligned within
    // `effective_width - body_width`.
    let body = r#"---
style:
    li:
        alignment: right
        max-width: 40
---

- sentinel_li_body
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    // Marker stays at column 0 (or wherever Ul column is, which is 0 here
    // since no Ul overrides).
    let marker_line = lines
        .iter()
        .find(|l| l.trim_start().starts_with('-') && !l.contains("sentinel_li_body"))
        .unwrap_or_else(|| panic!("marker line not found. plain:\n{}", frame.plain));
    let marker_leading = marker_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        marker_leading == 0,
        "marker should stay at Ul column 0 (style.li.* affects body only), got {marker_leading}: {marker_line:?}"
    );
    // Body line is right-aligned within the effective width, well past column 0.
    let body_line = lines
        .iter()
        .find(|l| l.contains("sentinel_li_body"))
        .unwrap_or_else(|| panic!("body line not found. plain:\n{}", frame.plain));
    let body_leading = body_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        body_leading >= 30,
        "li body must be right-aligned, got {body_leading} leading spaces: {body_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_cli_align_lists_broadcast_indents_in_real_terminal() {
    // `--align-lists center` plus `--fill-lists max=30` must broadcast to Ul/Ol/Li
    // so both bullet and numbered items render with extra leading indent vs
    // left-aligned baseline.
    let body = "- sentinel_alpha\n1. sentinel_numbered\n";

    let Some((left, _)) = run_md(
        body,
        "--align-lists left --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-lists center --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };

    let find_row = |plain: &str, needle: &str, label: &str| -> String {
        plain
            .lines()
            .find(|l| l.contains(needle))
            .map(|l| l.to_string())
            .unwrap_or_else(|| panic!("{label}: row '{needle}' not found in plain:\n{plain}"))
    };
    let left_ul = find_row(&left.plain, "sentinel_alpha", "left ul");
    let center_ul = find_row(&center.plain, "sentinel_alpha", "center ul");
    let left_ol = find_row(&left.plain, "sentinel_numbered", "left ol");
    let center_ol = find_row(&center.plain, "sentinel_numbered", "center ol");

    let lead = |s: &str| s.chars().take_while(|c| *c == ' ').count();
    assert!(
        lead(&center_ul) > lead(&left_ul),
        "--align-lists center must indent Ul more than left: left={}, center={}",
        lead(&left_ul),
        lead(&center_ul)
    );
    assert!(
        lead(&center_ol) > lead(&left_ol),
        "--align-lists center must indent Ol more than left: left={}, center={}",
        lead(&left_ol),
        lead(&center_ol)
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_cli_align_ul_granular_indents_only_ul_in_real_terminal() {
    // `--align-ul center` plus `--fill-ul max=30` must indent the unordered list
    // versus the left baseline, while leaving an ordered list unaffected.
    let body = "- sentinel_ul_only\n\n1. sentinel_ol_only\n";

    let Some((left, _)) = run_md(
        body,
        "--align-ul left --fill-ul max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-ul center --fill-ul max=30 --max-width 60",
    ) else {
        return;
    };

    let find_row = |plain: &str, needle: &str, label: &str| -> String {
        plain
            .lines()
            .find(|l| l.contains(needle))
            .map(|l| l.to_string())
            .unwrap_or_else(|| panic!("{label}: row '{needle}' not found in plain:\n{plain}"))
    };
    let left_ul = find_row(&left.plain, "sentinel_ul_only", "left ul");
    let center_ul = find_row(&center.plain, "sentinel_ul_only", "center ul");
    let left_ol = find_row(&left.plain, "sentinel_ol_only", "left ol");
    let center_ol = find_row(&center.plain, "sentinel_ol_only", "center ol");

    let lead = |s: &str| s.chars().take_while(|c| *c == ' ').count();
    assert!(
        lead(&center_ul) > lead(&left_ul),
        "--align-ul center must indent Ul more than left: left={}, center={}",
        lead(&left_ul),
        lead(&center_ul)
    );
    // Ol must NOT shift: --align-ul only targets Ul.
    assert_eq!(
        lead(&center_ol),
        lead(&left_ol),
        "--align-ul must not affect Ol: left_ol={}, center_ol={}",
        lead(&left_ol),
        lead(&center_ol)
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_ul_color_inherits_into_li_body() {
    let body = "---\n\
style:\n  ul:\n    color: red-500\n---\n\
- listbodyalpha\n- listbodybeta\n";

    let Some((frame, _)) = run_md(body, "--max-width 40") else {
        return;
    };

    let red_500 = Some((251, 44, 54));
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodyalpha").flatten(),
        red_500,
        "first list body must inherit ul.color. raw={:?}",
        frame.raw
    );
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodybeta").flatten(),
        red_500,
        "second list body must inherit ul.color. raw={:?}",
        frame.raw
    );
    // Layout must also still show the bodies in the plain view.
    assert!(
        frame.plain.contains("listbodyalpha") && frame.plain.contains("listbodybeta"),
        "list bodies missing in plain capture:\n{}",
        frame.plain
    );
}

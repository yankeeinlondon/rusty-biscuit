use super::support::*;

#[test]
#[serial(level2_terminal)]
fn level2_tree_rich_code_block_title_gutter_and_highlight_survive_real_terminal() {
    let body = "```rust title=\"Demo Snippet\" line-numbering=true highlight=2\n\
                fn parity_demo() {\n    println!(\"render tree\");\n}\n```\n";
    let Some((frame, _dir)) = run_in_pane(body, "code_block_rich") else {
        return;
    };

    // 1. The info-string title surfaces in the code-block header row.
    assert!(
        frame.plain.contains("Demo Snippet"),
        "code-block title missing from real-terminal capture. plain:\n{}",
        frame.plain
    );

    // 2. The code body survives syntax highlighting + the real-terminal trip.
    for token in &["parity_demo", "render tree"] {
        assert!(
            frame.plain.contains(token),
            "code body token {token:?} missing from capture. plain:\n{}",
            frame.plain
        );
    }

    // 3. Line-number gutter: the renderer emits a right-aligned line number
    //    followed by ` │ `. Three source lines produce gutters `1 │`, `2 │`,
    //    `3 │`. The `│` separator appears only in the gutter (the code body
    //    has no pipes), so each numbered separator proves the gutter layout
    //    survived to the pane.
    for gutter in &["1 \u{2502}", "2 \u{2502}", "3 \u{2502}"] {
        assert!(
            frame.plain.contains(gutter),
            "line-number gutter {gutter:?} missing from capture. plain:\n{}",
            frame.plain
        );
    }

    // 4. Syntax highlighting: foreground color SGRs must survive into the real
    //    pane (the plain no-color fallback emits none). WezTerm re-emits true
    //    color in the colon form; accept colon, semicolon, or 256-color.
    assert!(
        frame.raw.contains("\u{1b}[38;2;")
            || frame.raw.contains("\u{1b}[38:2:")
            || frame.raw.contains("\u{1b}[38;5;"),
        "expected foreground syntax-highlight SGRs in code-block capture; raw:\n{}",
        frame.raw
    );

    // 5. Highlighted-line styling: line 2 (`highlight=2`) is painted with a
    //    background distinct from the other code lines. WezTerm re-emits each
    //    cell's background, and `plain`/`raw` lines are index-aligned, so we can
    //    isolate the highlighted row by its text. WezTerm *omits* a background
    //    SGR when the cell background equals the pane default, so the
    //    non-highlighted code lines (whose background matches the theme/pane
    //    default) carry no explicit background sequence — but the highlighted
    //    line, whose background is the theme default plus the highlight delta,
    //    must carry an explicit background SGR that the non-highlighted line
    //    lacks. Comparing two rows from the *same* capture is normalization-proof.
    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();
    let row_index = |needle: &str| {
        plain_rows
            .iter()
            .position(|p| p.contains(needle))
            .unwrap_or_else(|| {
                panic!(
                    "could not locate row {needle:?} in capture.\nplain:\n{}",
                    frame.plain
                )
            })
    };
    let plain_row = row_index("parity_demo"); // line 1 — not highlighted
    let highlighted_row = row_index("render tree"); // line 2 — highlight=2

    let plain_bgs = background_colors(raw_rows[plain_row]);
    let highlight_bgs = background_colors(raw_rows[highlighted_row]);
    assert!(
        !highlight_bgs.is_empty(),
        "highlighted line (`highlight=2`) must carry an explicit background SGR; raw row:\n{}",
        raw_rows[highlighted_row]
    );
    assert_ne!(
        plain_bgs, highlight_bgs,
        "highlighted line (`highlight=2`) must carry a background distinct from a \
         non-highlighted line.\nnon-highlighted raw:\n{}\nhighlighted raw:\n{}",
        raw_rows[plain_row], raw_rows[highlighted_row]
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_page_code_panel_is_contiguous_inverted_rectangle() {
    let (ml, mr, mt, mb) = (4u16, 4u16, 1u16, 1u16);
    let body =
        "# A Heading\n\nLead prose paragraph.\n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n";
    let Some((frame, cols)) = run_page_in_pane(body, "page_code_panel", ml, mr, mt, mb) else {
        return;
    };

    let target = github_light_bg();
    let left = ml as usize;
    let right_edge = cols - mr as usize;
    let content_width = right_edge - left;

    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();

    // Every code-panel row must be a single contiguous background run (no
    // `\x1b[K` gap, Defect #1) that ends on the content right boundary
    // (Defect #2 — pill and body coherent). Full-bleed rows (body + padding)
    // additionally fill from the left margin; the right-aligned language pill is
    // narrow chrome that legitimately opens mid-line (spec Stage 4.2).
    let mut full_bleed_rows = 0;
    let mut pill_rows = 0;
    for (idx, raw) in raw_rows.iter().enumerate() {
        if let Some((start, end, count)) = target_extent(raw, target) {
            let plain = plain_rows.get(idx).copied().unwrap_or("");
            assert_eq!(
                count, 1,
                "code panel row must be one contiguous background rectangle (no \\x1b[K gap); \
                 row {idx} plain={plain:?} raw={raw:?}"
            );
            assert_eq!(
                end, right_edge,
                "code panel background must end at the content right edge (width - right = {right_edge}); \
                 row {idx} plain={plain:?}"
            );
            if end - start == content_width {
                assert_eq!(
                    start, left,
                    "full-bleed code row must start at the left margin ({left}); \
                     row {idx} plain={plain:?}"
                );
                full_bleed_rows += 1;
            } else {
                pill_rows += 1;
            }
        }
    }
    assert!(
        full_bleed_rows >= 3,
        "expected >= 3 full-bleed code-panel rows (body lines + top/bottom padding) filling \
         the content rectangle; found {full_bleed_rows}. plain:\n{}",
        frame.plain
    );
    assert!(
        pill_rows >= 1,
        "expected the right-aligned language pill row (narrow chrome ending at the right edge); \
         found {pill_rows}. plain:\n{}",
        frame.plain
    );

    // Contrast: the heading (prose) row must NOT carry the inverted code-panel
    // background — prose follows the real (dark) mode, code inverts.
    let heading_idx = plain_rows
        .iter()
        .position(|p| p.contains("A Heading"))
        .unwrap_or_else(|| panic!("heading row missing from capture. plain:\n{}", frame.plain));
    assert!(
        target_extent(raw_rows[heading_idx], target).is_none(),
        "prose heading row must not carry the inverted code-panel background; raw:\n{}",
        raw_rows[heading_idx]
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_page_no_double_blank_rows_between_code_blocks() {
    let (ml, mr, mt, mb) = (4u16, 4u16, 0u16, 0u16);
    let body = "```rust\nfn a() {}\n```\n\n\n\n```rust\nfn b() {}\n```\n";
    let Some((frame, _cols)) = run_page_in_pane(body, "page_rhythm", ml, mr, mt, mb) else {
        return;
    };

    let target = github_light_bg();
    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();

    // Restrict the scan to the span between the first and last code-panel row so
    // the shell prompt / sentinel framing is excluded.
    let panel_idxs: Vec<usize> = raw_rows
        .iter()
        .enumerate()
        .filter(|(_, raw)| target_extent(raw, target).is_some())
        .map(|(i, _)| i)
        .collect();
    assert!(
        panel_idxs.len() >= 2,
        "expected two code panels in the capture. plain:\n{}",
        frame.plain
    );
    let (first, last) = (panel_idxs[0], *panel_idxs.last().unwrap());

    let is_blank = |idx: usize| -> bool {
        let visibly_empty = plain_rows.get(idx).is_none_or(|p| p.trim().is_empty());
        let no_panel_bg = target_extent(raw_rows[idx], target).is_none();
        visibly_empty && no_panel_bg
    };

    let mut consecutive_blanks = 0;
    for idx in first..=last {
        if is_blank(idx) {
            consecutive_blanks += 1;
            assert!(
                consecutive_blanks < 2,
                "found a run of >= 2 consecutive blank rows between code blocks (row {idx}); \
                 Markdown vertical-rhythm invariant violated. plain:\n{}",
                frame.plain
            );
        } else {
            consecutive_blanks = 0;
        }
    }
}

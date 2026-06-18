mod common;

use common::level2::{rtrim, run_md, run_md_env};
use serial_test::serial;

#[test]
#[serial(level2_terminal)]
fn level2_margin_renders_blank_top_rows() {
    // Use a recognizable heading marker so we can anchor the output without
    // depending on parsing the command echo (which may wrap across pane lines).
    let Some((frame, _)) = run_md("# UniqueMarginMarker\n", "-m 2 --max-width 40") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let marker_idx = lines
        .iter()
        .position(|l| l.contains("UniqueMarginMarker"))
        .unwrap_or_else(|| {
            panic!(
                "marker line not found in captured pane. plain:\n{}",
                frame.plain
            )
        });

    // Margin-top=2 means the heading must be preceded by at least 2 blank
    // rows above it (within the rendered output region).
    assert!(
        marker_idx >= 2,
        "marker must have at least 2 preceding rows, found at idx {marker_idx}"
    );
    let row_above = rtrim(lines[marker_idx - 1]);
    let row_two_above = rtrim(lines[marker_idx - 2]);
    assert!(
        row_above.is_empty(),
        "row directly above heading should be blank (margin), got: {row_above:?}"
    );
    assert!(
        row_two_above.is_empty(),
        "row two above heading should be blank (margin), got: {row_two_above:?}"
    );
}

/// Returns `true` when the captured raw stream contains any background SGR
/// attribute (24-bit `48;2;...`, 256-color `48;5;...`, or basic `40-47`/
/// `100-107` palette codes). WezTerm's `get-text --escapes` re-emits cell
/// attributes; the exact form depends on terminal/runtime settings.
fn raw_has_background_sgr(raw: &str) -> bool {
    raw.contains("\x1b[48;2;") || raw.contains("\x1b[48;5;") || raw.contains("\x1b[48:")
}

#[test]
#[serial(level2_terminal)]
fn level2_page_bg_subtle_emits_bg_sgr() {
    // Use a multi-row body so padded background rows are guaranteed to fall
    // inside the captured viewport.
    let body = "# Subtle\n\nLine one.\n\nLine two.\n";
    let Some((frame, _)) = run_md(body, "--page-bg subtle --padding 2 --max-width 40") else {
        return;
    };

    assert!(
        raw_has_background_sgr(&frame.raw),
        "expected a background SGR for --page-bg subtle. raw len={}, plain:\n{}",
        frame.raw.len(),
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_page_bg_pronounced_emits_bg_sgr() {
    let body = "# Pronounced\n\nLine one.\n\nLine two.\n";
    let Some((frame, _)) = run_md(body, "--page-bg pronounced --padding 2 --max-width 40") else {
        return;
    };

    assert!(
        raw_has_background_sgr(&frame.raw),
        "expected a background SGR for --page-bg pronounced. raw len={}",
        frame.raw.len()
    );
    // Pronounced should also reset attributes so following content (the next
    // shell prompt) is not stuck with the page bg.
    assert!(
        frame.raw.contains("\x1b[0m") || frame.raw.contains("\x1b[m"),
        "expected a reset SGR after the pronounced page surface"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_max_width_caps_visible_content_columns() {
    // Long paragraph; with --max-width 40 every wrapped line must fit in 40
    // columns. Anchor on a unique sentinel so we can locate the output region.
    let body = "Sentinel_paragraph Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore.\n";
    let Some((frame, _)) = run_md(body, "--max-width 40") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let sentinel_idx = lines
        .iter()
        .position(|l| l.contains("Sentinel_paragraph"))
        .unwrap_or_else(|| {
            panic!(
                "sentinel line not found in pane capture. plain:\n{}",
                frame.plain
            )
        });

    // Check the sentinel line and the next two lines (wrap continuation) all
    // fit within 40 visible columns.
    for (offset, line) in lines.iter().skip(sentinel_idx).take(3).copied().enumerate() {
        let visible = rtrim(line).chars().count();
        assert!(
            visible <= 40,
            "wrap line +{offset} exceeds --max-width 40, got {visible} cols: {line:?}"
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_line_numbers_true_renders_gutter() {
    let body = "```rust\nfn main() {}\nlet x = 1;\n```\n";
    let Some((frame, _)) = run_md(body, "--line-numbers=true --max-width 60") else {
        return;
    };

    // Line-number gutter uses the box-drawing "│" separator.
    assert!(
        frame.plain.contains('│'),
        "expected line-number gutter '│' in plain output. plain:\n{}",
        frame.plain
    );
    // And the gutter shows numeric line numbers.
    assert!(
        frame.plain.contains("1 │") || frame.plain.contains("1 │ "),
        "expected '1 │' gutter prefix in plain output. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_line_numbers_false_omits_gutter() {
    let body = "```rust\nfn main() {}\nlet x = 1;\n```\n";
    let Some((frame, _)) = run_md(body, "--line-numbers=false --max-width 60") else {
        return;
    };

    // The "│" gutter separator should not appear without line numbers.
    // Pages of regular code don't otherwise emit U+2502.
    assert!(
        !frame.plain.contains('│'),
        "did not expect gutter '│' with --line-numbers=false. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_end_to_end_layout_dimensions() {
    // Cross-component end-to-end capture: a small page with multiple
    // components rendered under a complete set of layout flags. Verifies
    // visible row dimensions and the top/bottom margin behavior at once.
    let body = "\
# E2E heading\n\
\n\
Some prose paragraph.\n\
\n\
- list item alpha\n\
- list item beta\n\
\n\
> A quoted observation.\n\
";

    let Some((frame, _)) = run_md(
        body,
        "-m 2 --padding 1 --max-width 40 --page-bg subtle --align-block-quotes left",
    ) else {
        return;
    };

    // Locate the heading anchor.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let head_idx = lines
        .iter()
        .position(|l| l.contains("E2E heading"))
        .unwrap_or_else(|| panic!("missing heading anchor in pane:\n{}", frame.plain));

    // Margin top = 2: two blank rows above the (left-padded) heading.
    assert!(
        head_idx >= 2,
        "heading should sit at row index >= 2 to allow 2 margin rows above, got {head_idx}"
    );
    assert!(
        rtrim(lines[head_idx - 1]).is_empty(),
        "row above heading must be blank (margin), got: {:?}",
        lines[head_idx - 1]
    );
    assert!(
        rtrim(lines[head_idx - 2]).is_empty(),
        "two rows above heading must be blank (margin), got: {:?}",
        lines[head_idx - 2]
    );

    // Visible content rows should not exceed margin_x(2*2) + padding_x(1*2)
    // + max_width(40) = 46 columns.
    let max_visible = lines
        .iter()
        .skip(head_idx)
        .take(10)
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_visible <= 46,
        "content rows should fit in 4 margin + 2 padding + 40 max-width = 46 cols, got {max_visible}. plain:\n{}",
        frame.plain
    );

    // Every component anchor must be present in the captured pane.
    for anchor in [
        "Some prose paragraph",
        "list item alpha",
        "list item beta",
        "A quoted observation",
    ] {
        assert!(
            frame.plain.contains(anchor),
            "expected '{anchor}' in captured plain output. plain:\n{}",
            frame.plain
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_color_depth_none_preserves_visible_layout() {
    let body = "---\n\
style:\n  page:\n    color: red-500\n---\n\
# UniqueNoColorHeading\n\n\
- BulletItem\n\n\
| Hdr |\n|---|\n| Cell |\n";

    let Some((frame, _)) = run_md_env(body, "--max-width 40", &[("TERM", "dumb")]) else {
        return;
    };

    // All structural anchors must survive ColorDepth::None.
    assert!(
        frame.plain.contains("UniqueNoColorHeading"),
        "heading text must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("BulletItem"),
        "list body must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Hdr") && frame.plain.contains("Cell"),
        "table cells must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
}

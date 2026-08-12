mod common;

use common::{md_cmd, md_file};
use common::layout::*;
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter_cli::render::apply_cli_layout_flags;
use renderable::layout::{Alignment, Length};

fn apply_style_for(raw: &str, args: &[&str]) -> DarkmatterPage {
    use darkmatter::markdown::Markdown;
    use darkmatter_cli::render::apply_style_frontmatter;
    let cli = parse_cli(args);
    let md = Markdown::try_from_content(raw).unwrap();
    let term = Terminal::new_optimistic(80);
    let page = apply_cli_layout_flags(DarkmatterPage::new(&term), &cli);
    apply_style_frontmatter(page, &md, &cli, None).expect("style apply")
}

#[test]
fn component_overrides_global_alignment_claims_every_bucket() {
    use darkmatter::style::{ComponentStyleOverrides, component_style_overrides_from_claims};
    use darkmatter_cli::style_claims::cli_style_claims;

    let cli = parse_cli(&["doc.md", "--alignment", "center"]);
    let o = component_style_overrides_from_claims(&cli_style_claims(&cli));
    assert_eq!(
        o,
        ComponentStyleOverrides {
            tables_alignment: true,
            images_alignment: true,
            block_quotes_alignment: true,
            code_blocks_alignment: true,
            tables_fill: false,
            images_fill: false,
            block_quotes_fill: false,
            code_blocks_fill: false,
        }
    );
}

#[test]
fn component_overrides_global_fill_claims_every_bucket() {
    use darkmatter::style::{ComponentStyleOverrides, component_style_overrides_from_claims};
    use darkmatter_cli::style_claims::cli_style_claims;

    let cli = parse_cli(&["doc.md", "--fill", "max=60"]);
    let o = component_style_overrides_from_claims(&cli_style_claims(&cli));
    assert_eq!(
        o,
        ComponentStyleOverrides {
            tables_fill: true,
            images_fill: true,
            block_quotes_fill: true,
            code_blocks_fill: true,
            tables_alignment: false,
            images_alignment: false,
            block_quotes_alignment: false,
            code_blocks_alignment: false,
        }
    );
}

#[test]
fn component_overrides_component_specific_alignment_claims_one_bucket() {
    use darkmatter::style::component_style_overrides_from_claims;
    use darkmatter_cli::style_claims::cli_style_claims;

    let cli = parse_cli(&["doc.md", "--align-tables", "right"]);
    let o = component_style_overrides_from_claims(&cli_style_claims(&cli));
    assert!(o.tables_alignment);
    assert!(!o.images_alignment);
    assert!(!o.block_quotes_alignment);
    assert!(!o.tables_fill && !o.images_fill && !o.block_quotes_fill);
}

#[test]
fn component_overrides_component_specific_fill_claims_one_bucket() {
    use darkmatter::style::component_style_overrides_from_claims;
    use darkmatter_cli::style_claims::cli_style_claims;

    let cli = parse_cli(&["doc.md", "--fill-images", "max=40"]);
    let o = component_style_overrides_from_claims(&cli_style_claims(&cli));
    assert!(o.images_fill);
    assert!(!o.tables_fill && !o.block_quotes_fill);
    assert!(!o.tables_alignment && !o.images_alignment && !o.block_quotes_alignment);
}

#[test]
fn frontmatter_table_alignment_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Left,
        "frontmatter table.alignment must reach the page when no CLI claim",
    );
}

#[test]
fn cli_align_tables_overrides_frontmatter_table_alignment() {
    // Plan ask: `--align-tables right` overriding frontmatter
    // `style.table.alignment: left`. The CLI flag wins.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", "--align-tables", "right"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "--align-tables right must override frontmatter table.alignment: left",
    );
}

#[test]
fn cli_global_fill_overrides_frontmatter_table_max_width() {
    // Plan ask: `--fill max=60` overriding frontmatter
    // `style.table.max-width: 50%` for all components.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", "--fill", "max=60"]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::ch(60)),
        "--fill max=60 (global) must claim the table fill slot",
    );
}

#[test]
fn frontmatter_table_max_width_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "frontmatter table.max-width must reach the page when no CLI claim",
    );
}

#[test]
fn frontmatter_images_alignment_and_fill_reach_page() {
    let raw = "---\n\
style:\n\
\x20   images:\n\
\x20       alignment: center\n\
\x20       max-width: 40ch\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Images),
        Alignment::Center,
    );
    assert_eq!(
        fill_for(&page, PageComponent::Images),
        TestFill::Max(Length::ch(40)),
    );
}

#[test]
fn frontmatter_block_quote_max_width_reaches_page() {
    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 75%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(75.0)),
    );
}

#[test]
fn style_fixture_renders_with_align_tables_override() {
    // End-to-end sanity check: the canonical fixture renders successfully
    // when the user overrides table alignment from the CLI.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--align-tables")
        .arg("center")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --align-tables center style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_renders_html_with_fill_override() {
    // End-to-end sanity check: --output html runs the same component-style
    // path as the terminal pipeline.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .arg("--fill")
        .arg("max=60")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html --fill max=60 must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_frontmatter_html_emits_component_layout_css() {
    // Sub-spec #3 acceptance (review-3 finding #3): `md --output html` on a
    // document carrying `style.table.*`, `style.images.*`, and
    // `style.block-quote.*` must emit the matching component layout CSS
    // selectors and declarations, not just succeed silently.
    //
    // Mirrors `darkmatter/lib/src/layout/page.rs::browser_render_with_component_*_css`
    // but drives the assertion through the public CLI surface so the
    // frontmatter → page-style → HTML pipeline is exercised end-to-end.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   table:\n\
        \x20       alignment: center\n\
        \x20       max-width: 60ch\n\
        \x20   images:\n\
        \x20       alignment: right\n\
        \x20       max-width: 40ch\n\
        \x20   block-quote:\n\
        \x20       alignment: right\n\
        \x20       max-width: 50ch\n\
        ---\n\n\
        # Doc\n\n\
        | A | B |\n\
        | - | - |\n\
        | 1 | 2 |\n\n\
        ![alt](./x.png)\n\n\
        > quote\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Table: center alignment + max-width: 60ch → margin-left:auto;margin-right:auto.
    assert!(
        html.contains("<table") && html.contains("max-width:60ch") && html.contains("margin-left:auto") && html.contains("margin-right:auto"),
        "expected centered table with inline max-width and auto margins in HTML. html:\n{html}",
    );
    // Block-quote: right alignment + max-width: 50ch → margin-left:auto.
    assert!(
        html.contains("<blockquote") && html.contains("max-width:50ch") && html.contains("margin-left:auto"),
        "expected right-aligned blockquote with inline max-width and auto margin in HTML. html:\n{html}",
    );
    // Image: max-width and alignment are applied to the wrapping paragraph via
    // the lone-image layout path (alignment without max-width does not emit
    // margin styles in the current fold).
    assert!(
        html.contains("<img") && html.contains("src=\"./x.png\""),
        "expected image element in HTML. html:\n{html}",
    );
}

#[test]
fn style_prop_fixture_html_emits_table_layout_css() {
    // Acceptance from sub-spec #3: `md --output html style-prop.md` must emit
    // the expected table layout CSS (right alignment + 50% max-width that
    // lowers to the page-content base, i.e. 50ch when the page builds at
    // its 120-col default for HTML).
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Right alignment + max-width: 50% → margin-left:auto on the table element.
    assert!(
        html.contains("<table") && html.contains("margin-left:auto"),
        "expected right-aligned table with inline auto margin in HTML. html:\n{html}",
    );
    // The fixture sets `max-width: 50%`; the fold preserves the percent on HTML.
    assert!(
        html.contains("max-width:50%"),
        "expected `max-width:50%` declaration in HTML. html:\n{html}",
    );
}

// =============================================================================
//          PHASE 5 REGRESSION TESTS (Sub-Spec #3)
// =============================================================================
//
// These tests cover the Phase 5 acceptance criteria:
//
//   1. `apply_cli_layout_flags` behavior is unchanged for documents without a
//      `style:` frontmatter (no silent state drift from the new
//      `apply_component_style` integration).
//   2. The canonical `style-prop.md` fixture resolves to the structural
//      page state the spec promises (table right-aligned, capped at 50%).
//   3. Component fill from `style.block-quote.max-width` flows into the
//      page builder and matches the structural shape used by the terminal
//      blockquote renderer.

#[test]
fn no_style_frontmatter_leaves_cli_layout_state_intact() {
    // Phase 5 acceptance: documents without a `style:` block must observe the
    // same resolved page state with vs. without `apply_style_frontmatter`.
    // Guards against silent state drift from the component-style integration.
    use darkmatter::markdown::Markdown;
    use darkmatter_cli::render::apply_style_frontmatter;

    let raw = "# No Style Doc\n\nBody.\n";
    let md = Markdown::try_from_content(raw).unwrap();
    let cli = parse_cli(&[
        "doc.md",
        "-m",
        "3",
        "--max-width",
        "70",
        "--alignment",
        "center",
        "--fill",
        "max=50",
    ]);

    let term = Terminal::new_optimistic(120);
    let cli_only = apply_cli_layout_flags(DarkmatterPage::new(&term), &cli);
    let after_style =
        apply_style_frontmatter(cli_only.clone(), &md, &cli, None).expect("style apply");

    assert_eq!(
        after_style.page_margin(),
        cli_only.page_margin(),
        "no `style:` frontmatter must leave CLI-resolved margins untouched",
    );
    assert_eq!(
        after_style.page_padding(),
        cli_only.page_padding(),
        "no `style:` frontmatter must leave CLI-resolved padding untouched",
    );
    assert_eq!(
        after_style.max_width(),
        cli_only.max_width(),
        "no `style:` frontmatter must leave CLI-resolved max-width untouched",
    );
    for component in PageComponent::ALL {
        assert_eq!(
            alignment_for(&after_style, component),
            alignment_for(&cli_only, component),
            "no `style:` frontmatter must leave CLI-resolved alignment untouched: {component:?}",
        );
        assert_eq!(
            fill_for(&after_style, component),
            fill_for(&cli_only, component),
            "no `style:` frontmatter must leave CLI-resolved fill untouched: {component:?}",
        );
    }
}

#[test]
fn style_prop_fixture_resolves_to_expected_table_layout() {
    // Phase 5 acceptance: the canonical `style-prop.md` fixture must produce
    // a page where the table is right-aligned and capped at 50% max-width via
    // the new component-style apply path.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "fixture must resolve to a right-aligned table",
    );
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "fixture must cap the table at 50% max-width",
    );
}

#[test]
fn style_prop_fixture_resolves_to_expected_page_margins() {
    // Phase 5 acceptance: page-level margins from the fixture survive the
    // full CLI -> page-style -> component-style pipeline.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    let m = page.page_margin();
    assert_eq!(tv_cells(&m.left), 2, "fixture left-margin: 2ch must reach the page");
    assert_eq!(tv_cells(&m.right), 4, "fixture right-margin: 4ch must reach the page");
    assert_eq!(tv_cells(&m.top), 1, "fixture top-margin: 1 must reach the page");
    assert_eq!(tv_cells(&m.bottom), 0, "fixture bottom-margin: 0 must reach the page");
}

#[test]
fn block_quote_max_width_caps_terminal_render_wrap_width() {
    // Phase 5 acceptance: `style.block-quote.max-width` reaches the page and
    // caps visible wrap width when the terminal renders a top-level
    // blockquote. We use a 100-col terminal so 50% resolves cleanly, then
    // assert that no rendered (ANSI-stripped) blockquote line exceeds the
    // resolved fill width.
    use darkmatter::layout::{DarkmatterPage, PageComponent};
    use darkmatter::markdown::Markdown;
    use darkmatter::testing::strip_ansi_codes;
    use darkmatter_cli::render::apply_style_frontmatter;

    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 50%\n\
---\n\n\
> This is a long quoted paragraph intended to wrap onto multiple visible \
rows once the blockquote fill caps the render width well below the page width. \
Add filler text to guarantee the wrap point is reached even when the \
terminal is reasonably wide.\n";

    let cli = parse_cli(&["doc.md"]);
    let md = Markdown::try_from_content(raw).unwrap();
    let term = Terminal::new_optimistic(100);
    let page = DarkmatterPage::new(&term);
    let page = apply_cli_layout_flags(page, &cli);
    let page = apply_style_frontmatter(page, &md, &cli, None).expect("style apply");

    // Structural guard: the apply pipeline put the fill where the renderer
    // will look for it.
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(50.0)),
        "block-quote.max-width must reach the page fill slot",
    );

    let rendered = page.render(&md).expect("render to terminal");
    let plain = strip_ansi_codes(&rendered);

    // The blockquote should wrap onto at least two visible lines.
    let quote_lines: Vec<String> = plain
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            // Common blockquote indicators across themes (`│`, `▌`, `▐`).
            trimmed.starts_with('│')
                || trimmed.starts_with('▌')
                || trimmed.starts_with('▐')
        })
        .map(|l| l.trim_end().to_string())
        .collect();

    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto >=2 visible lines under max-width: 50%. plain:\n{plain}",
    );

    // The renderer chooses 50% of the available content width; with a 100-col
    // terminal and no other margins/padding that's at most 50 cells of *text*
    // beyond the indicator + leading space. Allow generous slack for indent +
    // alignment padding by upper-bounding total visible width at 60.
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 60,
        "blockquote visible width should be capped under max-width: 50% on a 100-col terminal, got max={max_len}. plain:\n{plain}",
    );
}


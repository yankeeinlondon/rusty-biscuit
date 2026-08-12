//! Pre-cutover reference output for representative `style:` per-component cases.
//!
//! These snapshots are the parity *reference* (not a byte contract) the cutover
//! is diffed against; intended diffs are re-accepted with a note in Phase 6.
//!
//! ## Full-document form via `render_to_browser_document` (accepted)
//!
//! The browser references characterize the full-page render (design-token
//! `:root` head, page-wrapper CSS), so they render through
//! `render_to_browser_document` — the content-independent full-document method —
//! after `render_to_browser` became body-only. The undecorated fixtures were
//! already complete documents and are byte-identical; the two decorated
//! fixtures (`reference_page_background_pronounced`,
//! `reference_page_margin_and_padding`) are wrapped in a real
//! `<!DOCTYPE html><html><head>…</head><body>…</body></html>` scaffold whose
//! `<head>` carries the charset/viewport/title and the design-token `:root` +
//! `.code-block` panel stylesheet, while the `<body>` holds only the
//! `.darkmatter-page` frame — so every reference is a complete, openable
//! document with a non-empty head (the head-fix for the standalone path).
//!
//! ## Phase 8 review: `auto` page-wrapper side margins (accepted)
//!
//! Five browser snapshots re-baselined the page wrapper's horizontal margins
//! from `margin: 0ch 0ch 0ch 0ch` to `margin: 0ch auto 0ch auto`:
//! `reference_block_quote_width_and_left`, `reference_centered_table`,
//! `reference_list_left_margin`, `reference_page_background_pronounced`, and
//! `reference_table_max_width`.
//!
//! **CSS rationale (accepted):** every one of these fixtures emits a
//! `max-width`-capped `.darkmatter-page` wrapper with both author side margins
//! left at their default (zero). `margin-left: auto; margin-right: auto` is the
//! canonical CSS idiom for horizontally centering a fixed-`max-width` block in
//! its container; the old `0ch` left/right pinned the capped frame to the left
//! edge instead. The vertical margins stay `0ch`. This is the intended browser
//! centering behavior of the CSS box cutover and mirrors the terminal frame's
//! left/right placement (authored side margins are emitted verbatim and
//! suppress the `auto` centering — see `layout/page.rs` `center_frame` and the
//! `browser_render_with_max_width` /
//! `browser_render_authored_side_margins_suppress_centering` unit tests). The
//! change is therefore **intended**, not regressive, and is accepted here.
//!
//! ## `Width::Auto` fills available width (accepted)
//!
//! Two terminal snapshots re-baselined from content-hugging to width-filling
//! tables: `ref_centered_table_terminal` (fills the full 80-column width) and
//! `ref_table_max_width_terminal` (fills to the 60ch cap). This mirrors the
//! repo-wide `Width::Auto` fill change (`feat(biscuit-terminal): make
//! Width::Auto fill the available width`) that regenerated the
//! `layout_matrix__Table__*` snapshots; the last column absorbs the slack. A
//! centered table that already fills its container shows no visible offset —
//! alignment only shifts a table narrower than the available width. The
//! matching browser snapshots were unaffected (already re-baselined above).

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, HrStyleOverrides, ListStyleOverrides,
    PageStyleOverrides, apply_bespoke_style, apply_color_style, apply_component_style,
    apply_hr_style, apply_list_style, apply_page_style, from_frontmatter,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const TABLE_FIXTURE: &str = "| A | B |\n|---|---|\n| 1 | 2 |\n";

const BLOCK_QUOTE_FIXTURE: &str = "> A quoted paragraph.\n";

const LIST_FIXTURE: &str = "- Item one\n- Item two\n";

const SIMPLE_DOC: &str = "# Hello\n\nSome prose.\n";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a Markdown document whose frontmatter carries the given `style:` block.
fn with_style(style_yaml: &str, content: &str) -> Markdown {
    let full = format!("---\nstyle:\n{}---\n\n{}", indent(style_yaml, 4), content);
    Markdown::try_from_content(&full).expect("parse markdown with style frontmatter")
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Apply the full `style:` pipeline (page → component → list → hr → color →
/// bespoke) with no CLI overrides.
fn apply_all_styles(page: DarkmatterPage, md: &Markdown) -> DarkmatterPage {
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page = apply_page_style(page, &style, PageStyleOverrides::default())
        .expect("apply_page_style");
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default())
        .expect("apply_component_style");
    let page = apply_list_style(page, &style, ListStyleOverrides::default())
        .expect("apply_list_style");
    let page = apply_hr_style(page, &style, HrStyleOverrides::default())
        .expect("apply_hr_style");
    let page = apply_color_style(page, &style).expect("apply_color_style");
    apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
        .expect("apply_bespoke_style")
}

/// Render `md` to terminal through a `DarkmatterPage` at the given width.
fn render_terminal(md: &Markdown, width: u32) -> String {
    let term = Terminal::new_optimistic(width);
    let page = DarkmatterPage::new(&term);
    let page = apply_all_styles(page, md);
    page.render(md).expect("terminal render")
}

/// Render `md` to a complete standalone browser document through a
/// `DarkmatterPage`. These parity references characterize the full-page render
/// (design-token `:root` head, page wrapper CSS), so they use
/// `render_to_browser_document` — the content-independent full-document method —
/// rather than the body-only `render_to_browser` fragment.
fn render_browser(md: &Markdown) -> String {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term);
    let page = apply_all_styles(page, md);
    page.render_to_browser_document(md).expect("browser render")
}

// ---------------------------------------------------------------------------
// Snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn reference_centered_table() {
    let md = with_style("table:\n  alignment: center\n", TABLE_FIXTURE);
    insta::assert_snapshot!("ref_centered_table_terminal", render_terminal(&md, 80));
    insta::assert_snapshot!("ref_centered_table_browser", render_browser(&md));
}

#[test]
fn reference_table_max_width() {
    let md = with_style("table:\n  max-width: 60ch\n", TABLE_FIXTURE);
    insta::assert_snapshot!("ref_table_max_width_terminal", render_terminal(&md, 80));
    insta::assert_snapshot!("ref_table_max_width_browser", render_browser(&md));
}

#[test]
fn reference_block_quote_width_and_left() {
    let md = with_style(
        "block-quote:\n  max-width: 60ch\n  alignment: left\n",
        BLOCK_QUOTE_FIXTURE,
    );
    insta::assert_snapshot!(
        "ref_block_quote_width_and_left_terminal",
        render_terminal(&md, 80)
    );
    insta::assert_snapshot!(
        "ref_block_quote_width_and_left_browser",
        render_browser(&md)
    );
}

#[test]
fn reference_list_left_margin() {
    let md = with_style("ul:\n  left-margin: 4ch\n", LIST_FIXTURE);
    insta::assert_snapshot!("ref_list_left_margin_terminal", render_terminal(&md, 80));
    insta::assert_snapshot!("ref_list_left_margin_browser", render_browser(&md));
}

#[test]
fn reference_page_background_pronounced() {
    let md = with_style("page:\n  background: pronounced\n", SIMPLE_DOC);
    insta::assert_snapshot!(
        "ref_page_background_pronounced_terminal",
        render_terminal(&md, 80)
    );
    insta::assert_snapshot!(
        "ref_page_background_pronounced_browser",
        render_browser(&md)
    );
}

#[test]
fn reference_page_margin_and_padding() {
    let md = with_style(
        "page:\n  left-margin: 2\n  right-margin: 2\n  top-padding: 1\n  bottom-padding: 1\n",
        SIMPLE_DOC,
    );
    insta::assert_snapshot!(
        "ref_page_margin_and_padding_terminal",
        render_terminal(&md, 80)
    );
    insta::assert_snapshot!(
        "ref_page_margin_and_padding_browser",
        render_browser(&md)
    );
}

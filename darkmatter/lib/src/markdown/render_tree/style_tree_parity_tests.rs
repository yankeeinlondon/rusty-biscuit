//! Independent render-tree parity for `style:` frontmatter (spec verification
//! item 6 / acceptance criterion 5,
//! `renderable/features/2026-06-30-style-everywhere/spec.md`).
//!
//! The sibling integration suite `tests/style_frontmatter_parity.rs` proves the
//! frontmatter *parser + policy* is correct: it renders a `from_frontmatter`
//! [`DarkmatterPage`] and a hand-built [`ComponentPolicy`]
//! [`DarkmatterPage`] and asserts the two match. Both sides funnel through the
//! same [`apply_node_policy`](super::build_context::apply_node_policy) lowering,
//! so a bug there is invisible to that suite.
//!
//! This module closes the residual gap flagged in review-1 finding 4: it proves
//! that a document carrying `style:` frontmatter lowers, through the *full*
//! chain (`from_frontmatter` → `apply_*_style` → [`ComponentPolicy`] →
//! [`apply_node_policy`](super::build_context::apply_node_policy)), onto a
//! [`renderable`] tree that renders **identically to an equivalent hand-built
//! `renderable` tree** — the same [`Layout`]/[`Style`] applied directly to the
//! folded [`RenderNode`], bypassing darkmatter's policy machinery entirely.
//!
//! Both trees are rendered through the *same* shared fold
//! ([`render_page_terminal_document`], [`render_browser_document_html`]), so the
//! only variable is how the node's attributes were populated: darkmatter policy
//! (system under test) versus hand-written [`renderable`] values (the oracle).
//! Parity is **semantic / within-target** (the property took effect the same
//! way), never cross-target byte equality — a spec non-goal.
//!
//! ## Coverage
//!
//! Every `Layout`/`Style` property *type* in `matrix.md` is proven against the
//! hand-built oracle: `margin`, `padding`, `width` (+ `Fixed`/`FitContent`
//! modes), `max_width`, `alignment`, `color`, `background`, `emphasis`,
//! `border`, `word_wrap`. `Table` carries the widest surface and isolates each
//! property; `BlockQuote` / `CodeBlock` / `Ul` / `Ol` / `Disclosure` / page
//! color exercise the remaining component lowerings.
//!
//! `Li` (typed `text_layout` hints), `Hr` (typed `thematic_break` attrs), and
//! `Images` / `Hyperlinks` (structured directives + `text_layout`) carry
//! bespoke typed lowerings beyond plain `attrs.layout` / `attrs.style` and add
//! no new property *type*; they remain covered by the policy-parity integration
//! suite.

use renderable::layout::{Alignment, Edges, Layout, Length, TargetValue, Width};
use renderable::style::{Border, BorderSides, PaintColor, PerMode, Style, TextEmphasis, UnderlineStyle};
use renderable::tree::{BrowserRenderOptions, Document, NodeKind, RenderNode, render_browser_document_html};
use renderable::wrap_policy::WordWrap;

use biscuit_terminal::terminal::Terminal;

use super::build_context::TreeBuildContext;
use super::entrypoints::{
    render_page_terminal_document, resolve_hr_defaults, to_render_document,
    to_render_document_with_context,
};
use crate::layout::DarkmatterPage;
use crate::markdown::Markdown;
use crate::markdown::highlighting::{ColorMode, ThemePair};
use crate::markdown::output::{ColorDepth, TerminalOptions};
use crate::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, DisclosureStyleOverrides, HrStyleOverrides,
    ListStyleOverrides, PageStyleOverrides, apply_bespoke_style, apply_color_style,
    apply_component_style, apply_disclosure_style, apply_hr_style, apply_list_style,
    apply_page_style, from_frontmatter,
};

const WIDTH: u16 = 80;

// ─────────────────────────────── fixtures ──────────────────────────────────

fn with_style(style_yaml: &str, body: &str) -> Markdown {
    let full = format!("---\nstyle:\n{}---\n\n{}", indent(style_yaml, 4), body);
    Markdown::try_from_content(&full).expect("parse markdown with style frontmatter")
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Builds the `DarkmatterPage` whose per-component policies are lowered from the
/// document's `style:` frontmatter — mirroring the sibling integration suite.
fn frontmatter_page(md: &Markdown) -> DarkmatterPage {
    let term = Terminal::new_optimistic(u32::from(WIDTH));
    let page = DarkmatterPage::new(&term);
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style frontmatter");
    let page = apply_page_style(page, &style, PageStyleOverrides::default()).expect("page style");
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default())
        .expect("component style");
    let page = apply_list_style(page, &style, ListStyleOverrides::default()).expect("list style");
    let page = apply_hr_style(page, &style, HrStyleOverrides::default()).expect("hr style");
    let page = apply_disclosure_style(page, &style, DisclosureStyleOverrides::default())
        .expect("disclosure style");
    let page = apply_color_style(page, &style).expect("color style");
    apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None).expect("bespoke style")
}

fn term_opts() -> TerminalOptions {
    TerminalOptions {
        code_theme: ThemePair::OneHalf,
        prose_theme: ThemePair::OneHalf,
        color_mode: ColorMode::Dark,
        color_depth: Some(ColorDepth::TrueColor),
        ..TerminalOptions::default()
    }
}

// ─────────────────────────────── pipelines ─────────────────────────────────

/// System under test: fold the styled document through the **full** darkmatter
/// policy pipeline, baking the frontmatter policies onto the tree.
fn sut_doc(md: &Markdown, page: &DarkmatterPage) -> Document {
    let hr_defaults = resolve_hr_defaults(md, &page.hr_defaults());
    let ctx = TreeBuildContext {
        component_policies: page.component_policies(),
        page_color: page.page_color().copied(),
        page_bg_color: page.page_bg_color().copied(),
        hyperlink_style: page.hyperlink_style(),
        local_hyperlink_style: page.local_hyperlink_style(),
        local_image_style: page.local_image_style(),
        hr_defaults: hr_defaults.as_ref(),
    };
    let (doc, _diags) = to_render_document_with_context(md, &ctx).expect("fold styled document");
    doc
}

/// Folds the same document with **no** policy — the base tree both the oracle
/// and the no-op guard start from.
fn base_doc(md: &Markdown) -> Document {
    let (doc, _diags) = to_render_document(md).expect("fold base document");
    doc
}

/// Oracle: the base tree with the property applied **directly** as
/// [`renderable`] `Layout`/`Style`, exactly as a hand-built tree would carry it.
fn oracle_doc(md: &Markdown, mutate: impl FnOnce(&mut RenderNode)) -> Document {
    let mut doc = base_doc(md);
    mutate(&mut doc.root);
    doc
}

fn render_terminal(doc: &Document) -> String {
    render_page_terminal_document(doc, Vec::new(), &term_opts(), WIDTH, true)
        .expect("terminal render")
        .output
}

fn render_html(doc: &Document) -> String {
    render_browser_document_html(doc, &BrowserRenderOptions::default())
        .expect("browser render")
        .output
}

/// Asserts `style:` output matches the hand-built oracle on **both** targets,
/// and — via `md` / `page` — that the property is not a silent no-op.
fn assert_tree_parity(md: &Markdown, page: &DarkmatterPage, mutate: impl FnOnce(&mut RenderNode)) {
    let sut = sut_doc(md, page);
    let oracle = oracle_doc(md, mutate);

    let sut_terminal = render_terminal(&sut);
    assert_eq!(
        sut_terminal,
        render_terminal(&oracle),
        "terminal output must match the hand-built renderable tree"
    );

    assert_eq!(
        render_html(&sut),
        render_html(&oracle),
        "browser HTML must match the hand-built renderable tree"
    );

    // No silent no-op (spec item 7): the styled tree must diverge from the
    // unstyled baseline, so parity above cannot be a both-sides-ignored pass.
    assert_ne!(
        sut_terminal,
        render_terminal(&base_doc(md)),
        "style: frontmatter must visibly change terminal output"
    );
}

// ─────────────────────────────── helpers ───────────────────────────────────

fn paint(raw: &str) -> PaintColor {
    crate::style::color::parse(raw)
        .expect("valid style color")
        .to_paint_color()
}

fn universal_color(raw: &str) -> TargetValue<PerMode<PaintColor>> {
    TargetValue::universal(PerMode::universal(paint(raw)))
}

fn fixed_ch(ch: u32) -> Width {
    Width::Fixed(TargetValue::universal(Length::ch(ch)))
}

/// The first block child — every fixture below has exactly one.
fn block(root: &mut RenderNode) -> &mut RenderNode {
    &mut root.children_mut().expect("root has children")[0]
}

// ──────────────────────────────── Table ────────────────────────────────────

const TABLE_BODY: &str = "| Name | Value |\n|:-----|------:|\n| Alpha | 42 |\n";

#[test]
fn table_alignment_width_color_matches_hand_built_tree() {
    let style = "\
table:
  alignment: center
  width: 32ch
  color: red-500
";
    let md = with_style(style, TABLE_BODY);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let table = block(root);
        assert!(matches!(table.kind, NodeKind::Table { .. }), "fixture is a table");
        table.attrs.set_layout(&Layout {
            alignment: Alignment::Center,
            width: fixed_ch(32),
            ..Layout::default()
        });
        table.attrs.set_style(&Style {
            color: Some(universal_color("red-500")),
            ..Style::default()
        });
    });
}

#[test]
fn table_width_fit_content_matches_hand_built_tree() {
    let style = "\
table:
  width: fit-content
";
    let md = with_style(style, TABLE_BODY);
    let page = frontmatter_page(&md);
    let sut = sut_doc(&md, &page);
    let oracle = oracle_doc(&md, |root| {
        block(root).attrs.set_layout(&Layout {
            width: Width::FitContent,
            ..Layout::default()
        });
    });

    assert_eq!(
        render_terminal(&sut),
        render_terminal(&oracle),
        "terminal output must match the hand-built renderable tree"
    );
    let sut_html = render_html(&sut);
    assert_eq!(
        sut_html,
        render_html(&oracle),
        "browser HTML must match the hand-built renderable tree"
    );

    // The default `Width::Auto` hugs on the terminal exactly like
    // `FitContent`, so the no-silent-no-op check must look at the browser
    // surface, where `fit-content` is emitted as distinct CSS.
    assert_ne!(
        sut_html,
        render_html(&base_doc(&md)),
        "style: frontmatter must visibly change browser output"
    );
}

#[test]
fn table_margin_padding_max_width_matches_hand_built_tree() {
    let style = "\
table:
  margin: 1ch
  padding: 1ch
  max-width: 40ch
";
    let md = with_style(style, TABLE_BODY);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        block(root).attrs.set_layout(&Layout {
            margin: Edges::all(Length::ch(1)),
            padding: Edges::all(Length::ch(1)),
            max_width: Some(TargetValue::universal(Length::ch(40))),
            ..Layout::default()
        });
    });
}

#[test]
fn table_background_and_border_matches_hand_built_tree() {
    let style = "\
table:
  bg-color: zinc-100
  border: true
";
    let md = with_style(style, TABLE_BODY);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        block(root).attrs.set_style(&Style {
            background: Some(universal_color("zinc-100")),
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        });
    });
}

// `word_wrap` is a distinct property type but is invisible on a full-width
// table (the cell wraps identically with or without `truncate`), so the no-op
// guard cannot fire in isolation there. Its tree lowering is proven against the
// hand-built oracle by the `ul` (`truncate`) and `code-block` (`wrap`) cases.

// ────────────────────────────── BlockQuote ─────────────────────────────────

#[test]
fn block_quote_full_surface_matches_hand_built_tree() {
    let body = "> A quoted paragraph with enough words to exercise the styled block quote path.\n";
    let style = "\
block-quote:
  margin: 1ch
  padding: 1ch
  border: true
  emphasis:
    italic: true
    underline: straight
  bg-color: zinc-100
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let bq = block(root);
        assert!(matches!(bq.kind, NodeKind::BlockQuote { .. }), "fixture is a block quote");
        bq.attrs.set_layout(&Layout {
            margin: Edges::all(Length::ch(1)),
            padding: Edges::all(Length::ch(1)),
            ..Layout::default()
        });
        bq.attrs.set_style(&Style {
            background: Some(universal_color("zinc-100")),
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            emphasis: TextEmphasis {
                italic: true,
                underline: Some(UnderlineStyle::Straight),
                ..TextEmphasis::default()
            },
            ..Style::default()
        });
    });
}

// ────────────────────────────── CodeBlock ──────────────────────────────────

#[test]
fn code_block_width_padding_background_wrap_matches_hand_built_tree() {
    let body = "```rust\nfn main() { let _ = 1; }\n```\n";
    let style = "\
code-block:
  width: 28ch
  padding: 1ch
  bg-color: slate-900
  word-wrap: wrap
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let code = block(root);
        assert!(matches!(code.kind, NodeKind::Code { .. }), "fixture is a code block");
        code.attrs.set_layout(&Layout {
            width: fixed_ch(28),
            padding: Edges::all(Length::ch(1)),
            word_wrap: WordWrap::WrapProse(None, None),
            ..Layout::default()
        });
        code.attrs.set_style(&Style {
            background: Some(universal_color("slate-900")),
            ..Style::default()
        });
    });
}

// ───────────────────────────────── Lists ───────────────────────────────────

#[test]
fn unordered_list_margin_max_width_wrap_matches_hand_built_tree() {
    let body = "- A long unordered list item whose text is intentionally wide enough to trigger list wrapping policy.\n- Short item.\n";
    let style = "\
ul:
  left-margin: 4ch
  max-width: 24ch
  word-wrap: truncate
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let list = block(root);
        assert!(
            matches!(list.kind, NodeKind::List { ordered: false, .. }),
            "fixture is an unordered list"
        );
        let margin = Edges {
            left: TargetValue::universal(Length::ch(4)),
            ..Edges::default()
        };
        list.attrs.set_layout(&Layout {
            margin,
            max_width: Some(TargetValue::universal(Length::ch(24))),
            word_wrap: WordWrap::Truncate(None),
            ..Layout::default()
        });
    });
}

#[test]
fn ordered_list_width_alignment_padding_color_matches_hand_built_tree() {
    let body = "1. First ordered item with enough words to exercise the ordered list width policy.\n2. Second item.\n";
    let style = "\
ol:
  width: 42ch
  alignment: center
  padding: 1ch
  color: green-600
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let list = block(root);
        assert!(
            matches!(list.kind, NodeKind::List { ordered: true, .. }),
            "fixture is an ordered list"
        );
        list.attrs.set_layout(&Layout {
            width: fixed_ch(42),
            alignment: Alignment::Center,
            padding: Edges::all(Length::ch(1)),
            ..Layout::default()
        });
        list.attrs.set_style(&Style {
            color: Some(universal_color("green-600")),
            ..Style::default()
        });
    });
}

// ──────────────────────────────── Disclosure ───────────────────────────────

#[test]
fn disclosure_full_surface_matches_hand_built_tree() {
    let body = "::disclosure\nSummary\n::details\nHidden body text.\n::end-disclosure\n";
    let style = "\
disclosure:
  width: 34ch
  alignment: right
  padding: 1ch
  border: true
  color: cyan-700
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        let disclosure = block(root);
        assert!(
            matches!(disclosure.kind, NodeKind::Disclosure { .. }),
            "fixture is a disclosure"
        );
        disclosure.attrs.set_layout(&Layout {
            width: fixed_ch(34),
            alignment: Alignment::Right,
            padding: Edges::all(Length::ch(1)),
            ..Layout::default()
        });
        disclosure.attrs.set_style(&Style {
            color: Some(universal_color("cyan-700")),
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        });
    });
}

// ─────────────────────────────────── Page ──────────────────────────────────

#[test]
fn page_color_matches_hand_built_tree() {
    // Page background / `background:` are page-frame concerns applied after the
    // shared fold (row decoration / browser wrapper), so only the page
    // foreground color lands on the tree root; that is what this oracle pins.
    let body = "# Page\n\nPage-level foreground color inherits to descendant text.\n";
    let style = "\
page:
  color: slate-900
";
    let md = with_style(style, body);
    let page = frontmatter_page(&md);
    assert_tree_parity(&md, &page, |root| {
        root.attrs.set_style(&Style {
            color: Some(universal_color("slate-900")),
            ..Style::default()
        });
    });
}

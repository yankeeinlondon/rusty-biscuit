//! Each container accepts the Prose shape its content model calls for:
//! table cells and `InlineContent` take `InlineProse` (phrasing), while
//! lists, block quotes, `TwoColumn` columns, `StatusBlock` bodies, `Section`,
//! and `Compose` embed a `Prose` as its own blocks.
//!
//! Every assertion goes through a public result: the container's render
//! tree, or its terminal, Markdown, and browser output.

use std::rc::Rc;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::inline_content::InlineContent;
use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose};
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::components::table::TableColumn;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    BrowserRenderOptions, MarkdownRenderOptions, NodeKind, RenderNode, RenderStrictness, TreeRenderable,
    ValidationMode, render_browser_node, render_markdown_node, validate,
};

use crate::prose_grammar::{CODE_SYNTAXES, assert_split_around_code, wrappers};

// ── helpers ─────────────────────────────────────────────────────────────

fn plain_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(false)
        .width(80)
        .build()
}

fn styled_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(false)
        .width(80)
        .build()
}

fn strip(s: &str) -> String {
    biscuit_terminal::utils::escape_codes::strip_escape_codes(s.to_string())
}

fn all_nodes(node: &RenderNode) -> Vec<&RenderNode> {
    let mut out = vec![node];
    for child in node.children() {
        out.extend(all_nodes(child));
    }
    out
}

/// Number of `Root` nodes anywhere below the top of the tree.
fn nested_roots(tree: &RenderNode) -> usize {
    all_nodes(tree)
        .into_iter()
        .skip(1)
        .filter(|n| matches!(n.kind, NodeKind::Root { .. }))
        .count()
}

/// The text of every `Paragraph` in the tree, in order, with inline
/// structure flattened.
fn paragraph_texts(tree: &RenderNode) -> Vec<String> {
    fn text(node: &RenderNode) -> String {
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => value.clone(),
            _ => node.children().iter().map(text).collect(),
        }
    }
    all_nodes(tree)
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Paragraph { .. }))
        .map(text)
        .filter(|t| !t.is_empty())
        .collect()
}

fn assert_valid(name: &str, tree: &RenderNode) {
    let report = validate(tree, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "{name}: tree must validate: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

fn ch(n: u32) -> TargetValue<Length> {
    TargetValue::universal(Length::ch(n))
}

fn two_paragraphs() -> Prose {
    Prose::new("<b>first</b> paragraph\n\nsecond `code`")
}

/// Every block container, holding `prose` as its content, projected to a
/// render tree.
fn block_containers(prose: &Prose) -> Vec<(&'static str, RenderNode)> {
    let item = || RenderableTerminalContent::Component(Rc::new(prose.clone()));
    let mut section = Section::new(HeadingLevel::h2, "Title");
    section.push(prose.clone());
    let mut compose = Compose::default();
    compose.add_prose(prose.clone());
    vec![
        ("UnorderedList", UnorderedList::from(vec![item()]).render_tree()),
        ("OrderedList", OrderedList::from(vec![item()]).render_tree()),
        ("BlockQuote", BlockQuote::from(prose.clone()).render_tree()),
        ("TwoColumn", TwoColumn::new(prose.clone(), "right").render_tree()),
        (
            "StatusBlock",
            StatusBlock::new(StatusState::Info).body(prose.clone()).render_tree(),
        ),
        ("Section", section.render_tree()),
        ("Compose", compose.render_tree()),
    ]
}

// ── block containers embed Prose as blocks (AC 12) ──────────────────────

/// A `Prose` with two paragraphs stays two paragraphs inside every block
/// container: no single folded paragraph holding a literal blank line, and
/// its inline structure (bold, inline code) is kept rather than flattened.
#[test]
fn block_containers_embed_each_prose_paragraph_as_its_own_block() {
    for (name, tree) in block_containers(&two_paragraphs()) {
        assert_valid(name, &tree);
        let texts = paragraph_texts(&tree);
        assert!(
            texts.iter().any(|t| t == "first paragraph")
                && texts.iter().any(|t| t == "second code"),
            "{name}: each Prose paragraph is its own Paragraph: {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.contains("first paragraph") && t.contains("second")),
            "{name}: paragraphs must not be folded into one: {texts:?}"
        );
        let nodes = all_nodes(&tree);
        assert!(
            nodes.iter().any(|n| matches!(n.kind, NodeKind::Strong { .. })),
            "{name}: bold survives as Strong"
        );
        assert!(
            nodes
                .iter()
                .any(|n| matches!(&n.kind, NodeKind::InlineCode { value } if value == "code")),
            "{name}: code span survives as InlineCode"
        );
        assert_eq!(nested_roots(&tree), 0, "{name}: no nested Root");
    }
}

/// A fenced block inside an embedded `Prose` stays a block-level `Code`
/// sibling of the paragraphs in every block container.
#[test]
fn block_containers_keep_fenced_code_as_a_sibling_block() {
    let prose = Prose::new("before\n\n```rust\nfn main() {}\n```\n\nafter");
    for (name, tree) in block_containers(&prose) {
        assert_valid(name, &tree);
        let paragraph_holds_code = all_nodes(&tree).iter().any(|n| {
            matches!(n.kind, NodeKind::Paragraph { .. })
                && n.children().iter().any(|c| matches!(c.kind, NodeKind::Code { .. }))
        });
        assert!(!paragraph_holds_code, "{name}: Code never nests in a Paragraph");
        assert!(
            all_nodes(&tree).iter().any(|n| matches!(
                &n.kind,
                NodeKind::Code { lang: Some(lang), value, .. } if lang == "rust" && value.contains("fn main()")
            )),
            "{name}: fenced block is a Code node"
        );
    }
}

/// Every wrapper branch around explicit or fenced block code, embedded in
/// every block container: the tree validates, the code is a sibling block
/// with the wrapper (style, link destination) resumed on both sides, and
/// every target renders the content.
#[test]
fn block_containers_split_every_wrapper_around_block_code() {
    let term = plain_term();
    for wrapper in wrappers() {
        for code in &CODE_SYNTAXES {
            let prose = Prose::new(wrapper.wrap(code.source));
            for (name, tree) in block_containers(&prose) {
                let context = format!("{name}: {} around {} code", wrapper.name, code.name);
                assert_valid(&context, &tree);
                let parent = all_nodes(&tree)
                    .into_iter()
                    .find(|n| n.children().iter().any(|c| matches!(c.kind, NodeKind::Code { .. })))
                    .unwrap_or_else(|| panic!("{context}: a Code block is embedded"));
                // Compose separates one Prose's blocks with a blank-line `Text`.
                let siblings: Vec<RenderNode> = parent
                    .children()
                    .iter()
                    .filter(|c| !matches!(&c.kind, NodeKind::Text { value } if value.trim().is_empty()))
                    .cloned()
                    .collect();
                let at = siblings
                    .iter()
                    .position(|c| matches!(c.kind, NodeKind::Code { .. }))
                    .expect("found above");
                assert!(at > 0, "{context}: a paragraph precedes the code");
                assert_split_around_code(&context, &wrapper, code, &siblings[at - 1..=at + 1]);

                let html = render_browser_node(&tree, &BrowserRenderOptions::default())
                    .unwrap_or_else(|e| panic!("{context}: {e}"))
                    .output
                    .render();
                assert!(html.contains("<code>x</code>"), "{context}: {html}");
                let markdown = render_markdown_node(&tree, &MarkdownRenderOptions::default())
                    .unwrap_or_else(|e| panic!("{context}: {e}"))
                    .output;
                assert!(markdown.contains("x\n"), "{context}: {markdown:?}");
                let terminal = render_terminal_node(&tree, &TerminalRenderOptions::new(&term, RenderStrictness::Warn))
                    .unwrap_or_else(|e| panic!("{context}: {e}"))
                    .output;
                assert!(strip(&terminal).contains('x'), "{context}: {terminal:?}");
            }
        }
    }
}

/// Paragraph separation survives every target in the block containers that
/// render blocks with blank-line spacing, and in `Compose`, whose sequence
/// has no separator of its own.
#[test]
fn embedded_paragraphs_stay_apart_on_every_target() {
    let prose = Prose::new("one\n\ntwo");

    let quote = BlockQuote::from(prose.clone());
    assert_eq!(strip(&quote.render(&plain_term())), "│ one\n│ \n│ two");
    assert_eq!(quote.render_markdown().trim_end(), "> one\n>\n> two");
    assert!(quote.render_html_fragment().render().contains("<p>one</p><p>two</p>"));

    let mut compose = Compose::default();
    compose.add_text("A: ").add_prose(prose.clone()).add_text(" :B");
    assert_eq!(strip(&compose.render(&plain_term())), "A: one\n\ntwo :B");
    assert_eq!(compose.render_markdown().trim_end(), "A: one\n\ntwo :B");

    let list = UnorderedList::from(vec![RenderableTerminalContent::Component(Rc::new(prose))]);
    let markdown = list.render_markdown();
    assert!(markdown.starts_with("- one\n\n  two"), "{markdown:?}");
}

// ── layout transfer (AC 31) ─────────────────────────────────────────────

/// An embedded single-paragraph `Prose` keeps its layout exactly once, on
/// its own paragraph: never on a nested `Root`, never dropped, and never
/// copied onto the container's node.
#[test]
fn embedded_prose_layout_appears_exactly_once_on_its_paragraph() {
    let prose = Prose::new("indented").with_left_margin(ch(4));
    for (name, tree) in block_containers(&prose) {
        assert_valid(name, &tree);
        assert_eq!(nested_roots(&tree), 0, "{name}: no nested Root");
        let carriers: Vec<&RenderNode> = all_nodes(&tree)
            .into_iter()
            .filter(|n| n.attrs.layout().is_some_and(|l| l.margin.left == ch(4)))
            .collect();
        assert_eq!(carriers.len(), 1, "{name}: layout appears exactly once");
        assert!(
            matches!(carriers[0].kind, NodeKind::Paragraph { .. }),
            "{name}: the Prose paragraph carries it, got {:?}",
            carriers[0].kind
        );
    }
}

/// A multi-block `Prose` with a layout spreads one box over its blocks:
/// each block takes the horizontal box, the top edge stays on the first
/// block, and the bottom edge on the last.
#[test]
fn embedded_multi_block_layout_keeps_one_outer_box() {
    let mut prose = Prose::new("one\n\n```\ncode\n```\n\nthree").with_left_margin(ch(4));
    prose.layout_mut().margin.top = TargetValue::universal(Length::ch(1));
    prose.layout_mut().margin.bottom = TargetValue::universal(Length::ch(2));
    let tree = BlockQuote::from(prose).render_tree();
    assert_valid("BlockQuote", &tree);
    let NodeKind::BlockQuote { children } = &tree.kind else {
        panic!("expected BlockQuote, got {:?}", tree.kind);
    };
    let layouts: Vec<_> = children
        .iter()
        .map(|c| c.attrs.layout().expect("every Prose block carries the box"))
        .collect();
    assert_eq!(layouts.len(), 3);
    let zero = TargetValue::universal(Length::Zero);
    for layout in &layouts {
        assert_eq!(layout.margin.left, ch(4));
    }
    assert_eq!(layouts[0].margin.top, TargetValue::universal(Length::ch(1)));
    assert_eq!(layouts[1].margin.top, zero);
    assert_eq!(layouts[2].margin.top, zero);
    assert_eq!(layouts[0].margin.bottom, zero);
    assert_eq!(layouts[1].margin.bottom, zero);
    assert_eq!(layouts[2].margin.bottom, TargetValue::universal(Length::ch(2)));
}

/// The transferred margin renders inside the quote border on the terminal
/// and as CSS on the paragraph in HTML, once.
#[test]
fn embedded_prose_margin_renders_inside_the_container() {
    let quote = BlockQuote::from(Prose::new("indented").with_left_margin(ch(4)));
    assert_eq!(strip(&quote.render(&plain_term())), "│     indented");
    let html = quote.render_html_fragment().render();
    assert_eq!(html.matches("margin-left:4ch").count(), 1, "{html}");
    let paragraph = &html[html.find("<p style=\"").expect("styled paragraph")..];
    let paragraph = &paragraph[..paragraph.find(">indented</p>").expect("paragraph text")];
    assert!(
        paragraph.contains("margin-left:4ch"),
        "the margin is on the paragraph, not the quote: {html}"
    );
}

// ── inline containers take InlineProse (AC 12, 28) ──────────────────────

#[test]
fn inline_content_takes_inline_prose() {
    let mut inline = InlineContent::from("status: ");
    inline.add_inline_prose(InlineProse::new("<b>ready</b> via `md hash`"));
    let rendered = inline.render(&styled_term());
    assert!(rendered.contains("\u{1b}[1m"), "bold styling kept: {rendered:?}");
    assert_eq!(strip(&rendered), "status: ready via md hash");
}

#[test]
fn inline_content_fence_is_one_inline_code_value() {
    let inline =
        InlineContent::from("run ").with(InlineProse::new("```sh\nmd hash\n  --all\n```"));
    assert_eq!(
        strip(&inline.render(&plain_term())),
        "run `md hash   --all`",
        "unstyled terminal shows one fenced code value on one line"
    );
}

/// Inline containers take `InlineProse`, where block code is inline code
/// inside the wrapper: every wrapper and code syntax stays one line.
#[test]
fn inline_content_keeps_block_code_inside_every_wrapper() {
    for wrapper in wrappers() {
        for code in &CODE_SYNTAXES {
            let context = format!("{} around {} code", wrapper.name, code.name);
            let prose = InlineProse::new(wrapper.wrap(code.source));
            assert_valid(&context, &prose.render_tree());
            let inline = InlineContent::from("run ").with(prose);
            let rendered = strip(&inline.render(&plain_term()));
            assert!(rendered.starts_with("run ") && rendered.contains("`x`"), "{context}: {rendered:?}");
            assert!(!rendered.contains('\n'), "{context}: {rendered:?}");
        }
    }
}

#[test]
fn table_header_label_is_inline_prose() {
    let column = TableColumn::new_with_bold("Name");
    let label: InlineProse = column.header_prose.expect("bold header label");
    assert_eq!(label.content(), "<bold>Name</bold>");
}

/// A `TwoColumn` column is a block region: its `Prose` keeps inline
/// structure in the tree instead of falling back to ANSI-stripped text.
#[test]
fn two_column_prose_column_projects_structurally() {
    let tree = TwoColumn::new(Prose::new("<b>left</b> side"), InlineProse::new("<i>right</i>"))
        .render_tree();
    assert_valid("TwoColumn", &tree);
    let nodes = all_nodes(&tree);
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::Strong { .. })));
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::Emphasis { .. })));
    assert_eq!(paragraph_texts(&tree), ["left side", "right"]);
}

#[test]
fn list_items_keep_the_hard_breaks_of_their_first_paragraph() {
    for term in [plain_term(), styled_term()] {
        let item = || Prose::new("Head\n  - Details: one").with_line_breaks(LineBreaks::Hard);
        let mut unordered = UnorderedList::empty();
        unordered.add(item()).add(item());
        let mut ordered = OrderedList::empty();
        ordered.add(item()).add(item());

        for (name, rendered) in [
            ("unordered", strip(&unordered.render(&term))),
            ("ordered", strip(&ordered.render(&term))),
        ] {
            let rows: Vec<&str> = rendered.lines().filter(|row| !row.trim().is_empty()).collect();
            assert_eq!(rows.len(), 4, "{name}: each item is two rows: {rendered:?}");
            for pair in rows.chunks(2) {
                assert!(pair[0].trim_end().ends_with("Head"), "{name}: {rendered:?}");
                assert_eq!(pair[1].trim(), "- Details: one", "{name}: {rendered:?}");
            }
        }
    }

    // Control: a soft break still joins the item into one row.
    let mut soft = UnorderedList::empty();
    soft.add(Prose::new("Head\nmore"));
    let rendered = strip(&soft.render(&plain_term()));
    assert_eq!(rendered.lines().filter(|row| !row.trim().is_empty()).count(), 1, "{rendered:?}");
    assert!(rendered.contains("Head more"), "{rendered:?}");
}

#[test]
fn a_hard_mode_status_puts_each_line_on_a_row_of_its_own() {
    use biscuit_terminal::components::status::Status;

    for term in [plain_term(), styled_term()] {
        let rendered = strip(
            &Status::from_prose("captured=[<b>line one</b>\n  line two]")
                .with_line_breaks(LineBreaks::Hard)
                .state(StatusState::Info)
                .render(&term),
        );
        let rows: Vec<&str> = rendered.lines().collect();
        assert_eq!(rows.len(), 2, "{rendered:?}");
        assert!(rows[0].ends_with("captured=[line one"), "{rendered:?}");
        assert_eq!(rows[1], "  line two]", "{rendered:?}");
    }

    // Control: the default soft mode joins the lines.
    let rendered = strip(&Status::from_prose("a\nb").render(&plain_term()));
    assert_eq!(rendered.lines().count(), 1, "{rendered:?}");
    assert!(rendered.ends_with("a b"), "{rendered:?}");
}

//! The shared Prose grammar through the public `Prose` and `InlineProse`
//! results: paragraphs and line breaks, code spans as inline code, fenced
//! code, block tags, layout, and the terminal inline-code fallback.
//!
//! Each table walks one rule across the three targets (Markdown, browser
//! HTML, terminal) and asserts the rendered output, not parser internals.

use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose, ProseTag};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue, WordWrap};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{
    NodeKind, RenderNode, RenderStrictness, TreeRenderable, ValidationMode, validate,
};

// ── helpers ─────────────────────────────────────────────────────────────

fn md(prose: &Prose) -> String {
    prose.render_markdown()
}

fn html(prose: &Prose) -> String {
    prose.render_html_fragment().render()
}

/// Terminal output with every SGR stripped (OSC 8 kept out by the plain
/// terminal), for structural assertions.
fn plain_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(false)
        .width(80)
        .build()
}

fn styled_term(osc8: bool) -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(osc8)
        .width(80)
        .build()
}

fn strip(s: &str) -> String {
    biscuit_terminal::utils::escape_codes::strip_escape_codes(s.to_string())
}

fn term_text(prose: &Prose) -> String {
    strip(&prose.render(&styled_term(false)))
}

fn paragraphs(prose: &Prose) -> usize {
    prose
        .render_tree()
        .children()
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Paragraph { .. }))
        .count()
}

fn assert_valid(node: &RenderNode) {
    let report = validate(node, ValidationMode::Full);
    assert!(!report.has_errors(), "{:?}", report.errors().collect::<Vec<_>>());
}

fn inline_kinds(nodes: &[RenderNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match &n.kind {
            NodeKind::Text { value } => format!("Text({value})"),
            NodeKind::InlineCode { value } => format!("Code({value})"),
            NodeKind::SoftBreak => "Soft".into(),
            NodeKind::HardBreak => "Hard".into(),
            NodeKind::Link { url, children, .. } => {
                format!("Link({url})[{}]", inline_kinds(children).join(" "))
            }
            NodeKind::Span { children, .. } => format!("Span[{}]", inline_kinds(children).join(" ")),
            other => format!("{other:?}"),
        })
        .collect()
}

// ── Paragraphs and line breaks (AC 2–7, 25, 26) ─────────────────────────

#[test]
fn single_newline_is_a_soft_break_on_every_target() {
    let prose = Prose::new("a\nb");
    assert_eq!(paragraphs(&prose), 1);
    assert_eq!(md(&prose), "a\nb");
    assert_eq!(html(&prose), "<p>a b</p>");
    assert_eq!(term_text(&prose), "a b");
}

#[test]
fn blank_line_runs_are_paragraph_boundaries_on_every_target() {
    for input in ["a\n\nb", "a\n\n\nb", "a\n  \nb", "a\n\t\n\n b"] {
        let prose = Prose::new(input);
        assert_eq!(paragraphs(&prose), 2, "{input:?}");
        assert_eq!(html(&prose).matches("<p>").count(), 2, "{input:?}");
        assert!(md(&prose).starts_with("a\n\n"), "{input:?}: {:?}", md(&prose));
        assert!(term_text(&prose).starts_with("a\n\n"), "{input:?}: {:?}", term_text(&prose));
    }
}

#[test]
fn backslash_newline_is_a_hard_break_on_every_target() {
    let prose = Prose::new("a\\\nb");
    assert_eq!(paragraphs(&prose), 1);
    assert_eq!(md(&prose), "a\\\nb");
    assert_eq!(html(&prose), "<p>a<br>b</p>");
    assert_eq!(term_text(&prose), "a\nb");
}

#[test]
fn hard_mode_turns_single_newlines_into_hard_breaks() {
    let prose = Prose::new("a\nb").with_line_breaks(LineBreaks::Hard);
    assert_eq!(md(&prose), "a\\\nb");
    assert_eq!(html(&prose), "<p>a<br>b</p>");
    assert_eq!(term_text(&prose), "a\nb");

    let inline = InlineProse::new("a\nb").with_line_breaks(LineBreaks::Hard);
    assert_eq!(inline.render_markdown(), "a\\\nb");
    assert_eq!(inline.render_html_fragment().render(), "a<br>b");

    let two = Prose::new("a\n\nb").with_line_breaks(LineBreaks::Hard);
    assert_eq!(paragraphs(&two), 2);
}

#[test]
fn trailing_spaces_are_not_a_hard_break() {
    let prose = Prose::new("first  \nsecond");
    assert_eq!(md(&prose), "first\nsecond");
    assert_eq!(html(&prose), "<p>first second</p>");
    assert_eq!(term_text(&prose), "first second");
    assert!(!md(&prose).contains("  \n"));
}

#[test]
fn spaces_and_tabs_around_a_soft_break_are_discarded() {
    assert_eq!(term_text(&Prose::new("a \t\n\t  b")), "a b");
    // Elsewhere whitespace is kept.
    assert_eq!(term_text(&Prose::new("a  b")), "a  b");
}

#[test]
fn inline_prose_blank_lines_are_one_soft_break() {
    let inline = InlineProse::new("first\n\nsecond");
    assert_eq!(strip(&inline.render(&styled_term(false))), "first second");
    assert_eq!(inline.render_html_fragment().render(), "first second");
    assert_eq!(inline.render_markdown(), "first\nsecond");
}

#[test]
fn inline_prose_hard_mode_breaks_on_every_newline_of_a_blank_run() {
    let nodes = InlineProse::new("a\n\nb").with_line_breaks(LineBreaks::Hard).to_render_nodes();
    assert_eq!(inline_kinds(&nodes), ["Text(a)", "Hard", "Hard", "Text(b)"]);
}

#[test]
fn escaped_backslash_before_newline_is_a_literal_backslash_and_the_mode_break() {
    let soft = Prose::new("a\\\\\nb");
    assert_eq!(html(&soft), "<p>a\\ b</p>");
    let hard = Prose::new("a\\\\\nb").with_line_breaks(LineBreaks::Hard);
    assert_eq!(html(&hard), "<p>a\\<br>b</p>");
}

#[test]
fn backslash_at_end_or_before_a_boundary_is_literal() {
    assert_eq!(html(&Prose::new("a\\")), "<p>a\\</p>");
    assert_eq!(html(&Prose::new("a\\\n\nb")), "<p>a\\</p><p>b</p>");
    let inline = InlineProse::new("a\\\n\nb").to_render_nodes();
    assert_eq!(inline_kinds(&inline), ["Text(a\\)", "Soft", "Text(b)"]);
    // `"a\\nb"` is a backslash and the letter n, not a break.
    assert_eq!(html(&Prose::new("a\\nb")), "<p>a\\nb</p>");
}

#[test]
fn crlf_and_lone_cr_match_lf_structure() {
    for (lf, variants) in [
        ("a\nb", ["a\r\nb", "a\rb"]),
        ("a\n\nb", ["a\r\n\r\nb", "a\r\rb"]),
        ("a\\\nb", ["a\\\r\nb", "a\\\rb"]),
        ("a\\\\\nb", ["a\\\\\r\nb", "a\\\\\rb"]),
        ("a\\\n\nb", ["a\\\r\n\r\nb", "a\\\r\rb"]),
    ] {
        for variant in variants {
            assert_eq!(
                Prose::new(variant).render_tree(),
                Prose::new(lf).render_tree(),
                "{variant:?} vs {lf:?}"
            );
        }
    }
}

#[test]
fn opaque_code_is_unaffected_by_break_mode() {
    let input = "`x\ny`\n```\nl1\nl2\n```";
    for mode in [LineBreaks::Soft, LineBreaks::Hard] {
        let prose = Prose::new(input).with_line_breaks(mode);
        let out = html(&prose);
        assert!(out.contains("<code>x y</code>"), "{mode:?}: {out}");
        assert!(out.contains("l1\nl2"), "{mode:?}: {out}");
    }
}

#[test]
fn empty_and_whitespace_input_yield_a_valid_single_node() {
    for input in ["", "   ", "\n\n", " \t\n  "] {
        let tree = Prose::new(input).render_tree();
        assert!(matches!(tree.kind, NodeKind::Root { .. }));
        assert!(tree.children().is_empty(), "{input:?}");
        assert_valid(&tree);
        assert_eq!(md(&Prose::new(input)), "", "{input:?}");
        assert_eq!(html(&Prose::new(input)), "", "{input:?}");
    }
    let empty = InlineProse::new("");
    let tree = empty.render_tree();
    assert!(matches!(tree.kind, NodeKind::Span { .. }));
    assert_valid(&tree);
    assert_eq!(empty.render_markdown(), "");
    assert_eq!(empty.render_html_fragment().render(), "");
    assert_eq!(empty.render(&styled_term(false)), "");
}

#[test]
fn inline_prose_keeps_spaces_around_a_label() {
    // A reader strips a line's edge whitespace, so the outer space and tab
    // are written as character references.
    let markdown = InlineProse::new("  label\t").render_markdown();
    assert_eq!(markdown, "&#32; label&#9;");
    let read: String = pulldown_cmark::Parser::new(&markdown)
        .filter_map(|event| match event {
            pulldown_cmark::Event::Text(text) => Some(text.into_string()),
            _ => None,
        })
        .collect();
    assert_eq!(read, "  label\t");
}

#[test]
fn multi_node_inline_prose_is_one_span_and_markdown_has_no_separators() {
    let inline = InlineProse::new("a **b** `c`");
    let tree = inline.render_tree();
    assert!(matches!(tree.kind, NodeKind::Span { .. }));
    assert_eq!(tree.children().len(), 4);
    assert_valid(&tree);
    assert_eq!(inline.render_markdown(), "a **b** `c`");
}

// ── InlineProse on every target (AC 1) ──────────────────────────────────

#[test]
fn inline_prose_renders_phrasing_on_every_target() {
    let label = InlineProse::new("Run `md hash` on [the plan](plan.md)");
    assert_eq!(label.render_markdown(), "Run `md hash` on [the plan](plan.md)");
    assert_eq!(
        label.render_html_fragment().render(),
        r#"Run <code>md hash</code> on <a href="plan.md">the plan</a>"#
    );
    let term = label.render(&styled_term(true));
    assert!(!term.contains('\n'), "{term:?}");
    assert!(strip(&term).starts_with("Run md hash on the plan"), "{term:?}");
}

// ── Code spans (AC 14–18) ───────────────────────────────────────────────

#[test]
fn code_spans_project_to_inline_code_everywhere_prose_grammar_applies() {
    let cases = [
        ("See `plain_code` here", vec!["Text(See )", "Code(plain_code)", "Text( here)"]),
        (
            "See [`inline-block`](https://u.io) here",
            vec!["Text(See )", "Link(https://u.io)[Code(inline-block)]", "Text( here)"],
        ),
        ("<red>run `md hash`</red>", vec!["Span[Text(run ) Code(md hash)]"]),
    ];
    for (input, expected) in cases {
        assert_eq!(inline_kinds(&InlineProse::new(input).to_render_nodes()), expected, "{input:?}");
        let tree = Prose::new(input).render_tree();
        let NodeKind::Paragraph { children } = &tree.children()[0].kind else {
            panic!("expected a paragraph for {input:?}");
        };
        assert_eq!(inline_kinds(children), expected, "{input:?}");
    }
}

#[test]
fn code_link_example_renders_per_target() {
    let input = "See [`inline-block`](https://x.io/a) here";
    let prose = Prose::new(input);
    assert_eq!(md(&prose), input);

    let term = prose.render(&styled_term(true));
    let expected = "See \x1b]8;;https://x.io/a\x1b\\\x1b[2minline-block\x1b[0m\x1b]8;;\x1b\\ here";
    assert_eq!(term, expected);
    assert!(!term.contains('`'));

    assert_eq!(
        html(&prose),
        r#"<p>See <a href="https://x.io/a"><code>inline-block</code></a> here</p>"#
    );
}

#[test]
fn code_span_holding_a_link_is_literal_code() {
    let prose = Prose::new("`[desc](ref)`");
    let tree = prose.render_tree();
    let NodeKind::Paragraph { children } = &tree.children()[0].kind else {
        panic!("paragraph");
    };
    assert_eq!(inline_kinds(children), ["Code([desc](ref))"]);
    assert_eq!(md(&prose), "`[desc](ref)`");
    assert_eq!(html(&prose), "<p><code>[desc](ref)</code></p>");
    let term = prose.render(&styled_term(true));
    assert_eq!(term, "\x1b[2m[desc](ref)\x1b[0m");
}

#[test]
fn code_span_backslashes_are_literal_on_every_target() {
    let prose = Prose::new("`a\\_b`");
    assert_eq!(md(&prose), "`a\\_b`");
    assert_eq!(html(&prose), "<p><code>a\\_b</code></p>");
    assert_eq!(term_text(&prose), "a\\_b");
}

#[test]
fn code_span_space_handling_follows_commonmark() {
    for (input, value) in [("`` `a` ``", "`a`"), ("` a `", "a"), ("`  `", "  ")] {
        let nodes = InlineProse::new(input).to_render_nodes();
        assert_eq!(inline_kinds(&nodes), [format!("Code({value})")], "{input:?}");
    }
}

#[test]
fn code_spans_never_cross_paragraphs_and_unmatched_runs_stay_literal() {
    let prose = Prose::new("`a\n\nb`");
    assert_eq!(html(&prose), "<p>`a</p><p>b`</p>");
    assert_eq!(inline_kinds(&InlineProse::new("``a`").to_render_nodes()), ["Text(``a`)"]);
    // A span may hold a single newline, which becomes a space in either mode.
    let hard = InlineProse::new("`a\nb`").with_line_breaks(LineBreaks::Hard);
    assert_eq!(inline_kinds(&hard.to_render_nodes()), ["Code(a b)"]);
}

// ── Style scope, fences, and attributes (AC 9, 27, 28) ──────────────────

#[test]
fn bracketed_style_spanning_paragraphs_reopens_in_each() {
    let prose = Prose::new("<red>one\n\ntwo</red>");
    let tree = prose.render_tree();
    assert_valid(&tree);
    assert_eq!(tree.children().len(), 2);
    for paragraph in tree.children() {
        let NodeKind::Paragraph { children } = &paragraph.kind else { panic!("paragraph") };
        assert!(matches!(children[0].kind, NodeKind::Span { .. }));
    }
    let term = prose.render(&styled_term(false));
    assert_eq!(term, "\x1b[31mone\x1b[0m\n\n\x1b[31mtwo\x1b[0m");
}

#[test]
fn fenced_block_inside_a_style_is_a_sibling_and_the_style_resumes() {
    let prose = Prose::new("<red>a\n```\nx\n\ny\n```\nb</red>");
    let tree = prose.render_tree();
    assert_valid(&tree);
    let kinds: Vec<&str> = tree
        .children()
        .iter()
        .map(|n| match n.kind {
            NodeKind::Paragraph { .. } => "p",
            NodeKind::Code { .. } => "code",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["p", "code", "p"]);
    // The blank line inside the fence is code, not a boundary.
    let NodeKind::Code { value, .. } = &tree.children()[1].kind else { panic!("code") };
    assert_eq!(value, "x\n\ny");
}

#[test]
fn fenced_prose_renders_paragraph_and_pre_as_siblings() {
    let out = html(&Prose::new("intro\n```rust\nfn main() {}\n```\noutro"));
    assert!(out.starts_with("<p>intro</p><pre"), "{out}");
    assert!(out.contains("<code"), "{out}");
    assert!(out.ends_with("</pre><p>outro</p>"), "{out}");
    assert!(!out.contains("<span"), "{out}");
}

#[test]
fn quoted_attribute_newlines_belong_to_the_attribute() {
    let prose = Prose::new("<a href=\"x\n\ny\">t</a>");
    assert_eq!(paragraphs(&prose), 1);
}

#[test]
fn inline_prose_fence_becomes_one_inline_code_value() {
    let inline = InlineProse::new("see\n```rust\nlet a = 1;\n  let b = 2;\n```\nend");
    let nodes = inline.to_render_nodes();
    assert_eq!(
        inline_kinds(&nodes),
        ["Text(see)", "Soft", "Code(let a = 1;   let b = 2;)", "Soft", "Text(end)"]
    );
    assert!(nodes.iter().all(|n| !matches!(n.kind, NodeKind::Code { .. })));
    // `Hard` mode still joins the fence body with spaces.
    let hard = InlineProse::new("```\na\nb\n```").with_line_breaks(LineBreaks::Hard);
    assert_eq!(inline_kinds(&hard.to_render_nodes()), ["Code(a b)"]);
    // An empty body adds no node.
    assert_eq!(inline_kinds(&InlineProse::new("```\n```").to_render_nodes()), Vec::<String>::new());
}

// ── Block code inside inline wrappers ───────────────────────────────────

/// One inline wrapper of the Prose grammar, written around `a{code}b`.
pub(crate) struct Wrapper {
    pub name: &'static str,
    pub open: &'static str,
    pub close: &'static str,
    /// Spelled with Markdown delimiters, which a fence line interrupts
    /// (CommonMark: a fenced block ends the paragraph that holds the opener).
    pub markdown: bool,
    /// Whether the wrapper carries the `https://e.io` destination.
    pub link: bool,
    /// Whether `node` is the wrapper's outermost node.
    pub is_wrapper: fn(&RenderNode) -> bool,
}

impl Wrapper {
    pub fn wrap(&self, code: &str) -> String {
        format!("{}a{code}b{}", self.open, self.close)
    }

    /// Whether this wrapper's delimiters survive around a block of `code`.
    pub fn resumes_around(&self, code: &Code) -> bool {
        !(self.markdown && code.fenced)
    }
}

/// A block-code spelling placed between the wrapper's `a` and `b`.
pub(crate) struct Code {
    pub name: &'static str,
    pub source: &'static str,
    pub fenced: bool,
}

pub(crate) const CODE_SYNTAXES: [Code; 2] = [
    Code {
        name: "explicit",
        source: "<code-block>x</code-block>",
        fenced: false,
    },
    Code {
        name: "fenced",
        source: "\n```\nx\n```\n",
        fenced: true,
    },
];

fn is_link(node: &RenderNode) -> bool {
    matches!(&node.kind, NodeKind::Link { url, .. } if url == "https://e.io")
}

fn is_strong(node: &RenderNode) -> bool {
    matches!(node.kind, NodeKind::Strong { .. })
}

fn is_styled_span(node: &RenderNode) -> bool {
    matches!(node.kind, NodeKind::Span { .. }) && node.attrs.style().is_some()
}

/// Every wrapper branch of the token parser (style → semantic wrapper,
/// style → styled span, link, transparent), in tag and Markdown spellings,
/// plus nested wrappers.
pub(crate) fn wrappers() -> Vec<Wrapper> {
    vec![
        Wrapper { name: "bold", open: "<b>", close: "</b>", markdown: false, link: false, is_wrapper: is_strong },
        Wrapper { name: "md bold", open: "**", close: "**", markdown: true, link: false, is_wrapper: is_strong },
        Wrapper {
            name: "italic",
            open: "<i>",
            close: "</i>",
            markdown: false,
            link: false,
            is_wrapper: |n| matches!(n.kind, NodeKind::Emphasis { .. }),
        },
        Wrapper {
            name: "strikethrough",
            open: "<~>",
            close: "</~>",
            markdown: false,
            link: false,
            is_wrapper: |n| matches!(n.kind, NodeKind::Delete { .. }),
        },
        Wrapper { name: "color", open: "<red>", close: "</red>", markdown: false, link: false, is_wrapper: is_styled_span },
        Wrapper {
            name: "background",
            open: "<bg-navy>",
            close: "</bg-navy>",
            markdown: false,
            link: false,
            is_wrapper: is_styled_span,
        },
        Wrapper { name: "dim", open: "<dim>", close: "</dim>", markdown: false, link: false, is_wrapper: is_styled_span },
        Wrapper { name: "underline", open: "<u>", close: "</u>", markdown: false, link: false, is_wrapper: is_styled_span },
        Wrapper {
            name: "link",
            open: "<a href=\"https://e.io\">",
            close: "</a>",
            markdown: false,
            link: true,
            is_wrapper: is_link,
        },
        Wrapper { name: "md link", open: "[", close: "](https://e.io)", markdown: true, link: true, is_wrapper: is_link },
        Wrapper {
            name: "clipboard",
            open: "<clipboard>",
            close: "</clipboard>",
            markdown: false,
            link: false,
            is_wrapper: |n| matches!(n.kind, NodeKind::Text { .. }),
        },
        Wrapper {
            name: "link in bold",
            open: "<b><a href=\"https://e.io\">",
            close: "</a></b>",
            markdown: false,
            link: true,
            is_wrapper: |n| is_strong(n) && n.children().iter().any(is_link),
        },
        Wrapper {
            name: "bold in link",
            open: "<a href=\"https://e.io\"><b>",
            close: "</b></a>",
            markdown: false,
            link: true,
            is_wrapper: |n| is_link(n) && n.children().iter().any(is_strong),
        },
        Wrapper {
            name: "md link in md bold",
            open: "**[",
            close: "](https://e.io)**",
            markdown: true,
            link: true,
            is_wrapper: |n| is_strong(n) && n.children().iter().any(is_link),
        },
        Wrapper {
            name: "color in link in underline",
            open: "<u><a href=\"https://e.io\"><red>",
            close: "</red></a></u>",
            markdown: false,
            link: true,
            is_wrapper: |n| {
                is_styled_span(n)
                    && n.children().iter().any(|l| is_link(l) && l.children().iter().any(is_styled_span))
            },
        },
    ]
}

/// The kinds of a block list: `p` per paragraph, `code` per code block.
pub(crate) fn block_kinds(blocks: &[RenderNode]) -> Vec<&'static str> {
    blocks
        .iter()
        .map(|n| match n.kind {
            NodeKind::Paragraph { .. } => "p",
            NodeKind::Code { .. } => "code",
            _ => "other",
        })
        .collect()
}

/// Flattened text of `node`.
pub(crate) fn flat_text(node: &RenderNode) -> String {
    match &node.kind {
        NodeKind::Text { value } | NodeKind::InlineCode { value } => value.clone(),
        _ => node.children().iter().map(flat_text).collect(),
    }
}

/// Checks the `[p, code, p]` shape a wrapper around block code splits into:
/// the code is a sibling block holding `x`, and each paragraph holds `a` or
/// `b` inside the resumed wrapper (or, for Markdown delimiters a fence
/// interrupts, the literal delimiters).
pub(crate) fn assert_split_around_code(context: &str, wrapper: &Wrapper, code: &Code, blocks: &[RenderNode]) {
    assert_eq!(block_kinds(blocks), ["p", "code", "p"], "{context}");
    let NodeKind::Code { value, .. } = &blocks[1].kind else { unreachable!() };
    assert_eq!(value, "x", "{context}");
    for (paragraph, text) in [(&blocks[0], "a"), (&blocks[2], "b")] {
        if wrapper.resumes_around(code) {
            let inner = paragraph.children();
            assert_eq!(inner.len(), 1, "{context}: one wrapper per paragraph: {inner:?}");
            assert!((wrapper.is_wrapper)(&inner[0]), "{context}: wrapper resumes: {inner:?}");
            assert_eq!(flat_text(paragraph), text, "{context}");
        } else {
            assert!(flat_text(paragraph).contains(text), "{context}");
        }
    }
}

/// A `Prose` wrapper around explicit or fenced block code splits into
/// paragraphs around a sibling code block, with the wrapper (style, link
/// destination) resumed on both sides, and renders on every target.
#[test]
fn prose_wrapper_around_block_code_splits_and_resumes_on_every_target() {
    for wrapper in wrappers() {
        for code in &CODE_SYNTAXES {
            let context = format!("{} around {} code", wrapper.name, code.name);
            let prose = Prose::new(wrapper.wrap(code.source));

            let tree = prose.render_tree();
            assert_valid(&tree);
            assert_split_around_code(&context, &wrapper, code, tree.children());

            let out = html(&prose);
            assert!(out.contains("<pre><code>x</code></pre>"), "{context}: {out}");
            assert!(!out.contains("render-tree error"), "{context}: {out}");
            if wrapper.link && wrapper.resumes_around(code) {
                assert_eq!(out.matches("href=\"https://e.io\"").count(), 2, "{context}: {out}");
            }
            let markdown = md(&prose);
            assert!(markdown.contains("```\nx\n```"), "{context}: {markdown:?}");
            if wrapper.link && wrapper.resumes_around(code) {
                assert_eq!(markdown.matches("](https://e.io)").count(), 2, "{context}: {markdown:?}");
            }
            let term = strip(&prose.render(&plain_term()));
            assert!(term.contains('a') && term.contains('x') && term.contains('b'), "{context}: {term:?}");
        }
    }
}

/// The review's reproduction: an explicit anchor around explicit code
/// renders linked paragraphs around a code block, not an empty string.
#[test]
fn linked_explicit_code_renders_linked_paragraphs_around_the_block() {
    let out = html(&Prose::new("<a href=\"https://e.io\">a<code-block>x</code-block>b</a>"));
    assert_eq!(
        out,
        "<p><a href=\"https://e.io\">a</a></p><pre><code>x</code></pre><p><a href=\"https://e.io\">b</a></p>"
    );
}

/// In `InlineProse` block code is inline code, which is valid phrasing, so
/// every wrapper keeps it inside instead of splitting.
#[test]
fn inline_prose_wrapper_keeps_block_code_as_inline_code_inside() {
    for wrapper in wrappers() {
        for code in &CODE_SYNTAXES {
            let context = format!("{} around {} code", wrapper.name, code.name);
            let inline = InlineProse::new(wrapper.wrap(code.source));

            let tree = inline.render_tree();
            assert_valid(&tree);
            let nodes = inline.to_render_nodes();
            assert!(
                nodes.iter().all(|n| !matches!(n.kind, NodeKind::Code { .. })),
                "{context}: {nodes:?}"
            );
            if wrapper.name != "clipboard" {
                assert_eq!(nodes.len(), 1, "{context}: one wrapper: {nodes:?}");
                assert!((wrapper.is_wrapper)(&nodes[0]), "{context}: {nodes:?}");
            }
            assert!(flat_text(&tree).contains('x'), "{context}");

            let out = inline.render_html_fragment().render();
            assert!(out.contains("<code>x</code>"), "{context}: {out}");
            if wrapper.link {
                assert_eq!(out.matches("href=\"https://e.io\"").count(), 1, "{context}: {out}");
            }
            assert!(!inline.render_markdown().is_empty(), "{context}");
            assert!(strip(&inline.render(&plain_term())).contains('x'), "{context}");
        }
    }
}

// ── Whitespace around soft breaks across wrappers ───────────────────────

/// Text of inline `nodes` with each break read as one space, as the
/// terminal and HTML targets render a soft break.
fn spaced_text(nodes: &[RenderNode]) -> String {
    nodes
        .iter()
        .map(|n| match &n.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => value.clone(),
            NodeKind::SoftBreak | NodeKind::HardBreak => " ".into(),
            _ => spaced_text(n.children()),
        })
        .collect()
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Markdown `**` does not open before whitespace or close after it
/// (CommonMark flanking), so directly around a whitespace edge it stays
/// literal. Behind a link bracket it still forms.
fn forms_around_whitespace(wrapper: &Wrapper) -> bool {
    !(wrapper.markdown && wrapper.open == "**")
}

/// The inline nodes, HTML text, and terminal text (OSC 8 links, so a link
/// shows only its text) of `input` through both components.
fn soft_break_outputs(input: &str) -> [(&'static str, Vec<RenderNode>, String, String); 2] {
    let prose = Prose::new(input);
    let inline = InlineProse::new(input);
    assert_valid(&prose.render_tree());
    assert_valid(&inline.render_tree());
    [
        (
            "Prose",
            prose.render_tree().children().iter().flat_map(|p| p.children().to_vec()).collect(),
            strip_tags(&html(&prose)),
            strip(&prose.render(&styled_term(true))),
        ),
        (
            "InlineProse",
            inline.to_render_nodes(),
            strip_tags(&inline.render_html_fragment().render()),
            strip(&inline.render(&styled_term(true))),
        ),
    ]
}

/// The whitespace a soft break discards is found through every wrapper
/// boundary, into and out of the wrapper, in both components, while the
/// wrapper keeps its extent.
#[test]
fn soft_break_whitespace_is_discarded_across_every_wrapper_boundary() {
    for wrapper in wrappers() {
        for ws in [" ", " \t "] {
            for nl in ["\n", "\r\n"] {
                let (open, close) = (wrapper.open, wrapper.close);
                for (direction, input, inside) in [
                    ("into", format!("a{ws}{open}{ws}{nl}{ws}b{close}"), "b"),
                    ("out of", format!("{open}a{ws}{nl}{ws}{close}{ws}b"), "a"),
                ] {
                    for (component, nodes, html_text, term) in soft_break_outputs(&input) {
                        let context = format!("{component}: {} {direction} {input:?}", wrapper.name);
                        if !forms_around_whitespace(&wrapper) {
                            // Literal delimiters separate the break from the
                            // whitespace on their far side, which is kept.
                            let blanks = |s: &str| s.chars().filter(|c| matches!(c, ' ' | '\t')).count();
                            for out in [spaced_text(&nodes), html_text, term] {
                                assert_eq!(blanks(&out), blanks(ws) + 1, "{context}: {out:?}");
                            }
                            continue;
                        }
                        assert_eq!(spaced_text(&nodes), "a b", "{context}: {nodes:?}");
                        assert_eq!(html_text, "a b", "{context}");
                        assert_eq!(term, "a b", "{context}");
                        if wrapper.name != "clipboard" {
                            let wrapped: Vec<&RenderNode> =
                                nodes.iter().filter(|n| (wrapper.is_wrapper)(n)).collect();
                            assert_eq!(wrapped.len(), 1, "{context}: {nodes:?}");
                            assert_eq!(spaced_text(std::slice::from_ref(wrapped[0])).trim(), inside, "{context}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn whitespace_away_from_a_soft_break_is_kept_across_wrappers() {
    for (input, expected) in [
        ("a \t\n\t  b", "a b"),
        ("a  b", "a  b"),
        ("a <b> b</b>", "a  b"),
        ("<b>a </b> b", "a  b"),
        ("<a href=\"https://e.io\">a </a> b", "a  b"),
        ("a <b> b \nc</b>", "a  b c"),
    ] {
        for (component, nodes, html_text, term) in soft_break_outputs(input) {
            let context = format!("{component}: {input:?}");
            assert_eq!(spaced_text(&nodes), expected, "{context}");
            assert_eq!(html_text, expected, "{context}");
            assert_eq!(term, expected, "{context}");
        }
    }
}

/// Inline code beside a soft break keeps its own spaces; only the
/// surrounding text is trimmed.
#[test]
fn soft_break_whitespace_trim_stops_at_inline_code() {
    for (input, code, expected) in [
        ("`x` \n y", "x", "x y"),
        ("a \n` x`", " x", "a  x"),
        ("<b>`x ` \n</b> y", "x ", "x  y"),
        ("a <b>\n `x`</b>", "x", "a x"),
    ] {
        for (component, nodes, _, term) in soft_break_outputs(input) {
            let context = format!("{component}: {input:?}");
            assert_eq!(spaced_text(&nodes), expected, "{context}: {nodes:?}");
            assert_eq!(term, expected, "{context}");
            let flat = format!("{nodes:?}");
            assert!(flat.contains(&format!("InlineCode {{ value: {code:?} }}")), "{context}: {flat}");
        }
    }
}

/// Hard breaks are not soft breaks: their whitespace is kept, and the same
/// way whether or not a wrapper boundary sits beside the break.
#[test]
fn hard_break_whitespace_is_kept_the_same_across_wrappers() {
    for (wrapped, plain) in [("a <b> \nb</b>", "a  \nb"), ("<b>a \n</b> b", "a \n b"), ("a <b> \\\nb</b>", "a  \\\nb")] {
        let mode = |s: &str| Prose::new(s).with_line_breaks(LineBreaks::Hard);
        let wrapped_text = spaced_text(mode(wrapped).render_tree().children()[0].children());
        let plain_text = spaced_text(mode(plain).render_tree().children()[0].children());
        assert_eq!(wrapped_text, plain_text, "{wrapped:?}");
    }
}

/// A paragraph edge that meets a code block (explicit or fenced) carries no
/// break, space, or tab, through every wrapper and in both break modes; an
/// edge holding nothing else leaves no paragraph.
#[test]
fn paragraph_edges_at_block_code_drop_breaks_and_whitespace() {
    let spellings = [
        ("explicit inline", "<code-block>x</code-block>", false),
        ("explicit own line", "\n<code-block>x</code-block>\n", false),
        ("fenced", "\n```\nx\n```\n", true),
    ];
    for wrapper in wrappers() {
        for (name, source, fenced) in spellings {
            for mode in [LineBreaks::Soft, LineBreaks::Hard] {
                for nl in ["\n", "\r\n"] {
                    let source = source.replace('\n', nl);
                    let input = format!("{}a \t{source}\t b{}", wrapper.open, wrapper.close);
                    let context = format!("{} around {name} code, {mode:?}: {input:?}", wrapper.name);
                    let prose = Prose::new(&input).with_line_breaks(mode);
                    let tree = prose.render_tree();
                    assert_valid(&tree);
                    let blocks = tree.children();
                    let code = Code { name, source: "", fenced };
                    if wrapper.resumes_around(&code) {
                        assert_split_around_code(&context, &wrapper, &code, blocks);
                    }
                    for paragraph in [&blocks[0], &blocks[2]] {
                        let text = spaced_text(paragraph.children());
                        assert_eq!(text, text.trim(), "{context}: {paragraph:?}");
                    }
                    if wrapper.resumes_around(&code) {
                        assert_eq!(strip_tags(&html(&prose)), "axb", "{context}");
                    }
                }
            }
        }
    }
    let only_code = Prose::new("<b>\n<code-block>x</code-block>\n</b>");
    assert_eq!(block_kinds(only_code.render_tree().children()), ["code"]);
    assert_eq!(html(&Prose::new("a \n<code-block>x</code-block>")), "<p>a</p><pre><code>x</code></pre>");
    // Edges that meet no code block keep their whitespace.
    assert_eq!(html(&Prose::new("  a")), "<p>  a</p>");
    // In InlineProse the code stays inline, so the breaks around it stay.
    let inline = InlineProse::new("a \n<code-block>x</code-block>\n b").to_render_nodes();
    assert_eq!(inline_kinds(&inline), ["Text(a)", "Soft", "Code(x)", "Soft", "Text(b)"]);
}

// ── Sentinel safety (R1) ────────────────────────────────────────────────

#[test]
fn sentinel_lookalike_input_stays_user_content() {
    for input in [
        "x \u{2}0\u{2} y `real`",
        "x \u{1}HREF0\u{1} [a](u)",
        "`\u{2}0\u{2}`",
        "\\\u{2}0\u{2}",
        "[l](u\u{2}0\u{2})",
    ] {
        let nodes = InlineProse::new(input).to_render_nodes();
        let text = inline_kinds(&nodes).join(" ");
        assert!(text.contains('\u{2}') || text.contains('\u{1}'), "{input:?} lost its sentinel: {text}");
        assert_valid(&InlineProse::new(input).render_tree());
    }
    let nodes = InlineProse::new("x \u{2}0\u{2} y `real`").to_render_nodes();
    assert_eq!(inline_kinds(&nodes), ["Text(x \u{2}0\u{2} y )", "Code(real)"]);
    let nodes = InlineProse::new("`\u{2}0\u{2}`").to_render_nodes();
    assert_eq!(inline_kinds(&nodes), ["Code(\u{2}0\u{2})"]);
}

#[test]
fn escaped_tags_and_delimiters_stay_user_content() {
    let nodes = InlineProse::new("\\<red\\>x\\</red\\> \\*\\*b\\*\\* \\_i\\_").to_render_nodes();
    assert_eq!(inline_kinds(&nodes), ["Text(<red>x</red> **b** _i_)"]);
}

// ── ProseTag (AC 8) ─────────────────────────────────────────────────────

#[test]
fn prose_tag_sets_each_paragraph_element_in_html_only() {
    let one = Prose::new("x").with_tag(ProseTag::Div);
    assert_eq!(html(&one), "<div>x</div>");
    let two = Prose::new("one\n\ntwo").with_tag(ProseTag::Div);
    assert_eq!(html(&two), "<div>one</div><div>two</div>");
    assert_eq!(md(&two), md(&Prose::new("one\n\ntwo")));
    assert_eq!(two.render(&styled_term(false)), Prose::new("one\n\ntwo").render(&styled_term(false)));

    for (tag, element) in [
        (ProseTag::P, "p"),
        (ProseTag::Section, "section"),
        (ProseTag::Article, "article"),
        (ProseTag::Aside, "aside"),
        (ProseTag::Header, "header"),
        (ProseTag::Footer, "footer"),
    ] {
        assert_eq!(html(&Prose::new("x").with_tag(tag)), format!("<{element}>x</{element}>"));
    }
    // Code blocks stay `<pre><code>` whatever the tag.
    let fenced = html(&Prose::new("```\nc\n```").with_tag(ProseTag::Div));
    assert!(fenced.starts_with("<pre"), "{fenced}");
}

// ── Layout and class (AC 10, 11) ────────────────────────────────────────

#[test]
fn left_margin_renders_in_terminal_and_html() {
    let prose = Prose::new("hi").with_left_margin(TargetValue::universal(Length::ch(4)));
    assert_eq!(prose.render(&plain_term()), "    hi");
    let out = html(&prose);
    assert_eq!(out.matches("margin-left").count(), 1, "{out}");
    assert!(out.contains("margin-left:4ch"), "{out}");
    assert!(out.contains("<p>hi</p>"), "{out}");
}

#[test]
fn vertical_margins_survive_word_wrap_on_the_terminal() {
    // The trailing empty rows are the bottom margin; the word-wrap pass once
    // dropped them while `WordWrap::None` kept them.
    for wrap in [WordWrap::None, WordWrap::WrapProse(None, None)] {
        let mut prose = Prose::new("one\n\ntwo").with_word_wrap(wrap.clone());
        prose.layout_mut().margin.top = TargetValue::universal(Length::ch(1));
        prose.layout_mut().margin.bottom = TargetValue::universal(Length::ch(2));
        assert_eq!(prose.render(&plain_term()), "\none\n\ntwo\n\n", "{wrap:?}");
        let out = html(&prose);
        assert!(out.contains("margin-top:1lh;margin-bottom:2lh"), "{out}");
    }
}

#[test]
fn no_prose_class_is_generated_and_user_content_is_untouched() {
    for out in [
        html(&Prose::new("**x** `y`\n\nz")),
        InlineProse::new("**x** `y`").render_html_fragment().render(),
    ] {
        assert!(!out.contains("class=\"prose\""), "{out}");
    }
    let literal = html(&Prose::new("class=\"prose\""));
    assert_eq!(literal, "<p>class=\"prose\"</p>");
}

// ── Terminal inline-code fallback and restore (AC 21, 29) ───────────────

#[test]
fn unstyled_terminal_keeps_a_backtick_fence() {
    let term = plain_term();
    assert_eq!(Prose::new("See `md hash`").render(&term), "See `md hash`");
    assert_eq!(Prose::new("``a`b``").render(&term), "``a`b``");
    // Styled: no delimiter is added; literal backticks in the value remain.
    let styled = Prose::new("``a`b``").render(&styled_term(false));
    assert_eq!(styled, "\x1b[2ma`b\x1b[0m");
}

#[test]
fn nested_code_restores_the_enclosing_style() {
    let out = Prose::new("<red>a `b` c</red>").render(&styled_term(false));
    assert_eq!(out, "\x1b[31ma \x1b[2m\x1b[31mb\x1b[0m\x1b[31m c\x1b[0m");
}

#[test]
fn nested_code_preserves_a_surrounding_hyperlink() {
    for osc8 in [true, false] {
        let out = InlineProse::new("[go `x` now](https://e.io)").render(&styled_term(osc8));
        if osc8 {
            assert!(out.starts_with("\x1b]8;;https://e.io\x1b\\"), "{out:?}");
            assert!(out.ends_with("\x1b]8;;\x1b\\"), "{out:?}");
        } else {
            assert!(strip(&out).ends_with("](https://e.io)"), "{out:?}");
        }
        assert!(out.contains("\x1b[2mx\x1b[0m"), "{out:?}");
    }
}

#[test]
fn inherited_styling_without_new_code_styling_keeps_the_fence() {
    // Inside a dim run, inline code's dim adds nothing, so it stays marked.
    let out = Prose::new("<dim>a `b`</dim>").render(&styled_term(false));
    assert_eq!(strip(&out), "a `b`");
}

#[test]
fn unstyled_fallback_applies_to_a_directly_built_tree() {
    // The shape darkmatter produces: inline code inside a link in a paragraph.
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("Run "),
        RenderNode::link("https://e.io", None, vec![RenderNode::inline_code("md hash")]),
    ])]);
    for osc8 in [true, false] {
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .osc_link_support(osc8)
            .build();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let out = render_terminal_node(&tree, &opts).expect("renders").output;
        assert!(out.contains("`md hash`"), "osc8={osc8}: {out:?}");
        assert!(!out.contains("\x1b["), "osc8={osc8}: {out:?}");
    }
}

// ── escaping text that carries its own code spans ───────────────────────

#[test]
fn escape_text_outside_code_spans_table() {
    let cases = [
        // Text outside a span is escaped; the span is copied unchanged.
        (r"a_b `_a_[x]{y}a\b` c_d", r"a\_b `_a_[x]{y}a\b` c\_d"),
        // A double-backtick fence holds a single backtick.
        ("`` q`_a_` `` and x_y", r"`` q`_a_` `` and x\_y"),
        // A run closes only on a run of the same length.
        ("``x` y_z``", "``x` y_z``"),
        // Unmatched runs are ordinary text, escaped with what follows them.
        ("a ` b_c", r"a \` b\_c"),
        ("``x` y_z", r"\`\`x\` y\_z"),
        // A span never crosses a blank line.
        ("`a_b\n\nc_d`", "\\`a\\_b\n\nc\\_d\\`"),
        ("`a_b\nc_d`", "`a_b\nc_d`"),
        ("", ""),
    ];
    for (input, expected) in cases {
        assert_eq!(Prose::escape_text_outside_code_spans(input), expected, "input {input:?}");
        assert_eq!(InlineProse::escape_text_outside_code_spans(input), expected, "input {input:?}");
    }
}

#[test]
fn escape_text_outside_code_spans_renders_message_exactly() {
    let message = r"unknown field `foo_bar`, expected `a[0]` or `{{x}}` in my_file\_a";
    let prose = InlineProse::new(InlineProse::escape_text_outside_code_spans(message));
    let rendered = strip(&prose.render(&plain_term()));
    assert_eq!(rendered, message);
}

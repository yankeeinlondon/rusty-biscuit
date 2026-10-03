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
use biscuit_terminal::utils::layout::{Length, TargetValue};
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
    assert_eq!(InlineProse::new("  label\t").render_markdown(), "  label\t");
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
    // An absolute URL: Prose resolves relative link targets to `file://`
    // URLs in the tree (pre-existing behavior; see the implementation log).
    let label = InlineProse::new("Run `md hash` on [the plan](https://x.io/plan.md)");
    assert_eq!(label.render_markdown(), "Run `md hash` on [the plan](https://x.io/plan.md)");
    assert_eq!(
        label.render_html_fragment().render(),
        r#"Run <code>md hash</code> on <a href="https://x.io/plan.md">the plan</a>"#
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

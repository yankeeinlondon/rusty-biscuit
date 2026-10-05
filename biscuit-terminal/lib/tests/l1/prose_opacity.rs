//! Opaque regions of the Prose grammar: code bodies and quoted attributes.
//!
//! An explicit `<code-block>` body, a fenced body, and a quoted tag attribute
//! value reach every output exactly as authored: no Markdown, tag, escape, or
//! fence syntax inside them is interpreted, and no parser placeholder
//! character appears in any output unless the author wrote one. Each case is
//! checked through both components on every target: the render tree, HTML,
//! Markdown (read back with `pulldown-cmark`), and the terminal.

use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

/// The five bodies the opacity contract is checked with.
const BODIES: [&str; 5] = ["`x`", "**x**", "[x](https://e.io)", "a\u{0002}b", "x\ny"];

/// Characters the pre-processor uses as placeholder sentinels.
const SENTINELS: [char; 2] = ['\u{0001}', '\u{0002}'];

fn term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(false)
        .width(80)
        .build()
}

fn strip(s: &str) -> String {
    biscuit_terminal::utils::escape_codes::strip_escape_codes(s.to_string())
}

fn sentinel_count(s: &str) -> usize {
    s.chars().filter(|c| SENTINELS.contains(c)).count()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Every `Code` and `InlineCode` value in the trees, in document order.
fn code_values(nodes: &[RenderNode]) -> Vec<String> {
    let mut values = Vec::new();
    for node in nodes {
        match &node.kind {
            NodeKind::Code { value, .. } | NodeKind::InlineCode { value } => values.push(value.clone()),
            _ => values.extend(code_values(node.children())),
        }
    }
    values
}

/// Every link destination in the trees, in document order.
fn link_urls(nodes: &[RenderNode]) -> Vec<String> {
    let mut urls = Vec::new();
    for node in nodes {
        if let NodeKind::Link { url, .. } = &node.kind {
            urls.push(url.clone());
        }
        urls.extend(link_urls(node.children()));
    }
    urls
}

/// The code a CommonMark reader finds in `markdown`: each code block's text
/// without its final line ending, and each code span's value.
fn markdown_code(markdown: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut block: Option<String> = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => block = Some(String::new()),
            Event::End(TagEnd::CodeBlock) => {
                let text = block.take().unwrap_or_default();
                values.push(text.strip_suffix('\n').unwrap_or(&text).to_string());
            }
            Event::Text(text) if block.is_some() => block.as_mut().unwrap().push_str(&text),
            Event::Code(code) => values.push(code.to_string()),
            _ => {}
        }
    }
    values
}

/// The two authored forms of a code block holding `body`.
fn code_sources(body: &str) -> [(&'static str, String); 2] {
    [
        ("explicit", format!("<code-block>{body}</code-block>")),
        ("fenced", format!("```\n{body}\n```")),
    ]
}

/// Every output of both components for `source`, labeled.
fn all_outputs(source: &str) -> Vec<(&'static str, String)> {
    let prose = Prose::new(source);
    let inline = InlineProse::new(source);
    let term = term();
    vec![
        ("Prose/HTML", prose.render_html_fragment().render()),
        ("Prose/Markdown", prose.render_markdown()),
        ("Prose/MarkdownPlus", prose.render_markdown_plus()),
        ("Prose/terminal", prose.render(&term)),
        ("InlineProse/HTML", inline.render_html_fragment().render()),
        ("InlineProse/Markdown", inline.render_markdown()),
        ("InlineProse/MarkdownPlus", inline.render_markdown_plus()),
        ("InlineProse/terminal", inline.render(&term)),
    ]
}

/// No output carries more sentinel characters than the author wrote.
fn assert_no_generated_sentinels(source: &str) {
    let authored = sentinel_count(source);
    for (label, output) in all_outputs(source) {
        assert!(
            sentinel_count(&output) <= authored,
            "{label} leaked a placeholder for {source:?}: {output:?}"
        );
    }
}

/// Assert `body` survives as a block in `Prose` and as one inline value (line
/// endings as spaces) in `InlineProse`, on every target.
fn assert_code_body_is_opaque(source: &str, body: &str) {
    let inline_body = body.replace('\n', " ");
    let prose = Prose::new(source);
    let inline = InlineProse::new(source);
    let term = term();

    assert_eq!(code_values(prose.render_tree().children()), [body], "Prose tree {source:?}");
    assert_eq!(code_values(&inline.to_render_nodes()), [inline_body.as_str()], "InlineProse tree {source:?}");

    let prose_html = prose.render_html_fragment().render();
    assert!(prose_html.contains("<pre><code"), "Prose HTML {source:?}: {prose_html}");
    assert!(prose_html.contains(&html_escape(body)), "Prose HTML {source:?}: {prose_html:?}");
    let inline_html = inline.render_html_fragment().render();
    assert!(
        inline_html.contains(&format!("<code>{}</code>", html_escape(&inline_body))),
        "InlineProse HTML {source:?}: {inline_html:?}"
    );

    for markdown in [prose.render_markdown(), prose.render_markdown_plus()] {
        assert_eq!(markdown_code(&markdown), [body], "Prose Markdown {source:?}: {markdown:?}");
    }
    for markdown in [inline.render_markdown(), inline.render_markdown_plus()] {
        assert_eq!(markdown_code(&markdown), [inline_body.as_str()], "InlineProse Markdown {source:?}: {markdown:?}");
    }

    let prose_term = strip(&prose.render(&term));
    for line in body.lines() {
        assert!(prose_term.contains(line), "Prose terminal {source:?}: {prose_term:?}");
    }
    let inline_term = strip(&inline.render(&term));
    assert!(inline_term.contains(&inline_body), "InlineProse terminal {source:?}: {inline_term:?}");

    assert_no_generated_sentinels(source);
}

// ── Code bodies ──────────────────────────────────────────────────────────

#[test]
fn explicit_and_fenced_code_bodies_are_literal_in_both_components() {
    for body in BODIES {
        for (_, source) in code_sources(body) {
            assert_code_body_is_opaque(&source, body);
        }
    }
}

#[test]
fn code_bodies_are_literal_in_hard_break_mode() {
    for body in BODIES {
        for (_, source) in code_sources(body) {
            let prose = Prose::new(&source).with_line_breaks(LineBreaks::Hard);
            let inline = InlineProse::new(&source).with_line_breaks(LineBreaks::Hard);
            assert_eq!(code_values(prose.render_tree().children()), [body], "{source:?}");
            assert_eq!(code_values(&inline.to_render_nodes()), [body.replace('\n', " ")], "{source:?}");
        }
    }
}

#[test]
fn explicit_code_body_keeps_fence_lines_tags_escapes_and_blank_lines() {
    for body in [
        "a\n```\nb\n```\nc",
        "<red>x</red> <b>y</b>",
        r"\*a\* \_b\_ \<c\>",
        "\u{0002}0\u{0002} and \u{0001}HREF0\u{0001}",
        "a\\\u{0002}b",
        "_i_ `` ` `` ~",
    ] {
        assert_code_body_is_opaque(&format!("<code-block>{body}</code-block>"), body);
    }
    let blank = "a\n\nb";
    let prose = Prose::new(format!("<code-block>{blank}</code-block>"));
    assert_eq!(code_values(prose.render_tree().children()), [blank]);
}

#[test]
fn fenced_code_body_keeps_tags_escapes_and_placeholder_lookalikes() {
    for body in ["<code-block>x</code-block>", r"\*a\*", "\u{0002}0\u{0002}", "a\\\u{0002}b"] {
        assert_code_body_is_opaque(&format!("```\n{body}\n```"), body);
    }
}

#[test]
fn explicit_code_block_variants_stay_opaque() {
    // Upper-case tag, a language hint (with a `>` inside its quotes), text
    // around the block, and a style around it.
    assert_code_body_is_opaque("<CODE-BLOCK>**x**</Code-Block>", "**x**");
    assert_code_body_is_opaque("<code-block lang=\"r>s\">`x` **y**</code-block>", "`x` **y**");
    let tree = Prose::new("<code-block lang=\"rust\">**x**</code-block>").render_tree();
    assert!(matches!(
        &tree.children()[0].kind,
        NodeKind::Code { lang: Some(lang), value, .. } if lang == "rust" && value == "**x**"
    ));
    let tree = Prose::new("a **b** <code-block>**x**</code-block> c").render_tree();
    assert_eq!(code_values(tree.children()), ["**x**"]);
    let tree = Prose::new("<red>a <code-block>**x** `y`</code-block> b</red>").render_tree();
    assert_eq!(code_values(tree.children()), ["**x** `y`"]);
    assert_no_generated_sentinels("<red>a <code-block>**x** `y`</code-block> b</red>");
}

#[test]
fn explicit_code_body_ends_at_the_first_closing_tag() {
    // The body is not parsed, so a nested opening tag does not deepen it.
    let source = "<code-block><code-block>x</code-block> after";
    let prose = Prose::new(source);
    assert_eq!(code_values(prose.render_tree().children()), ["<code-block>x"]);
    assert!(strip(&prose.render(&term())).contains("after"));
    let inline = InlineProse::new(source);
    assert_eq!(code_values(&inline.to_render_nodes()), ["<code-block>x"]);
    assert_no_generated_sentinels(source);
}

#[test]
fn unclosed_explicit_code_block_takes_the_rest_of_the_input_literally() {
    let source = "<code-block>**x** `y`\n\n_z_";
    assert_eq!(code_values(Prose::new(source).render_tree().children()), ["**x** `y`\n\n_z_"]);
    assert_eq!(code_values(&InlineProse::new(source).to_render_nodes()), ["**x** `y`  _z_"]);
    assert_no_generated_sentinels(source);
}

#[test]
fn code_span_and_escaped_tag_controls_are_not_explicit_code() {
    // A code span that starts first wins over the tag text inside it.
    let source = "`<code-block>` **x**";
    assert_eq!(code_values(Prose::new(source).render_tree().children()), ["<code-block>"]);
    // An escaped declaration is literal text, and its body is ordinary prose.
    let html = Prose::new(r"\<code-block>**x**\</code-block>").render_html_fragment().render();
    assert!(html.contains("&lt;code-block&gt;<strong>x</strong>"), "{html}");
}

#[test]
fn literal_sentinels_in_prose_text_render_as_authored() {
    for source in ["a\u{0002}0\u{0002}b", "a\\\u{0002}0\u{0002}b", "\u{0001}HREF0\u{0001} [l](https://e.io)"] {
        assert_no_generated_sentinels(source);
        for (label, output) in all_outputs(source) {
            assert_eq!(sentinel_count(&output), sentinel_count(source), "{label} {source:?}: {output:?}");
        }
    }
    let html = Prose::new("a\\\u{0002}0\u{0002}b").render_html_fragment().render();
    assert!(html.contains("a\\\u{0002}0\u{0002}b"), "{html:?}");
}

// ── Quoted attributes ────────────────────────────────────────────────────

#[test]
fn quoted_attribute_with_fence_lines_keeps_its_value() {
    let href = "https://e.io/x\n```\ny\n```\nz";
    for source in [format!("<a href=\"{href}\">t</a>"), format!("<a href='{href}'>t</a>")] {
        assert_eq!(link_urls(Prose::new(&source).render_tree().children()), [href], "{source:?}");
        assert_eq!(link_urls(&InlineProse::new(&source).to_render_nodes()), [href], "{source:?}");
        assert!(code_values(Prose::new(&source).render_tree().children()).is_empty());
        assert_no_generated_sentinels(&source);
    }
}

#[test]
fn quoted_attribute_with_plain_newlines_keeps_its_value() {
    for href in ["https://e.io/x\ny", "https://e.io/x\n\ny"] {
        let source = format!("<a href=\"{href}\">t</a> after");
        let prose = Prose::new(&source);
        assert_eq!(link_urls(prose.render_tree().children()), [href], "{source:?}");
        // The newlines belong to the attribute, not the paragraph splitter.
        let paragraphs = prose
            .render_tree()
            .children()
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Paragraph { .. }))
            .count();
        assert_eq!(paragraphs, 1, "{source:?}");
        assert_eq!(link_urls(&InlineProse::new(&source).to_render_nodes()), [href], "{source:?}");
        assert_no_generated_sentinels(&source);
    }
}

#[test]
fn quoted_attribute_markup_is_never_interpreted() {
    for href in [
        "https://e.io/?q=a>**b**",
        "https://e.io/?q=a>`c`",
        "https://e.io/?q=a>[x](https://f.io)",
        "https://e.io/?q=a>_d_",
        "https://e.io/<code-block>x</code-block>",
    ] {
        let source = format!("<a href=\"{href}\">t</a> **b**");
        let prose = Prose::new(&source);
        assert_eq!(link_urls(prose.render_tree().children()), [href], "{source:?}");
        assert!(code_values(prose.render_tree().children()).is_empty(), "{source:?}");
        assert_eq!(link_urls(&InlineProse::new(&source).to_render_nodes()), [href], "{source:?}");
        let html = prose.render_html_fragment().render();
        assert!(html.contains("<strong>b</strong>"), "{source:?}: {html}");
        assert_no_generated_sentinels(&source);
    }
}

#[test]
fn literal_sentinels_in_a_quoted_attribute_are_kept() {
    let href = "https://e.io/\u{0002}0\u{0002}";
    let source = format!("`c` <a href=\"{href}\">t</a>");
    assert_eq!(link_urls(Prose::new(&source).render_tree().children()), [href]);
    assert_eq!(link_urls(&InlineProse::new(&source).to_render_nodes()), [href]);
}

#[test]
fn unrecognized_declaration_never_swallows_lifted_content() {
    // A `<` that opens no recognized tag is literal; code and fences after it
    // still render as code rather than as placeholder characters.
    for source in ["a <\"x `c` d", "a <'x\n```\nb\n```\n", "a <x `c` y> z", "a <b `c`"] {
        assert_no_generated_sentinels(source);
        let inline = InlineProse::new(source);
        assert!(!code_values(&inline.to_render_nodes()).is_empty(), "{source:?}");
    }
}

// ── Wrapper scope ────────────────────────────────────────────────────────

/// The nine wrapper categories: opening tag, closing tag, and the tag-name
/// spelling a nested opener uses.
const WRAPPERS: [(&str, &str, &str); 9] = [
    ("<b>", "</b>", "b"),
    ("<i>", "</i>", "i"),
    ("<~>", "</~>", "~"),
    ("<red>", "</red>", "red"),
    ("<bg-navy>", "</bg-navy>", "bg-navy"),
    ("<dim>", "</dim>", "dim"),
    ("<u>", "</u>", "u"),
    ("<clipboard>", "</clipboard>", "clipboard"),
    ("<a href=\"outer\">", "</a>", "a"),
];

/// Stands in for the tag spelling in a control source.
const MARK: &str = "ZQZ";

/// One scope shape: a body template holding `{X}` where the tag spelling
/// goes, and whether the shape nests an anchor (skipped inside a link).
struct ScopeShape {
    name: &'static str,
    body: &'static str,
    nests_anchor: bool,
}

const SCOPE_SHAPES: [ScopeShape; 5] = [
    ScopeShape { name: "nested anchor href", body: r#"<a href="https://e.io/{X}">label</a>"#, nests_anchor: true },
    ScopeShape { name: "code-block lang", body: r#"<code-block lang="{X}">body</code-block>"#, nests_anchor: false },
    ScopeShape { name: "styled declaration attribute", body: r#"<green note="{X}">label</green>"#, nests_anchor: false },
    ScopeShape { name: "code span", body: "`{X}`", nests_anchor: false },
    ScopeShape { name: "explicit code body", body: "<code-block>{X}</code-block>", nests_anchor: false },
];

/// A wrapped source with its control: the same source with the tag spelling
/// replaced by [`MARK`].
fn wrapped(open: &str, close: &str, body: &str, authored: &str) -> (String, String) {
    let template = format!("{open}before {body} after{close} tail");
    (template.replace("{X}", authored), template.replace("{X}", MARK))
}

/// Assert `source` renders exactly like `control` with [`MARK`] read as
/// `literal`, through both components' render trees and HTML, so the tag
/// spelling is content and the wrapper ends where the control's does.
///
/// HTML attribute values leave `>` unescaped while text escapes it, so both
/// HTML sides compare with `&gt;` read as `>`; every `<` stays escaped.
fn assert_scope_matches_control(source: &str, control: &str, literal: &str) {
    let prose = |s: &str| Prose::new(s);
    let inline = |s: &str| InlineProse::new(s);
    let html_of = |html: String| html.replace("&gt;", ">");
    let quoted = format!("{literal:?}");
    let debug_literal = &quoted[1..quoted.len() - 1];
    let checks: [(&str, String, String); 4] = [
        (
            "Prose tree",
            format!("{:?}", prose(source).render_tree()),
            format!("{:?}", prose(control).render_tree()).replace(MARK, debug_literal),
        ),
        (
            "InlineProse tree",
            format!("{:?}", inline(source).to_render_nodes()),
            format!("{:?}", inline(control).to_render_nodes()).replace(MARK, debug_literal),
        ),
        (
            "Prose HTML",
            html_of(prose(source).render_html_fragment().render()),
            html_of(prose(control).render_html_fragment().render()).replace(MARK, &html_of(html_escape(literal))),
        ),
        (
            "InlineProse HTML",
            html_of(inline(source).render_html_fragment().render()),
            html_of(inline(control).render_html_fragment().render()).replace(MARK, &html_of(html_escape(literal))),
        ),
    ];
    for (label, actual, expected) in checks {
        assert_eq!(actual, expected, "{label} for {source:?}");
    }
    assert_no_generated_sentinels(source);
}

#[test]
fn closing_tag_text_in_a_nested_anchor_href_keeps_the_link_and_the_bold_extent() {
    let source = r#"<b>before <a href="https://e.io/</b>">label</a> after</b> tail"#;
    for nodes in [Prose::new(source).render_tree().children().to_vec(), InlineProse::new(source).to_render_nodes()] {
        assert_eq!(link_urls(&nodes), ["https://e.io/</b>"], "{source:?}");
    }
    for html in [
        Prose::new(source).render_html_fragment().render(),
        InlineProse::new(source).render_html_fragment().render(),
    ] {
        assert!(html.contains("<strong>before <a href=\"https://e.io/"), "{html}");
        assert!(html.contains(">label</a> after</strong> tail"), "{html}");
    }
}

#[test]
fn tag_text_in_quoted_attributes_and_code_never_changes_wrapper_scope() {
    for (open, close, name) in WRAPPERS {
        let opening = format!("<{name}>");
        // (authored spelling, the literal value it reads as)
        let spellings = [
            (close.to_string(), close.to_string()),
            (opening.clone(), opening.clone()),
            (format!("\\{close}"), close.to_string()),
        ];
        for shape in &SCOPE_SHAPES {
            if shape.nests_anchor && name == "a" {
                continue;
            }
            let is_code = shape.name.contains("code span") || shape.name.contains("code body");
            for (authored, literal) in &spellings {
                // Code is verbatim: a backslash there is part of the value.
                let literal = if is_code { authored } else { literal };
                let (source, control) = wrapped(open, close, shape.body, authored);
                assert_scope_matches_control(&source, &control, literal);
            }
        }
    }
}

#[test]
fn escaped_tag_text_in_ordinary_content_never_changes_wrapper_scope() {
    for (open, close, name) in WRAPPERS {
        for literal in [close.to_string(), format!("<{name}>")] {
            let (source, control) = wrapped(open, close, "{X}", &format!("\\{literal}"));
            assert_scope_matches_control(&source, &control, &literal);
        }
    }
}

#[test]
fn bracket_text_in_a_quoted_attribute_never_ends_a_markdown_link_label() {
    for (authored, literal) in [("]", "]"), ("](https://x.io)", "](https://x.io)"), ("\\]", "\\]")] {
        let template = r#"[see <green note="{X}">label</green> more](https://e.io) tail"#;
        let source = template.replace("{X}", authored);
        assert_scope_matches_control(&source, &template.replace("{X}", MARK), literal);
        assert_eq!(link_urls(&InlineProse::new(&source).to_render_nodes()), ["https://e.io"], "{source:?}");
    }
}

//! Raw `Html` payloads inside MarkdownPlus HTML blocks keep their bytes.
//!
//! MarkdownPlus writes a disclosure summary and a columns container as one
//! raw HTML block, which a blank line would end. The writer encodes line
//! endings in the HTML it generates to keep the block open, but a raw `Html`
//! payload is the author's: script and style source, comments, and `xmp`,
//! `iframe`, `noembed`, and `noframes` content do not decode character
//! references, and whitespace separates unquoted attributes. So a payload
//! that fits in the block must appear byte for byte, and one that cannot
//! (it holds a blank line itself) must be reported, never silently changed.
//!
//! Each assertion is made on the rendered Markdown bytes and on an
//! independent CommonMark reading (`pulldown-cmark`) of where the HTML block
//! ends. No HTML text decoding is involved: a byte-identical payload reaches
//! the HTML parser as authored, whatever parsing state its content is in.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    BrowserRenderOptions, ColumnsHints, DiagnosticKind, MarkdownDialect, MarkdownRenderOptions,
    RawHtmlPolicy, RenderError, RenderNode, RenderStrictness, SequenceJoin, render_browser_node,
    render_markdown_node,
};

/// The diagnostic message a payload that cannot be embedded produces.
const UNFAITHFUL: &str = "raw HTML holds a blank line";

/// Whether a payload can sit inside a raw HTML block unchanged.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fit {
    /// No blank line: written byte for byte.
    Faithful,
    /// Holds a blank or whitespace-only line, which would end the block.
    Unfaithful,
}

/// Every payload shape, by name.
fn shapes() -> Vec<(&'static str, &'static str, Fit)> {
    use Fit::{Faithful, Unfaithful};
    vec![
        (
            "script, blank line",
            "<script>const a = 1;\n\nconsole.log(a);</script>",
            Unfaithful,
        ),
        (
            "script, whitespace-only line",
            "<script>const a = 1;\n \t\nconsole.log(a);</script>",
            Unfaithful,
        ),
        (
            "script, lone CR",
            "<script>const a = 1;\rconsole.log(a);</script>",
            Faithful,
        ),
        (
            "script, two lone CRs",
            "<script>const a = 1;\r\rconsole.log(a);</script>",
            Unfaithful,
        ),
        (
            "script, CRLF blank line",
            "<script>const a = 1;\r\n\r\nconsole.log(a);</script>",
            Unfaithful,
        ),
        (
            "style, blank line",
            "<style>p { color: red; }\n\nb { color: blue; }</style>",
            Unfaithful,
        ),
        (
            "unquoted attributes, blank line",
            "<span data-a=x\n\ndata-b=y>t</span>",
            Unfaithful,
        ),
        (
            "unquoted attributes, lone CR",
            "<span data-a=x\rdata-b=y>t</span>",
            Faithful,
        ),
        ("xmp, blank line", "<xmp>a\n\nb</xmp>", Unfaithful),
        ("iframe, blank line", "<iframe>a\n\nb</iframe>", Unfaithful),
        (
            "noembed, blank line",
            "<noembed>a\n\nb</noembed>",
            Unfaithful,
        ),
        (
            "noframes, blank line",
            "<noframes>a\n\nb</noframes>",
            Unfaithful,
        ),
        ("comment, blank line", "<!-- a\n\nb -->", Unfaithful),
        ("span text, blank line", "<span>a\n\nb</span>", Unfaithful),
        (
            "quoted attribute, blank line",
            "<span title=\"a\n\nb\">t</span>",
            Unfaithful,
        ),
        ("pre, blank line", "<pre>a\n\nb</pre>", Unfaithful),
        (
            "textarea, blank line",
            "<textarea>a\n\nb</textarea>",
            Unfaithful,
        ),
        (
            "script, single LF",
            "<script>const a = 1;\nconsole.log(a);</script>",
            Faithful,
        ),
        (
            "script, no line ending",
            "<script>const a = 1; console.log(a);</script>",
            Faithful,
        ),
        (
            "unquoted attributes, no line ending",
            "<span data-a=x data-b=y>t</span>",
            Faithful,
        ),
    ]
}

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

fn text(value: &str) -> RenderNode {
    RenderNode::text(value)
}

fn para(children: Vec<RenderNode>) -> RenderNode {
    RenderNode::paragraph(children)
}

fn inline_raw(payload: &str) -> RenderNode {
    RenderNode::html(payload, false)
}

fn disclosure(summary: Vec<RenderNode>) -> RenderNode {
    RenderNode::disclosure(summary, vec![para(vec![text("body")])], None)
}

fn columns(left: Vec<RenderNode>, right: Vec<RenderNode>) -> RenderNode {
    let left_count = left.len();
    let mut node = RenderNode::block_quote(left.into_iter().chain(right).collect());
    node.attrs.set_columns_hints(&ColumnsHints {
        left_count,
        ..Default::default()
    });
    node
}

/// Where the HTML block holding the payload must end.
#[derive(Debug, Clone, Copy)]
enum Block {
    /// The `<details>` line's block, through `</summary>`.
    Summary,
    /// The whole columns container.
    Columns,
}

impl Block {
    /// The markup that closes the HTML-lowered region.
    fn closer(self) -> &'static str {
        match self {
            Block::Summary => "</summary>",
            Block::Columns => "</div></div>",
        }
    }
}

/// Builds a placement around raw `Html` payload fragments, written as
/// adjacent raw nodes in order. One fragment is the whole payload.
type Place = fn(&[&str]) -> RenderNode;

fn inline_raws(fragments: &[&str]) -> Vec<RenderNode> {
    fragments.iter().map(|f| inline_raw(f)).collect()
}

/// Every placement that writes its content as an HTML block.
fn html_block_placements() -> Vec<(&'static str, Place, Block)> {
    vec![
        (
            "summary, direct",
            |p| disclosure(inline_raws(p)),
            Block::Summary,
        ),
        (
            "summary, inside strong",
            |p| disclosure(vec![RenderNode::strong(inline_raws(p))]),
            Block::Summary,
        ),
        (
            "summary, inside classed span",
            |p| disclosure(vec![RenderNode::span(vec!["note".into()], inline_raws(p))]),
            Block::Summary,
        ),
        (
            "summary, inside link",
            |p| disclosure(vec![RenderNode::link("u", None, inline_raws(p))]),
            Block::Summary,
        ),
        (
            "summary, inside unknown extension",
            |p| disclosure(vec![RenderNode::extended("shout", inline_raws(p), None)]),
            Block::Summary,
        ),
        (
            "columns, left",
            |p| columns(vec![para(inline_raws(p))], vec![para(vec![text("right")])]),
            Block::Columns,
        ),
        (
            "columns, right",
            |p| columns(vec![para(vec![text("left")])], vec![para(inline_raws(p))]),
            Block::Columns,
        ),
        (
            "columns, direct block HTML child",
            |p| {
                columns(
                    p.iter().map(|f| RenderNode::html(*f, true)).collect(),
                    vec![para(vec![text("right")])],
                )
            },
            Block::Columns,
        ),
    ]
}

/// Placements that never write their content as an HTML-lowered block.
fn clean_placements() -> Vec<(&'static str, Place)> {
    vec![
        ("concatenating root", |p| {
            let mut root = RenderNode::root(p.iter().map(|f| RenderNode::html(*f, true)).collect());
            root.attrs.set_sequence_join(SequenceJoin::None);
            root
        }),
        ("ordinary paragraph", |p| para(inline_raws(p))),
        ("disclosure body", |p| {
            RenderNode::disclosure(vec![text("s")], vec![para(inline_raws(p))], None)
        }),
        ("classed inline span", |p| {
            para(vec![RenderNode::span(vec!["note".into()], inline_raws(p))])
        }),
        ("link label", |p| {
            para(vec![RenderNode::link("u", None, inline_raws(p))])
        }),
    ]
}

/// The text of the first HTML block an independent CommonMark reader finds.
fn first_html_block(markdown: &str) -> String {
    let mut inside = false;
    let mut out = String::new();
    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            Event::Start(Tag::HtmlBlock) => inside = true,
            Event::End(TagEnd::HtmlBlock) => return out,
            Event::Html(html) if inside => out.push_str(&html),
            _ => {}
        }
    }
    panic!("no HTML block in {markdown:?}");
}

/// HTML input-stream preprocessing: CRLF and CR become LF before parsing.
fn preprocess(html: &str) -> String {
    html.replace("\r\n", "\n").replace('\r', "\n")
}

fn unfaithful_diagnostics(diagnostics: &[renderable::tree::Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::Lossy && d.message.contains(UNFAITHFUL))
        .count()
}

#[test]
fn faithful_raw_payloads_are_byte_identical_in_every_html_block_placement() {
    for (shape, payload, fit) in shapes() {
        if fit != Fit::Faithful {
            continue;
        }
        for (place, build, block) in html_block_placements() {
            let case = format!("{shape} / {place}");
            let node = build(&[payload]);
            for strictness in [RenderStrictness::Strict, RenderStrictness::Warn] {
                let rendered =
                    render_markdown_node(&node, &opts(MarkdownDialect::MarkdownPlus, strictness))
                        .unwrap_or_else(|e| panic!("{case}: {strictness:?} render failed: {e}"));
                let markdown = &rendered.output;
                assert!(
                    markdown.contains(payload),
                    "{case}: payload not byte-identical in {markdown:?}"
                );
                assert_eq!(
                    unfaithful_diagnostics(&rendered.diagnostics),
                    0,
                    "{case}: a faithful payload was reported"
                );
                let reading = first_html_block(markdown);
                assert!(
                    reading.contains(block.closer()),
                    "{case}: the HTML block ended before {}: {reading:?}",
                    block.closer()
                );
                assert!(
                    preprocess(&reading).contains(&preprocess(payload)),
                    "{case}: the reader's HTML block lost the payload: {reading:?}"
                );
            }
        }
    }
}

#[test]
fn unfaithful_raw_payloads_are_rejected_under_strict() {
    for (shape, payload, fit) in shapes() {
        if fit != Fit::Unfaithful {
            continue;
        }
        for (place, build, _) in html_block_placements() {
            let result = render_markdown_node(
                &build(&[payload]),
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
            );
            assert!(
                matches!(&result, Err(RenderError::LossyRejected { message }) if message.contains(UNFAITHFUL)),
                "{shape} / {place}: expected a lossy rejection, got {result:?}"
            );
        }
    }
}

#[test]
fn unfaithful_raw_payloads_are_reported_and_keep_the_block_open() {
    for (shape, payload, fit) in shapes() {
        if fit != Fit::Unfaithful {
            continue;
        }
        for (place, build, block) in html_block_placements() {
            let case = format!("{shape} / {place}");
            let node = build(&[payload]);
            let warn = render_markdown_node(
                &node,
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
            )
            .unwrap_or_else(|e| panic!("{case}: warn render failed: {e}"));
            assert_eq!(
                unfaithful_diagnostics(&warn.diagnostics),
                1,
                "{case}: expected one lossy diagnostic, got {:?}",
                warn.diagnostics
            );
            assert!(
                !warn.output.contains(payload),
                "{case}: the payload cannot be written unchanged without ending the block"
            );
            let reading = first_html_block(&warn.output);
            assert!(
                reading.contains(block.closer()),
                "{case}: the HTML block ended before {}: {reading:?}",
                block.closer()
            );

            let lossy = render_markdown_node(
                &node,
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Lossy),
            )
            .unwrap_or_else(|e| panic!("{case}: lossy render failed: {e}"));
            assert_eq!(lossy.output, warn.output, "{case}: lossy output differs");
            assert_eq!(unfaithful_diagnostics(&lossy.diagnostics), 0, "{case}");
        }
    }
}

#[test]
fn raw_payloads_outside_html_lowered_blocks_stay_byte_identical() {
    for (shape, payload, _) in shapes() {
        for (place, build) in clean_placements() {
            let case = format!("{shape} / {place}");
            let rendered = render_markdown_node(
                &build(&[payload]),
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
            )
            .unwrap_or_else(|e| panic!("{case}: render failed: {e}"));
            assert!(
                rendered.output.contains(payload),
                "{case}: payload not byte-identical in {:?}",
                rendered.output
            );
            assert_eq!(unfaithful_diagnostics(&rendered.diagnostics), 0, "{case}");
        }
        // Plain Markdown writes summaries and columns as Markdown, where raw
        // HTML keeps its own (documented, lossy) pass-through.
        for (place, build, _) in html_block_placements() {
            let case = format!("{shape} / plain Markdown {place}");
            let rendered = render_markdown_node(
                &build(&[payload]),
                &opts(MarkdownDialect::Markdown, RenderStrictness::Lossy),
            )
            .unwrap_or_else(|e| panic!("{case}: render failed: {e}"));
            assert!(
                rendered.output.contains(payload),
                "{case}: payload not byte-identical in {:?}",
                rendered.output
            );
        }
    }
}

#[test]
fn generated_line_endings_still_keep_the_block_open() {
    // A code block's blank line inside a column.
    let code = columns(
        vec![RenderNode::code(None, None, "a\n\nb")],
        vec![para(vec![text("right")])],
    );
    // Inline code with a blank line and a lone CR in a summary.
    let inline = disclosure(vec![RenderNode::inline_code("a\n\nb\rc")]);
    for (node, expected, block) in [
        (&code, "a&#10;\nb", Block::Columns),
        (&inline, "a&#10;\nb&#13;c", Block::Summary),
    ] {
        let rendered = render_markdown_node(
            node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        )
        .expect("generated text is always representable");
        assert!(
            rendered.output.contains(expected),
            "{expected:?} missing from {:?}",
            rendered.output
        );
        assert!(first_html_block(&rendered.output).contains(block.closer()));
    }
}

#[test]
fn a_generated_line_ending_beside_a_raw_one_takes_the_encoding() {
    // The blank line between the payload's trailing line feed and the
    // text's leading one: the generated ending is encoded and the payload is
    // kept, in either order.
    for (children, payload, expected) in [
        (
            vec![inline_raw("<script>a;\n"), text("\nmore")],
            "<script>a;\n",
            "<script>a;\n&#10;more",
        ),
        (
            vec![text("more\n"), inline_raw("\n<script>a;</script>")],
            "\n<script>a;</script>",
            "more&#10;\n<script>a;</script>",
        ),
    ] {
        let rendered = render_markdown_node(
            &disclosure(children),
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        )
        .expect("the generated ending can hold the block open");
        assert!(rendered.output.contains(payload));
        assert!(
            rendered.output.contains(expected),
            "{expected:?} missing from {:?}",
            rendered.output
        );
        assert!(first_html_block(&rendered.output).contains("</summary>"));
    }
}

/// Payloads split across two adjacent raw nodes, by name. A reader sees the
/// concatenation, so a CRLF whose CR and LF land in different nodes is still
/// one line ending.
fn split_shapes() -> Vec<(&'static str, &'static str, Fit)> {
    use Fit::{Faithful, Unfaithful};
    vec![
        (
            "two spans, CRLF",
            "<span>a</span>\r\n<span>b</span>",
            Faithful,
        ),
        (
            "script, CRLF",
            "<script>const a = 1;\r\nconsole.log(a);</script>",
            Faithful,
        ),
        (
            "unquoted attributes, CRLF",
            "<span data-a=x\r\ndata-b=y>t</span>",
            Faithful,
        ),
        (
            "script, single LF",
            "<script>const a = 1;\nconsole.log(a);</script>",
            Faithful,
        ),
        (
            "unquoted attributes, lone CR",
            "<span data-a=x\rdata-b=y>t</span>",
            Faithful,
        ),
        (
            "span text, LF blank line",
            "<span>a\n\nb</span>",
            Unfaithful,
        ),
        (
            "span text, CR blank line",
            "<span>a\r\rb</span>",
            Unfaithful,
        ),
        (
            "span text, CRLF blank line",
            "<span>a\r\n\r\nb</span>",
            Unfaithful,
        ),
        (
            "span text, whitespace-only line",
            "<span>a\r\n \t\r\nb</span>",
            Unfaithful,
        ),
    ]
}

/// Every way to write `payload` as two adjacent fragments, empty ones
/// included.
fn splits(payload: &str) -> impl Iterator<Item = [&str; 2]> {
    (0..=payload.len())
        .filter(|&at| payload.is_char_boundary(at))
        .map(move |at| [&payload[..at], &payload[at..]])
}

#[test]
fn split_faithful_payloads_render_as_the_whole_payload_in_every_html_block_placement() {
    for (shape, payload, fit) in split_shapes() {
        if fit != Fit::Faithful {
            continue;
        }
        for (place, build, block) in html_block_placements() {
            let strict = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict);
            let whole = render_markdown_node(&build(&[payload]), &strict)
                .unwrap_or_else(|e| panic!("{shape} / {place}: whole payload failed: {e}"));
            for fragments in splits(payload) {
                let case = format!("{shape} / {place} / split {fragments:?}");
                let split = render_markdown_node(&build(&fragments), &strict)
                    .unwrap_or_else(|e| panic!("{case}: strict render failed: {e}"));
                assert_eq!(split.output, whole.output, "{case}");
                assert!(split.output.contains(payload), "{case}: payload changed");
                assert_eq!(unfaithful_diagnostics(&split.diagnostics), 0, "{case}");
                assert!(
                    first_html_block(&split.output).contains(block.closer()),
                    "{case}: the HTML block ended early"
                );
            }
        }
    }
}

#[test]
fn split_blank_lines_stay_rejected_under_strict_and_reported_under_warn() {
    for (shape, payload, fit) in split_shapes() {
        if fit != Fit::Unfaithful {
            continue;
        }
        for (place, build, block) in html_block_placements() {
            for fragments in splits(payload) {
                let case = format!("{shape} / {place} / split {fragments:?}");
                let node = build(&fragments);
                let strict = render_markdown_node(
                    &node,
                    &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
                );
                assert!(
                    matches!(&strict, Err(RenderError::LossyRejected { message }) if message.contains(UNFAITHFUL)),
                    "{case}: expected a lossy rejection, got {strict:?}"
                );
                let warn = render_markdown_node(
                    &node,
                    &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
                )
                .unwrap_or_else(|e| panic!("{case}: warn render failed: {e}"));
                assert!(
                    unfaithful_diagnostics(&warn.diagnostics) >= 1,
                    "{case}: no lossy diagnostic in {:?}",
                    warn.diagnostics
                );
                assert!(
                    first_html_block(&warn.output).contains(block.closer()),
                    "{case}: the HTML block ended early"
                );
            }
        }
    }
}

#[test]
fn split_payloads_outside_html_lowered_blocks_stay_byte_identical() {
    for (shape, payload, _) in split_shapes() {
        for fragments in splits(payload) {
            for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
                for (place, build) in clean_placements() {
                    let case = format!("{shape} / {place} / {dialect:?} / split {fragments:?}");
                    let rendered = render_markdown_node(
                        &build(&fragments),
                        &opts(dialect, RenderStrictness::Warn),
                    )
                    .unwrap_or_else(|e| panic!("{case}: render failed: {e}"));
                    assert!(
                        rendered.output.contains(payload),
                        "{case}: payload not byte-identical in {:?}",
                        rendered.output
                    );
                    assert_eq!(unfaithful_diagnostics(&rendered.diagnostics), 0, "{case}");
                }
            }
            // Plain Markdown writes a column's block children as separate
            // blocks, so only its inline placements concatenate.
            for (place, build, _) in html_block_placements()
                .into_iter()
                .filter(|(place, _, _)| !place.contains("block HTML child"))
            {
                let case = format!("{shape} / plain Markdown {place} / split {fragments:?}");
                let rendered = render_markdown_node(
                    &build(&fragments),
                    &opts(MarkdownDialect::Markdown, RenderStrictness::Lossy),
                )
                .unwrap_or_else(|e| panic!("{case}: render failed: {e}"));
                assert!(
                    rendered.output.contains(payload),
                    "{case}: payload not byte-identical in {:?}",
                    rendered.output
                );
            }
            let allow = BrowserRenderOptions {
                raw_html: RawHtmlPolicy::Allow,
                ..BrowserRenderOptions::default()
            };
            let builds = clean_placements()
                .into_iter()
                .chain(html_block_placements().into_iter().map(|(p, b, _)| (p, b)));
            for (place, build) in builds {
                let case = format!("{shape} / browser {place} / split {fragments:?}");
                let html = render_browser_node(&build(&fragments), &allow)
                    .unwrap_or_else(|e| panic!("{case}: browser render failed: {e}"))
                    .output
                    .render();
                assert!(
                    html.contains(payload),
                    "{case}: payload not byte-identical in {html:?}"
                );
            }
        }
    }
}

/// How a summary with raw and generated pieces around a line ending must
/// render under `Strict`.
enum Expect {
    /// Succeeds, keeping every raw piece byte for byte and writing this.
    Writes(&'static str),
    /// Rejected: only raw bytes could hold the block open.
    Rejected,
}

#[test]
fn a_crlf_split_between_raw_and_generated_html_is_one_line_ending() {
    let raw = |p: &'static str| inline_raw(p);
    let cases: Vec<(&str, Vec<RenderNode>, Vec<&str>, Expect)> = vec![
        (
            "raw CR, generated LF",
            vec![raw("<i>a</i>\r"), text("\nb")],
            vec!["<i>a</i>\r"],
            Expect::Writes("<i>a</i>\r\nb"),
        ),
        (
            // A generated CR is always a reference; the raw LF stays the
            // one line ending.
            "generated CR, raw LF",
            vec![text("a\r"), raw("\n<i>b</i>")],
            vec!["\n<i>b</i>"],
            Expect::Writes("a&#13;\n<i>b</i>"),
        ),
        (
            "generated CR, generated LF",
            vec![text("a\r"), text("\nb")],
            vec![],
            Expect::Writes("a&#13;\nb"),
        ),
        (
            "raw CR, raw LF",
            vec![raw("<i>a</i>\r"), raw("\n<i>b</i>")],
            vec!["<i>a</i>\r", "\n<i>b</i>"],
            Expect::Writes("<i>a</i>\r\n<i>b</i>"),
        ),
        (
            // The blank line after a mixed ending: only the generated LF
            // after it can be encoded.
            "raw CR, generated LF, then a generated blank line",
            vec![raw("<i>a</i>\r"), text("\n\nb")],
            vec!["<i>a</i>\r"],
            Expect::Writes("<i>a</i>\r\n&#10;b"),
        ),
        (
            "generated blank line, then generated CR and raw LF",
            vec![text("a\n\r"), raw("\n<i>b</i>")],
            vec!["\n<i>b</i>"],
            Expect::Writes("a\n&#13;\n<i>b</i>"),
        ),
        (
            "generated LF blank line before a raw CRLF",
            vec![text("a\n"), raw("\r\n<i>b</i>")],
            vec!["\r\n<i>b</i>"],
            Expect::Writes("a&#10;\r\n<i>b</i>"),
        ),
        (
            // Encoding the generated LF alone would leave the raw CR as a
            // line ending, so neither side can hold the block open.
            "mixed ending, then a raw LF",
            vec![raw("<i>a</i>\r"), text("\n"), raw("\n<i>b</i>")],
            vec![],
            Expect::Rejected,
        ),
        (
            "raw CRLF across nodes, then a raw blank line",
            vec![raw("<i>a</i>\r"), raw("\n\n<i>b</i>")],
            vec![],
            Expect::Rejected,
        ),
    ];
    for (case, children, kept, expect) in cases {
        let node = disclosure(children);
        let strict = render_markdown_node(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        );
        match expect {
            Expect::Writes(expected) => {
                let rendered = strict.unwrap_or_else(|e| panic!("{case}: strict failed: {e}"));
                for piece in kept {
                    assert!(rendered.output.contains(piece), "{case}: {piece:?} changed");
                }
                assert!(
                    rendered.output.contains(expected),
                    "{case}: {expected:?} missing from {:?}",
                    rendered.output
                );
                assert!(
                    first_html_block(&rendered.output).contains("</summary>"),
                    "{case}"
                );
            }
            Expect::Rejected => {
                assert!(
                    matches!(&strict, Err(RenderError::LossyRejected { message }) if message.contains(UNFAITHFUL)),
                    "{case}: expected a lossy rejection, got {strict:?}"
                );
                let warn = render_markdown_node(
                    &node,
                    &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
                )
                .unwrap_or_else(|e| panic!("{case}: warn failed: {e}"));
                assert!(unfaithful_diagnostics(&warn.diagnostics) >= 1, "{case}");
                assert!(
                    first_html_block(&warn.output).contains("</summary>"),
                    "{case}"
                );
            }
        }
    }
}

/// Every block a CommonMark reader finds in `markdown`, by kind.
fn block_starts(markdown: &str) -> Vec<String> {
    Parser::new_ext(markdown, Options::all())
        .filter_map(|event| match event {
            Event::Start(
                tag @ (Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::BlockQuote(_)
                | Tag::List(_)
                | Tag::HtmlBlock),
            ) => Some(format!("{tag:?}")),
            Event::Rule => Some("Rule".to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn text_after_a_raw_carriage_return_is_protected_as_a_line_start() {
    // A reader takes a raw payload's carriage return as a line ending, with
    // or without a line feed after it from the next node, so text there
    // starts a line and must not read as a block start.
    let raw = |p: &'static str| inline_raw(p);
    for marker in ["# x", "> x", "- x", "1. x", "***"] {
        let with_lf = format!("\n{marker}");
        let sequences: Vec<(&str, Vec<RenderNode>)> = vec![
            ("lone CR", vec![raw("<i>a</i>\r"), text(marker)]),
            ("CR | LF", vec![raw("<i>a</i>\r"), text(&with_lf)]),
            (
                "CR in its own node | LF",
                vec![raw("<i>a</i>"), raw("\r"), text(&with_lf)],
            ),
        ];
        for (shape, children) in sequences {
            let placements: Vec<(&str, RenderNode, &str)> = vec![
                ("paragraph", para(children.clone()), "Paragraph"),
                (
                    "link label",
                    para(vec![RenderNode::link("u", None, children.clone())]),
                    "Paragraph",
                ),
            ];
            for (place, node, only) in placements {
                for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
                    let case = format!("{marker:?} / {shape} / {place} / {dialect:?}");
                    let rendered =
                        render_markdown_node(&node, &opts(dialect, RenderStrictness::Lossy))
                            .unwrap_or_else(|e| panic!("{case}: render failed: {e}"));
                    assert!(
                        rendered.output.contains("<i>a</i>\r"),
                        "{case}: raw payload changed in {:?}",
                        rendered.output
                    );
                    assert_eq!(
                        block_starts(&rendered.output),
                        vec![only.to_string()],
                        "{case}: {:?}",
                        rendered.output
                    );
                }
            }
        }
    }
}

#[test]
fn a_crlf_split_between_raw_markup_and_text_is_one_line_ending_in_a_link_label() {
    // The raw CR and the text's LF are one line ending; the blank line the
    // text's second LF would leave is kept out of the label.
    let node = para(vec![RenderNode::link(
        "u",
        None,
        vec![inline_raw("<i>a</i>\r"), text("\n\nb")],
    )]);
    for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
        let rendered = render_markdown_node(&node, &opts(dialect, RenderStrictness::Lossy))
            .unwrap_or_else(|e| panic!("{dialect:?}: render failed: {e}"));
        assert!(
            rendered.output.starts_with("[<i>a</i>\r\n"),
            "{dialect:?}: {:?}",
            rendered.output
        );
        assert_eq!(
            block_starts(&rendered.output),
            vec!["Paragraph".to_string()],
            "{dialect:?}: {:?}",
            rendered.output
        );
    }
}

//! A raw HTML node holding only comments has no visible content.
//!
//! The Markdown writer separates two touching code spans with `<!-- -->`, so a
//! reader folds a comment node between them. The browser renderer must show
//! nothing for it under every raw-HTML policy and strictness (writing the
//! comment itself under `Allow`), and the Markdown writer must not report it
//! as unportable. Real tags, comments followed by other content, and
//! comment-shaped text in code, text, or attribute values keep their
//! existing treatment.

use renderable::tree::{
    BrowserRenderOptions, Document, DocumentMetadata, MarkdownDialect, MarkdownRenderOptions,
    RawHtmlPolicy, RenderError, RenderNode, RenderStrictness, SourceRegistry, is_html_comment_only,
    render_browser_document_body, render_browser_node, render_markdown_node,
};

const STRICTNESS: [RenderStrictness; 3] = [
    RenderStrictness::Strict,
    RenderStrictness::Warn,
    RenderStrictness::Lossy,
];

fn browser(policy: RawHtmlPolicy, strictness: RenderStrictness) -> BrowserRenderOptions {
    BrowserRenderOptions {
        strictness,
        raw_html: policy,
        ..BrowserRenderOptions::default()
    }
}

/// The paragraph a reader folds from `` `a`<!-- -->`b` ``.
fn separated(html: RenderNode) -> RenderNode {
    RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::inline_code("a"),
        html,
        RenderNode::inline_code("b"),
    ])])
}

/// Renders `root` through the tree writer and the streaming document
/// writer, returning both outputs and their diagnostic counts.
fn render_both(
    root: &RenderNode,
    opts: &BrowserRenderOptions,
) -> Result<[(String, usize); 2], RenderError> {
    let tree = render_browser_node(root, opts)?;
    let doc = Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: root.clone(),
    };
    let streamed = render_browser_document_body(&doc, opts)?;
    Ok([
        (tree.output.render(), tree.diagnostics.len()),
        (streamed.output.body, streamed.diagnostics.len()),
    ])
}

#[test]
fn comment_only_values_are_recognized() {
    for value in [
        "<!-- -->",
        "<!---->",
        "<!-->",
        "<!--->",
        "<!-- a - b -- c -->",
        "  <!-- a -->\n<!-- b -->\n",
        "<!-- line\nbreak -->",
    ] {
        assert!(is_html_comment_only(value), "{value:?}");
    }
    for value in [
        "",
        "   ",
        "<!-- a --> b",
        "x <!-- a -->",
        "<!-- unclosed",
        "<b><!-- a --></b>",
        "<span title=\"<!-- -->\">",
        "<!DOCTYPE html>",
        "<![CDATA[x]]>",
    ] {
        assert!(!is_html_comment_only(value), "{value:?}");
    }
}

/// Escape (the default) and Reject show nothing between the two code
/// values, record no diagnostic, and never reject, in every strictness.
#[test]
fn comment_renders_nothing_under_escape_and_reject() {
    for block in [false, true] {
        let root = separated(RenderNode::html("<!-- -->", block));
        for policy in [RawHtmlPolicy::Escape, RawHtmlPolicy::Reject] {
            for strictness in STRICTNESS {
                let label = format!("{policy:?} {strictness:?} block={block}");
                let outputs = render_both(&root, &browser(policy, strictness))
                    .unwrap_or_else(|error| panic!("{label}: {error}"));
                for (output, diagnostics) in outputs {
                    assert!(
                        output.contains("<code>a</code><code>b</code>"),
                        "{label}: {output}"
                    );
                    assert!(!output.contains("&lt;!--") && !output.contains("<!--"), "{label}: {output}");
                    assert_eq!(diagnostics, 0, "{label}: {output}");
                }
            }
        }
    }
}

/// Allow writes the comment itself, which a browser shows as nothing.
#[test]
fn comment_is_written_verbatim_under_allow() {
    let root = separated(RenderNode::html("<!-- -->", false));
    for strictness in STRICTNESS {
        for (output, diagnostics) in
            render_both(&root, &browser(RawHtmlPolicy::Allow, strictness)).expect("render")
        {
            assert!(
                output.contains("<code>a</code><!-- --><code>b</code>"),
                "{strictness:?}: {output}"
            );
            assert_eq!(diagnostics, 0, "{strictness:?}");
        }
    }
}

/// A block of several comments is still only comments.
#[test]
fn comment_block_renders_nothing_under_the_default_policy() {
    let root = RenderNode::root(vec![
        RenderNode::paragraph(vec![RenderNode::text("before")]),
        RenderNode::html("<!-- a -->\n<!-- b -->\n", true),
        RenderNode::paragraph(vec![RenderNode::text("after")]),
    ]);
    for strictness in STRICTNESS {
        for (output, diagnostics) in
            render_both(&root, &browser(RawHtmlPolicy::Escape, strictness)).expect("render")
        {
            assert!(!output.contains("&lt;!--"), "{strictness:?}: {output}");
            assert!(output.contains("before") && output.contains("after"), "{output}");
            assert_eq!(diagnostics, 0, "{strictness:?}");
        }
    }
}

/// Raw HTML that is more than comments keeps the existing policy: escaped
/// with a diagnostic under Escape, rejected under Strict Reject.
#[test]
fn other_raw_html_keeps_its_policy() {
    for value in ["<b>x</b>", "<!-- a --> b", "<span title=\"<!-- -->\">"] {
        let root = separated(RenderNode::html(value, false));
        let escaped = render_both(&root, &browser(RawHtmlPolicy::Escape, RenderStrictness::Warn))
            .expect("render");
        for (output, diagnostics) in escaped {
            assert!(output.contains("&lt;"), "{value:?}: {output}");
            assert_eq!(diagnostics, 1, "{value:?}");
        }
        let rejected = render_browser_node(&root, &browser(RawHtmlPolicy::Reject, RenderStrictness::Strict));
        assert!(
            matches!(rejected, Err(RenderError::LossyRejected { .. })),
            "{value:?}: {:?}",
            rejected.map(|rendered| rendered.output.render())
        );
    }
}

/// Comment-shaped code and text are content, not raw HTML.
#[test]
fn comment_shaped_code_and_text_stay_visible() {
    let root = RenderNode::root(vec![
        RenderNode::paragraph(vec![
            RenderNode::inline_code("<!-- -->"),
            RenderNode::text(" and <!-- x -->"),
        ]),
        RenderNode::code(None, None, "<!-- block -->"),
    ]);
    for policy in [RawHtmlPolicy::Allow, RawHtmlPolicy::Escape, RawHtmlPolicy::Reject] {
        for (output, _) in render_both(&root, &browser(policy, RenderStrictness::Strict)).expect("render") {
            assert!(output.contains("<code>&lt;!-- --&gt;</code>"), "{policy:?}: {output}");
            assert!(output.contains("and &lt;!-- x --&gt;"), "{policy:?}: {output}");
            assert!(output.contains("&lt;!-- block --&gt;"), "{policy:?}: {output}");
        }
    }
}

/// Plain Markdown reports raw HTML as unportable, but not a comment: every
/// CommonMark reader shows it as nothing, as it does the writer's own
/// separator.
#[test]
fn plain_markdown_writes_comments_without_a_diagnostic() {
    let comment = separated(RenderNode::html("<!-- -->", false));
    for strictness in STRICTNESS {
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::Markdown,
            strictness,
            style: None,
        };
        let rendered = render_markdown_node(&comment, &opts).expect("comment is portable");
        assert_eq!(rendered.output, "`a`<!-- -->`b`", "{strictness:?}");
        assert!(rendered.diagnostics.is_empty(), "{strictness:?}");
    }
    let tag = separated(RenderNode::html("<b>x</b>", false));
    let strict = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Strict,
        style: None,
    };
    assert!(matches!(
        render_markdown_node(&tag, &strict),
        Err(RenderError::LossyRejected { .. })
    ));
}

//! Golden round-trip integration tests for the render-tree pipeline.
//!
//! Each test folds a Markdown fixture (`text → tree`) via darkmatter's
//! [`fold_markdown_to_document`], asserts key structural properties of the
//! folded tree, and renders it back to Markdown (`tree → Markdown`) via
//! [`render_markdown_document`].
//!
//! Markdown rendering targets **semantic stability, not byte-identical**
//! source preservation: the rendered output parses back to an equivalent
//! tree, but whitespace and delimiter choices are normalized. The rendered
//! Markdown is therefore pinned with `insta` snapshots rather than compared
//! against the source fixture.
//!
//! The fixtures live in `tests/fixtures/render_tree/`.

use darkmatter::markdown::render_tree::fold_markdown_to_document;
use renderable::tree::{
    render_markdown_document, ColumnAlign, DiagnosticKind, Document, MarkdownDialect,
    MarkdownRenderOptions, NodeKind, Provenance, RenderError, RenderNode, RenderStrictness,
    SourceDescriptor,
};

/// Reads a fixture from `tests/fixtures/render_tree/`.
fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/render_tree/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {path}: {err}"))
}

/// Folds a fixture from a virtual source named after the fixture file.
fn fold_fixture(name: &str) -> (Document, Vec<renderable::tree::Diagnostic>) {
    let source = SourceDescriptor::Virtual { name: name.into() };
    fold_markdown_to_document(source, &fixture(name))
}

/// Renders a document to Markdown with default options.
fn render(doc: &Document) -> renderable::tree::Rendered<String> {
    render_markdown_document(doc, &MarkdownRenderOptions::default())
        .expect("default-options render must succeed")
}

/// Returns the document root's child nodes.
fn roots(doc: &Document) -> &[RenderNode] {
    doc.root.children()
}

#[test]
fn render_tree_paragraph_round_trip() {
    let (doc, diags) = fold_fixture("paragraph.md");
    assert!(diags.is_empty(), "clean fixture must fold without diagnostics");

    let children = roots(&doc);
    assert_eq!(children.len(), 1);
    assert!(matches!(children[0].kind, NodeKind::Paragraph { .. }));

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("paragraph", rendered.output);
}

#[test]
fn render_tree_headings_round_trip() {
    let (doc, diags) = fold_fixture("headings.md");
    assert!(diags.is_empty());

    let children = roots(&doc);
    assert_eq!(children.len(), 3);
    let depths: Vec<u8> = children
        .iter()
        .map(|node| match &node.kind {
            NodeKind::Heading { depth, .. } => depth.get(),
            other => panic!("expected heading, got {other:?}"),
        })
        .collect();
    assert_eq!(depths, vec![1, 2, 3]);
    // The fold derives a slug id from the heading text.
    assert_eq!(children[0].attrs.id.as_deref(), Some("top-level"));

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("headings", rendered.output);
}

#[test]
fn render_tree_inline_styles_round_trip() {
    let (doc, diags) = fold_fixture("inline_styles.md");
    assert!(diags.is_empty());

    let para = &roots(&doc)[0];
    assert!(matches!(para.kind, NodeKind::Paragraph { .. }));
    let kinds: Vec<&NodeKind> = para.children().iter().map(|node| &node.kind).collect();
    assert!(kinds.iter().any(|k| matches!(k, NodeKind::Emphasis { .. })));
    assert!(kinds.iter().any(|k| matches!(k, NodeKind::Strong { .. })));
    assert!(kinds.iter().any(|k| matches!(k, NodeKind::Delete { .. })));

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("inline_styles", rendered.output);
}

#[test]
fn render_tree_lists_round_trip() {
    let (doc, diags) = fold_fixture("lists.md");
    assert!(diags.is_empty());

    let children = roots(&doc);
    assert_eq!(children.len(), 2);
    match &children[0].kind {
        NodeKind::List { ordered, start, children } => {
            assert!(!ordered);
            assert_eq!(*start, None);
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected unordered list, got {other:?}"),
    }
    match &children[1].kind {
        NodeKind::List { ordered, start, children } => {
            assert!(ordered);
            assert_eq!(*start, Some(1));
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected ordered list, got {other:?}"),
    }

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("lists", rendered.output);
}

#[test]
fn render_tree_task_list_round_trip() {
    let (doc, diags) = fold_fixture("task_list.md");
    assert!(diags.is_empty());

    let list = &roots(&doc)[0];
    let items = list.children();
    assert_eq!(items.len(), 2);
    let checked: Vec<Option<bool>> = items
        .iter()
        .map(|item| match &item.kind {
            NodeKind::ListItem { checked, .. } => *checked,
            other => panic!("expected list item, got {other:?}"),
        })
        .collect();
    assert_eq!(checked, vec![Some(true), Some(false)]);

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("task_list", rendered.output);
}

#[test]
fn render_tree_code_block_round_trip() {
    let (doc, diags) = fold_fixture("code_block.md");
    assert!(diags.is_empty());

    let code = &roots(&doc)[0];
    match &code.kind {
        NodeKind::Code { lang, meta, value } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(meta.as_deref(), Some("ignore"));
            assert!(value.contains("fn main()"));
        }
        other => panic!("expected code block, got {other:?}"),
    }

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("code_block", rendered.output);
}

#[test]
fn render_tree_table_round_trip() {
    let (doc, diags) = fold_fixture("table.md");
    assert!(diags.is_empty());

    let table = &roots(&doc)[0];
    match &table.kind {
        NodeKind::Table { align, children } => {
            assert_eq!(align, &[ColumnAlign::Left, ColumnAlign::Right]);
            // Header row plus two body rows.
            assert_eq!(children.len(), 3);
            for row in children {
                assert!(matches!(row.kind, NodeKind::TableRow { .. }));
            }
        }
        other => panic!("expected table, got {other:?}"),
    }

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("table", rendered.output);
}

#[test]
fn render_tree_links_images_round_trip() {
    let (doc, diags) = fold_fixture("links_images.md");
    assert!(diags.is_empty());

    let para = &roots(&doc)[0];
    let link = para
        .children()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Link { .. }))
        .expect("fixture has a link");
    match &link.kind {
        NodeKind::Link { url, title, .. } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(title.as_deref(), Some("Example"));
        }
        _ => unreachable!(),
    }
    let image = para
        .children()
        .iter()
        .find(|node| matches!(node.kind, NodeKind::Image { .. }))
        .expect("fixture has an image");
    match &image.kind {
        NodeKind::Image { url, alt, .. } => {
            assert_eq!(url, "image.png");
            assert_eq!(alt, "alt text");
        }
        _ => unreachable!(),
    }

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("links_images", rendered.output);
}

#[test]
fn render_tree_html_round_trip() {
    let (doc, diags) = fold_fixture("html.md");
    assert!(diags.is_empty(), "the fold itself raises no diagnostics for raw HTML");

    // The HtmlBlock wrapper is spliced away: its `Html` lines land at the
    // document root, and inline HTML lands inside the trailing paragraph.
    let children = roots(&doc);
    assert!(children
        .iter()
        .any(|node| matches!(node.kind, NodeKind::Html { block: true, .. })));
    let last_para = children
        .iter()
        .rev()
        .find(|node| matches!(node.kind, NodeKind::Paragraph { .. }))
        .expect("fixture ends with a paragraph");
    assert!(last_para
        .children()
        .iter()
        .any(|node| matches!(node.kind, NodeKind::Html { block: false, .. })));

    // Rendering raw HTML under the default plain-Markdown dialect is lossy:
    // the renderer emits the raw value but records a `Lossy` diagnostic per
    // HTML node.
    let rendered = render(&doc);
    assert!(
        !rendered.diagnostics.is_empty(),
        "raw HTML must be diagnosed as lossy under plain Markdown"
    );
    assert!(rendered
        .diagnostics
        .iter()
        .all(|diag| diag.kind == DiagnosticKind::Lossy));
    insta::assert_snapshot!("html", rendered.output);
}

/// The `unsupported_math_or_definition.md` fixture documents a **known gap**:
/// under the Milestone 1 fold options (`ENABLE_TABLES | ENABLE_STRIKETHROUGH
/// | ENABLE_TASKLISTS`) neither math spans nor definition lists are enabled
/// in `pulldown-cmark`, so neither construct emits a math/definition event.
/// Both fold to ordinary `Text` (and `SoftBreak`) nodes — there is **no**
/// `Unsupported` node and **no** diagnostic.
///
/// The fold's `Unsupported` path is genuinely unreachable from Markdown under
/// these options; it is only reached by parser events the options never emit.
/// This test therefore asserts the *current documented behavior* (plain text,
/// no diagnostics) rather than faking an `Unsupported` node. The renderer-side
/// `Unsupported` strictness behavior is exercised separately by
/// [`unsupported_node_strictness`], which builds a tree containing an
/// `Unsupported` node directly.
#[test]
fn render_tree_unsupported_math_or_definition_folds_to_plain_text() {
    let (doc, diags) = fold_fixture("unsupported_math_or_definition.md");
    assert!(
        diags.is_empty(),
        "math/definition syntax is plain text under the Milestone 1 options"
    );

    // No node in the folded tree is `Unsupported`.
    fn has_unsupported(node: &RenderNode) -> bool {
        matches!(node.kind, NodeKind::Unsupported { .. })
            || node.children().iter().any(has_unsupported)
    }
    assert!(
        !has_unsupported(&doc.root),
        "no Unsupported node is produced from Markdown under the Milestone 1 options"
    );
    // The `$...$` math text survives verbatim as a `Text` leaf somewhere.
    fn collected_text(node: &RenderNode, out: &mut String) {
        if let NodeKind::Text { value } = &node.kind {
            out.push_str(value);
        }
        for child in node.children() {
            collected_text(child, out);
        }
    }
    let mut text = String::new();
    collected_text(&doc.root, &mut text);
    assert!(
        text.contains("$x^2 + y^2 = z^2$"),
        "math span is preserved as literal text"
    );

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("unsupported_math_or_definition", rendered.output);
}

/// The `frontmatter.md` fixture documents another **known gap**: the
/// Milestone 1 fold does *not* extract frontmatter. A `---`-delimited block at
/// the top of the input is parsed by `pulldown-cmark` as ordinary Markdown — a
/// thematic break followed by a setext/ATX heading — and
/// [`renderable::tree::DocumentMetadata::frontmatter`] stays `None`.
///
/// Frontmatter folding is deferred to a later phase; this test pins the
/// current behavior so the future change is a deliberate, visible update.
#[test]
fn render_tree_frontmatter_is_not_extracted() {
    let (doc, diags) = fold_fixture("frontmatter.md");
    assert!(diags.is_empty());

    // Frontmatter is NOT extracted by the Milestone 1 fold.
    assert!(
        doc.metadata.frontmatter.is_none(),
        "frontmatter folding is deferred; metadata.frontmatter must stay None"
    );

    // The leading `---` is parsed as a thematic break, not a frontmatter fence.
    let children = roots(&doc);
    assert!(
        matches!(children[0].kind, NodeKind::ThematicBreak),
        "the `---` fence folds to a ThematicBreak under the current behavior"
    );

    let rendered = render(&doc);
    assert!(rendered.diagnostics.is_empty());
    insta::assert_snapshot!("frontmatter", rendered.output);
}

/// Renders a tree containing an `Unsupported` node under each
/// [`RenderStrictness`].
///
/// Because no Markdown construct folds to `Unsupported` under the Milestone 1
/// options (see [`render_tree_unsupported_math_or_definition_folds_to_plain_text`]),
/// the tree is built directly so the renderer's strictness handling is
/// genuinely exercised.
#[test]
fn unsupported_node_strictness() {
    let tree = RenderNode::root(vec![
        RenderNode::paragraph(vec![RenderNode::text("before")]),
        RenderNode::unsupported("math expression"),
    ]);

    // Strict: an `Unsupported` node escalates to a validation error.
    let strict = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Strict,
        style: None,
    };
    let strict_result = renderable::tree::render_markdown_node(&tree, &strict);
    assert!(
        matches!(strict_result, Err(RenderError::InvalidTree { .. })),
        "Strict mode must reject an Unsupported node"
    );

    // Warn: rendering succeeds and records diagnostics.
    let warn = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Warn,
        style: None,
    };
    let warn_rendered =
        renderable::tree::render_markdown_node(&tree, &warn).expect("Warn mode must succeed");
    assert!(
        !warn_rendered.diagnostics.is_empty(),
        "Warn mode must record diagnostics for the Unsupported node"
    );

    // Lossy: rendering succeeds and records no diagnostics.
    let lossy = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    let lossy_rendered =
        renderable::tree::render_markdown_node(&tree, &lossy).expect("Lossy mode must succeed");
    assert!(
        lossy_rendered.diagnostics.is_empty(),
        "Lossy mode must record no diagnostics"
    );
}

/// Renders the lossy `html.md` fixture under each [`RenderStrictness`].
///
/// Raw HTML is the construct that genuinely round-trips through the fold and
/// then hits the renderer's lossy path under the default plain-Markdown
/// dialect: `Strict` rejects it, while `Warn`/`Lossy` accept it.
#[test]
fn lossy_html_fixture_strictness() {
    let (doc, _) = fold_fixture("html.md");

    let strict = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Strict,
        style: None,
    };
    let strict_result = render_markdown_document(&doc, &strict);
    assert!(
        matches!(strict_result, Err(RenderError::LossyRejected { .. })),
        "Strict mode must reject raw HTML under plain Markdown, got {strict_result:?}"
    );

    let warn = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Warn,
        style: None,
    };
    let warn_rendered =
        render_markdown_document(&doc, &warn).expect("Warn mode must succeed");
    assert!(
        !warn_rendered.diagnostics.is_empty(),
        "Warn mode must record lossy diagnostics for raw HTML"
    );

    let lossy = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    let lossy_rendered =
        render_markdown_document(&doc, &lossy).expect("Lossy mode must succeed");
    assert!(
        lossy_rendered.diagnostics.is_empty(),
        "Lossy mode must record no diagnostics"
    );
}

/// Serializes a representative folded [`Document`] to pretty JSON and snapshots
/// it.
///
/// The fixture is chosen to cover a wide slice of the public JSON surface in a
/// single document: several `NodeKind` variants, `SourceSpan`s with `Parsed`
/// provenance carrying byte ranges, `NodeAttrs` (the heading slug `id`), and a
/// populated `SourceRegistry`. This pins the serde representation so a change
/// to the wire format is a deliberate, reviewed update.
#[test]
fn render_tree_document_json_surface() {
    let source = SourceDescriptor::File {
        path: "docs/sample.md".into(),
    };
    let input = concat!(
        "# Sample Heading\n",
        "\n",
        "A paragraph with **strong** text and a [link](https://example.com).\n",
        "\n",
        "- [x] a task\n",
        "\n",
        "```rust\n",
        "let answer = 42;\n",
        "```\n",
    );
    let (doc, diags) = fold_markdown_to_document(source, input);
    assert!(diags.is_empty());

    // Spot-check the provenance surface the snapshot pins.
    let heading = &doc.root.children()[0];
    assert_eq!(heading.span.provenance, Provenance::Parsed);
    assert!(heading.span.location.is_some());
    assert_eq!(heading.attrs.id.as_deref(), Some("sample-heading"));

    let json = serde_json::to_string_pretty(&doc).expect("document must serialize");
    insta::assert_snapshot!("document_json_surface", json);
}

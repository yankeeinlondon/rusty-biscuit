//! The browser fold's output width.
//!
//! `Width::Columns(n)` caps prose blocks at `n` `ch` through one page-level
//! rule; `Width::Source` and `Width::Unlimited` add nothing, so their pages
//! are byte-identical to the default.

use renderable::output::Width;
use renderable::tree::{
    BrowserRenderOptions, ColumnAlign, Document, RenderNode, render_browser_document,
    render_browser_document_body, render_browser_document_html,
};

const PROSE_RULE: &str = ":where(p, li, blockquote, dl, h1, h2, h3, h4, h5, h6){max-width: 72ch;}";

fn document() -> Document {
    Document::new(RenderNode::root(vec![
        RenderNode::paragraph(vec![RenderNode::text("Some prose.")]),
        RenderNode::code(None, None, "a very long line of code\n"),
        RenderNode::table(
            vec![ColumnAlign::None],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("cell"),
            ])])],
        ),
    ]))
}

fn with_width(width: Width) -> BrowserRenderOptions {
    BrowserRenderOptions {
        width,
        ..BrowserRenderOptions::default()
    }
}

/// The page through the fragment path and the streaming path, which agree.
fn pages(width: Width) -> String {
    let doc = document();
    let opts = with_width(width);
    let fragment = render_browser_document(&doc, &opts)
        .expect("the document renders")
        .output
        .render()
        .expect("the page renders");
    let streamed = render_browser_document_html(&doc, &opts)
        .expect("the document renders")
        .output;
    assert_eq!(fragment, streamed);
    streamed
}

#[test]
fn columns_caps_prose_blocks_in_ch() {
    let page = pages(Width::Columns(72));
    assert_eq!(page.matches(PROSE_RULE).count(), 1, "{page}");
    let body = render_browser_document_body(&document(), &with_width(Width::Columns(72)))
        .expect("the document renders")
        .output;
    assert!(body.assets.contains(PROSE_RULE), "{}", body.assets);
}

#[test]
fn source_and_unlimited_add_no_prose_constraint() {
    let default = pages(Width::Source);
    assert!(!default.contains(":where(p,"), "{default}");
    assert_eq!(pages(Width::Unlimited), default);
    let doc = document();
    let plain = render_browser_document_html(&doc, &BrowserRenderOptions::default())
        .expect("the document renders")
        .output;
    assert_eq!(plain, default);
}

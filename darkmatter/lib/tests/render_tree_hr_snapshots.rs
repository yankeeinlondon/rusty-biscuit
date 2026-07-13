//! Byte-stable snapshots for the **render-tree** HR-attribute path.
//!
//! The block-extension lift (`renderable/features/2026-05-26-block-extension`)
//! requires the render-tree pipeline to preserve byte-identical output for the
//! existing HR-attribute fixtures, including the combined `mark_dim_hr`
//! document. The sibling `horizontal_rule_snapshots.rs` snapshots the public
//! `Markdown::as_terminal` / `as_html` surface; this file pins the lower-level
//! fold-then-render path directly.
//!
//! This file pins the *tree* pipeline's own bytes
//! (`fold_markdown_spanned_with_frontmatter` → `render_*_document`) for the HR
//! fixtures so any drift in the lifted `BlockExtensionProcessor` path — a
//! changed attribute parse, a dropped hint, an altered rule glyph — fails an
//! exact-match assertion that names the fixture and surface.
//!
//! Terminal snapshots render through a pinned 80-wide *optimistic* terminal so
//! the captured bytes (width, ANSI palette, rule glyph) are reproducible
//! regardless of the host terminal's detected capabilities.

// Whitebox: these snapshots wire the deprecated `TerminalCodeRenderer` adapter
// directly to exercise the render-tree code path.
#![allow(deprecated)]

use std::rc::Rc;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter,
};
use insta::assert_snapshot;
use renderable::tree::{
    BrowserRenderOptions, Document, MarkdownDialect, MarkdownRenderOptions, RawHtmlPolicy,
    RenderStrictness, SourceDescriptor, render_browser_document, render_markdown_document,
};

/// HR-attribute fixtures locked through the tree pipeline.
///
/// `waves` / `kind_waves` cover the canonical `kind` and legacy `style` keys;
/// `all_attributes` exercises every hint; `mark_dim_hr` is the named combined
/// fixture from the spec — mark + dim inline spans interleaved with a styled
/// rule — kept small so the snapshot stays reviewable while still proving the
/// block-extension processor and the inline-span dispatcher compose.
const FIXTURES: &[(&str, &str)] = &[
    ("waves", "--- { style: waves }\n"),
    ("kind_waves", "--- { kind: waves }\n"),
    (
        "all_attributes",
        "--- { kind: waves, alignment: centered, weight: thick, width: \"75%\", color: \"blue\" }\n",
    ),
    (
        "mark_dim_hr",
        "Lead paragraph with ==highlighted phrase== and \u{2304}dimmed phrase\u{2304} inline.\n\n\
         --- { style: waves, weight: thick }\n\n\
         Trailing paragraph.\n",
    ),
];

/// Folds fixture text through the span-aware (production) tree path, asserting
/// the fold emitted no diagnostics.
fn fold(name: &str, markdown: &str) -> Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let md: Markdown = markdown.into();
    let (doc, diags) = fold_markdown_spanned_with_frontmatter(source, &md)
        .expect("span-aware fold must succeed");
    assert!(
        diags.is_empty(),
        "[{name}] span-aware fold emitted diagnostics: {diags:#?}",
    );
    doc
}

/// Pinned browser options: raw HTML escaped (the safe default — no fixture
/// here contains raw HTML), no code renderer (no fixture has code blocks).
fn browser_options() -> BrowserRenderOptions {
    BrowserRenderOptions {
        raw_html: RawHtmlPolicy::Escape,
        ..BrowserRenderOptions::default()
    }
}

/// Pinned terminal options: an 80-wide optimistic terminal gives a stable,
/// host-independent capability set. Image protocols are disabled so the styled
/// rule renders as its glyph form (e.g. the `≋` waves glyph the Level 2 test
/// asserts) rather than an embedded base64 PNG — the latter would make the
/// snapshot brittle (encoder/zlib-dependent) and unreviewable. The code
/// renderer is wired to mirror the production entry point even though these
/// fixtures carry no code blocks.
fn terminal_options() -> TerminalRenderOptions {
    let mut term = Terminal::new_optimistic(80);
    term.image_support = ImageSupport::None;
    TerminalRenderOptions {
        context: TerminalRenderContext::from_terminal(&term),
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

fn markdown_options(dialect: MarkdownDialect) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Warn,
        style: None,
    }
}

#[test]
fn render_tree_hr_html_snapshots() {
    for (name, markdown) in FIXTURES {
        let doc = fold(name, markdown);
        let html = render_browser_document(&doc, &browser_options())
            .expect("tree browser render must succeed")
            .output
            .render()
            .expect("tree browser render must succeed");
        assert_snapshot!(format!("html_{name}"), html);
    }
}

#[test]
fn render_tree_hr_terminal_snapshots() {
    for (name, markdown) in FIXTURES {
        let doc = fold(name, markdown);
        let term = render_terminal_document(&doc, &terminal_options())
            .expect("tree terminal render must succeed")
            .output;
        assert_snapshot!(format!("terminal_{name}"), term);
    }
}

#[test]
fn render_tree_hr_markdown_snapshots() {
    for (name, markdown) in FIXTURES {
        let doc = fold(name, markdown);
        let md = render_markdown_document(&doc, &markdown_options(MarkdownDialect::MarkdownPlus))
            .expect("tree markdown render must succeed")
            .output;
        assert_snapshot!(format!("markdown_{name}"), md);
    }
}

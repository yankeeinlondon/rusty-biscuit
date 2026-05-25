//! Criterion benchmarks for renderable tree rendering.
//!
//! Covers Markdown and HTML render targets over stress documents.

use criterion::{Criterion, criterion_group, criterion_main};
use renderable::tree::{
    BrowserRenderOptions, Document, DocumentMetadata, HeadingDepth, MarkdownRenderOptions,
    RenderNode, SourceRegistry, render_browser_document,
    render_markdown_document,
};
use std::hint::black_box;

// ---------------------------------------------------------------------------
// Stress-document generators
// ---------------------------------------------------------------------------

fn large_document() -> Document {
    let mut children = Vec::new();
    for section in 0..50 {
        children.push(RenderNode::heading(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text(format!("Section {section}"))],
        ));
        for para in 0..10 {
            children.push(RenderNode::paragraph(vec![
                RenderNode::text(format!(
                    "This is paragraph {para} in section {section}. ")
                ),
                RenderNode::strong(vec![RenderNode::text("Bold text. ")]),
                RenderNode::emphasis(vec![RenderNode::text("Italic text. ")]),
                RenderNode::link(
                    "https://example.com",
                    Some(format!("link-{section}-{para}")),
                    vec![RenderNode::text("a link")],
                ),
            ]));
        }
        children.push(RenderNode::code(
            Some("rust".to_string()),
            None,
            "fn main() { println!(\"hello\"); }",
        ));
    }

    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(children),
    }
}

fn deeply_nested_document() -> Document {
    fn nest(depth: usize) -> RenderNode {
        if depth == 0 {
            RenderNode::paragraph(vec![RenderNode::text("Leaf paragraph.")])
        } else {
            RenderNode::block_quote(vec![nest(depth - 1)])
        }
    }

    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![nest(20)]),
    }
}

fn table_heavy_document() -> Document {
    let mut rows = Vec::new();
    rows.push(RenderNode::table_row(
        (0..8)
            .map(|c| RenderNode::table_cell(vec![RenderNode::text(format!("Header {c}"))]))
            .collect(),
    ));
    for r in 0..100 {
        rows.push(RenderNode::table_row(
            (0..8)
                .map(|c| {
                    RenderNode::table_cell(vec![RenderNode::text(format!("r{r}c{c}"))])
                })
                .collect(),
        ));
    }

    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![RenderNode::table(
            vec![
                renderable::tree::ColumnAlign::Left,
                renderable::tree::ColumnAlign::Center,
                renderable::tree::ColumnAlign::Right,
                renderable::tree::ColumnAlign::None,
                renderable::tree::ColumnAlign::Left,
                renderable::tree::ColumnAlign::Center,
                renderable::tree::ColumnAlign::Right,
                renderable::tree::ColumnAlign::None,
            ],
            rows,
        )]),
    }
}

// ---------------------------------------------------------------------------
// Markdown render benchmarks
// ---------------------------------------------------------------------------

fn bench_render_markdown_large(c: &mut Criterion) {
    let doc = large_document();
    let opts = MarkdownRenderOptions::default();
    let mut group = c.benchmark_group("render_markdown");
    group.bench_function("large_doc", |b| {
        b.iter(|| render_markdown_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

fn bench_render_markdown_nested(c: &mut Criterion) {
    let doc = deeply_nested_document();
    let opts = MarkdownRenderOptions::default();
    let mut group = c.benchmark_group("render_markdown");
    group.bench_function("deeply_nested", |b| {
        b.iter(|| render_markdown_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

fn bench_render_markdown_tables(c: &mut Criterion) {
    let doc = table_heavy_document();
    let opts = MarkdownRenderOptions::default();
    let mut group = c.benchmark_group("render_markdown");
    group.bench_function("table_heavy", |b| {
        b.iter(|| render_markdown_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Browser (HTML) render benchmarks
// ---------------------------------------------------------------------------

fn bench_render_browser_large(c: &mut Criterion) {
    let doc = large_document();
    let opts = BrowserRenderOptions::default();
    let mut group = c.benchmark_group("render_browser");
    group.bench_function("large_doc", |b| {
        b.iter(|| render_browser_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

fn bench_render_browser_nested(c: &mut Criterion) {
    let doc = deeply_nested_document();
    let opts = BrowserRenderOptions::default();
    let mut group = c.benchmark_group("render_browser");
    group.bench_function("deeply_nested", |b| {
        b.iter(|| render_browser_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

fn bench_render_browser_tables(c: &mut Criterion) {
    let doc = table_heavy_document();
    let opts = BrowserRenderOptions::default();
    let mut group = c.benchmark_group("render_browser");
    group.bench_function("table_heavy", |b| {
        b.iter(|| render_browser_document(black_box(&doc), &opts).unwrap())
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_render_markdown_large,
    bench_render_markdown_nested,
    bench_render_markdown_tables,
    bench_render_browser_large,
    bench_render_browser_nested,
    bench_render_browser_tables,
);
criterion_main!(benches);

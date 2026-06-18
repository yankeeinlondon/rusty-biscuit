//! Criterion benchmarks for the darkmatter render-tree pipeline.
//!
//! The pipeline measured here has two stages:
//!
//! 1. **Fold** — [`fold_markdown_to_document`] walks a `pulldown-cmark` event
//!    stream and builds a [`renderable::tree::Document`].
//! 2. **Render** — the folded document is rendered to Markdown
//!    ([`render_markdown_document`]) or to a browser [`HtmlPage`]
//!    ([`render_browser_document`]).
//!
//! Inputs are programmatically generated stress documents — a large code
//! block, a wide-and-tall table, deeply nested lists, and a link/image-heavy
//! document — so the benchmark exercises the fold and the renderers over
//! markedly different node-shape distributions.
//!
//! [`HtmlPage`]: renderable::html::HtmlPage

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::render_tree::fold_markdown_to_document;
use renderable::tree::{
    BrowserRenderOptions, MarkdownRenderOptions, SourceDescriptor, render_browser_document,
    render_markdown_document,
};

// ---------------------------------------------------------------------------
// Stress-input generators
// ---------------------------------------------------------------------------

/// Builds a Markdown document with one very large fenced code block.
fn large_code_block() -> String {
    let mut out = String::from("# Large code block\n\n```rust\n");
    for line in 0..600 {
        out.push_str(&format!(
            "fn generated_{line}() -> usize {{ let value = {line}; value * 2 }}\n"
        ));
    }
    out.push_str("```\n");
    out
}

/// Builds a Markdown document with one wide, tall GFM table.
fn large_table() -> String {
    let columns = 8;
    let rows = 200;
    let mut out = String::from("# Large table\n\n");
    out.push('|');
    for col in 0..columns {
        out.push_str(&format!(" Column {col} |"));
    }
    out.push('\n');
    out.push('|');
    for _ in 0..columns {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in 0..rows {
        out.push('|');
        for col in 0..columns {
            out.push_str(&format!(" r{row}c{col} cell |"));
        }
        out.push('\n');
    }
    out
}

/// Builds a Markdown document with deeply nested unordered lists.
fn deeply_nested_lists() -> String {
    let depth = 14;
    let siblings = 3;
    let mut out = String::from("# Nested lists\n\n");
    for level in 0..depth {
        let indent = "  ".repeat(level);
        for sibling in 0..siblings {
            out.push_str(&format!(
                "{indent}- level {level} item {sibling} with descriptive text\n"
            ));
        }
    }
    out
}

/// Builds a Markdown document dense with links and image references.
fn many_links_images() -> String {
    let mut out = String::from("# Links and images\n\n");
    for paragraph in 0..120 {
        out.push_str(&format!(
            "Paragraph {paragraph} references \
             [link {paragraph}](https://example.com/page/{paragraph} \"Title {paragraph}\") \
             and shows ![alt {paragraph}](https://cdn.example.com/img/{paragraph}.png) inline.\n\n"
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Folds each stress input from text into a render-tree document.
fn bench_fold(c: &mut Criterion) {
    let inputs = [
        ("large_code_block", large_code_block()),
        ("large_table", large_table()),
        ("deeply_nested_lists", deeply_nested_lists()),
        ("many_links_images", many_links_images()),
    ];

    let mut group = c.benchmark_group("render_tree_fold");
    for (name, input) in &inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(*name, |b| {
            b.iter(|| {
                let source = SourceDescriptor::Virtual {
                    name: (*name).into(),
                };
                fold_markdown_to_document(source, black_box(input))
            })
        });
    }
    group.finish();
}

/// Renders each pre-folded stress document to Markdown.
fn bench_render_markdown(c: &mut Criterion) {
    let inputs = [
        ("large_code_block", large_code_block()),
        ("large_table", large_table()),
        ("deeply_nested_lists", deeply_nested_lists()),
        ("many_links_images", many_links_images()),
    ];
    let opts = MarkdownRenderOptions::default();

    let mut group = c.benchmark_group("render_tree_markdown");
    for (name, input) in &inputs {
        let source = SourceDescriptor::Virtual {
            name: (*name).into(),
        };
        let (doc, _diags) = fold_markdown_to_document(source, input);
        group.bench_function(*name, |b| {
            b.iter(|| render_markdown_document(black_box(&doc), &opts))
        });
    }
    group.finish();
}

/// Renders each pre-folded stress document to a browser `HtmlPage`.
fn bench_render_browser(c: &mut Criterion) {
    let inputs = [
        ("large_code_block", large_code_block()),
        ("large_table", large_table()),
        ("deeply_nested_lists", deeply_nested_lists()),
        ("many_links_images", many_links_images()),
    ];
    let opts = BrowserRenderOptions::default();

    let mut group = c.benchmark_group("render_tree_browser");
    for (name, input) in &inputs {
        let source = SourceDescriptor::Virtual {
            name: (*name).into(),
        };
        let (doc, _diags) = fold_markdown_to_document(source, input);
        group.bench_function(*name, |b| {
            b.iter(|| render_browser_document(black_box(&doc), &opts))
        });
    }
    group.finish();
}

/// Measures the full fold + Markdown-render pipeline end to end.
fn bench_fold_then_render(c: &mut Criterion) {
    let inputs = [
        ("large_code_block", large_code_block()),
        ("large_table", large_table()),
        ("deeply_nested_lists", deeply_nested_lists()),
        ("many_links_images", many_links_images()),
    ];
    let opts = MarkdownRenderOptions::default();

    let mut group = c.benchmark_group("render_tree_pipeline");
    for (name, input) in &inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(*name, |b| {
            b.iter(|| {
                let source = SourceDescriptor::Virtual {
                    name: (*name).into(),
                };
                let (doc, _diags) = fold_markdown_to_document(source, black_box(input));
                render_markdown_document(&doc, &opts)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_fold,
    bench_render_markdown,
    bench_render_browser,
    bench_fold_then_render,
);
criterion_main!(benches);

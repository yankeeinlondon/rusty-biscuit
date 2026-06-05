//! Step-broken render-pipeline benchmarks for the tree path (perf-gate Part 2).
//!
//! Each target's group isolates `parse` -> `fold` -> `render` and a `full`
//! end-to-end run, so a regression points at the stage that moved. The render
//! tree is now the only render path (the bespoke serializers were deleted in
//! the tree cutover), so this file is tree-only and baseline-tracked. Run:
//!
//! ```text
//! cargo bench -p darkmatter --bench render_pipeline_steps
//! ```

use std::hint::black_box;
use std::rc::Rc;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::yaml_block::YamlBlock;
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter,
};
use renderable::tree::{
    BrowserMermaidMode, BrowserRenderOptions, RawHtmlPolicy, RenderStrictness, SourceDescriptor,
    render_browser_document_html,
};

fn corpus() -> String {
    let mut out = String::from("# Pipeline corpus\n\n");
    for s in 0..8 {
        out.push_str(&format!("## Section {s}\n\nParagraph with ==highlight {s}== and \u{2304}dim {s}\u{2304} and *emphasis*.\n\n"));
        out.push_str(&format!("- item {s}a\n- item {s}b\n\n"));
        if s % 2 == 0 {
            out.push_str("```rust\nfn x() -> usize { 1 }\n```\n\n");
        } else {
            out.push_str("| A | B |\n| --- | --- |\n| 1 | 2 |\n\n");
        }
    }
    out
}

fn source() -> SourceDescriptor {
    SourceDescriptor::Virtual { name: "pipeline_corpus".into() }
}

fn tree_terminal_options() -> TerminalRenderOptions {
    let term = Terminal::new_optimistic(120);
    TerminalRenderOptions {
        context: TerminalRenderContext::from_terminal(&term),
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

fn browser_options() -> BrowserRenderOptions {
    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page: None,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
        mermaid_mode: BrowserMermaidMode::Code,
        ..Default::default()
    }
}

fn bench_render_pipeline_terminal(c: &mut Criterion) {
    let input = corpus();
    let term_opts = tree_terminal_options();
    let mut group = c.benchmark_group("render_pipeline_terminal");
    group.sample_size(20);

    group.bench_function("parse", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            black_box(md)
        })
    });
    group.bench_function("fold", |b| {
        let md: Markdown = input.as_str().into();
        b.iter(|| {
            let (doc, diags) = fold_markdown_spanned_with_frontmatter(source(), black_box(&md));
            black_box((doc, diags))
        })
    });
    group.bench_function("render", |b| {
        let md: Markdown = input.as_str().into();
        let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
        b.iter(|| render_terminal_document(black_box(&doc), &term_opts).expect("terminal render"))
    });
    group.bench_function("full", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
            render_terminal_document(&doc, &term_opts).expect("terminal render")
        })
    });
    group.finish();
}

fn bench_render_pipeline_browser(c: &mut Criterion) {
    let input = corpus();
    let browser_opts = browser_options();
    let mut group = c.benchmark_group("render_pipeline_browser");
    group.sample_size(20);

    group.bench_function("parse", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            black_box(md)
        })
    });
    group.bench_function("fold", |b| {
        let md: Markdown = input.as_str().into();
        b.iter(|| {
            let (doc, diags) = fold_markdown_spanned_with_frontmatter(source(), black_box(&md));
            black_box((doc, diags))
        })
    });
    // `render` / `full` measure the direct final-string document path
    // (`render_browser_document_html`) — the same surface the production
    // `render_tree_html` entry point now uses — so this localization bench
    // attributes cost to the path production actually pays.
    group.bench_function("render", |b| {
        let md: Markdown = input.as_str().into();
        let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
        b.iter(|| {
            render_browser_document_html(black_box(&doc), &browser_opts).expect("browser render")
        })
    });
    group.bench_function("full", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
            render_browser_document_html(&doc, &browser_opts).expect("browser render")
        })
    });
    group.finish();
}

fn bench_darkmatter_components(c: &mut Criterion) {
    let yaml = YamlBlock::new("name: example\nvalues:\n  - a\n  - b\n  - c\ncount: 3\nactive: true")
        .expect("valid yaml");
    let term = Terminal::new_optimistic(120);
    let page_md: Markdown =
        "# Page\n\nParagraph with *emphasis* and a list:\n\n- one\n- two\n\n```rust\nfn x() {}\n```\n".into();

    let mut group = c.benchmark_group("darkmatter_components");
    group.sample_size(20);

    group.bench_function("yaml_block/terminal", |b| {
        b.iter(|| TerminalRenderable::render(black_box(&yaml), &term))
    });
    group.bench_function("yaml_block/browser", |b| {
        b.iter(|| BrowserRenderable::render_html_fragment(black_box(&yaml)).render())
    });
    group.bench_function("darkmatter_page/terminal", |b| {
        b.iter(|| {
            DarkmatterPage::new(&term)
                .with_max_width(100)
                .render(black_box(&page_md))
                .expect("page terminal render")
        })
    });
    group.bench_function("darkmatter_page/browser", |b| {
        b.iter(|| {
            DarkmatterPage::new(&term)
                .with_max_width(100)
                .render_to_browser(black_box(&page_md))
                .expect("page browser render")
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_render_pipeline_terminal,
    bench_render_pipeline_browser,
    bench_darkmatter_components,
);
criterion_main!(benches);

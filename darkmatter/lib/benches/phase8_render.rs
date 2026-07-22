//! Criterion benchmarks for the 2026-07-15 performance follow-up Phase 8
//! (Finding 23): render-scoped code theme/environment resolution.
//!
//! The input is the committed, hashed manifest fixture `render_code_heavy`
//! (Architecture Decision A) — 40 fenced blocks across three languages — so the
//! measured bytes are frozen and the per-block resolution F23 hoists is
//! exercised 40 times per render.
//!
//! Both public render entry points that wire the code hook are measured:
//!
//! - `as_terminal_code_heavy` — [`Markdown::as_terminal`], whose hook resolves
//!   the theme pair, page/panel color modes, and chrome contrast.
//! - `as_html_code_heavy` — [`Markdown::as_html`], whose hook additionally
//!   cloned the page [`HtmlOptions`] (a `HashMap` plus four `Option<CommonStyle>`)
//!   per block before F23.
//! - `code_block_direct` — the page-less [`CodeBlock`] surface, the only one
//!   that reads `CODE_THEME` / `THEME` at hook time. One block per render, so
//!   this is F23's **control**: it must not regress.
//!
//! Terminal width and color depth are pinned so results never depend on live
//! terminal detection or the host.
//!
//! Baseline/candidate comparison (identical fixture + harness bytes; only
//! `render_tree/code_renderer.rs` differs):
//!
//! ```text
//! git checkout HEAD -- darkmatter/lib/src/markdown/render_tree/code_renderer.rs
//! cargo bench -p darkmatter --bench phase8_render -- --save-baseline f23-before
//! # restore the candidate file
//! cargo bench -p darkmatter --bench phase8_render -- --baseline f23-before
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::code_block::CodeBlock;
use darkmatter::markdown::output::html::HtmlOptions;
use darkmatter::markdown::output::terminal::{ColorDepth, TerminalOptions};
use std::hint::black_box;
use std::path::PathBuf;

/// Loads a committed manifest fixture by stem.
fn fixture(stem: &str) -> Markdown {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()));
    Markdown::from(text.as_str())
}

/// Pinned options: a `ColorDepth::None` context would short-circuit the code
/// hook entirely and measure nothing.
fn terminal_options() -> TerminalOptions {
    let mut opts = TerminalOptions::default();
    opts.color_depth = Some(ColorDepth::TrueColor);
    opts.max_width = Some(120);
    opts
}

fn bench_render(c: &mut Criterion) {
    let md = fixture("render_code_heavy");

    let mut group = c.benchmark_group("phase8_f23");
    group.bench_function("as_terminal_code_heavy", |b| {
        b.iter(|| {
            let out = md.as_terminal(terminal_options()).expect("terminal render");
            black_box(out);
        });
    });
    group.bench_function("as_html_code_heavy", |b| {
        b.iter(|| {
            let out = md.as_html(HtmlOptions::default()).expect("html render");
            black_box(out);
        });
    });
    group.bench_function("code_block_direct", |b| {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let term = Terminal::new_optimistic(120);
        let block = CodeBlock::rust("fn demo() -> usize { 42 }");
        b.iter(|| {
            let out = TerminalRenderable::render(&block, &term);
            black_box(out);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_render);
criterion_main!(benches);

//! Criterion benchmark for rendering a code-heavy ~100 KB document.
//!
//! The 2026-07-12 performance review
//! (`darkmatter/reviews/2026-07-12-perf/`) targets the render path's remaining
//! allocation-tier costs (syntect theme clone per block — Finding 23,
//! per-token `format!` churn — Finding 24, the `==` second parse — Finding 19,
//! and the disclosure re-allocation — Finding 20). Those all concentrate in
//! syntax-highlighted code blocks, which the existing `render_pipeline` bench
//! only exercises on small documents. This bench freezes a single large,
//! code-dominated document so every render-path phase has a before/after
//! checkpoint.
//!
//! Both public render entry points the review flagged are measured:
//!
//! - `as_terminal` — [`Markdown::as_terminal`], the ANSI render path.
//! - `page_render` — [`DarkmatterPage::render`], the page-frame assembler.
//!
//! Terminal width is pinned (`Terminal::new_optimistic(120)` /
//! `TerminalOptions::max_width`) so results never depend on live terminal-size
//! detection or the machine the bench runs on.
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench render_code_heavy -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench render_code_heavy -- --baseline before
//! ```

use biscuit_terminal::terminal::Terminal;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::TerminalOptions;
use std::hint::black_box;

/// Lower bound for the generated document, in bytes.
const TARGET_BYTES: usize = 100_000;

/// Builds a code-heavy document of at least [`TARGET_BYTES`] bytes: repeated
/// sections, each carrying prose plus fenced `rust`/`python`/`json` blocks with
/// enough lines that syntax highlighting dominates the render cost. A `==`
/// token is included in one block per section so the render path's
/// protected-range handling (Finding 19) is exercised.
fn build_code_heavy_document() -> String {
    let mut doc = String::with_capacity(TARGET_BYTES + 4_096);
    doc.push_str("# Code-Heavy Benchmark Document\n\n");

    let mut section = 0usize;
    while doc.len() < TARGET_BYTES {
        doc.push_str(&format!(
            "## Section {section}\n\n\
             Prose paragraph with **bold**, _italics_, and `inline code`, plus a\n\
             [link](https://example.com/{section}) to keep the fold busy.\n\n\
             ```rust\n"
        ));
        for line in 0..24 {
            doc.push_str(&format!(
                "    let item_{line} = compute({section}, {line}); \
                 // ready == {line}\n"
            ));
        }
        doc.push_str("```\n\n```python\n");
        for line in 0..24 {
            doc.push_str(&format!(
                "    value_{line} = compute({section}, {line})\n"
            ));
        }
        doc.push_str("```\n\n```json\n");
        doc.push_str(&format!("{{\"section\": {section}, \"items\": [\n"));
        for line in 0..24 {
            doc.push_str(&format!("  {{\"id\": {line}, \"name\": \"item-{line}\"}},\n"));
        }
        doc.push_str("  null\n]}\n```\n\n");
        section += 1;
    }

    doc
}

/// Deterministic terminal options: pin `max_width` so results do not depend on
/// the terminal the benchmark happens to run in.
fn render_terminal_options() -> TerminalOptions {
    let mut options = TerminalOptions::default();
    options.max_width = Some(120);
    options
}

fn bench_render_code_heavy(c: &mut Criterion) {
    let source = build_code_heavy_document();
    let md = Markdown::from(source.as_str());
    let terminal_options = render_terminal_options();
    let terminal = Terminal::new_optimistic(120);

    let mut group = c.benchmark_group("render_code_heavy");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.sample_size(20);

    group.bench_function("as_terminal", |b| {
        b.iter(|| {
            let output = black_box(&md)
                .as_terminal(terminal_options.clone())
                .expect("terminal render must not fault");
            black_box(output);
        });
    });

    group.bench_function("page_render", |b| {
        b.iter(|| {
            let output = DarkmatterPage::new(&terminal)
                .render(black_box(&md))
                .expect("page render must not fault");
            black_box(output);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_render_code_heavy);
criterion_main!(benches);

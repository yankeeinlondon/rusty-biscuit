//! Criterion benchmarks for the darkmatter render pipeline.
//!
//! Each render mode is benchmarked in isolation so a refactor's before/after
//! `change:` delta points at the specific mode that moved:
//!
//! - `parse` — building a [`Markdown`] from raw source (frontmatter parse).
//! - `terminal` — ANSI rendering via [`Markdown::as_terminal`].
//! - `html` — HTML rendering via [`Markdown::as_html`].
//! - `highlight` — terminal rendering of a code-block-heavy corpus, isolating
//!   `syntect` syntax-highlighting cost.
//!
//! The corpus is generated deterministically in-process so inputs stay frozen
//! across runs. Terminal rendering uses a fixed `max_width` rather than live
//! terminal-size detection for stable, machine-independent results.
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench render_pipeline -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench render_pipeline -- --baseline before
//! ```

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions};
use std::hint::black_box;

const CORPUS_SIZE: usize = 30;
const CODE_CORPUS_SIZE: usize = 20;

/// Builds one mixed-content document: headings, prose, lists, a table, and
/// fenced code blocks in three languages.
fn build_document(index: usize) -> String {
    format!(
        "# Document {index}\n\n\
         Intro paragraph with **bold**, _italics_, and `inline code`.\n\n\
         ## Section A\n\n\
         - list item one\n\
         - list item two\n\
         - list item three\n\n\
         | Col1 | Col2 | Col3 |\n\
         |------|------|------|\n\
         | a    | b    | c    |\n\
         | d    | e    | f    |\n\n\
         ```rust\n\
         fn main() {{\n    \
         let value = {index};\n    \
         println!(\"value = {{}}\", value);\n\
         }}\n\
         ```\n\n\
         ## Section B\n\n\
         More prose with a [link](https://example.com/{index}).\n\n\
         ```python\n\
         def compute(n):\n    \
         return n * {index}\n\
         ```\n\n\
         ```json\n\
         {{\"id\": {index}, \"name\": \"document\"}}\n\
         ```\n"
    )
}

/// Builds one code-heavy document used to isolate syntax-highlighting cost.
fn build_code_document(index: usize) -> String {
    let mut doc = format!("# Code Sample {index}\n\n");
    for lang in ["rust", "python", "json"] {
        doc.push_str(&format!("```{lang}\n"));
        for line in 0..30 {
            doc.push_str(&format!("let item_{line} = compute({index}, {line});\n"));
        }
        doc.push_str("```\n\n");
    }
    doc
}

/// Deterministic options: pin `max_width` so results do not depend on the
/// terminal the benchmark happens to run in.
fn render_terminal_options() -> TerminalOptions {
    let mut options = TerminalOptions::default();
    options.max_width = Some(120);
    options
}

fn bench_render_pipeline(c: &mut Criterion) {
    let sources: Vec<String> = (0..CORPUS_SIZE).map(build_document).collect();
    let corpus: Vec<Markdown> = sources.iter().map(|s| Markdown::from(s.as_str())).collect();
    let code_corpus: Vec<Markdown> = (0..CODE_CORPUS_SIZE)
        .map(|i| Markdown::from(build_code_document(i).as_str()))
        .collect();
    let terminal_options = render_terminal_options();

    let mut group = c.benchmark_group("render_pipeline");
    group.sample_size(20);

    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));

    group.bench_function("parse", |b| {
        b.iter(|| {
            for source in &sources {
                black_box(Markdown::from(black_box(source.as_str())));
            }
        });
    });

    group.bench_function("terminal", |b| {
        b.iter(|| {
            for md in &corpus {
                let output = black_box(md)
                    .as_terminal(terminal_options.clone())
                    .expect("terminal render must not fault");
                black_box(output);
            }
        });
    });

    group.bench_function("html", |b| {
        b.iter(|| {
            for md in &corpus {
                let output = black_box(md)
                    .as_html(HtmlOptions::default())
                    .expect("html render must not fault");
                black_box(output);
            }
        });
    });

    group.throughput(Throughput::Elements(CODE_CORPUS_SIZE as u64));
    group.bench_function("highlight", |b| {
        b.iter(|| {
            for md in &code_corpus {
                let output = black_box(md)
                    .as_terminal(terminal_options.clone())
                    .expect("highlight render must not fault");
                black_box(output);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_render_pipeline);
criterion_main!(benches);

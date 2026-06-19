//! Criterion benchmarks for the darkmatter compose pipeline.
//!
//! Each compose stage is benchmarked in isolation via
//! [`ComposeOptions::only`] so a refactor's before/after `change:` delta
//! points at the specific stage that moved. A `full_pipeline` benchmark
//! exercises the default operation set end to end.
//!
//! The corpus is generated deterministically in-process so inputs stay
//! frozen across runs. Every document packs each benchmarked feature:
//! frontmatter variables, frontmatter values that themselves carry `{{ }}`
//! references, a frontmatter `replace` map, body `{{ }}` interpolation,
//! nested `::block` regions, deliberately messy tables/spacing, and skewed
//! heading levels. The corpus
//! contains no shell directives and no transclusion directives — those
//! stages are out of scope (non-deterministic / require on-disk fixtures).
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench compose_pipeline -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench compose_pipeline -- --baseline before
//! ```

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use std::hint::black_box;

const CORPUS_SIZE: usize = 200;

/// Builds one benchmark document. The `index` seeds deterministic variety
/// across frontmatter values while keeping every feature present.
fn build_document(index: usize) -> String {
    let published = index.is_multiple_of(2);
    format!(
        "---\n\
         title: Post {index}\n\
         author: Author {author}\n\
         count: {index}\n\
         published: {published}\n\
         slug: post-{{{{count}}}}-by-{{{{author}}}}\n\
         nested:\n  \
         email: user{index}@example.com\n  \
         rank: {rank}\n\
         tags:\n  \
         - rust\n  \
         - markdown\n  \
         - bench\n\
         replace:\n  \
         entry: record\n  \
         table: grid\n\
         ---\n\
         # {{{{title}}}}\n\n\
         Written by {{{{author}}}} — entry number {{{{count}}}}.\n\n\
         Contact: {{{{nested.email}}}} (rank {{{{nested.rank}}}}).\n\n\
         Status: {{{{count > 0 ? \"active\" : \"empty\"}}}}, \
         fallback {{{{missing || \"n/a\"}}}}.\n\n\
         ::block when=\"count > 0\"\n\
         This block renders for entry {{{{count}}}}.\n\n\
         ::block when=\"published\"\n\
         Nested published block for {{{{title}}}}.\n\
         ::end-block\n\
         ::end-block\n\
         ::block when=\"count < 0\"\n\
         This block never renders.\n\
         ::end-block\n\n\
         ### Skipped Heading Level\n\
         | Name | Value |\n\
         |---|---|\n\
         | a | 1 |\n\
         |  b   |   2 |\n\
         Some text immediately follows the table.\n\
         ## Late Section\n\
         More content closing out the document.\n",
        author = index % 7,
        rank = index % 5,
    )
}

/// Generates a deterministic corpus of [`CORPUS_SIZE`] documents.
fn build_corpus() -> Vec<Markdown> {
    (0..CORPUS_SIZE)
        .map(|i| Markdown::from(build_document(i).as_str()))
        .collect()
}

/// Runs `compose_with` over the corpus using the supplied options.
fn compose_corpus(corpus: &[Markdown], options: &ComposeOptions) {
    for md in corpus {
        let (composed, report) = md
            .compose_with(options.clone())
            .expect("compose must not fault");
        black_box((composed, report));
    }
}

fn bench_compose_pipeline(c: &mut Criterion) {
    let corpus = build_corpus();
    let mut group = c.benchmark_group("compose_pipeline");
    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));
    group.sample_size(20);

    // Only the pure (no-I/O) stages are benched. Shell/transclusion stages
    // need subprocess/filesystem access and are deliberately excluded.
    let stages = [
        (
            "frontmatter_interpolation",
            ComposeOperation::FrontmatterInterpolation,
        ),
        ("text_replacement", ComposeOperation::TextReplacement),
        ("interpolation", ComposeOperation::Interpolation),
        ("page_blocks", ComposeOperation::PageBlocks),
        ("cleanup", ComposeOperation::Cleanup),
        ("normalization", ComposeOperation::Normalization),
    ];

    for (name, op) in stages {
        let options = ComposeOptions::new().only(&[op]);
        group.bench_function(name, |b| {
            b.iter(|| compose_corpus(black_box(&corpus), black_box(&options)));
        });
    }

    let full = ComposeOptions::new();
    group.bench_function("full_pipeline", |b| {
        b.iter(|| compose_corpus(black_box(&corpus), black_box(&full)));
    });

    group.finish();
}

criterion_group!(benches, bench_compose_pipeline);
criterion_main!(benches);

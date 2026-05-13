//! Criterion benchmark for [`DarkmatterSchemas::validate`] across a
//! 1000-document corpus.
//!
//! The benchmark exercises the full hot path that production callers use:
//!
//! 1. Parsing the markdown document into a [`Markdown`] value.
//! 2. Resolving `$schema` and merging baselines via [`DarkmatterSchemas`].
//! 3. Running the cached `jsonschema::Validator` over the frontmatter.
//!
//! It generates a deterministic corpus of 1,000 documents pinned to the same
//! inline SimplifiedSchema so that the validator cache is exercised exactly
//! once per benchmark sample. Splitting the corpus into mostly-valid /
//! mostly-invalid documents prevents an `iter_errors` short-circuit from
//! dominating the timings.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use std::hint::black_box;

const CORPUS_SIZE: usize = 1_000;

/// Inline schema used by every generated document. Touches strings, numbers,
/// enums, and arrays so the validator does meaningful work per document.
const SCHEMA_FRONTMATTER: &str = "$schema:\n  \
    title: 'string(required)'\n  \
    slug: 'string(pattern(^[a-z][a-z0-9-]*$);required)'\n  \
    views: 'number(min(0);required)'\n  \
    status: 'enum(draft, published, archived; required)'\n  \
    tags: 'string[]'\n";

fn build_corpus() -> Vec<Markdown> {
    (0..CORPUS_SIZE)
        .map(|i| {
            // Roughly 1-in-10 documents fail validation so iter_errors traverses
            // a real problem set rather than always short-circuiting at success.
            let invalid_slug = i % 10 == 0;
            let slug = if invalid_slug {
                format!("INVALID slug {i}")
            } else {
                format!("post-{i}")
            };
            let status = match i % 3 {
                0 => "draft",
                1 => "published",
                _ => "archived",
            };
            let body = format!(
                "---\n{SCHEMA_FRONTMATTER}title: 'Post {i}'\nslug: '{slug}'\n\
                 views: {i}\nstatus: {status}\ntags:\n  - rust\n  - markdown\n---\n\
                 Body of post {i}\n"
            );
            Markdown::from(body.as_str())
        })
        .collect()
}

fn validate_corpus(api: &DarkmatterSchemas, corpus: &[Markdown]) -> (usize, usize) {
    let mut ok = 0usize;
    let mut bad = 0usize;
    for md in corpus {
        let report = api.validate(md).expect("validate must not fault");
        if report.valid {
            ok += 1;
        } else {
            bad += 1;
        }
    }
    (ok, bad)
}

fn bench_validate_corpus(c: &mut Criterion) {
    let corpus = build_corpus();
    let mut group = c.benchmark_group("schema_validation/corpus_1000");
    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));
    group.sample_size(20);

    group.bench_function("validate_1000_docs", |b| {
        b.iter_batched(
            DarkmatterSchemas::new,
            |api| {
                let (ok, bad) = validate_corpus(black_box(&api), black_box(&corpus));
                black_box((ok, bad));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("validate_1000_docs_warm_cache", |b| {
        let api = DarkmatterSchemas::new();
        // Prime the validator cache so each iteration measures hot-path work.
        let _ = validate_corpus(&api, &corpus);
        b.iter(|| {
            let (ok, bad) = validate_corpus(black_box(&api), black_box(&corpus));
            black_box((ok, bad));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_validate_corpus);
criterion_main!(benches);

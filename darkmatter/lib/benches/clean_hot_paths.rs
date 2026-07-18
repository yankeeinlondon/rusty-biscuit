//! Phase 1 baseline benchmarks for the `md clean` hot paths.
//!
//! Captures the pre-change cost of the two common cases the
//! invalid-frontmatter feature must not regress (see
//! `darkmatter/features/2026-07-14-invalid-frontmatter/`):
//!
//! * `no_frontmatter` — a representative document with no frontmatter block.
//!   The v1 contract is "no frontmatter → zero YAML/schema/trigger work".
//! * `clean_frontmatter` — a representative document whose frontmatter is
//!   already standards-based. The v1 contract is "already-clean frontmatter
//!   parses once and is not reparsed".
//!
//! Each case times the full library pipeline the non-save CLI path runs
//! (`try_from_content` → `cleanup` → `as_string`) plus the parse stage alone,
//! so a Phase 7 regression can be attributed to parsing versus cleanup.
//!
//! ```text
//! cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline phase1-before
//! # ...implement phases 2-6...
//! cargo bench -p darkmatter --bench clean_hot_paths -- --baseline phase1-before
//! ```

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;

/// Representative no-frontmatter document: headings, prose, a nested list,
/// and a fenced code block — the shape an agent emits for a plain note.
const NO_FRONTMATTER: &str = "\
# Meeting Notes

Notes from the weekly sync. Action items are listed below.

## Attendees

- Alice
- Bob
  - Carol (notes)
- Dave

## Discussion

The rollout plan covers three regions. Each region gets a staged deploy:

1. staging validation
2. canary at five percent
3. full rollout

```sh
cargo build --release
cargo nextest run
```

Follow up next week with the metrics review.
";

/// Representative already-clean frontmatter document: a realistic Darkmatter
/// property set over the same body shape.
const CLEAN_FRONTMATTER: &str = "\
---
title: Weekly Sync Notes
tags:
- meetings
- ops
hash: 9f86d081884c7d65-8a2b8f1f3d9a3b1c
style:
  page:
    margin: 2
---

# Meeting Notes

Notes from the weekly sync. Action items are listed below.

## Attendees

- Alice
- Bob
  - Carol (notes)
- Dave

## Discussion

The rollout plan covers three regions. Each region gets a staged deploy:

1. staging validation
2. canary at five percent
3. full rollout

```sh
cargo build --release
cargo nextest run
```

Follow up next week with the metrics review.
";

/// The non-save `md clean` library pipeline: parse, cleanup, serialize.
fn clean_pipeline(source: &str) -> String {
    let mut md = Markdown::try_from_content(source).expect("baseline fixtures must parse");
    md.cleanup();
    md.as_string()
}

fn bench_clean_hot_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("clean_hot_paths");

    group.bench_function("no_frontmatter/full_pipeline", |b| {
        b.iter(|| black_box(clean_pipeline(black_box(NO_FRONTMATTER))))
    });
    group.bench_function("no_frontmatter/parse_only", |b| {
        b.iter(|| {
            black_box(Markdown::try_from_content(black_box(NO_FRONTMATTER)).expect("parse"))
        })
    });
    group.bench_function("clean_frontmatter/full_pipeline", |b| {
        b.iter(|| black_box(clean_pipeline(black_box(CLEAN_FRONTMATTER))))
    });
    group.bench_function("clean_frontmatter/parse_only", |b| {
        b.iter(|| {
            black_box(Markdown::try_from_content(black_box(CLEAN_FRONTMATTER)).expect("parse"))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_clean_hot_paths);
criterion_main!(benches);

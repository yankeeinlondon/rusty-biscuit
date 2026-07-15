//! Criterion benchmark for the regressed `compose_with` path.
//!
//! The 2026-07-12 performance review (`darkmatter/reviews/2026-07-12-perf/`)
//! found the compose path that combines **frontmatter interpolation**, a
//! document **`$schema`**, the **Darkmatter baseline schema**, and **one
//! `::file` transclusion** had no bench coverage — yet it is exactly where the
//! schema double-work (Findings 5/6/9) and multi-walk deduplication (Finding 7)
//! costs land. This bench freezes that path so every later phase has a concrete
//! before/after checkpoint.
//!
//! The fixture is written to a throwaway directory once, then composed through
//! the real, default operation set (mirroring `md compose <file>`) so the
//! measured cost includes context capture, schema resolution/validation, and
//! transclusion — not just the pure in-memory stages the existing
//! `compose_pipeline` bench covers.
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench compose_schema_transclusion -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench compose_schema_transclusion -- --baseline before
//! ```

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use std::fs;
use std::hint::black_box;
use std::path::Path;

/// Root document: inline `$schema` (typed top-level properties), frontmatter
/// interpolation (`slug` references `count`/`author`), body interpolation, and
/// a single `::file` transclusion of a peer document.
const ROOT_DOC: &str = "\
---
$schema:
  title: 'string(required)'
  author: 'string'
  count: 'number'
  published: 'boolean'
title: Benchmark Post
author: Bench Author
count: 7
published: true
slug: post-{{count}}-by-{{author}}
---
# {{title}}

Written by {{author}} — entry number {{count}} (slug `{{slug}}`).

Status: {{count > 0 ? \"active\" : \"empty\"}}.

::file ./child.md
";

/// Transcluded child document — its own frontmatter and a small section body.
const CHILD_DOC: &str = "\
---
title: Child Section
---
## Included Section

Some included prose with **bold** and _italic_ text, plus a
[link](https://example.com/child) and a short list:

- alpha
- beta
- gamma
";

/// Writes the fixture tree (a `.git` marker anchors the repo root) and returns
/// the root document path.
fn write_fixture(dir: &Path) -> std::path::PathBuf {
    fs::create_dir_all(dir.join(".git")).expect("create .git marker");
    let root = dir.join("root.md");
    fs::write(&root, ROOT_DOC).expect("write root.md");
    fs::write(dir.join("child.md"), CHILD_DOC).expect("write child.md");
    root
}

fn bench_compose_schema_transclusion(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = write_fixture(dir.path());
    let md = Markdown::try_from(root.as_path()).expect("load root.md");

    // Mirror `md compose <file>` defaults: full operation set + the Darkmatter
    // baseline schema injected alongside the document's inline `$schema`.
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_darkmatter_baseline_schema();

    let mut group = c.benchmark_group("compose_schema_transclusion");
    group.throughput(Throughput::Elements(1));
    group.sample_size(20);

    group.bench_function("compose_with", |b| {
        b.iter(|| {
            let (composed, report) = black_box(&md)
                .compose_with(black_box(options.clone()))
                .expect("compose must not fault");
            black_box((composed, report));
        });
    });

    group.finish();
    drop(dir);
}

criterion_group!(benches, bench_compose_schema_transclusion);
criterion_main!(benches);

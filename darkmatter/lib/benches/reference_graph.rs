//! Criterion benchmarks for the opaque reference graph.
//!
//! Three fixture shapes exercise the two costs the opacity cutover touches:
//!
//! * `small` — a single document with a handful of remote links.
//! * `large` — a single document with many remote links.
//! * `multi_transclusion` — a root plus several on-disk children reached
//!   through `::file`, so descendant verification has real work to do.
//!
//! For each fixture three things are measured:
//!
//! * `build_and_validate` — `validate_references` (build the graph **and**
//!   validate) — the baseline a caller pays without a prebuilt graph.
//! * `validate_prebuilt` — `validate_references_with_graph` with the graph
//!   constructed **outside** the timed loop, so only provenance checking,
//!   descendant re-verification, and flattening are measured.
//! * `construct` — `reference_graph` alone, so the provenance-construction cost
//!   the opacity cutover adds is measured directly. Compare it across the
//!   baseline and candidate commits with `--save-baseline` / `--baseline`.
//!
//! `validate_prebuilt` must stay materially faster than `build_and_validate`;
//! that gap is the Finding-18 reuse win the opaque-graph work must preserve.
//!
//! ```text
//! cargo bench -p darkmatter --bench reference_graph -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench reference_graph -- --baseline before
//! ```

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeSource};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use tempfile::TempDir;

/// Number of remote links packed into the `large` single-document fixture.
const LARGE_LINK_COUNT: usize = 200;

/// Number of on-disk children the `multi_transclusion` root includes.
const TRANSCLUSION_CHILD_COUNT: usize = 12;

/// A prepared fixture: an in-memory (or disk-loaded) document plus the temp
/// directory that must stay alive for its file-backed children to resolve.
struct Fixture {
    md: Markdown,
    // Kept only to own the on-disk children; `None` for in-memory fixtures.
    _dir: Option<TempDir>,
}

/// Single document with a few remote links (no transclusion, no disk reads).
fn small_fixture() -> Fixture {
    let body = "# Small\n\n\
        [a](https://example.com/a)\n\
        [b](https://example.com/b)\n\
        ![img](https://example.com/i.png)\n\
        [c](https://example.com/c)\n";
    Fixture {
        md: Markdown::new(body).with_source(ComposeSource::File(PathBuf::from("/tmp/small.md"))),
        _dir: None,
    }
}

/// Single document with many remote links.
fn large_fixture() -> Fixture {
    let mut body = String::from("# Large\n\n");
    for i in 0..LARGE_LINK_COUNT {
        body.push_str(&format!("[link {i}](https://example.com/page/{i})\n"));
    }
    Fixture {
        md: Markdown::new(body).with_source(ComposeSource::File(PathBuf::from("/tmp/large.md"))),
        _dir: None,
    }
}

/// Root document that includes several on-disk children via `::file`.
fn multi_transclusion_fixture() -> Fixture {
    let dir = TempDir::new().expect("temp dir");
    let mut root = String::from("# Root\n\n[top](https://example.com/top)\n\n");
    for i in 0..TRANSCLUSION_CHILD_COUNT {
        let name = format!("child{i}.md");
        std::fs::write(
            dir.path().join(&name),
            format!("# Child {i}\n\n[c{i}](https://child.example.com/{i})\n"),
        )
        .expect("write child");
        root.push_str(&format!("::file {name}\n\n"));
    }
    let root_path = dir.path().join("root.md");
    std::fs::write(&root_path, root).expect("write root");
    let md = Markdown::try_from(root_path.as_path()).expect("load root");
    Fixture {
        md,
        _dir: Some(dir),
    }
}

fn bench_reference_graph(c: &mut Criterion) {
    let fixtures = [
        ("small", small_fixture()),
        ("large", large_fixture()),
        ("multi_transclusion", multi_transclusion_fixture()),
    ];

    for (name, fixture) in &fixtures {
        let md = &fixture.md;
        let mut group = c.benchmark_group(format!("reference_graph/{name}"));
        group.sample_size(30);

        // Capture context exactly once per fixture so it is excluded from the
        // timed loops (a fresh `ComposeOptions::default()` runs sniff-driven
        // context discovery, which would dominate) and so the prebuilt graph
        // and the validation request share one options identity — otherwise the
        // opaque-graph guard would (correctly) reject the mismatched pairing.
        let graph_opts = ReferenceGraphOptions::with_compose(ComposeOptions::new());
        let val_opts = ReferenceValidationOptions::with_graph(graph_opts.clone());

        // Build + validate: the cost without a prebuilt graph.
        group.bench_function("build_and_validate", |b| {
            b.iter(|| {
                let report = md
                    .validate_references(val_opts.clone())
                    .expect("validate must not fault");
                black_box(report);
            });
        });

        // Validate a prebuilt graph: graph construction is outside the loop, so
        // only provenance + descendant checks + flatten are timed.
        let prebuilt = md
            .reference_graph(graph_opts.clone())
            .expect("graph build must not fault");
        group.bench_function("validate_prebuilt", |b| {
            b.iter(|| {
                let report = md
                    .validate_references_with_graph(black_box(&prebuilt), val_opts.clone())
                    .expect("validate must not fault");
                black_box(report);
            });
        });

        // Graph construction alone: measures provenance-construction cost.
        group.bench_function("construct", |b| {
            b.iter(|| {
                let graph = md
                    .reference_graph(graph_opts.clone())
                    .expect("graph build must not fault");
                black_box(graph);
            });
        });

        group.finish();
    }
}

criterion_group!(benches, bench_reference_graph);
criterion_main!(benches);

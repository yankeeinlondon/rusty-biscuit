//! Criterion benchmarks for the 2026-07-15 performance follow-up Phase 9
//! (Finding 33): remote-URL discovery line positions.
//!
//! Inputs are the committed, hashed manifest fixtures (Architecture Decision A)
//! so the measured bytes are frozen and reproducible.
//!
//! - `f33_discover_remote_heavy` (target) — `remote_heavy`, 300 read-side
//!   `frontmatter("https://…")` expressions across ~79 KB. Each expression's
//!   start offset grows toward the end of the document, so the per-expression
//!   `content[..start]` prefix rescan is quadratic in document size while one
//!   newline-offset table is linear.
//! - `f33_discover_no_http_guard` (control) — `toc_large` contains no `http`
//!   substring, so the cheap no-HTTP guard must still short-circuit before any
//!   expression scan. Guards against the offset table being built for documents
//!   that can never register a URL.
//! - `f33_discover_http_without_expressions` (control) — `render_code_heavy`
//!   with an appended bare `http` URL in prose: passes the guard but parses no
//!   expression, so the table must not be built for it either.
//!
//! Compare before/after with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench phase9_remote -- --save-baseline before
//! # ...change...
//! cargo bench -p darkmatter --bench phase9_remote -- --baseline before
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::remote::discover_remote_urls_from_expressions;
use std::hint::black_box;
use std::path::PathBuf;

/// Reads a committed manifest fixture by stem.
fn fixture_text(stem: &str) -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()))
}

fn bench_remote_discovery(c: &mut Criterion) {
    let source = ComposeSource::File(PathBuf::from("/bench/doc.md"));

    let heavy = fixture_text("remote_heavy");
    c.bench_function("f33_discover_remote_heavy", |b| {
        b.iter(|| {
            black_box(discover_remote_urls_from_expressions(
                black_box(&heavy),
                black_box(&source),
            ))
        })
    });

    let no_http = fixture_text("toc_large");
    c.bench_function("f33_discover_no_http_guard", |b| {
        b.iter(|| {
            black_box(discover_remote_urls_from_expressions(
                black_box(&no_http),
                black_box(&source),
            ))
        })
    });

    // Passes the `http` guard but contains no `{{ }}` expression at all, so the
    // offset table would be pure overhead if it were built unconditionally.
    let http_no_expr = format!(
        "{}\nSee http://example.com for details.\n",
        fixture_text("render_code_heavy")
    );
    c.bench_function("f33_discover_http_without_expressions", |b| {
        b.iter(|| {
            black_box(discover_remote_urls_from_expressions(
                black_box(&http_no_expr),
                black_box(&source),
            ))
        })
    });
}

criterion_group!(benches, bench_remote_discovery);
criterion_main!(benches);

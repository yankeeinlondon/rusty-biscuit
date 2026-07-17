//! Criterion benchmarks retaining raw sample vectors for the two 2026-07-15
//! performance-followup checkpoints whose measured surface is **public API**:
//! Finding 35.3 (`Arc<str>` fetched response bodies) and Finding 35.5
//! (`md hash --diff` / `--save` shared artifact).
//!
//! Review-2 ("Several benchmark dispositions still lack retained raw samples")
//! found that both were originally measured by *temporary* in-crate harnesses
//! that were deleted after capture, leaving only derived medians and prose. This
//! target is the retained replacement for the parts that need no crate-private
//! access; the crate-private checkpoints (23, 25, 35.6, 35.7) are measured by
//! the retained `#[cfg(test)]` harness in `src/perf_harness.rs` instead.
//!
//! Inputs are the committed, hashed manifest fixtures (Architecture Decision A),
//! so the measured bytes are frozen and reproducible.
//!
//! ```text
//! cargo bench -p darkmatter --bench phase11_evidence
//! ```
//!
//! Retain `target/criterion/<group>/<bench>/new/sample.json` per the benchmark
//! README's *Raw samples (mandatory)* rule; `benchmarks/recompute.ts` re-derives
//! every statistic quoted in a run record from those vectors.

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::hash::{MdHashKind, MdHashOptions, StoredHash};
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

/// Reads a committed manifest fixture by stem.
fn fixture_text(stem: &str) -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Finding 35.3 — `Arc<str>` fetched response bodies (implemented, REVERTED).
//
// Nothing shipped, so there is no candidate in the tree to call. What the
// disposition rests on is a *cost model*: whether swapping `FetchSlot::Ready`'s
// `String` for an `Arc<str>` can pay for itself. The model is pure `std` — a
// body `String`, an `Arc<str>` conversion, and the two read shapes — so it is
// reproducible here without re-applying the reverted change to the crate.
//
// The decisive fact the model establishes: `FetchSlot::Ready` is populated by
// MOVING `RemoteFetchOutcome.body` (a `String` from `String::from_utf8`).
// `Arc<str>` cannot reuse that allocation (its refcount header is inline), so
// storing one adds a full body copy per URL that the pre-change code never
// paid, while the public owned `get_content` facade must still hand out a
// `String`. Store cost is therefore charged once per URL against read savings
// that only materialize for `&str`-only consumers.
// ---------------------------------------------------------------------------

fn bench_f35_3_copy_model(c: &mut Criterion) {
    // The same fixture the original cost model used: a realistic fetched body.
    let body: String = fixture_text("remote_heavy");
    let shared: Arc<str> = Arc::from(body.as_str());

    // Equivalence gate: every shape below must yield the same bytes, or the
    // costs are not comparable.
    assert_eq!(
        body.clone(),
        shared.to_string(),
        "35.3 baseline and candidate read shapes must produce identical bytes"
    );
    assert_eq!(&*shared, body.as_str(), "Arc<str> must carry the body bytes");

    let mut group = c.benchmark_group("f35_3_copy_model");
    group.sample_size(200);

    // NEW cost, charged once per URL: the pre-change code moved the body into
    // the slot for free.
    group.bench_function("store_string_to_arc", |b| {
        b.iter(|| {
            let arc: Arc<str> = Arc::from(black_box(body.as_str()));
            black_box(arc)
        });
    });
    // Baseline read, per consumer.
    group.bench_function("read_string_clone", |b| {
        b.iter(|| black_box(black_box(&body).clone()));
    });
    // Candidate read through the owned public facade (`get_content -> String`).
    group.bench_function("read_arc_to_string", |b| {
        b.iter(|| black_box(black_box(&shared).to_string()));
    });
    // Candidate read for an `&str`-only consumer: a refcount bump.
    group.bench_function("read_arc_clone", |b| {
        b.iter(|| black_box(Arc::clone(black_box(&shared))));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Finding 35.5 — `md hash --diff` shared artifact.
//
// Baseline and candidate are BOTH current public API, so this needs no pinned
// copy and no cross-build comparison:
//
// - baseline  = `compare_hash` + `explain_hash_diff` — verbatim the two-call
//   path `run_hash_diff` used before commit `b8ecb88cb`, which computed the
//   like-for-like artifact twice;
// - candidate = `diff_hash` — what `run_hash_diff` calls now, computing it once
//   and returning both products.
//
// This is the delta of the CURRENT implementation, which review-2 found had no
// reproducible measurement: the retained −18.0% CLI figure measured an EARLIER
// state (`540262812`, whose own record calls the double-compute an unfixed
// residual), not this one.
// ---------------------------------------------------------------------------

/// Builds a stored hash of `kind` from `source`, then edits the body so the
/// diff has real work to explain (an unchanged document short-circuits parts of
/// the detailed alignment and would flatter both arms equally).
fn stored_and_edited(source: &str, kind: MdHashKind) -> (Markdown, StoredHash) {
    let options = MdHashOptions::default();
    let original: Markdown = Markdown::from(source);
    let stored = StoredHash {
        kind,
        value: original.compute_hash(kind, &options).to_stored_value(),
        ignored: Vec::new(),
    };
    let edited: Markdown = Markdown::from(format!("{source}\n\nAppended paragraph.\n").as_str());
    (edited, stored)
}

fn bench_f35_5_diff_hash(c: &mut Criterion) {
    let options = MdHashOptions::default();
    let large = fixture_text("toc_large");
    let small = fixture_text("hash_basic");

    for (fixture_name, source) in [("toc_large", &large), ("hash_basic", &small)] {
        for kind in [MdHashKind::Simple, MdHashKind::Structured, MdHashKind::Detailed] {
            let (edited, stored) = stored_and_edited(source, kind);

            // Equivalence gate: refuse to report a ratio between two paths that
            // do not agree. The candidate must return exactly the comparison and
            // the explanation the two-call baseline produces.
            let baseline_comparison = edited
                .compare_hash(&stored, &options)
                .expect("baseline comparison");
            let baseline_explanation = edited
                .explain_hash_diff(&stored, &options)
                .expect("baseline explanation");
            let (candidate_comparison, candidate_explanation) =
                edited.diff_hash(&stored, &options).expect("candidate diff");
            assert_eq!(
                baseline_comparison.frontmatter_changed, candidate_comparison.frontmatter_changed,
                "35.5 baseline/candidate must agree on frontmatter_changed ({fixture_name}/{kind:?})"
            );
            assert_eq!(
                baseline_comparison.body_changed, candidate_comparison.body_changed,
                "35.5 baseline/candidate must agree on body_changed ({fixture_name}/{kind:?})"
            );
            assert_eq!(
                baseline_explanation.render(),
                candidate_explanation.render(),
                "35.5 baseline/candidate must render an identical explanation ({fixture_name}/{kind:?})"
            );

            let kind_name = format!("{kind:?}").to_lowercase();
            let mut group =
                c.benchmark_group(format!("f35_5_diff_hash_{fixture_name}_{kind_name}"));
            group.sample_size(100);
            group.bench_function("baseline", |b| {
                b.iter(|| {
                    let comparison = black_box(&edited)
                        .compare_hash(black_box(&stored), black_box(&options))
                        .expect("comparison");
                    let explanation = black_box(&edited)
                        .explain_hash_diff(black_box(&stored), black_box(&options))
                        .expect("explanation");
                    black_box((comparison, explanation))
                });
            });
            group.bench_function("candidate", |b| {
                b.iter(|| {
                    black_box(
                        black_box(&edited)
                            .diff_hash(black_box(&stored), black_box(&options))
                            .expect("diff"),
                    )
                });
            });
            group.finish();
        }
    }
}

criterion_group!(benches, bench_f35_3_copy_model, bench_f35_5_diff_hash);
criterion_main!(benches);

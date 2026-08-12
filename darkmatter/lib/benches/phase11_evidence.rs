//! Criterion benchmarks retaining raw sample vectors for the 2026-07-15
//! performance-followup checkpoint whose measured surface is **public API**:
//! Finding 35.3 (`Arc<str>` fetched response bodies).
//!
//! Review-2 ("Several benchmark dispositions still lack retained raw samples")
//! found it was originally measured by a *temporary* in-crate harness that was
//! deleted after capture, leaving only derived medians and prose. This target is
//! the retained replacement for the parts that need no crate-private access; the
//! crate-private checkpoints (23, 25, 35.6, 35.7) are measured by the retained
//! `#[cfg(test)]` harness in `src/perf_harness.rs` instead.
//!
//! Finding 35.5 has no harness in either place: review-4 found its shared hash
//! seam was still public API under a non-default feature, so the seam — and with
//! it the measured candidate — was removed and `md hash` returned to the public
//! two-call path.
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

criterion_group!(benches, bench_f35_3_copy_model);
criterion_main!(benches);

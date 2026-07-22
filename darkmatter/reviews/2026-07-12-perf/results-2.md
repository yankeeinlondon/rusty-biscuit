---
scope: "Finding 29 ownership and zero-deep-clone follow-up for the 2026-07-12 Darkmatter performance review"
captured: "2026-07-15"
comparison_base: "cf90c1582"
arc_baseline: "arc-value-54e2f6b65"
zero_clone_baseline: "pre-zero-clone-cf90c1582"
host: "Apple M4 Max, macOS 26.5.2 (arm64)"
rustc: "1.96.0 (2026-05-25)"
criterion: "0.5.1"
build: "bench (optimized)"
benchmark: "effective_schema_ownership"
---

# Finding 29 Follow-up — Shared Effective Schemas and Zero-Clone Baselines

> **Still current (2026-07-16).** Unlike this review's other results, the
> 2026-07-15 audit sustained Finding 29: its same-source A/B comparison is
> valid, and the public `EffectiveSchema::json_schema` `Value` → `Arc<Value>`
> ownership exception plus its owned compatibility facade are **approved and
> preserved** as a standing invariant of
> [`2026-07-15-performance-followup`](../../features/2026-07-15-performance-followup/spec.md).
> No follow-up work reopened it. Final dispositions for every other finding are
> in
> [`2026-07-15-performance-followup/results.md`](../../features/2026-07-15-performance-followup/results.md).

This follow-up records two consecutive ownership changes:

1. `EffectiveSchema::json_schema` changed from `serde_json::Value` to
   `Arc<serde_json::Value>`, making clones of an assembled effective schema
   share the JSON allocation.
2. The built-in Darkmatter baseline path was refined so configuration and
   baseline-only resolution share the process-cached `Arc<Value>` directly.
   Baseline validation now borrows the cached value, and merging no longer
   clones the complete baseline property map before building its owned output.

The first change is an intentional public Rust API compatibility exception.
The second is additive: the owned `darkmatter_base_json_schema() -> Value`
accessor remains available with its independent-value semantics, while
`darkmatter_base_json_schema_ref()` and
`DarkmatterSchemas::with_darkmatter_baseline_json_schema()` provide the
read-only/shared paths.

## Method

- Command: `cargo bench -p darkmatter --bench effective_schema_ownership`.
- Criterion policy: 100 samples, default 3-second warm-up, and approximately
  5 seconds of measurement per function.
- Host gate: the sniff-backed `just _bench_preflight` had to pass before each
  saved run. A constrained post-build state was allowed to settle; no override
  was used.
- The `Value` versus `Arc<Value>` checkpoint used the same benchmark source and
  temporarily restored the pre-F29 owned field for comparison. The Arc result
  is retained under `arc-value-54e2f6b65`; the Value-side figures below are the
  Criterion console point estimates captured during that comparison.
- The zero-clone checkpoint added all initialization and merge cases before
  saving `pre-zero-clone-cf90c1582`. The benchmark source was unchanged for the
  after-run, so Criterion compared identical functions and fixture bytes.
- Times below are Criterion mean point estimates. Percentage changes in the
  second table come from each benchmark's saved `change/estimates.json`.

## Checkpoint 1: `Value` versus `Arc<Value>`

| Case | Owned `Value` | `Arc<Value>` | Arc improvement |
|------|--------------:|-------------:|----------------:|
| Darkmatter baseline — baseline only | 184.13 µs | 56.26 µs | 69.4% / 3.3× |
| Darkmatter baseline — baseline + document | 448.33 µs | 320.32 µs | 28.6% / 1.4× |
| Darkmatter baseline — clone effective schema | 125.53 µs | 340.21 ns | 99.7% / 369× |
| Synthetic 512 properties — baseline only | 172.24 µs | 75.45 µs | 56.2% / 2.3× |
| Synthetic 512 properties — baseline + document | 540.34 µs | 448.91 µs | 16.9% / 1.2× |
| Synthetic 512 properties — clone effective schema | 97.67 µs | 8.34 µs | 91.5% / 11.7× |

The document-only control showed no statistically significant change. The
largest improvement is the operation the ownership change directly targets:
cloning an effective schema becomes an Arc reference-count increment instead
of a recursive JSON tree clone.

## Checkpoint 2: zero-clone built-in baseline

| Case | Saved baseline | After | Criterion change | Assessment |
|------|---------------:|------:|-----------------:|------------|
| Built-in baseline configuration | 391.12 µs | 26.42 ns | −99.993% | improved |
| Configure + baseline-only resolve | 450.91 µs | 56.47 µs | −87.48% | improved |
| Merge empty document schema | 259.71 µs | 127.87 µs | −50.76% | improved |
| Merge tiny document schema | 259.42 µs | 127.47 µs | −50.86% | improved |
| Darkmatter baseline + document | 328.92 µs | 187.54 µs | −42.98% | improved |
| Synthetic baseline + document | 451.10 µs | 256.11 µs | −43.23% | improved |
| Darkmatter baseline only | 57.79 µs | 56.85 µs | −1.62% | within noise threshold |
| Synthetic baseline only | 76.39 µs | 75.07 µs | −1.73% | within noise threshold |
| Owned baseline accessor | 128.33 µs | 128.69 µs | +0.29% | no change detected |
| Clone effective, Darkmatter baseline | 344.43 ns | 346.45 ns | +0.59% | no change detected |
| Clone effective, synthetic baseline | 8.37 µs | 8.57 µs | +2.40% | small unrelated regression |
| Document-only control | 10.09 µs | 9.97 µs | −1.11% | within noise threshold |

The default configuration benchmark now measures an Arc increment plus
borrow-only validation of the cached schema's top-level contract. The
configuration-and-resolution case still parses and resolves the Markdown
document, which explains its remaining ~56 µs.

The ~43–51% merge improvements come from cloning each baseline property only
when it enters the owned merged result. The previous implementation first
deep-cloned the entire property map and then cloned inserted properties again.

The synthetic `clone_effective` control reported a 2.40% regression even
though that operation and its ownership structure were unchanged; the real
baseline equivalent reported no change. Treat it as a follow-up measurement
candidate rather than evidence against the targeted improvements.

## Ownership boundary after the change

- Built-in baseline configuration performs zero deep clones of the cached JSON
  Schema.
- Baseline-only `effective_for` performs zero deep clones and returns another
  reference to the same allocation.
- Compose's default baseline and DMLS documents without matching extension
  baselines use this shared path.
- Applying an extension or document schema must materialize an owned merged
  `Value`; each baseline property entering that result is cloned once.
- `darkmatter_base_json_schema() -> Value` still deep-clones by contract because
  its caller receives an independently mutable value.

Eliminating the remaining merge clones would require a structurally shared JSON
representation or a different public result model. That additional complexity
is not justified by these results.

## Verification

- `just test`: 5,616 Darkmatter library, 555 CLI, and 566 DMLS tests passed
  (6,737 total).
- `just test-l2`: 19 Darkmatter, 69 CLI, and 3 DMLS real-terminal tests passed
  (91 total).
- `just lint`: passed for Darkmatter, the CLI, and DMLS.
- `cargo check -p darkmatter -p dmls`: passed.
- `git diff --check`: passed.

These results resolve the review's performance uncertainty around the Arc
ownership choice and establish a repeatable baseline for the built-in schema
path. They do not resolve the separate OSC-query Level 2 evidence or the
original closeout fixture-reproducibility findings in [`review-1.md`](./review-1.md).

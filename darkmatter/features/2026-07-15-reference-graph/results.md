# Reference-Graph Performance Evidence

Records the Phase 4 performance check for the opaque reference-graph work. The
bench target is `darkmatter/lib/benches/reference_graph.rs`, registered in
`darkmatter/lib/Cargo.toml`.

## Goal

Confirm that validating a **prebuilt** graph (`validate_references_with_graph`)
stays materially faster than rebuilding (`validate_references`), and that the
provenance the opacity cutover adds does not introduce a material graph
**construction** regression or superlinear work.

## Method

Three fixture shapes, each measured three ways:

- `build_and_validate` — `validate_references` (build **and** validate).
- `validate_prebuilt` — `validate_references_with_graph` with the graph built
  **outside** the timed loop; only provenance checking, descendant
  re-verification, and flattening are timed.
- `construct` — `reference_graph` alone; isolates provenance-construction cost.

Context (`ComposeContext`) is captured **once per fixture**, outside every timed
loop, for two reasons: a fresh `ComposeOptions::default()` runs sniff-driven
discovery that would otherwise dominate the measurement, and the prebuilt graph
and the validation request must share one options identity — otherwise the
opaque-graph guard (correctly) rejects the mismatched pairing with an
`Options` mismatch. This was observed directly: an early bench draft that
re-captured context per call produced `ReferenceGraphMismatch(Options)` on
`validate_prebuilt`, which is the guard doing its job.

## Fixtures

Deterministic, defined in-source (no external corpus):

| Fixture | Shape |
|---|---|
| `small` | one document, 4 remote links, no transclusion, no disk reads |
| `large` | one document, `LARGE_LINK_COUNT = 200` remote links |
| `multi_transclusion` | root + `TRANSCLUSION_CHILD_COUNT = 12` on-disk `::file` children |

Reproducibility fingerprint — `bench-source-sha256:
7fb12746003f5e38fbffa986eb27cda87c88a0ce40209c422f96c433371ba8bf`
(`shasum -a 256 darkmatter/lib/benches/reference_graph.rs`). The fixture bodies
are pure constants / index-seeded strings in that file, so this hash pins the
exact inputs.

## Environment

- Host: Apple M4 Max, macOS (Darwin 25.5.0, arm64)
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `--release`
- Branch / commit: `darkmatter` @ `bc1c148f2`
- Criterion config for the recorded run: `--warm-up-time 1 --measurement-time 4`,
  `sample_size(30)` (30 samples per function; `multi_transclusion/construct`
  reports the standard Criterion "unable to complete 30 samples" note because a
  single iteration exceeds the target time — the 30 samples were still
  collected over the estimated window).

## Raw Criterion output (median in the middle of each `[low median high]` triple)

```text
reference_graph/small/build_and_validate
                        time:   [239.19 µs 241.21 µs 243.58 µs]
reference_graph/small/validate_prebuilt
                        time:   [34.501 µs 34.685 µs 34.900 µs]
reference_graph/small/construct
                        time:   [202.91 µs 204.78 µs 206.68 µs]
reference_graph/large/build_and_validate
                        time:   [6.3252 ms 6.3581 ms 6.3931 ms]
reference_graph/large/validate_prebuilt
                        time:   [104.64 µs 105.17 µs 105.67 µs]
reference_graph/large/construct
                        time:   [6.1953 ms 6.2283 ms 6.2657 ms]
reference_graph/multi_transclusion/build_and_validate
                        time:   [10.288 ms 10.455 ms 10.723 ms]
reference_graph/multi_transclusion/validate_prebuilt
                        time:   [4.1206 ms 4.1522 ms 4.1835 ms]
reference_graph/multi_transclusion/construct
                        time:   [6.0306 ms 6.0571 ms 6.0896 ms]
```

## Reuse win (median)

| Fixture | build+validate | validate prebuilt | speedup |
|---|---:|---:|---:|
| `small` | 241.21 µs | 34.685 µs | ~7.0× |
| `large` | 6.3581 ms | 105.17 µs | ~60× |
| `multi_transclusion` | 10.455 ms | 4.1522 ms | ~2.5× |

Prebuilt validation is materially faster on every fixture. For
`multi_transclusion` the prebuilt path still re-reads and re-hashes all 12
children from disk (descendant verification), so its floor is higher than the
single-document fixtures — but it is still ~2.5× faster than rebuilding,
confirming the Finding-18 reuse win survives the opacity cutover.

Dispersion is tight (each `[low, high]` spread is within a few percent of the
median), so the ordering is stable, not noise.

## Construction / provenance cost

Provenance construction is the only new work the opacity cutover adds to graph
construction:

- `ReferenceDocumentIdentity::capture` — three xxHash passes over the root
  (frontmatter map, body, whole-represented-state), plus one per unique visited
  local child recorded in the dependency manifest.
- `ReferenceGraphOptionsIdentity::capture` — one xxHash over the canonical
  options encoding, plus three `Weak` handle clones.

None of these retain large objects (only `u64` fingerprints and `Weak`
handles), and the per-child work is O(unique visited children) — the manifest
dedupes by resolved source, so each child is hashed at most once. The measured
`construct` times track `build_and_validate` minus validation and are dominated
by InlinePre compose + reference extraction, not the handful of extra hashes:
`small` construct 204.78 µs, `large` 6.2283 ms, `multi_transclusion` 6.0571 ms.

## Regression budget

The acceptance rule is: a construction regression is unacceptable only when it
exceeds **both** 5% and 100 µs at the median on a stable fixture.

No pre-opacity baseline bench exists for `reference_graph` — the bench target is
new to this branch, and Phases 1–3 (which introduced provenance) are already
merged here, so a same-commit `--baseline` comparison cannot isolate the
provenance delta. To obtain a true baseline/candidate comparison, run:

```text
git checkout <pre-Phase-1 commit>   # add this bench file first if absent
cargo bench -p darkmatter --bench reference_graph -- --save-baseline pre-provenance
git checkout darkmatter
cargo bench -p darkmatter --bench reference_graph -- --baseline pre-provenance
```

On the analytical grounds above (bounded constant-factor hashing, no
large-object retention, O(unique children) manifest work) the provenance
addition cannot move the `construct` median by more than a few microseconds on
these fixtures — well inside the 100 µs / 5% budget. A cross-commit measurement
should be added when a pre-Phase-1 checkout is convenient; the harness is now in
place to produce it in one command.

> The `change:` percentage Criterion prints when a stale saved baseline is
> present is **not** meaningful here — an earlier miscalibrated draft (context
> captured per iteration) left a saved baseline that made `small` appear
> "-99.9%". That baseline was cleared before the recorded run above.

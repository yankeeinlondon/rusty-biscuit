# F13 / F14 — raw-sample re-measurement

Supersedes the F13 and F14 measurements in
[`../run-20260716T085358/summary.md`](../run-20260716T085358/summary.md), which
review-1 rejected for retaining only Criterion's derived `estimates.json`
fields. Every number here is recomputed from the sample vectors beside this file.

**Both claims reproduce.** No correction to `results.md` is required for F13 or
F14.

| Claim | Original | **Re-measured** | Verdict |
|---|---|---|---|
| F13 `apply_replacements` matcher | 2.371 ms -> 0.087 ms (~27x) | **2.3668 ms -> 87.54 us (27.0x, -96.3 %)** | Reproduces |
| F14 skipped scan vs guard | 240.1 us -> 2.3 us (~104x) | **247.94 us -> 2.33 us (106.5x)** | Reproduces |

## What changed in the harness (F13)

The original F13 baseline was captured by reverting source files and running a
**second, separately compiled** binary. On this shared host that is unsound:
Phase 10 recorded identical code drifting +50 % across runs, and the F33
re-measurement in this same session showed a spurious -19 % common-mode shift
produced by exactly that method.

`phase6_interpolation.rs` therefore now carries the pre-Finding-13
`scan_and_replace` — copied verbatim from `b425fb466` — as a pinned baseline
beside the candidate, benched **interleaved in one process** under the new
`f13_scan_and_replace` group. This is the Phase-10 precedent (`phase10_residuals`
does the same for 35.2), with one deliberate difference: **this harness is
retained, not deleted**, because the deleted temporary harnesses are precisely
what review-1 faulted.

An equivalence gate runs before any timing: the bench asserts the pinned baseline
and the shipped `apply_replacements` produce byte-identical output **and** an
identical replacement count over the `replace_heavy` body. A ratio is never
reported between two algorithms that disagree, and the pinned copy cannot
silently drift from the algorithm it claims to represent.

`apply_replacements_direct` (candidate-only, pre-existing) is retained unchanged
and still recorded, but it cannot on its own support a speed-up claim.

## Environment

| Item | Value |
|------|-------|
| Run id | `run-20260716T153700` (2026-07-16 15:37 local) |
| Host | Apple M4 Max (Mac16,5), macOS Darwin 25.5.0, arm64 |
| Load average | ~5.9 -> ~8.3 (1-min) — quiet window |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`, pinned `stable` |
| Criterion | 0.5.1 |
| Profile | `bench` (release, `harness = false`) |
| Command | `cargo bench -p darkmatter --bench phase6_interpolation -- 'f13_scan_and_replace|f14_|apply_replacements_direct'` |
| TTY mode | non-interactive, piped |
| Warm-up / samples | 3 s warm-up; **50 samples** (`group.sample_size(50)`), `Linear` sampling |

Fixtures (`../../../manifest.yaml`, generator 1.3.0): `replace_heavy`
(40944 bytes, xxHash64 `a5510e6aea6f4d31`) for F13; `toc_large` (80936 bytes,
xxHash64 `a7ec984add997455`) for F14.

## Retained raw samples

Criterion `<bench>/new/sample.json` (parallel `iters` / `times` vectors;
per-iteration time is `times[i] / iters[i]`):

```
f13_scan_and_replace-baseline-sample.json
f13_scan_and_replace-candidate-sample.json
apply_replacements_direct-sample.json
f14_baseline_markdown_scan-sample.json
f14_candidate_contains_guard-sample.json
```

Recompute:

```
bun darkmatter/features/2026-07-15-performance-followup/benchmarks/recompute.ts \
    darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T153700
```

## Recomputed statistics

| benchmark | n | mean | 95 % CI (mean) | median | std dev | min | max |
|---|---|---|---|---|---|---|---|
| `f13_scan_and_replace-baseline` | 50 | 2.3662 ms | [2.3562 ms, 2.3768 ms] | 2.3668 ms | 37.5895 us | 2.2928 ms | 2.5036 ms |
| `f13_scan_and_replace-candidate` | 50 | 105.8451 us | [95.3610 us, 119.7586 us] | 87.5374 us | 44.8931 us | 84.7850 us | 355.0833 us |
| `apply_replacements_direct` | 50 | 87.1806 us | [86.5467 us, 87.9519 us] | 86.7120 us | 2.5894 us | 83.7415 us | 99.8401 us |
| `f14_baseline_markdown_scan` | 50 | 250.7417 us | [248.7446 us, 253.1874 us] | 247.9391 us | 8.0870 us | 243.6044 us | 291.3053 us |
| `f14_candidate_contains_guard` | 50 | 2.3292 us | [2.3213 us, 2.3372 us] | 2.3284 us | 28.6260 ns | 2.2824 us | 2.4085 us |

**Dispersion note — read F13's candidate on the median, not the mean.** Its
sample carries a single scheduling excursion (max 355 us against a min of
84.8 us), which drags the mean to 105.8 us and widens its CI. The median
(87.54 us) and the independently benched `apply_replacements_direct` median
(86.71 us) agree to within 1 %, and both match the original run's recorded
87 us. The excursion is host noise, not matcher behavior; it is reported rather
than trimmed, and the retained vector lets anyone confirm that reading.

### F13 — `scan_and_replace` over the 43-rule `replace_heavy` body

| variant | median | 95 % CI (mean) |
|---|---|---|
| baseline (per-character `starts_with` retry, `O(content x rules x keylen)`) | **2.3668 ms** | [2.3562 ms, 2.3768 ms] |
| candidate (Aho-Corasick `LeftmostLongest`, one linear pass) | **87.54 us** | [95.36 us, 119.76 us] |

**27.0x faster (-96.3 %).** Baseline and candidate CIs are separated by more
than an order of magnitude. Predeclared threshold — any repeatable out-of-noise
win with byte-identical output on the canonical precedence — met, with output
equality asserted in-process before timing.

### F14 — the parse skipped on a `{{`-free body

| operation | median |
|---|---|
| `f14_baseline_markdown_scan` (`ExpressionFinder::new(body).find_all()`) | **247.94 us** |
| `f14_candidate_contains_guard` (`body.contains("{{")`) | **2.33 us** |

**106.5x less work per compose** for every `{{`-free body. Both are sampled in
one process, so the ratio needs no drift argument. (The original recorded
~104x; the 2 % difference is run-to-run variation, and the re-measured figure is
if anything slightly more favorable.)

## Cross-platform

Unchanged: F13 and F14 are OS-identical (in-memory byte automaton / byte scan;
no `cfg`, filesystem, or platform branch). The added bench code is test-tier
only and introduces no production path.

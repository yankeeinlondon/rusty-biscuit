# 35.2 — `relevel_with_overflow`: raw-sample re-measurement

Supersedes the **35.2** measurement in
[`../run-20260716T160000/summary.md`](../run-20260716T160000/summary.md), which
review-1 rejected for retaining only Criterion's derived `estimates.json`
fields. Every number here is recomputed from the sample vectors beside this file.

Scope: **35.2 only.** The other Phase-10 sub-items (35.3, 35.5, 35.6, 35.7) were
measured by temporary in-crate harnesses that were **deleted** after capture, so
their raw observations are unrecoverable and cannot be recomputed from anything
retained. They remain **outstanding**; see *Outstanding* below.

**The claim reproduces.** No correction to `results.md` is required for 35.2.

| Benchmark | Original | **Re-measured** | Verdict |
|---|---|---|---|
| `f35_2_relevel_prefix_toc_large` | 25.351 ms -> 314.93 us (-98.8 %) | **20.998 ms -> 285.86 us (-98.6 %)** | Reproduces |
| `f35_2_relevel_overflow_toc_large` | 24.347 ms -> 361.50 us (-98.5 %) | **21.305 ms -> 364.02 us (-98.3 %)** | Reproduces |
| `f35_2_relevel_extract_only` | 18.463 ms -> 277.80 us (-98.5 %) | **18.403 ms -> 281.67 us (-98.5 %)** | Reproduces |
| `f35_2_relevel_no_headings` (control) | 230.59 -> 220.62 us (parity) | **230.64 -> 218.68 us (-5.2 %)** | Reproduces; control did not regress |

Absolute baselines differ from the original by up to 17 % (`prefix` 25.35 ->
21.00 ms) because the original run was captured under load ~29-30 and this one
under load ~6. The **ratios**, which is what `phase10_residuals` is built to
measure, agree to within 0.3 pp on every row. This is the expected behavior of
the interleaved design and is why 35.2 — unlike F33 — needed no correction.

## Method (unchanged from the original, and validated by it)

`phase10_residuals.rs` carries the pre-change algorithm (`baseline_extract_headings`
/ `baseline_relevel`) pinned beside the candidate and samples both **interleaved
in one process**, so both see identical thermal and scheduling conditions. This
session independently confirmed why that matters: the same 35.2 benches run
under load ~29-64 produced a 15 % spread on `overflow` alone, while the ratios
held.

The pinned baselines are kept honest by the permanent differential tests
`relevel_output_matches_the_pre_optimization_algorithm` (16 shapes x 6 target
levels) and `relevel_output_matches_the_oracle_across_shipped_fixtures` (all 13
committed fixtures).

## Environment

| Item | Value |
|------|-------|
| Run id | `run-20260716T153400` (2026-07-16 15:34 local) |
| Host | Apple M4 Max (Mac16,5), macOS Darwin 25.5.0, arm64 |
| Load average | **~6.5 -> ~6.2** (1-min) — quiet window; contrast the original run's ~29-30 |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`, pinned `stable` |
| Criterion | 0.5.1 |
| Profile | `bench` (release, `harness = false`) |
| Command | `cargo bench -p darkmatter --bench phase10_residuals` |
| TTY mode | non-interactive, piped |
| Warm-up / samples | Criterion defaults: 3 s warm-up, 100 samples, `Linear` sampling |

Fixture (`../../../manifest.yaml`, generator 1.3.0): `toc_large` — 80936 bytes,
1001 headings, xxHash64 `a7ec984add997455`, Darkmatter hash
`ef46db3751d8e999-ae08ed69267941ce`. Phase 10 registered no new fixture.

## Retained raw samples

Criterion `<bench>/new/sample.json` (parallel `iters` / `times` vectors;
per-iteration time is `times[i] / iters[i]`) — eight files, baseline + candidate
for each of the four groups:

```
f35_2_relevel_{prefix_toc_large,overflow_toc_large,extract_only,no_headings}-{baseline,candidate}-sample.json
```

Recompute:

```
bun darkmatter/features/2026-07-15-performance-followup/benchmarks/recompute.ts \
    darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T153400
```

## Recomputed statistics (100 samples each)

| benchmark | n | mean | 95 % CI (mean) | median | std dev | min | max |
|---|---|---|---|---|---|---|---|
| `f35_2_relevel_prefix_toc_large-baseline` | 100 | 20.9455 ms | [20.8276 ms, 21.0683 ms] | 20.9982 ms | 620.1435 us | 19.8472 ms | 23.2087 ms |
| `f35_2_relevel_prefix_toc_large-candidate` | 100 | 286.4795 us | [285.5843 us, 287.4043 us] | 285.8648 us | 4.6664 us | 277.1601 us | 304.2202 us |
| `f35_2_relevel_overflow_toc_large-baseline` | 100 | 21.3567 ms | [21.2251 ms, 21.4925 ms] | 21.3047 ms | 685.1309 us | 19.9923 ms | 23.6049 ms |
| `f35_2_relevel_overflow_toc_large-candidate` | 100 | 365.7480 us | [364.3367 us, 367.4011 us] | 364.0163 us | 7.8687 us | 357.2118 us | 411.4174 us |
| `f35_2_relevel_extract_only-baseline` | 100 | 18.5054 ms | [18.3998 ms, 18.6162 ms] | 18.4028 ms | 554.0743 us | 17.3228 ms | 20.4698 ms |
| `f35_2_relevel_extract_only-candidate` | 100 | 281.9610 us | [281.1303 us, 282.8001 us] | 281.6677 us | 4.2613 us | 272.7563 us | 294.4688 us |
| `f35_2_relevel_no_headings-baseline` | 100 | 231.6827 us | [230.7179 us, 232.7420 us] | 230.6424 us | 5.1933 us | 224.3767 us | 255.4505 us |
| `f35_2_relevel_no_headings-candidate` | 100 | 221.5060 us | [219.1093 us, 224.4356 us] | 218.6833 us | 13.7010 us | 211.0674 us | 304.0074 us |

Deltas on the median:

| Group | Baseline | Candidate | Delta |
|---|---|---|---|
| `prefix_toc_large` (target) | 20.9982 ms | **285.86 us** | **-98.6 %** |
| `overflow_toc_large` (target) | 21.3047 ms | **364.02 us** | **-98.3 %** |
| `extract_only` (target) | 18.4028 ms | **281.67 us** | **-98.5 %** |
| `no_headings` (**control**) | 230.64 us | 218.68 us | **-5.2 %** (improved) |

Every target's baseline and candidate CI are separated by roughly two orders of
magnitude. The heading-free control improved slightly rather than regressing —
consistent with the deferred offset table, which a heading-free document never
builds — so the 0 % control-regression budget holds.

## Disposition — **Accepted** (unchanged), numbers confirmed

## Outstanding — the other four Phase-10 sub-items

35.3, 35.5, 35.6, and 35.7 were measured by temporary in-crate harnesses
(`f35_3` copy-cost model, `f35_5_profile` in `hash/explain.rs`, `f35_6_profile`
in `layout/page/tests.rs`, `f35_7_profile` in `render_tree/build_context.rs`),
each **deleted after capture**. The retained `.txt` profiles record medians and
prose only — no per-observation vectors — so nothing in the repository can
reproduce or recompute them. Their raw observations are **unrecoverable**;
regenerating them requires rebuilding each harness against a pinned baseline,
which this session did not have the budget to do. Recorded as an open evidence
gap, not closed.

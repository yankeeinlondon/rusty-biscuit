# Finding 35.6 — `normalize_body_rhythm` ANSI-strip allocation (retained re-capture)

Run id: `run-20260716T182214-f35-6`

Supersedes `run-20260716T160000/f35_6-rhythm-profile.txt`, whose harness
(`f35_6_profile`) was **deleted after capture**, leaving its numbers
unreproducible. This run re-measures the same three cases through a **retained**
harness and retains the per-observation vectors.

## Harness (retained)

- `darkmatter/lib/src/layout/page/tests.rs` → `finding_35_6::f35_6_rhythm_raw_samples`
- Pinned baseline: `finding_35_6::naive_normalize` / `naive_is_blank` (already
  committed alongside the equivalence tests).
- `#[ignore]`d **and** gated on `Harness::from_env` returning `Some`, so `just
  test` neither runs nor is slowed by it. Adds no public API (`#[cfg(test)]`).

`normalize_body_rhythm` is private; exposing it for a Criterion bench would be
exactly the public-API addition the feature's compatibility invariant 2 bars, so
it is measured in-crate.

## Commits

| role | commit |
|---|---|
| baseline (what the pinned copy reproduces) | `1224c05b0` — parent of the F35.6 change |
| candidate | working tree at `a80e032c3` (`normalize_body_rhythm` unchanged since `727bd5da3`) |

Baseline fidelity re-verified for this run: `naive_normalize` / `naive_is_blank`
are verbatim `git show 1224c05b0:darkmatter/lib/src/layout/page.rs`
(`fn normalize_body_rhythm`, lines 1423–1448). No drift.

## Commands

```
cd darkmatter
DM_PERF_RAW_DIR=darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T182214-f35-6 \
  cargo nextest run -p darkmatter --lib --release --run-ignored all \
  -E 'test(f35_6_rhythm_raw_samples)' --no-capture
```

Recomputation — every statistic below is reproducible by this command from the
vectors committed beside this file:

```
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun recompute.ts raw/f35-residuals/run-20260716T182214-f35-6
```

## Environment

- Profile: **release** (`--release`; workspace release profile).
- Host: macOS 26.5.2 (Darwin 25.5.0), Apple M4 Max, arm64, 16 CPUs. Shared host.
- TTY mode: piped (nextest `--no-capture` into a pipe); non-interactive session.
- Tool versions: rustc 1.96.0 (`ac68faa20`), cargo 1.96.0 (`30a34c682`),
  cargo-nextest 0.9.136, bun 1.3.3. No Criterion / hyperfine (in-crate harness).
- Host load average: **12.45 / 26.10 / 31.13 before**, **11.70 / 25.72 / 30.97 after**.
- Warm-up: 3 batches per arm. Samples: **100** per arm, batch size 1.
- Statistic: **median** (per-iteration). Dispersion: std dev + bootstrap 95 % CI
  of the mean, both recomputed by `recompute.ts`.

No drift bracket is needed: baseline and candidate are sampled **interleaved
sample-by-sample in one process**, so both see the same thermal and scheduling
conditions. This was verified empirically rather than assumed — see *Load
sensitivity* below.

## Fixtures (manifest identities, Architecture Decision A)

| case | source | lines (observed) |
|---|---|---|
| decorated prose (SGR) | `toc_medium.md` rendered through `DarkmatterPage` (margin 2, padding 1, `PageBackground::Subtle`, `Terminal::new_optimistic(100)`) | **549** |
| code panel (bg fill) | `render_code_heavy.md`, same page path | **329** |
| plain (escape-free, control) | raw `toc_medium.md` text | **531** |

The three line counts reproduce the deleted harness's recorded counts (549 / 329
/ 531) exactly — the available evidence that this reconstruction measures the
same bodies. No fixture was added; only committed manifest fixtures are read.

## Predeclared threshold

Carried over **verbatim** from `run-20260716T160000/f35_6-rhythm-profile.txt`,
where it was declared *before* the original baseline capture:

> Target operation: the `normalize_body_rhythm` pass on a decorated body.
> Minimum repeatable win: >= 10% on the decorated-prose target.
> Maximum permitted control regression: 0% on the escape-free body.

## Equivalence gate

`normalize_body_rhythm(body) == naive_normalize(body)` is asserted for **every
one of the three measured bodies before any timing runs**. A ratio between two
functions that disagree is not a result.

## Results (median per-iteration, recomputed)

| case | lines | baseline | candidate | change | recorded claim |
|---|---|---|---|---|---|
| decorated prose (SGR) | 549 | 169.8745 µs | 15.5410 µs | **−90.9 %** | −91.1 % — **confirmed** |
| code panel (bg fill) | 329 | 134.3335 µs | 9.2500 µs | **−93.1 %** | −93.3 % — **confirmed** |
| plain (escape-free) | 531 | 28.1045 µs | 21.5840 µs | **−23.2 %** | −30.3 % — **CONTRADICTED** |

Bootstrap 95 % CIs of the mean are disjoint on all three cases by wide margins
(prose baseline [171.06, 172.25] vs candidate [15.52, 15.65] µs; control
[27.83, 28.39] vs [21.55, 21.80] µs). No case is near the measurement floor.

## Load sensitivity (why the ratios are trustworthy and the control is not exact)

This host is shared and was heavily contended during this session (observed load
excursions to **148**). The harness was therefore run five times across a wide
load range to separate real ratios from host noise:

| load at capture | decorated prose | plain control |
|---|---|---|
| ~16 | −91.2 % | −28.1 % |
| ~31 | −90.9 % | −23.3 % |
| ~11–23 | −90.8 % | −24.4 % |
| ~30 | −90.9 % | −26.7 % |
| ~42 | −91.8 % | −22.1 % |
| **~12 (this record)** | **−90.9 %** | **−23.2 %** |

The decorated-prose ratio is **load-invariant** (−90.8 % … −91.8 % across load
11→42) even as its absolute baseline moved 169 → 416 µs. That is interleaving
working as designed, and it is why the headline result is reported with
confidence. The control ratio is **not** stable (−22.1 % … −28.1 %), because it
is the smallest absolute (20–30 µs) and so the most noise-exposed.

## Honest reading vs the recorded claim

- The two headline decorated cases **reproduce**: −90.9 % vs a recorded −91.1 %,
  and −93.1 % vs a recorded −93.3 %.
- **CONTRADICTION (escape-free control):** measured **−23.2 %** here, against a
  recorded **−30.3 %**. The recorded figure was **not reproduced in any of six
  captures** — the observed range is −22.1 % … −28.1 %, and −30.3 % sits outside
  it. The honest statement is: *the escape-free control improves by roughly
  20–28 %, load-dependent; it does not regress.* The disposition is unaffected
  (the budget is 0 % regression, and every capture is an improvement), but
  −30.3 % should not be quoted.
- **Not reproduced here:** the record's end-to-end figures (`full decorated
  DarkmatterPage::render` = 582.5 µs, rhythm-pass share 2.55 %, and the
  reconstructed "~20 % faster decorated render"). Those came from the same
  deleted harness and still have **no retained samples**. This run does not
  substantiate them and nothing above depends on them.

## Disposition

**ACCEPTED (implementation win)** — unchanged, now on retained, reproducible
evidence.

The predeclared floor is met on the target operation by a wide margin and
load-invariantly: **−90.9 %** against a >= 10 % floor. The control did not
regress in any capture, against a 0 % permitted-regression budget.

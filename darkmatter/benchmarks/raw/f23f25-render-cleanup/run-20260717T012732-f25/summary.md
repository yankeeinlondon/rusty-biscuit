# Run record — Finding 25, retained cleanup-pass profile + fusion cost model

Supersedes the F25 half of
[`../run-20260716T120000/`](../run-20260716T120000/summary.md), whose evidence
was `f25-cleanup-pass-profile.txt` — a **derived-median text table** from a
harness that was then deleted. Review-2 rejected it: nothing could be
recomputed and the claim could not be reproduced. **This run corrects that
record's ceiling figure — see *Correction*.**

- **Run id:** run-20260717T012732-f25 (2026-07-17 01:27:32 UTC)
- **Host:** Apple M4 Max (Mac16,5), macOS (Darwin 25.5.0), shared
  non-interactive sandbox. Piped (non-TTY) process.
- **Host load (1/5/15 min):** `15.95 13.84 18.18` before, `14.99 13.68 18.09`
  after.
- **Commit:** `a80e032c3` (branch `darkmatter`).
- **Build profile:** `release`, stable toolchain per `rust-toolchain.toml`,
  workspace lockfile unchanged.
- **Tool versions:** `rustc 1.96.0 (ac68faa20 2026-05-25)`,
  `cargo 1.96.0 (30a34c682 2026-05-25)`,
  `cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`, `bun 1.3.3`. Criterion and
  hyperfine are not used by this harness.
- **Fixtures (immutable, hashed in `manifest.yaml`, generator 1.2.0):**
  `toc_large` (80936 bytes) and `replace_heavy` (40944 bytes). No new fixture
  was registered.
- **Harness (retained):**
  `darkmatter/lib/src/markdown/cleanup/perf_profile.rs`,
  `f25_cleanup_profile_raw_samples`, built on the shared
  `darkmatter/lib/src/perf_harness.rs`. Both are `#[cfg(test)]` — no public API
  was added or widened. The test is `#[ignore]`d **and** gated on
  `DM_PERF_RAW_DIR`.

## Command

```
cd darkmatter
DM_PERF_RAW_DIR=darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260717T012732-f25 \
  cargo nextest run -p darkmatter --lib --release --run-ignored all \
  -E 'test(f25_cleanup_profile_raw_samples)' --no-capture
```

Recomputation — every statistic below is produced by this command from the
vectors committed beside this file:

```
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun recompute.ts raw/f23f25-render-cleanup/run-20260717T012732-f25
```

## Method

F25's recorded disposition is **no-win, not implemented**, so there is no
candidate to A/B and nothing to equivalence-gate against a baseline. The
evidence is therefore (a) a stage profile and (b) a fusion **cost model**.

- **Warm-up:** 3 batches per stage. **Samples:** 50 per stage, batch 10.
- **Statistic:** mean, with a bootstrap 95% CI from `recompute.ts`.
- Stage replicas call the same private functions `cleanup_content_internal`
  calls, in the same order, on the same bytes. Each stage-2 sample restores its
  input with a `String` clone; `f25-<fixture>-stage2-string-clone` prices that
  clone alone so it can be subtracted.
- **Nothing was implemented.** `cleanup/mod.rs` pass order and canonical output
  are untouched; the only change is the added `#[cfg(test)] mod perf_profile`.

### The cost model (labelled as such)

Every stage-2 line pass has the same shape: walk `output.lines()`, rebuild a
fresh `String::with_capacity(output.len())`. Fusion can remove the *repeated*
walk-and-rebuild; it cannot remove any pass's per-line work, which a fused scan
still performs. `scan_rebuild` is that skeleton with all per-line work removed,
run over the real stage-2 bytes.

The two arms are **equivalence-gated against each other** (`scan_rebuild` is
idempotent, so 7× must equal 1× byte-for-byte) and yield the **marginal** cost
of one additional scan, `(seven − one) / 6` — the correct unit, since only the
first scan in a batch pays first-touch page faults.

The ceiling is then `(N − 1) × unit`, where `N` is the number of stage-2 passes
that actually scan. `N` is **read off the retained per-pass vectors, not
assumed**: a pass whose whole cost is below one unit cannot be performing one.
That gives an upper bound on `N`, hence on the ceiling. This matters — several
passes early-out without scanning at all, so charging fusion for all seven
overstates the prize ~3×.

## Predeclared threshold

Carried over **verbatim** from `run-20260716T120000/summary.md`:

- **F25 target:** "an end-to-end `cleanup_content` win over the same fixtures
  from fusing line passes. **No-win rule (plan, Phase 8):** fusion within noise,
  or added allocation/complexity without a repeatable end-to-end gain, closes as
  a recorded no-win with no speculative code retained."

## Results — `toc_large` (80936 bytes)

| stage | n | mean | 95% CI (mean) | share of `cleanup_content` |
|---|---|---|---|---|
| `f25-toc-large-cleanup-content` | 50 | **1.2561 ms** | [1.2521 ms, 1.2604 ms] | 100% |
| `f25-toc-large-strip-incidental-newlines` | 50 | 285.0946 µs | [282.3162 µs, 287.5775 µs] | 22.7% |
| `f25-toc-large-stage1-parse` | 50 | 276.8021 µs | [275.3399 µs, 278.3933 µs] | 22.0% |
| `f25-toc-large-stage1-cmark-serialize` | 50 | 216.4278 µs | [215.2755 µs, 217.4943 µs] | 17.2% |
| `f25-toc-large-stage1-add-text-language` | 50 | 129.0102 µs | [128.6817 µs, 129.3761 µs] | 10.3% |
| `f25-toc-large-stage1-align-tables` | 50 | 112.1861 µs | [111.9746 µs, 112.4097 µs] | 8.9% |
| `f25-toc-large-stage1-preserve-emphasis` | 50 | 57.1145 µs | [56.7568 µs, 57.4610 µs] | 4.5% |
| `f25-toc-large-stage1-extract-list-markers` | 50 | 7.9956 µs | [7.9315 µs, 8.0961 µs] | 0.6% |
| `f25-toc-large-stage1-events-clone` (sample overhead) | 50 | 64.7413 µs | [64.3444 µs, 65.1459 µs] | — |

Stage-2 line passes (each mean includes one `String` clone of 1.0998 µs
[1.0574 µs, 1.1552 µs]):

| pass | n | mean | 95% CI (mean) | scans? |
|---|---|---|---|---|
| `f25-toc-large-stage2-fix-blockquote-formatting` | 50 | 110.6532 µs | [110.4083 µs, 110.9035 µs] | **yes** |
| `f25-toc-large-stage2-normalize-list-spacing` | 50 | 110.0679 µs | [109.2459 µs, 110.9184 µs] | **yes** |
| `f25-toc-large-stage2-fix-list-indentation` | 50 | 65.7996 µs | [64.7692 µs, 66.7111 µs] | **yes** |
| `f25-toc-large-stage2-unescape-emphasis-chars` | 50 | 6.9372 µs | [6.8711 µs, 7.0184 µs] | no — below one unit |
| `f25-toc-large-stage2-restore-emphasis-placeholders` | 50 | 6.8775 µs | [6.8088 µs, 6.9648 µs] | no — below one unit |
| `f25-toc-large-stage2-unescape-brackets` | 50 | 4.0885 µs | [4.0013 µs, 4.1790 µs] | no — below one unit |
| `f25-toc-large-stage2-restore-list-markers` | 50 | 1.0398 µs | [1.0335 µs, 1.0456 µs] | no — below one unit |
| `f25-toc-large-stage2-trailing-trim` | 50 | 3.4813 µs | [3.4652 µs, 3.4988 µs] | no — below one unit |

Sum of the seven line passes: **305.46 µs**; net of 7 clones (7 × 1.0998 µs):
**297.76 µs = 23.7%** of `cleanup_content`.

### Cost model — `toc_large`

| arm | n | mean | 95% CI (mean) |
|---|---|---|---|
| `f25-toc-large-costmodel-one-scan-rebuild` | 50 | 51.4243 µs | [51.2140 µs, 51.6511 µs] |
| `f25-toc-large-costmodel-seven-scan-rebuild` | 50 | 352.1617 µs | [351.3233 µs, 353.0772 µs] |

- Marginal cost of one scan: `(352.1617 − 51.4243) / 6` = **50.12 µs**.
- Passes above one unit (50.12 µs): **3** (`fix_blockquote_formatting`,
  `normalize_list_spacing`, `fix_list_indentation`). The other four cost
  1.0–6.9 µs — an order of magnitude below a single scan, so they demonstrably
  do not perform one. Hence `N ≤ 3`.
- **Ceiling ≤ (3 − 1) × 50.12 µs = 100.2 µs = 7.98% of `cleanup_content`.**
- Against Phase 5's `md compose` measurement (~19 ms ± 0.5 ms): **≈0.53% of one
  compose.**

## Results — `replace_heavy` (40944 bytes)

| stage | n | mean | 95% CI (mean) | share |
|---|---|---|---|---|
| `f25-replace-heavy-cleanup-content` | 50 | **151.0011 µs** | [150.0856 µs, 151.8067 µs] | 100% |
| `f25-replace-heavy-strip-incidental-newlines` | 50 | 104.4657 µs | [103.5213 µs, 105.4553 µs] | **69.2%** |
| `f25-replace-heavy-stage1-parse` | 50 | 26.7372 µs | [26.0764 µs, 27.4857 µs] | 17.7% |
| `f25-replace-heavy-stage1-cmark-serialize` | 50 | 3.5180 µs | [3.5103 µs, 3.5258 µs] | 2.3% |
| `f25-replace-heavy-stage1-align-tables` | 50 | 133.7460 ns | [132.4140 ns, 135.0780 ns] | 0.1% |
| `f25-replace-heavy-stage1-add-text-language` | 50 | 127.8360 ns | [126.7580 ns, 128.8440 ns] | 0.1% |
| `f25-replace-heavy-stage1-preserve-emphasis` | 50 | 60.2560 ns | [59.6760 ns, 60.8380 ns] | 0.0% |
| `f25-replace-heavy-stage1-extract-list-markers` | 50 | 5.4180 ns | [4.9160 ns, 5.9900 ns] | 0.0% |

Stage-2 line passes (each mean includes one `String` clone of 776.7480 ns):

| pass | n | mean | 95% CI (mean) |
|---|---|---|---|
| `f25-replace-heavy-stage2-fix-blockquote-formatting` | 50 | 4.0075 µs | [3.9759 µs, 4.0520 µs] |
| `f25-replace-heavy-stage2-restore-emphasis-placeholders` | 50 | 2.8580 µs | [2.8404 µs, 2.8904 µs] |
| `f25-replace-heavy-stage2-unescape-emphasis-chars` | 50 | 2.8415 µs | [2.8383 µs, 2.8448 µs] |
| `f25-replace-heavy-stage2-normalize-list-spacing` | 50 | 2.5157 µs | [2.5110 µs, 2.5203 µs] |
| `f25-replace-heavy-stage2-fix-list-indentation` | 50 | 2.3239 µs | [2.3180 µs, 2.3302 µs] |
| `f25-replace-heavy-stage2-unescape-brackets` | 50 | 1.6486 µs | [1.6468 µs, 1.6505 µs] |
| `f25-replace-heavy-stage2-restore-list-markers` | 50 | 516.3300 ns | [510.9140 ns, 521.6700 ns] |
| `f25-replace-heavy-stage2-trailing-trim` | 50 | 1.0172 µs | [1.0150 µs, 1.0193 µs] |

Sum of the seven line passes: **16.71 µs**; net of 7 clones: **11.27 µs =
7.5%** of `cleanup_content`.

### Cost model — `replace_heavy`

| arm | n | mean | 95% CI (mean) |
|---|---|---|---|
| `f25-replace-heavy-costmodel-one-scan-rebuild` | 50 | 2.7046 µs | [2.6358 µs, 2.7676 µs] |
| `f25-replace-heavy-costmodel-seven-scan-rebuild` | 50 | 14.8129 µs | [14.4152 µs, 15.2569 µs] |

- Marginal cost of one scan: `(14.8129 − 2.7046) / 6` = **2.018 µs**.
- Passes above one unit: **5**. Hence `N ≤ 5`.
- **Ceiling ≤ 4 × 2.018 µs = 8.07 µs = 5.35% of `cleanup_content`.**

## Verdict

**No-win — confirmed, and not implemented.** On the largest fixture, the entire
prize fusion could win is **≤7.98% of `cleanup_content`**, i.e. **≈0.53% of one
`md compose`** — below run-to-run σ on that fixture and below the ~1% effect
size the sibling F23 run needed 300 interleaved samples to separate from noise.
The plan's no-win rule applies: no speculative code written, none retained.

The two structural reasons in the recorded disposition are untouched by this run
and still stand: exact equivalence is not cheaply available (the passes are
sequential re-lining rewrites, not independent filters), and the blast radius on
`cleanup_content_internal` is HIGH (35 impacted symbols, 9 direct).

## Correction to the recorded disposition

**The recorded ceiling is contradicted, mildly, and in the direction that
*weakens* the no-win argument** — reported here because the correction goes
against the recorded conclusion, not for it.

| claim | recorded | measured here |
|---|---|---|
| fusion ceiling, `toc_large` | "under ~7% of cleanup" | **≤7.98%** |
| ceiling as share of a ~19 ms compose | "≈0.5%" | **≈0.53%** — confirmed |
| line passes, `toc_large` | ≈282 µs / 22.3% | 297.76 µs / **23.7%** |
| `normalize_list_spacing` | 101.8 µs | **110.07 µs** |
| `fix_blockquote_formatting` | 104.1 µs | **110.65 µs** |
| `fix_list_indentation` | 62.3 µs | **65.80 µs** |
| other four line passes | ≈18 µs | **19.0 µs** — confirmed |
| line passes, `replace_heavy` | 8.8% | **7.5%** |
| `strip_incidental_newlines`, `replace_heavy` | 70.8% | **69.2%** — confirmed |

The recorded "under ~7%" was not measured: it was reasoned as "a fraction of
≈268 µs". The figure here is derived from a retained, equivalence-gated cost
model, and lands slightly *above* 7%. The stage figures are all ~5% higher than
recorded, consistent with a different host-load regime; they are not
independently reproducible against the old record because that record retained
only medians, which is the defect this run exists to fix.

The verdict is unchanged: 7.98% and 7% support the same conclusion.

Raw: `f25-{toc-large,replace-heavy}-*-sample.json` (20 vectors per fixture).

## Cross-platform

No production diff — nothing to classify.

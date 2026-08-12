# F33 — Remote discovery line positions: raw-sample re-measurement

Supersedes the measurement in
[`../run-20260716T140000/summary.md`](../run-20260716T140000/summary.md), which
review-1 rejected for retaining only Criterion's derived `estimates.json`
fields. **Every number below is recomputed from the sample vectors committed
beside this file**, not transcribed from a run whose observations are gone.

## Headline: the original run's figures do not reproduce

The disposition (**ACCEPTED**) survives; the numbers supporting it do not.

| Measurement | Original claim | **Re-measured** | Verdict |
|---|---|---|---|
| Target `f33_discover_remote_heavy` | **-82.5 %** | **-77.5 %** | Win holds; headline was inflated |
| Control 1 `f33_discover_no_http_guard` | -19.3 % | **+0.1 %** (parity) | Claim does not reproduce |
| Control 2 `f33_discover_http_without_expressions` | -19.1 % | **+0.7 %** (parity) | Claim does not reproduce |

The original summary explained its two -19 % control movements at length:
control 1 was attributed to "build/inlining layout drift" and control 2 to a
"genuine, intended win" from the new early return eliding a `ComposeSource::File`
-> `PathBuf` clone. **Neither reproduces.** Both controls sit at parity, with
overlapping 95 % CIs, across two independent candidate runs.

That the effect vanished on a quiet host identifies its cause: the original
baseline and candidate were captured as **separate runs on a host under load
~29-30**, and this feature's own Phase-10 record already documents that
identical unmodified code re-measured across runs on this host drifted **+50 %**
(`f35_2_relevel_no_headings` read 290 -> 397 -> 420 us). A -19 % common-mode
shift across every benchmark in the binary is that drift, not build layout. The
control-2 "genuine win" story is also mechanically implausible in hindsight: one
`PathBuf` clone of a short path is tens of nanoseconds against an 8 us path —
~0.5 %, not 19 %.

The original's own hedge was closer to the truth than its headline. It computed
that "discounting the entire -19 % as build drift, the target's code-specific
win is still ~= **-78 %**". The re-measured target win is **-77.5 %**. The
skeptical reading was right; the headline that led the row was wrong.

## Declared contract (unchanged from the original run record)

| Item | Value |
|------|-------|
| Target operation | `discover_remote_urls_from_expressions` over `remote_heavy` |
| Control 1 | same function over `toc_large` — no `http` substring; must short-circuit on the cheap guard |
| Control 2 | same function over `render_code_heavy` + one appended bare `http` prose URL — passes the guard, parses **no** expression |
| Minimum repeatable win | **>= 30 %** reduction in the target's median |
| Maximum permitted control regression | **<= 5 %** on either control |
| Correctness gate | discovered line numbers equal the naive `content[..offset]` newline count on every fixture and edge case |

**Contract met.** Target -77.5 % clears the >= 30 % floor. Neither control
regressed beyond +0.7 %, inside the <= 5 % budget. Unlike the original, the
verdict now rests on controls that behave the way a target-confined change
predicts — so no discounting argument is needed to defend it.

## Method — and why the baseline is a real revert

`phase9_remote.rs` benchmarks the candidate only, so the baseline cannot be
sampled in-process the way F13's and 35.2's can. It was captured by reverting
the single changed file in place:

```
git show b425fb466:darkmatter/lib/src/markdown/compose/remote.rs \
  > darkmatter/lib/src/markdown/compose/remote.rs   # baseline
cargo bench -p darkmatter --bench phase9_remote      # -> target/criterion/*/new/sample.json
# file restored from backup; `git diff` clean afterwards
```

Fixture bytes, bench bytes, toolchain, and lockfile are identical across all
three runs; only `remote.rs` differs.

**Cross-run drift was measured, not assumed.** Because this is necessarily a
cross-run comparison, the baseline is **bracketed** by two candidate runs
(`candidate_A` before, `candidate_B` after). Their medians differ by **0.7 %**
on the target and **< 0.5 %** on both controls — so the ~77 % target delta is
far outside drift, and the ~0 % control deltas are real parity rather than two
large effects cancelling. This bracket is the check the original run lacked; it
is what makes the -19 % claim falsifiable.

## Environment

| Item | Value |
|------|-------|
| Run id | `run-20260716T153200` (2026-07-16 15:32 local) |
| Host | Apple M4 Max (Mac16,5), macOS Darwin 25.5.0, arm64 |
| **Load average** | **~21 -> ~8, falling** (1-min, sampled each run) — contrast the original run's ~29-30 |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`, pinned `stable` |
| Criterion | 0.5.1 (`Cargo.lock`) |
| Profile | `bench` (release, `harness = false`) |
| Command | `cargo bench -p darkmatter --bench phase9_remote` |
| TTY mode | non-interactive, piped; benchmark performs no terminal I/O |
| Warm-up / samples | Criterion defaults: 3 s warm-up, 100 samples, `Linear` sampling |
| Baseline | `b425fb466`'s `remote.rs`, restored into the HEAD tree |
| Candidate | HEAD `remote.rs` |

## Fixture identity (`../../../manifest.yaml`, generator 1.3.0)

| Fixture | Bytes | xxHash64 | Darkmatter hash |
|---------|-------|----------|-----------------|
| `remote_heavy` (target) | 79028 | `0dc952a78995bde7` | `1796e83f20b84c4a-a63d5c0fd117ae58` |
| `toc_large` (control 1) | 80936 | `a7ec984add997455` | `ef46db3751d8e999-ae08ed69267941ce` |
| `render_code_heavy` (control 2) | 4157 | `2f5d9cca8c854cfd` | `ef46db3751d8e999-9e52d74f25fd5faf` |

## Retained raw samples

Nine `*-sample.json` files beside this summary — Criterion's
`<bench>/new/sample.json`, i.e. parallel `iters` / `times` vectors, one entry
per sample batch. Per-iteration time is `times[i] / iters[i]`.

```
{baseline,candidate_A,candidate_B}-f33_discover_remote_heavy-sample.json
{baseline,candidate_A,candidate_B}-f33_discover_no_http_guard-sample.json
{baseline,candidate_A,candidate_B}-f33_discover_http_without_expressions-sample.json
```

Regenerate every statistic in the next section from those bytes:

```
bun darkmatter/features/2026-07-15-performance-followup/benchmarks/recompute.ts \
    darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T153200
```

## Recomputed statistics (100 samples each, `bun recompute.ts`)

| benchmark | n | mean | 95 % CI (mean) | median | std dev | min | max |
|---|---|---|---|---|---|---|---|
| `baseline-f33_discover_remote_heavy` | 100 | 1.9433 ms | [1.9383 ms, 1.9484 ms] | 1.9401 ms | 26.0416 us | 1.8746 ms | 2.0309 ms |
| `candidate_A-f33_discover_remote_heavy` | 100 | 443.3418 us | [439.7317 us, 447.5220 us] | 438.9958 us | 20.0670 us | 425.0542 us | 547.5348 us |
| `candidate_B-f33_discover_remote_heavy` | 100 | 437.2942 us | [435.3192 us, 439.6875 us] | 435.8209 us | 11.2745 us | 426.3659 us | 514.3412 us |
| `baseline-f33_discover_no_http_guard` | 100 | 2.3227 us | [2.3148 us, 2.3322 us] | 2.3150 us | 44.8519 ns | 2.2627 us | 2.6458 us |
| `candidate_A-f33_discover_no_http_guard` | 100 | 2.3277 us | [2.3178 us, 2.3386 us] | 2.3292 us | 53.2881 ns | 2.2488 us | 2.5448 us |
| `candidate_B-f33_discover_no_http_guard` | 100 | 2.3298 us | [2.3173 us, 2.3450 us] | 2.3181 us | 71.2804 ns | 2.2377 us | 2.7959 us |
| `baseline-f33_discover_http_without_expressions` | 100 | 8.0740 us | [8.0001 us, 8.1592 us] | 7.9693 us | 409.2308 ns | 7.7659 us | 9.6536 us |
| `candidate_A-f33_discover_http_without_expressions` | 100 | 7.9648 us | [7.9192 us, 8.0197 us] | 7.8985 us | 258.1662 ns | 7.6811 us | 9.7460 us |
| `candidate_B-f33_discover_http_without_expressions` | 100 | 8.0397 us | [8.0203 us, 8.0598 us] | 8.0247 us | 101.4644 ns | 7.8422 us | 8.3486 us |

Deltas on the median, baseline -> candidate_B (candidate_A in parentheses):

| Benchmark | Baseline | Candidate | Delta |
|---|---|---|---|
| `f33_discover_remote_heavy` (target) | 1.9401 ms | **435.82 us** | **-77.5 %** (-77.4 %) |
| `f33_discover_no_http_guard` (control 1) | 2.3150 us | 2.3181 us | **+0.1 %** (+0.6 %) |
| `f33_discover_http_without_expressions` (control 2) | 7.9693 us | 8.0247 us | **+0.7 %** (-0.9 %) |

Both controls' baseline and candidate CIs overlap; neither delta is
distinguishable from zero.

The baseline's absolute target median (1.9401 ms) is also **19 % below** the
original run's recorded 2.3944 ms — the same common-mode inflation, visible on
the baseline side too. This is why only same-conditions ratios are quoted, and
why absolute figures from either run should not be compared across hosts.

## Disposition — **ACCEPTED** (unchanged), on corrected numbers

Target: **1.9401 ms -> 435.82 us, -77.5 %**, against a declared >= 30 % floor.
No control regressed. The quadratic prefix rescan is gone; the residual ~436 us
is the `ExpressionFinder` scan plus 300 expression parses, which F33 does not
touch.

Correctness evidence is unchanged and is not affected by this re-measurement —
`line_at_offset_matches_naive_at_every_offset`,
`remote_discovery_line_positions_match_fixture_text` (all 13 committed
fixtures), the LF/CRLF/Unicode/SOF/EOF edge cases, and the two mutation tests
all still pass.

## Cross-platform classification — **OS-identical** (unchanged)

Re-measurement changed no code and no classification. The change remains
confined to `discover_remote_urls_from_expressions` and two private helpers: a
`\n` byte scan and a `partition_point` search. No filesystem access, no
`cfg`-gated path, no line-ending normalization.

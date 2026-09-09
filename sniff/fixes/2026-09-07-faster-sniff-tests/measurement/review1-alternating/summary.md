# Review-1 alternating measurement — Sniff test suite

Answers review-1's finding *"The recorded comparison does not prove the
faster-tests outcome"* for its **local** half only. The CI half (matched CI
samples) remains blocked on the unpushed, human-gated branch and is out of
scope here.

Full provenance in `provenance.json`; raw logs in `raw-logs.tar.gz`
(`run-NN-<tree>-<cohort>.log`, plus the three warm-up logs — archived rather
than stored loose because this measurement's own finding is that committed
artifact volume costs the repo-walking tests time);
machine-readable records in `runs.jsonl`.

## Verdict

**The candidate is not measurably faster than the baseline on either cohort.**

- **Full L1 `test` cohort — no demonstrated change.** Median wall clock 23.66 s
  baseline vs 23.36 s candidate (−1.3%), against a drift bracket of 69.4%.
  Paired ratios split 3 up / 2 down. Nothing here is distinguishable from noise
  in either direction.
- **`sanity` cohort — candidate consistently slower, by ~9–19%.** The paired
  ratio exceeds 1 in **5 of 5 rounds**. The magnitude sits inside the drift
  bracket, but the direction does not (sign test, one-sided p ≈ 0.03).
- That sanity regression is **not caused by the test-speed changes**. It is
  caused by the branch committing ~9.4 MB / 395 files of its own measurement
  artifacts into the repository, which the repo-walking tests then traverse.
  See "Why sanity regressed" below.

## Protocol actually run

- Invocation form (verified before measuring; no adjustment needed, no `cd`):
  `just --justfile <TREE>/sniff/justfile --working-directory <TREE>/sniff <cohort>`
- Both trees warmed twice per cohort, all warm-up numbers discarded. Warm-up
  pass 2 confirmed both trees fully warm (`Finished test profile … in 0.26–0.37s`,
  zero `Compiling` lines).
- 5 alternating rounds, each in the order: baseline `test` → candidate `test` →
  baseline `sanity` → candidate `sanity`. 20 measured runs.
- **Zero `Compiling` lines in all 20 measured runs. All 20 exited 0 with every
  test passing.** No run needed discarding.
- Per run: `/usr/bin/time -p` wall clock, the verbatim nextest `Summary` line(s),
  `uptime` load averages before and after, and a sustained
  `/usr/bin/top -l 2 -s 3 -n 0` CPU-idle sample before.

### Sample validity

Both trees were unchanged across the whole window:

| Tree | HEAD before | HEAD after | Dirty before | Dirty after |
|---|---|---|---|---|
| baseline | `c2dee9217` | `c2dee9217` | 1 (untracked `target-sniff-phase1/`) | 1 |
| candidate | `fe83e7481` | `fe83e7481` | 9 | 9 |

**The sample is valid.**

## Paired table — wall clock (`/usr/bin/time -p` real, seconds)

### `test` cohort (full L1 suite)

| Round | baseline | candidate | ratio c/b | baseline idle% | candidate idle% |
|---|---|---|---|---|---|
| 1 | 20.37 | 20.24 | 0.994 | 83.8 | 73.1 |
| 2 | 20.22 | 21.69 | 1.073 | 87.7 | 67.4 |
| 3 | 24.41 | 30.99 | 1.270 | 85.6 | 86.7 |
| 4 | **36.63** | 30.42 | 0.830 | **61.4** | 67.4 |
| 5 | 23.66 | 23.36 | 0.987 | 82.1 | 87.3 |

- baseline: **median 23.66 s**, range 20.22–36.63 s
- candidate: **median 23.36 s**, range 20.24–30.99 s
- median-of-medians ratio **0.987 (−1.3%)**; median paired ratio **0.994**

### `sanity` cohort (fast-confidence path)

| Round | baseline | candidate | ratio c/b | baseline idle% | candidate idle% |
|---|---|---|---|---|---|
| 1 | 10.14 | 11.38 | 1.122 | 86.4 | 77.7 |
| 2 | 10.24 | 12.01 | 1.173 | 63.9 | 78.5 |
| 3 | 15.29 | 17.77 | 1.162 | 78.9 | 80.0 |
| 4 | 11.10 | 11.26 | 1.014 | 81.2 | 67.5 |
| 5 | 13.08 | 15.27 | 1.167 | 79.8 | **10.4** |

- baseline: **median 11.10 s**, range 10.14–15.29 s
- candidate: **median 12.01 s**, range 11.26–17.77 s
- median-of-medians ratio **1.082 (+8.2%)**; median paired ratio **1.162**

## Paired table — nextest runner elapsed (excludes cargo/just overhead)

Verbatim `Summary` lines are in each run log. For `sanity` the figure is the
sum of the two per-package invocations.

| Cohort | Round | baseline | candidate | ratio |
|---|---|---|---|---|
| test | 1 | 19.380 | 19.230 | 0.992 |
| test | 2 | 19.140 | 20.610 | 1.077 |
| test | 3 | 23.347 | 29.887 | 1.280 |
| test | 4 | 35.540 | 29.341 | 0.826 |
| test | 5 | 22.581 | 22.332 | 0.989 |
| sanity | 1 | 7.250 + 0.542 = 7.792 | 8.489 + 0.532 = 9.021 | 1.158 |
| sanity | 2 | 7.263 + 0.535 = 7.798 | 8.823 + 0.522 = 9.345 | 1.198 |
| sanity | 3 | 11.978 + 0.599 = 12.577 | 14.198 + 0.757 = 14.955 | 1.189 |
| sanity | 4 | 7.878 + 0.541 = 8.419 | 8.441 + 0.524 = 8.965 | 1.065 |
| sanity | 5 | 7.285 + 1.070 = 8.355 | 10.140 + 0.687 = 10.827 | 1.296 |

| Cohort | baseline median (range) | candidate median (range) | med ratio | median paired ratio |
|---|---|---|---|---|
| test | 22.58 (19.14–35.54) | 22.33 (19.23–29.89) | 0.989 (−1.1%) | 0.992 |
| sanity | 8.36 (7.79–12.58) | 9.35 (8.96–14.96) | 1.119 (+11.8%) | 1.189 |

## Drift bracket

The noise floor is the spread of the **baseline** runs, expressed against the
baseline median:

| Cohort | Measure | Baseline spread | Bracket (% of baseline median) | Candidate median delta | Inside bracket? |
|---|---|---|---|---|---|
| test | wall | 16.41 s | **69.4%** | −1.3% | yes — not a demonstrated change |
| test | runner | 16.40 s | **72.6%** | −1.1% | yes — not a demonstrated change |
| sanity | wall | 5.15 s | **46.4%** | +8.2% | yes — magnitude not demonstrated |
| sanity | runner | 4.79 s | **57.3%** | +11.8% | yes — magnitude not demonstrated |

**Every observed difference is inside the drift bracket.** No percentage in this
document may be reported as a speedup or a slowdown magnitude.

The one thing the bracket does *not* absorb is the **direction** of the sanity
result. Alternation is what makes drift cancel, and the candidate's paired
sanity ratio is above 1 in 5 of 5 rounds on both wall clock and runner elapsed
(one-sided sign test p ≈ 0.03). Directionally real; magnitude unresolved.

## Load anomalies and sensitivity

Load averages climbed from ~19 to ~100 over the window, but that is mostly the
measurement itself — nextest spawns thousands of short-lived processes, which
inflates this host's load average without saturating CPU. Sustained idle stayed
in the 61–88% band for 19 of 20 runs. Two runs deviate:

1. **Round 4, baseline `test`** — 61.4% idle before, load 100.69 after; 36.63 s,
   the slowest run of the entire sample and the sole reason the baseline range
   reaches 36.63 s.
2. **Round 5, candidate `sanity`** — **10.4% idle** before. Genuine external
   saturation. 15.27 s, the second-slowest candidate sanity run.

Excluding them (paired, so the round is dropped from both trees):

| Cohort | Excluding | baseline median | candidate median | ratio |
|---|---|---|---|---|
| test | round 4 | 22.02 | 22.53 | 1.023 (+2.3%) |
| sanity | round 5 | 10.67 | 11.70 | 1.096 (+9.6%) |
| sanity | rounds 3 and 5 | 10.24 | 11.38 | 1.111 (+11.1%) |

**Excluding the anomalies does not change either conclusion.** The `test` result
flips sign (−1.3% → +2.3%) and stays deep inside the bracket, which is itself
evidence that the `test` number is noise. The `sanity` direction is unchanged
and remains 4/4 or 3/3 consistent.

## Sanity cohort against the 15-second fast-confidence budget

| Tree | Measure | Median | Max | Rounds over 15 s |
|---|---|---|---|---|
| baseline | wall clock (`just sanity`) | 11.10 s | 15.29 s | **1 of 5** |
| candidate | wall clock (`just sanity`) | 12.01 s | 17.77 s | **2 of 5** |
| baseline | nextest runner elapsed | 8.36 s | 12.58 s | 0 of 5 |
| candidate | nextest runner elapsed | 9.35 s | 14.96 s | 0 of 5 (round 3 left **0.045 s** of headroom) |

**Verdict: the budget is met at the median in both trees and is not robustly met
under load in either.** What a developer actually waits for is the wall clock,
and that breached 15 s in 1 of 5 baseline rounds and 2 of 5 candidate rounds.
Even on the more forgiving runner-elapsed measure the candidate came within
45 ms of the budget once. The candidate has strictly less headroom than the
baseline on every measure.

The gap between the two measures (~2.6–5 s) is fixed cost inside the recipe, not
test execution: `sanity` runs `_storage_preflight`, then two separate
`cargo metadata` + `cargo nextest` invocations, one per package.

## Test counts differ — read the raw cohort time, not the per-test mean

The candidate runs **more** tests in the `test` cohort; it added coverage.

| Cohort | baseline tests | candidate tests | delta |
|---|---|---|---|
| test | 2599 (23 skipped) | 2618 (24 skipped) | **+19** |
| sanity | 1419 + 401 = 1820 | 1419 + 401 = 1820 | **0** |

Per-test mean for the `test` cohort, at the median runner elapsed:

| Tree | runner elapsed | tests | per-test mean |
|---|---|---|---|
| baseline | 22.581 s | 2599 | 8.688 ms |
| candidate | 22.332 s | 2618 | 8.531 ms |

That is −1.8%. **Do not read it as a headline.** Cohort elapsed is a *parallel*
wall clock across 28 test binaries; dividing it by a test count does not yield
per-test cost, and a −1.8% figure derived from a −1.1% cohort difference that is
itself 65× smaller than the drift bracket carries no information. The honest
headline is the raw cohort wall clock: **23.66 s → 23.36 s, within drift.**

The `sanity` cohort's counts are identical, so its per-test mean ratio equals
its cohort ratio exactly.

## Why sanity regressed — and why it is not the fix's doing

The `sanity` recipe is `--lib --bins`. The candidate's entire 521-line working
diff lives in `sniff/cli/tests/*`, which are integration test *targets* and are
never built into `--lib --bins`. Across the committed range `c2dee9217..fe83e7481`
the only `sniff/lib/src` or `sniff/cli/src` hunks are in
`sniff/lib/src/os/user.rs` and `sniff/lib/src/programs/types.rs`, and both are
entirely inside `#[cfg(test)] mod tests` — assertion-quality rewrites costing a
two-entry `HashMap` and two `serde_json` parses. **The sanity cohort is therefore
a near-perfect control: identical test population, effectively identical code.**

Attributing the delta per test (mean over the three quietest rounds, 1820 tests
joined by name) shows it is not diffuse — 956 tests got slower, 790 got faster,
and the top 20 account for 11.69 s of the 20.30 s total:

| Mean delta (s) | Test |
|---|---|
| +1.276 | `filesystem::docs::tests::integration::documents_have_correct_packages` |
| +1.196 | `filesystem::docs::tests::integration::repo_documents_from_current_dir` |
| +1.167 | `filesystem::docs::tests::integration::all_docs_have_content_hash` |
| +1.155 | `filesystem::docs::tests::integration::detect_docs_returns_some_in_repo` |
| +1.118 | `filesystem::docs::tests::integration::all_docs_have_relative_paths` |
| +0.564 … +0.263 | ten `filesystem::git::worktree::tests::list_worktrees_*` / `inside_*` tests |
| +0.335 … +0.319 | `filesystem::blast_radius::tests::…::*_in_real_repo`, `*_filter_*`, `*_package_area_*` |

Every one of these walks the **real surrounding repository**. And the candidate
tree's repository is bigger, because this fix cycle committed its own
measurement artifacts into it:

| | baseline | candidate |
|---|---|---|
| tracked files | 10 963 | 11 358 (**+395**) |
| tracked `.md` files | 4 140 | 4 174 (**+34**) |
| `sniff/fixes/` size | 4.6 M | 14 M (**+9.4 M**) |

The five `filesystem::docs` tests enumerate and content-hash every Markdown
document in the repo; 34 more documents and 9.4 MB more to traverse is a direct,
sufficient explanation for the largest deltas, and the tests' own names say what
they do.

So the sanity regression is a **self-inflicted, incidental cost of the branch's
committed artifacts**, not a property of the test-speed work. It is still a real
cost that a developer running `just sanity` on this branch pays.

Two consequences worth recording:

1. Any future between-tree comparison in this repo must place both trees in
   structurally comparable locations **and** account for tracked-content volume,
   because Sniff's own suite measures the repository it sits in. This is the
   `sniff`-specific instance of the known "capture tests walk the monorepo"
   hazard.
2. The ~9–19% systematic offset the sanity control exposes means the `test`
   cohort's −1.3% is measured against a candidate tree carrying a handicap on
   the repo-walking tests it shares. That *could* mean the `test` cohort's real
   change is more favourable than −1.3%. **It is not claimed here**: the two
   cohorts have different test populations and different I/O profiles, so the
   offset cannot be assumed to transfer, and subtracting it would be
   manufacturing a win the data does not support.

## Conclusion

Under a clean, pinned, alternating, fully warm measurement with compatible load
and a validated sample, **the candidate tree is not measurably faster than the
baseline on either the full L1 `test` cohort (−1.3%, inside a 69% drift bracket)
or the `sanity` cohort (where it is in fact consistently slower, 5 of 5 rounds,
for reasons traceable to the branch's committed measurement artifacts rather
than to its test changes) — so review-1's finding stands: this measurement does
not demonstrate a faster-tests outcome.**

The candidate does run 19 more tests in the same time, and it does not regress
the full suite. Neither of those is a speedup.

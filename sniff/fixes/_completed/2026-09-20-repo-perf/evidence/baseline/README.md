# Baseline tranche: pre-parallelization nested-marker walk

The baseline side is the Phase 1 harness applied to `HEAD` `1634e6e55`, with no
change to `walk_for_nested_markers`; the `production` rows prove that. The host
and provenance are in [`../environment.md`](../environment.md).

## Corpus facts

| Fact | Value | Source |
|---|---:|---|
| Root spelling | `/Volumes/coding/wt/rusty-biscuit/fix-sniff` | bench stderr |
| Non-directory entries admitted by the walk | 12,893 | `nested_benchmark::corpus`, printed at bench registration (all four brackets agree) |
| Marker entries | 116 | same |
| Nested candidates (after root exclusion and grouping) | 112 | `serial_reference_walk(..).len()` |
| Historical figures (context only) | 11,290 paths, 92 markers | spec |

The spike (`../spike.md`) counted 12,889 entries earlier on 2026-09-25. The four
additional entries are this fix's own evidence files, written between the two
readings.

**Fallback verified.** `detect_repo_structure(<corpus root>)` under a fresh
collector records `filesystem.repo.nested_marker_walks = 1` and
`filesystem.io.read_dirs = 2`: the nested-marker walk plus the
`ManifestIndex` walk. So the measured public request does take the fallback;
it reuses no supplied marker evidence. See `work_counts-release.md` and
`work_counts-debug.md` (`repo_structure_corpus`). The two profiles record
identical counters.

## Timing (warm cache, 3 s warm-up, 20 flat samples per case)

A sample's time is `time / iters` from Criterion's `sample.json`. The raw files
are in `raw/`.

| profile | bracket | case | samples | iters/sample | median ms | min ms | max ms | spread % |
|---|---|---|---:|---:|---:|---:|---:|---:|
| debug | r1 | detect_repo_structure | 20 | 3 | 241.04 | 236.57 | 246.69 | 4.2 |
| debug | r1 | production | 20 | 3 | 168.74 | 166.71 | 175.62 | 5.3 |
| debug | r1 | serial_reference | 20 | 3 | 168.59 | 166.48 | 176.92 | 6.2 |
| debug | r2 | detect_repo_structure | 20 | 3 | 239.09 | 235.45 | 242.19 | 2.8 |
| debug | r2 | production | 20 | 3 | 168.43 | 164.91 | 174.89 | 5.9 |
| debug | r2 | serial_reference | 20 | 3 | 168.83 | 166.56 | 175.11 | 5.1 |
| release | r1 | detect_repo_structure | 20 | 7 | 75.65 | 74.11 | 77.57 | 4.6 |
| release | r1 | production | 20 | 9 | 62.40 | 60.01 | 70.87 | 17.4 |
| release | r1 | serial_reference | 20 | 8 | 62.80 | 60.55 | 65.11 | 7.3 |
| release | r2 | detect_repo_structure | 20 | 7 | 78.61 | 74.32 | 81.62 | 9.3 |
| release | r2 | production | 20 | 8 | 62.75 | 59.87 | 65.39 | 8.8 |
| release | r2 | serial_reference | 20 | 8 | 63.50 | 60.66 | 64.96 | 6.8 |

Bracket order was release r1 → debug r1 → release r2 → debug r2. The drift in
the median between brackets (r1 → r2) was:

- debug: `detect_repo_structure` −0.8%, `production` −0.2%, `serial_reference` +0.1%
- release: `detect_repo_structure` +3.9%, `production` +0.6%, `serial_reference` +1.1%

**Observed run-to-run variation for the Phase 4 decision rule:**
- walk medians: within about 1% between brackets, with a 5–9% per-sample
  spread;
- one release `production` bracket had a single 70.9 ms outlier, a 17% spread;
- `detect_repo_structure` release medians: about 4% between brackets.

## Time attribution for `detect_repo_structure` (remeasured)

These use the means of the two bracket medians.

| profile | `detect_repo_structure` | fallback walk (`production`) | other work | walk share |
|---|---:|---:|---:|---:|
| debug | 240.1 ms | 168.6 ms | ≈71.5 ms | ≈70% |
| release | 77.1 ms | 62.6 ms | ≈14.6 ms | ≈81% |

The historical figure was about 60 ms of 232 ms in debug (context only). The
subtraction assumes the in-request walk costs what the isolated walk costs.
Both run on the same warm tree.

## Commands

The benches and examples were compiled first, outside every timed region:

```sh
cargo bench -p sniff --features bench-internals --bench perf --no-run
cargo bench -p sniff --profile dev --features bench-internals --bench perf --no-run
cargo build -p sniff --release --example work_counts
cargo build -p sniff --example work_counts
```

Counters:

```sh
SNIFF_WORK_COUNTS_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff \
  target/release/examples/work_counts > work_counts-release.md
SNIFF_WORK_COUNTS_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff \
  target/debug/examples/work_counts > work_counts-debug.md
```

Timing. These were run from `sniff/lib`, one invocation per bracket, in the
order above:

```sh
export CRITERION_HOME=/tmp/repo-perf-criterion
export SNIFF_BENCH_NESTED_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff
# <prof> ∈ {release, debug}; <run> ∈ {r1, r2}
../../target/<prof>/deps/perf-<hash> --bench \
  --save-baseline baseline-<prof>-<run> '^nested_marker_walk_corpus/'
```

`CRITERION_HOME` keeps Criterion's output outside the corpus tree. The
per-sample JSON was copied from
`$CRITERION_HOME/nested_marker_walk_corpus/<case>/baseline-<prof>-<run>/sample.json`
to `raw/baseline-<prof>-<run>-<case>.sample.json`. The summary table came from
those files: per-sample `time / iters`, then median, min, and max.

For Phase 4, run the same commands with `--save-baseline after-<prof>-<run>`,
and alternate these brackets with a rerun of the baseline harness
(`serial_reference` gives the in-process "before" side in every run).

# `git2` Criterion Baseline — `git_ops` Group

Audit record of the pre-migration `git2`-backed timings for the `git_ops`
benchmark group. This is a **same-host audit record, not a portable
threshold**: the gitoxide port must be compared against a baseline captured on
the *same machine, toolchain, profile, and checkout*. Re-capture on your host
before comparing.

## Capture Environment

| Field | Value |
|-------|-------|
| OS | macOS 26.5 (build 25F71) |
| CPU | Apple M4 Max (16 cores, arm64) |
| Rust | rustc 1.96.0 (ac68faa20 2026-05-25) |
| Git commit | `c5e137227` |
| Backend | `git2` 0.20.x (production, pre-migration) |
| Power mode | AC power; macOS default scheduler (not pinned / not low-power) |
| `git` executable | available (commit-graph variants ran) |

## Capture Command

Run from `sniff/lib` so the saved baseline lands beside the bench harness:

```bash
cargo bench -p sniff --bench perf -- --save-baseline git2 git_ops
```

The committed numbers below were captured with **reduced sampling**
(`--measurement-time 3 --warm-up-time 1 --sample-size 10`) on a shared host to
keep the run tractable; the wide confidence intervals on the slower benches
(flagged below) reflect that sampling plus host contention. Re-run with the
default group sampling (no overrides) for a tighter baseline before a real
no-regression gate.

## Timings

Each row is Criterion's `[lower estimate upper]` 95% confidence interval.

| Benchmark | Lower | Estimate | Upper | CI band |
|-----------|-------|----------|-------|---------|
| `git_ops/discover` | 186.02 µs | 188.97 µs | 192.93 µs | ±~2% |
| `git_ops/status_dirty_flag/10` | 1.1563 ms | 1.1897 ms | 1.2227 ms | ±~3% |
| `git_ops/status_file_changes/10` | 3.8351 ms | 4.0168 ms | 4.1582 ms | ±~4% |
| `git_ops/status_dirty_flag/100` | 6.1185 ms | 6.3760 ms | 6.7079 ms | ±~5% |
| `git_ops/status_file_changes/100` | 24.502 ms | 26.423 ms | 27.368 ms | ±~7% |
| `git_ops/revwalk_recent_gated/nograph` | 50.831 ms | 55.548 ms | 64.969 ms | **+17%** ⚠ |
| `git_ops/revwalk_recent_full/nograph` | 81.069 ms | 87.125 ms | 91.970 ms | ±~7% |
| `git_ops/revwalk_recent_gated/graph` | 56.442 ms | 59.539 ms | 66.807 ms | **+12%** ⚠ |
| `git_ops/revwalk_recent_full/graph` | 81.536 ms | 83.530 ms | 85.008 ms | ±~2% |
| `git_ops/diff_commit_files` | 1.9673 ms | 2.8777 ms | 3.6629 ms | **±~30%** ⚠ |
| `git_ops/ancestry_containment` | 18.950 ms | 24.334 ms | 34.616 ms | **−22% / +42%** ⚠ |
| `git_ops/worktree_fanout/1` | 2.9214 ms | 3.3594 ms | 4.2024 ms | **+25%** ⚠ |
| `git_ops/worktree_fanout/4` | 5.4079 ms | 5.8340 ms | 6.2988 ms | ±~8% |
| `git_ops/worktree_fanout/8` | 7.0835 ms | 10.260 ms | 14.846 ms | **−31% / +45%** ⚠ |
| `git_ops/config_read` | 466.54 µs | 633.97 µs | 824.48 µs | **−26% / +30%** ⚠ |
| `git_ops/refs_enumerate` | 1.8343 ms | 1.9522 ms | 2.0920 ms | ±~7% |

## Benchmarks With Confidence Interval Beyond ±10%

These are **high-variance** under the reduced sampling above and on a shared
host; treat them with the median-based ±15% review the migration spec
prescribes rather than the strict no-regression gate:

- `git_ops/revwalk_recent_gated/nograph`
- `git_ops/revwalk_recent_gated/graph`
- `git_ops/diff_commit_files`
- `git_ops/ancestry_containment`
- `git_ops/worktree_fanout/1`
- `git_ops/worktree_fanout/8`
- `git_ops/config_read`

## Notes

- `status_dirty_flag/*` is the short-circuiting summary status (counts only);
  `status_file_changes/*` is the full per-file change walk — the ~4× gap at 100
  dirty files is the cost the summary path avoids.
- The commit-graph (`/graph`) and graph-absent (`/nograph`) revwalk variants are
  near-parity under `git2`, which exposes no public commit-graph reader; the
  expected commit-graph win is a gix-side improvement to record in later phases.
- Re-capturing without the sampling overrides will tighten the flagged rows.

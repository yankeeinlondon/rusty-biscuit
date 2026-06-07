# `gix` Criterion Baseline — `git_ops` Group

Audit record of the post-migration `gix`-backed timings for the `git_ops`
benchmark group, captured with the bench harness's built-in Criterion settings
— **identical to the `git2` baseline** so the comparison is methodologically
valid.

## Capture Environment

| Field | Value |
|-------|-------|
| OS | macOS 26.5 (build 25F71) |
| CPU | Apple M4 Max (16 cores, arm64) |
| Rust | rustc 1.96.0 (ac68faa20 2026-05-25) |
| Git commit | `b99dfef7b` (+ review-5 working-tree fixes) |
| Backend | `gix` 0.84.x (post-migration) |
| Power mode | AC power; macOS default scheduler (not pinned / not low-power) |
| `git` executable | available (commit-graph variants ran) |
| Host load | normal interactive load — captured back-to-back with `git2` |

## Capture Command

Run from `sniff/lib`, **no sampling overrides** (the harness sets the sampling
in code), into the same Criterion target dir as the `git2` baseline:

```bash
cargo bench -p sniff --bench perf -- --save-baseline gix git_ops
```

## Timings

Each row is Criterion's `[lower estimate upper]` 95% confidence interval.

| Benchmark | Lower | Estimate | Upper |
|-----------|-------|----------|-------|
| `git_ops/discover` | 251.54 µs | 254.14 µs | 257.00 µs |
| `git_ops/status_dirty_flag/10` | 1.3889 ms | 1.4200 ms | 1.4835 ms |
| `git_ops/status_file_changes/10` | 2.4515 ms | 2.4664 ms | 2.4853 ms |
| `git_ops/status_dirty_flag/100` | 3.0944 ms | 3.1523 ms | 3.1867 ms |
| `git_ops/status_file_changes/100` | 9.6728 ms | 10.820 ms | 12.076 ms |
| `git_ops/revwalk_recent_gated/nograph` | 44.141 ms | 47.219 ms | 51.267 ms |
| `git_ops/revwalk_recent_full/nograph` | 64.706 ms | 66.372 ms | 69.995 ms |
| `git_ops/revwalk_recent_gated/graph` | 36.995 ms | 39.316 ms | 42.016 ms |
| `git_ops/revwalk_recent_full/graph` | 67.008 ms | 67.801 ms | 68.546 ms |
| `git_ops/diff_commit_files` | 5.7442 ms | 5.8242 ms | 5.9125 ms |
| `git_ops/ancestry_containment` | 4.2922 ms | 4.4218 ms | 4.5517 ms |
| `git_ops/worktree_fanout/1` | 2.0777 ms | 2.0988 ms | 2.1304 ms |
| `git_ops/worktree_fanout/4` | 5.1667 ms | 5.2992 ms | 5.4460 ms |
| `git_ops/worktree_fanout/8` | 10.426 ms | 10.490 ms | 10.562 ms |
| `git_ops/config_read` | 317.98 µs | 321.43 µs | 325.75 µs |
| `git_ops/refs_enumerate` | 1.3779 ms | 1.4038 ms | 1.4272 ms |

## Same-Host Comparison vs. `git2` Baseline

Both baselines were captured **back-to-back on the same host with identical
bench-harness Criterion settings (no CLI overrides)**, then compared with
Criterion's saved-baseline change detection:

```bash
cargo bench -p sniff --bench perf -- --load-baseline gix --baseline git2 git_ops
```

Change is `gix` relative to `git2` (negative = faster). Criterion's decision is
its statistical change verdict at the default `significance_level = 0.05`.

| Benchmark | `git2` estimate | `gix` estimate | Change | Criterion decision |
|-----------|-----------------|----------------|--------|------------------|
| `git_ops/discover` | 429.07 µs | 254.14 µs | **−41.2%** | Improved |
| `git_ops/status_dirty_flag/10` | 3.5570 ms | 1.4200 ms | **−46.7%** | Improved |
| `git_ops/status_file_changes/10` | 3.9558 ms | 2.4664 ms | **−37.2%** | Improved |
| `git_ops/status_dirty_flag/100` | 4.1966 ms | 3.1523 ms | **−18.8%** | Improved |
| `git_ops/status_file_changes/100` | 12.642 ms | 10.820 ms | **−18.6%** | Improved |
| `git_ops/revwalk_recent_gated/nograph` | 119.15 ms | 47.219 ms | **−61.0%** | Improved |
| `git_ops/revwalk_recent_full/nograph` | 170.72 ms | 66.372 ms | **−65.0%** | Improved |
| `git_ops/revwalk_recent_gated/graph` | 71.206 ms | 39.316 ms | **−44.9%** | Improved |
| `git_ops/revwalk_recent_full/graph` | 138.18 ms | 67.801 ms | **−53.0%** | Improved |
| `git_ops/diff_commit_files` | 10.368 ms | 5.8242 ms | **−44.6%** | Improved |
| `git_ops/ancestry_containment` | 18.467 ms | 4.4218 ms | **−75.9%** | Improved |
| `git_ops/worktree_fanout/1` | 2.5293 ms | 2.0988 ms | **−13.5%** | Improved |
| `git_ops/worktree_fanout/4` | 5.7649 ms | 5.2992 ms | **−12.5%** | Improved |
| `git_ops/worktree_fanout/8` | 11.153 ms | 10.490 ms | **−4.4%** | Improved |
| `git_ops/config_read` | 576.23 µs | 321.43 µs | **−42.0%** | Improved |
| `git_ops/refs_enumerate` | 1.8143 ms | 1.4038 ms | **−22.7%** | Improved |

### Verdict: no regression

Every `git_ops` benchmark is **faster under `gix`** with `p < 0.05`; Criterion
reports "Performance has improved" for all 16 IDs and "regressed" for none. The
spec's hard no-regression gate (`spec.md` §"Pass criteria") is satisfied.

### Why earlier revisions showed regressions

The prior comparison reported regressions in discovery, the revwalk variants,
`diff_commit_files`, and `worktree_fanout/8`. Those were artifacts, not real
slowdowns:

1. **Sampling mismatch.** The old `git2` baseline used reduced CLI overrides
   (`--measurement-time 3 --warm-up-time 1 --sample-size 10`) while `gix` used
   fuller sampling. This recapture removes all overrides on both sides.
2. **Stale `gix` snapshot.** The old `gix` baseline was captured at `1833eb2bd`,
   *before* the perf commits that enable the commit-graph and lazily size the
   object cache. The current handle uses both levers, which is why the revwalk
   and ancestry walks now win decisively.
3. **Unrepresentative diff bench.** `diff_commit_files` opened a raw `gix`
   handle with no object cache, a path production never takes (every
   `recent_commits` entry point calls `open::configure_cache`). The bench now
   sizes the object cache to match production; libgit2's built-in ODB cache
   already gave `git2` that advantage for free.

### Variance note

`worktree_fanout/*` and the `status_file_changes/*` rows have wider CI bands
(filesystem + Rayon fan-out, `TempDir`-backed status). They are judged on the
median estimate per the spec's high-variance allowance; all still improve.

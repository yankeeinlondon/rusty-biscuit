# `gix` Criterion Baseline — `git_ops` Group

Audit record of the post-migration `gix`-backed timings for the `git_ops`
benchmark group, captured with default Criterion sampling (no overrides).

## Capture Environment

| Field | Value |
|-------|-------|
| OS | macOS 26.5 (build 25F71) |
| CPU | Apple M4 Max (16 cores, arm64) |
| Rust | rustc 1.96.0 (ac68faa20 2026-05-25) |
| Git commit | `1833eb2bd` |
| Backend | `gix` 0.84.x (post-migration) |
| Power mode | AC power; macOS default scheduler (not pinned / not low-power) |
| `git` executable | available (commit-graph variants ran) |

## Capture Command

Run from the workspace root so the saved baseline lands in the shared target:

```bash
cargo bench -p sniff --bench perf -- --save-baseline gix git_ops
```

## Timings

Each row is Criterion's `[lower estimate upper]` 95% confidence interval.

| Benchmark | Lower | Estimate | Upper | CI band |
|-----------|-------|----------|-------|---------|
| `git_ops/discover` | 197.68 µs | 205.19 µs | 214.28 µs | ±~4% |
| `git_ops/status_dirty_flag/10` | 1.1876 ms | 1.1993 ms | 1.2234 ms | ±~2% |
| `git_ops/status_file_changes/10` | 2.4567 ms | 2.5210 ms | 2.6026 ms | ±~3% |
| `git_ops/status_dirty_flag/100` | 2.5441 ms | 2.5721 ms | 2.6169 ms | ±~1% |
| `git_ops/status_file_changes/100` | 8.3911 ms | 8.4816 ms | 8.5628 ms | ±~1% |
| `git_ops/revwalk_recent_gated/nograph` | 82.695 ms | 84.430 ms | 85.745 ms | ±~2% |
| `git_ops/revwalk_recent_full/nograph` | 125.76 ms | 128.90 ms | 134.54 ms | ±~3% |
| `git_ops/revwalk_recent_gated/graph` | 52.094 ms | 52.416 ms | 52.800 ms | ±~1% |
| `git_ops/revwalk_recent_full/graph` | 101.59 ms | 103.78 ms | 106.93 ms | ±~3% |
| `git_ops/diff_commit_files` | 7.9103 ms | 8.1617 ms | 8.4670 ms | ±~3% |
| `git_ops/ancestry_containment` | 12.420 ms | 12.603 ms | 12.725 ms | ±~1% |
| `git_ops/worktree_fanout/1` | 1.8243 ms | 1.9198 ms | 1.9793 ms | ±~4% |
| `git_ops/worktree_fanout/4` | 4.6560 ms | 4.9579 ms | 5.3010 ms | ±~7% |
| `git_ops/worktree_fanout/8` | 15.384 ms | 17.600 ms | 19.738 ms | ±~12% ⚠ |
| `git_ops/config_read` | 262.94 µs | 269.33 µs | 278.20 µs | ±~3% |
| `git_ops/refs_enumerate` | 1.2681 ms | 1.3016 ms | 1.3323 ms | ±~3% |

## Same-Host Comparison vs. `git2` Baseline

> ⚠️ **Release blocker:** The `git2` baseline below was captured with **reduced
> sampling** (`--measurement-time 3 --warm-up-time 1 --sample-size 10`), while
> the `gix` numbers above use **default Criterion sampling**. This mismatch
> means the comparison is methodologically incomplete and cannot be used for a
> release no-regression decision. Both baselines must be recaptured with
> identical default settings on the same host before judging the gate.

The comparison below uses Criterion's statistical change detection against the
reduced-sampling baseline. Rows marked "Within noise" have Criterion's `p > 0.05`
after its noise threshold.

| Benchmark | `git2` estimate | `gix` estimate | Change | Criterion decision |
|-----------|-----------------|----------------|--------|------------------|
| `git_ops/discover` | 188.97 µs | 211.84 µs | +16.734% | Regressed |
| `git_ops/status_dirty_flag/10` | 1.1897 ms | 1.1993 ms | +4.333% | Within noise |
| `git_ops/status_file_changes/10` | 4.0168 ms | 2.5210 ms | **−36.216%** | Improved |
| `git_ops/status_dirty_flag/100` | 6.3760 ms | 2.5721 ms | **−58.878%** | Improved |
| `git_ops/status_file_changes/100` | 26.423 ms | 8.4816 ms | **−66.885%** | Improved |
| `git_ops/revwalk_recent_gated/nograph` | 55.548 ms | 84.430 ms | +53.993% | Regressed |
| `git_ops/revwalk_recent_full/nograph` | 87.125 ms | 128.90 ms | +56.582% | Regressed |
| `git_ops/revwalk_recent_gated/graph` | 59.539 ms | 52.416 ms | **−22.287%** | Improved |
| `git_ops/revwalk_recent_full/graph` | 83.530 ms | 103.78 ms | +27.648% | Regressed |
| `git_ops/diff_commit_files` | 2.8777 ms | 8.1617 ms | +235.15% | Regressed |
| `git_ops/ancestry_containment` | 24.334 ms | 12.603 ms | **−57.094%** | Improved |
| `git_ops/worktree_fanout/1` | 3.3594 ms | 1.9198 ms | **−47.409%** | Improved |
| `git_ops/worktree_fanout/4` | 5.8340 ms | 4.9579 ms | **−15.389%** | Improved |
| `git_ops/worktree_fanout/8` | 10.260 ms | 17.600 ms | +64.351% | Regressed |
| `git_ops/config_read` | 633.97 µs | 269.33 µs | **−57.519%** | Improved |
| `git_ops/refs_enumerate` | 1.9522 ms | 1.3016 ms | **−33.327%** | Improved |

### Variance Exceptions

Benchmarks whose CI band exceeded ±10% under the current default sampling:

- `git_ops/worktree_fanout/8` — ±~12%, high variance even with normal sampling.

### Notes

- The `git2` baseline used reduced sampling, so its confidence intervals are
  wider. Treat the Criterion "Regressed" / "Improved" labels above as
  directional signals rather than strict no-regression proof; the migration
  spec's ±15% median-based review gate is the authoritative threshold.
- **Status operations** (`status_dirty_flag/*`, `status_file_changes/*`) show
  large improvements under gix, especially at the 100-file scale.
- **Revwalk without commit-graph** (`/nograph`) is slower under gix; with
  commit-graph (`/graph`) the gated variant is faster but the full variant is
  slower.
- `diff_commit_files` is significantly slower under gix and warrants
  investigation in a future phase.
- `config_read` is substantially faster under gix (~58% improvement).
- `refs_enumerate` is faster under gix (~33% improvement).

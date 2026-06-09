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
| Git commit | `728614373` (last pre-migration commit carrying the `git_ops` bench) |
| Backend | `git2` 0.20.x (production, pre-migration) |
| Power mode | AC power; macOS default scheduler (not pinned / not low-power) |
| `git` executable | available (commit-graph variants ran) |
| Host load | normal interactive load (absolute timings inflated vs. a quiet host; see note) |

> The `git_ops` bench group did not exist at `c5e137227` (the commit named in
> earlier revisions of this file). It was introduced in `83f17100b`. This
> baseline is therefore captured at `728614373` — the last commit before the
> gix migration (`1833eb2bd`) that still has `git2` in production *and* carries
> the `git_ops` benches.

## Capture Command

Run from `sniff/lib` so the saved baseline lands beside the bench harness. **No
sampling overrides** — both this baseline and the `gix` comparison use the
bench harness's built-in Criterion configuration so the comparison is valid:

```bash
cargo bench -p sniff --bench perf -- --save-baseline git2 git_ops
```

## Timings

Each row is Criterion's `[lower estimate upper]` 95% confidence interval.

| Benchmark | Lower | Estimate | Upper |
|-----------|-------|----------|-------|
| `git_ops/discover` | 421.66 µs | 429.07 µs | 436.62 µs |
| `git_ops/status_dirty_flag/10` | 2.8638 ms | 3.5570 ms | 3.9726 ms |
| `git_ops/status_file_changes/10` | 3.8742 ms | 3.9558 ms | 4.0123 ms |
| `git_ops/status_dirty_flag/100` | 3.4715 ms | 4.1966 ms | 4.7818 ms |
| `git_ops/status_file_changes/100` | 11.143 ms | 12.642 ms | 15.130 ms |
| `git_ops/revwalk_recent_gated/nograph` | 105.29 ms | 119.15 ms | 138.55 ms |
| `git_ops/revwalk_recent_full/nograph` | 167.53 ms | 170.72 ms | 176.44 ms |
| `git_ops/revwalk_recent_gated/graph` | 68.943 ms | 71.206 ms | 73.152 ms |
| `git_ops/revwalk_recent_full/graph` | 134.61 ms | 138.18 ms | 142.12 ms |
| `git_ops/diff_commit_files` | 10.086 ms | 10.368 ms | 10.832 ms |
| `git_ops/ancestry_containment` | 17.663 ms | 18.467 ms | 19.366 ms |
| `git_ops/worktree_fanout/1` | 2.3743 ms | 2.5293 ms | 2.6998 ms |
| `git_ops/worktree_fanout/4` | 5.6931 ms | 5.7649 ms | 5.8676 ms |
| `git_ops/worktree_fanout/8` | 11.012 ms | 11.153 ms | 11.260 ms |
| `git_ops/config_read` | 562.64 µs | 576.23 µs | 591.43 µs |
| `git_ops/refs_enumerate` | 1.7905 ms | 1.8143 ms | 1.8378 ms |

## Notes

- This recapture and the `gix` comparison were run **back-to-back on the same
  host under the same (normal interactive) load**, using identical bench-harness
  Criterion settings (no CLI sampling overrides). Absolute timings are higher
  than the original quiet-host capture, but the relative comparison — the only
  thing the no-regression gate judges — stays valid because both backends paid
  the same host tax.
- `status_dirty_flag/*` is the short-circuiting summary status (counts only);
  `status_file_changes/*` is the full per-file change walk.
- The commit-graph (`/graph`) and graph-absent (`/nograph`) revwalk variants are
  near-parity under `git2`, which exposes no public commit-graph reader; the
  commit-graph win is a gix-side improvement (see `gix.md`).
- See `gix.md` for the same-host comparison and the no-regression verdict.

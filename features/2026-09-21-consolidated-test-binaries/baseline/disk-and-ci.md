---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
revision: c0f911f5a43b869bccf4b822a9c83a6114f27dc1
host: macOS aarch64-apple-darwin (Apple M4 Max, 16 cores, 128 GiB)
---

# Disk and CI-context baseline

## Local disk: direct observation (fresh target dir)

This comes from the S3 dry-run (`spikes/s3/before-phase1-targets.txt`): one clean
`cargo test --no-run -p claudine-cli --features terminal-tests` into an empty
target directory. It re-measures the spec's "136 executables, 2.67 GB"
observation under controlled conditions.

| Measure | Value |
|---|---:|
| `claudine-cli` test executables produced | 136 (139 targets minus 3 `real-tests`) |
| Their bytes on disk | 2,776,791,528 (2.78 GB; mean 20.4 MB) |
| Whole fresh target directory | 10.4 GB |

## Local disk: accumulated in this worktree (`target/debug/deps`)

This census of `<target>-<16 hex>` executables whose `<target>` is a test target
of the four packages was taken after Phase 1's listing capture. That capture
built claudine-cli under 5 feature sets, darkmatter under 6, biscuit-terminal
under 5, and darkmatter-cli under 2. Each feature set relinks every test target
under a new hash, and the old copies stay until a sweep. This is the
accumulation mechanism the spec describes. It is **not** a per-configuration
size.

| Package | Test executables present | Distinct targets | Bytes on disk (allocated) |
|---|---:|---:|---:|
| `claudine-cli` | 1,237 | 139 | 27.6 GB |
| `darkmatter` | 1,053 | 74 | 71.2 GB |
| `biscuit-terminal` | 279 | 38 | 10.9 GB |
| `darkmatter-cli` | 441 | 53 | 3.8 GB |
| All executables > 1 MB in `target/debug/deps` (any package) | 3,637 | — | 151.2 GB |

`darkmatter` and `biscuit-terminal` both declare targets named
`layout_matrix` and `render_comparison`. File names do not say which package
built an executable, so those executables are counted under both packages.
The per-package rows are therefore upper bounds. The census comes from
`baseline/deps-census.py`. Phase 3 and later
rollout observations should use the fresh-directory method above for
comparable numbers.

## CI skip baseline (R5 checkpoint)

`.github/ci/ci-baseline.toml` at this revision contains only
`schema_version = 3` and **no entries**:
sha256 `e6821f2b255f0c465ec9172c5ec9ce6c3caacbc72682bae1cd07cfc842a5a842`, last
changed in `ef1248875` (2026-09-20). Each package migration re-checks it.
A changed hash means entries must be translated through that package's
migration manifest.

## CI producer observations the spec quotes (single observations, not baselines)

These are pointers only. No CI run was triggered for this phase, and none
will be triggered to refresh them (spec §Out of scope).

| Spec claim | Source |
|---|---|
| `claudine-cli` archive 2.25 GB, 141 archived binaries, 911 files, 43 s to pack | pull request 92, `build (ubuntu-latest)` producer, run [`35662502516`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/35662502516) |
| an unidentified archive after `claudine-gen` at more than 2.13 GB (likely darkmatter, unconfirmed) | same run |
| `claudine` 375 MB / 14 binaries; `claudine-gen` 218 MB / 14; `biscuit-file` 75 MB / 16 | same run |
| `claudine-cli` archive 543 s of Cargo time vs 178 s for `claudine` | run [`35657985254`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/35657985254), analyzed by `2026-09-21-ci-build-feature-divergence` |

Phase 8 compares the first ordinary post-migration producer runs against
these numbers as context only, never as pass/fail (acceptance 10).

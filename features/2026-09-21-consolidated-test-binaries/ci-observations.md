---
kind: observations
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-22
status: pending
---

# CI producer observations (acceptance 10)

**Status: pending, 2026-09-22.** No ordinary CI run has selected a migrated
package after its migration. The implementation branch
(`fix/ci-build-feature-divergence`, 58 commits ahead of `origin/main` at
`8d2b0c6ce`) has not been pushed. `gh run list --branch
fix/ci-build-feature-divergence` returns no runs. Spec §Out of scope forbids
triggering a run only to fill this table, so none was triggered.

These are observations, not pass/fail gates. The pull request 92 figures are
context only.

## Observations

Fill one row per package from the first ordinary producer job that selects it.
Each row is a single observation.

| Package | Run URL | Producer job | Archive size | Archive file count | Packing time | Cargo build time |
|---|---|---|---:|---:|---:|---:|
| `claudine-cli` | pending | — | — | — | — | — |
| `darkmatter` | pending | — | — | — | — | — |
| `darkmatter-cli` | pending | — | — | — | — | — |
| `biscuit-terminal` | pending | — | — | — | — | — |

## Context: before the migration (pull request 92)

From `baseline/disk-and-ci.md`. These are single observations, not baselines.

| Package | Before | Source |
|---|---|---|
| `claudine-cli` | 2.25 GB, 141 archived binaries, 911 files, 43 s to pack | run [`35662502516`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/35662502516), `build (ubuntu-latest)` |
| `claudine-cli` | 543 s of Cargo time (178 s for `claudine`) | run [`35657985254`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/35657985254) |
| `darkmatter` | unconfirmed: an unnamed archive of more than 2.13 GB, probably darkmatter by build order | run `35662502516` |
| `darkmatter-cli`, `biscuit-terminal` | not observed | — |

The local executable sizes in `measurements.md` §Rollout observations
(for example `darkmatter` 5.01 GB → 0.51 GB) are from a macOS debug build, not
from a CI archive. Do not put them in the table above.

## How to harvest

1. Wait for the first ordinary run whose resolved plan
   (`scripts/ci/affected_scope.py`) selects the package: this branch's pull
   request, or the push to `main` after it merges.
2. Find the producer job with `gh run view <run-id> --json jobs`. For
   `claudine-cli` in run `35662502516` this was `build (ubuntu-latest)`.
3. Read that job's log with `gh run view <run-id> --job <job-id> --log`. The
   nextest `archive` step reports the file count and the packing time. The
   Cargo `Finished` line of the same step gives the build time. The artifact
   upload step, or `gh api repos/{owner}/{repo}/actions/runs/<run-id>/artifacts`,
   gives the archive size.
4. Record the run URL on every row. If a producer compiled the package with a
   different feature set from pull request 92, say so in the row, because
   `2026-09-21-ci-build-feature-divergence` changes build time independently.

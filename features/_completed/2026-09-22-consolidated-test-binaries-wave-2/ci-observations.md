---
kind: observations
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
status: pending
---

# CI producer observations (first-feature acceptance 10)

**Status: pending, 2026-09-23.** No ordinary CI run has selected a package
from this wave since its migration. The implementation branch
(`fix/ci-build-feature-divergence`) is 60 commits ahead of
`origin/fix/ci-build-feature-divergence`. The latest `ci` run on the branch,
[`35900378816`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/35900378816)
(2026-09-23, success), built a head from before any wave-2 migration. The
spec's out-of-scope list forbids triggering a run only to fill this table,
so none was triggered.

These are observations, not pass/fail gates.

## Observations

Fill one row per package from the first ordinary producer job that selects it.
Each row is a single observation.

| Package | Run URL | Producer job | Archive size | Archive file count | Packing time | Cargo build time |
|---|---|---|---:|---:|---:|---:|
| `tree-hugger` | pending | — | — | — | — | — |
| `claudine` | pending | — | — | — | — | — |
| `sniff` | pending | — | — | — | — | — |
| `biscuit-file` | pending | — | — | — | — | — |
| `schematic-gen` | pending | — | — | — | — | — |
| `biscuit-terminal-cli` | pending | — | — | — | — | — |
| `claudine-gen` | pending | — | — | — | — | — |
| `dmls` | pending | — | — | — | — | — |
| `sniff-cli` | pending | — | — | — | — | — |
| `biscuit-tui-cli` | pending | — | — | — | — | — |

The local executable sizes in `measurements.md` come from a macOS debug build,
not from a CI archive. Do not put them in the table above.

## How to harvest

Use the first feature's procedure (`2026-09-21-consolidated-test-binaries`,
`ci-observations.md` §How to harvest):

1. Wait for the first ordinary run whose resolved plan
   (`scripts/ci/affected_scope.py`) selects the package: this branch's pull
   request after the push, or the push to `main` after it merges.
2. Find the producer job with `gh run view <run-id> --json jobs`.
3. Read that job's log with `gh run view <run-id> --job <job-id> --log`. The
   nextest `archive` step reports the file count and the packing time. The
   Cargo `Finished` line of the same step gives the build time. The artifact
   upload step, or `gh api repos/{owner}/{repo}/actions/runs/<run-id>/artifacts`,
   gives the archive size.
4. Record the run URL on every row. One producer build can carry several of
   these packages; say so in the row when the figures are shared.

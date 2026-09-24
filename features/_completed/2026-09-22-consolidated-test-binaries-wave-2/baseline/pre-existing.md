---
kind: baseline
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
rev: 208051f75
host: macOS (darwin, Apple Silicon dev host)
runner: baseline/pre-existing.sh
logs: baseline/pre-existing-logs/
---

# Pre-existing state of the eight areas (unmigrated tree)

`baseline/pre-existing.sh` ran each recipe from the area directory, one after
another, against `208051f75`. The working tree was **not** pristine. It
carried the author's uncommitted edits to `prompts/_implement/implement-feature.md`
and `prompts/_implement/implement-plan.md`, which are unrelated to this
feature. Those edits cause the only failure below. Per-run logs (gzipped) and
`summary.tsv` are in `baseline/pre-existing-logs/`.

## Results

| Area | `just test` (L1) | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|
| `tree-hugger` | ✅ 585 passed | ✅ 3 passed | ✅ | ✅ 0 stranded | ✅ |
| `claudine` | ❌ 7316 passed, **2 failed** (see below) | ✅ 237 + 3 passed | ✅ | ✅ 0 stranded | ✅ |
| `sniff` | ✅ 2824 passed | ✅ 6 passed | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-file` | ✅ 839 passed | n/a (stub) | ✅ | ✅ 0 stranded | ✅ |
| `schematic` | ✅ 1699 passed | ✅ 3 passed | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-terminal` | ✅ 3263 passed | ✅ 2 + 76 passed | ✅ | ✅ 0 stranded | ✅ |
| `darkmatter` | ✅ 8490 passed | ✅ 18 + 69 + 3 passed | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-tui` | ✅ 991 passed | ✅ 21 passed | ✅ | ✅ 0 stranded | ✅ |

Counts are nextest `Summary` lines. Where a recipe makes more than one
invocation, each count is listed.

## Pre-existing failures (not attributable to the migration)

`claudine`'s `just test` failed on the first run, which is fail-fast: 5085 of
7318 run, 1 failed. The re-run with `--no-fail-fast`
(`claudine.test-no-fail-fast.log.gz`) finds exactly two failures, both in
`claudine-cli::l1`:

- `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body`
- `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`

**Cause:** the working tree's uncommitted `prompts/_implement/implement-plan.md`
adds a `::file ../_test-tiers.md` line, and the fixture
`claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
does not have it. The assertion message says exactly that. The cause is
environmental, the package is `claudine-cli` (outside the ten), and the failure
passes on a tree without those edits. It resolves when the author commits the
prompt change together with its fixture. No in-scope package (`claudine`,
`claudine-gen`) failed.

## Stranded and unrun tests today

`just check-tier-coverage <area>` reports **0 stranded** in all eight areas.

Two gaps are pre-existing. Neither is "stranded" by that recipe's definition,
because each test is L1-tier and CI's L1 cell runs it. Both are recorded so
neither is attributed to the migration.

- **F3, `biscuit-terminal-cli`, 49 L1-tier tests:** 43 in
  `level2_prose_cells.rs` and 6 in `level2_diagrams.rs`. Both targets require
  `terminal-tests`, and local `just test` builds the CLI without
  features, so neither target builds there. `just test-l2` builds them but
  selects only `level2_*` names. The logs confirm neither local recipe runs
  these 49 tests: `biscuit-terminal.test.log.gz` has 0 lines for
  `level2_prose_cells` or `level2_diagrams`, and `test-l2` runs only their
  marked tests. **They run only in CI's L1 cell,** which builds with
  `terminal-tests` (`[package.metadata.ci.tests] features`). The migration
  must preserve that: the four-way comparison under `(terminal-tests)` must
  show all 49 as L1-selected before and after.
- **F5, `biscuit-tui-cli` `windows_captured_stdout`, 1 test:** it is
  `#![cfg(windows)]` and `required-features = ["terminal-tests"]`. It is
  platform-absent on macOS and Linux, and on Windows it compiles only with
  `terminal-tests`. **It runs only in CI's Windows L1 cell** and in
  `cross-check --os windows` (S3: archive mode builds the CI feature union).

The migration must leave these exactly as they are. Closing either gap is out
of scope.

## Re-running

```sh
features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.sh
```

It overwrites `pre-existing-logs/`. A later phase that needs a fresh
comparison should copy the directory first.

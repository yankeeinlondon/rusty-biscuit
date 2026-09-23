---
kind: spike
feature: 2026-09-22-consolidated-test-binaries-wave-2
spike: S1
created: 2026-09-23
rev: 208051f75
---

# S1 — Feature-set and recipe census

This census is R1's input table. For each of the ten packages it records:

- every `--features` value its area's canonical recipes pass;
- the `[package.metadata.ci.tests]` `features` union, and `local-features`
  where one is set; and
- `l1-include-slow`.

Sources are the area `justfile`s at `208051f75`, the shared recipes in
`just/devops.just` (`_test_all`, `_test_local_all`, `_test_l2`, `_test_real`),
and `cargo metadata --no-deps`.

`()` is kept in every set even when no recipe builds the package bare. That is
the wave-1 convention. It is also the set a developer gets from a plain
`cargo nextest run -p <pkg>`, and the one `just test` gets when an area recipe
forwards no features.

## Target census (cargo metadata, `208051f75`)

| Package | Targets | `required-features` contracts |
|---|---:|---|
| `sniff` | 20 | `()` ×20 |
| `claudine` | 15 | `()` ×15 |
| `biscuit-file` | 15 | `()` ×14; `fetch`: `fetch_integration` |
| `biscuit-tui-cli` | 15 | `()` ×12; `terminal-tests`: `level3_chord_select`, `real_terminal_render`, `windows_captured_stdout` |
| `biscuit-terminal-cli` | 14 | `()` ×3; `terminal-tests` ×11 (every `level2_*` target) |
| `schematic-gen` | 14 | `()` ×13; `terminal-tests`: `terminal_capture` |
| `claudine-gen` | 11 | `()` ×10; `terminal-tests`: `level2_report_terminal` |
| `dmls` | 11 | `()` ×10; `terminal-tests`: `level2_editor_neovim` |
| `sniff-cli` | 11 | `()` ×8; `test-fixtures`: `level2_cicd_styling`, `level2_git_status_styling`, `level2_perf_tree_rendering` |
| `tree-hugger` | 10 | `()` ×10 |
| **Total** | **136** | workspace total **236** |

The largest packages outside scope are `biscuit-clipboard-cli` and
`worktree-cli`, with 8 each. The plan's table holds.

## Recipes and feature sets

"CI `test`" is the `NEXTEST_PROFILE=ci` / `BISCUIT_CI_ENVIRONMENT` branch of
the area's `test` recipe. The CI L1 producer builds with the metadata
`features` union. The repository-root `just test` uses `local-features` when
the key is present (even empty), and otherwise `features`
(`just/devops.just:1662-1677`).

| Package | `test` (local) | CI `test` | `test-l2` | `test-l3` | `test-real` | Other | CI `features` / `local-features` | `l1-include-slow` |
|---|---|---|---|---|---|---|---|---|
| `tree-hugger` | — | — | not built (`tree-hugger-cli` only) | stub | stub | — | none | no |
| `claudine` | — | — | not built | not built | not built (`claudine-contract`, `claudine-cli` only) | — | none (no `ci.tests` table) | no |
| `sniff` | `remote` | `remote` | not built | stub | `network` | lint/coverage/doctest `remote` | `remote` | no |
| `sniff-cli` | — | `test-fixtures` | `test-fixtures` | stub | — | — | `test-fixtures` / `[]` | no |
| `biscuit-file` | `fetch` | `fetch` | stub | stub | stub | `test-minimal`: `--no-default-features --lib` (no integration targets) | `fetch` | no |
| `schematic-gen` | — | — | `terminal-tests` | stub | — | — | `terminal-tests` / `[]` | no |
| `biscuit-terminal-cli` | — | — | `terminal-tests` | — (runs, `()`) | stub | — | `terminal-tests` / `[]` | no |
| `claudine-gen` | — | `terminal-tests` | `terminal-tests` | not built | not built | — | `terminal-tests` / `[]` | no |
| `dmls` | `effects-instrumentation` | `effects-instrumentation`, `BISCUIT_L1_INCLUDE_SLOW=1` | `terminal-tests` | not built | stub | — | `terminal-tests, effects-instrumentation` / `effects-instrumentation` | **yes** |
| `biscuit-tui-cli` | — | — | `terminal-tests` | `terminal-tests` | stub | — | `terminal-tests` / `[]` | no |

"—" means the recipe builds the package with no features. "Not built" means
the recipe does not build this package. "Stub" means the recipe is a
"not applicable" echo for the whole area.

## Feature sets for R1 (`PACKAGE_FEATURE_SETS`)

| Package | Sets | Change from the plan's proposal |
|---|---|---|
| `tree-hugger` | `()` | none |
| `claudine` | `()` | resolves the plan's open item: `claudine/justfile` passes no feature to `claudine` |
| `sniff` | `()`, `(remote)`, `(network)` | none. `remote` implies `network`, but they are listed separately because `test-real` builds `network` alone. |
| `sniff-cli` | `()`, `(test-fixtures)` | none |
| `biscuit-file` | `()`, `(fetch)` | none |
| `schematic-gen` | `()`, `(terminal-tests)` | none |
| `biscuit-terminal-cli` | `()`, `(terminal-tests)` | none |
| `claudine-gen` | `()`, `(terminal-tests)` | none |
| `dmls` | `()`, `(effects-instrumentation)`, `(terminal-tests)`, `(terminal-tests, effects-instrumentation)` | **adds `(terminal-tests)`**, which `test-l2` builds alone |
| `biscuit-tui-cli` | `()`, `(terminal-tests)` | none |

In every package the CI union is one of the listed sets, so `capture`'s
capture-time check can stay as it is. For `dmls`, `capture` must record both
`BISCUIT_L1_INCLUDE_SLOW` states (the `L1-include-slow` selector). No `dmls`
test is currently named `slow_*`, so both states are expected to select the
same L1 set. That makes it a no-op proof, not a skipped one.

## Feature gates inside test sources

The four-way comparison must preserve these. They are listed because
`required-features` alone does not show them.

- `sniff`: `focused_provider.rs` and `remote_providers.rs` are
  `#![cfg(feature = "remote")]`. `remote_observation.rs` is
  `#![cfg(feature = "network")]`. Their targets declare no
  `required-features`, so they compile to empty binaries under `()`.
- `sniff-cli`: **all four** `level2_*` files are
  `#![cfg(feature = "test-fixtures")]`, including
  `level2_recent_commits_rendering.rs`, whose target declares no
  `required-features`. So the spec's hazard 3 premise, that this target's
  tests "do not declare" `test-fixtures`, does not hold. Without the feature
  its target links an empty binary. `tests/common/mod.rs` also gates three
  helpers on `test-fixtures`. See rulings R3 and R14.
- `dmls`: `strict_mode_recovery_spike.rs` and `no_side_effects.rs` gate items
  on `effects-instrumentation` (`:60`, `:155`, `:34`, `:211`). This is why
  both states of that feature are in the set.
- No other in-scope `tests/` file uses `cfg(feature = …)`.

## Recipe-level observations used later

- `claudine`'s `test-real` does not build `claudine` (lib). A `real_`-named
  test in `claudine`'s tests would already be stranded. Whether one exists is
  answered by `baseline/pre-existing.md` (`check-tier-coverage claudine`).
- `biscuit-file`'s `test-minimal` is `--lib` only, so it is unaffected by the
  migration.
- `biscuit-terminal`'s `test-l3` builds `biscuit-terminal-cli` without
  features. That is already covered by the `()` set.

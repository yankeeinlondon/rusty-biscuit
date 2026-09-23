---
kind: acceptance
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
verdict: met-with-pending
pending:
    - first-feature acceptance 10, CI producer observations
---

# Acceptance record: consolidated test binaries, wave 2

This record goes through the first feature's acceptance criteria 1–8 and
10–12, as the spec amends them, and then new criteria 4–6. Each row names its
evidence file and, where one exists, the command that re-runs it. Phases 1–5's
evidence is committed with each package. Phase 6's (`acceptance/`,
`workspace-metadata.txt`, `ci-observations.md`, and
`baseline/test-selector-consumers-after.md`) lands in Phase 6's commit. Paths
are relative to this feature directory unless they start at the repository
root. `<pkg>/` is the package's evidence directory, next to its
`<pkg>-migration.json` manifest.

**Summary.** Every criterion is met except one. First-feature criterion 10
(CI observations) is pending because no ordinary run has selected these
packages yet (`ci-observations.md`). Pending is not counted as a pass.
First-feature criterion 5 is met as far as a tool can show, and it still needs
the reviewer's reading, as in the first feature. Failures in the local suites
are the ones `baseline/pre-existing.md` already recorded, and none of them is
in a migrated package (§8).

| Package | Area | Migration commit | Old targets | New targets |
|---|---|---|---:|---|
| `tree-hugger` | `tree-hugger` | `ef5fc2100` | 10 | `l1` |
| `claudine` | `claudine` | `f28f915ad` | 15 | `l1` |
| `sniff` | `sniff` | `db973c058` | 20 | `l1` |
| `biscuit-file` | `biscuit-file` | `9128b423b` | 15 | `l1`, `l1-fetch` |
| `schematic-gen` | `schematic` | `8eb813c74` | 14 | `l1`, `level2` |
| `biscuit-terminal-cli` | `biscuit-terminal` | `1355d6fc2` | 14 | `l1`, `level2` |
| `claudine-gen` | `claudine` | `8475cc93a` | 11 | `l1`, `level2` |
| `dmls` | `darkmatter` | `fe742107e` | 11 | `l1`, `level2` |
| `sniff-cli` | `sniff` | `e4e1bafd0` | 11 | `l1`, `level2` |
| `biscuit-tui-cli` | `biscuit-tui` | `74404cece` | 15 | `l1`, `level2`, `level3` |
| **Total** | | | **136** | **18** |

`sniff`'s 20th old target, `fixtures`, had 0 tests and was a child module of
`integration` too. It became a helper, not a root module (R19). That is why its
manifest maps 19 modules. The `sniff-cli` commit subject says "3 binaries";
the commit itself has 2, as R14 ruled.

## 1. Explicit targets and a complete manifest: met

- Each of the ten `Cargo.toml` files sets `autotests = false` and lists its
  `[[test]]` targets.
- `consolidation.py check-metadata` compares `cargo metadata` with each
  manifest's targets on name, path, and required features. It also checks
  `autotests = false`, and that every old target maps to exactly one module.
  Per package: `<pkg>/metadata-check.txt`, all PASS. All ten together:
  `metadata-check-all-ten.txt`, PASS, 18 targets.
- The manifests are the only authoritative old-to-new mapping, as in the first
  feature.
- The third clause, no undeclared crate root, is enforced by criterion 12's
  layout gate.

## 2. Identity preserved, four ways: met on macOS and Linux

`<pkg>/comparison-{darwin,linux,darwin-linux}.{json,md}` has verdict
`identical` and 0 failures for all ten packages, 30 comparisons in all. Each
compares the selected, excluded-as-other-tier, ignored, and platform-absent
sets for every supported feature set and selector, test by test. They are
produced by `scripts/ci/consolidation.py compare` from the committed
`capture-*` listings. `dmls` covers both `BISCUIT_L1_INCLUDE_SLOW` states.
Native Windows is covered by criterion 4.

Two deliberate identity changes are recorded, not hidden:

- **`biscuit-terminal-cli` F15:** 90 copies of the 10 `common::pane_geometry`
  unit tests fold to 10 identities in `level2`. `compare` folds the copies and
  fails if they disagree (`shared_tests` in the manifest).
- **Three neutral aliases (R5):** `level2_prose_cells` → `prose_cells`,
  `level2_diagrams` → `diagrams`, `real_terminal_render` → `terminal_render`.
  `compare` maps each old identity to its aliased path, and tier selection is
  unchanged.

## 3. Required-feature contracts kept: met

`check-metadata` checks each target's exact `required-features`. No
consolidation unioned features:

- `biscuit-file` keeps `fetch` on `l1-fetch` only (R3, R4).
- `sniff-cli` has one Level 2 contract, `level2` with exactly
  `["test-fixtures"]` (R14). The `()` comparison shows
  `level2_recent_commits_rendering` with 0 tests before and after, so no test
  was compiled under a feature it did not declare (spec hazard 3's premise did
  not hold; see §New 4).
- `biscuit-tui-cli`'s `windows_captured_stdout` joins the existing
  `terminal-tests` target, `level2`, rather than the feature-less `l1` (R4).

## 4. Platform conditions and platform proof: met

- **Conditions on module declarations.** `<pkg>/attribute-check.json`
  (`consolidation.py check-attributes`) has 0 failures for every package.
  `windows_captured_stdout` also keeps its inner `#![cfg(windows)]`, which
  `test-toolkit` asserts (R17).
- **macOS and Linux compile and list.** These are the comparisons in
  criterion 2. Linux ran on `build-linux`. `<pkg>/linux-l1-summary.txt` has
  each package's Linux L1 run with the CI feature union.
- **Native Windows and WSL2** (spec amendment 1): `just cross-check <pkg>
  --os windows` and `--os wsl`, run one leg at a time (R16), from the working
  tree (R13). Full output: `<pkg>/cross-check-{windows,wsl}.txt`.

  | Package | Windows (`build-win-native`) | WSL2 |
  |---|---|---|
  | `tree-hugger` | 484 passed | 484 passed |
  | `claudine` | 4,300 passed, **2 failed** (below) | 4,343 passed |
  | `sniff` | 1,937 passed, 22 skipped | 1,946 passed, 29 skipped |
  | `biscuit-file` | 784 passed | 778 passed |
  | `schematic-gen` | 559 passed, 8 skipped | 559 passed, 8 skipped |
  | `biscuit-terminal-cli` | 374 passed, 76 skipped | 377 passed, 76 skipped |
  | `claudine-gen` | 181 passed | 181 passed, 3 skipped |
  | `dmls` | 742 passed, 9 skipped | 740 passed, 9 skipped |
  | `sniff-cli` | 853 passed, 7 skipped | 857 passed, 7 skipped |
  | `biscuit-tui-cli` | 393 passed, 7 skipped | 412 passed, 18 skipped |

- **Windows-only modules compiled and run** (spec amendment 1):
  - `sniff`: `sniff::l1 windows_app_paths_orphan::orphaned_hkcu_entry_is_filtered`
    and `sniff::l1 windows_find_program_priority::path_wins_over_fallbacks_for_cmd`
    PASS in `sniff/cross-check-windows.txt`.
  - `biscuit-tui-cli`:
    `biscuit-tui-cli::level2 windows_captured_stdout::captured_stdout_receives_only_value_no_tui_bytes`
    PASS in `biscuit-tui-cli/cross-check-windows.txt`, selected by the L1
    filter. `biscuit-tui-cli/windows-captured-stdout.md` shows it absent
    from every macOS and Linux listing, before and after.
- **`claudine`'s two Windows failures are outside the move.** Both are unit
  tests in the `claudine` lib-test binary, which compiles only from
  `claudine/lib/src`. The move changed nothing there:
  - `composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`
    expects `B:\…\src` and gets `B:/…/src`.
  - `composition::sequence::task::tests::side_effect_tasks::a_failed_nested_mapping_set_projects_one_path_and_commits_nothing`
    gets an `unknown_root` error.

  An on-host base run could not be completed
  (`claudine/cross-check-windows-base.txt`: paging-file exhaustion, then a
  poisoned `target\`; implementation log, Phase 3). The failures are
  attributed by construction, not by a base run. They look like real Windows
  path-spelling defects in `claudine`, left to the author.

## 5. Test bodies unchanged: met, for reviewer confirmation

`<pkg>/body-diff.md` (`consolidation.py body-diff`) compares each old file
with its new module. `other` lines, which fail the check, are 0 in all ten
packages:

| Package | Files | Byte-identical | Structural | Comment | Other |
|---|---:|---:|---:|---:|---:|
| `tree-hugger` | 10 | 8 | 82 | 0 | 0 |
| `claudine` | 15 | 13 | 4 | 0 | 0 |
| `sniff` | 19 | 8 | 23 | 2 | 0 |
| `biscuit-file` | 15 | 15 | 0 | 0 | 0 |
| `schematic-gen` | 14 | 10 | 6 | 4 | 0 |
| `biscuit-terminal-cli` | 14 | 5 | 18 | 0 | 0 |
| `claudine-gen` | 11 | 10 | 4 | 7 | 0 |
| `dmls` | 11 | 6 | 24 | 0 | 0 |
| `sniff-cli` | 11 | 0 | 32 | 0 | 0 |
| `biscuit-tui-cli` | 15 | 4 | 27 | 0 | 0 |

Structural lines that are not attributes, `mod`, `use`, or blanks are allowed
only through a manifest disposition, and each is shown to be load-bearing:

- `crate_path` (`schematic-gen`, `claudine-gen`, `biscuit-tui-cli`):
  `crate::` inside string literals naming generated code, or a removed
  file-local `common` import (R19).
- `exact_path_string` / `exact_arg` (`claudine-gen`, `dmls`): self-exec
  `--exact` identities, each run once with the old string and seen to fail
  (`claudine-gen/exact-old-string.log.gz`, `dmls/exact-repair.txt`).
- `path_key` (`sniff-cli`): `spawn_site_guard`'s self-exclusion key (§7).

## 6. Snapshots and proptest seeds: met

- `biscuit-terminal-cli`: 7 of 7 snapshots mapped and moved byte for byte
  (`biscuit-terminal-cli/snapshot-check.json`, `snapshot-mapping.json`).
- `sniff-cli`: 12 of 12 (`sniff-cli/snapshot-check.json`,
  `snapshot-mapping.json`).
- No other package has an insta snapshot. `<pkg>/proptest-check.json` is clean
  for all ten. `biscuit-file`'s `yaml_mutation` seeds moved byte for byte to
  `tests/proptest-regressions/yaml_mutation.txt`, where proptest reads them
  now (R18).
- Every suite in §8 ran with `INSTA_UPDATE=no`, and no `.snap.new` file exists
  (checked after the final sweep).

## 7. Path-based guards cover the same files: met

S2 found one path-keyed guard over test files in scope, `sniff-cli`'s
`spawn_site_guard` (`spikes/s2-hazards.md`). `sniff-cli/guard-scans-after.md`
lists its scan set by path, before and after. The six former files are the
same once their `l1/` directory is removed. The only additions are the two
crate roots and the layout gate, which spawn nothing. Guard output is
unchanged: 0 spawn sites, and the same 4 PATH-escape sites at the same lines.
`claudine`'s `boundary_lint` reads fixed production paths and never scans
`tests/`, so the move does not affect it (implementation log, Phase 3).

A cross-package reader, `test-toolkit`'s
`the_windows_captured_stdout_test_is_discoverable_as_ordinary_l1`, now reads
`biscuit-tui/cli/tests/level2/windows_captured_stdout.rs` and passes (R17).

## 8. Local suites: met

`acceptance/final-sweep.sh` runs the same recipes as
`baseline/pre-existing.sh`, from each area directory in turn, with
`INSTA_UPDATE=no`, on `35295e442` plus Phase 6's two comment-only source edits.
Logs (gzipped) and `summary.tsv` are in `acceptance/final-sweep-logs/`. The
baseline column is `baseline/pre-existing.md` at `208051f75`.

| Area | `just test` (L1) | baseline | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|---|
| `tree-hugger` | ✅ 586 | 585 | ✅ 3 | ✅ | ✅ 0 stranded | ✅ |
| `claudine` | ✅ 7,320 (re-run, below) | 7,316 + 2 failed | ✅ 237 + 3 | ✅ | ✅ 0 stranded | ✅ |
| `sniff` | ✅ 2,826 | 2,824 | ✅ 6 | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-file` | ✅ 840 | 839 | n/a (stub) | ✅ | ✅ 0 stranded | ✅ |
| `schematic` | ✅ 1,700 | 1,699 | ✅ 3 | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-terminal` | ✅ 3,264 | 3,263 | ✅ 2 + 76 | ✅ | ✅ 0 stranded | ✅ |
| `darkmatter` | ✅ 8,491 | 8,490 | ✅ 18 + 69 + 3 | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-tui` | ✅ 992 | 991 | ✅ 21 | ✅ | ✅ 0 stranded | ✅ |

Each L1 count is the baseline plus one layout gate per package the area
migrated: +2 in `claudine` (`claudine`, `claudine-gen`) and `sniff` (`sniff`,
`sniff-cli`), and +1 in each of the other six areas. Every Level 2 count
equals the baseline. `test-l2` ran without taking focus: tmux and WezTerm are
used on this host, and Kitty has no usable instance (`os` skill, `macos.md`). Level 3 was not run, because it takes
desktop focus and is opt-in only.

**The one failure is the baseline's.** `claudine`**The baseline's two failures are fixed.** In the sweep, `claudine`'s
fail-fast `just test` stopped at 5,088 of 7,320. The `--no-fail-fast` re-run
(`claudine.test-no-fail-fast.log.gz`) passed 7,318 and failed the same two
baseline tests, both in `claudine-cli::l1`, a wave-1 package outside the ten:

- `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body`
- `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`

Commit `9d44e7988` (2026-09-23) adds one line to
`prompts/_implement/implement-plan.md`, `::file ../_test-tiers.md`, after the
spec note. The `claudine-cli` fixture and hash pin were not updated with it.
The fixture already had the `227e18c59` transclusion near the Completion
section, so the shipped prompt now pulls in `_test-tiers.md` twice. The review
fix (review-1) copies the new line into
`claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`.
After that, the fixture differs from the shipped file only by the documented
`say:`/`effect:`/`shell:` removals. The fix then refreshes
`shipped-hashes.json` with the test's own recipe
(`CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 just test-cli shipped_prompt_route_drift::`).
The frontmatter hash is unchanged. The body hash matches `md hash`
(`624f4bf489101b9f-9086e47509578e0a`). Re-run from `claudine/`, `just test`
passes all 7,320 tests with 9 skipped, and `just lint` passes.

er the sweep.

## 9. Replaced by spec §1: met

`measurements.md` records target count and test-executable bytes before and
after for each package, with `RUSTC_WRAPPER=""` and the before side in its own
target directory. Test executables went from 136 to 18, and their bytes from
2,538.7 MB to 794.7 MB (−68.7%) in a macOS debug build with each package's CI
feature set. The largest warm-edit change was +0.89 s (`biscuit-file`), well
under R8's trigger (more than 50% **and** more than 5 s), so the full
edit-latency protocol never ran.

## 10. CI observations: pending

`ci-observations.md` holds one row per package, all pending, and the harvest
procedure. No ordinary run has built a head that includes these migrations,
and none was triggered for this.

## 11. Docs honest: met

- `baseline/test-selector-consumers-after.md`
  (`baseline/consumer-sweep.py --after`, old names read from the ten
  manifests) has **no `--test <old>` selector** outside historical records and
  this feature's own evidence. Every remaining active or in-flight hit is a
  prose, binary-id, or file-path mention, and each carries a written
  disposition. None is left undispositioned. Historical hits are listed by
  file with their record date.
- Recipes that ran a former `--test` target were rewritten in the package's
  move commit, and each selects the same tests as before (R19):
  `schematic/justfile` `test-e2e` (`schematic-gen/recipe-rewrite.txt`) and
  `biscuit-tui/justfile` `test-pty` (`biscuit-tui-cli/recipe-rewrite.txt`).
- The `rust-testing` skill's "Consolidated Integration-Test Binaries" section
  covers the layout, positional name filtering, the feature boundary, and
  nextest process isolation. Phase 6 adds this wave's ten packages, target
  naming (R3), where feature-requiring L1 tests go (R4), neutral aliases (R5),
  and the proptest seed location (R18).

## 12. Structural guard inside each Level 1 binary: met

Each package's `l1` binary has
`test_layout::every_test_source_is_compiled_by_a_declared_target`, which calls
the shared `test_toolkit::test_layout` (spec amendment 3). `<pkg>/layout-guard.md`
shows each one red on a planted stray root (`tests/stray.rs`), red on an
undeclared module (`tests/l1/orphan.rs`), then green, for all ten packages.
It is a test inside the existing `l1` binary, not a new binary or CI gate.

## New 4. Every spec §3 hazard has a disposition: met

| Hazard | Disposition | Evidence |
|---|---|---|
| `biscuit-tui-cli` `real_terminal_render` | Neutral alias `terminal_render`, derived by `plan` and recorded in the manifest. The `real` selector picks 0 tests before and after on both hosts | `biscuit-tui-cli-migration.json`; `biscuit-tui-cli/comparison-*.md` |
| Former targets that start with a tier marker | `level2_prose_cells` → `prose_cells` and `level2_diagrams` → `diagrams`, which keep 49 unmarked tests in L1 (F3). `level3_chord_select` and every other `level2_*` target keep their names, because all their tests carry the marker. `compare` proves the tiers, not the names | `biscuit-terminal-cli/l1-tier-in-level2.txt`; `comparison-*.md` for the five packages |
| `sniff-cli`'s two Level 2 contracts | The premise did not hold: all four `level2_*` files are `#![cfg(feature = "test-fixtures")]` (F9). One `level2` target with exactly `test-fixtures` (R14) | `sniff-cli/metadata-check.txt`; the `()` comparison shows `level2_recent_commits_rendering` at 0 before and after |
| `windows_captured_stdout` and `terminal_capture` need `terminal-tests` | `windows_captured_stdout` joins `level2` and stays L1 (R4). `terminal_capture` is Level 2, so the hazard is moot (F2) | §4; `schematic-gen/comparison-*.md` |
| Two exact-path overrides | Both rewritten to module-qualified paths in every profile that carries them. Each matches exactly one test, and the old spelling matches 0 | `sniff/override-rewrite.txt`; `biscuit-terminal-cli/override-rewrite.txt` |
| Test-input identities | The narrowed filter selects the same tests before and after, for every package | `<pkg>/test-inputs.md`: verdict identical for all ten |

## New 5. No package with ten or more targets: met

`workspace-metadata.txt` (`cargo metadata --no-deps`): **118**
integration-test targets across the workspace (236 at `208051f75`). No
package has ten or more. The largest are `biscuit-clipboard-cli` and
`worktree-cli`, with 8 each.

## New 6. No stranded test: met

`just check-tier-coverage <area>` reports 0 stranded in all eight areas
(`acceptance/final-sweep-logs/<area>.check-tier-coverage.log`). The `real`
tier, whose `biscuit-tui` recipe is a stub, selects 0 tests before and after
(F4).

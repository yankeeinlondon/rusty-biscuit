---
title: Consolidate integration tests in the ten remaining double-digit packages
spec: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
phase: 1
total_phases: 6
agent: claude/opus
yolo: "true"
packages:
    - tree-hugger
    - claudine
    - sniff
    - biscuit-file
    - schematic-gen
    - biscuit-terminal-cli
    - claudine-gen
    - dmls
    - sniff-cli
    - biscuit-tui-cli
    - test-toolkit
source_files_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-probe.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-listings.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan.py
docs_updated_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/rulings.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s1-feature-sets.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-hazards.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan-output.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s3-cross-check.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s4-deps.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/skip-baseline.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers.md
    - darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
source_files_during_phase_2:
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.py
docs_updated_during_phase_2:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_2:
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/dry-run.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/r18-proptest-scratch.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-metadata-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-darkmatter-proptest-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-b.SHA256SUMS
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-a/
skills_files_updated_during_phase_2: []
---

# Implementation Plan — Consolidated Test Binaries, Wave 2

## Summary of Work and Definition of Success

### What this change is

This wave applies `2026-09-21-consolidated-test-binaries` to the ten workspace
packages that still have ten or more integration-test targets. In each package,
tests that share an execution contract are linked into one binary, declared
explicitly under `autotests = false`. Every test keeps its tier, features,
operating-system conditions, and assertions. The first feature's design
sections 1–7, alternatives, out-of-scope list, and acceptance criteria 1–8 and
10–12 apply unchanged, except where the spec amends them.

What is new here:

- **No pilot.** A lightweight before/after observation (target count and
  test-executable bytes) replaces criterion 9. The full edit-latency protocol
  runs only if a package trips the trigger defined in R8.
- **Reuse the tooling.** Use `scripts/ci/consolidation.py`, the mover,
  `metadata-check.py`, `body-diff.py`, and `test_toolkit::test_layout`,
  extending them only where these packages need it. Phase 2 does that
  extension.
- **Known identity hazards** (spec §3). Each needs a recorded disposition
  backed by evidence (new criterion 4).
- **No stranded tests** (new criterion 6). `just check-tier-coverage <area>` is
  clean for every area touched.

### Grounding against the current tree (2026-09-23)

The spec's counts were read at `e51fb0373`. `cargo metadata` today reports
**236** integration-test targets across the workspace, and **136** in the ten
packages (the spec said 232 and 132). Each package's generated inventory stays
authoritative. The expected column below is this plan's reading after the
checks in "Findings that change the spec's first reading".

| Package | Area | Targets now (spec) | Contracts (plan reading) | Expected targets |
|---|---|---:|---|---:|
| `tree-hugger` | `tree-hugger` | 10 (10) | L1, no features | 1 (`l1`) |
| `claudine` | `claudine` | 15 (13) | L1, no features | 1 (`l1`) |
| `sniff` | `sniff` | 20 (19) | L1, no features; `real_` tests live inside L1 files; 2 Windows-only files | 1 (`l1`) |
| `biscuit-file` | `biscuit-file` | 15 (15) | L1; L1 + `fetch` | 2 |
| `schematic-gen` | `schematic` | 14 (14) | L1; **L2** + `terminal-tests` (`terminal_capture` is L2, see F2) | 2 |
| `biscuit-terminal-cli` | `biscuit-terminal` | 14 (14) | L1; L2 + `terminal-tests`, which carries 49 L1-tier tests (F3) | 2 |
| `claudine-gen` | `claudine` | 11 (11) | L1; L2 + `terminal-tests` | 2 |
| `dmls` | `darkmatter` | 11 (10) | L1; L2 + `terminal-tests`; `l1-include-slow = true` | 2 |
| `sniff-cli` | `sniff` | 11 (11) | L1; L2 + `test-fixtures` (all four `level2_*` files are `#![cfg(feature = "test-fixtures")]`, F9) | 2 (was 3; rulings R14) |
| `biscuit-tui-cli` | `biscuit-tui` | 15 (15) | L1; L2 + `terminal-tests` (with the L1-tier `windows_captured_stdout`); L3 + `terminal-tests` | 3 |
| **Total** | | **136 (132)** | | **18** (was 19; F9) |

If those counts hold, the workspace drops from 236 targets to about **118**
(119 before Phase 1's F9 ruling).
The largest remaining packages are `worktree-cli` and `biscuit-clipboard-cli`,
with 8 each.

### Findings that change the spec's first reading

These came from reading the tree while planning. Each is ruled in Phase 1 and
re-proven by that package's inventory and four-way comparison.

- **F1 — Count drift.** `sniff` +1, `claudine` +2, `dmls` +1 since
  `e51fb0373`. Scope is still the ten named packages (R11).
- **F2 — `schematic-gen`'s `terminal_capture` is Level 2, not Level 1.** All
  three of its tests are named `level2_*`. It is the package's L2 +
  `terminal-tests` contract. The spec's hazard about joining the feature-less
  `l1` target therefore does not arise; record that disposition (R4).
- **F3 — `biscuit-terminal-cli` has 49 L1-tier tests inside `level2_*`
  targets.** `level2_prose_cells.rs` has 43 tests without a `level2_` prefix
  (inline modules `bold_containment`, `dim_containment`, …), and
  `level2_diagrams.rs` has 6 (`helper_tests`, …). CI's L1 cell builds this
  package with `terminal-tests` (`[package.metadata.ci.tests] features`), so
  those tests run at L1 in CI today. Under an un-aliased `level2_*` module they
  would silently become L2. This is the spec's hazard 2, confirmed. No other
  `level2_*` or `level3_*` file in scope has tests missing the marker.
- **F4 — `biscuit-tui`'s `test-real` recipe is a stub.** An un-aliased
  `real_terminal_render` module would put its 14 `level2_*` tests into a tier
  whose recipe does nothing. That is exactly the stranding criterion 6
  forbids.
- **F5 — `biscuit-tui-cli`'s `windows_captured_stdout` runs only in CI's L1
  cell.** It is `#![cfg(windows)]`, requires `terminal-tests`, and has one
  unmarked test. The area's local `just test` builds the CLI without features,
  so it has never run locally. That gap is pre-existing and out of scope, but
  it decides the target choice (R4).
- **F6 — Mixed-tier files exist.** `biscuit-tui-cli` has seven `level2_*`
  tests in feature-less files (`choose_one_output.rs`, `exit_codes.rs`, …).
  `sniff`, `sniff-cli`, `schematic-gen`, and `claudine-gen` carry `real_` tests
  inside otherwise-L1 files. Each area's `test-real` is live, apart from
  `biscuit-tui`, which has no `real_` tests.
- **F7 — The tooling is hard-coded to the first feature.** `consolidation.py`'s
  `PACKAGES` and `PACKAGE_FEATURE_SETS` name only the four wave-1 packages, so
  `capture` refuses an unknown package. The mover (`pilot/apply-move.py`),
  `acceptance/metadata-check.py`, and `acceptance/body-diff.py` fix their
  feature directory, manifest names, and base revisions in code. None of them
  can be "pointed at" this feature without edits (R1, R2).
- **F8 — Both exact-path overrides are confirmed.**
  `test(=level2_render_tree_style_in_wezterm)` appears in the `default` and
  `ci` profiles and is defined at
  `biscuit-terminal/cli/tests/level2_render_tree_style.rs:1064`.
  `test(=test_detect_completes_in_reasonable_time)` appears in the `ci`
  profile only and is defined at `sniff/lib/tests/integration.rs:73`.

Phase 1 added these (see `rulings.md` R14–R19):

- **F9 — `sniff-cli` has one L2 feature contract.**
  `level2_recent_commits_rendering.rs:22` is
  `#![cfg(feature = "test-fixtures")]`, like the other three `level2_*` files.
  So without the feature its manifest-feature-less target is empty, and
  spec hazard 3's premise does not hold. Result: 2 targets (R14; the author may
  restore the split before Phase 5).
- **F10 — Three more packages need an unconditional `test-toolkit`
  dev-dependency.** `schematic-gen`, `biscuit-terminal-cli`, and
  `claudine-gen` have it only as an optional dependency behind
  `terminal-tests` (R7, `spikes/s4-deps.md`).
- **F11 — Proptest regression files resolve to
  `tests/proptest-regressions/<stem>.txt` once `tests/<target>/main.rs`
  exists.** `biscuit-file`'s `yaml_mutation` seeds move there (R18).
  Wave 1's darkmatter seeds were moved beside their modules and are no longer
  replayed. That is filed as
  `darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation`.
- **F12 — `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053` reads
  `biscuit-tui/cli/tests/windows_captured_stdout.rs` by path** and asserts
  its inner `#![cfg(windows)]`. Phase 5 updates that path and keeps the
  inner attribute (R4, R17).
- **F13 — `test_inputs.py` never narrows into a `level2*`/`level3*`
  binary.** No in-scope reference is newly hidden by that (R15).
- **F14 — `cross-check` ships the working tree, not `HEAD`,** and keeps no
  per-test report. Windows evidence is its `tee`d PASS lines (R13, R16).

Phase 2 added these (see `selfproof/README.md` and the implementation log):

- **F15 — `biscuit-terminal-cli`'s shared `tests/common/` holds 10 unit tests**
  (`common::pane_geometry::tests::*`). Each of the 9 `level2_*` files that
  declares `mod common;` compiles a copy, so there are 90 identities today.
  With one `common` per root (first spec §3), they become 10 identities in
  `level2`. `plan` now records them as `shared_tests`, and `compare` folds the
  copies and fails if they disagree. These are L1-tier tests, in addition to
  F3's 49. Phase 4 records the 90 → 10 fold as this package's disposition.
- **F16 — The tier-by-name default does not give the ruled targets** for
  `schematic-gen` (`l1-terminal`), `biscuit-tui-cli` (`l1-terminal`, `real`),
  `sniff-cli` (`level2` plus `level2-test-fixtures`), or `sniff`'s `fixtures`
  helper. `consolidation.py`'s reviewed `RULED_TARGETS` table encodes R4, R14,
  and R19, and `plan` checks each entry against the captures.
- **F17 — Wave 1's `claudine-cli` feature table is stale.** Its CI union has
  since gained `test-fixtures`, so `capture --package claudine-cli` now refuses
  until that table is updated. `claudine-cli` is outside this wave; nothing
  here captures it.

### Non-negotiable constraints

- **Structural edits only** (first spec §3). A test that needs a behavior
  change stops that package's migration and becomes a separate defect.
- **Tier expressions come only from `just _tier_filter`.** OS `cfg` stays a
  module condition and never creates a target.
- **Identity comparison is exact and four-way**: selected,
  excluded-as-other-tier, ignored, and platform-absent. It runs per package,
  per supported feature set, and per platform (macOS, Linux, and both
  together). A count is never evidence.
- **Features are never unioned.** A target's `required-features` is exactly
  its contract's set (spec hazard 3: `sniff-cli`'s two L2 contracts).
- **One package at a time.** Each package is committed separately with its
  manifest and evidence, in the spec's order: single-contract, then
  two-contract, then hazard packages.
- **No new CI run, matrix cell, or gate.** Discovery uses local hosts and
  `just cross-check`. CI observations come from ordinary runs.
- **Snapshots move mechanically** under `INSTA_UPDATE=no`. No `.snap.new` may
  exist, and nothing is regenerated to get a suite green.
- **The migration manifests are the only authoritative old-to-new mapping**
  (first feature, author decision 2026-09-22).
- **Terminal and browser verification never takes host focus.**
- **The agent never moves the spec to `_completed`** and never runs
  `just complete`.

### What success looks like

- The ten packages each have `autotests = false`, an explicit `[[test]]` list
  matching the plan's expected targets or a recorded reason for a difference,
  and a committed `<package>-migration.json`. `check-metadata` passes against
  all ten.
- The four-way comparison is identical on macOS, on Linux, and on both
  together for every supported feature set. For `dmls` it also covers both
  `BISCUIT_L1_INCLUDE_SLOW` states.
- `just cross-check <package> --os windows` and `--os wsl` pass for every
  package. For `sniff` and `biscuit-tui-cli`, the Windows run shows the
  Windows-only modules compiled and run.
- Every spec §3 hazard has a disposition in `acceptance.md` with evidence:
  - the `real_terminal_render` alias in the manifest;
  - `sniff-cli`'s separate L2 targets;
  - each rewritten override matching exactly one test;
  - the test-input narrowed filter selecting the same tests before and after.
- Each package's `l1` binary holds a `test_layout` guard. It is shown red on a
  planted stray root and on an undeclared module, then green.
- `just test`, `just test-l2`, `just lint`, `just check-tier-coverage`, and
  `just check-canonical` pass for all eight areas touched, or fail only on
  failures proven pre-existing in Phase 1.
- `measurements.md` records target count and executable bytes before and after
  for each package.
- `cargo metadata` shows no workspace package with ten or more integration-test
  targets, and `acceptance.md` gives the new workspace total.
- No active recipe, doc, or skill recommends `--test <old-binary>` for a
  migrated target.

## Phase 1 — Rulings, Spikes, and Baselines

No production file changes in this phase, apart from the spec's `status`
moving from `draft-spec` to `planned`. The outputs are committed under this
feature directory: `rulings.md`, `spikes/`, `baseline/`, and
`implementation-log.md`.

### Necessary Rules

Each ruling has a recommended default, so implementation is never blocked. A
ruling task closes by writing its decision and consequences into `rulings.md`.
Wave-1 rulings R1–R9 of the first feature carry forward except where
restated here.

- [x] **R1 — Generalize `consolidation.py` for the ten packages.** Should the
      hard-coded `PACKAGES` and `PACKAGE_FEATURE_SETS` tables grow, or should
      feature sets be derived?
      *Recommended:* add the ten packages to both tables as reviewed data.
      Derive nothing from justfiles, which would be fragile. Keep the
      capture-time check that the CI feature union is one of the listed sets.
      Proposed sets, to be confirmed against each area's recipes in S1:
      - `sniff`: `()`, `(remote)`, `(network)`
      - `sniff-cli`: `()`, `(test-fixtures)`
      - `biscuit-tui-cli`: `()`, `(terminal-tests)`
      - `biscuit-file`: `()`, `(fetch)`
      - `schematic-gen`: `()`, `(terminal-tests)`
      - `biscuit-terminal-cli`: `()`, `(terminal-tests)`
      - `claudine`: `()`, plus whatever `claudine/justfile` passes
      - `claudine-gen`: `()`, `(terminal-tests)`
      - `tree-hugger`: `()`
      - `dmls`: `()`, `(effects-instrumentation)`,
        `(terminal-tests, effects-instrumentation)`
- [x] **R2 — Where the mover and acceptance checks live.** Both hard-code
      `features/2026-09-21-consolidated-test-binaries`, and that directory
      moves to `_completed` when the author closes it.
      *Recommended:* promote all three scripts into `consolidation.py`
      subcommands with explicit inputs:
      - `move <manifest> [--layout-gate <file>]`;
      - `check-metadata --manifest <file>…`; and
      - `body-diff --manifest <file> --base-rev <rev> --markdown <out>`.

      Cover each with `test_consolidation.py` fixtures. Leave the originals
      untouched as the first feature's record. This follows the spec's
      "extending it only where a package needs something new": a second
      feature is exactly that need, and copying 370 lines into a second
      feature directory creates two copies that will drift. Also let `move`
      emit root `mod` lines in rustfmt order and omit `common` from a root
      whose modules do not use it. Both were recurring manual steps in wave 1.
- [x] **R3 — Target names.** Tiers are named `l1`, `level2`, and `level3`.
      When a tier has more than one feature contract, the targets take a
      feature suffix, as darkmatter's `level3-terminal`/`level3-browser` did.
      *Recommended:*
      - `biscuit-file`: `l1` and `l1-fetch`.
      - `sniff-cli`: `level2` (no features) and `level2-fixtures`
        (`test-fixtures`).

      The feature-less contract keeps the bare name because it is the default
      contract. The feature-bearing one is the exception. Record this as a
      refinement of wave-1 R2.
- [x] **R4 — Where L1-tier tests that require a feature go.** This covers
      `windows_captured_stdout` (F5), the 49 tests in F3, and the spec's
      `terminal_capture` question (F2).
      *Recommended:* the target set is decided by distinct `required-features`
      sets. An L1-tier test that requires feature F joins the package's
      existing target for F, following the `harness_integrity` precedent. Only
      when no such target exists does it get its own `l1-<feature>` target
      (`biscuit-file`'s `fetch_integration`). Tier selection stays
      filter-based, so CI's L1 cell, which builds with the package's CI
      features, still selects these tests.
      Dispositions:
      - `windows_captured_stdout` joins `biscuit-tui-cli`'s `level2` target
        as a `#[cfg(windows)]` module.
      - The F3 tests stay in `biscuit-terminal-cli`'s `level2` target under
        neutral aliases (R5).
      - `terminal_capture` is `schematic-gen`'s `level2` target. Its
        spec-hazard disposition is "moot: no L1 test".
- [x] **R5 — Neutral aliases.** Wave-1 R2 carries forward: a former target
      name becomes the module name unless `plan`'s projection shows it would
      newly match, or stop matching, a `_tier_filter` or override filter. In
      that case the marker prefix is stripped. Required aliases known today:
      - `real_terminal_render` → `terminal_render`;
      - `level2_prose_cells` → `prose_cells`; and
      - `level2_diagrams` → `diagrams`.

      Any other alias `plan` emits is reviewed and recorded. Former `level2_*`
      targets whose tests all carry the marker keep their names, because the
      rule is mechanical and does not tidy names.
- [x] **R6 — Mixed-tier files stay whole.** Wave-1 R4 generalized: a source
      file whose tests carry different markers (F6) lands whole in the target
      its `required-features` dictates, and the filter decides tier. Files
      are never split.
- [x] **R7 — Test-toolkit dev-dependency.** Add `test-toolkit` as an
      unconditional dev-dependency to `biscuit-file` and `tree-hugger`. S4
      proves no cycle. Record in `docs/dependencies.md` and each area's
      `docs/dependencies.md` whether it adds any crate to either package's
      graph. Wave-1 found `test-toolkit` must be unconditional for the layout
      gate.
- [x] **R8 — What "noticeably longer" means** (spec §1 trigger).
      *Recommended:* record one warm edit-to-one-module observation per
      package, before and after. Touch one L1 module file, then run
      `cargo nextest run -p <pkg> -E "$(just _tier_filter L1 <pkg>)" <module>`
      with `RUSTC_WRAPPER=""` and kache shims off `PATH`. Run the full
      five-trial protocol only if the after figure exceeds the before by more
      than 50% **and** more than 5 s. Otherwise, the count and bytes rows are
      the evidence.
- [x] **R9 — Evidence layout.** Wave-1's per-package set carries forward.
      Each package gets `<package>-migration.json` at this feature's top level
      and a `<package>/` directory containing:
      - `capture-before/`, `capture-after/`, `capture-linux-before/`,
        `capture-linux-after/`;
      - `comparison-{darwin,linux,darwin-linux}.{json,md}`;
      - `attribute-check.json`;
      - `snapshot-mapping.json` and `snapshot-check.json`, where snapshots
        exist;
      - `metadata-check.txt` and `body-diff.md`;
      - `test-inputs.md`, which is new (spec hazard 6);
      - `layout-guard.md`, which is new (red/green proof); and
      - `guard-scans-after.md`, where a path guard exists.
- [x] **R10 — Override rewrites.** Rewrite:
      - `test(=level2_render_tree_style_in_wezterm)` →
        `test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)`,
        in both the `default` and `ci` profiles; and
      - `test(=test_detect_completes_in_reasonable_time)` →
        `test(=integration::test_detect_completes_in_reasonable_time)`, in
        `ci`.

      Each is shown matching exactly one test afterward. Use a
      `cargo nextest list -E '<filter>'` listing under the profile's feature
      set, committed in the package evidence. The package-level filters
      (`claudine-l1` test group, `sniff-windows-l1`) and the unanchored
      `test(/level2_/)` and `test(/browser_/)` are re-proven by `capture`'s
      per-override listing, not reasoned about.
- [x] **R11 — Scope under drift.** Scope is the ten named packages. If another
      package crosses ten targets before closeout, making new criterion 5
      unmeetable, stop and raise it with the author (human-in-the-loop). Do
      not silently widen scope.
- [x] **R12 — Commits.** The spec directs one package per commit series, so
      commits are authorized per package: manifest and evidence, structural
      move, then area docs, following wave-1 R7. Toolkit changes (Phase 2) are
      their own commit. Every commit is signed with the author's identity,
      carries no agent attribution trailer (repo `CLAUDE.md` overrides the
      harness default), and is checked with `git verify-commit HEAD`. A
      signing failure halts the work; `--no-gpg-sign` is never used.
- [x] **R13 — Sequencing overlap.** Wave-1 R8 carries forward. Package N's
      remote verification (Linux capture, Windows, WSL2) may run while
      package N+1's read-only baseline is captured. Package N+1's structural
      move starts only after package N's local commit. Remote legs run
      against committed revisions (confirmed by S3).

### Wave 1 — Rulings and ruling-independent spikes (parallel)

- [x] **Record rulings** — write R1–R13 into `rulings.md`. Move the spec's
      `status` to `planned`. Create `implementation-log.md`. This blocks
      Phase 2 on R1–R3 only.
- [x] **S1 — Feature-set and recipe census** — for each of the ten packages,
      record every `--features` value its area's canonical recipes pass
      (`test`, `test-l2`, `test-l3`, `test-real`, `test-minimal`, the CI
      branch of `test`), the `[package.metadata.ci.tests]` union, and
      `l1-include-slow`. Output: `spikes/s1-feature-sets.md`, which is R1's
      input table.
- [x] **S2 — Crate-global and identity-string scan** — run the first
      feature's S2 detector list (now in `consolidation.py check-attributes`)
      read-only across the ten packages' `tests/`. It flags:
      - `#[macro_export]`, global allocators, link or export attributes, and
        constructors;
      - duplicated crate-root symbols across files that will share a crate;
      - inner `#![…]` attributes anywhere in the leading block;
      - relative `mod`, `#[path]`, `include_*`, and fixture paths;
      - `--exact <path>` strings and `current_exe()` + `--list` self-exec;
      - insta usage and snapshot directories (`sniff-cli` has
        `tests/snapshots/`); and
      - path-keyed guards (`sniff-cli`'s `spawn_site_guard`, `claudine`'s
        `boundary_lint`).

      Output: `spikes/s2-hazards.md`, a per-package hazard list, each item
      with a planned disposition.
- [x] **S3 — `cross-check` behavior** — read `cross-check`'s source and the
      `os` skill to confirm three things:
      - whether it syncs the committed `HEAD` or the working tree;
      - which features it builds per package (it must include
        `terminal-tests` for `biscuit-tui-cli`, or `windows_captured_stdout`
        is never compiled); and
      - where its output shows per-test results, so a Windows-only module can
        be shown run rather than inferred.

      Output: `spikes/s3-cross-check.md`. If `cross-check` cannot show
      per-test Windows evidence, name the smallest addition to its report and
      raise it before Phase 3.
- [x] **S4 — Dependency census** — confirm `test-toolkit` depends on none of
      the ten packages, and list the crates it would add to `biscuit-file`'s
      and `tree-hugger`'s dev graphs (`cargo tree -e dev`). Output:
      `spikes/s4-deps.md`.

### Wave 2 — Baselines (parallel; needs S1)

- [x] **Pre-existing state** — on the unmigrated tree, run `just test`,
      `just test-l2`, `just lint`, `just check-tier-coverage`, and
      `just check-canonical` for the eight areas: `tree-hugger`, `claudine`,
      `sniff`, `biscuit-file`, `schematic`, `biscuit-terminal`, `darkmatter`,
      and `biscuit-tui`. Record every failure and stranded test in
      `baseline/pre-existing.md`, so none is later attributed to the
      migration. Record whether F3's 49 tests and F5's test count as stranded
      today.
- [x] **Skip baseline** — confirm `.github/ci/ci-baseline.toml` has no entry
      for the ten packages. Record in `baseline/skip-baseline.md`.
- [x] **Test-input probes** — for each package, pick one fixture or file its
      tests read, as recognized by `scripts/ci/test_inputs.py` (a literal or
      an `include_str!` in test context). Record today's narrowed
      `binary_id(...) & test(=...)` filter and its listed test set in
      `baseline/test-inputs.md`. Confirm from `test_inputs.py`'s source
      (`targets_from_metadata`, `_declarations`) that identities come from
      walking `mod` declarations from live target roots, with no stored
      identity.
- [x] **Active `--test` consumer sweep** — run wave-1's
      `baseline/consumer-sweep.py` logic (or its `consolidation.py`
      successor) over justfiles, docs, and skills for the ten packages' old
      target names. Output: `baseline/test-selector-consumers.md`, split into
      active and historical.

### Checkpoint

- [x] `rulings.md` records R1–R13. The spec's `status` is `planned`.
- [x] S1–S4 and the four baselines are committed. Every S2 hazard has a
      planned disposition.
- [x] Any finding that contradicts the expected-target table is written into
      this plan's table with its reason before Phase 3.

## Phase 2 — Toolkit Extension

Extend the first feature's toolkit per R1 and R2. Nothing in the ten packages
moves in this phase. The phase closes on a no-op self-proof against the
unmigrated tree.

### Wave 1 — Independent changes (parallel)

- [x] **Package tables** — add the ten packages to `PACKAGES` and
      `PACKAGE_FEATURE_SETS` in `scripts/ci/consolidation.py` from S1's table.
      Extend the `inventory` and `capture` fixtures in
      `scripts/ci/test_consolidation.py` so an unlisted package still
      refuses.
- [x] **`move` subcommand** — port `pilot/apply-move.py` behavior into
      `consolidation.py move`. That covers file moves, nested-root moves, and
      `inner_cfg` stripping onto the module declaration; it also covers
      `mod common;` → `use crate::common;`, the relative `include_*` repair,
      and root generation. Add the R2 refinements: rustfmt-ordered `mod` lines
      and `common` only where used. Add fixture tests, including one that
      proves an alias from the manifest becomes the module name.
- [x] **`check-metadata` and `body-diff` subcommands** — port both scripts,
      taking manifests and base revision as arguments. Exit 1 on a mismatch.
      Add fixture tests for a missing target, an extra target, a feature
      mismatch, and a body change.

### Wave 2 — Self-proof (needs Wave 1)

- [x] **Toolkit suite** — `python3 scripts/ci/test_consolidation.py` passes,
      every new failure oracle is shown red then green, and `just ci-local`'s
      Python leg is green.
- [x] **No-op comparison** — `capture` the unmigrated tree twice for all ten
      packages and every S1 feature set, then `compare`: identical, trivially
      mapped. Commit `selfproof/noop-comparison.{json,md}`.
- [x] **Regression check on wave 1** — run `check-metadata` against the four
      wave-1 manifests. It must pass, which proves the port matches the
      originals.

### Checkpoint

- [x] `just lint` passes for the repo `scripts/ci` surface, and the toolkit
      suite is green.
- [ ] The no-op self-proof for ten packages is committed. The toolkit commit
      is signed and verified.
      *(Phase 2 produced the self-proof in `selfproof/`. The phase instructions
      forbid committing, so the signed toolkit commit (R12) is left to the
      separate commit step.)*

## Phase 3 — Single-Contract Packages (`tree-hugger`, `claudine`, `sniff`)

### Standard package procedure (SPP)

Every package in Phases 3–5 follows these steps. Package sections list only
their distinct hazards and checklist.

1. **Before.**
   - Confirm the skip baseline.
   - `capture` on macOS for every feature set (both slow states where
     `l1-include-slow`).
   - Linux `capture` on `build-linux`, using the `os` skill's recipe with
     `RUSTC_WRAPPER="" KACHE_AUTO=0`.
   - Lightweight before-measurement: `cargo test --no-run --locked -p <pkg>
     --features <CI set>` in a detached base worktree with its own
     `--target-dir`, summing `compiler-artifact` test executables.
   - The R8 warm edit observation.

   Always re-capture fresh: `plan` refuses a stale capture after any
   `.config/nextest.toml` change.
2. **Manifest.** Run `inventory` and `plan`, then commit
   `<package>-migration.json`. Review every alias, contract, and the S2
   hazards for this package.
3. **Move.**
   - Run `consolidation.py move`.
   - Set `autotests = false` and the explicit `[[test]]` entries with exact
     `required-features`.
   - Make the manual structural edits the mover does not: helper directories,
     `#[path]` helpers, `cfg` on `use crate::common` imports, proptest
     regression files, snapshots, guards, and overrides.
4. **Layout gate.** Add `tests/l1/test_layout.rs` using
   `test_toolkit::test_layout`, with a non-vacuous file-count assertion and
   expected paths. Record in `<package>/layout-guard.md` three results: the
   gate fails on a planted `tests/stray.rs`, fails on an undeclared
   `tests/l1/orphan.rs`, then passes once both are removed.
5. **Checks.**
   - `check-attributes`, `check-snapshots` (where snapshots exist),
     `check-proptest` (R18), `check-metadata`, and `body-diff`.
   - `compare` on macOS. Any difference is a manifest or move defect, never a
     filter change.
6. **Suites.**
   - The area's `just test`, `just test-l2`, and `just lint` with
     `INSTA_UPDATE=no`, and no `.snap.new` in the tree.
   - `just check-tier-coverage <area>` and `just check-canonical <area>`.
   - Results are compared with `baseline/pre-existing.md`.
7. **Identity consumers.**
   - Rewrite any exact override (R10) and prove it matches exactly one test.
   - Run the test-input probe after the move and confirm the same test set
     (`<package>/test-inputs.md`).
   - Diff path guards' scanned-file lists by path.
8. **After.** Lightweight after-measurement and the R8 observation go into
   `measurements.md`. If R8's trigger fires, run the full protocol before
   the next package.
9. **Docs.** Fix the area's active `--test <old>` references from the
   consumer sweep, using positional name filters and keeping the tier's
   `-E`.
10. **Commit.** Commit the package series (R12) and log it in
    `implementation-log.md`.
11. **Remote.**
    - Linux after-capture, `compare` for Linux, and darwin-linux with
      platform-absent evaluated.
    - `just cross-check <pkg> --os windows` and `--os wsl`.
    - Any defect is fixed in a follow-up commit within this package, before
      its checkpoint closes.

    This step may overlap the next package's step 1 (R13).

### Wave 1 — `tree-hugger` (10 → 1)

Hazards: this is the first new ground for the promoted `move`, and
`test-toolkit` is a new dev-dependency (R7).

- [ ] **SPP 1–2** — before-evidence and `tree-hugger-migration.json`.
- [ ] **SPP 3–4** — move to `tests/l1/` and add the dev-dependency, with the
      `docs/dependencies.md` entries per S4. Add the layout gate.
- [ ] **SPP 5–9** — checks, suites, consumers, after-measurement, docs.
- [ ] **SPP 10–11** — commit, Linux compare, Windows, WSL2.

### Wave 2 — `claudine` (15 → 1)

Hazards:
- This is the heaviest per-binary package (375 MB archived for 14 binaries in
  pull request 92), and the likely source of most of this wave's CI saving.
- `boundary_lint` may be a path-keyed guard (S2 confirms). If it is, its
  scanned-file list must match by path.
- The two `*_spike` targets are not tier markers. Confirm by projection.
- The package-level `claudine-l1` CI test group must still match the same
  identities.

- [ ] **SPP 1–2** — before-evidence and `claudine-migration.json`.
- [ ] **SPP 3–4** — move and layout gate. Check whether
      `claudine/cli/tests/l1/test_placement.rs` scans `claudine/lib`. If it
      does, extend it rather than adding a second gate.
- [ ] **SPP 5–9** — include the `boundary_lint` scan diff and the
      `claudine-l1` override listing.
- [ ] **SPP 10–11** — commit, Linux, Windows, WSL2.

### Wave 3 — `sniff` (20 → 1)

Hazards:
- `windows_app_paths_orphan` and `windows_find_program_priority` become
  `#[cfg(target_os = "windows")]` modules, and must be shown run on Windows
  (spec criterion 1).
- R10's `integration::test_detect_completes_in_reasonable_time` override
  rewrite.
- The `bench_*` test targets are ordinary test targets, not `[[bench]]`, and
  they move.
- `real_` tests stay in L1 files (R6) and are exercised by the live
  `test-real` with `--features network`.

- [ ] **SPP 1–2** — before-evidence for `()`, `(remote)`, and `(network)`,
      and `sniff-migration.json`.
- [ ] **SPP 3–4** — move and layout gate. Check that `check-attributes`
      carries both Windows `cfg`s onto their declarations.
- [ ] **SPP 5–9** — include the override rewrite listing exactly one test in
      the `ci` profile, and `sniff-windows-l1` group identity equality.
- [ ] **SPP 10–11** — commit, Linux, and WSL2. The Windows run's evidence
      names both Windows-only modules' tests as run.

### Checkpoint

- [ ] Three packages are migrated and committed, each with complete R9
      evidence. Four-way identity is equal on macOS, Linux, and both
      together.
- [ ] Windows and WSL2 pass for all three, and `sniff`'s Windows-only modules
      are shown run.
- [ ] The tooling held on new ground. Any toolkit fix it needed is its own
      commit, with a fixture test.

## Phase 4 — Two-Contract Packages

Order follows the spec: `biscuit-file`, `schematic-gen`,
`biscuit-terminal-cli`, `claudine-gen`, `dmls`. The inventory is rebuilt for
each; no earlier package's shape is assumed.

### Wave 1 — `biscuit-file` (15 → 2: `l1`, `l1-fetch`)

Hazards:
- `test-toolkit` is a new dev-dependency (R7).
- The `test-minimal` recipe proves the unfeatured boundary, and must still
  pass.
- `fetch_integration` keeps exactly `required-features = ["fetch"]`.

- [ ] **SPP 1–2** — before-evidence for `()` and `(fetch)`, and
      `biscuit-file-migration.json`.
- [ ] **SPP 3–4** — move, dev-dependency, `docs/dependencies.md`, layout
      gate.
- [ ] **SPP 5–9** — checks, then suites including `just test-minimal`.
- [ ] **SPP 10–11** — commit and remote legs.

### Wave 2 — `schematic-gen` (14 → 2: `l1`, `level2`)

Hazards:
- `terminal_capture` is the L2 contract (F2, R4), and its disposition is
  recorded.
- The `real_` tests stay in L1 files (R6) and run under the live
  `test-real`.
- `postman_golden` and `artifact_drift` likely read golden files. S2 lists
  their path repairs.

- [ ] **SPP 1–2** — before-evidence for `()` and `(terminal-tests)`, and
      `schematic-gen-migration.json`.
- [ ] **SPP 3–4** — move and layout gate.
- [ ] **SPP 5–9** — checks, suites (`test-l2` with tmux, no focus), and a
      test-input probe on a golden file.
- [ ] **SPP 10–11** — commit and remote legs.

### Wave 3 — `biscuit-terminal-cli` (14 → 2: `l1`, `level2`)

Hazards:
- F3: the 49 unmarked tests need the `prose_cells` and `diagrams` aliases
  (R5), shown in the manifest and by the L1 selected set being equal.
- R10's override rewrite in both profiles.
- `level2_render_tree_style` keeps its name, because all its tests carry the
  marker.

- [ ] **SPP 1–2** — before-evidence for `()` and `(terminal-tests)`. Check
      that `biscuit-terminal-cli-migration.json` records both aliases, and
      that `plan`'s projection derived them rather than having them written
      by hand.
- [ ] **SPP 3–4** — move and layout gate.
- [ ] **SPP 5–9** — the comparison shows all 49 tests still in the L1
      selected set under `(terminal-tests)`. The override matches exactly one
      test in `default` and in `ci`. L2 runs on the WezTerm, Kitty, tmux, and
      Apple Terminal backends without focus.
- [ ] **SPP 10–11** — commit and remote legs.

### Wave 4 — `claudine-gen` (11 → 2: `l1`, `level2`)

Hazards:
- It archived 218 MB for 14 binaries in pull request 92.
- `drift`, `fixtures_provenance`, and `signals_sidecar_mirror` read
  repository files, so they are good test-input probe candidates.
- The `real_` tests run under the live `claudine` `test-real`.

- [ ] **SPP 1–2** — before-evidence and `claudine-gen-migration.json`.
- [ ] **SPP 3–4** — move and layout gate.
- [ ] **SPP 5–9** — checks, suites, consumers, after, docs.
- [ ] **SPP 10–11** — commit and remote legs.

### Wave 5 — `dmls` (11 → 2: `l1`, `level2`)

Hazards:
- `l1-include-slow = true`, so capture and compare cover both
  `BISCUIT_L1_INCLUDE_SLOW` states.
- The CI feature union is `terminal-tests` + `effects-instrumentation`.
- `level1_*` targets are not markers. Confirm by projection.
- Wave 1 saw a stale `zed-dmls-cli` build-script path from a shared target
  dir, so the before side uses its own `--target-dir`.

- [ ] **SPP 1–2** — before-evidence for three feature sets × two slow states,
      and `dmls-migration.json`.
- [ ] **SPP 3–4** — move and layout gate.
- [ ] **SPP 5–9** — checks, suites (`test-l2` on its tmux backend, no
      focus), consumers, after, docs.
- [ ] **SPP 10–11** — commit and remote legs.

### Checkpoint

- [ ] Five more packages are migrated and committed, with complete evidence.
      Four-way identity is equal on macOS, Linux, and both together.
- [ ] F2's and F3's dispositions are recorded with evidence. R10's
      `biscuit-terminal-cli` override matches exactly one test in each
      profile.
- [ ] Windows and WSL2 pass for all five.

## Phase 5 — Hazard Packages (`sniff-cli`, `biscuit-tui-cli`)

### Wave 1 — `sniff-cli` (11 → 3: `l1`, `level2`, `level2-fixtures`)

> **Phase 1 ruling R14 supersedes the split below unless the author restores
> it before this phase.** All four `level2_*` files are
> `#![cfg(feature = "test-fixtures")]`, so the ruled shape is **2 targets**
> (`l1`, `level2` with `required-features = ["test-fixtures"]`). Read
> "`level2-fixtures`" as `level2`, and "`level2` with no
> `required-features`" as not applicable. The `()` four-way comparison must
> show `level2_recent_commits_rendering` with zero tests before and after.

Hazards:
- Spec hazard 3: `level2_recent_commits_rendering` gets the feature-less
  `level2` target, and the other three L2 files get `level2-fixtures` with
  exactly `test-fixtures`.
- `spawn_site_guard` is a path-keyed guard, so its file list must match by
  path.
- The `snapshots` target and `tests/snapshots/` need an insta mapping.
- The package shares the `sniff-windows-l1` group.

- [ ] **SPP 1–2** — before-evidence for `()` and `(test-fixtures)`, and
      `sniff-cli-migration.json` with the two L2 targets.
- [ ] **SPP 3–4** — move, including a mechanical snapshot move under
      `INSTA_UPDATE=no`, and the layout gate.
- [ ] **SPP 5–9** — `check-metadata` shows `level2` with no
      `required-features`. The `spawn_site_guard` scan diff is written to
      `guard-scans-after.md`, and `check-snapshots` passes.
- [ ] **SPP 10–11** — commit and remote legs.

### Wave 2 — `biscuit-tui-cli` (15 → 3: `l1`, `level2`, `level3`)

Hazards:
- F4: the `real_terminal_render` → `terminal_render` alias.
- Spec hazard 2: `level3_chord_select` keeps its name, because all four of
  its tests carry `level3_`.
- F5/R4: `windows_captured_stdout` joins `level2` as a `#[cfg(windows)]`
  module and stays L1 tier.
- F6: the seven `level2_*` tests in feature-less files stay in `l1`.
- The L3 tier steals focus. Run it only per its existing opt-in, never
  unattended.

- [ ] **SPP 1–2** — before-evidence for `()` and `(terminal-tests)`, and
      `biscuit-tui-cli-migration.json` recording the alias.
- [ ] **SPP 3–4** — move and layout gate.
- [ ] **SPP 5–9** — check these:
      - The comparison shows no test newly selected by the `real` tier.
      - `check-tier-coverage biscuit-tui` is clean.
      - `captured_stdout_receives_only_value_no_tui_bytes` is in the L1
        selected set under `(terminal-tests)` on the Windows capture and
        platform-absent on macOS and Linux.
      - L2 passes without focus on tmux, WezTerm, and Kitty.
- [ ] **SPP 10–11** — commit, Linux, and WSL2. The Windows run shows
      `windows_captured_stdout` compiled with `terminal-tests` and run
      (spec criterion 1; relies on S3).

### Checkpoint

- [ ] All ten packages are migrated and committed. Every spec §3 hazard has a
      disposition backed by evidence.
- [ ] `check-metadata` passes across all ten manifests together.

## Phase 6 — Documentation, Acceptance, and Closeout

### Wave 1 — Cross-cutting updates (parallel)

- [ ] **Active-doc sweep** — re-run the consumer sweep with `--after`. Every
      remaining `--test <old>` hit for the ten packages is historical and
      annotated with its record date in
      `baseline/test-selector-consumers-after.md`.
- [ ] **Skill drift** — update `rust-testing` only if this wave changed a
      documented rule, such as R3's naming refinement or R4's target-set
      rule. Update the `os` skill with any new Windows or WSL2 fact found in
      the remote legs, in the same change. Update the area skills (`sniff`,
      `claudine`, `biscuit-file`, `tree-hugger`, `biscuit-tui`,
      `biscuit-terminal`, `darkmatter`) where they name per-file test
      binaries.
- [ ] **Dependencies** — the `docs/dependencies.md` entries from R7 are
      present and accurate.

### Wave 2 — Acceptance (sequential)

- [ ] **Workspace metadata** — `cargo metadata` shows no package with ten or
      more integration-test targets. Record the new workspace total and the
      largest remaining package (new criterion 5, R11).
- [ ] **Final sweep** — run for all eight areas: `just test`,
      `just test-l2`, `just lint`, `just check-tier-coverage`, and
      `just check-canonical`. Failures must match
      `baseline/pre-existing.md` only (new criterion 6).
- [ ] **`acceptance.md`** — walk first-feature criteria 1–8 and 10–12 per
      package, plus new criteria 4–6, linking each to committed evidence.
      Criterion 10 (CI observations) is recorded as pending until an ordinary
      producer run selects the packages. `ci-observations.md` holds the
      harvest procedure, and no run is triggered for it.
- [ ] **Status** — set the spec to `implemented` with `implemented_by`, and
      leave the tree at "implementation complete, ready for review". Do not
      move the spec to `_completed`.

### Checkpoint

- [ ] Every acceptance criterion is linked to evidence, and every pending item
      names the evidence it is missing.
- [ ] All commits are signed and verified. The handoff note in
      `implementation-log.md` is written for the reviewer.

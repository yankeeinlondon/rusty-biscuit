---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
plan: features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
implemented_by: claude/opus
started_phase: 1
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
packages: []
---

# Implementation Log for 2026-09-22-consolidated-test-binaries-wave-2 (6 phases)

## Phase 1

Phase 1 (rulings, spikes, baselines) ran at `208051f75`. No production source
changed. The spec `status` moved `draft-spec → planned`. Nothing was
committed, per the phase instructions.

### What was produced

| Plan task | Output | Result |
|---|---|---|
| Record rulings | `rulings.md` | R1–R13 decided, plus new R14–R19 from Phase 1 evidence |
| S1 | `spikes/s1-feature-sets.md` | Census matches the plan: 236 workspace targets, 136 in scope. `dmls` needs a 4th feature set, `(terminal-tests)`. `claudine` is `()`. |
| S2 | `spikes/s2-hazards.md`, `s2-scan.py`, `s2-scan-output.txt` | 144 files scanned with `consolidation.py`'s detectors. Zero crate-global constructs or crate-only inner attributes. 95 path repairs, 2 self-exec identities, 3 `crate::` false positives, a proptest relocation hazard, and a cross-package guard. Done by a subagent; I verified the proptest and guard claims independently. |
| S3 | `spikes/s3-cross-check.md` | `cross-check` ships the working tree (not `HEAD`) and builds the CI feature union, so `windows_captured_stdout` is compiled. Per-test PASS lines appear live only. |
| S4 | `spikes/s4-deps.md` | No cycle. Five packages need an unconditional `test-toolkit` dev-dependency, not two. The added crates are all already in `Cargo.lock`. |
| Pre-existing state | `baseline/pre-existing.md`, `pre-existing.sh`, `pre-existing-logs/` | 8 areas × `test`, `test-l2`, `lint`, `check-tier-coverage`, plus `check-canonical`: all green except 2 environment-induced `claudine-cli` failures. 0 stranded tests. |
| Skip baseline | `baseline/skip-baseline.md` | `ci-baseline.toml` has no entries at all |
| Test-input probes | `baseline/test-inputs.md`, `test-input-probe.py`, `test-input-listings.py`, `*-before.json` | 9 probes, 1 per package (`biscuit-tui-cli` has 0 references). Identities are confirmed derived from the `mod` walk. |
| Consumer sweep | `baseline/test-selector-consumers.md`, `consumer-sweep.py` | 27 active selector hits and 41 file-path references. One is load-bearing and crosses packages (R17). Done by a subagent. |

### Findings that changed the plan (written into the plan's table and F9–F14)

- **F9 / R14:** all four `sniff-cli` `level2_*` files are
  `#![cfg(feature = "test-fixtures")]`, including
  `level2_recent_commits_rendering`. So spec hazard 3's "no features"
  contract is empty without the feature. `sniff-cli` becomes 2 targets, and
  the in-scope total is 18. The author may restore the split before Phase 5.
- **F10 / R7:** `schematic-gen`, `biscuit-terminal-cli`, and `claudine-gen`
  have `test-toolkit` only as an optional dependency, so their feature-less
  `l1` layout gate needs an extra unconditional dev-dependency.
- **F11 / R18:** proptest 1.11 `SourceParallel` resolves to
  `tests/proptest-regressions/<stem>.txt` once `tests/<target>/main.rs`
  exists. I verified this from `proptest-1.11.0/src/test_runner/failure_persistence/file.rs:77-81`
  and `:336-367`. **This is a wave-1 defect:** darkmatter's 5 seeds under
  `darkmatter/lib/tests/l1/*.proptest-regressions` are no longer replayed,
  and `darkmatter/lib/tests/proptest-regressions/` does not exist. Filed as
  `darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md`.
- **F12 / R17:** `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053`
  reads `windows_captured_stdout.rs` by path, and `:7067` asserts its inner
  `#![cfg(windows)]`. R4's disposition is therefore an outer cfg on the `mod`
  line **and** the inner attribute kept in the file.
- **F13 / R15:** `test_inputs.py`'s `NON_L1_BINARIES` never narrows into a
  `level2*` binary. That already applies to F3's old targets, and
  `biscuit-tui-cli` has no references, so nothing new is hidden. No tool
  change.
- **F14 / R13, R16:** `cross-check` ships the working tree. Remote legs must
  finish before the next package's structural edit starts. Windows evidence
  is `tee`d PASS lines, and `--features` must never be passed.

### Pre-existing failures and gaps (not attributable to the migration)

- `claudine-cli::l1 shipped_prompt_route_drift::{fixture_body_matches_the_shipped_body, shipped_implement_prompts_have_not_drifted_from_their_fixture}`
  fail because this worktree carries the author's uncommitted
  `prompts/_implement/*.md` edits. `claudine-cli` is outside the ten. With
  `--no-fail-fast`, 7316 passed and 2 failed.
- F3's 49 L1-tier tests in `biscuit-terminal-cli` run in no local recipe;
  they run only in CI's L1 cell. F5's `windows_captured_stdout` runs only on
  Windows with `terminal-tests`. Neither counts as "stranded" under
  `check-tier-coverage`. Both must be preserved exactly.

### Other things noticed (out of scope, not acted on)

- `claudine/cli/claudine/cli/tests/snapshots/` is a tracked, doubly nested
  directory holding two `wrap_commands__*.snap` files. It looks like a stray
  insta write. It is outside the ten packages.
- The `os` skill's `build-hosts.md` said `cross-check` routes feature flags
  into the archive build. The code does the opposite: any build flag switches
  every host to native mode. **Drift was detected, the code was taken as
  correct, and the skill was corrected.**

### Gates run in this phase

- `just test`, `just test-l2`, `just lint`, and `just check-tier-coverage <area>`
  for all eight areas, plus `just check-canonical` over the eight. Results are
  in `baseline/pre-existing.md`. This phase changed no Rust, justfile, or
  manifest, so these runs are both the baseline and this phase's own gate.
- `python3 -m py_compile` over the five new evidence scripts.
- The three frontmatters (spec, plan, log) parse as YAML.
- There is no requirement-to-test mapping, because Phase 1 changes no
  behavior. Its outputs are evidence documents, and each claim cites file
  and line or a committed log.

## Phase 2

Phase 2 (toolkit extension) ran at `9d44e7988`. No file in the ten packages
changed. Nothing was committed, per the phase instructions. R12's signed
toolkit commit is left to the separate commit step.

### What changed in `scripts/ci/consolidation.py`

| Plan task / ruling | Change |
|---|---|
| Package tables (R1) | `PACKAGES` gained the ten packages. `PACKAGE_FEATURE_SETS` holds S1's reviewed sets, including `dmls`'s fourth set `(terminal-tests)`. `capture` refuses an unlisted package **before** any subprocess (it used to run `cargo metadata` first), and `inventory --package <unlisted>` now refuses too (exit 2). |
| `move` (R2, R17, R18) | Ports `pilot/apply-move.py`, generalized: it refuses before touching anything (missing source, existing destination, undeclared module, unresolved `mod`). It resolves every `include_str!`/`include_bytes!`/`#[path]` literal against the old location and rewrites it only when the resolution would change, which also covers nested-root children. `mod common;`, bare or `#[path]` to `tests/common/mod.rs`, becomes `use crate::common;` with its other attributes kept. Any other bare `mod x;` gains a `#[path]` to its old file (sniff's `fixtures`). The row flag `keep_inner_cfg` keeps the inner cfg (R17). Roots declare `common` only where a module uses it, and list modules in rustfmt order (byte order, verified against `rustfmt --edition 2024`). Seed files move to `proptest_regression_path` (R18). It prints the `[[test]]` entries for `Cargo.toml`. |
| `check-proptest` (R18) | New. Seed content must survive byte for byte, and every seed file must be the resolution of some test source. A file proptest never reads fails. |
| `check-metadata` | Ports `acceptance/metadata-check.py` with `--manifest` (repeatable) and `--metadata`. Output on wave 1's four manifests is identical to the original's. |
| `body-diff` | Ports `acceptance/body-diff.py` with `--manifest`, `--base-rev`, `--after-rev` (default: the working tree), and `--markdown`. It now pairs nested-root children, and counts a line that differs only by a path literal as structural. **It exits 1** on any `other` line or a missing side, because wave 2 allows structural edits only. |
| `plan`: `RULED_TARGETS` (R4, R14, R19; new finding F16) | A reviewed table places `schematic-gen`'s `terminal_capture` and `biscuit-tui-cli`'s `real_terminal_render` and `windows_captured_stdout` in `level2`, places `sniff-cli`'s `level2_recent_commits_rendering` in `level2` (`test-fixtures`), and rules `sniff`'s `fixtures` a helper. `plan` fails if an entry names no target, if a helper lists tests, if the target would compile where it never did, or if it joins a target with more features while listing tests without them. |
| `plan`/`compare`: shared `common` tests (new finding F15) | Tests inside the shared `common` module are recorded as `shared_tests` and not projected per module. `compare` folds their copies into the consolidated identity and fails if the copies disagree. |

### Self-proof (all in `selfproof/`; see its `README.md`)

- **No-op:** `capture` ran twice for the ten packages (21 feature sets).
  `compare --require-identical-digests` gave `identical`: 0 failures and 0
  notes across 487 selector cells, and the two runs were byte-identical.
  `capture-a/` is committed as the before-side for Phases 3–5, with
  `inventory.json`.
- **`plan` over `capture-a`** gives exactly the ruled table: 18 targets; the
  aliases `prose_cells`, `diagrams`, and `terminal_render` only; R10's two
  rewrites; and `sniff` with 19 modules plus the `fixtures` helper.
  `ShippedWave2PlanTests` pins this against the frozen evidence and the
  filters each capture recorded.
- **`move` dry run** of all ten packages in a throwaway worktree: 0 `other`
  body lines, 0 `check-proptest` failures, and the `biscuit-file` seed
  relocated. The 7 `check-attributes` failures are exactly the S2/R19
  dispositions. `tree-hugger` and `biscuit-file` were then built and compared
  against `capture-a`: `identical` (`dry-run.md`).
- **R18:** a scratch crate with a failing property wrote
  `tests/proptest-regressions/always_fails.txt`, exactly as predicted.
  `check-proptest` is red on wave 1's darkmatter seeds (the filed defect).
- **Red then green:** `mutation-check.py` gave 17 of 17 `OK`. Wave 1's
  `mutation-check.py` still gives 8 of 8.

### Requirement-to-test mapping (`scripts/ci/test_consolidation.py`, 86 → 96 tests)

| Behavior | Tests |
|---|---|
| Ruled feature sets; unlisted package refused (capture and inventory) | `PackageTableTests.*`; shipped: `ShippedPackageTableTests` (each wave-2 set against the real `Cargo.toml`, CI union listed) |
| proptest path resolution | `ProptestPathTests` |
| `move`: moves, path repairs (include, `#[path]`, bare `mod`, nested child), `common` rewrite, alias as module name, rustfmt order, common only where used, `keep_inner_cfg`, seed relocation, `[[test]]` entries, refusal leaves the tree untouched, CLI | `MoveTests.*` (10 tests), including `test_a_generated_root_is_already_rustfmt_clean` and `test_the_moved_tree_passes_every_after_check` |
| `check-proptest` | `CheckProptestTests.*`: the wave-1 defect shape, changed or lost seeds, correct relocation |
| `check-metadata` | `CheckMetadataTests.*`: missing target, extra target, feature mismatch, missing `autotests`, double mapping, unknown target, CLI exit codes; shipped: `ShippedWave1MetadataTests` |
| `body-diff` | `BodyDiffTests.*`: structural-only passes, body change fails with the line, missing side fails |
| `RULED_TARGETS`, `shared_tests` in `plan` | `SharedAndRuledPlanTests.*` (5 tests); shipped: `ShippedWave2PlanTests` (3 tests) |
| Shared-copy fold in `compare` | `CompareTests.test_shared_common_copies_fold_into_the_consolidated_identity`, `test_shared_copies_that_disagree_fail` |

### Gates run

- `python3 scripts/ci/test_consolidation.py`: 96 tests, OK. No shipped class
  skipped.
- `just ci-local`'s Python leg (its ten suites, run as `ci-local` runs them):
  all pass.
- Lint: the repository has no Python lint recipe, and `scripts/` is not a
  `just` area, so `just lint` (Rust areas) does not cover this surface. Run
  instead: `python3 -W error -m py_compile` and
  `uvx ruff check --select F,E9,B` on the three changed Python files, both
  clean. The base file was clean under the same rules; my three findings
  (B905 ×2, B023) were fixed.
- No Rust, manifest, or justfile changed, so area `just test`/`just lint` are
  unaffected by this phase. Phase 1's baseline stands.

### Findings (written into the plan as F15–F17)

- **F15:** `biscuit-terminal-cli`'s `tests/common/pane_geometry.rs` has 10 unit
  tests. Each of 9 `level2_*` binaries compiles a copy (90 identities). They
  are L1-tier and run in CI's L1 cell with `terminal-tests`. Phase 1 (F3, S2)
  missed them because they are not in a target file. Under one `common` per
  root they fold to 10. **Open for Phase 4:** record the fold as that
  package's disposition (recommended: it follows first-spec §3 and
  `rust-testing`'s "declared once" rule, while wave 1's per-module
  `parity_helpers` copies are labeled "legacy shape, not a pattern"), or keep
  per-module copies. The toolkit supports the fold. Keeping copies would need
  `plan` to project `common::` tests per module again.
- **F16:** without `RULED_TARGETS`, `plan` gave `schematic-gen: l1-terminal`,
  `biscuit-tui-cli: l1-terminal + real`, and
  `sniff-cli: level2 + level2-test-fixtures`, which contradict R4 and R14.
- **F17:** wave 1's `claudine-cli` row in `PACKAGE_FEATURE_SETS` is stale. Its
  CI union now includes `test-fixtures`, so `capture --package claudine-cli`
  refuses. It is outside scope and was not changed. `ShippedPackageTableTests`
  checks wave-2 rows only, and says why.
- **S2 miscount:** S2 says 12 `#[path]` sites in scope. There are 11 (its
  `sniff` row says ×7 and lists 6). All 11 are handled.
- **Still manual in Phases 3–5** (the mover does not guess these):
  - `Cargo.toml`, including removing stale `[[test]]` entries such as
    `biscuit-file`'s `fetch_integration`;
  - `dispositions` for the 7 `check-attributes` hits;
  - `keep_inner_cfg: true` on `biscuit-tui-cli`'s `windows_captured_stdout`
    row (R17);
  - the snapshot moves;
  - the `spawn_site_guard` key; and
  - the self-exec `--exact` strings.

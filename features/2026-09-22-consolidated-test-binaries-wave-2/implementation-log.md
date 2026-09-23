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
source_files_during_phase_3:
    - tree-hugger/lib/Cargo.toml
    - claudine/lib/Cargo.toml
    - sniff/lib/Cargo.toml
    - .config/nextest.toml
    - Cargo.lock
    - sniff/lib/benches/support/bench_ids.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/lib/tests/l1/boundary_lint.rs
    - claudine/lib/tests/l1/canonical_dispatch.rs
    - claudine/lib/tests/l1/deprecated_compatibility.rs
    - claudine/lib/tests/l1/diagnostic_detail_conformance.rs
    - claudine/lib/tests/l1/kimi_wire.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/model_catalog_integration.rs
    - claudine/lib/tests/l1/opencode_stderr_lifecycle.rs
    - claudine/lib/tests/l1/protocol_fixture_replay.rs
    - claudine/lib/tests/l1/semantic_fidelity.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/lib/tests/l1/test_layout.rs
    - claudine/lib/tests/l1/tts_phase1_contract.rs
    - claudine/lib/tests/l1/tts_phase5_contract.rs
    - claudine/lib/tests/l1/typed_stream_protocols.rs
    - sniff/lib/tests/l1/bench_fixtures.rs
    - sniff/lib/tests/l1/bench_ids_sync.rs
    - sniff/lib/tests/l1/bench_plans.rs
    - sniff/lib/tests/l1/benchmark_workloads.rs
    - sniff/lib/tests/l1/focused_provider.rs
    - sniff/lib/tests/l1/git_parity.rs
    - sniff/lib/tests/l1/host_capability_cache.rs
    - sniff/lib/tests/l1/integration.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/merge_conflict_prediction.rs
    - sniff/lib/tests/l1/network_primitives.rs
    - sniff/lib/tests/l1/program_installable.rs
    - sniff/lib/tests/l1/program_serialization.rs
    - sniff/lib/tests/l1/recent_commits.rs
    - sniff/lib/tests/l1/remote_observation.rs
    - sniff/lib/tests/l1/remote_providers.rs
    - sniff/lib/tests/l1/remote_resolution.rs
    - sniff/lib/tests/l1/test_layout.rs
    - sniff/lib/tests/l1/uv_with_install_plan.rs
    - sniff/lib/tests/l1/windows_app_paths_orphan.rs
    - sniff/lib/tests/l1/windows_find_program_priority.rs
    - tree-hugger/lib/tests/l1/adapter_tests.rs
    - tree-hugger/lib/tests/l1/cache_tests.rs
    - tree-hugger/lib/tests/l1/corpus_tests.rs
    - tree-hugger/lib/tests/l1/lint_diagnostics.rs
    - tree-hugger/lib/tests/l1/main.rs
    - tree-hugger/lib/tests/l1/phase1_diagnostics.rs
    - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
    - tree-hugger/lib/tests/l1/query_compile.rs
    - tree-hugger/lib/tests/l1/resolver_tests.rs
    - tree-hugger/lib/tests/l1/test_layout.rs
    - tree-hugger/lib/tests/l1/tree_file.rs
    - tree-hugger/lib/tests/l1/tree_package.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/measure.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/layout-guard.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/test-input-check.py
docs_updated_during_phase_3:
    - tree-hugger/lib/README.md
    - docs/comment-quality.md
    - docs/dependencies.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_3:
    - features/2026-09-22-consolidated-test-binaries-wave-2/measurements.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger/
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine/
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after-edit.log
skills_files_updated_during_phase_3:
    - .claude/skills/os/build-hosts.md
    - .claude/skills/sniff/network.md
    - .claude/skills/tree-hugger/query-system.md
packages:
    - tree-hugger
    - claudine
    - sniff
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

## Phase 3

Phase 3 (single-contract packages) ran at `c02e691d9` on the macOS dev host.
It migrated `tree-hugger` (10 → 1), `claudine` (15 → 1), and `sniff` (20 → 1)
into one `l1` binary each. Nothing was committed, per the phase instructions.
R12's commit series (manifest and evidence, structural move, area docs) is
left to the separate commit step, one package at a time.

### Before-side

- Fresh macOS captures (SPP 1) are byte-identical to `selfproof/capture-a`
  (`compare --require-identical-digests`: identical for all three). Nothing in
  the three crates, their justfiles, or `.config/nextest.toml` changed between
  `9d44e7988` and `c02e691d9`.
- Linux before-captures came from a private `--shared` scratch clone on
  `build-linux` (`os` skill procedure): the standing clone at `7685d1ac2`,
  plus a bundle `7685d1ac2..HEAD`, checked out at `c02e691d9`, with
  `RUSTC_WRAPPER="" KACHE_AUTO=0`. One base serves all three packages because
  they share it. The clone, bundle, and patch were deleted afterward.
- Each `plan` output equals the one from `capture-a` (checked for `sniff` with
  a JSON diff). No alias was emitted: the two `*_spike` targets carry no tier
  marker, as projected.

### Per package

| Step | `tree-hugger` | `claudine` | `sniff` |
|---|---|---|---|
| `move` | 10 modules, 41 `include_str!` repairs in 2 files | 15 modules, 2 repairs (`#[path]` into `darkmatter/features/…/spike/model.rs`, one fixture literal) | 19 modules, 6 bench-support `#[path]` repairs, `integration`'s `mod fixtures;` → `#[path = "../fixtures.rs"]` (R19) |
| `Cargo.toml` | `autotests = false`, `[[test]] l1`, **new** unconditional `test-toolkit` dev-dependency (R7) | `autotests = false`, `[[test]] l1` (`test-toolkit` was already a dev-dependency) | `autotests = false`, `[[test]] l1` (already had `test-toolkit`) |
| Module `cfg`s | none | none | `feature = "remote"` ×2, `feature = "network"`, `target_os = "windows"` ×2, all carried onto the `mod` lines |
| Layout gate | `tests/l1/test_layout.rs`, ≥ 12 files | `tests/l1/test_layout.rs`, ≥ 17 files | `tests/l1/test_layout.rs`, ≥ 22 files, names `tests/fixtures.rs` and both Windows modules |
| Red/green | `layout-guard.md`: red (stray), red (orphan), green | same | same |
| `check-attributes` / `check-proptest` / `check-metadata` | 0 failures (2 reviewed path-sensitive files, both compile) / clean / PASS | 0 / clean / PASS | 0 / clean / PASS |
| `body-diff` (other lines) | 0 (82 structural) | 0 (4 structural) | 0 (23 structural, 2 comment) |
| `compare` darwin / linux / darwin-linux | identical / identical / identical | identical / identical / identical | identical / identical / identical (all 3 feature sets; platform-absent evaluated) |
| Test-input probe | 89 → 89 (`binary_id(tree-hugger::l1) & test(/^tree_file::/)`) | 7 + 1 → 7 + 1 | 3 → 3 |
| Overrides | none | `claudine-l1` group (`override-ci-1`): identical 4342-test identity set | R10 rewrite in `.config/nextest.toml`: `override-rewrite.txt` shows the new filter matches exactly 1 test and the old spelling 0; `sniff-windows-l1` (`override-ci-2`) identical |
| Windows (`cross-check`) | 484/484 | 4300/4302: 2 lib unit-test failures (below); every `claudine::l1` test passed | 1937/1937, **both Windows-only modules named PASS** |
| WSL2 (`cross-check`) | 484/484 | 4343/4343 | 1946/1946 |
| Linux L1 (scratch clone) | 484/484 | 4339/4343: 4 lib unit-test failures, proven pre-existing (below) | 1946/1946 (`remote`) |

The compare runs report `override-ci-3 was rewritten` as a note wherever the
after capture used the new `.config/nextest.toml`. That is R10's rewrite, and
the identity it selects is unchanged.

### Hazards and dispositions

- **`claudine` `boundary_lint`**: not a path-keyed test guard. It reads
  production files by fixed paths under `claudine/lib/src`, `claudine/cli/src`,
  and `darkmatter/lib/src`, and never scans `tests/`, so the move changes its
  scan set by nothing. There is no `guard-scans-after.md` for any Phase 3
  package, because none has a path guard over test files.
- **`claudine-cli`'s `test_placement.rs`** walks only `claudine/cli/tests` for
  the layout rule, and it runs only when `claudine-cli` is tested. So it was
  not extended: `claudine` got its own gate in its own `l1`. Otherwise a
  `claudine`-only change would never run it.
- **`sniff` Windows-only files**: `check-attributes` confirmed both
  `#![cfg(target_os = "windows")]` moved onto the declarations. The Windows
  leg names both tests as run (`sniff/cross-check-windows.txt`).
- **`sniff` `bench_*`**: ordinary test targets. They moved like the rest.
- **`sniff` `real_` tests** stay inside L1 modules (R6). The area's
  `check-tier-coverage` reports 0 stranded.

### Suites (macOS; compared with `baseline/pre-existing.md`)

| Area | `just test` | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|
| `tree-hugger` | ✅ 586 passed (585 + gate) | ✅ 3 | ✅ | ✅ 0 stranded | ✅ |
| `claudine` | 7317 passed, 2 failed: the **same two** pre-existing `claudine-cli::l1 shipped_prompt_route_drift` failures (baseline 7316 + 2) | ✅ 237 + 3 | ✅ | ✅ 0 stranded | ✅ |
| `sniff` | ✅ 2825 passed (2824 + gate) | ✅ 6 | ✅ | ✅ 0 stranded | ✅ |

All suites ran with `INSTA_UPDATE=no`. No `.snap.new` exists. Logs are
gzipped under each package's `suites/`.

### Failures not attributable to the migration

- **`claudine` on `build-linux`, 4 lib unit tests**
  (`composition::sequence::task::tests::group_framing::{a_members_body_output_lands_on_the_data_channel_not_the_status_one, a_serial_group_uses_the_invisible_bar_at_the_same_left_edge, every_member_task_opens_and_closes_exactly_one_stream, every_body_line_carries_its_own_tasks_bar}`).
  **Proven pre-existing**: with the patch stashed in the scratch clone (0
  dirty paths, `c02e691d9`), the same four fail (`claudine/linux-l1-summary.txt`).
  They pass on macOS and WSL2, so they look specific to that host's
  non-interactive SSH session.
- **`claudine` on native Windows, 2 lib unit tests**
  (`composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`:
  expected `B:\…\src`, got `B:/…/src`;
  `composition::sequence::task::tests::side_effect_tasks::a_failed_nested_mapping_set_projects_one_path_and_commits_nothing`:
  an `unknown_root` error). **Not proven on the base.** Both live in the
  `claudine` lib-test binary, which is compiled only from `claudine/lib/src`
  and its dependencies. `git status claudine/lib/src` is empty, and the test
  targets the move changes are not part of that binary, so the move cannot
  reach them. A base run from a clean worktree
  (`claudine/cross-check-windows-base.txt`) first failed with `os error 1455`
  (paging file exhausted while the WSL guest on the same machine was
  building). The retry then failed on the poisoned `target\` (`E0463`). The
  lesson is now in the `os` skill (`build-hosts.md`, Remote-process hygiene).
  A later leg pruned the stale clone. **For the author:** both look like real
  Windows path-spelling defects in `claudine`. They are outside this feature's
  scope and are not filed.

### Docs (SPP 9)

- `tree-hugger/lib/README.md`: test paths now `tests/l1/…`, plus one sentence
  saying a new file must be declared in `tests/l1/main.rs`.
- `.claude/skills/tree-hugger/query-system.md`, `.claude/skills/sniff/network.md`:
  moved test paths.
- `docs/comment-quality.md:293`: `claudine/lib/tests/l1/canonical_dispatch.rs`.
- `sniff/lib/tests/l1/remote_providers.rs:11`: `--test remote_providers` →
  `--test l1 remote_providers::`. It selects the same 70 tests.
- `sniff/lib/benches/support/bench_ids.rs:7,11,74`: comment paths.
- `docs/dependencies.md`: `tree-hugger`'s dev-only `test-toolkit` (R7).
  `Cargo.lock` gains only that edge, and no new crate. `tree-hugger` has no
  area dependency doc, and none was created (R7).
- Left as history: `playa/docs/{plans,specs}/2026-04-29-*` (dated plans),
  `tree-hugger/reviews/…`, `sniff/docs/cli/repo_*.md` (sample CLI output of
  past commits), and `fixes/2026-09-22-test-input-blind-spot/spec.md:335`
  (another active spec's measurement record, which names
  `claudine::boundary_lint`). Module-name prose (`bench_ids_sync`,
  `protocol_fixture_replay`) is still correct.

### Measurements

See `measurements.md`. Test executables went from 45 to 3, and from
1,488.8 MB to 362.6 MB. The warm edit changed by +0.21 s, +0.38 s, and
+0.56 s, so R8's trigger was never reached.

### Deviations from the SPP, and why

- **No commits** (phase instructions). So R13's "remote legs against a
  committed revision" could not hold literally. Each `cross-check` shipped the
  working tree at its start. Every leg's banner names its synthetic revision.
  A leg's package does not depend on the other two packages' test files:
  `claudine` depends only on `sniff`'s library, and test targets are not part
  of a library build. So a later package's edits could not change an earlier
  leg's evidence. No structural edit started while a leg was being bundled.
- **One Linux before/after pair for all three packages**, taken from one
  scratch clone. The after patch covered `tree-hugger/lib`, `claudine/lib`,
  `sniff/lib`, and `.config/nextest.toml`. Cargo regenerated the same
  `Cargo.lock` edge there.
- **`cross-check` output was redirected, not `tee`d** for `claudine` and
  `sniff`. The files are the same evidence.

### Requirement-to-test mapping

| Changed behavior | Evidence / test |
|---|---|
| Every former test keeps its identity, tier, feature, and platform reachability | `compare` four-way, darwin/linux/darwin-linux, every selector and feature set (`<pkg>/comparison-*.md`) |
| A `tests/` file no root declares fails the build gate | `<pkg>::l1 test_layout::every_test_source_is_compiled_by_a_declared_target`, shown red on a stray root and an undeclared module, then green (`<pkg>/layout-guard.md`) |
| Windows-only modules still compile and run on Windows | `sniff/cross-check-windows.txt` PASS lines for `windows_app_paths_orphan::orphaned_hkcu_entry_is_filtered` and `windows_find_program_priority::path_wins_over_fallbacks_for_cmd` |
| CI's `threads-required` override still hits its test | `sniff/override-rewrite.txt` (1 match; old spelling 0) plus `compare`'s `override-ci-3` row |
| CI's test-input narrowing still selects the same tests | `<pkg>/test-inputs.md` (the planner's own index, re-run) |
| Manifests match Cargo | `check-metadata` PASS (`<pkg>/metadata-check.txt`) |
| Only structural edits | `body-diff` 0 `other` lines (`<pkg>/body-diff.md`) |

### New files (evidence tooling)

- `measure/measure.sh`: count, bytes, and warm edit (R8).
- `measure/layout-guard.sh`: red/green proof.
- `measure/test-input-check.py`: the after probe with the manifest mapping.

All three are reusable unchanged in Phases 4–5. `shellcheck` and
`ruff --select F,E9,B` are clean.

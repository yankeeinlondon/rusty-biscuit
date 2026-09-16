---
title: CI redundancies — one scheduling model, one owner per suite, one report
created: 2026-09-14
phase: 11
total_phases: 11
agent: claude/opus
yolo: true
spec: fixes/_complete/2026-09-13-cicd-redundancies/spec.md
depends_on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_1:
  - fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
skills_files_updated_during_phase_1: []
packages:
  - repo-deps
  - test-toolkit
source_files_during_phase_2:
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - scripts/ci-rollup-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_2:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_2:
  - fixes/_complete/2026-09-13-cicd-redundancies/oracles.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - Cargo.toml
  - Cargo.lock
  - scripts/Cargo.toml
  - scripts/Cargo.lock
  - scripts/drift.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_resolved_plan.py
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/ci.yml
  - .github/workflows/_area-ci.yml
  - justfile
  - just/devops.just
  - just/ci-local.just
  - biscuit-file/justfile
  - homelab/justfile
  - queue/justfile
  - release-plz.toml
docs_updated_during_phase_3:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
  - fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/os/windows.md
packages_during_phase_3:
  - repo-deps
  - test-toolkit
source_files_during_phase_4:
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_local_evidence.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci-plan-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/ci.yml
docs_updated_during_phase_4:
  - .github/ci/README.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
  - fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
  - fixes/_complete/2026-09-13-cicd-redundancies/oracles.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/rust-devops/ci-cd.md
packages_during_phase_4:
  - repo-deps
  - test-toolkit
source_files_during_phase_5:
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/suite_runner.py
  - scripts/ci/companion_suites.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/Cargo.toml
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/_package-ci.yml
  - .github/workflows/_area-ci.yml
  - .github/ci/schemas/contract.json
  - homelab/justfile
docs_updated_during_phase_5:
  - .github/ci/README.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
  - fixes/_complete/2026-09-13-cicd-redundancies/oracles.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - repo-deps
  - test-toolkit
  - homelab-server
source_files_during_phase_6:
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_local_evidence.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_publish_gaps.py
  - scripts/ci-rollup.rs
  - scripts/ci-plan-tests.rs
  - .github/ci/schemas/contract.json
  - .githooks/tests/fixtures/affected_scope_stub.py
  - .githooks/tests/fixtures/plan-macos-executing.json
  - .githooks/tests/fixtures/plan-macos-two-packages.json
  - .githooks/tests/fixtures/plan-wsl-absent.json
  - .githooks/tests/fixtures/plan-wsl-executing.json
  - .githooks/tests/fixtures/plan-wsl-reused.json
docs_updated_during_phase_6:
  - .github/ci/schemas/README.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - repo-deps
source_files_during_phase_7:
  - biscuit-tui/cli/tests/windows_captured_stdout.rs
  - biscuit-tui/justfile
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_7:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
  - fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
  - biscuit-tui-cli
  - test-toolkit
source_files_during_phase_8:
  - .github/workflows/ci.yml
  - .github/workflows/ci-infra-retry.yml
  - .github/workflows/biscuit-tui-windows-captured-stdout.yml
  - scripts/ci/affected_scope.py
  - scripts/ci/runner_loss.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_runner_loss.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_8:
  - .github/ci/README.md
  - docs/topics/ci-cd.md
  - docs/dependencies.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/rust-testing/test-audit-tooling.md
packages_during_phase_8:
  - repo-deps
  - test-toolkit
source_files_during_phase_9:
  - .github/workflows/ci.yml
  - just/ci-local.just
  - scripts/ci-change-inventory.rs
  - scripts/ci-plan.rs
  - scripts/ci-plan-tests.rs
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_ci_local.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_9:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9: []
packages_during_phase_9:
  - repo-deps
  - test-toolkit
source_files_during_phase_10:
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - tools/test-audit/justfile
docs_updated_during_phase_10:
  - .github/ci/README.md
  - docs/topics/ci-cd.md
  - docs/dependencies.md
  - tools/test-audit/README.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_10: []
skills_files_updated_during_phase_10:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/rust-testing/SKILL.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
packages_during_phase_10:
  - test-toolkit
source_files_during_phase_11: []
docs_updated_during_phase_11:
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
docs_created_during_phase_11:
  - fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md
skills_files_updated_during_phase_11: []
packages_during_phase_11:
  - repo-deps
  - test-toolkit
  - biscuit-tui-cli
source_code:
  - Cargo.toml
  - Cargo.lock
  - scripts/Cargo.toml
  - scripts/drift.rs
  - scripts/ci-plan.rs
  - scripts/ci-plan-tests.rs
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci-change-inventory.rs
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/suite_runner.py
  - scripts/ci/companion_suites.py
  - scripts/ci/runner_loss.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_local_evidence.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_publish_gaps.py
  - scripts/ci/test_runner_loss.py
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - tools/test-audit/justfile
  - biscuit-tui/cli/tests/windows_captured_stdout.rs
  - biscuit-tui/justfile
  - .github/workflows/ci.yml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/ci-infra-retry.yml
  - .github/ci/schemas/contract.json
  - .githooks/tests/fixtures/affected_scope_stub.py
  - .githooks/tests/fixtures/plan-macos-executing.json
  - .githooks/tests/fixtures/plan-macos-two-packages.json
  - .githooks/tests/fixtures/plan-wsl-absent.json
  - .githooks/tests/fixtures/plan-wsl-executing.json
  - .githooks/tests/fixtures/plan-wsl-reused.json
  - justfile
  - just/devops.just
  - just/ci-local.just
  - biscuit-file/justfile
  - homelab/justfile
  - queue/justfile
  - release-plz.toml
documentation:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - docs/topics/ci-cd.md
  - docs/dependencies.md
  - tools/test-audit/README.md
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/rust-testing/SKILL.md
  - .claude/skills/rust-testing/test-audit-tooling.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
  - fixes/_complete/2026-09-13-cicd-redundancies/plan.md
  - fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
  - fixes/_complete/2026-09-13-cicd-redundancies/oracles.md
  - fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md
---

# Implementation Plan — CI Redundancies

## Summary of Work and Definition of Success

### What this change actually is

The specification is not four unrelated fixes; it is one invariant applied in
four places: **every unit of verification work is selected by the resolved
plan, owned by exactly one `{package, environment, gate}` cell, and reported
once.** Everything below follows from enforcing that.

Grounded against the tree at `fix/cicd-improvements`, the work decomposes into
six mechanical tracks:

1. **Package-model track.** `scripts/Cargo.toml` (package `repo-deps`) has a
   nested `[workspace]` and its own `scripts/Cargo.lock`, which is the only
   reason its suites cannot be scheduled. Removing both and adding `scripts` to
   the root `members` list gives it an identity. Its area derives to `root`
   (`package_area` returns `ROOT_AREA` when the manifest directory's parent is
   `.`), which is an already-exercised path — `renderable` is a `root`-area
   gating package today. `tools/test-toolkit` needs only its
   `promotion-pending` exclusion record deleted, because `tools/justfile`
   already defines the canonical 12-recipe set its record was blocked on.

2. **Selection track.** `CI_TOOLING_PREFIXES` / `CI_TOOLING_PATHS` in
   `scripts/ci/affected_scope.py` currently fold six prefixes into one boolean
   `flags.ci_tooling`. That boolean is replaced by a declared, tested
   path-to-owner table that selects `repo-deps` or `test-toolkit` as ordinary
   packages. No new scheduler, no full-workspace escalation.

3. **Companion track.** `[package.metadata.ci.tests].companion-suites` already
   exists and is already parsed into the plan's `companion_suites` field, but
   both `_package-ci.yml` and `scripts/ci-rollup.rs` hard-code the single name
   `homelab-frontend` and record exactly one `companion` string per producer
   status. This generalizes to a closed registry of named suites with one
   owner, one canonical recipe, one environment, and one machine-readable
   outcome each (counts + command duration), with per-suite outcomes in
   producer status.

4. **Windows track.** `biscuit-tui-cli` *already* declares
   `features = ["terminal-tests"]` under `[package.metadata.ci.tests]`, so its
   CI L1 cell already compiles the `windows_captured_stdout` test target. Only
   three things stand between the test and ordinary L1 discovery: the
   `#[ignore]` attribute, the two fixed `thread::sleep` calls, and the
   dedicated workflow/recipe/job that exist to invoke it. This is the
   lowest-risk track and can run fully concurrent with tracks 1–3 and 5.

5. **Inventory + reporting track.** The resolved plan carries only
   `change_class`; it gains a categorized, sorted, repository-relative change
   inventory. `RESOLVED_PLAN_SCHEMA_VERSION` bumps 2 → 3,
   `.github/ci/schemas/contract.json` regenerates, and older scope receipts miss
   once with the existing `scope-schema` reason. Both consumers —
   `scripts/ci-plan.rs` (terminal, `TerminalRenderable`) and the new
   `ci-reporting` job (GitHub Markdown) — read that one field.

6. **Workflow-surgery track.** `preflight` sheds its nine Python suite steps and
   becomes prerequisites only; `ci-tooling` and `biscuit-tui-captured-stdout`
   are deleted along with `biscuit-tui-windows-captured-stdout.yml`; `ci-gate`
   loses two `needs` entries and nothing else; `summary` becomes `ci-reporting`;
   and the documentation-class preflight matrix becomes empty so unscheduled
   matrices resolve immediately after `scope`.

### What success looks like

The change is complete when **all sixteen acceptance criteria in the
specification hold and are demonstrated by a named, re-runnable artifact**, not
by inspection. Concretely:

- **Structural.** `grep -c 'scripts/ci/test_' .github/workflows/ci.yml` returns
  `0`; `ci.yml` defines the jobs `validation`, `scope`, `preflight`, `area-ci`,
  `ci-gate`, `ci-reporting` and no others; `scripts/Cargo.lock`,
  `biscuit-tui-windows-captured-stdout.yml`, the `test-windows-captured-stdout`
  recipe, and the `promotion-pending` record no longer exist.
- **Selection.** `python3 scripts/ci/affected_scope.py scripts/ci/schema.py`
  selects `repo-deps` and nothing else; `... tools/test-audit/package.json`
  selects `test-toolkit` and nothing else; `... darkmatter/README.md` selects
  neither and reports a documentation inventory naming that file.
- **Suites.** Every suite in the specification's ownership table appears exactly
  once in the registry, resolves to one owner, and a contract test fails if a
  suite is unknown, unowned, doubly-owned, recipe-less, or silently absent from
  a producer status.
- **Windows.** A native `windows-latest` run lists
  `captured_stdout_receives_only_value_no_tui_bytes` in the
  `biscuit-tui-cli/windows-latest/L1` JUnit, with `F2 precondition HELD` in the
  log, and a deliberately broken precondition fails that cell.
- **Reporting.** On one hosted run: `ci-reporting` renders the change
  inventory, direct and reverse dependency sets, per-environment counts and
  durations, Linux lint duration, per-suite companion results, and literal
  `ci` / `local` / `prior-local` origins; unavailable measurements render as
  `not recorded` with a reason. A forced `ci-reporting` failure does not block
  `ci-gate`; a forced `repo-deps` suite failure does.
- **Gate.** `ci-gate` remains the `protect-your-bacon` required context, folds
  only `needs.*.result`, and still accepts exactly `success` and `skipped`.
- **Regression oracles.** Every pending fixture added in Phase 2 has been
  promoted (its `@pending` decorator deleted) and passes.

### Explicit non-goals

No second scope calculator, no area-keyed store, no new persistent report
store, no `cicd` origin literal, no change to required environments or governed
capability gaps, no weakening of the Windows console-precondition assertion,
and no dependency version bumped merely to promote `repo-deps`.

---

## Phase 1 — Rulings, Spikes, and Current-State Baseline

Nothing is edited in this phase except the ruling record. Its output is a set
of decisions the remaining ten phases may cite without re-deriving.

### Necessary Rules

Each ruling below is a question the specification leaves under-determined
against the actual tree. Record the answer in
`fixes/_complete/2026-09-13-cicd-redundancies/rulings.md` with the evidence that settled
it; a later phase that contradicts a ruling must amend the record, not work
around it.

- [x] **R1 — `repo-deps` area and canonical recipes.**
    - `package_area()` maps manifest directory `scripts` → parent `.` →
      `ROOT_AREA` (`"root"`). `renderable` already proves the `root` area
      schedules.
    - **Rule to confirm:** `repo-deps` needs *no* `scripts/justfile` and must
      *not* be added to the root justfile's `areas` variable, because
      `_package-ci.yml` invokes package-scoped `just _test <pkg>` /
      `just _lint <pkg>` from the repository root, and `check-canonical`
      iterates area justfiles only.
    - Blocks Phase 3.

- [x] **R2 — `repo-deps` feature set in its L1 cell.**
    - The manifest declares `default = ["local-tools"]`, which links
      `biscuit-terminal`, `sniff`, `cargo_metadata`, and `ctrlc`. The
      specification's ownership table says the L1 suite covers *both*
      `ci-rollup` and `ci-plan` tests, which requires default features.
    - **Rule to confirm:** the L1 cell runs with default features (no
      `[package.metadata.ci.tests].features` entry), **and** the existing
      `--no-default-features --bin ci-rollup` build used by the coverage
      audit's always-runs path is left byte-unchanged.
    - Record the measured cost of the resulting three-OS build (Spike S1).

- [x] **R3 — required environments for the two tooling packages.**
    - Implementation Boundaries forbid changing required environments, but
      these two packages have never had cells.
    - **Rule to decide:** whether `repo-deps` and `test-toolkit` take the
      default required-environment set from `.github/ci/environments.json`
      (ubuntu + macOS + windows + wsl2) or a narrower declared set. Companion
      suites are separately pinned to `ubuntu-latest` (R7), so this ruling
      governs the Rust L1/lint/check cells only.
    - State the reason in the ruling; a narrower set must be expressed through
      the existing declared-environment mechanism, never a new one.

- [x] **R4 — publication and release-plz.**
    - **Rule to confirm:** `repo-deps` keeps `publish = false` and joining the
      root workspace does not enrol it in `release-plz.yml` or the maintenance
      audit. Verify by inspecting `release-plz.toml` / `release-plz.yml`
      member selection.

- [x] **R5 — `scripts/target` and `--manifest-path` call sites.**
    - Root-workspace membership moves build output from `scripts/target/` to
      the root `target/`. Known affected call sites: the
      `BISCUIT_CI_PLAN_BIN:-scripts/target/release/ci-plan` default in
      `just/ci-local.just`, every `--manifest-path scripts/Cargo.toml`
      invocation in `.github/workflows/`, `just/`, and `scripts/*.sh`.
    - **Rule:** enumerate the complete call-site list in the ruling record
      before Phase 3 edits anything; `-p repo-deps` replaces
      `--manifest-path scripts/Cargo.toml` wherever a package selector
      suffices.

- [x] **R6 — Windows test tier and process model.**
    - The test calls `AllocConsole` and rewires **process-wide** std handles
      via `SetStdHandle`. Nextest runs one test per process, which is what
      makes process-wide rewiring safe inside a parallel suite; `cargo test`
      would not be.
    - **Rule to confirm:** tier is L1 (per spec D4); safety rests on nextest's
      process-per-test model; the `_tier_filter L1` filterset
      (`!(test(/(^|::)level2_/) + level3_ + browser_ + real_ + slow_)`) selects
      `captured_stdout_receives_only_value_no_tui_bytes` once `#[ignore]` is
      removed, and selects nothing from `real_terminal_render` or
      `level3_chord_select` (whose tests are `level2_`/`level3_`-prefixed).
    - Verified by Spike S2 before Phase 7 edits the test.

- [x] **R7 — companion-suite environment and reuse blast radius.**
    - Spec §2: companions stay CI-origin on their declared environment, and
      their presence "must not make unrelated L1 cells non-reusable."
    - **Rule to decide:** the declaration shape that expresses *"this suite
      runs on `ubuntu-latest` only"*. Preferred: each registry entry carries an
      explicit `environment`, and only the cell matching that environment is
      marked non-reusable. The alternative (package-wide non-reuse) is
      explicitly rejected by the specification.
    - Blocks Phase 5.

- [x] **R8 — `change_class` is retained, not replaced.**
    - `classify_preflight()` returns `change_class` and it drives preflight
      breadth. The inventory is **additive**.
    - **Rule to confirm:** `change_class` stays in the plan and the legacy
      scope projection; the inventory is a new sibling field.

- [x] **R9 — which schema counters bump.**
    - **Rule to confirm:** `RESOLVED_PLAN_SCHEMA_VERSION` 2 → 3;
      `SCOPE_RECEIPT_SCHEMA_VERSION` stays 1 (its embedded
      `plan_schema_version` check is what produces the one-time `scope-schema`
      rejection); `RECEIPT_SCHEMA_VERSION` and
      `LEGACY_RECEIPT_SCHEMA_VERSION` unchanged. Validation receipts stay
      reusable where their existing cell and gate-input checks still qualify.

- [x] **R10 — documentation-class preflight matrix becomes empty.**
    - `classify_preflight()` today returns `["ubuntu-latest"]` for the
      documentation class with the reason *"preflight runs on the scope host
      only"*. Spec §3 requires `preflight_os` to be **empty** so preflight
      skips.
    - **Rule to confirm:** return `[]` with a new reason string, and confirm
      `area-ci`'s `needs: [scope, preflight]` edge still resolves promptly
      because a skipped `preflight` satisfies `!cancelled()`.

- [x] **R11 — `ci-gate` dependency list after retirement.**
    - **Rule to confirm:** `needs` becomes exactly
      `[validation, scope, preflight, area-ci]`; `ci-reporting` is **not**
      added (it is advisory and carries `continue-on-error: true`); the job
      `name: ci-gate` and the `protect-your-bacon` ruleset context are
      untouched; the fold script body is unchanged apart from the two deleted
      `RESULTS` lines.

- [x] **R12 — empty-matrix guard shape.**
    - `preflight` has no `if:` and relies on an empty `fromJSON` matrix;
      `area-ci` already carries `if: has_packages == 'true'`.
    - **Rule to decide:** whether `preflight` gains an explicit scalar guard
      (e.g. `if: needs.scope.outputs.preflight_os != '[]'`) in addition to the
      empty matrix, per spec §8's "guarded by a scalar plan output **before**
      matrix expansion". Recommend yes: an explicit `if:` is what makes the
      skip deterministic and observable rather than a property of GitHub's
      empty-matrix handling. Neither job may carry a matrix-expression `name:`.

- [x] **R13 — `.github/workflows/**` selects `test-toolkit`, but `ci.yml` and
      `_package-ci.yml` remain global.**
    - `GLOBAL_PATHS_ALL_GATES` already contains `.github/workflows/ci.yml` and
      `.github/workflows/_package-ci.yml`; `.github/ci/environments.json` and
      `scripts/ci/affected_scope.py` are global too.
    - **Rule to confirm:** the new path-to-owner table is applied **in
      addition to** the existing global-path escalation, never instead of it.
      A change to `ci.yml` therefore selects every package *and*
      `test-toolkit`; a change to `.github/workflows/_area-ci.yml` selects
      `test-toolkit` only. Record the intended interaction explicitly — this is
      the single most likely source of a silently-widened or silently-narrowed
      scope.

- [x] **R14 — what "command duration" means in producer status.**
    - Spec §6: "Command duration, not runner setup/queue duration, is recorded
      in producer status; existing result-cell `duration_s` remains the
      report's normalized field."
    - **Rule to confirm:** lint duration is measured around the `just _lint`
      invocation inside `_package-ci.yml` (not the job's total elapsed time),
      each companion suite records its own command duration and counts, and the
      report's normalized field stays `duration_s`.

### Spikes

- [x] **S1 — root-workspace migration dry run** (de-risks Phase 3)
    - On a throwaway branch: delete `scripts/Cargo.lock`, remove the nested
      `[workspace]`, add `"scripts"` to root `members`, run
      `cargo metadata --no-deps`, `cargo nextest list -p repo-deps`, and
      `cargo nextest run -p repo-deps --no-default-features --bin ci-rollup`
      from **both** the repository root and `scripts/`.
    - Record: root `Cargo.lock` diff size, whether `rstest 0.23`,
      `quick-xml 0.38`, and `toml 1.0` unify with existing root versions or add
      duplicate entries, and the cold-build wall time of a `repo-deps` L1 cell.
    - **Kill criterion:** if the lockfile migration forces a version change to
      any unrelated crate, stop and escalate — Implementation Boundaries forbid
      bumping a dependency merely to promote `repo-deps`.

- [x] **S2 — Windows console test under Nextest** (de-risks Phase 7)
    - On a native `windows-latest` runner (or the `BUILD_WIN` host per the `os`
      skill), run
      `cargo nextest list -p biscuit-tui-cli --features terminal-tests -E '<L1 filter>'`
      and confirm the captured-stdout test is listed once `#[ignore]` is
      removed, and that no `real_terminal_render` / `level3_chord_select` test
      is listed.
    - Then run the full L1 cell at the CI thread count and confirm: the test
      passes, `F2 precondition HELD` is printed, no window is opened or
      focused, and the result is stable across at least five consecutive runs.
    - Record the observed latencies the two `thread::sleep(750ms)` /
      `sleep(250ms)` calls were covering, so Phase 7's bounded readiness loop
      has a real bound rather than a guessed one.
    - **Kill criterion:** if the test is flaky under parallel L1 load even with
      a bounded readiness loop, escalate before deleting the dedicated
      workflow — spec AC8 requires the test to be *discovered and executed* by
      the normal cell, and a flaky required cell is worse than the status quo.

- [x] **S3 — companion registry shape probe** (de-risks Phase 5)
    - Write the proposed registry as a literal document (one entry per suite:
      `name`, `owner`, `recipe`, `environment`, `outcome-schema`) and hand-run
      each of the six recipes it names — the Python suites, the test-audit
      typecheck/Vitest pair — capturing what counts and duration each actually
      emits today.
    - **Question to answer:** can machine-readable counts be obtained from
      every suite without changing the suites themselves (e.g. `unittest`'s
      summary line, Vitest's `--reporter=json`), or does one of them need a
      reporting flag added? The answer determines whether Phase 5 is one task
      or two.

### Baseline Capture

- [x] **Record the current-state inventory**
    - Capture, from the current `ci.yml`, the exact list of steps in
      `preflight` (9 Python suites) and `ci-tooling` (8 Python suites +
      `ci_workflow_contracts` + `ci-rollup` + test-audit + `ci-plan`), and the
      8-suite duplication the specification measured at `08f536b08`.
    - Capture the contract tests that assert today's shape and must be
      rewritten rather than deleted:
      `tools/test-toolkit/tests/ci_workflow_contracts.rs` lines ~237–256,
      ~365, ~752–836, ~1009, ~2033–2036, ~2172, ~2559–2620, ~2917–2918.
    - **Checkpoint:** this list is the Phase 8 work order; nothing in it may be
      dropped without a ruling.

---

## Phase 2 — Pending Contract Fixtures (Failing Oracles)

Uses the existing `scripts/ci/pending_contracts.py` `@pending` decorator, whose
whole purpose is this phase: the fixture asserts target behavior, the decorator
asserts it currently fails *for the recorded oracle string*, and promotion is
deleting the decorator. Every fixture below must be demonstrated failing for
its intended reason before any implementation phase starts.

The three work-groups touch disjoint files and run concurrently.

### Work-group 2.A — Python plan/scope fixtures (`scripts/ci/test_*.py`)

- [x] **Suite-ownership fixtures**
    - In `test_affected_scope.py`: every registry suite has exactly one owner;
      an unknown suite name, a duplicate owner, a missing recipe, and an
      absent result each fail validation.
    - Oracle: the registry symbol does not yet exist.

- [x] **Path-to-owner fixtures**
    - `scripts/ci/schema.py` → selects `repo-deps` only.
    - `.github/ci/ci-baseline.toml` → selects `repo-deps` only.
    - `.github/workflows/_area-ci.yml` → selects `test-toolkit` only.
    - `tools/test-audit/package.json`, `pnpm-lock.yaml`,
      `pnpm-workspace.yaml` → select `test-toolkit` only.
    - `docs/topics/ci-cd.md` → selects neither.
    - `.github/workflows/ci.yml` → global escalation **plus** `test-toolkit`
      (R13).
    - Oracle: today these produce `flags.ci_tooling == true` and zero packages.

- [x] **Change-inventory fixtures**
    - Plan carries a sorted, repository-relative, de-duplicated inventory with
      `configuration` / `documentation` / `source` / `other` buckets, per-bucket
      and total counts; a rename contributes one logical path; `--all` records
      *no* diff inventory rather than inventing one.
    - `test_schema.py`: plan schema version is 3 and `contract.json` matches.
    - `test_resolved_plan.py`: a version-2 scope receipt is rejected with
      `scope-schema` and triggers one fresh calculation, never an in-place
      upgrade.

- [x] **Preflight fixtures**
    - `test_affected_scope.py`: documentation class yields
      `preflight_os == []`.
    - `test_ci_local.py`: the extracted `preflight` job block contains no
      `python3 scripts/ci/test_` step and no `cargo nextest` step.

- [x] **Empty-matrix fixtures**
    - `test_ci_local.py`: for a documentation-only plan, a whole-run-reuse
      plan, and a zero-executing-cell plan, both `preflight` and `area-ci`
      carry a scalar `if:` guard and neither declares a `name:` containing
      `${{ matrix.`.

### Work-group 2.B — Rust workflow-contract fixtures

- [x] **Job-inventory fixtures** (`ci_workflow_contracts.rs`)
    - `ci.yml`'s top-level job set is exactly
      `{validation, scope, preflight, area-ci, ci-gate, ci-reporting}`.
    - `ci-gate`'s `needs` is exactly `[validation, scope, preflight, area-ci]`
      and its accepted conclusions are still `success|skipped`.
    - `biscuit-tui-windows-captured-stdout.yml` is absent from the workflow
      directory and from the specialized-workflow inventory.
    - `ci-reporting` carries `if: always()` and `continue-on-error: true`, and
      no blocking job carries `continue-on-error`.
    - Oracle: the current file asserts the opposite for each.

- [x] **Companion-completeness fixtures** (`scripts/ci-rollup-tests.rs`)
    - Extend the existing `a_skipped_companion_downgrades_a_green_report` /
      `..._green_lint` / `a_companion_with_no_reported_outcome_...` family:
      with two declared suites, one `success` and one `skipped` must downgrade;
      a status naming an unregistered suite must fail; a suite with no recorded
      counts or duration must render `not recorded`, never `0`.
    - Oracle: `Status::companion` is a single `Option<String>` today.

### Work-group 2.C — Windows discovery fixture

- [x] **L1 discovery fixture**
    - A test (or `ci_workflow_contracts.rs` assertion) that
      `biscuit-tui/cli/tests/windows_captured_stdout.rs` carries no `#[ignore]`
      attribute and no `thread::sleep(Duration::from_millis(` call, and that
      `biscuit-tui/justfile` defines no `test-windows-captured-stdout` recipe.
    - Oracle: all three are present today.

### Checkpoint

- [x] **Prove every fixture fails for its recorded reason**
    - Run each suite and capture the failure message per fixture into
      `fixes/_complete/2026-09-13-cicd-redundancies/oracles.md`. A fixture that fails for
      a *setup* reason (import error, missing fixture file) is not an oracle and
      must be repaired before proceeding.
    - The Python suite as a whole must be **green** (pending fixtures invert),
      so a real regression in a later phase is still visible.

---

## Phase 3 — Package-Model Foundations

Depends on rulings R1–R5 and spike S1. The two work-groups are independent and
concurrent.

### Work-group 3.A — `repo-deps` joins the root workspace

- [x] **Remove the nested workspace**
    - Delete the `[workspace]` stanza at the head of `scripts/Cargo.toml` and
      delete `scripts/Cargo.lock`.
    - Add `"scripts"` to `members` in the root `Cargo.toml`.
    - Keep the `local-tools` feature split and its explanatory comment intact —
      the always-runs `ci-rollup` build depends on it (R2).

- [x] **Migrate every `--manifest-path scripts/Cargo.toml` call site**
    - Replace with `-p repo-deps` wherever a package selector suffices; keep
      `--no-default-features --bin ci-rollup` semantics byte-identical.
    - Sites enumerated in R5, minimally: `just/ci-local.just`'s
      `BISCUIT_CI_PLAN_BIN` default (`scripts/target/release/ci-plan` →
      `target/release/ci-plan`), `.github/workflows/_area-ci.yml`,
      `.github/workflows/ci.yml`, `scripts/*.sh`.
    - **Complexity:** `scripts/target/` is now stale build output. Confirm it
      is gitignored and note its deletion in the log; do not add a cleanup step
      to a recipe.

- [x] **Declare CI metadata for `repo-deps`**
    - Add `[package.metadata.ci.tests]` with the environments ruled in R3 and
      `tiers = ["L1"]`. No `features` entry (default features — R2).
    - Verify the derived area is `root` and that `just _test repo-deps` and
      `just _lint repo-deps` resolve from the repository root.

- [x] **Validate from both working directories**
    - From repository root and from `scripts/`: `cargo metadata --no-deps`,
      `cargo nextest run -p repo-deps`, and the `--no-default-features
      --bin ci-rollup` run must all use one lockfile, one `target/`, and the
      root `.config/nextest.toml`.

### Work-group 3.B — `test-toolkit` gates

- [x] **Retire the `promotion-pending` record**
    - Delete the `[package.metadata.ci]` block in `tools/test-toolkit/Cargo.toml`
      (`gates = false`, `exclusion-class`, `owner`, `reason`, `expiry`) and the
      comment above it.
    - Add `[package.metadata.ci.tests]` with `tiers = ["L1"]` and the
      environments ruled in R3.
    - **Prerequisite already satisfied:** `tools/justfile` defines all 12
      canonical recipes; `tools` is already in the root justfile's `areas`.

- [x] **Confirm the 63 L1 tests now schedule**
    - `python3 scripts/ci/affected_scope.py tools/test-toolkit/src/lib.rs`
      produces `test-toolkit` cells; `just _test test-toolkit` passes locally.
    - Confirm `exclusion-class` validation still rejects an unknown class
      (`EXCLUSION_CLASSES` keeps `promotion-pending` as a *legal* class; only
      this package's use of it is retired).

### Checkpoint

- [x] **Both packages schedule and pass**
    - `just ci-local --plan` from the repository root shows `repo-deps` (area
      `root`) and `test-toolkit` (area `tools`) cells.
    - The root `Cargo.lock` diff contains no version change to an unrelated
      crate (S1 kill criterion).

---

## Phase 4 — Suite Ownership Registry and Path-to-Owner Selection

Depends on Phase 3 (the owners must exist) and rulings R7, R13.

- [x] **Define the suite registry**
    - One declaration site in `scripts/ci/affected_scope.py` (or a sibling
      module it imports), each entry: suite `name`, `owner` package, canonical
      `recipe`, `environment`, and `kind` (`cargo` | `companion`).
    - Entries per the specification's ownership table:
      `repo-deps` owns the eight `scripts/ci/test_*.py` suites plus
      `test_ci_local.py` and `test_runner_loss.py`; `test-toolkit` owns the
      `tools/test-audit` typecheck and Vitest suites.
    - **Constraint:** this is a derived/declared table in the planner, not a
      new persistent store (spec D9).

- [x] **Replace `flags.ci_tooling` with path-to-owner selection**
    - Delete `CI_TOOLING_PREFIXES`, `CI_TOOLING_PATHS`, and the
      `flags["ci_tooling"] = ...` assignment in `plan()`.
    - Add the trigger table: `scripts/**` → `repo-deps` (normal package
      ownership via `input_paths`), plus explicit triggers for its manifest,
      canonical runner, and suite inputs; `.github/ci/**` → `repo-deps`;
      `.github/workflows/**` → `test-toolkit`; `tools/test-audit/**`,
      `pnpm-lock.yaml`, `pnpm-workspace.yaml` → `test-toolkit`.
    - A shared input may select both owners. Applied **in addition to**
      `GLOBAL_PATHS` escalation (R13).

- [x] **Remove `ci_tooling` from workflow outputs**
    - Delete the `ci_tooling=$(jq -r '.flags.ci_tooling' ...)` output in
      `ci.yml`'s `scope` job and the corresponding key in
      `legacy_scope_document()`.
    - **Ordering note:** the `ci-tooling` job itself is deleted in Phase 8;
      this task only removes the flag's *production*. Do them in that order or
      CI is momentarily inconsistent — if implementing in a single branch,
      sequence Phase 8's `ci-tooling` deletion immediately after this task.

- [x] **Promote the Phase 2.A path-to-owner fixtures**
    - Delete their `@pending` decorators; all must pass.

### Checkpoint

- [x] **Selection is exact, not merely non-empty**
    - AC5: `scripts/` only → `repo-deps`; `tools/test-audit/` only →
      `test-toolkit`; neither-owner-nor-declared-input → neither package.
    - No path selects the full workspace merely because CI tooling changed
      (Implementation Boundary).

---

## Phase 5 — Companion-Suite Generalization

Depends on Phase 4 (the registry) and ruling R7. Spike S3 determines whether
the counts task splits.

- [x] **Carry per-suite companion records in the plan**
    - `package_cells()` currently attaches `companion_suites` (a `Vec<String>`)
      to L1 cells only. Extend each attached record to the registry shape
      (name, recipe, environment, expected outcome fields).
    - Apply the R7 reuse rule: only the cell whose environment matches a
      companion's declared environment becomes non-reusable; Rust-only L1
      cells in other environments retain normal reuse.

- [x] **Generalize the workflow's companion steps**
    - In `.github/workflows/_package-ci.yml`, replace the two hard-coded
      `contains(fromJSON(inputs.companion-suites), 'homelab-frontend')` guards
      (test job ~line 446, lint job ~line 639) with a loop or per-suite step set
      driven by the declared registry.
    - Each suite records its **own** outcome, counts, and command duration into
      producer status — not one shared `companion` string (R14).
    - Node + pnpm provisioning stays gated on the existing `node-environments`
      input; `tools/test-audit` inherits the same mechanism `homelab-frontend`
      uses today.

- [x] **Carry per-suite outcomes through `ci-rollup`**
    - In `scripts/ci-rollup.rs`: change `Status::companion: Option<String>`
      (~line 682) to a per-suite map; update `companion_lint_downgrade()`
      (~line 2321) and the R12 expectation check (~line 1992) so one suite's
      success cannot mask another's failure or skip.
    - Unknown suite names, absent results, and duplicate owners fail contract
      validation.

- [x] **Measure and record counts + duration**
    - Per S3: add whatever reporting flag each suite needs (e.g. Vitest
      `--reporter=json`) so counts are machine-read rather than parsed from
      prose. A suite that cannot report counts renders `not recorded` with a
      reason — never `0` (AC13).

- [x] **Promote the Phase 2.B companion fixtures**

### Checkpoint

- [x] **AC4 holds end to end**
    - Every table suite has one owner, one recipe, one trigger set, one
      selected cell, one machine-readable outcome; a deliberately skipped
      companion downgrades its owning cell; an unregistered suite name fails a
      contract test.
    - AC6 rehearsal: a documentation-only second push reuses eligible cells and
      re-runs only the still-required companion work.

---

## Phase 6 — Resolved-Plan Change Inventory

Depends on rulings R8, R9. Independent of Phases 4–5 in intent but edits the
same two files (`affected_scope.py`, `schema.py`), so run it **after** Phase 4
rather than concurrently.

- [x] **Classify changed paths once, in the calculator**
    - The calculator already receives `files`; normalize (`\` → `/`, strip
      `./`), sort, de-duplicate, and bucket into at least `configuration`,
      `documentation`, `source`, `other`. Exhaustive: every input path lands in
      exactly one bucket.
    - Renames contribute one logical path (confirm how the caller supplies
      renames — `git diff --name-only` yields the new path; a
      `--name-status R` form yields both).
    - `--all` (manual full scope) records an explicit *"no diff inventory"*
      marker rather than an empty list that reads as "nothing changed".

- [x] **Bump the plan schema and regenerate the contract**
    - `RESOLVED_PLAN_SCHEMA_VERSION` 2 → 3; add the inventory to the required
      field map in `schema.py` (~line 160) and to `validate_resolved_plan`.
    - Regenerate `.github/ci/schemas/contract.json`; `test_schema.py`'s drift
      assertion must pass.

- [x] **Confirm one-time scope-receipt rejection**
    - A version-2 receipt is rejected with the existing `scope-schema` reason
      and causes one fresh calculation; it is never upgraded in place.
    - Validation receipts remain reusable where their existing cell and
      gate-input checks still qualify (spec §5).

- [x] **Promote the Phase 2.A inventory fixtures**

### Checkpoint

- [x] **AC10 holds**
    - Plan validation, the generated contract, the scope-receipt rejection
      fixture, and both renderers (Phase 9) agree on the inventory's shape.
      Renderer agreement is re-verified in Phase 9.

---

## Phase 7 — The Windows Captured-Stdout Test Becomes Ordinary L1

**Fully concurrent with Phases 4–6** — it touches
`biscuit-tui/cli/tests/windows_captured_stdout.rs`, `biscuit-tui/justfile`, and
(in Phase 8) workflow files. Depends only on spike S2.

Starting advantage confirmed during planning: `biscuit-tui/cli/Cargo.toml`
already declares `[package.metadata.ci.tests] features = ["terminal-tests"]`
with `local-features = []`, so the CI L1 cell **already compiles** this test
target and the local `just test` does not. No feature plumbing is required.

- [x] **Remove the `#[ignore]` attribute**
    - Delete the `#[ignore = "requires a Windows host; ..."]` attribute (~line
      280). The `#![cfg(windows)]` inner attribute (line 75) stays — it is how
      the tier contract expresses "Windows-only" (spec D4).

- [x] **Replace the fixed sleeps with bounded readiness**
    - `thread::sleep(Duration::from_millis(750))` (~line 303) and
      `sleep(Duration::from_millis(250))` (~line 308) become a bounded
      observation loop or bounded retry, with the bound derived from S2's
      measured latencies.
    - The loop must **fail loudly on timeout**, preserving the
      console-precondition assertion strength (Implementation Boundary: no
      weakening).
    - Must not open or focus a terminal or browser window (AC9).

- [x] **Update the test's module documentation**
    - The `//!` block (lines 1–74) currently explains at length *why* the test
      is `#[ignore]`d, cites the claudine `level3_wrap_ctrl_c.rs` precedent for
      that shape, and says it "RUNS only when explicitly invoked with
      `--ignored`". All of that is now stale and must be rewritten to state the
      L1 tier ruling (spec D4) and the nextest process-per-test property that
      makes process-wide handle rewiring safe (R6).
    - **Drift note:** this is the spec's "stale Biscuit TUI test/reproduction
      docs" item; the code is correct and the comment is wrong.

- [x] **Delete the dedicated recipe**
    - Remove `test-windows-captured-stdout` from `biscuit-tui/justfile`
      (~lines 78–88) and its header comment referencing the retired workflow.
    - Verify `just check-canonical biscuit-tui` still passes (it is not one of
      the 12 required recipes).

- [x] **Promote the Phase 2.C discovery fixture**

### Checkpoint (requires native Windows — Validation step 6)

- [x] **Real Windows runtime evidence**
    - `cargo nextest list` under the L1 filterset lists the test; a full L1
      cell run passes with `F2 precondition HELD` in the log; five consecutive
      runs are stable at CI thread count.
    - A deliberately broken precondition fails the cell loudly.
    - **A macOS→Windows GNU cross-check is compile evidence only and does not
      satisfy this checkpoint.** Use the `os` skill to reach a native host.

---

## Phase 8 — Workflow Surgery

Depends on Phases 3–7: nothing may be deleted until its replacement owner is
scheduling. Sequence the three groups in order — 8.A's deletions assume 8.B's
contract rewrites land in the same commit, or the contract suite is red.

### Work-group 8.A — Job removal and preflight slimming

- [x] **Slim `preflight` to prerequisites only**
    - Delete all nine `python3 scripts/ci/test_*.py` steps (~lines 418–437).
    - Retain: checkout, `rustup show`, `just` install, nextest install, the
      toolchain/tooling verification step, the no-wrapper `cargo metadata`
      step, and `just check-canonical`.
    - Add the explicit scalar guard ruled in R12.
    - AC1: no Python, Rust, or TypeScript suite remains.

- [x] **Delete the `ci-tooling` job**
    - Remove the whole job (~lines 503–612) and every `needs`/`RESULTS`
      reference to it.

- [x] **Delete `biscuit-tui-captured-stdout` and its workflow**
    - Remove the job (~lines 489–501) and delete
      `.github/workflows/biscuit-tui-windows-captured-stdout.yml`.

- [x] **Update `ci-gate`**
    - `needs` → `[validation, scope, preflight, area-ci]`; remove the two
      corresponding `RESULTS` lines. Nothing else changes (R11, spec D6).
    - The job `name: ci-gate` and the `protect-your-bacon` required context
      are untouched.

- [x] **Make documentation-class preflight empty**
    - `classify_preflight()` returns `[]` for the documentation class with a
      new reason string (R10).

### Work-group 8.B — Contract-test rewrites

- [x] **Rewrite `ci_workflow_contracts.rs` for the new shape**
    - Per the Phase 1 baseline list: the job-inventory constant (~line 2033),
      the preflight/ci-tooling block extraction (~line 2172, ~line 2917), the
      specialized-workflow inventory (~lines 752–836), the summary
      classification assertion (~line 1009), and the two `ci_tooling_*` tests
      (~lines 2559–2620).
    - The two `ci_tooling_*` tests are **replaced**, not deleted: their new
      subject is the registry and path-to-owner table. Spec §3: validate suite
      **identities and owners**, not fragile shell-command strings.
    - Add: every registered suite has exactly one owner and may be invoked only
      by that owner's package job.

- [x] **Update `test_ci_local.py`'s gate-step fixtures**
    - The `WorkflowGateStepTests` family feeds `RESULTS` with six job names;
      reduce to four and keep the negative test that a fold accepting `failure`
      is rejected.

### Work-group 8.C — Empty-matrix resolution

- [x] **Guard both matrix jobs on scalar plan outputs**
    - `preflight` and `area-ci` each carry a scalar `if:`; neither declares a
      `name:` containing a matrix expression (the existing comments at ci.yml
      ~lines 376–382 and ~446–452 explain why — keep and update them).
    - AC14: a zero-entry area matrix resolves `skipped` immediately after
      `scope`.

- [x] **Promote the Phase 2.A/2.B workflow fixtures**

### Checkpoint

- [x] **AC1, AC7, AC14, AC15 hold statically**
    - `ci.yml` defines exactly six top-level jobs; no retired job name appears
      anywhere in `.github/`; `actionlint` passes; the workflow-contract suite
      is green.

---

## Phase 9 — `ci-reporting` and the Local Change Report

Depends on Phases 5, 6, and 8.

- [x] **Replace `summary` with `ci-reporting`**
    - Rename the job and rewrite its `needs` to
      `[validation, scope, area-ci, ci-gate]`; keep `if: always()` and
      `continue-on-error: true`.
    - **Done in Phase 8:** the rename itself (job id `summary` → `ci-reporting`,
      `name: ci-reporting (advisory)`) and the removal of the two retired
      `needs`/`RESULTS` entries, because Phase 2.B's promoted job-inventory
      fixture asserts the six-job set by id. Its `needs` is currently
      `[validation, scope, preflight]`; rewriting it to
      `[validation, scope, area-ci, ci-gate]` and implementing the three modes
      remain this phase's work.
    - Implement the three modes: (1) reused whole-PR validation — link the
      authoritative prior run, state that no package cells executed;
      (2) successful scope — read the resolved plan and the existing
      `ci-results-<area-slug>` slices via `ci-rollup summarize` (or its shared
      typed model); (3) failed/cancelled bootstrap — report the first
      actionable infrastructure failure.
    - **Constraint:** mode 2 must not parse raw artifacts into a second result
      model (spec D7).

- [x] **Render the required report fields**
    - Change inventory; direct and reverse dependency package sets;
      per-environment test counts including machine-recorded companion counts;
      Linux-only `ci` lint command duration, explicitly labeled; per-environment
      test duration; literal `ci` / `local` / `prior-local` origins;
      `not recorded` + reason for unavailable measurements.
    - The literal `cicd` is never introduced. `check` and `lint` stay CI-origin.
    - No baseline, accepted-gap, missing-cell, or merge policy. The report may
      state that no package tests were required; it must **not** claim
      mergeability.

- [x] **Ensure all-reused areas still fan out**
    - AC16: an area whose cells are all reused must still produce its
      `ci-results-<slug>` slice, or `ci-reporting` loses those cells. Verify
      against `_area-ci.yml`'s fan-out condition and add a contract fixture if
      the condition can produce a slice-less area.

- [x] **Render the same inventory locally**
    - Extend `scripts/ci-plan.rs`'s `Plan` struct with the inventory field and
      render it through `TerminalRenderable` components (`Prose`,
      `UnorderedList`, `Table`) — spec §7 prefers the existing `ci-plan` typed
      renderer.
    - Update `just/ci-local.just` so the pre-push path names the changed
      documents and states that no package tests are required when the plan
      selects no cells. Today it prints only
      `CI scope: N package(s) (class=...)` (~line 319) and, for an empty diff,
      `nothing to gate` (~line 214).
    - **This is an affirmative successful scheduling decision** — never a
      warning, failure, accepted gap, or fabricated passing result.

- [x] **Prove byte-equivalent data reaches both renderers**
    - Validation step 4: documentation-only fixtures at repository, area, and
      package level; assert the terminal and Markdown renderers consume the
      identical inventory payload.

### Checkpoint

- [x] **AC11, AC12, AC13 hold**
    - A documentation-only change at each of the three ownership levels names
      its documents locally and in CI, states no package tests are required,
      creates zero package/preflight executions, and leaves the merge decision
      to `ci-gate`.

---

## Phase 10 — Documentation and Drift

Per CLAUDE.md's Drift Maintenance rule and the specification's Implementation
Boundaries. All five work items are independent and concurrent.

- [x] **`.github/ci/README.md`**
    - Rewrite the job inventory (the six-job list at ~line 564), the tooling
      selection section, the companion-suite registry, the change inventory
      field, and the `ci-reporting` modes.

- [x] **`docs/topics/ci-cd.md`**
    - Remove the `biscuit-tui-windows-captured-stdout.yml` row (~line 309) from
      the specialized-workflow table; document the new job set and the tooling
      ownership table.

- [x] **Skills** (`.claude/skills/`)
    - `rust-devops`: the scheduling model, suite registry, `ci-reporting`,
      plan schema v3.
    - `rust-testing`: `repo-deps` and `test-toolkit` now gate; the Windows
      console test is L1, not a manually-invoked `#[ignore]` gate.
    - `os`: the Windows captured-stdout test is ordinary L1 evidence on
      `windows-latest`; record whatever S2 taught about console allocation
      under nextest (CLAUDE.md requires OS facts learned the hard way to land
      in the same change).

- [x] **Dependency documentation**
    - `docs/dependencies.md` (and any per-area file) for the `scripts` →
      root-workspace membership and the retired `scripts/Cargo.lock`. No
      version changes.

- [x] **Comment pass over every behavior change**
    - Each edited symbol's `///` / `//!` / `//` comments are corrected or
      deleted in the same change. Named stale sites: the `CI_TOOLING_PREFIXES`
      block comment (`affected_scope.py` ~lines 99–121), the `ci-tooling` and
      `summary` job comments in `ci.yml`, the `windows_captured_stdout.rs`
      module doc (Phase 7), the `test-toolkit` exclusion comment, and the
      `scripts/Cargo.toml` `--manifest-path` example in its feature comment.
    - **Scope discipline:** if a comment-only cleanup is separable, it is its
      own commit with no behavior lines in the diff.

### Checkpoint

- [x] **No document describes a retired job, flag, or recipe**
    - `rg 'ci-tooling|ci_tooling|biscuit-tui-captured-stdout|test-windows-captured-stdout|promotion-pending'`
      over `docs/`, `.github/`, `.claude/skills/`, `just/`, and every `justfile`
      returns only historical `fixes/` and `features/` records.
    - **Amended for `promotion-pending`.** That token is a *live* exclusion
      class, not a retired entity: `EXCLUSION_CLASSES` still accepts it (Phase
      3) and six packages still declare it (`biscuit-visualized`, the three
      `biscuit-clipboard` members, `biscuit-test-harness`,
      `biscuit-browser-harness`). Its three remaining hits — all in
      `.github/ci/README.md`'s `[package.metadata.ci]` field documentation —
      are required, not drift. Only `tools/test-toolkit`'s *use* of it was
      retired, and that manifest carries no exclusion record.
    - Enforced mechanically rather than by a one-time grep:
      `no_reader_facing_document_or_recipe_names_a_retired_ci_entity` in
      `tools/test-toolkit/tests/ci_workflow_contracts.rs` globs `docs/`,
      `.github/`, `.claude/skills/`, `just/`, every justfile, and
      `tools/test-audit/README.md` for the four genuinely retired identities.

---

## Phase 11 — Validation and Rollout

The specification's Validation and Rollout section, in dependency order. This
phase is the acceptance gate; no criterion may be waived by inspection.

- [x] **Local dependency-derived scope** (Validation 2)
    - Run: the compact Python CI suites; `repo-deps` and `test-toolkit` L1
      suites; the test-audit check; `ci_workflow_contracts`; the
      schema-generation drift check; `actionlint`; and the root
      `just ci-local --plan` preview.
    - **Do not substitute a full workspace test run.**

- [x] **Dual-directory workspace validation** (Validation 3)
    - From repository root and from `scripts/`: one lockfile, one target
      directory, one nextest config.

- [x] **Documentation-only fixtures at three levels** (Validation 4)
    - `docs/...` at repository root, `<area>/docs/...`, `<package>/README.md`.
    - Each: zero packages, zero areas, named documents in both renderers,
      byte-equivalent inventory payload.

- [ ] **Hosted run on this fix's branch** (Validation 5) — **BLOCKED: needs push**
    - Verify: job ownership, artifact collection, report aggregation,
      `ci-reporting`'s advisory failure behavior, and prompt zero-matrix
      resolution.
    - **Two forced-failure probes:** a failed package-owned tooling suite must
      block through `area-ci` → `ci-gate`; a forced `ci-reporting` failure must
      not block.
    - Phase 11 was instructed not to commit or push. Every part reachable
      without a hosted run is recorded in `acceptance.md`; the structural
      halves (advisory `continue-on-error`, scalar zero-matrix guards, the
      `ci-gate` fold) are additionally locked by `ci_workflow_contracts`.

- [x] **Native Windows evidence** (Validation 6)
    - `captured_stdout_receives_only_value_no_tui_bytes` **passes natively on
      `$BUILD_WIN`** under the package's declared `terminal-tests` feature and
      the real `ci` nextest profile (run ID `f5317a6d`), and the deliberately
      broken console precondition **fails** that cell. Not cross-compile
      evidence. The hosted `windows-latest` JUnit artifact lands with
      Validation 5.

- [ ] **Documentation-only follow-up push** (Validation 7) — **BLOCKED: needs push**
    - Prove eligible prior cells report as reused and no suite is silently
      dropped. One `scope-schema` receipt miss is the expected migration
      behavior; record it rather than suppressing it.
    - Reuse behavior is locked locally by
      `a_reused_cell_reaches_its_area_summary_without_being_re_executed`,
      `an_all_reused_area_still_produces_its_result_slice`, and
      `the_grid_shows_a_reused_cell_with_its_origin_and_evidence`.

- [x] **Final acceptance sweep**
    - Walk all sixteen acceptance criteria and record the artifact that
      demonstrates each in
      `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md`.
    - Confirm every `@pending` decorator added in Phase 2 is gone and its test
      passes — a surviving pending fixture means a contract did not land.
    - Confirm `ci-gate` is still the `protect-your-bacon` required context and
      that its ruleset was not edited.

- [x] **Close and move the fix to `_complete`** — user decision, 2026-09-15
    - The user accepted the remaining hosted uncertainty and explicitly requested
      `fixes/_complete/` as the destination.
    - Validations 5 and 7 and accumulated hosted assertions remain deferred, not
      passed; the closure decision in `acceptance.md` supersedes the hosted gate
      on this archival step. The specification records `status: complete` and
      `implemented: true`.

---

## Concurrency Map

| Phase | May run concurrently with | Reason |
|---|---|---|
| 2.A / 2.B / 2.C | each other | disjoint files (Python suites, Rust contracts, Biscuit TUI test) |
| 3.A / 3.B | each other | `scripts/` vs `tools/test-toolkit/` |
| 7 | 4, 5, 6 | Biscuit TUI files share nothing with the planner or rollup |
| 8.A → 8.B | **sequential** | contract rewrites must land with the deletions they describe |
| 10 (all items) | each other | five independent documents |

**Serialization points.** Phases 4, 5, and 6 all edit
`scripts/ci/affected_scope.py` and `scripts/ci/schema.py`; run them in order
rather than concurrently. Phase 8 must not start until 3–7 are complete —
every deletion it makes assumes its replacement owner is already scheduling.

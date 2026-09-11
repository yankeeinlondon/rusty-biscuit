---
title: CI cleanup — impacted areas, reusable evidence, and area-owned outcomes
status: blocked
created: 2026-09-11
phase: 4
total_phases: 9
agent: codex/gpt-5.6-sol
yolo: true
spec: fixes/2026-09-11-cicd-cleanup/spec.md
depends_on:
  - fixes/2026-09-10-local-affected-scope/spec.md
blocked_on: fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md
packages:
  - test-toolkit
  - repo-deps
source_files_during_phase_1: []
docs_created_during_phase_1:
  - fixes/2026-09-11-cicd-cleanup/prerequisite-audit.md
  - fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md
  - fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md
docs_updated_during_phase_1:
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - scripts/ci/schema.py
  - scripts/ci/pending_contracts.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_ci_local.py
  - scripts/ci-rollup-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .githooks/tests/test-pre-push.sh
  - .github/workflows/ci.yml
  - .github/ci/schemas/contract.json
docs_created_during_phase_2:
  - .github/ci/schemas/README.md
docs_updated_during_phase_2:
  - .github/ci/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_ci_local.py
  - .github/ci/schemas/contract.json
  - .github/workflows/ci.yml
  - just/ci-local.just
docs_created_during_phase_3: []
docs_updated_during_phase_3:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_3:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_4:
  - scripts/ci/local_evidence.py
  - scripts/ci/constraints.py
  - scripts/ci/schema.py
  - scripts/ci/affected_scope.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_constraints.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - scripts/ci-plan.rs
  - scripts/ci-plan-tests.rs
  - scripts/Cargo.toml
  - scripts/cross-check.sh
  - .githooks/pre-push
  - .githooks/tests/test-pre-push.sh
  - .github/workflows/ci.yml
  - .github/ci/schemas/contract.json
  - just/ci-local.just
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_created_during_phase_4: []
docs_updated_during_phase_4:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_4:
  - .claude/skills/rust-devops/ci-cd.md
---

# CI Cleanup Execution Plan

## Outcome

Deliver the specification as one coherent CI contract: select only impacted
package areas, preserve package-keyed result identity, combine verified local
and hosted results per cell, present one owned outcome per selected area, and
remove `ci-verdict` without weakening merge protection.

The implementation is complete only when the GitHub UI, machine-readable
results, local pre-trigger plan, and merge rules all agree on the same resolved
cell model.

## Execution constraints

The dependent 2026-09-10 specification is an entry condition, not work to
silently duplicate here. At plan creation, the checkout still exposes the old
single-environment verifier (`verified_environment`), one
`--exclude-environment`, and compile-only reverse-dependent package selection.
Phase 1 must establish whether the dependency will land first or be staged as a
reviewed prerequisite before this plan changes those surfaces.

The four open questions in the specification are implementation blockers. The
recommended starting choices are OQ1 Option B, OQ2 Option B, OQ3 Option C with
Option B as a proven fallback, and OQ4 Option A when the controlled fixture
shows acceptable PR presentation. These recommendations are not substitutes
for Ken's recorded rulings.

No task in this plan authorizes a commit, push, workflow dispatch, ruleset edit,
or WSL rerun. Obtain the relevant authorization at the task boundary. In
particular, prior WSL evidence must be reused or the trigger must stop.

## Phase 1 outcome (2026-09-11) — STOPPED

Three of five Phase 1 tasks are done; two are blocked on decisions and
authorization no agent can supply. The plan is stopped here by Phase 1 task
1's own instruction, because the audit found the inherited contracts absent.

| Deliverable | File |
|---|---|
| Prerequisite acceptance-criterion checklist | `prerequisite-audit.md` |
| Dated baseline of every surface this fix touches | `baseline-2026-09-11.md` |
| What Ken must decide, and what was settled without him | `open-questions-and-blockers.md` |

**Headline findings.**

- `fixes/2026-09-10-local-affected-scope/spec.md` is **not implemented** —
  3 of 16 criteria met, 2 vacuous, 2 partial, 9 absent. Phases 3 and 4 are
  written to modify contracts that do not exist. Ken must choose whether to
  land it first or absorb it (blocker B0).
- The PR #76 root cause is now exact: excluding an environment edits the
  *matrix* while `ci-rollup` derives expected cells from the *policy*, and
  nothing injects local outcomes. `scripts/ci-rollup.rs` contains no
  occurrence of `excluded`, `local_evidence`, `local-origin`, or `LOCAL`.
- The reusable-workflow chain is **already three levels deep**. Phase 6's
  area-level workflow consumes the fourth and last. No margin remains.
- `scripts/ci-rollup.rs` must keep linking none of the monorepo crates; the
  `--plan` renderer belongs in a new `local-tools`-gated bin, not in
  `ci-rollup`.
- No job in `ci.yml` holds `checks: write`, which OQ4's mechanism requires.
- `sniff` and the planner already disagree on four package names, so Phase
  3's drift contract cannot be naive set equality.
- All eight focused gates are green at `674d0d886`, so any red in Phase 2 is
  new — except that the hook suite needs `env -u CDPATH` on this host.

## Phase 1 — Close prerequisites and design gates

- [x] Audit `fixes/2026-09-10-local-affected-scope/spec.md` against the current
  hook, evidence schema, scope artifact, workflow, rollup, and tests; record an
  acceptance-criterion checklist and stop this plan until the inherited
  contracts are implemented or staged as an explicit prerequisite.
  → `prerequisite-audit.md`. **Verdict: not implemented** (3 of 16 criteria
  met, 2 vacuous, 2 partial, 9 absent). This plan is stopped at Phase 1 per
  the task's own instruction; see `open-questions-and-blockers.md` for the
  land-first-versus-absorb decision Ken must make.
- [x] Capture the current planner, receipt, rollup, workflow-job, ruleset, and
  GitHub UI behavior in a dated baseline note, including the PR #76 seven-cell
  macOS failure, current result schema versions, current full-scope job count,
  and current required-check context.
  → `baseline-2026-09-11.md`. Full scope 66 matrix entries / 486 job estimate;
  run 34638047631 = 144 checks, 63 with literal `${{ … }}`, 1 failure; the
  seven `MISSING` macOS cells named exactly; ruleset 19747338 requires the
  single context `ci-verdict`; schema versions rollup 2 / environments 1 /
  receipt 1 / scope document **unversioned**. Also records two traps found
  while measuring: `CDPATH` breaks the hook suite on this host, and the
  planner/sniff package universes already disagree by four names.
- [ ] **BLOCKED (B5).** Create the minimal controlled presentation fixture on an
  authorized throwaway branch and the required-check fixture in an authorized
  scratch repository; prove skipped-job naming, reusable-workflow display
  nesting, synthetic `cancelled` and `neutral` check runs, workflow
  conclusions, and required-workflow behavior without launching the workspace
  test matrix.
  → Needs a push that triggers workflows plus scratch-repo and ruleset writes;
  no authorization is obtainable in a non-interactive session. Requirements
  written up in `open-questions-and-blockers.md` §B5 so it can run the moment
  it is authorized. **Two of the six items are already proven live** by run
  34638047631 at no cost — skipped-job naming (63 literal `${{ … }}` labels)
  and reusable-workflow nesting (already at three levels, so Phase 6's
  area-level workflow consumes the last one).
- [ ] **BLOCKED (B0–B4).** Record Ken's decisions for OQ1–OQ4 in the
  specification and this plan, including the selected downstream-seam location,
  persisted-constraint store, merge gate, and accepted-gap conclusion; remove
  conditional branches from subsequent implementation tasks once each choice is
  settled.
  → No ruling is obtainable in a non-interactive session. Each question's
  decision-ready state, and the read-only facts that narrow it, are in
  `open-questions-and-blockers.md`. OQ3 and OQ4 additionally cannot be closed
  until the B5 fixtures run.
- [x] Run GitNexus upstream impact analysis for every existing function or
  method that the implementation will modify, warn before any HIGH or CRITICAL
  change, and map the affected symbols to the concrete test scopes with
  `sniff repo packages`, `sniff repo package-areas`, and
  `sniff repo package-dependencies`.
  → `baseline-2026-09-11.md` §8–§9. Ten symbols analyzed, all
  `epistemic: "exact"`, no execution flow touched. **HIGH risk: `verdict` in
  `scripts/ci-rollup.rs`** (27 direct / 35 total) — warned in §8; Phases 5
  and 7 own it and are gated on OQ3. Only one workspace package is in the
  blast radius: `test-toolkit` (area `tools`).
- [ ] **Validation checkpoint — NOT MET.** Confirm the prerequisite checklist is
  green, all four rulings are recorded, the GitHub fixtures have retained
  evidence, and the planned reusable-workflow nesting is no deeper than four
  levels.
  → Of the four conditions, one holds and three do not:
  - prerequisite checklist green — **NO.** 9 of 16 criteria absent
    (`prerequisite-audit.md`).
  - four rulings recorded — **NO.** None; OQ3/OQ4 are additionally gated on
    the B5 fixtures.
  - GitHub fixtures have retained evidence — **NO** for the check-run and
    ruleset cases; **YES** for skipped-job naming and display nesting, both
    evidenced by run 34638047631.
  - nesting no deeper than four levels — **YES, but exactly at the ceiling.**
    The chain is already three levels; the area-level workflow makes four,
    which is GitHub's maximum. No margin remains for a later insertion.

## Phase 2 outcome (2026-09-11) — COMPLETE, still blocked for Phase 3

All eight tasks are done. Every focused gate is green and the plan remains
stopped for **implementation**: B0–B5 are untouched and Phase 3 may not start.

**Why Phase 2 could proceed while the plan is blocked.** The Phase 1 stop is on
tasks that *change* CI behavior. Phase 2 changes none: it adds a schema module
nothing imports yet and regression fixtures that assert the target contract and
currently fail. Each blocker was checked against this phase's deliverables:

| Blocker | Effect on Phase 2 |
|---|---|
| B0 land-first vs absorb | **None.** Both options need the same end-state contracts; only which plan builds them differs. |
| B1 OQ1 seam location | **None.** AC1 holds under every option, so the fixture asserts "no area, job, or cell for an unchanged dependent" and is option-neutral. `dependent_seam` is an optional package field, present under Option B and absent otherwise. |
| B2 OQ2 constraint store | **One line.** `write_prohibition` in the hook suite is the only thing that knows the location; the contracts assert the refusal, which holds under every option. |
| B3 OQ3 merge gate | **None.** No Phase 2 fixture asserts a merge mechanism. |
| B4 OQ4 gap conclusion | **None.** Design Decision 10 settles the machine-readable `ACCEPTED GAP` state regardless of the GitHub conclusion used to display it. |

**How the failing fixtures are retained without a red suite.** A plain failing
test would leave every suite red and hide the next real regression. Instead each
language gets a small wrapper — `@pending` (Python), `pending_contract` (Rust),
`pending_contract` (shell) — that runs the fixture, requires it to fail, and
requires the failure message to contain a recorded oracle string:

- fails with the oracle → pass; the contract is still pending;
- fails without it → **fail**, reported as broken fixture setup;
- passes → **fail**, demanding the wrapper be deleted.

The third case is what makes it safe: a contract cannot be implemented and
silently leave a pending fixture behind claiming it is not.

**Counts.**

| Suite | Before | After | Pending |
|---|---:|---:|---:|
| `scripts/ci/test_schema.py` (new) | — | 37 | 0 |
| `scripts/ci/test_resolved_plan.py` (new) | — | 22 | 16 |
| `scripts/ci/test_evidence_reuse.py` (new) | — | 23 | 17 |
| `scripts/ci/test_ci_local.py` | 8 | 13 | 4 |
| `scripts/ci-rollup-tests.rs` | 137 | 146 | 6 |
| `tools/test-toolkit/tests/ci_workflow_contracts.rs` | 58 | 67 | 6 |
| `.githooks/tests/test-pre-push.sh` | 9 | 13 | 3 |

**Two facts measured here that later phases need.**

- **sniff's area rule is not "the manifest's parent directory."** It answers
  `tools` for `tools/test-toolkit` but `biscuit-test-harness` for a root-level
  manifest and `biscuit-visualized` for `biscuit-visualized/src`. Phase 3 must
  derive the rule to match sniff on all five layouts the repo uses, not assume
  the common case. AC15's fixture calls sniff rather than reimplementing it.
- **A prohibited cell can never also be scheduled.** `--plan` exits non-zero
  because a *required* cell is prohibited and unsatisfied, not because the plan
  contains a prohibited-and-executing cell — that shape is rejected as invalid,
  so the dangerous document cannot be written at all.

## Phase 2 — Freeze the shared contracts with regression fixtures

- [x] Define and version one canonical resolved-plan schema containing selected
  areas, package contributions, per-cell execution/origin/state, explicit
  target kinds, compile-coverage source, accepted evidence, policy gaps,
  prohibited cells, selection reasons, and job estimates; keep package as the
  stored identity and area as a derived grouping field.
  → `scripts/ci/schema.py` (`RESOLVED_PLAN_SCHEMA_VERSION = 1`), dumped to
  `.github/ci/schemas/contract.json` for cross-language assertion, documented
  in `.github/ci/schemas/README.md`. `validate_resolved_plan` enforces the
  cross-field rules (area derived from package, reused cell names evidence,
  gap names its policy entry, prohibited cell never scheduled).
- [x] Define the next validation-receipt schema at
  `{package, environment, gate/tier}` granularity with outcome, exit code,
  completion, counts, duration, bounded failure detail, host/report
  provenance, backend proof, exact revision/tree/scope identity, and
  gate-input identity.
  → same module, `RECEIPT_SCHEMA_VERSION = 2`. `reusable_cells()` splits a
  receipt into reusable cells and coded rejections; v1 notes are reported
  `v1-requires-exact-tree`, never `malformed-receipt`.
- [x] Add planner fixtures for Claudine-plus-Playa source changes, nested areas,
  documentation/CI-tooling-only changes, explicit full scope, gates-disabled
  packages, relevant target kinds, and unchanged reverse dependencies receiving
  no area/job/cell.
  → `scripts/ci/test_resolved_plan.py`, 22 fixtures against the real workspace
  (16 pending, 6 live guards). AC15's drift fixture calls `sniff repo
  package-area` per manifest directory rather than reimplementing the rule —
  measured here: sniff answers `tools` for `tools/test-toolkit` but
  `biscuit-test-harness` for a root-level manifest, so "the manifest's parent
  directory" is the **wrong** rule and Phase 3 must not assume it.
- [x] Add evidence fixtures for exact-tree matches, matching older-head input
  identities, changed inputs, multiple environments, duplicate agreement and
  conflict, pass and fail outcomes, partial/interrupted runs, malformed notes,
  `--no-verify`, and version-1 pass-only receipts with unrecorded measurements.
  → `scripts/ci/test_evidence_reuse.py`, 23 fixtures on real Git repositories
  with real notes (17 pending, 6 live guards). Freezes two Phase 4 entry
  points: `verify_cells(plan, base, head) -> (accepted, rejections)` and
  `gate_input_identity(paths, ref)`. The identity fixtures advance the head by
  editing a file inside and outside the closure, so a Phase 4 implementation
  that merely echoed the stored identity string would still fail them.
- [x] Add rollup fixtures for mixed local/CI origins, the exact PR #76 seven-cell
  regression, area-scoped missing/failure behavior, accepted/expired/revoked
  gaps, unrelated cancellations, package-level failure details, and a
  reporting-only combined summary.
  → `scripts/ci-rollup-tests.rs`, 9 new fixtures (137 → 146 tests). The PR #76
  fixture runs the real `expected_cells` + `classify` over the six baseline
  packages and reproduces the seven macOS `MISSING` cells **by name**, plus the
  two governed `claudine-cli` L2 `POLICY GAP` cells that did not block.
  Accepted/expired/revoked gap governance was already covered by
  `an_ungoverned_policy_gap_still_blocks` and the `policy-gap-expired` cases;
  those are referenced rather than duplicated.
- [x] Add hook and workflow contract fixtures for persisted prohibitions,
  `just ci-local --plan`, non-focusing L2 behavior, unchanged worker limits,
  explicit overrides, static whole-stage skip names, and the absence of literal
  `${{ ... }}` text in reader-facing job labels.
  → `.githooks/tests/test-pre-push.sh` (9 → 13 tests),
  `scripts/ci/test_ci_local.py` (+5 `PlanSurfaceTests`), and
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` (58 → 67 tests).
  Worker limits, explicit overrides, and non-focusing L2 backends are already
  covered by `CiLocalTests`/`ThreadPolicyTests`/`L1ThreadForwardingTests` and
  are referenced, not duplicated. New suites registered in `ci.yml`'s
  `preflight` and `ci-tooling` legs; `actionlint` clean.
- [x] **Parallelizable:** split fixture authoring among the Python planner and
  evidence suites, the Rust rollup suite, and the workflow/hook contract suite
  after the schemas above are frozen; do not edit shared fixtures concurrently.
  → Authored sequentially in one session, in the stated order. No shared
  fixture was edited twice.
- [x] **Validation checkpoint:** demonstrate that each new regression fixture
  fails for the intended pre-change reason, not from broken setup, and retain
  those failure messages as the implementation oracle.
  → Met by construction. `@pending` / `pending_contract` / `pending_contract()`
  (Python, Rust, shell) run each fixture, require it to fail, and require the
  failure message to contain the recorded oracle; a fixture that fails for any
  other reason is reported as broken setup, and one that starts passing fails
  the suite until it is promoted. `BISCUIT_PROMOTE_PENDING=1` runs the Python
  fixtures unwrapped and shows 16/17/4 genuine failures across the planner,
  evidence, and `ci-local` suites.

## Phase 3 outcome (2026-09-11) — COMPLETE except the OQ1 seam

Seven of eight tasks are done; task 5 is the OQ1 seam and remains blocked on
Ken's B1 ruling. `calculate_scope` now returns the canonical resolved plan and
validates against the frozen schema; every Phase 2 fixture that held a Phase 3
contract has been promoted.

**Why Phase 3 could proceed while the plan is blocked.** Each blocker was
checked against this phase's deliverables, as Phase 2 did:

| Blocker | Effect on Phase 3 |
|---|---|
| B0 land-first vs absorb | **Absorbed in effect.** The per-cell result set the 2026-09-10 spec never built is now the planner's *input* (`accepted_cells`), built here. Nothing from that spec was implemented and then reversed. |
| B1 OQ1 seam | **Task 5 only.** AC1's presentation rule holds under all three options and is implemented and tested; `dependent_seam` stays absent from every package record. |
| B2 OQ2 constraint store | **None.** `prohibited_cells` is emitted as `[]`; no constraint can exist before Phase 4 builds the store. |
| B3 OQ3 merge gate | **None.** The planner names no merge mechanism. |
| B4 OQ4 gap conclusion | **None.** Governed gaps are emitted as the machine-readable `accepted-gap` state, which Design Decision 10 settles independently of the GitHub conclusion. |

**Three findings that later phases need.**

- **sniff's area rule *is* the manifest directory's parent** (`root` at the top
  level), contradicting Phase 2's note. `make_package_area` in
  `sniff/lib/src/filesystem/repo/detection.rs` is four lines, and a sweep of
  all 73 workspace members found exactly **one** divergence:
  `sniff repo package-area` answers `biscuit-test-harness` from that directory,
  while `sniff repo package-areas` does not list that name and the three
  identically shaped root-level members (`renderable`,
  `biscuit-browser-harness`, `tabby`) all answer `root`. That is a sniff
  self-inconsistency; the planner emits `root`, and the divergence is named in
  `SNIFF_SELF_INCONSISTENT` so it fails the day sniff is fixed.
- **The `dmls` crate belongs to area `darkmatter`, not `darkmatter/dmls`.**
  `darkmatter/dmls` *is* an area in sniff's universe — it is the area of
  `darkmatter/dmls/zed-dmls-cli`, not of the crate whose manifest sits at
  `darkmatter/dmls`. AC15 makes sniff the authority, so the Phase 2 fixture
  asserting the intuitive grouping was wrong and was corrected.
- **The frozen resolved-plan schema was amended once**, adding
  `source_packages` and `reverse_dependencies`. AC1 is stated over the
  difference between "changed" and "selected", and OQ1's seam report needs the
  dependent names; without both fields the plan could not express AC1 and two
  Phase 2 fixtures were unsatisfiable. The version stayed 1 because no consumer
  had read version 1 yet.

**Transitional projection.** `calculate_scope` returns the resolved plan;
`legacy_scope_document` projects it back into today's `scope.json` shape, which
`ci.yml`, `just/ci-local.just`, and `ci-rollup` still read. It is a projection
of the same cells, not a second calculation, so the two documents cannot
disagree about what CI will run. Phases 5 and 6 move those consumers onto
`cells` and delete it.

## Phase 3 — Produce the canonical area-aware execution plan

- [x] Extend `scripts/ci/affected_scope.py` to derive each workspace member's
  area from its manifest directory using the same rule as
  `sniff repo package-area`; emit the area on every matrix and policy record
  without introducing a committed mapping file.
  → `package_area()` replicates `make_package_area`. Area is emitted on every
  resolved-plan package and cell record, on every legacy matrix and policy
  record, and grouped into `areas[]` with a selection reason. No mapping file.
- [x] Remove `CHECK_OS`, `scheduled_reverse_ids`, the single
  `excluded_environment`, and compile-only selection of unchanged reverse
  dependencies; accept the verified per-cell result set and omit exactly those
  executions while retaining them as expected local-origin result cells.
  → All four are gone. `calculate_scope` takes `accepted_cells`
  (`{package, environment, gate}` records) and `accepted_environments`; an
  accepted cell becomes `execution: reuse`, `origin: local`, `state: reused`
  with its evidence named, and stays in `cells`. Mutating the planner to drop
  reused cells instead — the PR #76 shape — fails three fixtures.
- [x] Preserve Linux and native-Windows compile coverage when macOS cells are
  reused, and model WSL compile coverage as the `ubuntu-latest` archive build,
  never as compilation inside the toolchain-free WSL guest.
  → `test_reusing_macos_keeps_linux_and_windows_compile_coverage` and
  `test_wsl_compile_coverage_is_the_ubuntu_archive_build`. The `wsl2-ubuntu`
  L1 cell reports `compile_coverage_from: "ubuntu-latest archive build"`; no
  `check` cell is ever emitted for it.
- [x] Replace blanket check generation with explicit library, binary, test,
  example, and benchmark target decisions. Treat L1 compilation as coverage
  for matching library/binary/test targets and schedule an additional check
  only for required target kinds not produced by that gate.
  → `declared_target_kinds()` reads the kinds from `cargo metadata`. L1 is
  credited with `lib`/`bin`/`test`; a `check` cell exists only where `example`
  or `bench` targets do. Full-scope job estimate fell from 486 to 432.
- [ ] **BLOCKED (B1).** Implement the selected OQ1 seam policy. If Option B is
  chosen, attach the Ubuntu-only direct-dependent check to the changed
  package's own compile cell, report the dependent package names/count, and
  never create a result identity for their unchanged areas.
  → No ruling is obtainable in a non-interactive session. The half of this
  task that holds under *every* option is done and tested: an unchanged
  dependent receives no area, package record, or cell (AC1), and its names are
  reported in `reverse_dependencies`. `dependent_seam` is absent from every
  package record, which is correct under Options A and C and is the field
  Option B fills.
- [x] Emit an area matrix whose entries contain applicable package targets and
  per-gate matrices, with a concrete selection reason for every area, package,
  environment, tier, and target kind. Retain the existing per-gate global-input
  widening rule and package metadata for features, native dependencies,
  backends, tools, and companion suites.
  → `areas[]`, `packages[]` (targets, tiers, features, native, backends,
  tools, companion suites), and `cells[]` (one per `{package, environment,
  gate}`). `validate_resolved_plan` rejects any area, package, or cell without
  a selection reason. `gate_triggers`/`just_change_gates` are untouched.
  Shaping cells into GitHub matrix objects is Phase 6's task.
- [x] Add the sniff drift contract: compare every workspace member's derived
  area with `sniff repo package-area` from its manifest directory and compare
  the resulting universe with `sniff repo package-areas` in local self-tests
  and preflight wherever sniff is present.
  → `test_every_workspace_members_area_matches_sniff` (all 73 members, an
  eight-way thread pool, ~9 s) plus the five-layout and universe-subset
  fixtures. Registered in `just/ci-local.just`'s self-test loop and already in
  `ci.yml`'s `preflight` and `ci-tooling` legs, where `skipUnless` skips it
  because hosted runners have no sniff.
- [x] **Validation checkpoint:** run `python3 scripts/ci/test_affected_scope.py`
  and inspect stable fixture JSON to prove AC1–AC3 and AC15, including matrix
  size and reusable-workflow depth estimates.
  → `test_affected_scope.py` 90/90, `test_resolved_plan.py` 23/23,
  `test_schema.py` 37/37, `test_ci_local.py` 13/13, `test_evidence_reuse.py`
  23/23, `test_local_evidence.py` 5/5, `test_reuse_validation.py` 15/15,
  `test_runner_loss.py` 15/15, the hook suite 13/13, `ci-rollup` 146/146,
  `ci_workflow_contracts` 67/67, `actionlint` clean on `ci.yml`. Full-scope
  plan: 432 jobs (was 486), matrix 73 packages, well under both the 256-entry
  and 1000-job ceilings. Reusable-workflow depth is untouched by this phase —
  it stays the three levels Phase 1 measured, with Phase 6 consuming the
  fourth.

## Phase 4 outcome (2026-09-11) — COMPLETE except the OQ2 store location

Eight of nine tasks are done; task 6 is the OQ2 store location and remains
blocked on Ken's B2 ruling. Every Phase 2 fixture that held a Phase 4 contract
has been promoted: `test_evidence_reuse.py` 17 pending → 0,
`test_ci_local.py` 4 → 0, the hook suite 3 → 0.

**Why Phase 4 could proceed while the plan is blocked**, checked the same way
Phases 2 and 3 checked it:

| Blocker | Effect on Phase 4 |
|---|---|
| B0 land-first vs absorb | **Absorbed, as in Phase 3.** `scope-only`, the `off` alias, and failure-path publication are *built* here rather than inherited. Nothing from the 2026-09-10 spec was implemented and then reversed. |
| B1 OQ1 seam | **None.** The verifier and the receipt say nothing about dependents. |
| B2 OQ2 constraint store | **Task 6's location only.** The record, the satisfaction rule, and both enforcement points are built and tested against an explicitly named directory, exactly as the Phase 2 fixtures were written. |
| B3 OQ3 merge gate | **None.** Nothing here names a merge mechanism. |
| B4 OQ4 gap conclusion | **None.** A governed gap remains the machine-readable `accepted-gap` state. |

**Four findings later phases need.**

- **A stored gate-input identity cannot be trusted, so it is not used for the
  decision.** Phase 2's own note required this ("a Phase 4 implementation that
  merely echoed the stored identity string would still fail them"). Both trees
  are present locally, so equivalence recomputes over each and compares. The
  receipt still records the identity as provenance.
- **The JUnit staging manifest is the receipt's source, and it accumulates.**
  `just _stage_junit` appends to `manifest.jsonl` and `_stage_junit_reset` only
  clears the *report*, so the default staging path holds every local run ever
  made. `ci-local` now points staging at a directory the run owns; publishing
  from the default path would have credited a passing cell to a tree that never
  ran it.
- **An L2 cell needs backend proof or it is worthless.** An absent backend makes
  the suite skip and nextest prints PASS in ~0.02 s. A cell is `complete` only
  when a backend it required appears in `test-toolkit`'s execution records;
  otherwise it is `partial` and stays scheduled. Phase 5's rollup must keep that
  distinction when it reads a local-origin cell.
- **`lint` and `check` can never come from a local receipt.** They stage no
  report, so they are always CI-origin. Phase 6 must not expect a local-origin
  lint cell.

**A narrowing that needs recording.** `scope-only` publishes no standalone
*scope* document. The note ref now carries a validation receipt, whose schema
requires at least one cell, and the separate scope-evidence document of the
2026-09-10 spec was never built. CI recalculates scope, as it does today.

## Phase 4 — Combine multi-environment evidence and enforce trigger constraints

- [x] Replace `verified_environment()` with verification that reads every
  `refs/notes/ci-local/<environment>` note, returns all accepted cells plus a
  rejection reason per rejected cell, and fails safe by scheduling work on
  missing, malformed, incompatible, stale, conflicting, or incomplete input.
  → `local_evidence.verify_cells(plan, base, head) -> (accepted, rejections)`.
  It walks `base..head` newest-first for every environment's notes ref, so
  macOS and a prior WSL cross-check combine in one answer.
  `verified_environment` survives only as the version-1 reader of spec 3.6.
- [x] Compute and store each cell's gate-input identity from a canonical,
  sorted representation of the build-closure `git ls-tree` entries (including
  dev-dependencies) and the existing gate-specific global inputs. Use Git's
  hashing boundary consistently and never restamp an older result for a new
  tree.
  → `gate_input_identity(paths, ref)` hashes the sorted `git ls-tree -r`
  entries; `gate_global_inputs(gate)` is derived from the planner's own
  per-gate classification (Design Decision 4), plus `Cargo.lock`. The plan
  carries each package's closure as `input_paths`. Equivalence recomputes over
  **both** trees rather than trusting the receipt's stored string — a stored
  identity is an unverified claim, and both trees are present locally.
- [x] Implement version-1 migration exactly as specified: exact-tree only,
  pass-only whole-environment reuse, no input-equivalence reuse, no in-place
  upgrade, and the visible measurement text `not recorded (v1 receipt)`.
  → `_accept_legacy`; a v1 note off the exact tree is rejected
  `v1-not-equivalence-eligible`. Nothing writes v1 any more.
- [x] Update local gate staging and `.githooks/pre-push` so `strict` and `warn`
  publish every complete passing or failing cell before returning, while
  interrupted or dirty-tree work publishes no reusable result; preserve
  `scope-only` and the deprecated `off` alias semantics from the prerequisite.
  → `ci-local` points the canonical tier recipes' JUnit staging at a directory
  the run owns (`BISCUIT_CI_REPORTS_OUT`), so the manifest describes that run
  and not every run since. `local_evidence record-cells` turns it into a
  version-2 receipt; the hook writes it before reporting the gate result. A
  gate with no report is recorded `partial` and is not reusable, so a compile
  failure cannot masquerade as a tested cell. `scope-only` resolves and prints
  the plan without running a gate, and `off` is its deprecated alias.
  **Narrowing:** `scope-only` publishes no standalone *scope* document. That
  half of the 2026-09-10 design was never built (see `prerequisite-audit.md`)
  and this specification re-keys the note to a validation receipt, whose schema
  requires at least one cell. CI recalculates scope, which is what it does now.
- [x] Update `scripts/cross-check.sh` to publish a normal `wsl2-ubuntu` receipt
  only after an archive-mode run against the exact outgoing tree with a clean
  remote worktree; emit a clear no-publication reason for dirty, patched, or
  mismatched runs.
  → The WSL leg prints `cross-check-tree:`, `-dirty:`, `-exit:`, and
  `-duration:` markers and copies its JUnit report back;
  `local_evidence cross-check` decides. A patched tree, a dirty worktree, a
  filtered run, a package outside the plan, or a missing report each print a
  specific reason and publish nothing. The report path is *searched*, not
  assumed: if nextest puts it somewhere else the run says so rather than
  fabricating a measurement.
- [ ] **BLOCKED (B2).** Implement the selected OQ2 persisted-constraint interface
  with owner, reason, affected environment/tier, branch/repository identity, and
  expiry; enforce it only at direct command, push, dispatch, and manual-retry
  trigger boundaries, never inside CI scheduling.
  → No ruling is obtainable in a non-interactive session, so the half that holds
  under every option is built and tested and the half that encodes the choice is
  not. `scripts/ci/constraints.py` implements the record (owner, reason,
  environment, optional gate, repository/branch identity, expiry), the
  satisfaction rule, and the refusal; `.githooks/pre-push` enforces it before
  any gate and `just ci-local --plan` exits non-zero on an unsatisfied one. The
  store is named by `BISCUIT_CI_CONSTRAINTS_DIR`; `constraints.default_directory()`
  is the one line OQ2 fills, and a fixture fails the day it is filled without
  the ruling being recorded.
- [x] Extend `just ci-local --plan` to show reused, executing,
  accepted-policy-gap, prohibited, and rejected-evidence cells and to exit
  nonzero when a prohibited cell would execute. Render the human-facing plan
  through a `TerminalRenderable` `Table`/`Prose` surface in the existing Rust
  repository tooling while keeping canonical JSON as the machine interface.
  → `scripts/ci-plan.rs`, a `local-tools` bin (`ci-rollup` still links none of
  the monorepo's crates). `--plan` uses it when it is already built and renders
  the same cells as plain lines otherwise, because a pre-trigger review must
  not start a build. `--plan-out` keeps the canonical JSON.
- [x] **Parallelizable after Phase 2:** evidence-schema and constraint-store
  implementation may proceed in separate files, but merge only after both
  consume the same resolved-plan schema and share rejection vocabulary.
  → Both read the resolved plan and neither invents a code: `constraints.py`
  reports refusals in prose at the trigger boundary, and every evidence refusal
  uses `schema.REJECTIONS`.
- [x] **Validation checkpoint:** run `python3 scripts/ci/test_local_evidence.py`,
  `python3 scripts/ci/test_ci_local.py`, and
  `bash .githooks/tests/test-pre-push.sh`; verify AC4–AC8, AC16, and AC17 without
  contacting a remote build host.
  → `test_local_evidence.py` 5/5, `test_ci_local.py` 17/17, the hook suite
  16/16 (`env -u CDPATH`), plus `test_evidence_reuse.py` 47/47,
  `test_constraints.py` 24/24, `test_resolved_plan.py` 33/33,
  `test_schema.py` 37/37, `test_affected_scope.py` 90/90,
  `test_reuse_validation.py` 15/15, `test_runner_loss.py` 15/15,
  `ci-rollup` 146/146, `ci-plan` 9/9, `ci_workflow_contracts` 72/72, and
  `actionlint` clean on `ci.yml`. No remote build host was contacted and no
  workflow was triggered. The AC evidence table is below.

### Phase 4 requirement-to-test mapping

| Criterion | Behavior | Targeted test |
|---|---|---|
| AC4 | A valid receipt supplies a completed cell with outcome, counts, duration, provenance | `ReceiptRecordingTests.test_a_passing_run_publishes_measured_cells_that_verify_back` |
| AC4 | No test runner starts for a reused cell | `PlanSurfaceTests.test_plan_runs_no_gate_and_builds_nothing`; the planner emits `execution: reuse` (`ProhibitionTests.test_evidence_satisfies_a_constraint_instead_of_violating_it`) |
| AC5 | macOS and WSL evidence reused together | `MultiEnvironmentTests.test_two_environments_are_accepted_together`, `CrossCheckPublicationTests.test_a_hook_receipt_and_a_cross_check_receipt_combine` |
| AC5 | A `cross-check` receipt qualifies like a hook one | `CrossCheckPublicationTests.test_an_exact_tree_clean_unfiltered_run_publishes_and_verifies` |
| AC6 | Excluding proven cells creates no `MISSING` | Phase 3's `ci-rollup-tests.rs` PR #76 fixture, unchanged; the planner keeps the cell (`test_resolved_plan.py`) |
| AC7 | Stale/malformed/partial evidence rejected with the reason | `RejectionTests` (3), `OutcomeTests` (2), `DuplicateEvidenceTests` (2) |
| AC7 | Gate-input identity accepts an unchanged older head and rejects a changed one | `GateInputIdentityTests` (5), `GateGlobalInputTests` (5) |
| AC7 | A prohibited environment with no evidence blocks the trigger | `CommandTests.test_an_active_constraint_blocks_and_names_itself`, hook `test_prohibition_blocks_the_push` |
| AC8 | A complete failure is evidence and stays a failure | `OutcomeTests.test_a_complete_failure_is_accepted_as_a_failed_cell`, `ReceiptRecordingTests.test_a_complete_failure_is_published_and_stays_a_failure` |
| AC8 | An incomplete gate never stands in for a CI execution | `ReceiptRecordingTests.test_a_gate_that_produced_no_report_is_not_reusable`, `BackendProofTests.test_a_backend_that_never_ran_makes_the_cell_unreusable` |
| AC16 | A v1 receipt is exact-tree, pass-only, unmeasured | `LegacyReceiptTests` (4) |
| AC17 | `--plan` shows every disposition | `PlanSurfaceTests.test_plan_shows_every_cell_disposition`, `ci-plan` `every_cell_appears_in_the_rendered_plan` / `every_disposition_is_visible` |
| AC17 | `--plan` exits non-zero on an unsatisfied prohibition | `PlanSurfaceTests.test_plan_exits_nonzero_when_a_prohibited_cell_is_unsatisfied` |
| AC17 | The rendered plan and the canonical JSON describe one plan | `PlanSurfaceTests.test_plan_writes_the_same_cells_it_rendered` |
| AC17 | Expiry bounds a constraint | `LoadTests.test_a_past_expiry_is_reported_and_stops_binding`, hook `test_expired_prohibition_does_not_block` |
| AC13 | Worker boundaries, L2 forwarding, and overrides preserved | `CiLocalTests`, `ThreadPolicyTests`, `L1ThreadForwardingTests` — unchanged and still green |

Passive corpus and round-trip coverage, as the test-design rules require:
`ClosurePathTests.test_every_declared_path_exists_in_the_checkout` walks every
selected package's declared closure in the real workspace;
`test_every_workspace_members_area_matches_sniff` (Phase 3) still covers all 73
members; `ReceiptRecordingTests.test_recording_the_same_run_twice_produces_identical_bytes`
is the write/read/write round trip; and every receipt fixture ends by feeding
the document back through `verify_cells`, so the writer and the reader are
pinned to each other rather than to a fixture's idea of the document.

**Pre-existing failure, untouched:** `cargo clippy --manifest-path
scripts/Cargo.toml --all-targets` cannot compile `scripts/drift.rs`, which calls
`fallback_render` on `Prose` and on `Rc<dyn TerminalRenderable>`; neither exists
in biscuit-terminal any more. `drift.rs` is unmodified by this phase
(`git diff HEAD -- scripts/drift.rs` is empty) and is not in the blast radius.
`ci-plan` and `ci-rollup` lint clean on their own.

## Phase 5 — Make result evaluation area-owned

- [ ] Extend `scripts/ci-rollup.rs` so one result model represents CI-origin,
  current local-origin, prior verified-origin, and accepted-gap cells with
  evidence links, test counts, durations, target coverage, and package detail.
- [ ] Add the distinct machine-readable `ACCEPTED GAP` state. Apply owner,
  reason, expiry, affected cell, policy link, revocation instructions, and
  closure-feature links from `.github/ci/environments.json`; never infer this
  state from a GitHub cancellation conclusion.
- [ ] Add an area scope to existing `ci-rollup rollup` and
  `ci-rollup verdict` commands so each area alone applies its baseline,
  accepted gaps, and missing-cell rule and emits its own result slice. Keep
  baseline and artifact keys package-based.
- [ ] Ensure complete local failures and CI failures block only their owning
  area while other areas/environments continue, and ensure missing/unexcused
  cells, expired gaps, and infrastructure loss also block that area's rollup.
- [ ] Move runner-loss attribution and the area-owned `ci-results` slices into
  the per-area path; keep retry classification based on explicit producer and
  stage evidence so an upload failure after passing tests is not called a test
  failure.
- [ ] Implement accepted-gap check-run publication using the OQ4 conclusion
  proved in Phase 1, with details linked to the policy entry, while keeping the
  enclosing workflow conclusion unchanged.
- [ ] Make the combined run summary a pure aggregation of area result slices;
  it may render and link evidence but must not apply baseline, gap, missing, or
  merge policy again.
- [ ] **Validation checkpoint:** run
  `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`
  and `python3 scripts/ci/test_runner_loss.py`; prove AC6, AC8, AC9, AC11, and
  the versioned result-schema migration.

## Phase 6 — Restructure workflow scheduling and presentation

- [ ] Make `.github/workflows/ci.yml` fan out one caller identity per selected
  area from the canonical area matrix, passing each area's package and per-gate
  matrices to an area-level reusable workflow that delegates package execution
  to `_package-ci.yml` and WSL archive execution to `_wsl-ci.yml`.
- [ ] Hoist gate/tier selection into planner-generated matrices so only actual
  work becomes a matrix job. Give any empty whole stage one static,
  human-readable skip identity and ensure no unresolved expression can reach a
  job or check label.
- [ ] Schedule hosted runners only for cells not satisfied by verified evidence;
  publish reused cells immediately into their area summary/check presentation
  without invoking setup, build, archive, or test steps.
- [ ] Add one `if: always()` rollup job per selected area, dependent on that
  area's producers, and have it combine hosted cells, local-origin cells, and
  accepted gaps before applying only that area's policy.
- [ ] Keep infrastructure/contract work clearly separate from tested areas.
  Make advisory jobs unable to fail the overall run, while required
  infrastructure failures remain visible and blocking through their owning
  check.
- [ ] Preserve package features, native dependency closure, backend proof,
  companion suites, WSL archive-only execution, and the worker policy of all
  cores at four or fewer in CI and `cores - 2` above four; keep shared-resource
  L2 serial and explicit overrides authoritative.
- [ ] Assert the expanded job count remains within GitHub limits and the
  reusable-workflow chain remains within four levels for both affected-area and
  explicit full-scope plans.
- [ ] **Validation checkpoint:** run
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`, run
  `actionlint` on every changed workflow, and replay the controlled presentation
  fixture to prove AC2, AC3, AC4, AC5, AC9, AC10, and AC13.

## Phase 7 — Migrate merge authority atomically

- [ ] Re-prove the selected OQ3 mechanism in the scratch repository: a failed
  selected area blocks, an unselected area creates no pending requirement,
  missing required coverage blocks, accepted gaps do not fail the workflow,
  and a successful run satisfies the exact ruleset configuration to be used.
- [ ] Prepare a compatibility revision in the implementation PR that has the
  new per-area path while temporarily retaining the old `ci-verdict`; get the
  PR's own run green under the old required context before changing repository
  rules.
- [ ] With explicit authorization, replace `ci-verdict` in
  `protect-your-bacon` (ruleset 19747338) with the proven required-workflow
  mechanism, or the proven policy-free conjunction fallback, and immediately
  verify the PR has neither a stale pending context nor an unprotected failure
  path.
- [ ] Remove the standalone `ci-verdict` job and every dependency, comment, and
  contract that treats it as global policy authority. If the fallback
  conjunction is used, constrain it to a pure fold of area job conclusions;
  all baseline, gap, and missing-cell decisions remain inside area rollups.
- [ ] Rewire `ci-infra-retry.yml`, `reuse_validation.py`, `release-plz.yml`, and
  `pr-health.yml` to the new run-conclusion/result locations; verify validation
  reuse accepts only successful equivalent runs and rejects failure,
  cancellation, changed inputs, and missing evidence.
- [ ] Confirm the `ci-results` artifact remains complete for reporting and
  future reuse and that the workflow run conclusion is exactly the conjunction
  consumed by branch protection, validation reuse, and release automation.
- [ ] **Validation checkpoint:** rerun the scratch ruleset cases, focused reuse
  tests, workflow contracts, and the implementation PR's CI; do not complete
  this phase while either the obsolete check can block all PRs or incomplete
  coverage can merge.

## Phase 8 — Align documentation and run focused local validation

- [ ] Update `.github/ci/README.md`, `docs/topics/ci-cd.md`, `CLAUDE.md`,
  `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/os/`, workflow
  comments, and the 2026-09-09 memory note to describe area grouping versus
  package identity, multi-environment evidence, target coverage, constraints,
  accepted gaps, and the actual merge gate.
- [ ] Update `ci-baseline.toml` commentary and policy links without re-keying
  package identities or changing accepted failures unrelated to this fix.
- [ ] Review every edited `///`, `//!`, and inline `//` comment against the new
  behavior; remove verdict-era and single-environment drift while avoiding
  unrelated comment cleanup.
- [ ] Run the focused Python suites for affected scope, local evidence,
  `ci-local`, validation reuse, and runner loss; run the hook shell suite,
  rollup Nextest suite, workflow-contract Nextest suite, and `actionlint`.
- [ ] Run `just ci-local --plan` for representative source, nested-area,
  infrastructure-only, mixed-evidence, prohibited-environment, and explicit
  full-scope fixtures; verify its terminal output and canonical JSON describe
  the same cells.
- [ ] Do not run `cargo fmt`; use the repository's formatting-check path only
  where it is already part of an authorized focused lint gate.
- [ ] Run GitNexus `detect_changes(scope: compare, base_ref: main)` before any
  authorized commit and reconcile every changed symbol/process and file with
  this plan; investigate any partial result or unexpected execution flow.
- [ ] **Validation checkpoint:** produce an AC1–AC17 evidence table linking each
  criterion to a focused test, fixture result, or explicitly pending live-CI
  observation.

## Phase 9 — Roll out with authorized evidence and verify end to end

- [ ] Generate the final pre-trigger plan for the actual outgoing head and
  review every cell's area, package, environment, gate, origin, state, evidence,
  and action against all active execution constraints.
- [ ] Run only explicitly authorized impacted-area validation on available
  hosts, retain full reports under the documented evidence location, and
  publish exact receipts. Do not rerun WSL; reuse a qualifying prior receipt or
  stop and report the unsatisfied plan.
- [ ] Trigger CI only when no prohibited cell remains scheduled and the
  ruleset/workflow migration sequence from Phase 7 is safe for all open PRs.
- [ ] Inspect the resulting Actions graph and Checks tab: one top-level identity
  per selected area, package detail underneath, visible reused and accepted-gap
  cells, environment-qualified compile/lint labels, no unresolved expressions,
  and no standalone global policy verdict.
- [ ] Download and inspect the scope and result artifacts; prove they contain
  the same accepted evidence set used for scheduling, every required cell is
  present or explicitly governed, and failures/missing results block the
  correct area.
- [ ] Verify successful PR-validation reuse and the release-plz trigger against
  the new workflow conclusion without publishing a release or changing package
  distribution policy.
- [ ] Record local execution time separately from CI queue, setup, build, test,
  archive, and artifact-publication time; record per-cell test counts/durations
  and never attribute aggregate local duration to one area.
- [ ] **Final validation checkpoint:** close the AC1–AC17 evidence table, retain
  the GitHub fixture/run/ruleset links, confirm all documentation matches live
  behavior, and mark the fix complete only when no criterion depends on an
  unverified assumption.

## Critical path

Phase 1 decisions and prerequisite evidence gate all implementation. Phase 2
freezes interfaces; Phases 3 and 4 then produce the single resolved plan and
evidence set consumed by Phase 5. Phase 6 wires those contracts into GitHub.
Phase 7 is an atomic operational migration, after which documentation and
focused validation in Phase 8 lead to the authorized live rollout in Phase 9.


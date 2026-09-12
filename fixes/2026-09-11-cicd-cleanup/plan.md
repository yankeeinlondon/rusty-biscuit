---
title: CI cleanup — impacted areas, reusable evidence, and area-owned outcomes
status: blocked
created: 2026-09-11
phase: 9
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
source_files_during_phase_5:
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci/runner_loss.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/ci/environments.json
  - .github/workflows/ci.yml
docs_created_during_phase_5: []
docs_updated_during_phase_5:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_5:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_6:
  - .github/workflows/_area-ci.yml
  - .github/workflows/ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_created_during_phase_6: []
docs_updated_during_phase_6:
  - .github/ci/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_6:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_7:
  - scripts/ci/runner_loss.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_ci_local.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/ci-infra-retry.yml
  - .github/workflows/pr-health.yml
  - just/ci-local.just
docs_created_during_phase_7: []
docs_updated_during_phase_7:
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_7:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_8:
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - scripts/ci-rollup.rs
  - scripts/ci/affected_scope.py
  - scripts/ci/runner_loss.py
  - scripts/ci/schema.py
  - just/ci-local.just
  - .githooks/pre-push
  - .github/workflows/ci.yml
  - .github/workflows/_package-ci.yml
  - .github/ci/ci-baseline.toml
docs_created_during_phase_8: []
docs_updated_during_phase_8:
  - docs/topics/ci-cd.md
  - CLAUDE.md
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_8:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/ci-runners.md
source_files_during_phase_9: []
docs_created_during_phase_9:
  - fixes/2026-09-11-cicd-cleanup/rollout-2026-09-11.md
docs_updated_during_phase_9:
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_9: []
source_code:
  - scripts/ci/schema.py
  - scripts/ci/pending_contracts.py
  - scripts/ci/affected_scope.py
  - scripts/ci/local_evidence.py
  - scripts/ci/constraints.py
  - scripts/ci/runner_loss.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_constraints.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci-plan.rs
  - scripts/ci-plan-tests.rs
  - scripts/Cargo.toml
  - scripts/cross-check.sh
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .githooks/pre-push
  - .githooks/tests/test-pre-push.sh
  - .github/workflows/ci.yml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/ci-infra-retry.yml
  - .github/workflows/pr-health.yml
  - .github/ci/schemas/contract.json
  - .github/ci/environments.json
  - .github/ci/ci-baseline.toml
  - just/ci-local.just
documentation:
  - fixes/2026-09-11-cicd-cleanup/plan.md
  - fixes/2026-09-11-cicd-cleanup/prerequisite-audit.md
  - fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md
  - fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md
  - fixes/2026-09-11-cicd-cleanup/rollout-2026-09-11.md
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - docs/topics/ci-cd.md
  - CLAUDE.md
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/ci-runners.md
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

**Closed by review-1.** The standalone *scope* receipt of the 2026-09-10 spec
now exists: `local_evidence.py scope-record` / `scope-verify`,
`schema.validate_scope_receipt`, and `refs/notes/ci-local/scope`. Every hook
mode publishes it before the gates from the committed `base..head`, and
`ci.yml`'s scope job takes a receipt matching the event's exact
`{base, head, tree}` as its plan without running the planner, recalculating
only on a coded miss.

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

## Phase 5 outcome (2026-09-11) — COMPLETE except the OQ4 check-run publication

Seven of eight tasks are done; task 6 is the OQ4 conclusion and remains blocked
on Ken's B4 ruling and the B5 fixture. The rollup now reads the resolved plan,
so a cell a receipt satisfied is reported as a completed local-origin result
instead of `MISSING` — the PR #76 regression is closed in the live workflow, not
only in a fixture.

**Why Phase 5 could proceed while the plan is blocked**, checked the same way
Phases 2, 3, and 4 checked it:

| Blocker | Effect on Phase 5 |
|---|---|
| B0 land-first vs absorb | **Absorbed, as before.** The per-cell result set is consumed here, not inherited. |
| B1 OQ1 seam | **None.** The result model says nothing about dependents; an unchanged one still has no cell to report. |
| B2 OQ2 constraint store | **None.** A prohibited cell arrives labelled in the plan; the rollup reports it as missing coverage and names the constraint, wherever the store lives. |
| B3 OQ3 merge gate | **None.** `--area` narrows a verdict; which verdict branch protection consults is Phase 7's. |
| B4 OQ4 gap conclusion | **Task 6 only.** The machine-readable `ACCEPTED GAP` state, its governance detail, and the rule that it is never inferred from a GitHub conclusion are built and tested; the GitHub *conclusion* used to display it is not. |

**Four findings later phases need.**

- **A cell the plan reused and CI also executed must report the execution.** The
  fixture existed only because the two documents can disagree; when they do, the
  result with a report behind it is the honest one, and the disagreement is
  stated in the cell's reasons. Phase 6 must not treat a reused cell as a
  guarantee that no job will run for it.
- **`lint` and `check` cells cannot come from the JUnit walker.** They stage no
  report, so their state comes from the producer status. Reading the plan
  changed this from "a status with no expectation" to "an expectation that may
  have no status", which is how a lint job that never reported became `MISSING`
  rather than absent. Phase 6's scheduling must keep uploading those statuses.
- **A `gates = false` package owns no plan cells.** Its governed `NOT SCHEDULED`
  entries come from the legacy policy document, which is the only place its
  owner and expiry live. Phase 6 cannot delete that document without moving the
  exclusion metadata into the plan first.
- **The result document is now `schema_version: 3`, and `just ci-diff` compares
  against `main`'s artifact.** Until `main` runs the new rollup, that comparison
  fails with the migration error by design — the same cost the 1→2 re-key paid.

**Counts.**

| Suite | Before | After |
|---|---:|---:|
| `scripts/ci-rollup-tests.rs` | 146 | 181 |
| `scripts/ci/test_runner_loss.py` | 15 | 23 |
| `tools/test-toolkit/tests/ci_workflow_contracts.rs` | 72 | 72 (one assertion extended) |

### Phase 5 requirement-to-test mapping

| Criterion | Behavior | Targeted test |
|---|---|---|
| AC6 | Seven proven macOS cells are not `MISSING` | `the_pr_76_macos_cells_must_not_resolve_to_missing` |
| AC6 | The defect itself still reproduces without a plan | `the_pr_76_fixture_reproduces_the_regression_exactly`, `the_pr_76_hosted_cells_still_pass` |
| AC6 | Reusing macOS does not relabel what did run | `the_pr_76_hosted_cells_stay_ci_origin_when_the_plan_is_read` |
| AC4 | A reused cell carries origin, counts, duration, evidence link, host, report | `a_reused_cell_carries_the_measurements_its_receipt_recorded`, `a_result_cell_reports_where_its_result_came_from` |
| AC4 | The document records the evidence it was scheduled against | `the_result_document_records_the_evidence_it_was_scheduled_against` |
| AC4 | A prior-local receipt is a weaker claim and says so | `a_prior_local_receipt_is_distinguishable_from_the_outgoing_head` |
| AC16 | A v1 receipt renders `not recorded (v1 receipt)` | `a_version_one_receipt_renders_its_measurements_as_unrecorded` |
| AC8 | A complete failed local result stays a failure and blocks | `a_complete_failed_local_result_stays_a_failure` |
| AC8 | A failure blocks its own area and no other | `a_failed_area_blocks_only_its_own_slice`, `another_areas_baseline_entry_is_out_of_scope_in_this_slice` |
| AC8 | Missing and prohibited-unsatisfied cells block their area | `a_missing_cell_blocks_its_own_area`, `a_prohibited_cell_is_missing_and_names_the_constraint` |
| AC8 | An executed result wins over a contested reuse | `a_reused_cell_that_also_produced_ci_evidence_reports_the_executed_result` |
| AC9 | `ACCEPTED GAP` is a distinct machine-readable state | `accepted_gap_is_a_distinct_machine_readable_state` |
| AC9 | Owner, expiry, policy link, closure, revocation travel with the cell | `an_accepted_gap_carries_its_owner_expiry_policy_and_closure`, `the_grid_explains_an_accepted_gap_where_a_reader_sees_it` |
| AC9 | Accepted does not block; expired, ungoverned, and unattributed do | `an_accepted_gap_is_neither_a_pass_nor_a_failure_and_does_not_block`, `an_expired_accepted_gap_blocks_its_area`, `an_ungoverned_gap_is_never_accepted_however_the_plan_labels_it`, `an_accepted_gap_state_with_no_policy_entry_blocks` |
| AC9 | Never inferred from a cancellation; never suppresses evidence | `a_cancelled_job_is_not_an_accepted_gap`, `a_real_failure_outranks_an_accepted_gap` |
| AC11 | `--area` narrows cells, scope, scheduled set, and evidence together | `narrowing_keeps_only_the_areas_own_cells_scope_and_evidence`, `the_rollup_and_verdict_commands_accept_an_area_scope` |
| AC11 | The combined summary applies no policy | `the_combined_summary_aggregates_area_slices_and_applies_no_policy` |
| AC2 | Area groups the grid; package stays the identity | `the_grid_groups_cells_under_their_area`, `package_stays_the_stored_identity_of_every_cell`, `a_result_cell_carries_its_area_alongside_its_package` |
| AC3 | Each cell says which target kinds its compile coverage came from | `a_cell_reports_the_target_kinds_and_compile_coverage_the_plan_gave_it`, `a_status_gate_reports_the_target_coverage_the_plan_scheduled_it_for` |
| — | Result and baseline schemas version independently; an older document is refused | `a_result_document_from_the_previous_generation_is_refused`, `the_result_and_baseline_schemas_version_independently`, `a_plan_from_another_generation_is_refused_rather_than_partly_read` |
| §4 | An upload failure after passing tests is not a test failure | `FailureStageTests.test_an_upload_failure_after_passing_tests_is_not_a_test_failure`, `…test_the_stage_reaches_the_retry_reason` |
| §5 | Runner-loss attribution narrows to one area's packages | `SynthesizeStatusTests.test_an_area_synthesizes_only_its_own_packages` |

Passive corpus and end-to-end coverage, as the test-design rules require:
`plan_fields_match_the_frozen_contract` asserts every plan field this tool reads
against the shipped `.github/ci/schemas/contract.json`, including the origin and
accepted-gap vocabularies; `the_shipped_capability_table_names_what_closes_its_l2_gaps`
walks the real `environments.json` and proves both named features exist;
`the_real_planners_plan_rolls_up` runs the shipped `affected_scope.py` through
its normal invocation and feeds its own output to the plan reader;
`the_command_surface_writes_reads_and_judges_one_areas_slice` goes through the
CLI — `rollup --plan --area --out`, then `verdict --results --area`, then
`summarize` — and includes the write/read/write round trip (rolling the same run
up twice produces identical bytes);
`a_non_gating_packages_governed_cells_survive_the_plan_path` proves the
`gates = false` governance is not lost by the new input path.

**Nothing was skipped, and no pre-existing failure was introduced.** The
`scripts/drift.rs` clippy break recorded in Phase 4 is unchanged and still
unrelated: `ci-rollup` and `ci-plan` lint clean on their own.

## Phase 5 — Make result evaluation area-owned

- [x] Extend `scripts/ci-rollup.rs` so one result model represents CI-origin,
  current local-origin, prior verified-origin, and accepted-gap cells with
  evidence links, test counts, durations, target coverage, and package detail.
  → The rollup now reads the canonical resolved plan (`--plan`). Every cell
  carries `area`, `origin`, `evidence`, `duration_s`, `target_kinds`, and
  `compile_coverage_from`; the document carries `accepted_evidence`. Result
  schema 3, versioned independently of the baseline's 2.
- [x] Add the distinct machine-readable `ACCEPTED GAP` state. Apply owner,
  reason, expiry, affected cell, policy link, revocation instructions, and
  closure-feature links from `.github/ci/environments.json`; never infer this
  state from a GitHub cancellation conclusion.
  → `CellState::AcceptedGap`, taken from the plan's `accepted-gap` state and
  never from a GitHub conclusion. `environments.json` gained `closes` on the
  four Windows and four WSL2 L2-backend gaps; a corpus test proves both named
  features exist.
- [x] Add an area scope to existing `ci-rollup rollup` and
  `ci-rollup verdict` commands so each area alone applies its baseline,
  accepted gaps, and missing-cell rule and emits its own result slice. Keep
  baseline and artifact keys package-based.
  → `--area` on both, implemented as `Rollup::narrowed`, which narrows cells,
  scope, scheduled set, and accepted evidence together. Baseline and artifact
  keys are untouched.
- [x] Ensure complete local failures and CI failures block only their owning
  area while other areas/environments continue, and ensure missing/unexcused
  cells, expired gaps, and infrastructure loss also block that area's rollup.
  → A reused cell whose receipt recorded a complete failure is `FAIL`;
  narrowing means another area's cells and baseline entries cannot block or
  excuse this one. Missing cells, prohibited-and-unsatisfied cells, expired
  gaps, and ungoverned gaps all block their own slice.
- [x] Move runner-loss attribution and the area-owned `ci-results` slices into
  the per-area path; keep retry classification based on explicit producer and
  stage evidence so an upload failure after passing tests is not called a test
  failure.
  → `runner_loss.py attribute --package` narrows synthesis to one area's
  packages (package, not area: a job name carries no area). Every non-lost
  failure now records the step that failed and its `stage`, and the retry
  reason names it, so passing tests followed by a failed upload read as
  `artifact-upload`, never as a test failure. The area-owned slice is
  `ci-rollup rollup --area … --out`; the per-area *jobs* that call it are
  Phase 6's, and this phase leaves `ci-verdict` running unnarrowed.
- [ ] **BLOCKED (B4/B5).** Implement accepted-gap check-run publication using
  the OQ4 conclusion proved in Phase 1, with details linked to the policy entry,
  while keeping the enclosing workflow conclusion unchanged.
  → OQ4 (`cancelled` versus `neutral`) is unruled and cannot be closed until the
  B5 fixture runs; no job holds `checks: write`, and granting it is part of the
  mechanism being chosen. The half that holds under every option is built and
  tested: the distinct `ACCEPTED GAP` state, its owner/reason/expiry/policy
  link/closure/revocation detail, and the rule that it never comes from a GitHub
  conclusion. `the_gap_publishing_job_holds_the_checks_write_permission_it_needs`
  stays pending and fails the day the permission lands without the ruling.
- [x] Make the combined run summary a pure aggregation of area result slices;
  it may render and link evidence but must not apply baseline, gap, missing, or
  merge policy again.
  → `ci-rollup summarize --results <slice>…` counts the states each area's own
  rollup recorded, links the evidence behind reused cells, and always exits 0.
  `the_combined_summary_aggregates_area_slices_and_applies_no_policy` fails if
  a verdict word or a finding rule ever reaches its output.
- [x] **Validation checkpoint:** run
  `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`
  and `python3 scripts/ci/test_runner_loss.py`; prove AC6, AC8, AC9, AC11, and
  the versioned result-schema migration.
  → `ci-rollup` 181/181 (was 146), `test_runner_loss.py` 23/23 (was 15),
  `ci_workflow_contracts` 72/72, `actionlint` clean on `ci.yml`, and the seven
  other Python CI suites unchanged and green. The requirement-to-test mapping is
  below.

## Phase 6 outcome (2026-09-11) — COMPLETE except the B5 live replay

All seven implementation tasks are done. The validation checkpoint is met
except for its last clause: the controlled presentation fixture is B5, which is
still unauthorized, so every criterion below is proven by static contract
rather than by a live run.

`ci.yml` now fans out `area-ci` over the planner's `scheduled_areas` into a new
`_area-ci.yml`, which fans out that area's packages into `_package-ci.yml`,
which still delegates the WSL2 cell to `_wsl-ci.yml`. Each area owns an
`if: always()` `rollup` job that runs `ci-rollup rollup --area` and
`verdict --area` and narrows `runner_loss.py attribute --package` to its own
packages.

**Why Phase 6 could proceed while the plan is blocked**, checked the same way
Phases 2–5 checked it:

| Blocker | Effect on Phase 6 |
|---|---|
| B0 land-first vs absorb | **Absorbed, as before.** The area matrix is built here from the plan Phase 3 produced. |
| B1 OQ1 seam | **None.** An unchanged dependent still has no package record, so it reaches no area matrix. |
| B2 OQ2 constraint store | **None.** Constraints bind the trigger; `ci_never_reads_the_execution_constraint_store` now covers `_area-ci.yml` too. |
| B3 OQ3 merge gate | **Deferred intact.** `ci-verdict` is retained and still required; the area rollups run beside it. That is exactly the compatibility revision Phase 7 task 2 asks for. |
| B4 OQ4 gap conclusion | **None.** An accepted gap reaches the reader through the area rollup's grid, which is the machine-readable state, not a GitHub conclusion. |
| B5 presentation fixture | **The checkpoint's last clause only.** The GitHub behaviors the design leans on are named below with how each is evidenced. |

**Four findings later phases need.**

- **Dropping `name:` is the fix for AC10, not renaming it.** GitHub does not
  evaluate the matrix context for a job it skips, so *any* `name:` holding
  `${{ matrix.… }}` reaches the Checks tab as raw text. Omitting `name:` makes
  the label the job id when skipped and `job-id (matrix values)` when it runs —
  static and informative at once. `lint` is the exception: it has no matrix, so
  it keeps the static `lint (ubuntu-latest)` that AC10's environment clause
  requires. Six job ids were renamed to carry the reader-facing wording that
  used to live in `name:` (`l2` → `test-l2`, `browser` → `test-browser`,
  `wsl` → `wsl2`).
- **A reusable workflow's token cannot exceed its caller's.** The area rollup
  needs `actions: read` and `checks: read`, so those had to be granted on
  `ci.yml`'s `area-ci` job as well as inside `_area-ci.yml`. Granting them only
  in the called workflow silently yields the caller's narrower set.
- **The chain now sits exactly at four levels.** `the_reusable_workflow_chain_stays_within_githubs_four_levels`
  no longer hard-codes the edges: it walks every `uses: ./.github/workflows/…`
  edge from `ci.yml`, detects cycles, and asserts the depth is exactly four. A
  fifth level cannot be added without that test failing first.
- **A nested area name is not a legal artifact name.** `claudine/rendezvous`
  would fail its upload, so the planner emits `area_slugs` (`/` → `--`) and
  only the artifact *name* uses it. Nothing stored is re-keyed by area.

**AC10's remaining honest limit.** A gate a package does not declare still
produces one skipped job (`test-l2` for a package with no L2 tier), because the
gate `if:` lives inside `_package-ci.yml` and GitHub errors on an empty matrix
vector. AC10 permits exactly this — "a stage skipped as a whole may appear once
under a static, human-readable name" — and the name is now static. The half
that matters for cost is done in the planner: `native_environments`,
`check_os`, `l2_environments`, and `browser_environments` are the *executing*
set, so no runner is scheduled for a cell a receipt satisfied.

**Counts.**

| Suite | Before | After |
|---|---:|---:|
| `tools/test-toolkit/tests/ci_workflow_contracts.rs` | 72 | 76 |
| `scripts/ci/test_affected_scope.py` | 90 | 100 |

**Full-scope size, measured on this checkout:** 28 area jobs, largest area 7
packages (`homelab`), 432-job estimate — against GitHub's 256-entry matrix
ceiling and the 1000-job run ceiling.

### Phase 6 requirement-to-test mapping

| Criterion | Behavior | Targeted test |
|---|---|---|
| AC2 | Area is the top-level identity; `ci.yml` declares no package fan-out | `the_area_is_the_top_level_identity_of_the_package_fan_out` |
| AC2 | Package stays the identity underneath the area | same test's `_area-ci.yml` half; `test_every_matrix_entry_appears_under_exactly_one_area` |
| AC2 | A nested area is its own entry, never folded into its parent | `test_areas_group_their_packages_and_are_sorted`, `test_a_nested_area_fans_out_beside_its_parent_never_inside_it` |
| AC2 | Both matrices are planner-derived, never static | `the_package_matrix_is_scope_derived_not_static` |
| AC3 | Target/compile coverage forwarding survives the restructure | `the_reusable_workflow_invokes_the_canonical_recipes`, `messenger_policy_and_matrix_contract_are_promoted` |
| AC4 | A reused cell is published with no setup, build, archive, or test step | `a_reused_cell_reaches_its_area_summary_without_being_re_executed` |
| AC5 | The scope job still verifies per cell and hands the accepted set to the planner | `the_scope_job_consumes_verified_cells_not_one_environment` (unchanged, still green) |
| AC9 | An accepted gap reaches the reader through its area's grid | `every_selected_area_owns_an_always_rollup_job` plus Phase 5's grid fixtures |
| AC9 | Check-run publication | **pending** — `the_gap_publishing_job_holds_the_checks_write_permission_it_needs` (B4/B5) |
| AC10 | No skippable job carries an unresolved expression | `no_skippable_job_is_labelled_with_an_unresolved_expression` (promoted; the defect-pinning fixture is deleted) |
| AC10 | A skippable job still names something actionable | `every_skippable_job_has_a_static_human_readable_identity` |
| AC10 | Lint's environment is visible; check takes its own from the matrix | `lint_and_check_labels_identify_their_environment` (promoted) |
| AC11 | Each area owns an `if: always()` rollup that applies only its policy | `every_selected_area_owns_an_always_rollup_job` |
| AC11 | Runner-loss attribution narrows to the owning area | `runner_loss_attribution_is_narrowed_to_the_owning_area` |
| AC11 | Advisory jobs cannot fail the run; gates are not advisory | `advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory` |
| AC11 | `ci-verdict` removal | **pending** — `no_standalone_global_verdict_job_remains` (Phase 7) |
| AC13 | Worker policy, backend proof, companion suites preserved | `the_worker_policy_survives_the_area_restructure`, `CiLocalTests`/`ThreadPolicyTests`/`L1ThreadForwardingTests` unchanged |
| §2 | The chain stays within four levels | `the_reusable_workflow_chain_stays_within_githubs_four_levels` (now walks the edges) |
| §2 | Job/matrix counts stay under GitHub's ceilings, full scope included | `test_full_scope_stays_within_githubs_matrix_and_job_ceilings`, `test_an_affected_area_plan_is_far_under_the_full_scope_ceilings` |
| — | Every area has a slug, and no two share one | `test_every_shipped_area_slug_is_a_legal_artifact_name`, `test_a_nested_area_slug_is_a_legal_artifact_name` |
| — | A `gates = false` package creates no area fan-out | `test_a_gates_false_package_gets_no_area_fan_out` |
| — | The rollup binary still links none of the monorepo crates, in both callers | `the_rollup_binary_is_still_built_without_the_monorepo_crates` |

Passive corpus and end-to-end coverage, as the test-design rules require:
`RealWorkspaceAreaFanOutTests` runs the shipped planner over every workspace
member for both an affected-area and an explicit full-scope plan;
`every_cargo_workflow_neutralizes_a_stray_rustc_wrapper` and
`required_ci_honors_the_toolchain_file_without_stable_override` walk every
workflow file including the new one; `every_skippable_job_has_a_static_human_readable_identity`
walks every job in all four reader-facing workflows rather than a named list;
and `actionlint` parses every shipped workflow.

**Nothing was skipped, and no pre-existing failure was introduced.** The
`scripts/drift.rs` clippy break recorded in Phase 4 is unchanged and still
unrelated; `ci-rollup`, `ci-plan`, and `test-toolkit` lint clean on their own.
The root `just lint` was **not** run to completion: it lints the whole
monorepo, which is outside this phase's blast radius and too slow for it.

## Phase 6 — Restructure workflow scheduling and presentation

- [x] Make `.github/workflows/ci.yml` fan out one caller identity per selected
  area from the canonical area matrix, passing each area's package and per-gate
  matrices to an area-level reusable workflow that delegates package execution
  to `_package-ci.yml` and WSL archive execution to `_wsl-ci.yml`.
  → `ci.yml`'s `area-ci` fans out over `scheduled_areas` and calls the new
  `_area-ci.yml` with that area's `area_matrix` slice; `_area-ci.yml` fans out
  the packages into `_package-ci.yml`, which still delegates WSL to
  `_wsl-ci.yml`. The planner emits `scheduled_areas`, `area_matrix`, and
  `area_slugs`, so the grouping is tested rather than assembled in workflow
  `jq`.
- [x] Hoist gate/tier selection into planner-generated matrices so only actual
  work becomes a matrix job. Give any empty whole stage one static,
  human-readable skip identity and ensure no unresolved expression can reach a
  job or check label.
  → Every environment list the area hands a package is the plan's *executing*
  set, so a reused or governed cell schedules no runner. Every skippable job
  now carries a static label: `name:` is dropped where a matrix supplies the
  detail, and six job ids were renamed to carry the wording it used to hold.
  `no_skippable_job_is_labelled_with_an_unresolved_expression` is promoted out
  of its pending wrapper and the fixture that pinned the defect is deleted.
  **Limit, stated rather than hidden:** a gate a package does not declare still
  produces one skipped job, which is the case AC10 explicitly allows.
- [x] Schedule hosted runners only for cells not satisfied by verified evidence;
  publish reused cells immediately into their area summary/check presentation
  without invoking setup, build, archive, or test steps.
  → The scheduling half is the planner's executing set above. The publication
  half is the area rollup's step summary: `render_grid`'s "Reused results"
  table names each receipt's evidence ref, measurements, and host, and
  `a_reused_cell_reaches_its_area_summary_without_being_re_executed` fails if
  the rollup job ever grows a test, nextest, or build step.
- [x] Add one `if: always()` rollup job per selected area, dependent on that
  area's producers, and have it combine hosted cells, local-origin cells, and
  accepted gaps before applying only that area's policy.
  → `_area-ci.yml`'s `rollup`: `needs: package-ci`, `if: always()`, reading the
  plan and the policy artifacts and running `ci-rollup rollup --area` then
  `verdict --area`. It publishes `ci-results-<slug>` and fails closed.
- [x] Keep infrastructure/contract work clearly separate from tested areas.
  Make advisory jobs unable to fail the overall run, while required
  infrastructure failures remain visible and blocking through their owning
  check.
  → Tested areas are exactly the `area-ci` fan-out; `validation`, `scope`,
  `preflight`, `ci-tooling`, and `biscuit-tui-captured-stdout` keep their own
  non-area identities and stay blocking. The one advisory job is renamed
  `infrastructure summary (advisory)` and carries `continue-on-error: true`;
  `advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory` also asserts
  the converse, that no gate opts out of blocking.
- [x] Preserve package features, native dependency closure, backend proof,
  companion suites, WSL archive-only execution, and the worker policy of all
  cores at four or fewer in CI and `cores - 2` above four; keep shared-resource
  L2 serial and explicit overrides authoritative.
  → No step inside `_package-ci.yml` or `_wsl-ci.yml` changed; only job ids and
  labels did. `the_worker_policy_survives_the_area_restructure` pins
  `_test_threads`, the `l2-parallel-self-spawn` gate on `BISCUIT_L2_THREADS`,
  `BISCUIT_TEST_REQUIRED_BACKENDS`, and the companion suites, and
  `test_ci_local.py`'s 17 cases are unchanged and green.
- [x] Assert the expanded job count remains within GitHub limits and the
  reusable-workflow chain remains within four levels for both affected-area and
  explicit full-scope plans.
  → `test_full_scope_stays_within_githubs_matrix_and_job_ceilings` and
  `test_an_affected_area_plan_is_far_under_the_full_scope_ceilings` run the
  shipped planner over the real workspace: 28 area jobs, largest area 7
  packages, 432-job estimate. `the_reusable_workflow_chain_stays_within_githubs_four_levels`
  now walks the `uses:` graph and asserts the depth is exactly four.
- [x] **Validation checkpoint:** run
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`, run
  `actionlint` on every changed workflow, and replay the controlled presentation
  fixture to prove AC2, AC3, AC4, AC5, AC9, AC10, and AC13.
  → `ci_workflow_contracts` 76/76 (was 72), `actionlint` clean on every workflow
  in `.github/workflows/`, `test_affected_scope.py` 100/100 (was 90), and the
  eight other Python CI suites, the hook suite (16/16), `ci-rollup` (183/183),
  and `ci-plan` (9/9) all unchanged and green. `just check-canonical` passes
  27/27. **The fixture replay did not happen:** it is B5, which needs a push
  that triggers workflows plus scratch-repository writes, and no authorization
  is obtainable in a non-interactive session. AC2, AC3, AC4, AC10, and AC13 are
  proven statically by the mapping above; AC5 is unchanged from Phase 4; AC9's
  check-run half stays pending on B4/B5.

## Phase 7 outcome (2026-09-11) — the seams are migrated, the authority is not

Two of the seven tasks are done. The other five all reduce to the same two
things no agent can supply: Ken's OQ3 ruling (B3), and authorization to push,
to create a scratch repository, and to edit ruleset 19747338 (B5). Phase 7 is
by construction the one phase whose headline act is an operational migration,
so unlike Phases 2–6 the blocked half here is the majority of the phase.

What *was* available is the set of seams the migration crosses — and one of
them was broken.

**The finding: Phase 6's renaming silently killed runner-loss attribution.**
`runner_loss.parse_job_name` matched whole job names with two regexes anchored
on the pre-area spelling (`playa-cli / wsl2 (playa-cli) / test (playa-cli on
wsl2-ubuntu)`). Phase 6 inserted the area level and moved the environment out
of the parenthetical, so every shipped producer label now reads
`area-ci (playa) / playa-cli / wsl2 / test (wsl2-ubuntu)`. **All six** of them
parsed to `None`: a job whose hosted runner died would have synthesized no
status at all, and its cell would have read `MISSING — produced no report`,
which is the exact ambiguity `runner_loss.py` exists to remove. The suite
stayed green because every fixture spelled its job names by hand.

A second, quieter break sat beside it. `NON_PRODUCER_JOBS` excluded
`ci-verdict` by name so that the judging job's failure could not veto the
one-shot retry. After Phase 6 the judging jobs are each area's `rollup`, which
fails *because* a producer lost its runner — so the retry would have been
vetoed by the very consequence it exists to repair, on every lost runner.

Both are fixed, and both are now pinned by a corpus test that derives each
producer's composite label and its `status-…` artifact from the shipped
workflow files rather than from a hand-written string.

**Why Phase 7's two tasks could proceed while the plan is blocked**, checked
the same way Phases 2–6 checked it:

| Blocker | Effect on the two completed tasks |
|---|---|
| B0 land-first vs absorb | **None.** Both tasks read contracts this plan already built. |
| B1 OQ1 seam | **None.** No dependent has a cell, so none reaches a slice or a job label. |
| B2 OQ2 constraint store | **None.** A prohibited cell is labelled in the plan and reported by the owning area's rollup, wherever the store lives. |
| B3 OQ3 merge gate | **None for these two.** Both properties the gate needs — a faithful run conclusion and complete per-area results — hold under Option B and Option C alike, which is why they could be built before the choice. |
| B4/B5 gap conclusion and fixtures | **None.** Neither task publishes a check run. |

**Three findings later phases need.**

- **The specification's migration checklist is incomplete by two entries.**
  `just ci-diff` downloads the whole-run `ci-results` artifact *by name*, and
  `.claudine/scripts/ci-watchdog.ts` waits for a job literally called
  `ci-verdict`. Both stop working the moment task 4 deletes the job, and
  neither appears in spec §5's list. Rewiring them early would break them
  while the job still exists, so they are pinned as a second pending contract,
  `the_verdict_consumers_are_rewired_when_the_job_goes`. Task 4 is complete
  only when **both** pending fixtures are deleted together.
- **An all-reused area must still fan out.** If a receipt covers every cell an
  area owns and the area then dropped out of `scheduled_areas`, no
  `ci-results-<slug>` slice would ever be written for it — once `ci-verdict`
  is gone, that area's local-origin results would be reported nowhere. The
  planner gets this right today (the matrix is built from the package's
  declared gates, not its executing cells) and a mutation that drops the area
  fails the new fixture. This is the PR #76 failure mode wearing a different
  hat, and it is invisible in CI because nothing turns red when it happens.
- **Exactly one job may be `continue-on-error`.** Once the gate folds the run
  conclusion, every such job is invisible to it. The sweep is now over all
  four reader-facing workflows with an exact-set assertion, rather than the
  named list Phase 6 left; the set is `{ci.yml:summary}`.

**A narrowing that needs recording.** `reuse_validation.py` and
`release-plz.yml` needed **no code change**. Both already key on a completed,
successful `ci` run, which is what the conclusion becomes after the migration;
task 5 asked them to be rewired, and the honest answer is that they were
already pointed at the right signal. What was missing was a contract saying so
in one place, which is now
`every_downstream_consumer_reads_the_same_ci_run_conclusion`. Their rejection
behavior (failure, cancellation, in-flight, changed tree or base, missing and
expired receipts) is already covered by `test_reuse_validation.py` and is
referenced rather than duplicated.

**Counts.**

| Suite | Before | After |
|---|---:|---:|
| `tools/test-toolkit/tests/ci_workflow_contracts.rs` | 76 | 81 |
| `scripts/ci/test_runner_loss.py` | 23 | 31 |
| `scripts/ci/test_resolved_plan.py` | 33 | 36 |

### Phase 7 requirement-to-test mapping

| Criterion | Behavior | Targeted test |
|---|---|---|
| §5 | A lost runner's cell is attributed at the label the workflows actually produce | `JobNameCorpusTests.test_every_producer_job_parses_to_the_status_it_uploads` |
| §5 | The corpus covers every producer, not an empty set | `JobNameCorpusTests.test_every_status_uploading_job_is_covered` |
| §5 | Each gate's label maps to the tier and environment its artifact is keyed by | `JobNameTests.test_native_legs_map_to_their_status_names`, `…test_wsl2_leg_maps_to_l1_on_wsl2_ubuntu` |
| §5 | `lint`'s label names an environment its artifact does not | the `lint` case above plus `test_status_directory_omits_a_missing_environment` |
| §5 | A nested area name does not break the package segment | `test_a_nested_area_name_still_yields_its_package` |
| §5 | The pre-area spelling is a clean break, not a silent dual mode | `test_the_retired_pre_area_spelling_no_longer_parses` |
| §5 | Non-producers (archive, preflight, the tui workflow, the area rollup) yield no cell | `test_non_producer_and_archive_jobs_map_to_nothing` |
| §5 | An area rollup failing from a lost runner does not veto the retry | `ClassifyTests.test_a_judging_job_never_counts_as_a_failure` |
| §5 | A real producer is never mistaken for a judge | `ClassifyTests.test_a_producer_is_never_mistaken_for_a_judge` |
| §5 | The judge set is read off the shipped workflows | `JobNameCorpusTests.test_the_areas_own_rollup_is_recognized_as_a_judge`, `…test_the_advisory_summary_is_recognized_as_a_judge` |
| Task 6 | Every resolved cell reaches some area's result slice | `ResultCompletenessTests.test_every_resolved_cell_belongs_to_a_scheduled_area` (two areas, nested area, full scope) |
| Task 6 | An all-reused area still owns a slice and still schedules no runner | `ResultCompletenessTests.test_an_area_whose_every_cell_is_reused_still_owns_a_result_slice` |
| Task 6 | No two areas share a slice name; every slug is artifact-legal | `ResultCompletenessTests.test_every_scheduled_area_has_an_artifact_safe_slice_name` |
| Task 6 | A blocked area still publishes its results, before the verdict step | `a_blocked_area_still_publishes_its_result_slice` |
| Task 6 | The run conclusion is a faithful conjunction | `only_the_advisory_summary_is_excluded_from_the_run_conclusion` |
| Task 6 | Branch protection, validation reuse, and release automation read one signal | `every_downstream_consumer_reads_the_same_ci_run_conclusion` |
| Task 2 | The transitional verdict and the area rollups cannot disagree | `the_transitional_verdict_and_the_area_rollups_judge_the_same_inputs` |
| Task 4 | Removal is atomic with its two unlisted consumers | **pending** — `the_verdict_consumers_are_rewired_when_the_job_goes` |
| AC11 | `ci-verdict` removal | **pending** — `no_standalone_global_verdict_job_remains` (B3/B5) |

Every new fixture was proven non-vacuous by mutation rather than by assertion
count: the six shipped producer labels were replayed through the parser as it
stood at `HEAD` and all six returned `None`; dropping an area from the fan-out
once nothing in it executes fails the all-reused fixture; making the slice
upload conditional fails the publication fixture; and adding
`continue-on-error: true` to `ci-tooling` fails the conjunction sweep. Each
mutation was reverted in the same step that made it.

**Gates run.** `test_runner_loss.py` 31/31 (was 23), `test_resolved_plan.py`
36/36 (was 33), `ci_workflow_contracts` 81/81 (was 76), `test_ci_local.py`
17/17, `test_schema.py` 37/37, `test_affected_scope.py` 100/100,
`test_local_evidence.py` 5/5, `test_evidence_reuse.py` 47/47,
`test_constraints.py` 24/24, `test_reuse_validation.py` 15/15, the hook suite
16/16 (`env -u CDPATH`), `ci-rollup` 183/183, `ci-plan` 9/9, and
`cargo clippy -p test-toolkit --all-targets` clean.

**Pre-existing, untouched.** `actionlint` reports six `SC2086:info` shellcheck
findings in `_package-ci.yml` and `_wsl-ci.yml`. All six are present at `HEAD`
— verified by running `actionlint` over the committed copies of those two
files — and neither file is edited by this phase. The `scripts/drift.rs`
clippy break recorded in Phase 4 is likewise unchanged. The root `just lint`
was **not** run: it lints the whole monorepo, which is outside this phase's
blast radius.

## Phase 7 — Migrate merge authority atomically

- [ ] **BLOCKED (B3/B5).** Re-prove the selected OQ3 mechanism in the scratch
  repository: a failed selected area blocks, an unselected area creates no
  pending requirement, missing required coverage blocks, accepted gaps do not
  fail the workflow, and a successful run satisfies the exact ruleset
  configuration to be used.
  → Unchanged from Phase 1: OQ3 is unruled, and Option C cannot even be tested
  for availability without *writing* a ruleset in a scratch repository the
  session is not authorized to create. Requirements are in
  `open-questions-and-blockers.md` §B3/§B5. The two properties the mechanism
  needs from this repository, rather than from GitHub, are built and tested
  under both options: the run conclusion is a faithful conjunction, and every
  area publishes a complete result slice.
- [ ] **BLOCKED (trigger authorization).** Prepare a compatibility revision in
  the implementation PR that has the new per-area path while temporarily
  retaining the old `ci-verdict`; get the PR's own run green under the old
  required context before changing repository rules.
  → The revision itself is **prepared and pinned**: Phase 6 kept `ci-verdict`
  beside the area fan-out, and
  `the_transitional_verdict_and_the_area_rollups_judge_the_same_inputs` now
  asserts that both read the same plan, policy, environment table, artifact
  patterns, and baseline, so the two cannot reach opposing verdicts while both
  exist. The remaining clause — *green under the old required context* — needs
  a push that triggers workflows, which this plan's own execution constraints
  withhold; it is the first item of Phase 9.
- [ ] **BLOCKED (B3/B5 + explicit authorization).** With explicit
  authorization, replace `ci-verdict` in `protect-your-bacon` (ruleset
  19747338) with the proven required-workflow mechanism, or the proven
  policy-free conjunction fallback, and immediately verify the PR has neither
  a stale pending context nor an unprotected failure path.
- [ ] **BLOCKED (atomic with the task above).** Remove the standalone
  `ci-verdict` job and every dependency, comment, and contract that treats it
  as global policy authority. If the fallback conjunction is used, constrain
  it to a pure fold of area job conclusions; all baseline, gap, and
  missing-cell decisions remain inside area rollups.
  → Removing the job before the required context moves leaves every PR waiting
  on a check that never reports, so this is not separable. The **complete**
  change set is now named rather than discovered later: the `ci-verdict` job
  in `ci.yml`; the `NON_PRODUCER_JOBS` entry in `scripts/ci/runner_loss.py`;
  the advisory summary's closing line; `just ci-diff`'s
  `gh run download -n ci-results`; `.claudine/scripts/ci-watchdog.ts`'s
  `ci-verdict` job lookup; and the two pending fixtures
  `no_standalone_global_verdict_job_remains` and
  `the_verdict_consumers_are_rewired_when_the_job_goes`, which must be deleted
  together. The last two are **not** in the specification's §5 checklist and
  were found here.
- [x] Rewire `ci-infra-retry.yml`, `reuse_validation.py`, `release-plz.yml`, and
  `pr-health.yml` to the new run-conclusion/result locations; verify validation
  reuse accepts only successful equivalent runs and rejects failure,
  cancellation, changed inputs, and missing evidence.
  → `ci-infra-retry.yml`'s dependency was not its trigger but its
  classification: `runner_loss.parse_job_name` no longer matched **any**
  shipped job label after Phase 6, and the area rollup vetoed the retry it was
  supposed to trigger. The parser now reads the composite label from the tail
  and `is_non_producer` covers every judging job; a corpus test derives all six
  producer labels and their `status-…` artifacts from the workflow files.
  `reuse_validation.py` and `release-plz.yml` already key on a completed,
  successful `ci` run and needed no change — now asserted in one place, with
  their rejection behavior referenced from `test_reuse_validation.py` rather
  than duplicated. `pr-health.yml` and `ci-infra-retry.yml` comments no longer
  describe `ci-verdict` as the thing that fails to report; they name the run
  conclusion and the area outcomes, which is true both before and after the
  migration.
- [x] Confirm the `ci-results` artifact remains complete for reporting and
  future reuse and that the workflow run conclusion is exactly the conjunction
  consumed by branch protection, validation reuse, and release automation.
  → Completeness is a planner property and is now tested as one: every
  resolved cell belongs to a scheduled area across two-area, nested-area, and
  full-scope plans, an all-reused area still fans out and still owns a slice
  while scheduling no runner, and no two areas share a slice name. Publication
  is tested in the workflow: the slice is uploaded `if: always()` and *before*
  the verdict step, under the one filename the rollup writes and judges. The
  conclusion is a faithful conjunction because exactly one job
  (`ci.yml:summary`) is `continue-on-error`, asserted as an exact set over all
  four reader-facing workflows.
- [ ] **PARTIAL — the phase's own stop condition is unmet.** Rerun the scratch
  ruleset cases, focused reuse tests, workflow contracts, and the
  implementation PR's CI; do not complete this phase while either the obsolete
  check can block all PRs or incomplete coverage can merge.
  → The focused half is green and listed under **Gates run** above. The scratch
  ruleset cases and the PR's own CI did not run — both need authorization this
  session cannot obtain. The checkpoint's terminal clause is the reason tasks 3
  and 4 stay open rather than being attempted: `ci-verdict` is still the single
  required context, so the obsolete check cannot yet block all PRs *and*
  incomplete coverage cannot merge. That is the safe state to stop in, and it
  is the state this phase leaves behind.

## Phase 8 outcome (2026-09-11) — COMPLETE

All eight tasks are done. This is the first phase of the plan with no blocked
task, because none of its work changes CI behavior: it aligns the documentation
with what Phases 3–7 built and validates that alignment locally.

**Why Phase 8 could proceed while the plan is blocked**, checked the same way
Phases 2–7 checked it:

| Blocker | Effect on Phase 8 |
|---|---|
| B0 land-first vs absorb | **None.** The documentation describes the contracts that exist. |
| B1 OQ1 seam | **None.** AC1's presentation rule is documented; where the seam gets compiled is named as unruled rather than described as settled. |
| B2 OQ2 constraint store | **None.** The store's *interface* is documented and `BISCUIT_CI_CONSTRAINTS_DIR` is named; the default location is stated as unruled. |
| B3 OQ3 merge gate | **None, and load-bearing.** The honest statement is that `ci-verdict` is still the required context, which the new AC14 guard now asserts in both files a reader consults before touching branch protection. |
| B4 OQ4 gap conclusion | **None.** Governed gaps are documented as the machine-readable state, never as a GitHub conclusion. |
| B5 presentation fixture | **None.** No task here needs a live run. |

**The finding: prose has no compiler, and two `os`-skill claims had gone
false.** `.claude/skills/os/SKILL.md` told a reader that "the current
verifier/calculator supports only one excluded environment per run" and that
"the proposed scope-only mode … is not implemented yet". Both were true when
written; Phase 4 falsified both and nothing caught it. The first is the
dangerous one — an agent reading it would conclude that a macOS receipt plus a
WSL constraint *cannot* both be honored and would either rerun WSL or stop,
which is exactly the failure mode `CLAUDE.md`'s execution-constraint rules
exist to prevent. `os/ci-runners.md` carried the matching error in its "a clean
pre-push can replace one hosted **environment**" bullet.

That is why this phase adds a test rather than only editing text.
`the_ci_documentation_states_the_implemented_behavior` pins the six retired
phrases and the ten live facts, and fails against `HEAD`'s copies of all six
documents.

**A second finding, flagged rather than fixed.** `ci-local`'s `cargo check`
branch is now unreachable: it fires only when `check` is a package's sole gate,
and every gating package owns a `lint` cell, so that can no longer happen. The
live consequence is a real local/CI divergence — CI schedules a `check` cell
for a package declaring `example` or `bench` targets and the pre-push hook does
not, so an examples-only compile break surfaces in CI instead of pre-push.
Widening the condition adds a compile pass to every push, which is a cost
decision after Ken's 2026-09-09 scope ruling rather than a drift fix, so the
behavior is unchanged and the condition is documented as unreachable at both
its definition and the recipe header. **This needs Ken's call.**

**Counts.**

| Suite | Before | After |
|---|---:|---:|
| `tools/test-toolkit/tests/ci_workflow_contracts.rs` | 81 | 82 |

**Gates run.** `test_schema.py` 37/37, `test_affected_scope.py` 100/100,
`test_resolved_plan.py` 36/36, `test_evidence_reuse.py` 47/47,
`test_local_evidence.py` 5/5, `test_constraints.py` 24/24,
`test_ci_local.py` 17/17, `test_reuse_validation.py` 15/15,
`test_runner_loss.py` 31/31, the hook suite 16/16 (`env -u CDPATH`),
`ci-rollup` 183/183, `ci-plan` 9/9, `ci_workflow_contracts` 82/82,
`just check-canonical` 27/27, `cargo clippy -p test-toolkit --all-targets`
clean, `cargo clippy --manifest-path scripts/Cargo.toml --bin ci-rollup --bin
ci-plan --all-features` clean, and `python3 -m py_compile scripts/ci/*.py`
clean. **Nothing skipped, nothing red.**

**Pre-existing, untouched.** `actionlint` over every workflow reports the same
six `SC2086:info` shellcheck findings Phase 7 recorded in `_package-ci.yml` and
`_wsl-ci.yml` and nothing else; this phase's only edits to those files are
comments. The `scripts/drift.rs` clippy break recorded in Phase 4 is unchanged
and still unrelated. `cargo fmt` was not run in any mode. The root `just lint`
was **not** run: it lints the whole monorepo, which is outside this phase's
blast radius.

## Phase 8 — Align documentation and run focused local validation

- [x] Update `.github/ci/README.md`, `docs/topics/ci-cd.md`, `CLAUDE.md`,
  `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/os/`, workflow
  comments, and the 2026-09-09 memory note to describe area grouping versus
  package identity, multi-environment evidence, target coverage, constraints,
  accepted gaps, and the actual merge gate.
  → `docs/topics/ci-cd.md` carried the most drift and was rewritten across six
  sections. Two **false** claims were removed from `.claude/skills/os/SKILL.md`
  ("the current verifier/calculator supports only one excluded environment per
  run"; "the proposed scope-only mode … is not implemented yet"), and
  `os/ci-runners.md`'s "a clean pre-push can replace one hosted environment"
  became the per-cell, multi-environment rule. `CLAUDE.md` gained a **CI
  Structure** section and the constraint store in **Test Execution
  Constraints**. `.github/ci/README.md`'s cache-key paragraph no longer claims
  Phase 6 measured the package-scoped key — it did not.
- [x] Update `ci-baseline.toml` commentary and policy links without re-keying
  package identities or changing accepted failures unrelated to this fix.
  → Header rewritten: the enforcing check is now each area's own
  `ci-rollup verdict --area` plus the transitional whole-run `ci-verdict`, an
  entry belongs to the area owning its package, and `check` joined `lint` in
  the non-JUnit tier vocabulary. No `[[failure]]` or `[[skip]]` entry exists to
  change, and `schema_version` stays 2 — the baseline versions independently of
  the result document's 3.
- [x] Review every edited `///`, `//!`, and inline `//` comment against the new
  behavior; remove verdict-era and single-environment drift while avoiding
  unrelated comment cleanup.
  → Nine drifted comments corrected, all in files this fix already touched:
  three "`verdict` is the single required branch-protection check" docblocks in
  `scripts/ci-rollup.rs`; the `NON_PRODUCER_JOBS` note in
  `scripts/ci/runner_loss.py` and the `ci-verdict` block in `ci.yml`, both of
  which claimed Phase 7 had removed the job; `whole_environment_evidence`'s
  docstring in `affected_scope.py`, which called itself transitional when it is
  in fact the version-1 migration path; `schema.py`'s module note, which said
  the verifier and rollup still predated the plan; and the `classify`-retirement
  and compile-check notes in `_package-ci.yml`. The `.githooks/pre-push` header
  and `just/ci-local.just`'s recipe header both still promised a reverse-
  dependency compile check.
  **Finding, flagged not fixed.** `ci-local`'s `cargo check` branch is now dead
  code: it fires only when `check` is a package's *sole* gate, and every gating
  package owns a `lint` cell, so that can no longer happen. The live gap is that
  CI schedules a `check` cell for a package declaring `example`/`bench` targets
  and the hook does not, so an examples-only compile break surfaces in CI
  instead of pre-push. Widening the condition adds a compile pass to every push
  — a cost decision after the 2026-09-09 scope ruling, not a drift fix — so the
  behavior is unchanged and the condition is documented as unreachable.
- [x] Run the focused Python suites for affected scope, local evidence,
  `ci-local`, validation reuse, and runner loss; run the hook shell suite,
  rollup Nextest suite, workflow-contract Nextest suite, and `actionlint`.
  → All green, no skips: `test_schema.py` 37, `test_affected_scope.py` 100,
  `test_resolved_plan.py` 36, `test_evidence_reuse.py` 47,
  `test_local_evidence.py` 5, `test_constraints.py` 24, `test_ci_local.py` 17,
  `test_reuse_validation.py` 15, `test_runner_loss.py` 31, the hook suite 16/16
  (`env -u CDPATH`), `ci-rollup` 183/183, `ci-plan` 9/9, and
  `ci_workflow_contracts` 81/81. `actionlint` over every workflow reports the
  same six pre-existing `SC2086:info` findings Phase 7 recorded in
  `_package-ci.yml` and `_wsl-ci.yml` and nothing else; this phase's only edits
  to those files are comments.
- [x] Run `just ci-local --plan` for representative source, nested-area,
  infrastructure-only, mixed-evidence, prohibited-environment, and explicit
  full-scope fixtures; verify its terminal output and canonical JSON describe
  the same cells.
  → All six run live against this checkout, not synthesized. Results are in
  the table below. The cell-set identity was checked twice — once against the
  plain-line fallback and once against the `TerminalRenderable` `Table` after
  building `scripts/target/release/ci-plan` — and both render exactly the 36
  cells `--plan-out` writes, with no double render when the binary is present.
- [x] Do not run `cargo fmt`; use the repository's formatting-check path only
  where it is already part of an authorized focused lint gate.
  → `cargo fmt` was not run in any mode, including `--check`. No focused lint
  gate in this phase's blast radius invokes it.
- [x] Run GitNexus `detect_changes(scope: compare, base_ref: main)` before any
  authorized commit and reconcile every changed symbol/process and file with
  this plan; investigate any partial result or unexpected execution flow.
  → Run at both scopes. `compare` against `main` is the whole branch, not this
  fix: 2,164 symbols across 297 files, `risk_level: critical`, `truncated: true`
  on the listing (`partial` absent). 21 affected processes, of which exactly two
  are CI tooling — `Cmd_rollup → One` and `Main → Validate_expiry`; the other 19
  belong to Claudine and Darkmatter work elsewhere on `feat/unifi` and are
  outside this plan. `unstaged` is the fix's own working set: 324 of 324 symbols
  listed (**not** truncated, `partial` absent), 27 files, `risk_level: medium`,
  and the *same* two affected processes — changed at `cmd_rollup` step 1 and
  `load_environments` step 2, which are precisely the functions Phases 5 and 3
  rewrote. **No unexpected execution flow.** Every changed file maps to a
  plan-declared source file; the only Phase-8-only entries are `CLAUDE.md`,
  `docs/topics/ci-cd.md`, and this plan.
  **Stated limit:** the index covers Rust, Python, and Markdown sections, so
  `_area-ci.yml`, `just/ci-local.just`, `.githooks/pre-push`, `.github/ci/*`,
  and the skill files this phase edited produce no changed symbols and were
  reconciled by reading the diff instead. The HIGH-risk `verdict` symbol Phase 1
  warned about is in the changed set; it is Phase 5's edit, not this phase's,
  and remains gated on OQ3.
- [x] **Validation checkpoint:** produce an AC1–AC17 evidence table linking each
  criterion to a focused test, fixture result, or explicitly pending live-CI
  observation.
  → Table below. 14 of 17 criteria are closed by focused tests or live fixture
  results; AC11 is partly pending (the merge gate is B3/B5), AC9's check-run
  half is pending (B4/B5), and AC3's "when macOS is satisfied locally" clause
  had no macOS receipt available on this host to satisfy it live.

### Phase 8 requirement-to-test mapping

| Criterion | Evidence | Status |
|---|---|---|
| AC1 | `just ci-local --plan` on this checkout selects exactly the Claudine, Playa, biscuit-speaks, root, and tools areas; the 12 unchanged reverse dependents (`dmls`, `darkmatter`, `sniff`, …) appear only in `reverse_dependencies` with no area, package record, or cell | **live fixture** |
| AC2 | 5 areas / 8 packages / 36 cells on the source fixture; `claudine/rendezvous` and `darkmatter/dmls` appear as their own areas beside their parents on the nested fixture; `scheduled_areas: []` for a `gates = false`-only area | **live fixture** |
| AC3 | every cell carries `target_kinds` and `compile_coverage_from`; `check` cells appear only where declared — `biscuit-speaks/windows-latest/check` cites *example* targets, `claudine/windows-latest/check` cites *bench* | **live fixture**; the macOS-satisfied clause has no local receipt to exercise it |
| AC4 | Phase 4's `ReceiptRecordingTests`; Phase 5's `a_reused_cell_carries_the_measurements_its_receipt_recorded`; Phase 6's `a_reused_cell_reaches_its_area_summary_without_being_re_executed` | closed |
| AC5 | `MultiEnvironmentTests.test_two_environments_are_accepted_together`, `CrossCheckPublicationTests.test_a_hook_receipt_and_a_cross_check_receipt_combine` | closed |
| AC6 | `the_pr_76_macos_cells_must_not_resolve_to_missing` + `the_pr_76_fixture_reproduces_the_regression_exactly` (`ci-rollup` 183/183) | closed |
| AC7 | `RejectionTests`, `GateInputIdentityTests`, `GateGlobalInputTests`; **live** — the repository's real `macos-latest` note is refused as `v1-not-equivalence-eligible` naming tree `534511a04`, and the prohibited fixture exits 1 naming all six blocked cells | closed |
| AC8 | `a_complete_failed_local_result_stays_a_failure`, `a_failed_area_blocks_only_its_own_slice` | closed |
| AC9 | `accepted_gap_is_a_distinct_machine_readable_state` and the governance fixtures; **live** — `claudine-cli/windows-latest/L2` and `…/wsl2-ubuntu/L2` render as `accepted-gap` with owner, expiry, and `closes`, and stay un-prohibited under an active WSL constraint because a governed gap consumes no host | closed except the check-run publication (**pending** B4/B5) |
| AC10 | `no_skippable_job_is_labelled_with_an_unresolved_expression`, `every_skippable_job_has_a_static_human_readable_identity`, `lint_and_check_labels_identify_their_environment` | closed statically; Checks-tab confirmation is Phase 9 |
| AC11 | `every_selected_area_owns_an_always_rollup_job`, `advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory`, `the_combined_summary_aggregates_area_slices_and_applies_no_policy` | **pending** — `no_standalone_global_verdict_job_remains` and `the_verdict_consumers_are_rewired_when_the_job_goes` (B3/B5) |
| AC12 | `test_reuse_validation.py` 15/15, `every_downstream_consumer_reads_the_same_ci_run_conclusion`, `post_merge_reuse_preserves_the_verdict_and_normal_ci_fallback` | closed |
| AC13 | `CiLocalTests`, `ThreadPolicyTests`, `L1ThreadForwardingTests` (17/17, unchanged), `the_worker_policy_survives_the_area_restructure` | closed |
| **AC14** | **`the_ci_documentation_states_the_implemented_behavior`** — a new corpus guard over all six reader-facing CI documents, asserting six retired phrases absent, ten live facts present, and `ci-verdict` described as transitional in both files consulted before touching branch protection | **closed by this phase** |
| AC15 | `test_every_workspace_members_area_matches_sniff` (all 73 members) and the universe-subset fixture; **live** — the nested fixture's 17 area names all come from the planner's sniff-matching rule | closed |
| AC16 | `LegacyReceiptTests` (4), `a_version_one_receipt_renders_its_measurements_as_unrecorded`; **live** — the real v1 note is refused off a non-exact tree | closed |
| AC17 | `PlanSurfaceTests` (5), `ci-plan` 9/9; **live** — the prohibited fixture exits 1, and the rendered cell set is byte-identical to `--plan-out`'s JSON through **both** the plain-line fallback and the `TerminalRenderable` `Table` | closed |

**Non-vacuity of the one new test.** `the_ci_documentation_states_the_implemented_behavior`
was replayed against the six documents as they stand at `HEAD` — it fails
there, naming `.claude/skills/os/SKILL.md still claims "only one excluded
environment"` — and passes against the corrected copies. The originals were
restored in the same step (507 insertions confirmed back).

**Live fixture results.**

| Fixture | Invocation | Result |
|---|---|---|
| Source change (Claudine + Playa, AC1's shape) | `just ci-local --plan` | 5 areas, 8 packages, 36 cells, 40 jobs; 2 accepted gaps; rc 0 |
| Nested area | `just ci-local --plan --base 3cd7d7ea7~1` | `claudine/rendezvous` and `darkmatter/dmls` are their own areas beside their parents |
| Infrastructure-only | `just ci-local --plan --base HEAD` | `change_class: documentation`, 0 cells, 0 jobs, `scheduled_areas: []`; the comment-only `ci.yml`/`affected_scope.py` edits correctly widened nothing (the PR #59 rule) |
| Mixed evidence | same as the source fixture | the repository's real v1 `macos-latest` note is refused with the coded reason and named tree |
| Prohibited environment | `BISCUIT_CI_CONSTRAINTS_DIR=… just ci-local --plan` | 6 prohibited cells named, **rc 1**; the governed `wsl2-ubuntu` L2 gap is not prohibited |
| Explicit full scope | `just ci-local --plan --all` | 31 areas / 73 packages / 402 cells / **432 jobs**; 28 scheduled areas, 66 matrix entries, largest area `homelab` at 7 — matching Phase 6's recorded figures and under both GitHub ceilings |

## Phase 9 outcome (2026-09-11) — STOPPED at the trigger boundary

Three of eight tasks are done. Five need a push, a workflow run, or a ruleset
write, and no authorization for any of them is obtainable in a non-interactive
session. The full record — the pre-trigger review, the unsatisfied plan, the
migration-safety verification, the final criterion table, the timing record, and
the runbook that runs the moment authorization exists — is in
`rollout-2026-09-11.md`.

**This phase is different from Phases 2–8.** Every earlier phase found work it
could legitimately do while the plan was blocked, because its deliverable was
code, fixtures, or prose. Phase 9's deliverable *is* the live run. What remains
doable is the review that precedes the trigger and the report that replaces it.

**The pre-trigger plan is clean.** At `05ab70065..0bd1d618e`, `just ci-local
--plan` exits 0: 5 areas, 8 packages, 36 cells, 40 jobs, **0 prohibited cells**,
2 governed gaps, 1 rejected receipt. The rendered table and `--plan-out`'s JSON
carry byte-identical cell sets.

**The trigger is nevertheless unsatisfied.** The only receipt ref in the
repository is `refs/notes/ci-local/macos-latest`, a version-1 record of tree
`534511a04`; the outgoing tree is `016ab9b1b`, so it is refused. **No
`wsl2-ubuntu` receipt exists at all**, and task 2 forbids creating one. All 34
executable cells would run in CI.

**Migration safety was established read-only**, so task 3 is blocked only on
the push itself: ruleset 19747338 still requires exactly `ci-verdict`, the
`ci-verdict` job still exists in `ci.yml`, both open PRs (#76 and #74) stay
satisfiable, and release-plz's `workflow_run` gate on `workflows: ["ci"]` with
`conclusion == 'success'` survives the area restructure.

**One defect found that would fail the run before any job starts.**
`.github/workflows/_area-ci.yml` is untracked while `ci.yml`'s `area-ci` job
calls it. A push that does not stage it fails the whole run at parse time.

**Two findings flagged, not fixed.**

- **The 82 workflow-contract tests run in no CI job, and now carry six
  criteria.** The gap itself is already documented — `.github/ci/README.md`
  names the promotion of the "R11 contract suite" as its durable fix, and is
  careful to claim only that the *Python* suites run on the `ci-tooling` leg.
  No documentation correction is needed. What is new is the weight: since
  Phase 8, `ci_workflow_contracts.rs` is the primary evidence for AC10, AC11,
  AC14, AC15, AC16, and AC17. `test-toolkit` declares `gates = false` — visible
  in this plan as the `tools` area being selected with *zero cells* — and the
  `ci-tooling` job has no `-p test-toolkit` step, so the suite passed 82/82
  today only because it was invoked by hand.
- **Both `gates = false` exclusions cite a blocker that has cleared.**
  `test-toolkit` and `biscuit-test-harness` each justify
  `exclusion-class = "promotion-pending"` by "promotion is blocked on the
  canonical just recipe set (`check-canonical`) for …". `just check-canonical`
  now passes 27 areas and fails 0, **including both**, and the recipes work:
  `just test test-toolkit` runs 153 tests and `just test biscuit-test-harness`
  runs 99, both green. The reasons were written against 63 and 92, so both
  suites have grown while excluded. Promoting them adds gates to every push
  touching those packages, so it is a cost decision after the 2026-09-09 scope
  ruling rather than a drift fix. **This needs Ken's call**, and the coverage
  gap above is the concrete cost of leaving it.

**No behavior changed in this phase, so no test was added.** Phase 9 executes
and verifies; it does not implement. The gates below are re-runs of Phase 8's
set against the final outgoing head, which is the verification this phase owes.

**Gates run — 660 tests, 0 failures, 0 skips.** `test_schema.py` 37,
`test_affected_scope.py` 100, `test_resolved_plan.py` 36,
`test_evidence_reuse.py` 47, `test_local_evidence.py` 5, `test_constraints.py`
24, `test_ci_local.py` 17, `test_reuse_validation.py` 15, `test_runner_loss.py`
31, the hook suite 16/16 (`env -u CDPATH`), `ci-rollup` + `ci-plan` 192/192,
`just test test-toolkit` 153 (of which `ci_workflow_contracts` 82/82),
`just check-canonical` 27/27, `clippy -p test-toolkit --all-targets` clean,
`clippy` over `ci-rollup`/`ci-plan` clean, and `python3 -m py_compile
scripts/ci/*.py` clean. The two `#[ignore]` tests in `test-toolkit` pass under
`--run-ignored=all` (155/155).

**Pre-existing, untouched.** `actionlint` reports the same **six**
`SC2086:info` shellcheck findings in `_package-ci.yml` (5) and `_wsl-ci.yml`
(1) that Phases 7 and 8 recorded, and nothing else. `cargo nextest run
--manifest-path scripts/Cargo.toml --all-features` still fails to compile
`scripts/drift.rs` — the break Phase 4 recorded as unrelated — which is why the
rollup gates are run against the `ci-rollup` and `ci-plan` bins by name. The
root `just lint` was **not** run: it orchestrates every area in the monorepo,
far outside this phase's blast radius. `cargo fmt` was not run in any mode.

## Phase 9 — Roll out with authorized evidence and verify end to end

- [x] Generate the final pre-trigger plan for the actual outgoing head and
  review every cell's area, package, environment, gate, origin, state, evidence,
  and action against all active execution constraints.
  → `rollout-2026-09-11.md` §1. `05ab70065..0bd1d618e`, exit 0, 4.62 s: 5 areas,
  8 packages, 36 cells, 40 jobs. Execution 34 `execute` / 2 `omit`; origin 34
  `ci` / 2 `none`; state 34 `pending` / 2 `accepted-gap`; gates L1 24, lint 6,
  L2 4, check 2; environments ubuntu 13, windows 9, macOS 7, WSL 7. The 16
  unchanged reverse dependents carry no area, package record, or cell. The
  rendered table and `--plan-out`'s JSON are byte-identical cell sets.
  **On constraints:** `prohibited_cells` is empty because the store is empty —
  `BISCUIT_CI_CONSTRAINTS_DIR` is unset, `default_directory()` returns `""`
  pending B2, and `~/.rusty-biscuit/ci-constraints/` does not exist. The
  standing PR #74 "no reruns, no extra full-scope runs" instruction is a live
  constraint the store cannot yet express, so it must be applied by hand.
- [ ] **BLOCKED (authorization).** Run only explicitly authorized impacted-area
  validation on available hosts, retain full reports under the documented
  evidence location, and publish exact receipts. Do not rerun WSL; reuse a
  qualifying prior receipt or stop and report the unsatisfied plan.
  → The task's second branch is the one that applies, and it is discharged in
  `rollout-2026-09-11.md` §2. No authorization exists to run the `claudine`,
  `playa`, or `biscuit-speaks` gates or to write receipt notes. The one receipt
  that exists is refused (`v1-not-equivalence-eligible`, tree `534511a04` versus
  the outgoing `016ab9b1b`) and **no `wsl2-ubuntu` receipt exists at all**, so
  all 34 executable cells are unsatisfied.
- [ ] **BLOCKED (push authorization; B3).** Trigger CI only when no prohibited
  cell remains scheduled and the ruleset/workflow migration sequence from Phase
  7 is safe for all open PRs.
  → Both preconditions are **verified** read-only in `rollout-2026-09-11.md` §4:
  0 prohibited cells, ruleset 19747338 still requiring exactly `ci-verdict`, the
  `ci-verdict` job still present at `ci.yml:447`, both open PRs satisfiable, and
  release-plz's trigger intact. Only the push is missing — plus one defect that
  must be fixed first: **`.github/workflows/_area-ci.yml` is untracked** and
  `area-ci` calls it, so an unstaged push fails the run at parse time.
- [ ] **BLOCKED (depends on the trigger).** Inspect the resulting Actions graph
  and Checks tab: one top-level identity per selected area, package detail
  underneath, visible reused and accepted-gap cells, environment-qualified
  compile/lint labels, no unresolved expressions, and no standalone global
  policy verdict.
  → The static half is asserted by `ci_workflow_contracts` 82/82, but see the
  §5.1 finding: that suite runs in no CI job. The last clause cannot be
  satisfied at all until B3 — `ci-verdict` is deliberately retained.
- [ ] **BLOCKED (depends on the trigger).** Download and inspect the scope and
  result artifacts; prove they contain the same accepted evidence set used for
  scheduling, every required cell is present or explicitly governed, and
  failures/missing results block the correct area.
  → The artifact contract is asserted against fixtures by `ci-rollup` 183/183,
  including `the_pr_76_macos_cells_must_not_resolve_to_missing`. No run exists
  to download.
- [ ] **BLOCKED (depends on the trigger).** Verify successful PR-validation
  reuse and the release-plz trigger against the new workflow conclusion without
  publishing a release or changing package distribution policy.
  → The wiring is verified statically in `rollout-2026-09-11.md` §4:
  `ci.yml`'s `name:` is still `ci`, release-plz keys on `workflow_run` of
  `workflows: ["ci"]` with `conclusion == 'success'`, and omitted accepted-gap
  cells create no job and so cannot move that conclusion.
  `test_reuse_validation.py` is 15/15. Observing one live fire still needs a run.
- [x] Record local execution time separately from CI queue, setup, build, test,
  archive, and artifact-publication time; record per-cell test counts/durations
  and never attribute aggregate local duration to one area.
  → `rollout-2026-09-11.md` §7, per gate. **No impacted-area gate ran**, so
  there is no per-area local duration to attribute and no CI queue, setup,
  build, archive, or publication time to separate it from. Totals: 660 tests,
  0 failures, 0 skips; the slowest gate is `test_evidence_reuse.py` at 27.66 s
  and the plan render is 4.62 s.
- [ ] **NOT MET — Final validation checkpoint:** close the AC1–AC17 evidence
  table, retain the GitHub fixture/run/ruleset links, confirm all documentation
  matches live behavior, and mark the fix complete only when no criterion
  depends on an unverified assumption.
  → `rollout-2026-09-11.md` §6. Eleven criteria are closed to the limit of what
  a host can prove. **Six still depend on an unverified assumption**: AC1, AC4,
  AC6, AC8, and AC12 need a live run; AC9 and AC11 need B3/B4/B5; AC10 and AC14
  additionally rest on a suite that no CI job executes (§5.1). The only retained
  GitHub links are read-only observations — ruleset 19747338, run 34638047631
  from Phase 1, and PRs #76 and #74. **The fix must not be marked complete.**

## Critical path

Phase 1 decisions and prerequisite evidence gate all implementation. Phase 2
freezes interfaces; Phases 3 and 4 then produce the single resolved plan and
evidence set consumed by Phase 5. Phase 6 wires those contracts into GitHub.
Phase 7 is an atomic operational migration, after which documentation and
focused validation in Phase 8 lead to the authorized live rollout in Phase 9.


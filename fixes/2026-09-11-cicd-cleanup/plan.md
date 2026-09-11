---
title: CI cleanup — impacted areas, reusable evidence, and area-owned outcomes
status: blocked
created: 2026-09-11
phase: 1
total_phases: 9
agent: codex/gpt-5.6-sol
yolo: true
spec: fixes/2026-09-11-cicd-cleanup/spec.md
depends_on:
  - fixes/2026-09-10-local-affected-scope/spec.md
blocked_on: fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md
packages: []
source_files_during_phase_1: []
docs_created_during_phase_1:
  - fixes/2026-09-11-cicd-cleanup/prerequisite-audit.md
  - fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md
  - fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md
docs_updated_during_phase_1:
  - fixes/2026-09-11-cicd-cleanup/plan.md
skills_files_updated_during_phase_1: []
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
- [ ] Add planner fixtures for Claudine-plus-Playa source changes, nested areas,
  documentation/CI-tooling-only changes, explicit full scope, gates-disabled
  packages, relevant target kinds, and unchanged reverse dependencies receiving
  no area/job/cell.
- [ ] Add evidence fixtures for exact-tree matches, matching older-head input
  identities, changed inputs, multiple environments, duplicate agreement and
  conflict, pass and fail outcomes, partial/interrupted runs, malformed notes,
  `--no-verify`, and version-1 pass-only receipts with unrecorded measurements.
- [ ] Add rollup fixtures for mixed local/CI origins, the exact PR #76 seven-cell
  regression, area-scoped missing/failure behavior, accepted/expired/revoked
  gaps, unrelated cancellations, package-level failure details, and a
  reporting-only combined summary.
- [ ] Add hook and workflow contract fixtures for persisted prohibitions,
  `just ci-local --plan`, non-focusing L2 behavior, unchanged worker limits,
  explicit overrides, static whole-stage skip names, and the absence of literal
  `${{ ... }}` text in reader-facing job labels.
- [ ] **Parallelizable:** split fixture authoring among the Python planner and
  evidence suites, the Rust rollup suite, and the workflow/hook contract suite
  after the schemas above are frozen; do not edit shared fixtures concurrently.
- [ ] **Validation checkpoint:** demonstrate that each new regression fixture
  fails for the intended pre-change reason, not from broken setup, and retain
  those failure messages as the implementation oracle.

## Phase 3 — Produce the canonical area-aware execution plan

- [ ] Extend `scripts/ci/affected_scope.py` to derive each workspace member's
  area from its manifest directory using the same rule as
  `sniff repo package-area`; emit the area on every matrix and policy record
  without introducing a committed mapping file.
- [ ] Remove `CHECK_OS`, `scheduled_reverse_ids`, the single
  `excluded_environment`, and compile-only selection of unchanged reverse
  dependencies; accept the verified per-cell result set and omit exactly those
  executions while retaining them as expected local-origin result cells.
- [ ] Preserve Linux and native-Windows compile coverage when macOS cells are
  reused, and model WSL compile coverage as the `ubuntu-latest` archive build,
  never as compilation inside the toolchain-free WSL guest.
- [ ] Replace blanket check generation with explicit library, binary, test,
  example, and benchmark target decisions. Treat L1 compilation as coverage
  for matching library/binary/test targets and schedule an additional check
  only for required target kinds not produced by that gate.
- [ ] Implement the selected OQ1 seam policy. If Option B is chosen, attach the
  Ubuntu-only direct-dependent check to the changed package's own compile cell,
  report the dependent package names/count, and never create a result identity
  for their unchanged areas.
- [ ] Emit an area matrix whose entries contain applicable package targets and
  per-gate matrices, with a concrete selection reason for every area, package,
  environment, tier, and target kind. Retain the existing per-gate global-input
  widening rule and package metadata for features, native dependencies,
  backends, tools, and companion suites.
- [ ] Add the sniff drift contract: compare every workspace member's derived
  area with `sniff repo package-area` from its manifest directory and compare
  the resulting universe with `sniff repo package-areas` in local self-tests
  and preflight wherever sniff is present.
- [ ] **Validation checkpoint:** run `python3 scripts/ci/test_affected_scope.py`
  and inspect stable fixture JSON to prove AC1–AC3 and AC15, including matrix
  size and reusable-workflow depth estimates.

## Phase 4 — Combine multi-environment evidence and enforce trigger constraints

- [ ] Replace `verified_environment()` with verification that reads every
  `refs/notes/ci-local/<environment>` note, returns all accepted cells plus a
  rejection reason per rejected cell, and fails safe by scheduling work on
  missing, malformed, incompatible, stale, conflicting, or incomplete input.
- [ ] Compute and store each cell's gate-input identity from a canonical,
  sorted representation of the build-closure `git ls-tree` entries (including
  dev-dependencies) and the existing gate-specific global inputs. Use Git's
  hashing boundary consistently and never restamp an older result for a new
  tree.
- [ ] Implement version-1 migration exactly as specified: exact-tree only,
  pass-only whole-environment reuse, no input-equivalence reuse, no in-place
  upgrade, and the visible measurement text `not recorded (v1 receipt)`.
- [ ] Update local gate staging and `.githooks/pre-push` so `strict` and `warn`
  publish every complete passing or failing cell before returning, while
  interrupted or dirty-tree work publishes no reusable result; preserve
  `scope-only` and the deprecated `off` alias semantics from the prerequisite.
- [ ] Update `scripts/cross-check.sh` to publish a normal `wsl2-ubuntu` receipt
  only after an archive-mode run against the exact outgoing tree with a clean
  remote worktree; emit a clear no-publication reason for dirty, patched, or
  mismatched runs.
- [ ] Implement the selected OQ2 persisted-constraint interface with owner,
  reason, affected environment/tier, branch/repository identity, and expiry;
  enforce it only at direct command, push, dispatch, and manual-retry trigger
  boundaries, never inside CI scheduling.
- [ ] Extend `just ci-local --plan` to show reused, executing,
  accepted-policy-gap, prohibited, and rejected-evidence cells and to exit
  nonzero when a prohibited cell would execute. Render the human-facing plan
  through a `TerminalRenderable` `Table`/`Prose` surface in the existing Rust
  repository tooling while keeping canonical JSON as the machine interface.
- [ ] **Parallelizable after Phase 2:** evidence-schema and constraint-store
  implementation may proceed in separate files, but merge only after both
  consume the same resolved-plan schema and share rejection vocabulary.
- [ ] **Validation checkpoint:** run `python3 scripts/ci/test_local_evidence.py`,
  `python3 scripts/ci/test_ci_local.py`, and
  `bash .githooks/tests/test-pre-push.sh`; verify AC4–AC8, AC16, and AC17 without
  contacting a remote build host.

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


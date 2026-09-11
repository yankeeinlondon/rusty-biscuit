---
created: 2026-09-11
status: proposed
implemented: false
reviewed: false
area: repository-ci
---

# CI Cleanup: Impacted Areas, Reused Results, and Area-Owned Outcomes

## Objective

A viewer of a CI run must be able to identify the impacted package areas being
tested and inspect a complete environment matrix for each area. Verified local
results appear in that matrix without executing those tests again in CI. CI
runs only the remaining authorized cells. Each area owns its outcome; a
separate global verdict job must not reinterpret successful area results.

Only areas selected for testing receive functional tests, compile checks, and
applicable lint gates. Other areas receive no package jobs, including
compile-only jobs for unchanged reverse dependencies.

## Observed Problems

PR [#76](https://github.com/yankeeinlondon/rusty-biscuit/pull/76), including
[CI run 34638047631](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/34638047631),
exposed the following failures in the current contract:

- Unchanged dependent packages such as DMLS appeared as green top-level entries
  after only a Windows compile check. No functional tests ran for those entries.
- Top-level entries represented individual Cargo packages rather than package
  areas, separating libraries and CLIs that belong to the same area.
- Unscheduled jobs appeared as skipped placeholders with unresolved labels such
  as `${{ inputs.package }}` and `${{ matrix.environment }}`.
- The macOS receipt suppressed hosted execution but did not inject local results
  into the visible results matrix.
- All executed test jobs passed, but `ci-verdict` failed because it classified
  six macOS L1 cells and one macOS L2 cell as missing. Those cells had passed
  locally. The Windows and WSL L2 policy gaps were accepted notes, not the
  blocking reason for that verdict.
- The current verifier returns the first matching environment receipt, and the
  scope calculator accepts one excluded environment. Evidence from multiple
  environments cannot be combined.
- A request not to rerun WSL was incorrectly treated as applying only to manual
  reruns. A subsequent push automatically scheduled WSL anyway.
- Documentation still prescribes `ci-verdict` as the sole required merge check,
  conflicting with the requested removal. It also incorrectly claimed that
  `--no-verify` prevents reuse of already-published valid receipts.

The successful macOS validation of `0156096ff` took 160.696 seconds for 28 gates,
including 7,546 tests. Claudine L2 passed 248 tests with 14 workers in 35.494
seconds. These are recorded observations of the existing run, not performance
thresholds for this change.

## Scope and Terminology

- **Area:** the repository's existing package-area identity, such as `claudine`
  or `playa`. Use the canonical area mapping rather than guessing from crate
  name prefixes; an area may contain more than a library/CLI pair.
- **Package:** a Cargo workspace member within an area. Package-level feature,
  native dependency, tier, and backend declarations remain authoritative.
- **Cell:** an area's result for an environment and gate/tier, with package-level
  contributions available underneath it.
- **Environment:** macOS, Linux, native Windows, or WSL2. WSL2 remains distinct
  from native Linux and native Windows for execution and evidence.
- **Selected area:** an impacted area selected by the canonical scope planner
  for testing. Merely depending on changed code does not independently select
  an unchanged area for a compile-only job.

This specification changes selection, scheduling, evidence, reporting, and
merge-check ownership together. Renaming existing jobs alone is insufficient.
It does not authorize a full-workspace validation run, removal of platform
support, or bypassing real failures to make a run green.

## Required Behavior

### 1. Select and Execute Only Impacted Test Areas

1. Compute scope once from the event's actual changes and canonical package-area
   mapping. Local validation and hosted CI consume the same resolved plan.
2. Select the impacted test areas and their applicable package targets, preserving
   declared features, test tiers, native dependencies, and backend requirements.
   The plan must state why each area and package target was selected.
3. Selected areas receive functional tests and compile checks on the required
   environments, with valid reused results satisfying their corresponding cells.
   Applicable lint gates remain included and identify their execution environment.
4. Remove compile-only selection and top-level entries for unchanged reverse
   dependent areas. Ordinary dependency compilation needed to build a selected
   package remains normal build work, not another selected area or result cell.
5. Remove the hard-coded Windows-only compile-check policy. Linux and Windows
   compile coverage must remain when macOS coverage comes from local evidence.
   Respect WSL's archive execution contract rather than adding compilation inside
   its toolchain-free guest.
6. Do not use blanket `--all-targets` as the default check contract. Select the
   relevant library, binary, test, example, or benchmark targets explicitly when
   their coverage is required. `--all-targets` does not mean all packages or all
   operating systems, but its target breadth still requires a concrete reason.
7. Avoid a redundant compile step when an executed gate demonstrably covers the
   same target, features, and environment. Report where compile coverage came
   from; do not equate a compile success with functional-test success.
8. Infrastructure and documentation changes run their applicable contract checks
   without making unrelated Cargo areas appear as tested areas. Explicit full
   runs remain a separate, intentional operation.

### 2. Present an Area Matrix

The normal package portion of the CI run shows one top-level entry per selected
area. Packages, target details, and build plumbing are subordinate to that area.
Necessary infrastructure checks may remain, clearly identified as infrastructure.

Each area's matrix must expose:

| Field | Required information |
|---|---|
| Environment | macOS, Linux, Windows, or WSL2 |
| Gate/tier | Compile, lint, L1, L2, or another applicable declared suite |
| State | Pending, running, passed, failed, or accepted-policy-gap cancellation |
| Origin | Local validation, prior verified validation, or current CI execution |
| Evidence | Validated revision/input identity and a link to the report or logs |
| Measurements | Recorded test counts and execution duration where applicable |

- A locally satisfied cell remains visible and completed; it is not removed or
  represented as missing, skipped, or newly executed in CI.
- A valid receipt requires no test runner for that cell. Publishing its result
  may take CI bootstrap/API time, but must not rebuild or rerun its tests.
- No unscheduled browser, L2, or other gate appears as a placeholder job.
  Accepted policy gaps are the deliberate visible exception described below.
- No reader-facing label exposes unexpanded workflow expressions.
- Lint and compile labels identify their environment. Build/archive steps do not
  masquerade as functional-test results.
- Area rollups retain enough package detail to locate a failure without making
  each package a separate top-level area.

### 3. Reuse Verified Evidence Across Multiple Environments

Replace the single-excluded-environment model with evidence resolved per cell.
Local macOS and prior WSL evidence must be usable together when both qualify.

The evidence contract must retain:

- area, package, environment, gate/tier, feature and relevant target selection;
- tested revision/tree and the scope/input identity used to establish validity;
- actual outcome, counts, start/end or elapsed duration, and report provenance;
- applicable backend execution proof and schema/toolchain/policy compatibility.

Requirements:

1. The scheduler, area outcome, and summary consume one verified result model.
   Evidence accepted for scheduling must also satisfy the corresponding result
   cell. Recalculating a conflicting expectation in the reporting path is forbidden.
2. Valid results from multiple environments combine without overwriting one
   another. Mixed local and CI results contribute to the same area matrix.
3. Older results are not reusable merely because a run was recent. Either their
   existing exact identity matches, or a defined and tested input-equivalence
   rule proves the relevant execution inputs unchanged. Never restamp old
   results as if they tested a new tree.
4. Missing, malformed, incompatible, stale, interrupted, or insufficient evidence
   cannot become a passing result. Report the specific reason it was rejected.
5. A complete failed local result remains a failed result. Reuse must not turn
   failure into success, conceal it, or prevent independent environments from
   reporting their own results.
6. Define migration behavior for existing receipts, which currently lack detailed
   outcomes, counts, and duration. Preserve original evidence when a supported
   migration can validate it; never invent measurements or silently rerun tests
   to fill a legacy receipt's gaps.
7. `--no-verify` produces no new hook evidence, but does not invalidate receipts
   already published and verified for the outgoing work.

### 4. Honor Execution Constraints Before Triggering CI

A restriction such as "do not rerun WSL" applies to direct commands, automatic
push-triggered jobs, dispatches, and retries. Repush authorization retains that
restriction unless the user explicitly changes it.

Before a push or other workflow trigger, make the resolved execution plan
reviewable and reconcile it with every active environment/tier restriction.
Show which cells are reused, which will execute, and which are accepted gaps.

- Verify all requested exclusions; checking macOS alone does not establish that
  WSL is excluded.
- If reusable evidence is insufficient and an environment is prohibited from
  running, stop before the trigger and explain the unsatisfied constraint.
- Do not automatically convert a user restriction into a policy-gap acceptance,
  silently rerun the environment, or fabricate evidence to satisfy the plan.
- Carry explicit execution constraints through the supported local/CI scheduling
  interface so this boundary has regression coverage rather than depending only
  on an agent remembering a chat instruction.
- Classify job failures by stage. Passing tests followed by an artifact-upload
  failure do not establish a test regression or justify rerunning the suite.

### 5. Remove the Standalone Verdict Job

Remove `ci-verdict` as a standalone job and merge authority. Do not replace it
with the same global verdict mechanism under a different name.

- Each selected area owns the correctness of its combined local/CI result.
- Real failed or missing required results prevent that area's success.
- A reporting-only summary presents those same area results without applying a
  second, conflicting policy evaluation or becoming another verdict gate.
- Update branch-protection/ruleset requirements, validation-reuse code, workflow
  dependencies, baseline/policy ownership, and documentation together.
- Verify how required checks work for dynamically selected areas. A deselected
  area must not leave an expected check pending, and a failed selected area must
  not be mergeable simply because no static required-check name covers it.
- Failure of CI infrastructure that prevents required coverage must remain
  visible and blocking through the appropriate owning check. Removing the global
  verdict does not authorize merging incomplete validation.
- Preserve detailed machine-readable results where they support reporting and
  future reuse; the removal concerns the redundant global decision job.

### 6. Represent Accepted Policy Gaps Explicitly

An applicable, accepted policy gap appears immediately in the area's environment
matrix as a **cancelled** cell, without starting a test or archive build.

Its visible description/details must provide:

- the reason and affected environment, packages, and gate/tier;
- who owns/accepted the gap and its expiration;
- a direct link to the policy entry;
- instructions for changing or revoking acceptance and the coverage needed to
  close the gap.

An accepted gap counts as neither a pass nor a test failure. Area evaluation
recognizes its explicit, unexpired acceptance. An absent, expired, or revoked
acceptance cannot silently excuse missing required coverage and blocks the area.
An ordinary user cancellation, runner cancellation, or interrupted test is not
an accepted policy gap.

Validate GitHub's native job/check capabilities before choosing the mechanism:
synthetic cancelled results, descriptions/details links, required-check behavior,
and whether cancellation propagates to the parent workflow must be proven. Do
not cancel the whole workflow to produce this presentation or silently substitute
an ambiguous skipped placeholder. If GitHub cannot express the requested UI,
present the verified limitation and a concrete alternative before adopting it.

### 7. Preserve the Agreed Concurrency Policy

- Local runs: `max(1, logical cores - 2)` workers.
- CI with four or fewer logical cores: use all available logical cores.
- CI with more than four logical cores: `logical cores - 2` workers.
- Keep required shared-resource serialization and explicitly documented group
  limits. Do not reintroduce CI caps into local isolated suites accidentally.

The two-slot allowance preserves capacity for other host work; small CI runners
use their full capacity because reserving two would discard half or more of it.
This controls test workers, not CPU affinity or a guarantee of reserved cores.

## Implementation Boundaries

Review and update these existing surfaces as one contract:

- `scripts/ci/affected_scope.py` and its tests: area selection, targets, the
  execution plan, and removal of compile-only dependent-area selection.
- `scripts/ci/local_evidence.py`, its tests, and `.githooks/pre-push`: richer
  multi-environment evidence, execution restrictions, and pre-trigger validation.
- `.github/workflows/ci.yml`, `_package-ci.yml`, and `_wsl-ci.yml`: area grouping,
  scheduling only real work, visible reused results, and removal of `ci-verdict`.
- `scripts/ci-rollup.rs`, its tests, and validation-reuse code: one result model,
  area-owned outcomes, accepted cancellations, and reporting-only aggregation.
- `.github/ci/` policy records and repository required-check configuration:
  explicit policy-gap ownership and migration away from a global verdict.
- Shared Just recipes, agent instructions/skills, and human documentation:
  matching scope, concurrency, evidence, and execution-authorization semantics.

Determine the smallest coherent schema/workflow changes after examining these
surfaces. Avoid parallel planners, duplicate policy stores, or cosmetic job
renaming that preserves the conflicting underlying behavior.

## Acceptance Criteria

1. A fixture changing only Claudine and Playa source selects those test areas.
   Unchanged DMLS, icon, research, and other reverse-dependent areas get no jobs.
2. Each selected area has one top-level identity; applicable package contributions
   appear under it. Compile-only work does not create additional tested areas.
3. Selected areas receive required functional and compile coverage on Linux and
   Windows when macOS is satisfied locally. Relevant-target coverage replaces
   unexplained blanket `--all-targets` checks.
4. A valid macOS receipt immediately supplies visible completed cells with actual
   outcome, counts, duration, and provenance. No corresponding test runner starts.
5. Valid macOS and WSL evidence can both be reused in one run; the remaining
   authorized environments execute normally.
6. The exact PR #76 regression is covered: excluding seven proven macOS cells
   from execution does not turn any of them into `MISSING` in area results.
7. Stale/malformed/partial evidence is rejected. A prohibited environment with
   rejected evidence blocks the trigger with an explanation rather than rerunning.
8. A complete failed local result and a failed CI result both fail their owning
   area. Independent areas/environments still complete and remain visible.
9. An accepted policy gap creates an immediate, clearly explained cancelled cell
   with owner, expiry, and change instructions. Expired/revoked acceptance blocks;
   unrelated cancellations are not treated as accepted gaps.
10. Unscheduled gates create no placeholder jobs, and rendered names contain no
    literal `${{ ... }}` expressions. Lint's environment is visible.
11. No standalone `ci-verdict` or renamed equivalent remains. Required-check
    configuration blocks failed selected areas without waiting for unselected
    areas. The combined summary does not introduce another verdict.
12. Current successful PR-validation reuse continues to work with the new area
    results; failure, cancellation, changed inputs, and missing evidence cannot
    be reused as successful validation.
13. Tests cover local/CI worker boundaries, isolated L2 forwarding, and explicit
    overrides so the scheduling redesign preserves the concurrency correction.
14. Documentation and skills state both actual behavior and its rationale, remove
    obsolete verdict/single-environment/compile-only-area guidance, and clearly
    distinguish any remaining limitations from implemented capabilities.

## Validation and Rollout

1. Establish fixtures for scope, area mapping, mixed evidence, execution
   restrictions, policy gaps, and outcome propagation before changing scheduling.
2. Validate the GitHub presentation and dynamic required-check design with a
   minimal controlled fixture. Do not use the workspace's full test matrix just
   to discover check/job UI semantics.
3. Run focused planner, evidence, hook, workflow, and report contract tests, plus
   appropriate Nextest tests for changed Rust tooling. Do not run `cargo fmt`.
4. Prepare a reviewable pre-push plan showing selected areas, per-cell origin,
   actual executions, accepted gaps, and prohibited executions. The plan must
   satisfy the user's active restrictions before any push.
5. Run only authorized impacted-area validation locally, retain its actual
   reports, and publish verified evidence for the outgoing work. No WSL rerun is
   authorized by this specification; reuse qualifying prior evidence or surface
   the gap before triggering CI.
6. Coordinate the workflow and required-check migration so obsolete checks do
   not block every PR and incomplete coverage is not accidentally allowed.
7. Verify the resulting CI UI and machine-readable results end to end. Record
   local validation wall time separately from remaining CI wall time; distinguish
   test execution from setup, builds, queueing, and artifact publication. Never
   attribute the overall local duration to an individual area's cell.
8. Preserve existing valid results during rollout. This specification does not
   itself authorize committing, pushing, or rerunning completed suites.

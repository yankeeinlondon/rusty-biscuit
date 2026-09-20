---
created: 2026-09-12
status: draft
implemented: false
area: repository-ci
depends-on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
related:
  - fixes/2026-09-12-single-os-compile/spec.md
---

# Direct, Evidence-Aware CI Execution

## Status of This Draft

The immediate presentation mitigation described below is implemented alongside
this draft: the per-area `rollup` job is named `coverage-audit`, and it does not
produce a second red check when a producer has already failed. The remaining
design is proposed and must not be described as implemented.

## Problem

The canonical planner already resolves one `{package, environment, gate}` cell
for every required unit of coverage. It also decides whether each cell must run
in hosted CI, is satisfied by qualifying local evidence, or is an accepted
capability gap. Hosted execution nevertheless consumes a transitional package
projection containing several environment lists instead of consuming those
cells directly.

The additional projection and the post-execution rollup create avoidable
failure modes:

- a cell can exist in the plan but disappear while it is projected into nested
  matrices;
- a passing local cell can be omitted from execution without its result being
  represented correctly downstream;
- an ordinary test failure can make its producer, the area rollup, and the
  required merge gate all appear independently broken; and
- a reader must understand artifact reconciliation before trusting an
  apparently green or red producer.

The result is a poor developer experience and an unnecessarily indirect merge
decision.

## Objective

Make the canonical resolved plan the direct execution contract. Run every
required cell that lacks qualifying passing evidence, run no cell that such
evidence already satisfies, and make each executing producer authoritative for
its own outcome and completeness.

Accepted exceptions are decided before execution and rendered as `neutral`.
The fixed-name `ci-gate` remains a policy-free fold of truthful blocking job
results. Combined result grids remain available for diagnosis and comparison,
but reporting must not reinterpret an ordinary producer failure.

## Terminology

- **Cell:** one `{package, environment, gate}` unit of required coverage.
- **Cell inventory:** the complete `cells[]` collection in the canonical
  resolved plan. It is an inventory of gates, not individual Rust test cases.
- **Expected-test manifest:** the target-environment inventory of individual
  tests that should appear in one cell's result.
- **Producer:** the job that executes one or more explicitly identified cells
  and fails when any of them is incomplete or unsuccessful.
- **Coverage audit:** post-execution reconciliation used to detect missing or
  unscheduled evidence and exception-policy violations. It is not the owner of
  an ordinary test, lint, or compile failure.

## Design Decisions

### 1. The resolved plan is the only scheduling authority

`scripts/ci/affected_scope.py` emits every required cell exactly once. Before a
matrix is created, each cell has one terminal scheduling decision:

- `execute`: hosted CI must run it;
- `reuse`: qualifying passing evidence satisfies it and no hosted runner is
  scheduled;
- `accepted-gap`: a governed, unexpired capability exception satisfies policy
  without claiming a pass; or
- blocking configuration error: the plan is invalid and the scope job fails.

Only a complete passing receipt with equivalent gate inputs may produce
`reuse`. Missing, stale, partial, interrupted, mismatched, or failing evidence
leaves the cell as `execute`. This applies independently to macOS, Linux,
native Windows, and WSL2.

A same-head retry is cumulative. It overlays the newly executed cell outcomes
onto the compatible prior receipt and preserves both the receipt entries and
retained reports for cells the planner reused. The new outcome wins for a cell
that reran. A prior receipt with different plan bindings, host provenance, or
report directory must not be merged.

The planner must never remove a required cell merely because it removes that
cell's hosted execution.

### 2. Hosted matrices consume executing cells directly

Replace the transitional package projection with matrices derived directly
from `cells[] | select(.execution == "execute")`. Every matrix row carries the
full cell identity and all execution inputs needed by its producer. Area may
group rows for presentation, but it must not become a stored identity or a
second scheduling calculation.

Projection tests must prove a bijection:

```text
planned execute cells == hosted matrix cells
```

There may be no independently maintained environment lists, gate lists, or
workflow conditions capable of silently dropping one side of that equality.

An area containing only reused cells or accepted gaps needs no test runner.
Its results remain visible in the scope summary and reporting artifacts without
creating an otherwise empty execution workflow.

Within an executing test cell, the shared OS-aware worker budget is the
default: use all logical cores on GitHub runners with four or fewer. A narrower
limit must be tied to a measured shared-resource constraint, not applied to an
entire package merely because individual tests spawn child processes.

### 3. Every producer is truthful and self-validating

A failed test, lint, compile, setup, staging, or required upload step fails its
producer job. Gate commands never use `continue-on-error`. Matrix strategies
use `fail-fast: false` so one failure does not cancel unrelated cells that still
need coverage.

For a test cell, the producer performs all of the following before it may
succeed:

1. Generate or load the expected-test manifest on the target environment.
2. Execute the complete planned gate with no unplanned test filter.
3. Retain JUnit and producer-status evidence under the cell's stable identity.
4. Compare expected test identities with observed pass, fail, and skip
   identities.
5. Apply the exact approved skip policy attached to that cell.

A missing expected test, missing report, unexpected skip, failed companion
suite, or contradictory status is a producer failure. Artifact publication
failures are classified distinctly from test regressions but remain blocking;
they never turn passing tests into reusable evidence.

Lint and compile cells have no JUnit requirement, but they must emit and
self-validate their producer status before succeeding.

### 4. Exceptions have one pre-execution authority

Capability exceptions live in `.github/ci/environments.json`. The planner
validates their availability, owner, reason, expiry, and closing work and marks
qualifying cells `accepted-gap`. A small publisher may perform the GitHub API
call, but it consumes the planner's decision and does not reinterpret policy.
Each accepted-gap cell appears as a `neutral` check on the pull-request head.

Exact test-skip exceptions live in `.github/ci/ci-baseline.toml`. The planner
attaches the applicable entries to executing cells; the producer validates
them against the actual target-side test result. Known test failures are not
exceptions and cannot be made neutral or pardoned downstream.

An absent, incomplete, or expired exception fails before execution where
possible. A runtime contradiction, such as an unexpected skip, fails the
owning producer.

### 5. `ci-gate` remains a policy-free conjunction

Branch protection requires one stable check name. `ci-gate` reads only
`needs.*.result`, accepts `success` and intentionally unselected `skipped`
jobs, and blocks on `failure` or `cancelled`. It reads no plan, JUnit, baseline,
capability table, or result artifact.

The workflow structure must ensure that a selected executing matrix cannot be
reported as an intentionally unselected skipped job. Scope, preflight, direct
cell producers, specialized blocking workflows, and CI-tooling validation all
reach the conjunction.

### 6. Combined reporting is advisory

Machine-readable cell results and a combined grid remain useful for local
evidence, comparison, timing, skip visibility, and diagnosis. Their summary may
reconcile planned and observed results, but it applies no second ordinary
test-failure verdict and cannot contradict truthful producers.

During migration, `coverage-audit` remains blocking only for conditions not
already represented by a red producer: missing or unscheduled evidence,
invalid governed gaps, and exact-skip violations. It always renders its result
slice, but skips its enforcement step when the area's producer fan-out already
failed.

After every producer performs its own completeness and skip validation, the
coverage audit becomes fully advisory and may move out of the blocking area
workflow.

## Relationship to Compile-Once Work

The related compile-once specification may introduce build records and
producer/consumer archives. A build record is not a result cell and compiled
artifacts are not passing test evidence. This specification determines which
result cells still require execution; compile-once planning may then group
compatible executing cells behind one immutable build without changing their
identities or outcomes.

## Migration

### Immediate mitigation

- Rename the visible per-area `rollup` job to `coverage-audit`.
- Always render and upload the area result slice.
- Run its blocking policy step only when the package producers succeeded.
- Preserve blocking behavior for missing evidence, invalid gaps, exact-skip
  violations, and audit-tool failures when producers are otherwise green.
- Continue recognizing the former `/ rollup` job label when classifying older
  runs for infrastructure retry.

### Direct execution

- Add a plan projection containing exact executing cell rows.
- Change the hosted fan-out to consume those rows without rebuilding gate or
  environment lists.
- Prove the planned-execute/matrix bijection in planner and workflow contract
  tests.
- Stop fanning out areas that contain no executing cells; report their reused
  and neutral cells from the plan.
- Remove `legacy_scope_document()` only after every consumer has migrated.

### Producer completeness

- Wire target-generated expected-test manifests into each test producer.
- Validate JUnit, companion outcomes, status, and exact skips before a producer
  can succeed.
- Make lint and compile status validation local to those producers.
- Retain artifacts for reporting and evidence reuse after producer validation.

### Advisory reporting

- Remove exception and missing-result judgment from the combined reporter
  once those checks are producer-local or planner-owned.
- Move result aggregation outside the blocking area result.
- Keep `ci-gate` unchanged and verify the required ruleset context separately.

## Acceptance Criteria

1. The resolved plan contains every required `{package, environment, gate}`
   cell and exactly identifies `execute`, `reuse`, and `accepted-gap` states.
2. The hosted matrix contains every and only `execute` cell.
3. Qualifying local evidence suppresses only the equivalent cell on the same
   environment; all other required OS cells execute.
4. Every executing producer fails directly for a failed command, missing
   expected test, missing evidence, unexpected skip, or failed companion.
5. One producer failure creates one actionable red producer path plus the
   expected downstream `ci-gate` failure; `coverage-audit` does not create a
   duplicate red check.
6. A governed, unexpired capability gap produces one `neutral` check and no
   execution. Invalid or expired governance blocks.
7. No known test failure can be accepted by a downstream baseline.
8. An area with only reused or accepted-gap cells schedules no test runner and
   still presents every planned result.
9. macOS, Linux, native Windows, and WSL2 fixtures cover mixed reuse,
   execution, failure, missing evidence, and accepted-gap cases.
10. The compact CI schema, planner, evidence, workflow-contract, runner-loss,
    rollup, and actionlint checks pass.
11. Executing L1 cells inherit the OS-aware worker budget unless a documented,
    resource-specific exception requires a narrower limit.

## Open Questions

- Whether the direct row should be one cell per GitHub job or a small,
  explicitly enumerated group of cells per producer for compile reuse.
- The expected-test manifest format and how it represents tests compiled out by
  target `cfg` versus tests present but skipped at runtime.
- Where reused-cell and accepted-gap-only area summaries should appear once
  those areas no longer enter the execution fan-out.
- Whether exact skip policy should be embedded into each matrix row or read by
  a producer-side validator from the canonical plan.

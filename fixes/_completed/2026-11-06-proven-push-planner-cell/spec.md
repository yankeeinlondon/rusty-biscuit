---
kind: fix
name: proven-push-planner-cell
date: 2026-11-06
status: implemented
next_spec: 2026-11-06-release-path-repair
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
review_iterations: 2
implemented: true
implemented_by: "claude/opus"
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
completed: true
---

# A proven merge must not plan a cell for an environment it removed

CI plans one **cell** for each package, environment, and gate (such as tests,
lint, or a compile check). When a successful pull request already validated a
merge's tree, its push to `main` should schedule no work in those environments.
This fix prevents a check of unchanged dependent packages from escaping that
rule, and catches inconsistent plans before CI dispatches them.

## Why

Seven of the eight most recent failing pushes to `main` failed in `Resolve this
cell's execution contract`, before compiling anything:

```text
cell_contract: cell-contract-mismatch: the plan knows no environment 'ubuntu-latest'
```

When a merge's pull request already proved the tree, the repo-deps package's
[environment scheduler](../../scripts/ci/affected_scope.py)
(`schedule_environments`) moves Linux and macOS to `proven_environments`.
Its [package cell planner](../../scripts/ci/affected_scope.py) (`package_cells`)
still emits a `check` cell when a package has unchanged dependents but no
example or benchmark targets needing a check. That branch unconditionally
names `ubuntu-latest`, even though the scheduler removed it. The branch for
example and benchmark checks already respects the scheduled environments.

Replaying the planner for `a816f1447` with `--event push --proven-event
pull_request` reproduced the exact failing cells: `messenger` and
`test-toolkit`. Planning the same tree as a pull request does not trigger it, so
this reported failure appears after merge. The unguarded branch can also affect
other events without Linux cells, so the repair must follow scheduling rather
than special-case pushes.

## Outcome

- A push proven by its pull request plans no cells for a proven environment,
  including dependents checks, because the pull request compiled them on the
  same tree.
- A plan in which any cell names an unplanned environment is refused by the
  planner, rather than discovered later by the cell contract.
- The contract failure no longer leaves an empty artifact name that adds a
  second, misleading error in `Upload producer status`.

## Design

### Schedule the dependent check only where this event requires it

Use the scheduled cell environments passed to the repo-deps package's
[package cell planner](../../scripts/ci/affected_scope.py) (`package_cells`)
to decide whether the dependents-only check exists. Emit it only when that
list contains `DEPENDENTS_ENVIRONMENT`, currently `ubuntu-latest`. Preserve
the existing combined check when example or benchmark targets also need one;
there must be at most one check per package and environment.

The plan may retain Linux solely to build test archives for WSL2. Such a
producer appears in `environments` and `producing_environments` but owns no
Linux cells. Its presence must not enable a dependent check. Windows and WSL2
coverage remains scheduled by the event table; neither inherits Linux proof.

**Reader's note:** “a dependent check always runs” means that a *scheduled*
dependent check cannot reuse a local L1 test receipt, because that receipt
does not compile the unchanged consumers. It does not override event scheduling.
Removing an environment already validated by a qualifying pull request is a
different operation from reusing a local receipt for a cell that still exists.
Keep scheduled dependent checks `reusable: false`; do not turn them into reused
cells or re-add Linux to a proven push to satisfy the old unconditional branch.

The existing pull request validation verifier remains the authority for passing
`--proven-event pull_request`: it requires a completed successful run of this
repository's CI workflow with a matching tested tree and integration base.
This fix does not weaken that verification or infer proof from an event name.
Without qualifying proof, ordinary push planning still requires the check.

### Reject cells outside the plan's scheduled cell environments

Extend the repo-deps package's
[resolved-plan validator](../../scripts/ci/schema.py) (`validate_resolved_plan`)
so every cell, including reused or omitted cells, names an environment in the
plan's `environments` and not in `producing_environments`. The latter list
specifically identifies producers retained without cells. Checking only the
repository-wide list of recognized environment names is insufficient.

Use the existing `malformed-receipt` rejection code with a diagnostic naming
the package, environment, gate, and whether the environment is absent or
producer-only. Reject the plan; do not silently delete bad cells, since that
could hide missing coverage. Keep the existing consumer-side contract refusal
as defense for externally supplied or older invalid plans.

Call this validation before a freshly calculated plan is returned and before
the CLI writes `--plan-out` or prints a workflow projection. Invalid planning
must exit nonzero without publishing a new plan or dispatch document. Carried
plans entering the evidence overlay must receive the same check through the
shared validator, which that path already calls.

No required fields, document shapes, or evidence identities change. Keep the
resolved-plan schema version at 7 under the repository's
[schema version rules](../../docs/cicd/schema-versions.md); this enforces an
existing relationship rather than introducing a new format.

### Keep the original failure visible without an empty artifact name

The repo-deps package's [cell contract reader](../../scripts/ci/cell_contract.py)
publishes outputs only after the entire contract resolves. Preserve that
all-or-nothing behavior. In the check, native-test, and lint jobs of
[_package-ci.yml](../../.github/workflows/_package-ci.yml), and the WSL2 job of
[_wsl-ci.yml](../../.github/workflows/_wsl-ci.yml), require a nonempty
`steps.cell.outputs.status_artifact` in addition to the existing `always()`
condition for `Upload producer status`.

Status recording may still run for diagnostics, but a refused contract must
not upload an unnamed artifact or invent a fallback identity in the workflow.
Inspect the other uploads using contract outputs and apply the same nonempty
name guard where they can run after contract resolution fails. Keep diagnostic
uploads for failures *after successful resolution*, using the plan's existing
artifact names. The resolution step remains failed, no completion record is
created, and the failed producer continues to block through the existing gate.
Do not make the job successful by marking resolution optional.

## Scope

- Repair planning, strengthen the shared validator, and guard the affected
  workflow uploads as described above.
- Add focused planner, validator, and workflow contract regression tests.
- Correct `CLAUDE.md` (`AGENTS.md` is a symlink to it) and
  `docs/topics/ci-cd.md` where they say a dependent check “always runs” or
  “always executes.” Qualify that rule by event scheduling. Update the
  `rust-devops` skill's CI guidance to explain the same distinction.
- Review the changed functions' comments and documentation for the same
  unconditional wording; keep documentation aligned with behavior.

## Verification and acceptance

Use a small, self-contained planner fixture representing `a816f1447`: a changed
library, an unchanged direct consumer, and no example or benchmark targets.
The regression must not require fetching historical commits or calling GitHub.
Extend the existing event-scheduling tests, whose empty-push fixture currently
does not exercise this dependent-check branch.

| Case | Required result |
| --- | --- |
| Push with qualifying pull request proof | No cells, builds, dispatch rows, scheduled areas, or preflight runners; Linux and macOS remain recorded as proven under the current event table. Package and reverse-dependency facts may remain. |
| Pull request or push without proof | Exactly one Linux dependent check for the changed package; the unchanged consumer receives no cell or area. |
| Same package with an example or benchmark | One combined Linux check, with the dependent names retained; none in a proven environment. |
| Nightly with Linux retained solely for WSL2 archives | No Linux cells, including dependent checks; required archive builds and Windows/WSL2 cells remain. |
| Ordinary plan with accepted local L1 evidence | The scheduled dependent check still executes and remains non-reusable. |
| Hand-altered cell naming an absent or producer-only environment | Shared validation refuses it, including when entering the evidence overlay; legitimate producer-only build records remain valid. |
| Contract resolution fails before emitting outputs | The job stays failed, contract-derived uploads with empty names skip, and no completion record is produced. |
| Gate fails after successful contract resolution | Producer status still uploads under its existing name and reports failure. |

Exercise the planner's CLI refusal as well as validator return values: an invalid
calculated plan must not reach plan-file or standard-output publication. Use
existing workflow contract tests to pin the upload conditions across the four
producer jobs; do not launch a hosted CI run just to test expression behavior.

Run the focused Python suites through `scripts/ci/suite_runner.py` for
`test_affected_scope.py`, `test_schema.py`, and `test_resolved_plan.py`, plus
the test-toolkit workflow contract tests through the canonical Nextest-based
recipe and `actionlint` for edited workflows. Include any existing cell-contract
failure tests affected by the validator change. These are correctness checks;
no performance spike or full-workspace CI run is needed.

## Open Questions

None. The repair preserves the established scheduling, evidence, artifact, and
merge-gate contracts; it introduces no new policy choice.

## Not in scope

- The Oct 26 `coverage-audit` failure, which has a different cause. Investigate
  it only if it recurs.
- Porting the planner (`2026-11-06-rust-ci-planner`).

## Note on language

This is a small change to existing Python. Repairing current behavior is the
accepted exception to "no new Python" until the planner is ported.

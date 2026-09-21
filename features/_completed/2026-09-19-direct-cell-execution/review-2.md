---
$schema: feature-review.yaml
ready: false
findings:
  - title: Reusing a hostable L2 cell makes the resolved plan invalid
    priority: high
  - title: The schema-version follow-up still describes the obsolete v5 to v6 migration
    priority: medium
human_review: true
human_review_items:
  - |-
    After the implementation finding in this review is fixed, choose how to validate the all-at-once CI rollout before merging:

    - **A — Run one watched pull-request build (recommended).** Push the branch, compare every planned unit of work with the jobs and completion files that GitHub produced, and merge only if they agree.
    - **B — Restore a per-package-area rollout switch.** This permits a gradual rollout, but reintroduces two descriptions of what CI should run and therefore creates a new drift risk.
    - **C — Merge without the watched build.** This is fastest, but the first real validation occurs after the change reaches the main branch.

    The repository's `acceptance.md` gives the commands and pass conditions for option A. The reviewer is expected to select an option and, for option A, confirm that the observed GitHub jobs and completion files exactly match the plan.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-21T09:03:37-07:00"
spec: 2026-09-19-direct-cell-execution/spec.md
implemented: true
implemented_by: claude/default
log: features/2026-09-19-direct-cell-execution/implementation-log.md
description: "A **feature** review of `2026-09-19-direct-cell-execution/spec.md`"
feature: 2026-09-19-direct-cell-execution/review-2.md
previous: 2026-09-19-direct-cell-execution/review-1.md
next: 2026-09-19-direct-cell-execution/review-3.md
---

# Review 2

**Not production-ready.** Both high-priority findings from Review 1 are
implemented for executing cells: native L2 now carries a cell-specific backend
set and publishes the proof document consumed by `completion.py`, and canonical
selection validation now rejects the wrong-tier and inverted expressions from
the review. Review 1 contained no blocked-findings section, so no blocked item
could have become newly actionable between iterations.

The backend fix introduced a new failure in the evidence-reuse path. A passing
L2 receipt changes an executing cell to `reuse` without removing the new
execution-only `backends` field, producing a plan that the same schema version
rejects. Cross-OS and hosted-run evidence did not determine this readiness
verdict; the defect is deterministic in the Level 1 planner boundary.

## Findings

### High — Reusing a hostable L2 cell makes the resolved plan invalid

`package_cells` adds `backends` before it applies accepted evidence
(`scripts/ci/affected_scope.py:2611-2626`). `mark_reused` then changes the cell
to `execution: reuse` and removes `profile` and `requires_node`, but does not
remove `backends` (`scripts/ci/affected_scope.py:2467-2485`). The carried-plan
path has the same behavior because `apply_accepted_cells` calls the same helper
(`scripts/ci/affected_scope.py:3621-3627`). Schema v6 deliberately rejects that
shape: `backends` is legal only on an executing L2 cell
(`scripts/ci/schema.py:1675-1703`).

A direct planner-boundary reproduction using an L2 package that declares
`tmux` and `wezterm`, with passing macOS L2 evidence, produced:

```text
execution: reuse
state: reused
backends: [tmux]
```

Passing that cell to `_cell_backends` returns:

```text
malformed-receipt: cell a/macos-latest/L2 carries required backends but is not an executing L2 cell
```

This breaks a core feature contract: qualifying evidence must remove the
corresponding execution on every OS. A fresh calculation can emit the invalid
shape, and applying newly verified evidence to a carried plan can do the same.
The current suites stay green because the reuse fixtures either exercise L1 or
use packages whose L2 backend is unavailable on every scheduled environment;
none transitions a hostable L2 cell from `execute` to `reuse` after schema v6
added the field.

Remove execution-only backend requirements when a cell becomes reused, in the
same centralized transition that removes the profile and Node requirement. Add
Level 1 regressions for both paths: resolving from scratch with accepted L2
evidence and `apply_accepted_cells` over an executing plan. Each test should use
a hostable tmux cell and prove that the resulting plan validates, contains no
L2 dispatch row or build consumer for that cell, retains the evidence, and
carries no `backends` field on the reused cell.

### Medium — The schema-version follow-up still describes the obsolete v5 to v6 migration

The Review 1 fix correctly moved the resolved-plan schema to version 6, but the
feature handoff still says retiring `execution_path` would move the plan from
v5 to v6 (`spec.md:99-101` and `acceptance.md:261-265`). That follow-up would
now require version 7. The implementation log acknowledges that the schema was
bumped while calling the remaining work a v6 follow-up, so the stale text is
not merely historical phase narration.

Update the current spec handoff and acceptance follow-up to say v6 to v7. Keep
the phase-by-phase implementation history unchanged where it intentionally
records what version existed during that phase.

## Prior-review disposition

| Review 1 finding | Status in this iteration | Strongest verification |
|---|---|---|
| Native L2 cells cannot publish a completion record | Implemented for executing cells. The plan carries the hostable subset, the Rust sidecar writes `backend-proofs.json`, and native certification reads it. The reuse transition above is a new defect in that fix. | Level 1 real-planner/contract fixture, cross-language JSON fixture, Rust writer tests, and static shipped-workflow contracts. |
| Completion accepts a canonical marker from the wrong test tier | Implemented. All three expressions cited by Review 1 are refused, along with inverted, package-scoped wrong-tier, and ad hoc narrowing cases. | Level 1 shipped-filter invocation plus validator tests. |
| Blocked findings | Review 1 had no `## Blocked Findings` section and no blocked finding to reassess. | Document inspection. |

## Verification-level assessment

This feature changes CI planning, workflow dispatch, artifact certification,
and audit behavior. It introduces no keyboard, mouse, paste, IME, terminal
rendering, or browser-UI behavior, so Level 2 real-terminal capture and Level 3
OS input injection are not applicable. Level 1 executable and
workflow-contract tests are the appropriate local minimum; a hosted GitHub
Actions trial remains an operational human-review item.

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — one unique row per executing cell | Level 1 planner/schema fixtures and offline plan generation | **Gap:** applying valid L2 evidence creates a schema-invalid non-executing cell, so the final row source is not valid for this required state. |
| AC2 — consumers bind rows to the plan and accept no environment lists | Level 1 row-resolution tests and static workflow contracts | Appropriate and passing for valid plans. |
| AC3 — reused/gap-only areas retain their audit | Level 1 synthetic rollup and workflow-contract fixtures | **Gap for hostable L2 reuse:** existing reused-area fixtures do not exercise the new `backends` field. |
| AC4 — native and WSL2 consumers preserve archive/build identity | Level 1 plan, resolver, archive, and workflow contracts | Appropriate local boundary; hosted OS results are external evidence. |
| AC5 — producers prove the complete planned test set | Level 1 validator, shipped-filter, Rust proof-writer, and workflow-boundary fixtures | The two Review 1 producer defects are fixed at the appropriate level. |
| AC6 — audit rejects absent or mismatched proof | Level 1 Python-to-Rust audit fixtures | Appropriate and passing for plans that reach the audit. |
| AC7 — attribution, sibling isolation, and unchanged gate | Level 1 runner-loss and static workflow contracts | Appropriate and passing. |
| AC8 — budgets and focused hosted trial | Level 1 offline capacity checks; hosted trial not run | Capacity proof is appropriate. The hosted trial remains a human-review item and does not itself set readiness. |

## Verification performed

- Read the specification, acceptance evidence, implementation log, Review 1,
  the review-fix diff, planner, schema, cell resolver, completion validator,
  backend-proof writer, native/WSL workflows, and relevant tests.
- Refreshed GitNexus and analyzed the branch. The full change is high risk
  across six extracted CI flows; the review-fix symbols are low risk except
  plan validation, which has medium upstream impact across five readers. The
  graph could not resolve the Rust proof writer's caller, so its call from
  `backend-proof verify` was confirmed by text inspection.
- Reproduced the invalid reused-L2 shape directly through `package_cells` and
  confirmed that schema v6 rejects it.
- Ran `python3 scripts/ci/test_completion.py`: 59 passed.
- Ran `python3 scripts/ci/test_schema.py`: 140 passed.
- Ran `python3 scripts/ci/test_affected_scope.py`: 283 passed.
- Ran `python3 scripts/ci/test_resolved_plan.py`: 96 passed.
- Ran `just _test repo-deps`: 445 passed, 1 expected tier-filter skip.
- Ran `just _test test-toolkit`: 233 passed, 2 expected tier-filter skips.
- Ran `just _lint repo-deps` and `just _lint test-toolkit`: clean.
- Ran `actionlint` on `_package-ci.yml` and `_wsl-ci.yml`: clean.

## Production readiness

Not ready for production. The prior findings are addressed, but valid L2
evidence currently corrupts the resolved-plan state transition and defeats the
feature's evidence-reuse contract. Fix that transition, add both missing
Level 1 regressions, correct the schema-version handoff text, and repeat the
review before the watched hosted trial.

---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
  - |-
    Choose how to validate the all-at-once CI rollout before merging:

    - **A — Run one watched pull-request build (recommended).** Push the branch, compare every planned unit of work with the jobs and completion files that GitHub produced, and merge only if they agree.
    - **B — Restore a per-package-area rollout switch.** This permits a gradual rollout, but reintroduces two descriptions of what CI should run and therefore creates a new drift risk.
    - **C — Merge without the watched build.** This is fastest, but the first real validation occurs after the change reaches the main branch.

    `acceptance.md` in this feature's directory gives the commands and pass conditions for option A. Select an option and, for option A, confirm that the observed GitHub jobs and completion files exactly match the plan.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-21T09:36:29-07:00"
spec: 2026-09-19-direct-cell-execution/spec.md
implemented: false
description: "A **feature** review of `2026-09-19-direct-cell-execution/spec.md`"
feature: 2026-09-19-direct-cell-execution/review-3.md
previous: 2026-09-19-direct-cell-execution/review-2.md
---

# Review 3

**Production-ready.** Both findings from Review 2 are implemented at the
appropriate Level 1 boundary, and this iteration found no new implementation,
correctness, performance, ergonomics, or test-rigor defect. The watched hosted
trial remains a human-review item; under this review's closure rules, external
hosted and cross-OS proof does not determine implementation readiness.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/features/2026-09-19-direct-cell-execution/review-2.md`, does
not exist in this worktree. The canonical colocated review at
`features/2026-09-19-direct-cell-execution/review-2.md` was reviewed and
updated.

| Review 2 finding | Status in this iteration | Strongest verification |
|---|---|---|
| Reusing a hostable L2 cell makes the resolved plan invalid | Implemented. `mark_reused` removes the execution-only `backends` field for both fresh and carried-plan evidence paths. `package_cells` now attaches `backends` only after the execution decision, which also fixes the same invalid shape for prohibited cells. | Level 1 real-planner regressions validate the complete plan, row projection, build consumers, retained evidence, and unaffected sibling L2 execution. |
| The schema-version follow-up still describes the obsolete v5 to v6 migration | Implemented. The current spec handoff and acceptance follow-up now identify the optional `execution_path` retirement as schema v6 to v7. Historical phase narration remains unchanged. | Document inspection plus successful frontmatter/schema parsing recorded in the implementation log. |

Review 2 contains neither an `## Unblocked Findings` nor an
`## Blocked Findings` section. Its two findings are under `## Findings`, and
both are closed above. Review 1 likewise had no blocked-findings section, so no
previously blocked implementation finding became actionable before this cycle.

## Unblocked Findings

None.

## Blocked Findings

### Deferred — Watched hosted rollout

The focused GitHub Actions trial and the first executing WSL2-row observation
remain unperformed. The local implementation contains the plan comparison,
workflow contracts, artifact contracts, and acceptance commands needed to
judge the run. Producing the remaining evidence requires publishing the branch
and observing GitHub-hosted execution, which is outside this non-interactive
review's authorization.

This is retained as a human-review item rather than a readiness finding. The
review instructions explicitly exclude cross-OS evidence from readiness, and
the earlier review cycle already established this hosted trial as the external
rollout check rather than a substitute for deterministic implementation tests.

## Verification-level assessment

This feature changes CI planning, workflow dispatch, artifact certification,
and audit behavior. It introduces no keyboard, mouse, paste, IME, terminal
rendering, browser UI, or other behavior requiring Level 2 terminal capture or
Level 3 OS input injection. Executable Level 1 planner, subprocess, schema,
Rust contract, and static workflow tests are the appropriate local boundary.

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — one unique row per executing cell | Level 1 planner/schema fixtures and offline plan generation, including fresh and carried hostable-L2 reuse | Appropriate and passing. Reused and prohibited L2 cells now carry no execution-only backend contract, row, or build consumer. |
| AC2 — consumers bind rows to the plan and accept no environment lists | Level 1 row-resolution tests and static shipped-workflow contracts | Appropriate and passing. |
| AC3 — reused/gap-only areas retain their audit | Level 1 synthetic rollup and workflow-contract fixtures, plus the corrected hostable-L2 reuse plan | Appropriate and passing. |
| AC4 — native and WSL2 consumers preserve archive/build identity | Level 1 plan, resolver, archive, and workflow contracts | Appropriate local proof; hosted OS outcomes are external evidence. |
| AC5 — producers prove the complete planned test set | Level 1 validator, shipped-filter, backend-proof writer, and workflow-boundary fixtures | Appropriate and passing. Review 1's L2-proof and canonical-tier defects remain closed. |
| AC6 — audit rejects absent or mismatched proof | Level 1 Python-to-Rust audit fixtures | Appropriate and passing. |
| AC7 — attribution, sibling isolation, and unchanged gate | Level 1 runner-loss and static workflow contracts | Appropriate and passing. |
| AC8 — budgets and focused hosted trial | Level 1 offline capacity and workflow-limit checks; hosted trial deferred | Local implementation proof is appropriate and passing. The hosted observation remains the human-review item above and does not change readiness. |

## Verification performed

- Read the specification, acceptance evidence, implementation log, Reviews 1
  and 2, planner and schema transitions, new regressions, and affected reader
  contracts.
- Ran GitNexus change detection: 7 files and 16 symbols, no affected indexed
  processes, overall low risk, with no partial or truncated result. Upstream
  impact is exact and low for both changed planner symbols: `mark_reused` has
  two direct callers and `package_cells` has one.
- Ran `python3 scripts/ci/test_affected_scope.py ApplyAcceptedCellsTests`: 9
  passed.
- Ran `python3 scripts/ci/test_affected_scope.py`: 286 passed.
- Ran `python3 scripts/ci/test_schema.py`: 140 passed.
- Ran `python3 scripts/ci/test_resolved_plan.py`: 96 passed.
- Ran `just _test repo-deps`: 445 passed, 1 expected tier-filter skip.
- Ran `just _test test-toolkit`: 233 passed, 2 expected tier-filter skips.
- Ran `just _lint repo-deps` and `just _lint test-toolkit`: clean.
- Ran `git diff --check`: clean before the review metadata edits.

## Production readiness

Ready for production. The two Review 2 findings are closed with non-vacuous
Level 1 regressions at the real planner/schema boundary, the relevant package
tests and lints pass, and no new readiness finding was identified. Complete the
human-selected hosted rollout check before final specification closure.

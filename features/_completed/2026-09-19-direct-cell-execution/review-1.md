---
$schema: feature-review.yaml
ready: false
findings:
  - title: Native L2 cells cannot publish a completion record
    priority: high
  - title: Completion accepts a canonical marker from the wrong test tier
    priority: high
human_review: true
human_review_items:
  - |-
    After the implementation findings in this review are fixed, choose how to validate the all-at-once CI rollout before merging:

    - **A — Run one watched pull-request build (recommended).** Push the branch, compare every planned unit of work with the jobs and completion files that GitHub produced, and merge only if they agree.
    - **B — Restore a per-package-area rollout switch.** This permits a gradual rollout, but reintroduces two descriptions of what CI should run and therefore creates a new drift risk.
    - **C — Merge without the watched build.** This is fastest, but the first real validation occurs after the change reaches the main branch.

    The repository's `acceptance.md` gives the commands and pass conditions for option A. The reviewer is expected to select an option and, for option A, confirm that the observed GitHub jobs and completion files exactly match the plan.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T21:21:14-07:00"
spec: 2026-09-19-direct-cell-execution/spec.md
implemented: true
next: 2026-09-19-direct-cell-execution/review-2.md
description: "A **feature** review of `2026-09-19-direct-cell-execution/spec.md`"
feature: 2026-09-19-direct-cell-execution/review-1.md
log: features/2026-09-19-direct-cell-execution/implementation-log.md
implemented_by: claude/default
---

# Review 1

**Not production-ready.** The planner, row adapter, completion validator, audit,
and workflow contracts have broad Level 1 coverage, but the shipped native L2
workflow cannot satisfy the completion contract. Separately, the validator can
certify a run whose expected and observed test sets were both selected with the
wrong tier expression.

Cross-OS and hosted-run evidence did not determine this readiness verdict. The
two findings are deterministic implementation defects present in the shipped
workflow and validator. The requested watched GitHub Actions trial remains a
human-review item rather than a readiness finding.

## Findings

### High — Native L2 cells cannot publish a completion record

The native L2 workflow requires only the CI-hostable `tmux` backend when it
lists and runs tests (`.github/workflows/_package-ci.yml:707-712` and
`:769-780`). The expected manifest therefore records `backends: ["tmux"]`.
However, `completion.py` compares that list with every backend declared by the
package, without narrowing it to backends hostable in this cell
(`scripts/ci/completion.py:340-383`). Fourteen of the 22 L2 rows in a full
pull-request plan belong to packages that declare `tmux` plus one or more GUI
backends, so those cells are refused with
`completion-manifest-selection` even after their required tmux tests ran.

The remaining tmux-only rows cannot complete either. `completion.py` requires a
`{backend: {proven: true}}` document for every required backend
(`scripts/ci/completion.py:660-678`), but the workflow's certification command
does not pass `--backend-proofs` (`.github/workflows/_package-ci.yml:1002-1016`)
and no workflow step creates that JSON document. Consequently the validator
uses an empty proof map and returns `completion-backend-unproven` for every
tmux-only L2 cell. No native L2 row can upload the completion artifact required
by the area audit, so any change that schedules L2 makes the producer fail and
blocks `ci-gate`.

The existing Level 1 tests validate only hand-assembled matching inputs:
`test_a_required_backend_proof_absent_fails_and_present_passes` explicitly
passes a synthetic proof document, while
`test_declared_backends_matching_the_plan_complete` uses a tmux-only plan. The
workflow contract named `the_l2_consumer_still_provisions_and_proves_its_runtime_backend`
checks the recipe's internal backend-proof bracket, not the separate document
required by `completion.py`. All 49 completion tests and all 228
`test-toolkit` tests therefore pass while the real wiring is impossible.

Represent the backends required by each executing cell in the plan, rather
than treating every backend supported by the package as required on every
runner. Have the backend-proof tool emit or translate its execution evidence
into the document consumed by `completion.py`, and pass that document from the
native test workflow. Add a workflow-boundary L1 fixture that resolves a real
mixed-backend L2 row, runs the shipped proof/manifest wiring, and demonstrates
that tmux proof completes the cell while absent tmux proof does not.

### High — Completion accepts a canonical marker from the wrong test tier

`selection_problems` checks that each `test(...)` argument is *a* recognized
tier marker, but it never checks that the complete expression is the canonical
selection for the cell's gate (`scripts/ci/completion.py:183-240`). As a
result, all of these return no problems:

```text
L1       test(/(^|::)level2_/)
L2       test(/(^|::)browser_/)
browser  !(test(/(^|::)browser_/))
```

If `_tier_filter` drifts to one of those expressions, both `nextest list` and
`nextest run` use it. Their expected and observed identities agree, and
`completion.py` writes `complete: true` even though the cell ran the wrong
tier. This violates the producer-completeness requirement that the expected set
be derived independently of an ad hoc invocation filter and directly permits
the silent narrowing this feature is intended to eliminate.

The current Level 1 negative tests cover only an obviously ad hoc predicate
such as `test(green_one)`. `ShippedSelectionTests` also calls
`selection_problems` with an L1 cell for every tier expression, so it proves
that every marker belongs to the accepted vocabulary rather than that each
gate receives its own expression (`scripts/ci/test_completion.py:1339-1403`).

Bind the canonical filter identity to the plan/cell or compare the recorded
expression with a gate-specific canonical representation that cannot drift
with the invocation under test. Add negative cases for a valid marker from the
wrong tier, inverted inclusion, and a package-scoped wrong-tier expression;
retain positive cases for the L1 slow/performance variants and archive package
scoping.

## Verification-level assessment

This feature changes CI planning, workflow dispatch, artifact certification,
and audit behavior. It introduces no keyboard, mouse, paste, IME, terminal
rendering, or browser-UI behavior, so real-terminal Level 2 capture and Level 3
OS input injection are not applicable. Level 1 executable and workflow-contract
tests are the appropriate local minimum; a hosted GitHub Actions trial is
separate operational evidence.

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — one unique row per executing cell | Level 1 planner/schema fixtures and offline full-plan generation | Appropriate and passing. |
| AC2 — consumers bind rows to the plan and accept no environment lists | Level 1 row-resolution and static workflow contracts | Appropriate and passing. |
| AC3 — reused/gap-only areas retain their audit | Level 1 synthetic end-to-end rollup fixtures and workflow contracts | Appropriate and passing. |
| AC4 — native and WSL2 execution preserves archive/build identity | Level 1 plan, resolver, archive, and workflow contracts | Appropriate local boundary; hosted OS results are external evidence. |
| AC5 — producers prove the complete planned test set | Level 1 validator fixtures | **Gap:** native L2 workflow inputs cannot satisfy the tested validator contract, and wrong-tier canonical markers are accepted. |
| AC6 — audit rejects absent/mismatched proof | Level 1 Python-to-Rust end-to-end audit fixtures | Appropriate and passing for the artifacts supplied to it. |
| AC7 — attribution, sibling isolation, and unchanged gate | Level 1 runner-loss and static workflow contracts | Appropriate and passing. |
| AC8 — budgets and focused hosted trial | Level 1 offline capacity checks; hosted trial not run | Capacity proof is appropriate. The hosted trial is an external human-review item and does not set readiness here. |

## Verification performed

- Read the specification, plan, acceptance record, implementation log, planner,
  row resolver, completion validator, native/WSL2 workflows, audit workflow,
  rollup, and relevant tests.
- Refreshed the GitNexus index and inspected `package_cells` callers and
  dependencies. Its upstream impact is low and reaches the planner entry point
  plus the capacity/equality spikes.
- Generated a full pull-request plan and inspected all 22 native L2 rows: 14
  declare tmux plus GUI backends; 8 declare only tmux; the shipped completion
  command can certify neither group.
- Ran `python3 scripts/ci/test_completion.py`: 49 passed.
- Ran `just _test test-toolkit`: 228 passed, 2 pre-existing skips.
- Directly exercised `completion.selection_problems` with wrong-tier and
  negated tier expressions; each incorrectly returned an empty problem list.

## Production readiness

Not ready for production. Fix both high-severity findings and add the missing
workflow-boundary regressions before repeating the review. The watched hosted
trial should then be performed according to the human-review decision, without
using its absence as a substitute for the deterministic defects above.

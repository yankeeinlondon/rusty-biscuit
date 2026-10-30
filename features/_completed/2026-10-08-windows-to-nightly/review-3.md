---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Schema changelog retains feature references
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-12T06:14:52-07:00"
spec: 2026-10-08-windows-to-nightly/spec.md
implemented: false
description: "A **feature** review of `2026-10-08-windows-to-nightly/spec.md`"
feature: 2026-10-08-windows-to-nightly/review-3.md
previous: 2026-10-08-windows-to-nightly/review-2.md
---

# Review: Windows to Nightly — Iteration 3

The feature is **production ready** within the specification's delivery scope. The repair addresses the previous blocking finding: later qualifying failures now reopen coverage that the next nightly would otherwise inherit. No new high, medium, or low findings were identified.

This review includes the implementation at `HEAD` and the uncommitted repairs in the working tree. It checks the specification, both earlier reviews, the repair log, the planner and evidence readers, publication and recovery, seed execution, workflow wiring, document contracts, reporting, and their tests. Release enforcement and the optional Windows-risk selection interface remain explicitly deferred.

A cell is one package's test tier or check on one environment. Inherited coverage is proof credited from a successful nightly without scheduling that cell again.

## Previous findings

Review 2 contains one unblocked high finding and no blocked findings. The repair log marks it fixed and records no disputed findings. There was therefore no blocked finding to reopen or disputed finding to reconsider.

| Earlier finding | Repair verified | Result |
| --- | --- | --- |
| Review 2: Inherited nightly coverage hides later qualifying failures | The planner verifies would-be inherited cells, schedules the failed cell with its build and dispatch row, and certification treats it as current work | Fixed for native Windows and WSL2 |
| Review 1: Publication order can turn an older pass into a retry of a newer failure | Independent publications remain unlinked; rig retries identify their predecessor before execution; hosted retry links use run/attempt identity | Repair remains intact |
| Review 1: Nightly anchors inherit coverage without checking the required cell contract | Changed backends/features select work; certifications retain coverage contracts; legacy anchors inherit nothing | Repair remains intact |
| Review 1: Completed hosted observations depend on a later workflow-wide publisher | Retained artifacts are recovered before nightly planning, then made available to the shared verifier | Repair remains intact |
| Review 1: Seeding cannot execute an unproved tier beyond Level 1 | Unsupported tiers remain unproved, start no Level-1 leg, and make execution exit nonzero | Earlier review's documentation remedy remains implemented |

The seed's Level-1 limitation, the five-run automatic artifact-recovery window, and conservative migration from legacy anchors are documented limits already evaluated in review 2. They are not new requirements for this repair cycle.

## Unblocked Findings

None.

## Blocked Findings

None.

## Repair verification and bounded sweep

The earlier defect class was inherited passing coverage bypassing failure precedence. In the `repo-deps` package, [inherited_blockers](../../scripts/ci/affected_scope.py) asks the shared evidence resolver about cells before the planner inherits them. [blocking_evidence](../../scripts/ci/local_evidence.py) uses the same qualification and result-ordering logic as ordinary receipt verification. This keeps decisions about input identity, features, backends, and retries consistent across inherited and selected cells.

The regression fixtures use an isolated Git repository and bare remote. They create a successful certification, publish later failure reports through the shipped rig publication command, and follow planning through verification, overlay, dispatch, and certification. The sweep checked both nightly environments and the other selection branches:

| Site | Input or transition checked | Result |
| --- | --- | --- |
| Certified Windows and WSL2 inheritance | Later rig failure at the anchor; failure followed by documentation-only descendants | Only the failed package's cell reopens; unrelated coverage and the successful anchor remain |
| Certified inheritance controls | No later failure; complete passing retry explicitly linked to the failure | No package work; inherited coverage certifies successfully |
| Unordered outcomes | Independent pass and failure for the same inputs | Cell reopens; publication order grants no passing credit |
| Source-selected cells | Failure followed by a source fix | Changed package executes on both environments; unrelated package remains inherited |
| Policy-delta selection | New gate, backend, or test features | Changed coverage is selected before evidence verification |
| Bootstrap and legacy selection | No certifiable coverage contract | Required cells remain scheduled; neither path inherits unsupported proof |
| Evidence overlay and dispatch | Reopened failing cell | Failure diagnostic remains; the cell has its build and dispatch row |
| Certification writer | Reopened execution succeeds or fails; cell is both inherited and reopened | Success covers current execution; failure does not certify; contradictory inheritance is refused |
| Hosted recovery and workflow ordering | Completed producer artifacts with no collector; recovery before planner; artifact probe supplied to planner | Observations become discoverable before inheritance is decided; unavailable passing reports earn no credit |

The repaired function documentation explains why a successful anchor alone cannot settle later results. The current CI topic page describes reopening, passing-retry recovery, and preservation of unrelated inherited coverage. No comment drift affecting this behavior was found.

## Recurrence

No finding is re-raised. Review 2's recurrence sweep is now addressed: successful-anchor inheritance, evidence overlay onto the resulting plan, and certification of inherited versus reopened cells all participate in the repair. The `recurrence` flag is therefore false for this review.

## Verification

These requirements concern planning, files, Git evidence, and workflow orchestration. Level-1 in-process and isolated subprocess fixtures are appropriate. No terminal rendering or keyboard behavior was added that needs Level 2 or Level 3.

| Requirement | Strongest verification checked | Assessment |
| --- | --- | --- |
| Event cadence, opt-ins, empty PR-proven pushes, demanded WSL2 archive builds | Level 1 planner fixtures and executable workflow contracts | Appropriate |
| Bounded wait for unfinished matching PR validation | Level 1 public validation-reader and waiter fixtures | Appropriate |
| Separate nightly anchors, changed contracts, migration, empty unchanged plans | Level 1 planner and certification fixtures | Appropriate |
| Per-cell publication, concurrency, delayed results, retry lineage, failure precedence | Level 1 isolated Git repositories/remotes and public publication/verification | Appropriate |
| Later failures against inherited coverage | Level 1 publication → planner → verifier → dispatch/certification regression fixtures | Appropriate; passing |
| Hosted recovery after collector interruption and artifact expiry | Level 1 retained artifacts, simulated artifact service, and workflow contracts | Appropriate |
| Seed resume, unavailable rigs, unsupported tiers, native/WSL2 transport | Level 1 public orchestration and shell-transport fixtures | Appropriate |
| Malformed policy, notes, envelopes, anchor documents, certifications, and seed state | Level 1 field-matrix and strict-document tests through public results | Appropriate |
| Reporting empty, reused, gap-only, anchored, and reopened plans | Level 1 Rust reporting tests | Appropriate |

Input robustness checks include positive controls and edited documents for absent, null, wrong-type, malformed-element, empty, duplicate, and invalid-content cases. The changed contract fields are checked by the public anchor planner, certification reader, and resolved-plan validator: missing or malformed contracts cannot establish inherited coverage; reopened cells must be scheduled, required, and disjoint from inherited cells. Strict JSON loading rejects duplicate keys and trailing content. Empty lists retain their defined meaning instead of standing in for absent fields.

The Python suites are declared `repo-deps` companion suites. The Rust workflow contracts compile as a live Level-1 target in `test-toolkit`, and reporting tests run in the `repo-deps` `ci-rollup` binary. No added test is stranded behind an inactive tier.

Passing checks: 761 Python tests across the focused planner, policy, inheritance, evidence, publication, hosted recovery, local verification, validation, schema, seed, transport, and resolved-plan suites; 177 workflow contract tests through `BISCUIT_TEST_NARROW='binary(ci_workflow_contracts)' just test test-toolkit`; and 285 reporting tests through `BISCUIT_TEST_NARROW='binary(ci-rollup)' just test repo-deps`. Actionlint passed for the main, native, and WSL2 workflows, and `git diff --check` passed. The unchanged expensive whole-checkout source-input audit was not rerun.

No hosted workflow was dispatched, no terminal window was opened, and no production implementation was changed during this review. Hosted token permissions and cross-OS execution evidence remain deployment/CI checks rather than readiness findings, as requested.

## Observations

**Schema changelog retains feature references.** The new rows in [the CI schema changelog](../../docs/cicd/schema-versions.md) follow the existing table's convention of naming a dated feature in its `Source` column. That convention conflicts with the repository instruction that current documentation must not name feature snapshots. The rows already explain the contract changes directly, so this is a documentation-convention issue for the author to resolve separately; it does not affect scheduling or evidence correctness and requires no repair in this cycle.

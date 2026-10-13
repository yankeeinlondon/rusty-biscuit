---
$schema: feature-review.yaml
ready: false
findings:
  - "[high] Inherited nightly coverage hides later qualifying failures"
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-11T20:33:02-07:00"
spec: 2026-10-08-windows-to-nightly/spec.md
implemented: true
next: 2026-10-08-windows-to-nightly/review-3.md
implemented_by: claude/opus
log: features/2026-10-08-windows-to-nightly/log.md
description: "A **feature** review of `2026-10-08-windows-to-nightly/spec.md`"
feature: 2026-10-08-windows-to-nightly/review-2.md
previous: 2026-10-08-windows-to-nightly/review-1.md
---

# Review: Windows to Nightly — Iteration 2

The feature is **not production ready**. The earlier repairs pass their focused checks, but a later recorded failure cannot reopen coverage inherited from a successful nightly. The next nightly can execute nothing and certify that failing package as covered again.

A cell means one package's test tier or check on one environment. An inherited cell means coverage credited from a successful nightly without scheduling that cell in the current run.

This review includes the uncommitted repairs in the working tree, not just the implementation at `HEAD`. It checks the specification, both implementation logs, the first review, the changed planner and evidence paths, workflow wiring, seed transport, schema readers, and their tests. Release enforcement and the optional Windows-risk interface remain explicitly deferred.

## Previous findings

The first review uses `## Findings` rather than separate unblocked and blocked sections. All four findings were unblocked; there were no blocked findings to reopen and no disputed findings in the repair log.

| Earlier finding | Repair checked | Result |
| --- | --- | --- |
| Publication order can turn an older pass into a retry of a newer failure | Independent publications remain unlinked; rig retries name their predecessor before execution; hosted reruns use run/attempt identity | Original reproduction fixed |
| Nightly anchors inherit coverage without checking the required cell contract | Certifications retain coverage contracts; backend and feature changes select work; legacy anchors conservatively inherit nothing | Original reproductions fixed |
| Completed hosted observations depend on a later workflow-wide publisher | Nightly scope recovers retained artifacts before verification; the publisher also pushes recovered notes | Collector-absent native/WSL2 recovery fixtures pass |
| Seeding cannot execute an unproved tier beyond Level 1 | Unsupported tiers remain unproved, start no Level-1 leg, and produce a nonzero seed exit | Documentation remedy offered by the earlier review implemented |

The repair log explicitly records the conservative legacy-anchor migration, the five-run recovery window, and deferred execution of seed tiers above Level 1. These limits do not introduce another blocking finding.

## Unblocked Findings

### High — Inherited nightly coverage hides later qualifying failures

**Authority: spec.** “Nightly scope and recovery” requires that “a newer qualifying failure prevents reuse of an older pass for that cell, without erasing unrelated proof or changing the successful baseline.” The acceptance criteria also require that “a newer failure cannot be hidden by older passing evidence.” A developer running `just cross-check alpha --os windows` after a successful nightly, including after a documentation-only commit, is a realistic producer of such failure evidence.

**Defect class:** inherited passing coverage bypasses the evidence decision that would keep a later failing cell scheduled.

In the `repo-deps` package, [apply_policy_delta](../../scripts/ci/affected_scope.py:4057), which determines coverage inherited from a nightly, treats a matching coverage contract as sufficient proof and removes that cell from current work. [verify_cells](../../scripts/ci/local_evidence.py:455), which decides whether prior results satisfy the current plan, considers only `plan["cells"]`; inherited cells are outside that decision. It reports the rig failure, but supplies no instruction to reopen the missing cell. [apply_accepted_cells](../../scripts/ci/affected_scope.py:4727) carries the diagnostic without changing inheritance. Finally, [certification_document](../../scripts/ci/reuse_validation.py:499) copies that inherited package into the next successful certification. The workflow's [evidence overlay](../../.github/workflows/ci.yml:338) uses these same paths.

The reproduction combined the existing `RigCellFixture` and planner fixtures in a real temporary Git repository with a bare remote. It generated a successful anchor using the public certification writer, then published complete failing reports through the shipped `publish-receipt --kind rig` command for `alpha` on both nightly environments. Planning and public verification returned no executing cells despite `failed-cell` diagnostics. With the normal successful empty-run inputs, the writer returned `certified: true` and included `alpha` in `covered_cells` for both environments. Repeating after a documentation-only descendant commit produced the same result; `beta` remained unchanged throughout.

The bounded sweep checked both environments for every planning branch, the shared observation reader, the evidence overlay, and the certification projection:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Certified Windows inheritance | Complete later rig failure; same head and documentation-only descendant | No `alpha` execution; coverage inherited | Schedule only the failed cell |
| Certified WSL2 inheritance | Same two shapes | Same incorrect inheritance | Same selective execution |
| Source-selected cells, both environments | Same failures with the package selected by a source path | `alpha` stays executing; no passing credit | Same; clean |
| Policy-delta cells, both environments | Same failures; anchor has no coverage for `alpha` | `alpha` stays executing | Same; clean |
| Full/bootstrap cells, both environments | Same failures; no anchor | `alpha` stays executing | Same; clean |
| Legacy-anchor cells, both environments | Same failures; anchor records no contracts | `alpha` stays executing | Same; clean |
| Shared `verify_cells` reader | Real published rig notes against inherited and selected plans | Failure diagnosed in both; selected plan retains execution, inherited plan remains empty | Failure must also reopen inherited coverage |
| Evidence overlay | Empty certified plan plus the two failure diagnostics | Diagnostics retained; cells remain empty and inherited | Reopened failure cells must reach dispatch |
| Certification writer, both environments | Empty inherited plan after later failures; successful gate and zero required audits | `certified: true`; failing `alpha` is covered again | Do not certify unresolved failing coverage |

Make inherited coverage participate in the same qualifying-result and failure-precedence decision as selected cells. Preserve the successful comparison anchor and unaffected inherited packages; reopen only cells whose qualifying evidence requires execution. Keep this in the canonical plan rather than adding another scheduler. Add a regression that follows successful certification → later rig failure → nightly planning/verification → dispatch/certification, with controls for no failure, unrelated packages, changed inputs, and a genuine later passing retry.

## Blocked Findings

None.

## Recurrence

This repeats the broader failure-precedence class from review-1's **“Publication order can turn an older pass into a retry of a newer failure.”** Its specific publication-order defect is fixed. That repair swept publication, retry links, and the selected-cell verifier, but did not sweep successful-anchor inheritance, the overlay of failure diagnostics onto an empty plan, or the certification writer's inherited cells. Those sibling decisions also allow older passing coverage to suppress later failure. This review sweeps those paths for both native Windows and WSL2, including the clean selection branches listed above.

## Verification

These requirements concern planning, retained files, Git observations, and workflow orchestration. Level-1 in-process and isolated subprocess fixtures are appropriate. The feature adds no terminal rendering or keyboard behavior requiring Level 2 or Level 3.

| Requirement | Strongest verification checked | Assessment |
| --- | --- | --- |
| Event cadence, opt-ins, empty PR-proven pushes, demanded Linux archive producer | Level 1 event/planner tests and executable workflow contracts | Appropriate |
| Bounded wait for unfinished matching PR validation | Level 1 public validation-reader and waiter fixtures | Appropriate |
| Separate nightly anchors, changed coverage contracts, conservative migration | Level 1 public planner and certification-reader fixtures | Appropriate |
| Publication races, delayed results, explicit retry lineage | Level 1 real Git repositories/remotes and publication/verification fixtures | Appropriate |
| Recovery when the aggregate publisher never ran | Level 1 retained observation/report artifacts and simulated artifact API, plus workflow contracts | Appropriate |
| Seed resume, missing rigs, higher-tier refusal, native/WSL2 transport | Level 1 public orchestration and shell-transport fixtures | Appropriate |
| Malformed policy, observations, envelopes, anchors, certifications, and seed state | Level 1 public-result field matrices and strict-document tests | Appropriate |
| Reporting empty, reused, gap-only, and anchored plans | Level 1 Rust rollup tests | Appropriate |
| Later failures against inherited coverage | New review reproduction through rig publication, planner, verifier, overlay, and certification writer | Fails; blocking finding above |

Passing checks: 465 Python tests across the focused publication, rig-failure, hosted-observation, validation, seed, anchor, event, schema, input-matrix, policy, and cross-check suites; 177 `ci_workflow_contracts` tests through `BISCUIT_TEST_NARROW='binary(ci_workflow_contracts)' just test test-toolkit`; 15 focused `ci-rollup` tests through `just test repo-deps` with a binary/name filter; actionlint on the main/native/WSL2 workflows; and `git diff --check`. Python suites are declared `repo-deps` companion suites; the Rust workflow contracts compile as a live Level-1 test target in `test-toolkit`.

No full-workspace hosted workflow was dispatched, no terminal window was opened, and no production code was changed. Cross-OS execution evidence and human sign-off are outside this readiness decision.

## Observations

None newly raised. The first review's observation about an unchanged source-input audit remains outside this repair's scope.

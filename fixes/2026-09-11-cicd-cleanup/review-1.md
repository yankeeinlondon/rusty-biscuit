---
$schema: feature-review.yaml
ready: false
findings:
  - title: Receipt verification can credit the wrong environment
    priority: high
  - title: Manual full runs fail during scope setup
    priority: high
  - title: The hook rejects constraints already satisfied by evidence
    priority: high
  - title: Compile execution still uses Windows-only blanket target checks
    priority: high
  - title: Published receipts point to reports the hook does not retain
    priority: high
  - title: The standalone global policy verdict remains
    priority: high
  - title: Accepted-gap publication and hosted presentation verification are incomplete
    priority: high
  - title: Workflow regression contracts are not executed by CI
    priority: high
  - title: Constraints do not survive an unconfigured fresh session
    priority: high
  - title: Authoritative local scope evidence is not implemented
    priority: high
human_review: true
human_review_items:
  - |-
    Choose how pull requests must prove that their selected package areas passed before merging: require the CI workflow itself to pass, if supported, or require a fixed check that only combines the areas' results. Confirm the choice after a small test in a scratch repository; update branch protection together with the workflow migration.
  - |-
    Choose where a saved instruction such as “do not execute this environment during maintenance” lives: a persistent host directory, a shell setting, or a Git note. The specification recommends a host directory because a new terminal session must still find the instruction.
  - |-
    Review a small GitHub example showing an accepted coverage gap as cancelled versus neutral, and select the appearance you want. Cancelled matches the requested presentation; neutral avoids making an accepted gap look like an unsuccessful check.
  - |-
    Decide whether unchanged consumers of a modified library should still be compile-checked: remove that check, run it inside the changed area's job, or use a separate infrastructure job. The specification recommends keeping it inside the changed area's job.
reviewed_by: codex/gpt-6-astra
created: "2026-09-11T18:01:05-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: true
next: 2026-09-11-cicd-cleanup/review-2.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-1.md
log: fixes/2026-09-11-cicd-cleanup/log.md
implemented_by: claude/fable
---

> **Policy correction — 2026-09-12:** Ken clarified that every OS follows
> the same evidence rule: reuse qualifying passes; execute required tests when
> qualifying passing evidence is absent. Earlier WSL-ban interpretations and
> related authorization blockers in this historical record are superseded by
> the [spec ruling](spec.md#evidence-based-execution-ruling-2026-09-12).
> Historical observations and synthetic explicit-ban tests remain evidence of
> what was evaluated; they do not establish a current WSL prohibition.


# Review 1

**Not production-ready.** The current working-tree implementation improves area grouping, result attribution, and per-cell reuse, but contains reproducible execution and evidence defects and leaves required behavior unfinished. Human decisions are tracked separately; requiring human review is not itself the reason for this verdict.

Reviewed the specification, implementation plan, rollout notes, relevant Python/Rust/shell code, workflows, and contract tests, including the untracked `_area-ci.yml`. This review covers the implementation as present in the working tree, not only HEAD. No implementation files were changed.

## Findings

### High — Receipt verification can credit the wrong environment

`scripts/ci/local_evidence.py:225–293` selects a notes ref by environment, validates the receipt's structure, then labels accepted cells with the **ref's** environment without checking that `document.environment` agrees. Structural validation only checks that the declared environment is a recognized value.

A focused in-process probe supplied a valid receipt declaring `macos-latest` from the `wsl2-ubuntu` notes lookup. `verify_cells()` accepted `alpha/wsl2-ubuntu/L1` with no rejection. This can suppress required WSL execution using macOS results, violating §3 and AC5/7/12. The probe mocked Git reads, not receipt validation or the verifier.

Reject a receipt whose declared environment differs from its notes ref. Also validate that its declared tested head/tree agree with the commit carrying the note before accepting exact identity or equivalence. Add mismatched-environment and mismatched-revision fixtures that assert no execution is suppressed.

### High — Manual full runs fail during scope setup

In `.github/workflows/ci.yml:101–125`, `workflow_dispatch` sets `scope_args=(--all)` but never initializes `base` or `head`. The shared code then expands both under `set -u`.

Executing the actual extracted scope-setup block with `EVENT_NAME=workflow_dispatch` fails with exit 1 and `base: unbound variable`, before invoking the planner. Explicit full runs therefore cannot reach any area (required behavior §1.8).

Initialize a valid revision pair for dispatch, or omit those arguments under a documented full-run identity contract. Test the executable scope script for dispatch, PR, ordinary push, and zero-base push events; planner-only tests cannot catch this workflow-shell failure.

### High — The hook rejects constraints already satisfied by evidence

`.githooks/pre-push:67–73` invokes `constraints.py check` before calculating a plan and supplies no `--plan`. `scripts/ci/constraints.py:155–165` deliberately treats every active constraint as unsatisfied when no plan is supplied. The hook therefore blocks even a push whose WSL cells are all reused or whose scope contains no WSL work. Its omission of repository identity also prevents reliable filtering of repository-specific records.

A temporary-fixture probe returned exit 1 for the hook-shaped invocation, and exit 0 for the same constraint with a plan containing only a reused WSL cell. This violates §4 and AC17's requirement to decide from the resolved executions.

Resolve and verify the outgoing plan before evaluating constraints, pass repository and branch identity, and test the real hook with reused, absent, and executing prohibited cells. Preserve fail-closed behavior for unreadable plans.

### High — Compile execution still uses Windows-only blanket target checks

`scripts/ci/affected_scope.py:150` defines `CHECK_ENVIRONMENT = "windows-latest"`; `resolved_cells` creates supplemental example/bench coverage only there, and `matrix_record` projects that same singleton. `_package-ci.yml:181–182` still executes `cargo check --all-targets`. Its `check_args` contain package/features, not explicit target selectors.

Thus the plan labels supplemental coverage by target kind without controlling the command's target selection. It still recompiles L1-covered kinds and provides no corresponding supplemental Linux/macOS example or bench check, contrary to §1.5–1.7 and AC3. L1 compilation does not close a missing example/bench check.

Generate explicit selectors for uncovered target kinds on the applicable native build environments, credit WSL's builder honestly, and consume those selectors in the workflow. `TargetCoverageTests.test_no_matrix_entry_asks_for_every_target_by_flag` only checks planner argument strings, so it passes while the workflow injects the banned flag. Test the final command and environment coverage.

### High — Published receipts point to reports the hook does not retain

`.githooks/pre-push:80–81` stages reports in a temporary directory and deletes that directory on exit. Its `record-cells` call supplies no persistent report destination. `scripts/ci/local_evidence.py:596–597` nevertheless defaults the receipt's `host.report_dir` to `~/.rusty-biscuit/ci-evidence/<head>` without copying reports there.

The receipt preserves counts but its advertised underlying reports disappear after the hook exits. This violates §2's explicit local-report retention/provenance contract and prevents investigation of a reused failure from the named location.

Persist the staged reports before publishing the receipt, pass their actual durable location, and test that each referenced report remains readable after hook cleanup. Do not fabricate a fallback provenance path.

### High — The standalone global policy verdict remains

`.github/workflows/ci.yml` still declares `ci-verdict`, rebuilding and evaluating global results in addition to `_area-ci.yml`'s per-area rollups. This is an explicit unmet §5/AC11 contract, even though retaining it during migration is safer than prematurely deleting the required check.

The workflow tests `no_standalone_global_verdict_job_remains` and `the_verdict_consumers_are_rewired_when_the_job_goes` remain pending-contract wrappers: their success confirms the requested behavior is still absent. Complete the selected merge-gate mechanism, its scratch-repository verification, consumer migration, and coordinated ruleset transition. Do not remove the current check in isolation.

### High — Accepted-gap publication and hosted presentation verification are incomplete

The planner and rollup implement an `ACCEPTED GAP` result, but no publisher creates the requested immediate cancelled/neutral check. `_area-ci.yml` renders results only after `package-ci` finishes; its permissions are read-only. `the_gap_publishing_job_holds_the_checks_write_permission_it_needs` explicitly remains pending.

The rollout document records no live run of the new workflow or controlled GitHub presentation fixture. Source-string assertions cannot establish Actions graph grouping, skipped-label expansion, visibility of reused cells, or accepted-gap effects on the run conclusion. These are user-observable requirements in §2/§6 and AC2/4/9/10, with the wrong strongest verification level currently present.

Implement gap publication and run the prescribed small hosted fixture, capturing its graph/check results and machine-readable outcomes. This is a product presentation/integration gap, not a demand for cross-OS test results. Terminal L2/L3 tests would not verify GitHub behavior.

### High — Workflow regression contracts are not executed by CI

The fix adds essential workflow contracts in `tools/test-toolkit/tests/ci_workflow_contracts.rs`, but `tools/test-toolkit/Cargo.toml:51` has `gates = false`. Neither preflight nor the `ci-tooling` job invokes that binary; the latter executes Python suites plus `ci-rollup` and `ci-plan` only.

The exclusion predates this fix, but reliance on the newly added contracts is directly in scope. Manual passes cannot guard subsequent workflow changes. Add a focused Nextest invocation for this integration-test binary to the relevant infrastructure gate, without requiring a broad package-promotion decision. Retain live-workflow verification separately; running static contracts automatically does not raise their verification level.

### High — Constraints do not survive an unconfigured fresh session

`scripts/ci/constraints.py:43–56` returns an empty default directory, and `.githooks/pre-push:67` only consults the store when `BISCUIT_CI_CONSTRAINTS_DIR` is set. A later shell without that variable silently finds no restrictions, even if a previous session saved them elsewhere. This leaves §4's motivating failure unresolved.

Finish the persistent discovery mechanism after the recorded OQ2 decision, and test recording in one session followed by planning/pushing in a fresh environment. The configured-directory tests verify record evaluation, not cross-session discovery. This is separate from the erroneous rejection of satisfied constraints above.

### High — Authoritative local scope evidence is not implemented

The specification inherits distinct scope and validation documents and requires a matching local scope document to be authoritative. Instead, the scope-only hook exits after `just ci-local --plan`, and `.github/workflows/ci.yml:101–143` computes scope in CI, then computes it again when accepted validation cells exist. No standalone local scope receipt is published or consumed.

The plan itself records this as a narrowing. A validation receipt's `scope_identity` is not an authoritative scope document. Implement the inherited exact base/head/tree/schema scope handoff and test both a matching hit and a rejected/missing fallback. Do not describe the predecessor contract as complete merely because validation evidence reuse works.

## Requirement-to-verification map

“L1” below includes in-process logic tests and manufactured subprocess fixtures. It does not imply that these suites were rerun during this review. GitHub rendering and branch protection require a hosted integration fixture; terminal IPC and OS keyboard injection are not applicable to this CI feature.

| Requirement | Strongest verification present in the inspected implementation | Assessment |
|---|---|---|
| AC1: impacted-area selection, no reverse-dependent entries | L1 planner fixtures | Selection covered; downstream seam decision remains open. |
| AC2: area grouping and nested areas | L1 planner/workflow assertions | Hosted grouping verification missing. |
| AC3: relevant-target compile coverage | L1 target metadata assertions | Final command and environment contract broken. |
| AC4: visible completed local results, measurements, provenance | L1 receipt/rollup fixtures | Hosted visibility unverified; report retention broken. |
| AC5: macOS plus WSL reuse and cross-check receipts | L1 multi-environment and cross-check fixtures | Wrong-environment rejection missing. |
| AC6: seven reused macOS cells do not become MISSING | L1 `the_pr_76_macos_cells_must_not_resolve_to_missing` | Appropriate logic-level regression; hosted handoff unverified. |
| AC7: reject bad evidence; block only prohibited executions | L1 rejection, input-identity, constraint fixtures | Receipt identity and hook integration defects remain. |
| AC8: failed results block their owning area | L1 area rollup/verdict fixtures | Appropriate policy tests; hosted failure propagation unverified. |
| AC9: accepted/expired/revoked gaps and visible explanation | L1 gap-state tests; pending publication contract | Publisher and hosted integration missing. |
| AC10: expanded labels and environment-qualified lint | L1 workflow/label assertions | Real GitHub display verification missing. |
| AC11: area-owned merge authority | L1 area narrowing; pending removal contracts | Global verdict remains; scratch fixture absent. |
| AC12: successful PR-validation reuse | L1 `test_reuse_validation.py` and workflow assertions | Changed hosted result chain not exercised end to end. |
| AC13: worker budgets and overrides | L1 thread policy and forwarding tests | Appropriate level for calculation/argument forwarding. |
| AC14: documentation matches implemented behavior | L1 documentation assertions | Limitations documented, but final migration is incomplete and contracts are not gated. |
| AC15: mapping agrees with sniff | Local subprocess drift test when sniff exists | One explicitly tracked sniff inconsistency; this is not evidence of universal equality. |
| AC16: v1 exact-tree, unmeasured migration | L1 legacy receipt fixtures | Appropriate migration tests present. |
| AC17: persisted constraints and plan display | L1 constraint/renderer fixtures | Hook integration and fresh-session discovery incomplete. |
| Inherited authoritative scope handoff | No completed handoff fixture | Missing implementation. |

## Verification performed for this review

- Extracted and executed the workflow's dispatch scope-setup shell: reproduced exit 1 before planning.
- Ran the real constraint checker against temporary static JSON fixtures: the hook-shaped invocation blocked; supplying a fully reused plan passed.
- Ran the real receipt verifier with fixture documents and mocked Git reads: a macOS receipt was credited to WSL without a rejection.
- Inspected the tests and their workflow invocation paths. Prior rollout suite counts are historical claims, not fresh results from this review. No full-workspace or WSL suites, pushes, workflow dispatches, ruleset writes, or commits were performed.

GitNexus was queried for CI flows and impact on the spec metadata edit. Its index is 65 commits behind and the file impact result was `UNKNOWN`, with no resolved callers/processes; this was not treated as clearance. Text inspection confirmed the document references, and only the review artifact and requested `review_iterations` property were edited. The requested empty skill name could not be resolved; the relevant CI, testing, exploration, and file-reference skills were used.

---
$schema: feature-review.yaml
ready: false
findings:
  - title: Trigger review uses the wrong comparison base for stacked pull requests
    priority: high
  - title: The standalone global policy verdict remains
    priority: high
  - title: Accepted-gap publication and hosted presentation verification remain incomplete
    priority: high
  - title: Constraints still disappear in an unconfigured fresh session
    priority: high
human_review: true
human_review_items:
  - |-
    Choose how pull requests become eligible to merge: require the CI workflow to pass, if a disposable-repository experiment proves that works, or require one fixed check that only combines the individual area results. Approve the corresponding branch-protection change after the experiment and implementation are ready.
  - |-
    Choose where saved instructions such as “do not execute this environment during maintenance” should live: a persistent directory on the computer, a shell setting, or a Git note. The specification recommends a persistent directory so a fresh terminal session still finds the instruction.
  - |-
    Once the small GitHub example is available, choose whether an accepted coverage gap should appear as cancelled or neutral. Cancelled matches the requested appearance; neutral avoids presenting an accepted gap as an unsuccessful check.
  - |-
    Decide whether unchanged applications using a modified library should receive a compile check: remove that check, run it inside the changed area's job, or use a separate infrastructure job. The specification recommends keeping it inside the changed area's job.
reviewed_by: codex/gpt-6-astra
created: "2026-09-11T23:18:20-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: true
implemented_by: claude/fable
log: fixes/2026-09-11-cicd-cleanup/log.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-4.md
previous: 2026-09-11-cicd-cleanup/review-3.md
next: 2026-09-11-cicd-cleanup/review-5.md
---

> **Policy correction — 2026-09-12:** Ken clarified that every OS follows
> the same evidence rule: reuse qualifying passes; execute required tests when
> qualifying passing evidence is absent. Earlier WSL-ban interpretations and
> related authorization blockers in this historical record are superseded by
> the [spec ruling](spec.md#evidence-based-execution-ruling-2026-09-12).
> Historical observations and synthetic explicit-ban tests remain evidence of
> what was evaluated; they do not establish a current WSL prohibition.


# Review 4

**Not production-ready.** The per-ref constraint checks and default-Bash harness failure from review 3 are fixed. Three explicitly deferred requirements remain incomplete, and this review reproduces another trigger-scope mismatch. Passing pending-contract tests do not establish completion.

Reviewed the working tree at `e4279b59a`, including the uncommitted review-3 implementation. This review changes only its output document, the previous review's metadata, and the specification's iteration count. Missing cross-OS results and the need for human decisions are not themselves readiness failures; the findings concern implementation defects, unimplemented requirements, and verification at an insufficient boundary.

## Findings

### High — Trigger review uses the wrong comparison base for stacked pull requests

`.githooks/pre-push:244–265` assumes every non-main branch is validated against `origin/main`. Its comment expressly claims this is the CI event's comparison base. However, `.github/workflows/ci.yml:11–20` deliberately supports pull requests targeting other branches, and `:111–122` computes the changed paths from the actual `PR_BASE` and `PR_HEAD`. Reviewing every pushed ref does not fix disagreement about what that ref will execute.

A temporary Git fixture reproduced the omission using the real `calculate_scope` and `constraints.unsatisfied` implementations, with static package/environment metadata from the existing planner tests:

1. Main has `alpha/lib/src/lib.rs` returning `1`.
2. A parent branch changes it to return `2`.
3. A child branch restores `1` and targets the parent in a stacked PR.
4. The hook's `merge-base origin/main HEAD` comparison yields no changed paths, no selected source packages, and `blocked=False` for a WSL prohibition.
5. CI's parent-to-child comparison yields `alpha/lib/src/lib.rs`, selects `alpha-core`, and gives `blocked=True` for the same prohibition.

This probe exercised real Git, planner selection, and constraint evaluation; it did not create a hosted PR or invoke the full hook. The hook's hard-coded base and the workflow's actual-base comparison are directly visible in the cited code. When the scope receipt misses because its base differs, CI recalculates the larger plan after the push has already been permitted. That violates §4 and AC7/17.

Resolve the actual target/base of each workflow the branch update can trigger, and check the corresponding plan before permitting the push. When that identity cannot be established, fail closed with a specific explanation instead of claiming the main-based plan is authoritative. Add an L1 real-hook regression for this parent-change/child-revert case, plus coverage for a target branch advancing. Correct the base-contract comment in the same implementation change. The new multi-ref tests manufacture the output plan and do not establish parity with an actual PR base.

### High — The standalone global policy verdict remains

`.github/workflows/ci.yml:501` still declares `ci-verdict`; `:680` still invokes the whole-run policy verdict after the individual areas have applied their own policies. This remains contrary to §5 and AC11. The removal and consumer-migration tests at `tools/test-toolkit/tests/ci_workflow_contracts.rs:2454` and `:2481` still use `pending_workflow_contract`, which passes when the required behavior fails for its recorded reason.

Complete the chosen merge mechanism and consumer migration, exercise the scratch-repository fixture, and coordinate the required-check transition. Keep the current gate until that transition is ready. The unresolved design choice explains the deferral but does not satisfy the implementation contract. This is carried forward from review 3.

### High — Accepted-gap publication and hosted presentation verification remain incomplete

The planner and rollup retain a distinct `ACCEPTED GAP` state, but the immediate check publisher is still absent. `_area-ci.yml:95–103` waits for `package-ci` before reporting and grants only `checks: read`. The publisher permission test at `ci_workflow_contracts.rs:2679` remains pending. Reporting the gap after the producers finish does not implement §6 and AC9's immediate explained result without starting tests for that gap.

`rollout-2026-09-11.md` remains `status: not-triggered`, and the review-3 implementation log explicitly defers the hosted experiment. L1 planner, extracted-shell, Rust rendering, and workflow-source assertions do not verify actual Actions grouping, displayed reused results, skipped labels, cancellation effects, or merge protection. AC2/4/8/9/10/11 and the final hosted result chain in AC12 require the specified controlled hosted integration fixture. This is a verification-boundary mismatch, not missing cross-OS test evidence.

Implement publication after the display decision and run the controlled branch/scratch-repository fixtures when authorized. Record actual checks, displayed labels, run conclusions, and merge decisions for mixed reused/executing cells, an accepted gap, a failed selected area, and an unselected area. Promote the pending contracts to ordinary assertions. This is carried forward from review 3.

### High — Constraints still disappear in an unconfigured fresh session

`scripts/ci/constraints.py:53–66` still has an empty `default_directory()`. With `BISCUIT_CI_CONSTRAINTS_DIR` absent, `load` receives an empty directory and returns no active constraints. The same persisted prohibition can therefore block a configured session and become invisible in a fresh session. The new repeatable branch argument fixes identity matching inside a configured store, not discovery of that store.

Complete the selected persistence/discovery mechanism and add a fixture that records a prohibition in one session and then runs both the plan preview and hook in a fresh environment without the variable. Preserve expiry, repository/branch matching, and fail-closed malformed-record handling. This remains a §4/AC17 gap carried forward from review 3.

## Prior-review disposition

| Review-3 finding | Assessment |
|---|---|
| Every pushed ref and its branch constraints | Fixed for ref iteration, ordering, destination repository, other-branch pushes, and renamed refspecs. The distinct actual-PR-base defect is described above. |
| Standalone global verdict | Deferred; still a finding. |
| Gap publication and hosted verification | Deferred; still a finding. |
| Fresh-session constraint discovery | Deferred; still a finding. |
| Default macOS Bash abort | Fixed. The direct shebang invocation now completes all 45 hook tests, including the no-selection path. |

The implementation log accurately says two findings were fixed and three deferred. Its “Successful Completion” heading should not be interpreted as completion of the specification. No additional performance or API-ergonomics finding was established.

## Requirement-to-verification map

L1 here includes in-process tests and manufactured subprocess/Git/filesystem fixtures. Hosted integration is the appropriate boundary for GitHub presentation and merge protection. Terminal IPC L2 and OS-keyboard L3 do not prove those behaviors and are not required by this specification. The plan command's data/status contract is covered at L1; the specification does not prescribe terminal glyph, color, input, or scrolling behavior.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1: impacted areas without unchanged dependent jobs | L1 planner fixtures | Appropriate selection coverage; OQ1 remains a design choice. |
| AC2: one visible identity per area, including nested areas | L1 mapping/workflow contracts | Hosted presentation missing; finding above. |
| AC3: native target coverage after local macOS reuse | L1 planner/command contracts | Appropriate logic coverage; OS results remain CI's responsibility. |
| AC4: visible reused results, measurements/provenance, no rerun | L1 receipt, scheduling, retention, and rollup tests | Logic covered; hosted visibility missing. |
| AC5: mixed macOS/WSL evidence and cross-check receipts | L1 real-Git/subprocess fixtures | Appropriate evidence-boundary coverage. |
| AC6: seven reused macOS cells do not become MISSING | L1 Rust regression | Appropriate; passed. |
| AC7: evidence rejection and prohibited-trigger blocking | L1 evidence/constraint/hook fixtures | Actual-PR-base omission remains. |
| AC8: local/CI failure blocks its owning area | L1 rollup/verdict tests | Policy covered; hosted propagation unverified. |
| AC9: immediate explained gaps and expiry/cancellation semantics | L1 policy/rollup and pending publisher contract | Publisher and hosted integration missing. |
| AC10: resolved labels and environment-qualified lint | L1 workflow/label contracts | Actual hosted rendering unverified. |
| AC11: area-owned merge authority | L1 area narrowing and pending migration contracts | Global policy job and scratch-fixture gap remain. |
| AC12: successful-validation reuse; failure/cancellation rejection | L1 reuse-validation/evidence contracts | Logic covered; final hosted chain incomplete. |
| AC13: concurrency boundaries, isolated L2 forwarding, overrides | L1 worker/argument fixtures | Appropriate; passed. |
| AC14: documentation and skill accuracy | Source review | Deferrals documented; incorrect hook base claim needs correction. |
| AC15: mapping agrees with sniff | Conditional subprocess drift test | Appropriate boundary; suite passed, comparison is conditional on sniff availability. |
| AC16: v1 exact identity and unrecorded measurements | L1 real-Git and Rust rendering fixtures | Appropriate; passed. |
| AC17: persisted restrictions and reviewable plan | L1 checker, renderer, and hook tests | Fresh-session discovery and actual-PR-base handling incomplete. |

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 400 passed in 71.904 seconds.
- `env -u CDPATH ./.githooks/tests/test-pre-push.sh </dev/null`: 45 passed, 0 failed.
- `just test test-toolkit`: 155 passed, 2 skipped. Pending contracts within the passing set verify continued incompleteness, not implemented behavior.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: 183 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan`: 9 passed.
- `shellcheck .githooks/pre-push`: passed.
- `actionlint` on the four orchestration workflows: exit 1 with the six previously reported SC2086 informational diagnostics in native-prerequisite commands; not a newly introduced finding.
- Temporary Git/planner/constraint probe reproduced the stacked-PR scope mismatch described above.

No full-workspace suite, WSL run, terminal window, hosted workflow trigger, repository commit, hosted push, or ruleset change was performed.

GitNexus was queried for CI flows and document impact. The index identifies this worktree at the current HEAD, but does not represent the uncommitted implementation as verified graph changes. The document impacts were `UNKNOWN` with no resolved callers/processes; text inspection confirmed their review-chain and plan references. No code symbols were edited.

The supplied previous-review reference under `prompts/_reviews/fixes/` does not resolve through `bf reference`. The existing previous review resolves at `fixes/2026-09-11-cicd-cleanup/review-3.md`; its `next` now points to this review and its `implemented: true` is preserved. The specification records `review_iterations: 4`.

---
$schema: feature-review.yaml
ready: false
findings:
  - title: Simultaneous PR head and target updates bypass the target-aware constraint check
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
    Choose how pull requests become eligible to merge: require the CI workflow to pass, if a disposable-repository experiment proves that works, or require one fixed check that only combines individual area results. Approve the branch-protection change after the experiment and implementation are ready.
  - |-
    Choose where saved instructions such as “do not run WSL again” should live: a persistent directory on the computer, a shell setting, or a Git note. The specification recommends a persistent directory so a fresh terminal session still finds the instruction.
  - |-
    After a small GitHub example demonstrates both options, choose whether an accepted coverage gap should appear as cancelled or neutral. Cancelled matches the requested appearance; neutral avoids presenting an accepted gap as an unsuccessful check.
  - |-
    Decide whether unchanged applications using a modified library should receive a compile check: remove that check, run it inside the changed area's job, or use a separate infrastructure job. The specification recommends keeping it inside the changed area's job.
reviewed_by: codex/gpt-6-astra
created: "2026-09-12T00:06:05-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: true
implemented_by: claude/fable
log: fixes/2026-09-11-cicd-cleanup/log.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-5.md
previous: 2026-09-11-cicd-cleanup/review-4.md
next: 2026-09-11-cicd-cleanup/review-6.md
rulings: 2026-09-11-cicd-cleanup/spec.md#rulings
ruled_on: "2026-09-12"
---

# Review 5

**Not production-ready.** The ordinary stacked-PR and advanced-target cases from review 4 are fixed. A simultaneous target/head update still permits a prohibited execution, and three prior findings remain explicitly deferred. Pending-contract tests passing does not mean those requirements are implemented.

Reviewed HEAD `a69c02c5cd363bc7f05c40fc50e05ac2c26001e4` plus the existing uncommitted review-4 implementation. This review changes only review documents and the specification's iteration count. Missing cross-OS results and human decisions are not themselves readiness failures; the findings concern defective behavior, missing implementation, and verification at the wrong boundary.

## Findings

### High — Simultaneous PR head and target updates bypass the target-aware constraint check

`.githooks/pre-push:211–224` resolves a PR target only with `git ls-remote`; `:265–269` uses that result even when the target branch is also being updated in the captured push. Reviewing each update separately does not account for their combined resulting state. `.github/workflows/ci.yml:111–122` instead plans from the event's actual base/head revisions.

A temporary extension of the existing L1 hook harness reproduced this:

1. Remote `main` and `parent` have the original library contents.
2. Local `parent` changes `pkg/alpha/src/lib.rs`; local `child` descends from it and restores the original contents.
3. A manufactured open PR has head `child` and target `parent`. A persisted WSL prohibition applies only to `child`.
4. Feed the real hook both updates, parent first and child second, in one stdin payload.
5. The parent plan selects work but has no matching branch prohibition. The child plan compares against the **old remote parent**, gets an empty path set, and the hook exits **0**, publishing a scope receipt to the fixture's bare repository.
6. Diffing the incoming parent against the incoming child produces `pkg/alpha/src/lib.rs`. Once both updates land, that is the PR delta the workflow can plan, selecting the prohibited cell.

The probe used the shipped hook, real Git repositories, real evidence and constraint code, and the existing static planner fixture and stubbed `gh`. The assertion expecting exit 1 failed with exit 0. No hosted push was made; hosted event behavior is inferred from the workflow's explicit event-base contract, not claimed as an observed run.

Resolve target branches against the entire incoming ref-update set as well as the current remote state. Check the incoming target revision when present; if multiple event states can occur, conservatively check each applicable state or refuse the ambiguous update. Handle deleted targets explicitly. Add a permanent regression with a child-only prohibition, both ref orders, and a passing control where no prohibited work is selected. This is an AC7/17 correctness defect independent of the open design questions.

### High — The standalone global policy verdict remains

`.github/workflows/ci.yml:501` still declares `ci-verdict`, and `:680` invokes the whole-run policy verdict after area rollups have already applied policy. This violates §5 and AC11. The removal and migration contracts at `tools/test-toolkit/tests/ci_workflow_contracts.rs:2455` and `:2481` remain wrapped in `pending_workflow_contract`, which expects the recorded incompleteness.

Complete the selected merge mechanism and consumer migration, prove it in the scratch-repository fixture, and coordinate the required-check transition. Preserve the existing gate until the replacement is ready. The specification explicitly gates this section on OQ3; respecting that instruction is appropriate, but does not make the implementation complete. Carried forward from review 4.

### High — Accepted-gap publication and hosted presentation verification remain incomplete

The distinct `ACCEPTED GAP` result state exists, but immediate check publication still does not. `_area-ci.yml:95–103` waits for `package-ci` and grants its rollup only `checks: read`; the publisher contract at `ci_workflow_contracts.rs:2680` remains pending. A later summary does not satisfy §6/AC9's immediate explained result.

`rollout-2026-09-11.md:10` still records `status: not-triggered`, and the latest implementation log explicitly defers the hosted fixture. L1 planner, Rust rendering, and workflow-source assertions cannot establish actual Actions grouping, reused-result visibility, skipped labels, cancellation effects, or required-check behavior. This leaves AC2/4/8/9/10/11 and the final hosted result chain in AC12 at the wrong verification boundary.

Implement publication after the display decision and execute the controlled branch/scratch-repository fixtures when authorized. Record actual labels, checks, run conclusions, and merge decisions for mixed reused/executing cells, an accepted gap, a failed selected area, and an unselected area. Turn the pending contracts into ordinary assertions. This is missing implementation and integration verification, not missing cross-OS evidence. Carried forward from review 4.

### High — Constraints still disappear in an unconfigured fresh session

`scripts/ci/constraints.py:53–66` still returns an empty default directory; `load` at `:169` returns no records when no directory is selected. A prohibition can block a configured session and disappear from consideration when the environment variable is absent. The new target-aware hook logic does not change store discovery.

Complete the selected persistence/discovery mechanism and verify that a record written in one session is enforced by both the plan preview and the hook in a fresh environment without `BISCUIT_CI_CONSTRAINTS_DIR`. Preserve expiry, branch/repository matching, and fail-closed malformed-record handling. This remains a §4/AC17 implementation gap. OQ2 explains the deferral; the outstanding human choice itself is not the readiness finding. Carried forward from review 4.

## Prior-review disposition

| Review-4 finding | Assessment |
|---|---|
| Wrong base for stacked PRs | Fixed for ordinary single-head pushes, an advanced remote target, and multiple open PR targets. The new simultaneous-update defect is distinct. |
| Standalone global verdict | Deferred; remains a finding. |
| Gap publication and hosted verification | Deferred; remains a finding. |
| Fresh-session constraint discovery | Deferred; remains a finding. |

The restored Bash 3.2 empty-array handling also passes its regression. The latest implementation log accurately reports one fix and three deferrals; its completion heading should not be read as completion of the specification. No additional performance or API-ergonomics finding was established.

## Requirement-to-verification map

L1 includes in-process and hermetic subprocess/Git/filesystem tests. Hosted integration is the appropriate boundary for GitHub presentation and merge protection. Terminal IPC L2 and OS-keyboard L3 do not prove those behaviors and are not required here. The specification prescribes plan data and exit status, not terminal styling or keyboard behavior.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1: impacted areas; no unchanged dependent jobs | L1 planner fixtures | Appropriate selection coverage; OQ1 remains a design decision. |
| AC2: one visible identity per area, including nested areas | L1 mapping/workflow contracts | Hosted presentation missing; finding above. |
| AC3: relevant compile targets and environments after local reuse | L1 planner/command contracts | Appropriate logic coverage; cross-OS execution evidence is CI's responsibility. |
| AC4: completed reused results, measurements, provenance, no rerun | L1 evidence/scheduling/rollup fixtures | Logic covered; hosted visibility unverified. |
| AC5: combined macOS/WSL and cross-check receipts | L1 real-Git/subprocess fixtures | Appropriate boundary. |
| AC6: seven reused macOS cells never become MISSING | L1 Rust regression | Appropriate; passed. |
| AC7: evidence rejection and prohibited-trigger blocking | L1 evidence/constraint/hook fixtures | Simultaneous target/head updates bypass the restriction. |
| AC8: failures block their owning area; independent results survive | L1 rollup/verdict fixtures | Policy covered; hosted propagation unverified. |
| AC9: immediate explained gaps; expiry and cancellation semantics | L1 policy/rollup; pending publisher contract | Missing publisher and hosted integration. |
| AC10: resolved labels and environment-qualified lint | L1 workflow-source contracts | Actual hosted labels unverified. |
| AC11: area-owned merge authority | L1 area narrowing; pending migration contracts | Global policy job and scratch-fixture gap remain. |
| AC12: valid successful-validation reuse and invalid-result rejection | L1 reuse/evidence fixtures | Logic covered; final hosted chain incomplete. |
| AC13: local/CI workers, isolated L2 forwarding, overrides | L1 worker/argument fixtures | Appropriate; passed. |
| AC14: accurate documentation and rationale | Source review and L1 contracts | Deferrals documented; hook's “every run” claim needs the simultaneous-update qualification/fix. |
| AC15: area mapping agrees with sniff | Conditional subprocess drift test | Appropriate; comparison remains conditional on sniff availability. |
| AC16: v1 exact identity and unrecorded measurements | L1 real-Git/Rust fixtures | Appropriate; passed. |
| AC17: persisted restrictions and reviewable plans | L1 checker/renderer/hook fixtures | Fresh-session discovery and simultaneous-update handling incomplete. |

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 400 passed in 69.785 seconds.
- `env -u CDPATH GIT_TERMINAL_PROMPT=0 /bin/bash ./.githooks/tests/test-pre-push.sh </dev/null`: 53 passed, 0 failed.
- `just test test-toolkit`: 155 passed, 2 skipped. Pending contracts within the passing total do not prove implementation.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: 183 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan`: 9 passed.
- `shellcheck .githooks/pre-push`: passed.
- Temporary simultaneous-update hook regression: failed as expected against the current implementation, exit 0 instead of required exit 1.

An initial combined Nextest command disabled default features for both binaries and was rejected because `ci-plan` requires `local-tools`; the corrected separate commands above passed. No test failed in that initial command because none ran.

No full-workspace suite, WSL execution, terminal window, hosted workflow trigger, repository commit, hosted push, or ruleset change was performed. The hook fixtures create commits and publish notes only in disposable local repositories.

GitNexus was queried for CI flow and document impact. Its index binds this worktree to HEAD `a69c02c5c`; it does not establish the uncommitted implementation as indexed. Document impacts were UNKNOWN with no resolved callers/processes, so text inspection confirmed the review-chain and plan references before metadata edits. No code symbols were edited.

The requested previous-review path under `prompts/_reviews/fixes/` returns no match through `bf reference` (the biscuit-file FileReference CLI). The existing review resolves at `fixes/2026-09-11-cicd-cleanup/review-4.md`; its `next` now points here and `implemented: true` is preserved. The specification records `review_iterations: 5`.

## Rulings and pointers (2026-09-12)

Ken ruled every human-review item in this review, plus the blockers and the
absorption decisions that surfaced while ruling, on 2026-09-12. The
authoritative text is the `## Rulings` section of `spec.md`; the entries
below only point there and to the records each ruling produced. Nothing in
the findings above is amended by this section.

### Human-review items

| Item | Ruling | Where |
|---|---|---|
| 1. How pull requests become eligible to merge | Option C (ruleset "Require workflows to pass") with Option B as fallback. The scratch experiment showed the rule is unavailable on a user-owned repository, so **Option B is the mechanism**: a fixed-name, policy-free `ci-gate` fold job as the single required check. | `spec.md` Rulings → OQ3; `fixtures/scratch-2026-09-12.md` Findings 1 and 2 |
| 2. Where saved instructions live | Option B, a per-repository directory on the host beside the evidence directory, environment variable kept as override, identical behavior on macOS, Linux, native Windows, and WSL2. | `spec.md` Rulings → OQ2 |
| 3. Cancelled or neutral for an accepted gap | **`neutral`**, superseding section 6's cancelled request. The experiment confirmed `neutral` leaves a PR CLEAN and `cancelled` makes it UNSTABLE. | `spec.md` Rulings → OQ4; `fixtures/scratch-2026-09-12.md` Finding 3 |
| 4. Compile check for unchanged dependents | Option B, an in-job step of the changed package's check cell on Linux, reported as "also compiled N dependents". | `spec.md` Rulings → OQ1 |

### Findings

| Finding | Status after the rulings | Where |
|---|---|---|
| Simultaneous PR head and target updates bypass the constraint check | Needs no ruling; implementable now. Still open. | this review, first finding |
| The standalone global policy verdict remains | Unblocked: replace `ci-verdict` with the `ci-gate` fold and migrate the required-check context per Validation and Rollout step 6. The live ruleset edit is a separate approval after the implementation is ready. | `spec.md` Rulings → OQ3, B5 |
| Accepted-gap publication and hosted verification incomplete | Unblocked: publisher emits `neutral` from a job holding `checks: write`. The throwaway-branch fixture now owes only the nested-area display and mixed-cell labels, and runs once the publisher and area workflow exist. | `spec.md` Rulings → OQ4, B5; `fixtures/scratch-2026-09-12.md` |
| Constraints disappear in an unconfigured fresh session | Unblocked: one line in `constraints.default_directory()` plus the fresh-session test on every OS. | `spec.md` Rulings → OQ2 |

### Blockers and absorption decisions ruled in the same session

| Decision | Ruling | Where |
|---|---|---|
| B0 prerequisite | The 2026-09-10 specification is **absorbed**; its requirements are this fix's own scope. A formal pass found the absorption substantive and listed fourteen work items (W1–W14) and five missing tests (T1–T5) now owned here. | `spec.md` Rulings → B0; `absorption-audit-2026-09-12.md` (supersedes `prerequisite-audit.md`) |
| D1 strict-failure publication | A complete failing `strict` run **publishes** its validation note before blocking (09-10 R4/AC7 stand); the source-grep pin becomes a behavioral test. | `spec.md` Rulings → D1 |
| D2 cross-note conflicts | **Newest wins** in both directions. New commits compute their blast radius from prior commits' evidence, so W14 (the hook applies accepted evidence to its own local gates) is required behavior. | `spec.md` Rulings → D2 |
| B5 GitHub fixtures | Both authorized. Scratch half complete, recorded, and the scratch repository deleted on 2026-09-12. Throwaway-branch half deferred to implementation. | `spec.md` Rulings → B5; `fixtures/scratch-2026-09-12.md` |
| Governing principle | Test only what needs testing; CI turnaround is measured in hours and every fixture, cell, and gate must justify its runtime. | `spec.md` Rulings → Governing principle |

---
$schema: feature-review.yaml
ready: false
findings:
  - title: Trigger review does not cover every pushed ref or its branch constraints
    priority: high
  - title: The standalone global policy verdict remains
    priority: high
  - title: Accepted-gap publication and hosted presentation verification remain incomplete
    priority: high
  - title: Constraints still disappear in an unconfigured fresh session
    priority: high
  - title: The revised hook test suite aborts under the default macOS Bash
    priority: medium
human_review: true
human_review_items:
  - |-
    Choose how a pull request becomes eligible to merge: require the CI workflow to pass, if a small experiment in a disposable repository proves that option works, or require one fixed check that only combines the individual area results. Approve changing branch protection together with that choice.
  - |-
    Choose where saved instructions such as “do not run WSL again” should live: a persistent directory on the computer, a shell setting, or a Git note. The specification recommends a persistent directory so a fresh terminal session still finds the instruction.
  - |-
    After the small GitHub example is available, compare an accepted coverage gap displayed as cancelled versus neutral and choose its appearance. Cancelled matches the requested display; neutral avoids showing an accepted gap as an unsuccessful check.
  - |-
    Decide whether unchanged applications using a modified library should receive a compile check: remove the check, run it inside the changed area's job, or use a separate infrastructure job. The specification recommends keeping it inside the changed area's job.
reviewed_by: codex/gpt-6-astra
created: "2026-09-11T21:38:11-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: true
implemented_by: claude/fable
log: fixes/2026-09-11-cicd-cleanup/log.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-3.md
previous: 2026-09-11-cicd-cleanup/review-2.md
next: 2026-09-11-cicd-cleanup/review-4.md
---

# Review 3

**Not production-ready.** Four prior findings have targeted fixes, but the implementation log explicitly defers three required behaviors. This review also found a trigger-constraint bypass and a regression in the hook test harness. Human decisions and missing cross-OS execution evidence are not independently counted against readiness; incomplete implementation and verification at the wrong boundary are.

Reviewed the current working tree, including the uncommitted implementation changes, against the specification and review 2. Changes made by this review are limited to review artifacts and metadata.

## Findings

### High — Trigger review does not cover every pushed ref or its branch constraints

`.githooks/pre-push:93–110` reduces all stdin ref updates to one `TRIGGER_SHA`: HEAD if any update carries it, otherwise the last non-deleted SHA. Only that revision is planned. At `:287–290`, constraint checking then uses `git rev-parse --abbrev-ref HEAD` and origin's URL, rather than the branch and remote of the update being reviewed. The new support for pushing a revision other than HEAD therefore materializes the correct tree but evaluates it against the wrong branch's restrictions.

A temporary Git fixture using the real hook, real constraint checker, and existing manufactured WSL-executing planner reproduced both cases. With `feature` checked out, a separate committed `other` branch changed `pkg/alpha/src/lib.rs`. A saved WSL prohibition scoped to `other` was ignored when the hook received the update for `other`: exit 0 despite its plan executing WSL. Supplying both the `feature` and `other` updates also exited 0; the planner log contained selection only for `feature`, omitting the second branch's source change entirely. These probes used local fixture repositories and made no hosted push.

This violates §4 and AC7/17 even when the constraint directory is configured correctly. Review every workflow-triggering ref update with its own base, revision, branch identity, and actual remote before publishing notes or running gates. Define how branch constraints apply to renamed refspecs; do not silently substitute the current checkout. Add real-hook L1 fixtures for a non-HEAD branch prohibition, two updated branches with different scopes, ref ordering, and a remote other than origin. The existing single-ref dirty-tree tests do not cover these cases.

### High — The standalone global policy verdict remains

`.github/workflows/ci.yml:501` still defines `ci-verdict`, and its later steps still perform the global rollup/verdict in addition to the per-area decisions in `_area-ci.yml`. This remains an implementation gap against §5 and AC11, as it was in reviews 1 and 2. The removal and consumer-rewiring tests in `tools/test-toolkit/tests/ci_workflow_contracts.rs` still use pending-contract wrappers; their passing results establish that the pending state is unchanged.

Complete the chosen merge mechanism, consumer migration, and scratch-repository verification together with the required-check transition. Keep the current gate until that coordinated transition is ready. The absence of a human ruling explains the deferral; it does not make the specified functionality complete.

### High — Accepted-gap publication and hosted presentation verification remain incomplete

The planner and Rust rollup represent `ACCEPTED GAP`, but no immediate cancelled/neutral check publisher exists. `_area-ci.yml:95–103` waits for `package-ci` before its rollup and grants only `checks: read`. The publication contract at `ci_workflow_contracts.rs:2680` remains pending. A summary produced after the test jobs finish does not fulfill §6 and AC9's immediate visible gap result.

`rollout-2026-09-11.md` remains `status: not-triggered`; this cycle's log explicitly defers the hosted fixture. Available verification is L1 planner, rollup, extracted-shell, and workflow-source testing. It does not verify actual GitHub grouping, local-result presentation, skipped-label rendering, cancellation effects, or selected/deselected merge protection. AC2/4/8/9/10/11 and the changed hosted result chain in AC12 need the specified hosted integration experiment. This is a verification-boundary mismatch, not missing cross-OS proof. Terminal IPC L2 or keyboard-injection L3 would not prove these GitHub behaviors.

After the display choice, implement the publisher and exercise the controlled branch/scratch-repository fixtures. Include mixed reused/executed results, an accepted gap, a failed selected area, and an unselected area; record the actual checks, run conclusions, merge decisions, and displayed labels.

### High — Constraints still disappear in an unconfigured fresh session

`scripts/ci/constraints.py:48–62` still returns an empty default directory. A later session without `BISCUIT_CI_CONSTRAINTS_DIR` therefore finds no saved restrictions. The committed-tree hook changes fix a different boundary and do not resolve §4's persistence requirement or AC17.

Complete the selected persistent discovery mechanism and verify recording a restriction in one session, then planning and invoking the hook in another with the variable absent. Retain expiry, repository/branch filtering, and fail-closed handling of malformed records. This is the third explicitly deferred finding from review 2.

### Medium — The revised hook test suite aborts under the default macOS Bash

The documented direct command, `env -u CDPATH ./.githooks/tests/test-pre-push.sh </dev/null`, exited 1 before running a test with `line 224: env_args[@]: unbound variable`. The new `run_hook` initializes an empty array and expands it under `set -u`; the Bash selected by the script's `#!/usr/bin/env bash` on this host rejects that expansion.

Invoking the same suite with `/opt/homebrew/bin/bash` passed all 37 tests. This is a harness prerequisite/portability defect, not a failure of the production hook under Ubuntu. Preserve compatibility with the default shell or explicitly resolve and require a suitable Bash before entering the suite, with a clear diagnostic and documented command. Add a small regression for the no-extra-environment-arguments path. The Python workflow-step Bash resolver fixed in this cycle does not cover this separate shell suite.

## Prior-review disposition

| Review-2 finding | Current assessment |
|---|---|
| Committed work hidden by an unstaged revert | Addressed for a single pushed revision: committed path selection and temporary worktree planning are exercised by the hook suite. The broader ref/branch defect above remains. |
| Newer narrow receipts hide older cells | Addressed: candidate notes are walked per environment, with each cell resolved independently. Real-Git fixtures cover narrow receipts, changed inputs, malformed newer notes, and newer complete failures overriding older passes. |
| Scope evidence recomputed on a validation hit | Addressed: the version-2 plan carries projection inputs, and `--apply-to` overlays evidence without selecting scope again. Extracted workflow tests assert one overlay call on a hit, invariant selection fields, and projection consistency. |
| Standalone verdict | Deferred; still a finding. |
| Gap publisher and hosted verification | Deferred; still a finding. |
| Fresh-session constraint discovery | Deferred; still a finding. |
| Workflow-step Bash prerequisite | Addressed by compatible-Bash resolution and prerequisite fixtures. A separate new hook-harness failure is reported above. |

The OS skill's obsolete claim that scope-only publishes no scope receipt has been corrected. The evidence walk also avoids one Git subprocess per missing note by listing each notes ref once; this is a useful bounded efficiency improvement. No additional performance issue was established in this review.

## Requirement-to-verification map

L1 includes in-process and manufactured subprocess/Git/filesystem fixtures. Hosted integration is the appropriate boundary for GitHub UI and merge protection. No requirement here calls for terminal-emulator input encoding, so terminal L2 and OS-keyboard L3 are not applicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1: impacted areas; no unchanged dependent entries | L1 planner fixtures | Selection covered; OQ1 remains a design choice. |
| AC2: one identity per area, including nested areas | L1 mapping and workflow contracts | Hosted presentation missing. |
| AC3: relevant native target coverage after macOS reuse | L1 planner and command contracts | Appropriate logic coverage; cross-OS results remain CI's responsibility. |
| AC4: visible reused results, measurements and provenance, no rerun | L1 receipts, rollup, hook retention and scheduling fixtures | Hosted visibility missing. |
| AC5: mixed macOS/WSL evidence and cross-check receipts | L1 real-Git and subprocess fixtures | Appropriate evidence-boundary coverage; narrow-history defect fixed. |
| AC6: seven reused macOS cells do not become MISSING | L1 Rust regression | Appropriate, passed. |
| AC7: rejection reasons and prohibited-trigger blocking | L1 evidence, constraint and hook fixtures | Ref/branch bypass remains. |
| AC8: failures block only their owning area | L1 rollup/verdict fixtures | Policy tested; hosted propagation unverified. |
| AC9: immediate explained gaps; expiry/revocation and cancellation | L1 policy/rollup; pending publication contract | Publisher and hosted integration missing. |
| AC10: resolved labels and environment-qualified lint | L1 workflow/label contracts | Actual hosted rendering unverified. |
| AC11: area-owned merge authority | L1 area narrowing; pending migration contracts | Global policy job and scratch-fixture gap remain. |
| AC12: successful validation reuse with failure/cancellation rejection | L1 reuse-validation and evidence contracts | Logic covered; final hosted result chain still incomplete. |
| AC13: worker boundaries, L2 forwarding, overrides | L1 calculation and subprocess argument fixtures | Appropriate, passed. |
| AC14: documentation and skill accuracy | Source review | Main limitations documented; old rollout/blocker records should be reconciled at closure. |
| AC15: area mapping agrees with sniff | Conditional local subprocess drift comparison | Appropriate boundary; part of the passing Python suite. |
| AC16: v1 exact-tree and unrecorded measurements | L1 real-Git and Rust rendering fixtures | Appropriate, passed. |
| AC17: persisted constraints and plan output | L1 checker, renderer and hook fixtures | Fresh-session discovery and ref/branch handling incomplete. |
| Inherited authoritative scope handoff | L1 real-note, pure-overlay and extracted-shell fixtures | Prior recomputation defect addressed. |

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 398 passed in 71.602 seconds.
- Direct hook suite: aborted before any test under the default Bash; explicit modern Bash: 37 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: 183 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan`: 9 passed.
- `just test test-toolkit`: 155 passed, 2 skipped. This includes pending contracts, which are not proof of completed behavior.
- `actionlint` on the four orchestration workflows: exit 1, six SC2086 informational diagnostics in native-prerequisite commands. These same diagnostics were recorded in review 2 and are not a new readiness finding.
- Temporary real-hook probes reproduced the other-branch constraint bypass and single-revision handling of a two-ref push. Planner output was manufactured; constraint evaluation and Git ref/tree handling used the real implementation.

No full-workspace tests, remote OS runs, terminal windows, hosted dispatches, repository commits, hosted pushes, or ruleset writes were performed.

GitNexus was queried for CI flows and upstream impact of the existing review/spec files before editing. Its index is 75 commits behind; file impacts were `UNKNOWN`, with no resolved callers or processes. Text inspection confirmed the document references rather than treating that result as an all-clear. No code symbols were edited.

The requested previous-review reference under `prompts/_reviews/fixes/` does not resolve through `bf reference`. The existing previous review resolves at `fixes/2026-09-11-cicd-cleanup/review-2.md`; that file now has the requested `next` link and retains `implemented: true`. The specification records `review_iterations: 3`.

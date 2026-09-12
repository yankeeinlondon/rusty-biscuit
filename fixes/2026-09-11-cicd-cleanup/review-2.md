---
$schema: feature-review.yaml
ready: false
findings:
  - title: The trigger constraint check can overlook committed work
    priority: high
  - title: Newer partial package receipts hide valid older evidence
    priority: high
  - title: Scope evidence is still recomputed when validation is reused
    priority: high
  - title: The standalone global policy verdict remains
    priority: high
  - title: Accepted-gap publication and hosted presentation verification remain incomplete
    priority: high
  - title: Constraints still disappear in an unconfigured fresh session
    priority: high
  - title: Workflow shell tests do not declare their Bash requirement
    priority: medium
human_review: true
human_review_items:
  - |-
    Choose how merging a pull request should depend on its selected areas passing: require the CI workflow to pass, if the small scratch-repository experiment confirms support, or require one fixed check that only combines the area results. Approve the branch-protection transition together with that choice.
  - |-
    Choose where saved instructions such as “do not run WSL again” should live: a persistent host directory, a shell setting, or a Git note. The specification recommends a persistent host directory so a new terminal session still finds the instruction.
  - |-
    Review a small GitHub example of an accepted coverage gap displayed as cancelled versus neutral, then choose the desired appearance. Cancelled matches the requested display; neutral avoids presenting an accepted gap as an unsuccessful check.
  - |-
    Decide whether unchanged applications that use a modified library should still receive a compile check: remove the check, run it inside the changed area's job, or use a separate infrastructure job. The specification recommends keeping it inside the changed area's job.
reviewed_by: codex/gpt-6-astra
created: "2026-09-11T19:54:10-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-2.md
previous: 2026-09-11-cicd-cleanup/review-1.md
log: fixes/2026-09-11-cicd-cleanup/log.md
implemented: true
next: 2026-09-11-cicd-cleanup/review-3.md
implemented_by: claude/fable
---

# Review 2

**Not production-ready.** The revised working tree fixes several concrete first-review defects, but three findings were explicitly deferred and additional execution/evidence defects remain. Human decisions and missing cross-OS run evidence are not themselves readiness failures; incomplete required behavior and the defects below are.

Reviewed the specification, first review, implementation log, workflows, Python evidence/planner/constraint code, shell hook, Rust rollup, and regression contracts. Existing working-tree changes, including the untracked area workflow, are included in this assessment. No implementation code was changed.

## Findings

### High — The trigger constraint check can overlook committed work

`.githooks/pre-push:184` reviews `just ci-local --plan`, but `just/ci-local.just:138` determines changes using `git diff --name-only <merge-base>` against the **working tree**, followed by untracked files. It does not compare the outgoing committed head. When that diff is empty, the recipe returns success without writing a plan; the hook explicitly skips constraint evaluation for an empty plan file.

A temporary Git fixture reproduced the mismatch: commit a change to `pkg/alpha/src/lib.rs`, then restore its base contents as an unstaged edit. The local recipe's tracked diff is empty, while `git diff --name-only <base> <head>` still names that file. The push sends the committed change, so CI can schedule the prohibited environment even though the hook never evaluated those cells. Untracked files are unnecessary for this scenario. This violates §4 and AC7/17.

The subsequent `publish_scope_evidence` uses committed filenames, but happens **after** the constraint decision. Its planner also reads Cargo metadata and policy from the current checkout (`affected_scope.py:195,2188`), even though the hook permits dirty-tree scope publication. Consequently, an unstaged manifest/policy change can be recorded as scope for the committed tree. The new dirty-tree hook test uses a planner stub and cannot establish that committed policy was used.

Resolve the trigger plan from the outgoing revision and the actual event base, including committed manifests and policy; apply evidence and constraints to that same plan. Keep working-tree planning for local development separate. Add a real-hook fixture with a committed source change masked by an unstaged revert, plus a dirty-policy fixture proving that scope receipts describe committed policy. Both must preserve the required cells and block prohibited executions.

### High — Newer partial package receipts hide valid older evidence

`local_evidence.py:184–190,246–250` calls `latest_note()` once per environment, then processes only that note. It does not combine non-overlapping cells from older notes on the same environment ref. Here “partial package” means a complete run covering fewer packages, not an interrupted receipt.

A probe using the existing temporary Git fixture recorded a valid WSL `alpha/L1` receipt, committed only unrelated documentation, then recorded a valid WSL `beta/L1` receipt. Both packages' gate inputs remained unchanged. `verify_cells()` returned only `beta/wsl2-ubuntu/L1`, with no rejection explaining the lost alpha evidence. This affects successive cross-check runs and narrower subsequent validations: valid prior work is scheduled again, or an otherwise satisfiable execution constraint blocks the push. It contradicts the per-cell resolution contract in §3 and AC5/7.

Walk candidate notes per environment and resolve the newest qualifying evidence **per cell**, preserving complete failures and defining conflict precedence. Add fixtures for non-overlapping receipts across commits, changed inputs, malformed newer notes, and a newer failure for a previously passing cell. Cross-environment combination tests alone do not cover this case.

### High — Scope evidence is still recomputed when validation is reused

The new scope receipt handoff works on the no-validation-evidence path. However, `.github/workflows/ci.yml:168–191` invokes the full planner again whenever any accepted validation cells exist. On a scope hit it compares only area and package names, then replaces the receipt's plan and scope with the recomputed output. Gate selection, targets, features, environment policy, and other execution details are not included in that comparison.

The implementation log expressly calls this “compared, not handed off.” That is a partial implementation of the first review's scope finding, not fulfillment of the inherited authoritative-plan contract. Sections 1.1 and 3.1 require one resolved plan; the specification permits recalculation on a scope miss, not replacement of a matching receipt when reuse is present. The new test `test_accepted_cells_on_a_scope_hit_are_resolved_by_a_compared_planner_run` pins the workaround rather than the requested behavior.

Apply verified evidence to the carried plan without recalculating selection or policy, and derive the scheduling projection from that result. If fields needed for the projection are missing, complete the plan schema. Verify that a matching scope plus validation receipt never invokes scope selection, and that every non-evidence execution attribute survives unchanged.

### High — The standalone global policy verdict remains

`.github/workflows/ci.yml` still defines `ci-verdict` and runs global rollup/verdict policy in addition to `_area-ci.yml`'s area rollups. AC11 and §5 therefore remain unimplemented. The workflow suite still passes pending-contract wrappers for removing the job and rewiring its consumers; those passes explicitly represent unfinished work.

Complete the selected gate mechanism, scratch-repository verification, consumer changes, and coordinated required-check transition. Retaining the current required check during migration is prudent, but is not completion. Do not delete it independently of that transition. This is the same unresolved production requirement as review 1.

### High — Accepted-gap publication and hosted presentation verification remain incomplete

The distinct `ACCEPTED GAP` machine state and explanatory rollup details exist. There is still no immediate cancelled/neutral check publisher; `_area-ci.yml` has read-only check permissions and publishes its grid after package producers finish. The pending publication contract remains. This leaves §6 and AC9 incomplete regardless of the eventual display choice.

The rollout record remains `status: not-triggered`; the implementation log also defers the hosted fixture. Inspected verification consists of L1 planner, shell, Rust, and workflow-source contracts. It does not exercise GitHub's actual area grouping, reused-cell display, unresolved skipped labels, cancellation effects, or selected/deselected merge-gate behavior. These user-observable requirements in AC2/4/9/10/11 need the specified hosted integration fixture. This is a verification-level mismatch, not a request for cross-OS test proof. Terminal L2/L3 cannot verify a GitHub UI or ruleset.

Implement the publisher after the display decision and exercise the controlled hosted fixture, including mixed local/CI results, an accepted gap, a failed selected area, and a deselected area. Record actual check/run conclusions and presentation evidence.

### High — Constraints still disappear in an unconfigured fresh session

`constraints.py:48–63` still returns an empty default directory. The hook now always calls the checker, fixing its earlier unconditional refusal of satisfied constraints, but a new session without `BISCUIT_CI_CONSTRAINTS_DIR` still discovers no saved restrictions. Section 4's persistence requirement and AC17 remain incomplete.

Complete the chosen persistent discovery mechanism and exercise recording in one session followed by planning/pushing in another with the variable absent. Retain repository/branch filtering, expiry handling, and fail-closed malformed-record behavior. The log correctly calls this deferred; it is not resolved by changing the hook's call site.

### Medium — Workflow shell tests do not declare their Bash requirement

The new `WorkflowScopeStepTests` checks only whether `bash` and `jq` exist (`test_ci_local.py:639`) and invokes `bash` through PATH (`:727`). The extracted workflow uses `mapfile`, which the Bash selected on this review host lacks. The Python run produced 11 failing assertions, all with `bash: line 22: mapfile: command not found` or the resulting missing expected diagnostic.

This is a test prerequisite/portability defect, not evidence that the Ubuntu-hosted workflow has the same problem. Make the harness explicitly resolve a compatible Bash and report a clear prerequisite failure or justified skip when unavailable. Keep the hosted infrastructure gate executing these contracts under its supported shell. Add the prerequisite behavior to the fixture rather than relying on an individual's PATH ordering.

## First-review disposition

The revised code and inspected tests address wrong-environment/revision attribution, manual-dispatch variable initialization, supplied-plan constraint evaluation, explicit native example/bench checks, durable hook report retention, and CI invocation of workflow contracts. The hook suite passed its new publication and constraint fixtures; Nextest executed the workflow contracts successfully. Dispatch shell verification remains limited by the test prerequisite issue above on this host.

Authoritative scope evidence is partially implemented, with new defects described above. Global verdict removal, accepted-gap publication/hosted verification, and fresh-session constraint discovery remain deferred. The log's “7 fixed, 3 deferred” statement therefore overstates scope-handoff completion.

## Requirement-to-verification map

L1 here means in-process logic or manufactured subprocess fixtures. GitHub presentation and merge protection require hosted integration evidence. None of this specification's requirements concerns terminal input encoding, so terminal IPC L2 and OS-keyboard L3 are not applicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1: impacted areas; no unchanged reverse-dependent entries | L1 planner fixtures | Selection logic covered; OQ1 seam decision remains separate. |
| AC2: area grouping and distinct nested areas | L1 mapping/workflow contracts | Hosted presentation gap. |
| AC3: target-specific native coverage with local macOS reuse | L1 planner and executable-command contracts | Prior Windows-only/blanket-check defect addressed. |
| AC4: visible local outcomes, measurements, provenance; no rerun | L1 receipt/rollup and real-hook retention fixtures | Retention addressed; hosted visibility unverified. |
| AC5: combined macOS/WSL and cross-check evidence | L1 real-Git receipt fixtures | Cross-environment combination covered; same-environment history defect reproduced. |
| AC6: seven macOS cells never become MISSING | L1 Rust regression | Appropriate policy-level test, passed. |
| AC7: evidence rejection and prohibited-trigger blocking | L1 receipt/constraint/hook fixtures | Dirty-working-tree trigger mismatch and receipt-history gap. |
| AC8: failed local/CI cells block their owning area | L1 rollup/verdict fixtures | Logic covered; hosted propagation unverified. |
| AC9: immediate accepted gaps; expiry/revocation semantics | L1 policy/rollup; pending publisher contract | Publisher and hosted integration missing. |
| AC10: resolved labels and environment-qualified lint | L1 workflow/label contracts | Hosted rendering gap. |
| AC11: area-owned merge authority | L1 narrowing; pending removal contracts | Required migration and scratch fixture absent. |
| AC12: successful validation reuse; failures not reused as success | L1 reuse-validation/receipt contracts | Logic tested; changed hosted result chain unverified. |
| AC13: worker budgets and forwarding | L1 calculation/argument fixtures | Appropriate level. |
| AC14: accurate documentation and skills | Source/document review | Limitations documented; OS skill still incorrectly says scope-only publishes no scope receipt. |
| AC15: mapping agrees with sniff | Conditional local subprocess drift test | Appropriate mapping comparison; not a hosted UI test. |
| AC16: v1 exact-tree, unmeasured migration | L1 legacy-receipt fixtures | Appropriate level. |
| AC17: persisted constraints and plan display | L1 checker/hook/renderer contracts | Fresh-session discovery missing; trigger can review the wrong changes. |
| Inherited authoritative scope handoff | L1 real-note and extracted-shell fixtures | Partial implementation; evidence path still recomputes. |

The AC14 drift is in `.claude/skills/os/SKILL.md`'s scope-only paragraph. The hook now publishes a scope receipt; update that paragraph to match the code. This documentation observation does not independently determine readiness.

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 378 tests executed; failed with 11 assertion failures in workflow shell fixtures due to unavailable `mapfile` in the selected Bash.
- `env -u CDPATH ./.githooks/tests/test-pre-push.sh </dev/null`: 33 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: 183 passed.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 84 passed, including explicit pending contracts; passing those does not mean the pending features exist.
- `actionlint` over the four orchestration workflows: exit 1 with six SC2086 informational diagnostics in native-prerequisite shell commands; no other diagnostics. These are not the basis for the readiness verdict.
- Temporary Git probes reproduced hidden older WSL evidence and the difference between working-tree and outgoing-commit scope. Fixtures used temporary repositories; no repository commits were made.

No package-wide/full-workspace validation, remote OS tests, terminal windows, pushes, hosted dispatches, or ruleset writes were performed.

GitNexus was queried for CI flows and upstream impact of the two existing Markdown files edited for closure. Its index is 65 commits behind; both file impacts were `UNKNOWN`, with no resolved callers or processes. This was not treated as an all-clear: text inspection confirmed the review/spec references, and edits are limited to the requested review artifacts and metadata.

The requested previous-review path under `prompts/_reviews/fixes/` does not resolve. The actual previous review is `fixes/2026-09-11-cicd-cleanup/review-1.md`; that file now links to this review and retains `implemented: true`. The spec now records `review_iterations: 2`.

---
$schema: feature-review.yaml
ready: false
findings:
    - title: Changed origins still show the old head request's warning and fallback notice
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T20:17:36-07:00
spec: 2026-10-03-list-overlap-and-keyless-notice/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-10-03-list-overlap-and-keyless-notice/log.md
description: "A **fix** review of `2026-10-03-list-overlap-and-keyless-notice/spec.md`"
fix: 2026-10-03-list-overlap-and-keyless-notice/review-2.md
previous: 2026-10-03-list-overlap-and-keyless-notice/review-1.md
next: 2026-10-03-list-overlap-and-keyless-notice/review-3.md
---

# Review 2

The fix is **not production ready**. The three findings from review #1 have implementations and appropriate regression coverage, but the required suppression of notices when `origin` changes is incomplete. Two head-request notice paths still describe the previous repository after the listing has detected the change. No human decision is needed to repair this.

## Previous findings

Review #1 has a `Findings` section rather than separate `Unblocked Findings` and `Blocked Findings` sections. All three findings were unblocked; its frontmatter reports no blocked findings. There were therefore no blocked findings to unblock before this implementation.

| Previous finding | Implementation and verification | Result |
| --- | --- | --- |
| Local gathering still waits for the remote worker | Scoped list/history tasks overlap the calling thread's wait. Initial/final snapshots decide reuse; changed or failed reads regather. Dirtiness is retained except for a moved checkout. Persistence is deferred. Real Git pipeline tests verify the resulting caption, target, tree, comparisons, graph, history, status walks, and stored state. | Implemented |
| Successful API credential evidence and the keyless notice are missing | `sniff` records the actual request selection, including pagination and host-bound overrides. Head format 3 and PR format 6 carry validated evidence; legacy answers remain usable with unknown credentials. Wait tests cover adoption, contention, publication before receipt, timeout, and retaining the accepted publication's metadata. Shipped CLI tests and a real tmux capture exercise the notice. | Implemented; the additional changed-origin defect below remains |
| The performance report still adds overlapping work | Measured groups contain diagnostic children without percentages. Reconciliation excludes those children, and excess attribution is reported instead of clipped. Stage readers match whole labels. Collector, pipeline, and shipped CLI tests verify the grouped reports. | Implemented |

## Unblocked Findings

### High — Changed origins still show the old head request's warning and fallback notice

**Defect class:** request-notice projections apply repository-identity validation inconsistently, allowing evidence about a superseded remote to be presented as information about the current repository.

In `worktree-cli`, [follow_remote](../../cli/src/commands/list.rs:162) detects an origin change and removes old PR badges. [observed_pr_failure](../../cli/src/commands/list.rs:244) suppresses the PR warning, and [observed_keyless](../../cli/src/commands/list.rs:259) suppresses anonymous-success notices from both halves. However, [run_pipeline](../../cli/src/commands/list.rs:595), which turns the accepted listing into terminal output, still passes the old followed head attempt to the credential-warning renderer and fallback-notice selector without checking `remote.origin_changed`.

This can tell the user that their API key was rejected, or that Git checked `origin` successfully, even though those requests concerned the repository that `origin` no longer identifies. It violates the specification's instruction to suppress **all request notices** for the old origin, including the new keyless notice.

I reproduced the complete sibling class through the shipped `wt list --perf` binary. A temporary L1 probe copied the existing `MixedFixture`/`FakeGitea` cases in `cli/tests/list_prs.rs`. Each case first ran an unchanged-origin positive control. On its second listing, the stand-in held the PR response until the **new attempt ID**, distinct from the control's, had a completed head outcome. It then changed only `origin` to `http://gitea.example.invalid/o/other.git` before returning the PR response. All provider and Git traffic stayed on the local stand-in; the fallback and head-success cases used the fixture's real bare repository. Worker teardown completed. The probe was removed after execution.

| Site | Shape tested | Observed result after origin change | Expected result |
| --- | --- | --- | --- |
| Head credential warning | Head HTTP 401 with `GITEA_TOKEN`; Git fallback refused; PR response held until the current head attempt finished | `Gitea didn't accept GITEA_TOKEN` still appears. The unchanged-origin control also displays it. | Suppress the old head warning |
| Closing Git fallback notice | Anonymous head HTTP 401 followed by successful `ls-remote`; PR response held until the current head attempt finished | `Git checked origin using ls-remote` still appears. The unchanged-origin control also displays it. | Suppress the old fallback notice |
| Head anonymous-success notice | Anonymous head API success and completed fetch | Suppressed — clean; unchanged-origin control displays the notice | Suppress |
| PR credential warning | PR HTTP 401 with `GITEA_TOKEN`; generic head failure | Suppressed — clean; unchanged-origin control displays the warning | Suppress |
| PR anonymous-success notice | Anonymous empty PR answer; generic head failure | Suppressed — clean; unchanged-origin control displays the notice | Suppress |
| PR badges and status reasons | Existing shipped CLI changed-origin fixtures with stored/new badges and failed refreshes | Suppressed — clean in the passing L1 suite | Suppress |

The head warning selector handles rejected keys, insufficient permission, authenticated/anonymous rate limits, and an invisible repository through the same unguarded head-attempt input. The repair must guard that entire projection, not only the reproduced rejected-key variant. The closing fallback notice is a separate projection and must receive the same identity guard. Keep the already-correct PR and anonymous-success paths suppressed, and preserve warning precedence and fallback coexistence when the origin is unchanged.

Add permanent L1 shipped-binary regressions for both affected paths using the completed-current-attempt checkpoint above. Retain the positive controls so a missing warning or failed fallback cannot make suppression pass accidentally. Sweep both a replaced and a removed origin through the same shared guard. Update the `worktree` list topic page to state that a changed or removed origin suppresses every old request notice; its existing changed-origin paragraph currently names only PR badges and failure reasons.

## Blocked Findings

None.

## Recurrence

No finding repeats the defect class of review #1. Its credential finding concerned successful requests losing authentication evidence before publication. That evidence now survives the complete pipeline. This finding concerns failure-warning and Git-fallback projections retaining evidence after the origin identity check rejects it. Review #1 requested changed-origin coverage for successful notices; those sibling paths are now clean. The two failing projections are enumerated together here.

## Input robustness sweep

The changed persistent input format is JSON in both stores. I inspected both real-writer fixture matrices and ran them in the full L1 suite. The new load-bearing fields are the credential object, its state tag, and the variable list when the state is `keyed`. The matrices assert public cache/publication or answer/attempt selection, with an anonymous-success positive control. Existing envelope, identity, timestamp, receipt, and answer matrices remain in place.

`Drop attempt` below means the independently valid head answer remains usable. `Miss` means the PR publication is rejected. Shapes for variable lists use the `keyed` state; absent variables on `anonymous` or `unknown` are valid because those states have no variable list.

| Shape | PR credential object | PR state | PR keyed variables | Head credential object | Head state | Head keyed variables |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Explicit null | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Wrong whole type | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| One wrong element | Not an array | Not an array | Miss | Not an array | Not an array | Drop attempt |
| Every element wrong | Not an array | Not an array | Miss | Not an array | Not an array | Drop attempt |
| Empty object/list/string | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Duplicate object key | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Trailing/invalid document content | Miss | Miss | Miss | Whole file miss | Whole file miss | Whole file miss |
| Valid anonymous control | Usable, anonymous | Usable, anonymous | Not applicable | Answer and attempt usable, anonymous | Anonymous | Not applicable |
| Valid unknown/keyed state | Usable; never anonymous | Usable | Valid nonempty variable names retained | Answer and attempt usable; never anonymous | Usable | Valid nonempty variable names retained |

Strict parsing rejects repeated keys before serde's tagged-enum buffering can erase them. Invalid names and token-like mixed-case strings are rejected. There is no new permissive default, element filtering, or absent/null coercion in credential parsing. Head formats 1/2 and PR format 5 migration controls pass; future formats are misses. No input-validation finding emerged from this sweep.

## Requirement and verification coverage

| Requirement | Appropriate level | Strongest verification present | Result |
| --- | --- | --- | --- |
| Local gathers overlap the held worker outcome; no-origin/non-image controls remain | L1 synchronization and real Git fixtures | Pipeline rendezvous and control-path tests | Pass |
| Complete snapshot equality; advancement, rewind, additions/deletions, failed reads | L1 public listing results | Snapshot matrix and final-state pipeline comparisons | Pass |
| Graph/history queries and branch decorations use accepted tips; tags/HEAD retained | L1 real Git fixtures | Moved-ref gather and verbose-label tests | Pass |
| Dirty status is reused; only a moved checkout is refreshed | L1 operation counts and public results | Fast-forward outcome matrix and final-state comparisons | Pass |
| Speculation persists nothing; accepted results save/prune once and retain successful cache entries | L1 stored state and counts | Held-gather persistence fixture, shared-cache tests, fork-record guards | Pass |
| Wait budgets, detached-worker continuation, receipt/adoption rules remain | L1 bounded seams and shipped CLI | Wait state-machine tests and held-request fixtures | Pass |
| Credential evidence comes from actual request selection, across pages and environments | L1 public API and request assertions | `sniff` branch-head/PR fixtures plus wait adoption/publication fixtures | Pass |
| Notice precedence, cached/ignored/unknown suppression, token exclusion, fallback coexistence | L1 shipped CLI and selector matrix | Keyless CLI cases, precedence controls, stores and migration matrices | Pass except changed-origin paths above |
| Old-origin notices disappear | L1 shipped CLI | Five-path review probe plus existing badge/status cases | **Two failing projections** |
| Keyless line is dim, beneath caption, after spinner cleanup | L2 real terminal | Passing tmux pane and styling capture | Pass |
| Concurrent performance spans reconcile and nested stage readers select intended rows | L1 collector and shipped CLI | Group/reconciliation/parser tests and serial performance suite | Pass |

No keyboard behavior is introduced; L3 is unnecessary. L2 targets are declared, `terminal-tests` is enabled by the recipe and CI metadata, and `just check-tier-coverage worktree` reports no stranded tests. The required terminal capture actually ran; readiness is not inferred from style-only unit tests.

## Validation

- Worktree `just test`: **954 passed**.
- Worktree `just test-l2`: **32 passed**, including the real-terminal keyless notice capture.
- Sniff `just test`: **3,127 passed**.
- Both areas' `just lint`: passed.
- `just check-tier-coverage worktree`: passed, no stranded tests.
- Temporary shipped-CLI notice sweep: one probe passed while recording the defective behavior at both affected paths and the clean controls; removed afterward.
- Worktree `just test-perf`: **32 passed** on the serial rerun without competing test suites. The first run, while other suites were active, failed a full-command timing bound (10.15 s versus a 3.00 s remote-wait span). The clean rerun passed that test and the full suite; the first result is recorded as contention during review, not a separate implementation finding.

Reviewed the current source and existing implementation changes without modifying product behavior. Documentation and skill pages describe the implemented overlap, store versions, notice, and grouped timings; the remaining changed-origin documentation correction is included in the finding. Cross-platform execution evidence is left to CI and does not affect this readiness decision.

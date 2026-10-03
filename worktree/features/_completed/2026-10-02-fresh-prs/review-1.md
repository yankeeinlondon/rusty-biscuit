---
$schema: feature-review.yaml
ready: false
findings:
    - title: Retries discard results already established by the other half
      priority: high
    - title: Retry paths bypass the original deadline and hide timeouts
      priority: medium
    - title: Several worker fixtures do not guarantee cleanup after an assertion fails
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-02T20:52:06-07:00
spec: 2026-10-02-fresh-prs/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/features/2026-10-02-fresh-prs/log.md
description: "A **feature** review of `2026-10-02-fresh-prs/spec.md`"
feature: 2026-10-02-fresh-prs/review-1.md
next: 2026-10-02-fresh-prs/review-2.md
---

# Review 1: Fresh pull requests

**Not production ready.** The ordinary listing, every-run PR request, stored-answer handling, and terminal presentation work in the tested paths. The remaining defects concern retries and test cleanup. No human decision is needed to fix them.

Reviewed the specification, plan, implementation log, implementation commits, library readers and writer, worker, wait, rendering, changed tests, and current documentation. Temporary reproduction tests were removed after running; implementation files were restored byte for byte. This review changes only this document and the specification's review counter.

## Findings

### High: Retries discard results already established by the other half

**Defect class:** restarting one remote operation loses verified information from the other operation when the replacement worker cannot supply a usable result.

In the `worktree-cli` package, [launch_and_follow and Follow::unavailable](../../cli/src/commands/list/wait.rs) coordinate launches and return failures. At lines 186–195, both failure to create a retry id and failure to launch the retry call `unavailable`; lines 305–307 always return an unavailable head, even when `Follow::last` holds a completed, verified head result. A failed PR retry therefore changes a successful branch-head caption into “couldn't check origin.”

The sibling head-retry path has the same information-loss problem in the opposite direction. At lines 255–261, adoption can request another launch before the current receipt's PR result is processed. The wait retains a successful publication in `pr_published`, but has no corresponding retained receipt-derived failure. A verified authentication rejection becomes a generic failure if the replacement worker exits without a receipt.

Reproduction: copy the existing scripted wait fixture and call the production `wait` function. Give the first attempt a completed head and a contended PR receipt, release the PR lock without publishing, and change only how the second launch ends. For the opposite direction, give the first attempt a head-contention receipt with a PR authentication rejection and change only the replacement receipt to missing. These probes failed on the current implementation.

| Site / sibling path | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| PR retry → `launch` error | First head is `InSync`; replacement executable cannot be launched | `head: Unavailable` | Retain the verified finished head; report the PR failure separately |
| PR retry → attempt-id error | Same fixture; no second id is available | `head: Unavailable` | Retain the verified finished head |
| Head retry → missing replacement receipt | First receipt confirms `CredentialsRejected { key: GITHUB_TOKEN }`; replacement head finishes but worker exits without a receipt | PR failure becomes `Other` | Preserve the observed PR diagnosis until newer usable evidence supersedes it |
| Ordinary wait, no retry | Finished head; PR request remains pending at the budget | Finished head retained — clean | Finished head plus pending PR result and timeout hint |
| Any wait, publication before a failed receipt | New usable publication; later receipt reports failure | Publication retained — clean | Successful publication takes precedence |
| Initial launch failure, no previous answer | First worker cannot start | Both halves unavailable/failed — clean | Generic failure; no invented result |

Retain each half's established result across launches, and distinguish initial unavailability from failure to start a replacement. Continue rejecting results from another origin or default branch. Add regression cases for both retry causes, both pre-launch failures, and an exited replacement without a receipt. The existing retry tests cover successful replacements and a shared clock, but miss these failures.

### Medium: Retry paths bypass the original deadline and hide timeouts

**Defect class:** retry and lock-adoption transitions make launch or presentation decisions before enforcing the shared wait deadline.

In `worktree-cli`, [Follow::run](../../cli/src/commands/list/wait.rs) returns a PR retry at lines 271–273 before the deadline check at line 289. [Follow::adoption](../../cli/src/commands/list/wait.rs), which waits for a head lock held for another origin or branch, checks the deadline only while the lock remains held. A release at the deadline returns `Relaunch`. The outer launch loop has no deadline guard. When the head lock remains held beyond the deadline, adoption returns `None`, and the eventual result says `timed_out: false`; the renderer consequently omits the refresh hint after a full-budget wait.

Reproduction: reuse the scripted contention fixture, keep the original monotonic clock, and move only the holder's release time to before, exactly at, and after the 75-second forced budget. The clock advances instantly in this test; it does not make a real network request or wait 75 wall-clock seconds.

| Site / sibling path | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Forced PR-contention retry | Holder releases at 75 s without publishing; head already finished | Second launch occurs at 75 s; result loses the first completed head and reports pending PRs | No replacement launch; retain completed head, report the failed holder, and record timeout |
| Forced head-contention retry | Holder for another branch releases at 75 s | Second launch occurs; final result has `timed_out: false` | No replacement launch after budget; timeout remains visible |
| Forced head-contention wait | Holder remains locked beyond 75 s | Returns at 75 s with `timed_out: false` | Return a timeout result and show the hint |
| Forced head-contention retry | Holder releases at 100 ms | Replacement starts inside the budget — clean for deadline enforcement | Retry is allowed |
| Ordinary PR contention | Holder remains locked through 3 s | One launch; timeout at original budget — clean | No retry; preserve separate head and PR results |
| Forced PR retry before deadline | Replacement remains pending | Existing test ends at the original 75 s budget — clean | Retry does not restart the clock |

Check the remaining budget before every replacement launch, and represent deadline exhaustion distinctly from “no matching attempt.” Preserve final store/receipt reads and independently resolved results. Extend the boundary tests to both retry routes; testing only a retry that starts early does not cover these transitions.

### Medium: Several worker fixtures do not guarantee cleanup after an assertion fails

**Defect class:** tests that start background workers rely on cleanup statements after assertions instead of guaranteeing release and worker completion during stack unwinding.

The specification explicitly requires a drop guard that releases held requests and waits for workers and both locks, including after an assertion fails. In `worktree-cli`, [the new held-PR performance test](../../cli/tests/perf_pr_request.rs) asserts inside its sample loop before releasing the provider and waiting for workers. The changed [terminal scenes](../../cli/tests/level2_list_verbose.rs) likewise reach cleanup only after their assertions. `FakeGitea::drop` releases requests but does not wait for workers; `MixedFixture`, `DesignFixture`, and `RemoveOnDrop` do not supply that wait. The proxy and held-Git fixtures have additional instances of this class.

Reproduction: run the shipped CLI against each held-request fixture, manufacture an assertion failure before its explicit cleanup, catch the unwind, and inspect worker completion. Release and reap any remaining workers before the probe ends. The same failure was applied to guarded controls. The table distinguishes observed leaks from a sample that happened to finish promptly.

| Fixture / sibling sites | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `MixedFixture` + `FakeGitea`, without `Reaper`: `list_flags::refresh_waits_for_both_halves_and_asks_again_like_every_listing`, `refresh_against_a_holder`, and `perf_pr_request::perf_a_held_pr_request_costs_the_listing_only_its_wait` | Panic after listing returns with PR request held; server drops | One worker still running after unwind | Release requests and wait for worker completion before fixture/cache removal |
| `MixedFixture` + `ProxyStub`: `list_prs` proxy-backed tests using `finish_worker`; stale/failing-refresh performance helpers | Panic before explicit worker cleanup | One worker still running | Guaranteed cleanup on failure as well as success |
| `MixedFixture` + `HoldingOrigin`, without release guard: held-check performance cases | Panic while Git head request is held | One worker running and head lock still held | Close the request and wait for both locks and worker completion |
| `DesignFixture` + `FakeGitea`: stale-PR, failed-PR, credentials, spinner, and held-fetch/fallback terminal scenes | Same held-PR panic, through shipped CLI with the fixture's isolated cache | Zero workers in this sample; destructor releases requests but contains no completion wait | Completion guaranteed regardless of scheduling; this sample does not prove a guard exists |
| `list_prs::Reaper`: all Gitea-backed PR binary tests using this guard | Same held-PR panic | Zero workers and free PR lock — clean | Guard releases and waits |
| `list_prs::ReleaseOnDrop`: held-head binary test | Same held-head panic | Zero workers and free head lock — clean | Guard closes requests and waits |
| `remote_fixture::Fixture` + `UploadPackGate`: Git-backed `list_remote_head`, `list_flags`, and held-fetch performance cases | Panic with upload-pack held; gate drops before fixture | Captured worker process has exited — clean | Gate releases before fixture waits |

The first three rows can outlive their temporary repositories or race cache removal. `refresh_against_a_holder` also owns ordinary child handles without a kill-and-reap guard, so setup failures before its waits need coverage too. Some sibling helpers predate this feature; they are listed so the same cleanup defect is not rediscovered one test at a time.

Reuse or share the existing release-and-wait guards in every affected fixture family. Declare them after the provider and cache cleanup object so worker cleanup runs first. Give directly spawned children their own cleanup guard. Keep teardown bounded, and preserve the original assertion failure during unwinding.

## Requirement verification

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Every eligible sequential listing asks for PRs, including a fresh stored answer | Level 1: real worker/binary with loopback Gitea and request counts | Verified |
| Answer received within wait appears in that listing; empty answer clears badges | Level 1: binary tests and post-wait gather tests | Verified |
| Worker is the only PR writer; hidden command has no `--force`; every attempt writes a receipt | Level 1: worker tests, binary receipt test, call-site search | Verified |
| PR/head operations remain independent, including panic and publication failure | Level 1: worker and scripted wait tests | Verified outside the retry failures above |
| Ordinary wait bounds both halves; pending PRs preserve completed head caption | Level 1 plus Level 2 tmux capture | Verified |
| Concurrent queries use the lock and publication id; same-second and empty publication count | Level 1: real-store, scripted wait, and binary contention tests | Verified outside deadline boundaries above |
| Failure with stored/empty/no answer; fresh-answer failure; all age boundaries | Level 1: binary tests, renderer tests, and presentation snapshot | Verified |
| Dim failure/age item, correct order, timeout hint, cleared spinner | Level 2: stale-PR and failed-PR tmux scenes with style and pane assertions | Verified; cleanup gap on failed assertions |
| Ordinary PR authentication warning; head-warning precedence; ambiguous errors stay generic | Level 1 binary tests; Level 2 credentials-line styling capture | Verified outside retry diagnosis loss |
| No origin, ignored repository, local/unsupported origin, changed origin during wait | Level 1: binary/gather tests and renderer badge suppression | Verified |
| PR failure does not prevent a permitted `--ff` | Level 1: loopback-provider binary test | Verified |
| Receipt ownership, old-receipt sweep, malformed/misbound receipt, missing receipt | Level 1: worker, store, and scripted wait tests | Verified outside retry preservation cases |
| Three-second held PR cost and unchanged local gather/render bounds | Serial performance tests | Verified |
| Background-worker cleanup after assertions fail | Level 1 unwind probes of shared fixtures | Incomplete; finding above |

No new requirement needs OS keyboard injection, so Level 3 is not applicable. The changed rendering requirements have real-terminal coverage rather than relying only on manufactured terminal bytes.

The two changed on-disk JSON readers were checked against their writer-generated malformed-input tests: missing/null/wrong-type fields, invalid list elements, duplicate keys, trailing content, format versions, and origin/publication bindings. Invalid stores remain misses; invalid receipts remain missing results. Explicit null on optional repository/URL/key fields has its documented meaning, and a valid empty PR list remains a stored answer. No permissive-empty-answer defect was reproduced in these readers. Provider parsing itself was not changed by this feature.

Current README, list documentation, and worktree skill describe every-run worker requests and one wait for both halves. The performance page records an unauthenticated before/after sample and explicitly records the unavailable authenticated sample, as the implementation plan permits. No additional host evidence or human review is required for this review's readiness decision.

## Validation performed

- `just test`: **836 passed**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`: runner summary **29 passed**; backend proof records **23 actual tmux executions**. The changed stale-PR and failed-PR scenes executed. Unrelated Kitty pixel checks reported unavailable screenshots internally, so the runner's passing count does not prove those pixels were checked.
- `just test-perf`: **30 passed**, run serially without concurrent test load.
- `just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **no stranded tests**. CLI terminal targets are declared, require `terminal-tests`, and that feature is enabled by the area's live L2 recipe and CI metadata.
- Additional scripted wait probes reproduced retry-result loss and both deadline-transition defects. CLI unwind probes reproduced unguarded worker survival and exercised the guarded controls. These probes were temporary and are not included in the passing suite counts.

Passing existing gates does not close the reproduced defects. Add their regression cases, guarantee teardown, and rerun the affected gates before the next review.

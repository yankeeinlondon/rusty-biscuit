---
$schema: feature-review.yaml
ready: false
findings:
  - title: The process-count regression test does not cover the complete worker
    priority: high
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T08:11:10-07:00"
spec: 2026-10-05-worker-git-processes/spec.md
implemented: true
next: 2026-10-05-worker-git-processes/review-2.md
implemented_by: claude/opus
log: worktree/fixes/2026-10-05-worker-git-processes/log.md
description: "A **fix** review of `2026-10-05-worker-git-processes/spec.md`"
fix: 2026-10-05-worker-git-processes/review-1.md
---

# Review 1

The fix is **not production ready** because the required complete-worker process-count test is missing. The metadata replacements work in the tested cases, and all 1,098 local Level 1 tests passed. No human design decision is needed to address the finding.

## Findings

### High — The process-count regression test does not cover the complete worker

**Authority: spec, acceptance criterion 2:** “A test pins the worker's git-process count for a fetch attempt (with the `count-git` counter) to `ls-remote`, `fetch`, and the checkout lookup.” The added test in the `worktree` library, [a_fetching_attempt_starts_only_ls_remote_and_fetch](../../lib/src/remote_update/tests.rs:246), counts only the head-refresh attempt. It does not execute the worker's checkout lookup, setup reads, PR half, or receipt-target lookup.

**Defect class:** a performance regression test measures only one component of the execution path whose process count the spec requires it to protect. Extra metadata processes in the omitted stages could recreate the original Windows slowdown while this test stays green.

I swept the complete worker entry point in `worktree-cli`, [refresh_worker::run](../../cli/src/commands/refresh_worker.rs:90), using a temporary Level 1 probe: initialize a main checkout and local bare origin, push a new commit, move `origin/main` back to the old commit, then run the actual worker. The updated tracking ref proved that fetching occurred. Git trace independently showed the three commands directly started by the application. Git's own transport children are outside the application's counter contract.

| Site within the worker | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Checkout lookup and setup | Main checkout with a local origin ahead of its tracking ref | One checkout `rev-parse`; no metadata Git commands | Same; included in the maintained worker-count assertion |
| PR half | Same fixture; local origin has no supported PR provider | No directly started Git commands | Same; included in the maintained worker-count assertion |
| Head half | Same fixture; remote head differs from tracking tip | One `ls-remote` and one `fetch`; tracking tip updated | Same; the existing component test correctly pins these two calls |
| Complete worker | Same fixture, counter opened around the public worker entry point | Successful fetch, but counter reports **1**, not **3**; no maintained complete-worker assertion exists | Counter includes both threads and a maintained test asserts exactly **3** |

The counter result comes from the existing [run_halves](../../cli/src/commands/refresh_worker.rs:246) thread boundary: its threads do not carry the caller's counting scope. This is relevant to constructing the required test, not a separate finding against unchanged worker code.

Add one Level 1 test through the complete worker entry point using this local-origin fixture. Carry the existing `worktree::git::calls::TaskHandle` into both worker threads and merge their counts with `calls::joined`, so the assertion can see all directly started Git processes. Assert that the tracking ref actually moves and that the total is three. Keep the existing two-process component test. This needs no new protection layer, live network, real terminal, or timing threshold.

## Verification and scope

Reviewed implementation commit `8e7178815` and the current versions of its changed call sites, tests, documentation, and implementation log.

| Requirement | Verification present | Assessment |
| --- | --- | --- |
| Origin URL parity | Level 1 comparisons with Git for local, scp-like, HTTPS, SSH, overlapping rewrites, multiple URLs, and missing origin | Required cases covered |
| Default branch parity | Level 1 tests for symbolic `origin/HEAD`, missing `origin/HEAD`, local `main`, local `master`, and neither | Required cases covered |
| Branch-name validation | Level 1 comparison with Git across valid names and invalid syntax | Required cases covered |
| SSH command and tracking-tip parity | Level 1 comparisons with Git for missing and multiple SSH settings, missing refs, symbolic refs, annotated-tag targets, and packed refs | Covered |
| Configuration changes remain visible | Fresh repository open per read; existing origin-change tests passed | No new blocking defect found |
| Complete worker process count | Level 1 component test plus temporary complete-worker probe | Missing required maintained test; finding above |
| Ordinary wait remains three seconds | Wait unchanged; ordinary local-origin and provider-fixture listings passed in L1 | No timing-policy change found |
| Documentation updates | Metadata reads and fallback described in `docs/performance-testing.md`; worktree skill updated | Implemented |

Ran `just test` from `worktree/`: **1,098 passed, 14 excluded by the selected targets/tier filters**. The temporary complete-worker probe passed and was removed after recording its results. All behavior changed by this fix concerns repository reads and subprocess execution, so Level 1 is appropriate; it introduces no keyboard or terminal-rendering requirement needing Level 2 or Level 3. The new tests compile in active L1 targets without a stranded tier marker.

Cross-OS results are recorded in the implementation log and were not rerun in this review. Their absence from this review's local run does not block readiness. Only the review file and the spec's `review_iterations` metadata are changed; the spec is not marked completed.

## Observations

None.

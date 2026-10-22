---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-18T18:50:27-07:00
spec: 2026-10-18-dmls-cache-optimization/spec.md
fix: 2026-10-18-dmls-cache-optimization/review-1.md
implemented: true
description: "A **fix** review of `2026-10-18-dmls-cache-optimization/spec.md`"
ready: true
findings: []
observations:
    - Existing fingerprint limitation for newly shadowing files
human_in_the_loop: null
human-in-the-loop: false
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
recurrence: false
implemented_by: claude/opus
---

# Review 1: DMLS cache optimization

The fix is production ready under the specification's readiness criteria. No
high, medium, or low findings were identified. This is the first implementation
review, so there are no previous findings to reconcile.

The review compared the implementation with its pre-implementation base,
`df86e5163`, focusing on the Darkmatter payload cache and DMLS scheduling,
startup, configuration refresh, and request timing changes.

## Decisions and deferred work

Two independent agents reviewed the implementation log and the specification
with its plan before the code review proceeded. Neither found an unresolved
choice requiring human review. The implementation made these bounded choices:

- The Darkmatter cache retains the requesting context's original tree and
  tree origin in addition to derived contexts. Nested imports can otherwise
  resolve differently for two callers whose contexts agree at the trigger's
  folder. This preserves the specification's conservative sharing rule.
- DMLS can finish a queued document preparation using its captured context,
  then check the current context and queue another preparation before answering
  requests. A context change can waste one preparation, but requests still use
  the current context. This follows the specification's permitted recheck path.
- Startup preparation has a separate queue lane ahead of ordinary workspace
  refreshes. Documents retain priority over both, and every preparation runs on
  the same worker.
- Detailed cache counts are logged for ordinary workspace refresh passes;
  startup retains its existing readiness summary. Request wait duration is
  omitted when the request never waited. Neither convention changes schema
  selection or request results.

No required implementation work was deferred. The log records an existing
fingerprint limitation, described below, and remaining latency before the server
reads a request. Neither is introduced by this fix. Wider performance sampling
and optimizing individual schema conversions remain outside the specified scope.

## Findings

None.

## Requirement verification

All changed behavior concerns library operations or the language-server
protocol. Level 1 is appropriate: no requirement depends on terminal rendering,
an editor's keyboard encoding, or OS input injection.

| Requirement | Strongest verification reviewed |
| --- | --- |
| Share payloads across compatible package contexts; preserve incompatible contexts and schema identity | Level 1 library scans assert hit/miss counts, shared schema `Arc` identity, dependency paths, and separate document contexts. Cases cover request boundaries, launch scope, home, environment, explicit tree controls, inherited failure, external roots, and nested imports/examples. |
| Bound the cache and refresh recency on hits | Level 1 scans fill 18 distinct keys and repeat them without misses; a separate 33-key case proves hit protection and least-recently-used eviction at capacity 32. Dependency edits and root additions force misses. |
| An unchanged configuration notification adds no preparation | Level 1 protocol sessions check no queued jobs or scans, including the next schema request. Separate controls cover effective changes and an unchanged notification whose rebuilt repository context differs. |
| Prioritize documents and merge workspace work | Level 1 protocol sessions hold probes and verify document responses before remaining probes, merged queued passes, and one follow-up during a running pass. Queue unit tests cover scheduling after worker completion but before dispatch handles completion. |
| Serialize startup and refresh preparation while discovery proceeds | Level 1 protocol sessions verify one worker, no overlapping scans, immediate file-list installation during held preparation, discovered repositories in the startup summary, and change replay. |
| Preserve request lifecycle behavior | Level 1 protocol sessions cover shared waiters, cancellation, shutdown during both startup phases, and requeue after a context change. |
| Report pass-local cache counts and summed registry wait time | Level 1 captured tracing excludes interrupting document work, distinguishes fresh and failed probes, and checks log contents. An injected clock verifies repeated waits, cancellation, shutdown, and requests that never park. Timing unit tests separately exclude time between parked intervals. |
| Preserve schema selection, diagnostics, and summaries | Existing schema-trigger, shipped-schema, validation, document-text, and DMLS scope tests passed with the full area suite. |

The implementation log records the incident's before/after sample: the unchanged
configuration variant fell from 1,260–1,334 ms to 178–192 ms, with a reported
20 ms registry wait in the debug run. These measurements came from a busy host,
so they are directional evidence rather than a portable latency guarantee. The
recorded cold scan had three payload misses and the forced repeat had zero.
The review did not repeat that temporary-instrumentation measurement; automated
work-count tests provide the repeatable regression checks.

The review ran `just test` in `darkmatter/`: **10,533 passed, 10 skipped** in
34.763 seconds. The new cache, queue, startup, and observability tests all ran
and passed. The DMLS integration modules are declared by its explicit L1 target;
the cache tests are compiled into the library's unit-test target. No changed
requirement relies on the skipped tests. Cross-OS evidence remains a CI concern
and does not block this review.

`just check-tier-coverage darkmatter` also passed with zero stranded tests.
The metadata edits passed `git diff --check`. No production code was changed
during this review.

The input robustness matrix does not apply: the change adjusts cache identity,
scheduling, and configuration-change comparison, without adding or changing a
file-format parser, field validator, or deserializer.

## Observations

### Existing fingerprint limitation for newly shadowing files

The implementation log identifies an existing limitation in the Darkmatter
payload cache: its fingerprint tracks dependency bytes and schema-root and
dependency-folder listings. Adding a file in an earlier searched folder that
was not fingerprinted can leave a later cached dependency selected until another
tracked input changes. The specification explicitly preserves the fingerprint,
and this change does not introduce the limitation. Document it or schedule a
separate dependency-search invalidation fix if that workflow becomes important;
it does not warrant expanding this repair cycle.

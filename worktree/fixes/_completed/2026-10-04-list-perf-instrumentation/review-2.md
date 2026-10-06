---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-04T20:33:55-07:00
spec: 2026-10-04-list-perf-instrumentation/spec.md
implemented: false
description: "A **fix** review of `2026-10-04-list-perf-instrumentation/spec.md`"
fix: 2026-10-04-list-perf-instrumentation/review-2.md
previous: 2026-10-04-list-perf-instrumentation/review-1.md
---

The fix is **production ready**. All four findings from review 1 are resolved. This review found no additional defect requiring another repair cycle. No human design decision is outstanding.

## Previous review findings

Review 1 placed all findings under `Findings`, rather than separate unblocked and blocked headings. Its metadata marks every finding unblocked. There were no blocked findings to reconsider.

| Previous finding | Implementation checked | Result |
| --- | --- | --- |
| High: The human performance report lacks real-terminal verification | In worktree-cli, [the new terminal tests](../../cli/tests/level2_list_perf.rs) run the shipped command against a local bare origin in headless tmux. They capture connectors, Git counts, share columns, worker diagnostics, alignment, and wrapping at 80/120 columns, plus graph history at 100 columns. | Resolved; both new Level 2 tests passed. |
| Medium: Worker diagnostics lose share and reconciliation semantics when rendered | In worktree-cli, [the renderer](../../cli/src/perf.rs) uses one span projection for foreground and worker trees. In biscuit-terminal, [MetricsTree](../../../biscuit-terminal/lib/src/components/metrics_tree.rs) can honor the root's supplied share. | Resolved; worker headings and descendants have no percentages, and sequential worker parents retain remainder/excess rows. |
| Medium: Saturated arithmetic accepts timing documents that do not reconcile | In worktree, [the timing model](../../lib/src/timing.rs) computes exact sums with checked wide arithmetic. Both public constructors reject durations or remainders outside the wire range. Decoder and receipt tests exercise the previously accepted boundary. | Resolved; unrepresentable excess is rejected, while maximum representable excess and large concurrent children remain valid. |
| Low: Disabled worker instrumentation still reads a stage clock | In worktree-cli, [the worker](../../cli/src/commands/refresh_worker.rs) creates its halves clock only when timing is enabled and consumes it within the same enabled branch. | Resolved; untimed halves receive no instrumentation list and the receipt has no durations member. |

## Class sweeps

These sweeps cover the sibling sites identified in review 1, including those that were already correct.

| Class | Sites checked | Verification and observed result |
| --- | --- | --- |
| Human-report verification through a real terminal | Human/JSON subprocess tests; pseudo-terminal framing test; synthetic renderer tests; new plain-terminal and graph-history tmux scenes; existing listing, graph, and spinner terminal suites | Level 1 remains responsible for data/framing; Level 2 now verifies human layout. All 41 terminal tests passed. No keyboard behavior was added, so Level 3 is unnecessary. |
| Display projection drops share or remainder rules | Foreground root; foreground sequential and concurrent spans; worker heading; complete/partial/missing/invalid launch rows; nested PR and head halves; worker concurrent group; adopted/origin-changed summaries; other MetricsTree consumers in sniff and claudine | Final rendered synthetic reports cover both worker halves, nested excess, the 1 ms remainder threshold, and all six summary states. The shared component's default root percentage is preserved for existing consumers; the worker heading opts into its unknown share. |
| Numeric validation clamps a false reconciliation equation | Command root; nested sequential spans; standalone worker root and head half; command-embedded worker root and head half; optional receipt durations; both constructors and serializers; concurrent-parent controls | Command, standalone-worker, construction, and receipt boundary tests passed. The receipt keeps its accepted outcome when durations are invalid. Shared recursive validation applies the same rule at every depth and for either worker half. |
| Timing-only work runs before the disabled guard | Library record/parent wrappers; local/history gathering; foreground launch; CLI render/write wrappers; worker entry, individual halves, and halves envelope; task count scopes | Guards bypass stage clocks and count scopes when timing is disabled. Enabled/disabled listing and wait tests preserve facts, Git operations, outcomes, budgets, and receipt cleanup. Deadline clocks and the existing process-entry anchor serve functional behavior and remain appropriate. |

## Input robustness

JSON is the only timing format. The command and receipt matrices passed, together with the standalone-worker numeric-boundary test. The readers share strict object parsing and recursive span validation.

The command matrix covers root version, scope, total, reconciliation fields, span arrays, worker arrays and summary; span stage, elapsed time, child kind, children and optional Git counts; and launch report presence. The receipt matrix checks optional durations, worker version/total/spans/remainders, nested span fields, and optional Git counts through the public receipt result. Separate launch tests check ordering, attempt identity, status/report agreement, and summary consistency.

Required fields reject absence, null, wrong types, duplicate keys, and invalid values. Arrays reject malformed elements instead of filtering them out. Empty arrays remain distinct from missing fields and must satisfy reconciliation and status rules. Optional Git counts distinguish absence from measured zero and reject null. Absent receipt durations mean missing; malformed durations mean invalid without discarding the outcome. Trailing content is rejected. Unknown optional fields remain accepted.

The repaired numeric row is verified across each public reading route:

| Reading route | Excess beyond the wire range | Valid boundary/control |
| --- | --- | --- |
| Command root and nested sequential parent | Rejected | Exact maximum excess accepted; concurrent children excluded from sums |
| Standalone worker document | Root and nested head excess rejected | Largest fitting excess accepted |
| Worker document embedded in a command | Root and nested head excess rejected, including a partial launch | Valid worker reports still decode |
| Receipt's optional worker document | Root and nested head excess become invalid timings; outcome retained | Fitting excess and large concurrent halves retain complete timings |
| Public construction followed by serialization | Unrepresentable durations, accumulated stages, and excess rejected before serialization | Maximum excess serializes and round-trips exactly |

## Requirements and verification levels

| User-facing requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Library-owned pipeline, explicit repository paths, concurrent repository isolation | Level 1 public pipeline tests, including concurrent listings | Appropriate |
| Typed stage paths, child kinds, exact microsecond reconciliation, and strict decoding | Level 1 timing, constructor, and decoder tests | Appropriate |
| Worker reports captured before deletion without extending waits or changing outcomes | Level 1 scripted wait tests plus real worker/receipt tests | Appropriate |
| Disabled instrumentation preserves facts and Git operations | Level 1 enabled/disabled equivalence tests and guarded clock paths | Appropriate |
| Stderr-only output, final framed JSON record, LF/CRLF, misleading commit text, flag errors, and no report on listing errors | Level 1 subprocess and pseudo-terminal tests | Appropriate |
| Human hierarchy, percentage rules, Git suffixes, and column alignment | Level 2 real-terminal capture, backed by deterministic Level 1 renderer tests | Appropriate |
| Existing listing, graph, and spinner presentation after moving gathering into the library | Level 2 terminal suite plus Level 1 behavior tests | Appropriate |
| Warm/cold gathering bounds, graph timing, and bounded remote waits | Serial performance gates reading structured stages and outer elapsed times | Appropriate |

The new terminal target is declared in the CLI manifest, requires the existing `terminal-tests` feature enabled by CI metadata, and has names selected by the live Level 2 recipe. The tier audit reports zero stranded tests. Source inspection confirms that the library has no terminal dependency, the benchmark uses the shared local entry point without launching a worker, and functional behavior tests no longer scrape timing labels. The implementation log records the required three warm runs with top-level gaps below 1 ms. Public constructor documentation and the performance topic page describe the new fallible construction contract.

## Recurrence

None. All defect classes reported in review 1 are resolved; this review reports no recurring finding.

## Validation

- `just test` in worktree: 1,080 passed; 14 performance tests excluded as intended.
- `just test-l2` in worktree: 41 passed, including both new performance-report terminal tests.
- `just test-perf` in worktree: 14 passed in an isolated serial run.
- `just lint` in worktree: passed.
- `just check-tier-coverage worktree`: passed; zero stranded tests.
- `just test metrics_tree` in biscuit-terminal: 50 passed.
- `just lint` in biscuit-terminal: passed.
- `git diff --check`: passed.

Performance gates ran after the other review test jobs finished. Cross-OS evidence is left to CI and does not determine readiness. This review changes only review documents and the requested spec metadata; no implementation changes or commits were made.


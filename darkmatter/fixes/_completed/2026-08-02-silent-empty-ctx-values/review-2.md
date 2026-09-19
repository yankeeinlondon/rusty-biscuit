---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T02:25:08-07:00"
spec: 2026-08-02-silent-empty-ctx-values/spec.md
implemented: false
description: "A **fix** review of `2026-08-02-silent-empty-ctx-values/spec.md`"
fix: 2026-08-02-silent-empty-ctx-values/review-2.md
previous: 2026-08-02-silent-empty-ctx-values/review-1.md
---

# Review 2: Silent Empty `ctx` Values

## Verdict

The fix is **ready for production**. Both high-priority findings from Review 1
are closed, and this iteration found no new functionality, correctness,
performance, ergonomics, or test-rigor gap in the specification's scope.

Human review is not required. The remaining release evidence is ordinary
cross-OS CI, which does not affect this review's readiness decision.

## Previous-Finding Closure

### Persistent cache hits discard child partial-capture diagnostics — closed

Semantic compose-result persistence was removed after Review 1, so a warm
cache-root run recomposes the child and regenerates its warnings. The remaining
run-local compose cache stores the child's complete `ComposeResult`, including
its `ComposeReport`, and the transclusion path merges that report on both cache
misses and hits.

Level 1 regressions now prove both boundaries:

- `a_run_local_hit_replays_a_child_only_partial_capture_warning` observes an
  actual run-local hit and compares its warning count, content, and output with
  cache-disabled composition.
- `a_child_only_partial_capture_warns_identically_on_cold_and_warm_runs`
  compares two requests sharing a cache root and also proves that no semantic
  result was persisted.
- `a_captured_group_without_evidence_renders_its_typed_projection` positively
  matches report warnings to the context's `PartialRuntimeCapture` diagnostics
  once each and in order.

This closes the finding without restoring the semantic-result persistence path
removed by the separate content-policy fix.

### Body failures omit the available authored location — closed

The interpolation engine now returns a first-pass expression span through
`LocatedInterpolationError`. The body-stage boundary promotes that span to an
authored line only when the scanned prefix and expression still match the
loaded source's body after line-ending normalization. Generated expressions
and expressions following an earlier rewrite retain file identity but do not
receive a guessed line.

Level 1 tests cover a CRLF document with frontmatter, a transcluded child,
generated rescan content, and content shifted by an earlier page-block stage.
The rendered diagnostics assert the child/root file, one-based authored line,
and focused excerpt. This satisfies the spec's “when available” location
contract without mislabeling generated offsets as authored positions.

## Findings

No unblocked findings remain. Review 1 had no blocked findings to reassess, and
no previously blocked item became actionable in this iteration.

## Verification-Level Assessment

All requirements concern composition values, typed diagnostics, cache/report
behavior, source provenance, or CLI process outcomes. Level 1 is the correct
maximum tier: no requirement depends on a real terminal emulator, terminal
input encoding, OS keyboard injection, browser layout, paste/IME, mouse input,
or scrolling. Levels 2 and 3 are therefore not required.

| Spec verification | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Typed fatal error names variable, group, source, and available location | Level 1 library, rendered-status-block, and CLI process tests | Sufficient; authored body and transcluded-child lines are asserted. |
| 2. Frontmatter, body, condition, and `$()` surfaces | Level 1 integration and shared-evaluator tests | Sufficient. |
| 3. Captured null/empty values remain values | Level 1 unit and integration tests | Sufficient. |
| 4. Unavailable evidence retains `PartialRuntimeCapture` | Level 1 direct-compose, run-local-hit, and cold/warm cache-root tests | Sufficient; warning count and content are positively compared. |
| 5. Unknown keys retain only the existing diagnostic | Level 1 integration and checked-lookup tests | Sufficient. |
| 6. Missing projected key is an internal invariant failure | Level 1 unit and integration tests | Sufficient. |
| 7. Local, nested, sibling, and remote child-first capture uses one epoch | Level 1 filesystem, concurrency, and mocked-HTTP tests | Sufficient. |
| 8. Frozen child-first capture fails without ambient fallback | Level 1 integration and authority tests | Sufficient. |
| 9. Cache reuse cannot return stale output or bypass the fatal contract | Level 1 cache-access and no-semantic-persistence tests | Sufficient under the current memory-only semantic-result cache design. |
| 10. Removing root upgrade or child handoff produces a named failure | Level 1 authority tests plus recorded mutation evidence | Sufficient. |
| 11. Context-free graphs avoid discovery-backed capture | Level 1 graph and constructor tests | Sufficient. |
| 12. Standalone conditions capture only reached groups and preserve classification | Level 1 tests through the public entry point | Sufficient. |

## Implementation Assessment

The checked lookup preserves the four required absence states, and every
compose expression surface reaches that typed classification. Ambient context
growth remains request-scoped and monotonic, while caller-supplied frozen
contexts cannot consult ambient state. Child context requirements are handed
off before child evaluation, and run-local cache hits replay the child's report
and recorded context groups.

The authored-location repair is deliberately conservative: it adds useful
lines and excerpts only when the loaded-source prefix proves the location. Its
thin compatibility wrapper leaves non-body interpolation callers on their
existing API and behavior. I found no simpler or materially faster design that
would preserve the same provenance guarantee.

## Validation Performed

- GitNexus was refreshed to the current commit. `interpolate_text` has HIGH
  upstream impact: 47 symbols across the interpolation and inline-compose
  flows. The focused regression coverage exercises both its compatibility
  wrapper and the new located body path.
- The Review 1 cache-warning test, authored root-line test, and authored
  transcluded-child-line test passed individually at Level 1.
- Both lenient interpolation characterization tests passed against the final
  reviewed source state.
- `darkmatter/just lint` passed for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.
- The final broad `darkmatter/just test` run reached 2,262 passes before 14
  unrelated network/effects tests timed out together under severe shared-host
  contention; nextest then canceled the remaining tests. These are the same
  load-sensitive test families recorded in the implementation log. All tests
  specific to this fix passed, so the host-load timeouts do not change the
  readiness verdict.
- GitNexus `detect-changes --scope all` reports critical aggregate risk across
  518 dirty-worktree files and 72 flows. That result covers many concurrent,
  unrelated changes and is not a clean per-fix risk measurement; the exact
  HIGH impact above and the focused executable checks are the applicable
  evidence for this review.

---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T10:01:07-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-2.md
next: 2026-07-13-error-propogation/review-3.md
previous: 2026-07-13-error-propogation/review-1.md
---

# Review 2: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. Review 1's registry, production
snapshot-consumer, and generator-fixture issues have materially improved, but
the central no-flattening contract is still violated at several transport
boundaries. The required proxy-route parity remains explicitly unimplemented,
and the full Level-2 gate is red.

## Findings

### 1. Critical — Typed causes and snapshots are still flattened at in-scope boundaries

The new Rust-aware guard removed the 64-entry
error-propagation-followup burn-down, but its remaining exceptions document
known losses that conflict with D1, D8, D9, and Acceptance Criterion 1:

- transport-allow.toml:32-38 permits build_and_run_loop to record a flattened
  Report after provenance was erased by its upstream return type.
- transport-allow.toml:144-150 permits try_inline_closure to flatten a typed
  CompositionError because correcting it would alter lifecycle routing fields.
- transport-allow.toml:240-246 permits run_sequence_steps to store flattened
  Report text after the same upstream erasure.
- reporting/ingest.rs:166-192 converts filesystem, fingerprint, and database
  errors directly to strings. SyncFailure then persists only message
  (reporting/types.rs:52-72), even though D9 requires the shared versioned
  snapshot shape at persistence boundaries. The allow entry itself acknowledges
  that some causes are double-flattened.

There is also a loss outside the guard's modeled shapes. Composition prep
captures launch discovery as a DiagnosticSnapshot, but pipeline.rs:665-671
extracts snapshot.message and launch.rs:73-82 wraps that text in a generic
Report for --repo. The snapshot's code, category, detail, origin, and causal
chain can no longer participate in selection or rendering. The corresponding
test checks only message substrings, so it cannot detect this regression.

The distinction between a concrete error and color_eyre::Report explains why a
source cannot be attached at the final site; it does not satisfy the spec's
requirement to preserve a projection before entering that boundary.

Required change: carry either a concrete typed error or DiagnosticSnapshot
through the execution, loop, sequence, reporting, and --repo boundaries.
Persist the snapshot in report records, and make the guard reject
snapshot.message-to-generic-Report re-erasure. If the inline lifecycle routing
change is intentionally deferred, the specification and Acceptance Criterion 1
must be narrowed rather than declaring the migration complete.

### 2. High — The real-terminal proxy test proves failure of Acceptance Criterion 5

level2_typed_error_render_capture.rs:563-636 states that the two motivating
proxy routes do not share a resolver and deliberately asserts their divergence.
The initialize route renders “Unresolvable file reference”; the terminal route
renders “failed to load Markdown.” The exact Level-2 tmux test passes, so this
is verified behavior rather than a theoretical concern.

Acceptance Criterion 5 requires parity of code, headline, hint, and typed
resolution detail, allowing only event/property context to differ. A decision
to defer resolver convergence to another feature does not implement that
criterion.

Required change: route both proxies through the same resolution seam, then
change the Level-2 test to assert equal code, headline, hint, and resolution
detail while retaining only the allowed route-specific context.

### 3. High — The required full Level-2 gate fails

just test-l2 reached the real tmux suite and failed
level2_context_default_at_140_fills_cap_in_tmux on all four attempts. The
contract in level2_context_capture.rs:176-188 expects a maximum visible width of
138..=139 cells but captured 140. Fail-fast stopped the run after 41 of 141
tests, leaving 100 Level-2 tests unexecuted.

This rendering check is not central to typed error propagation, but Acceptance
Criterion 9 explicitly requires the full Claudine Level-2 gate to pass.

Required change: restore the reserved right margin (or deliberately revise the
documented width contract), then rerun the complete Level-2 suite.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed errors or a versioned snapshot across transports | Level 1 Rust-aware source guard and record tests | **Gap.** The allowlist retains known losses and the guard misses the --repo snapshot re-erasure. |
| Discover and select registered diagnostics | Level 1 registry parity, downcast, source-chain, and selection tests | Appropriate for this in-process selection contract. |
| Use the same effective diagnostic for rendering, expression context, and machine fields | Level 1 integration and snapshot assertions | **Gap.** --repo and persisted reporting records discard the selected snapshot. |
| Render the motivating initialize proxy failure as a structured block | Level 2 tmux capture, with Level 1 process assertions | Appropriate; the focused Level-2 route passes. |
| Give both proxy routes the same diagnostic identity and content | Level 2 tmux capture | **Gap.** The test intentionally asserts different headlines and failure stages. |
| Preserve frontmatter excerpts, no-color behavior, and exactly-once emission | Level 2 tmux capture plus Level 1 assertions | Appropriate for the exercised routes. |
| Keep diagnostic detail structured and catalog metadata complete | Level 1 catalog, corpus, and detail assertions | Appropriate for serialized data and registry invariants. |
| Preserve exit codes, ordering, retry semantics, message hygiene, and route behavior | Level 1 characterization with representative Level 2 captures | Appropriate for exercised routes; no keyboard encoder contract is involved. |
| Pass all required Claudine gates | Level 1/Level 2/guard execution | **Gap.** Full Level 2 is red; full test and lint runs did not finish inside the non-interactive command ceiling. |

Level 3 is not applicable to this feature: none of its requirements depend on
physical keyboard events or a terminal emulator's input encoder.

## Verification Performed

- The focused proxy-parity Level-2 tmux test passed and confirmed the documented
  diagnostic divergence.
- just test-l2 failed on the 140-column context capture described above.
- The generator drift integration target passed 6 of 6 tests, confirming Review
  1's stale fixture-path blocker is fixed.
- The transport/error guard suite passed 17 of 17 tests.
- Warm just test completed the catalog tests and all 3,526 claudine library
  tests without an observed failure before the command ceiling; the complete
  area run did not finish.
- just lint completed the transport guard, lifecycle documentation guard,
  catalog-types, claudine library, and contract stages without an observed
  failure before the command ceiling; the complete CLI lint did not finish.

## Production Readiness Closure

Production readiness requires all three findings to be closed: eliminate or
explicitly respecify the remaining typed/snapshot losses, make the two proxy
routes satisfy the specified Level-2 parity contract, and obtain a clean full
Level-2 run. The incomplete full test and lint gates must also be rerun to
completion.

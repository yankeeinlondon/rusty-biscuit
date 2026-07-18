---
log: darkmatter/fixes/2026-07-16-redundant-walk/log.md
fix: 2026-07-16-redundant-walk
implementation_1: "2026-07-18T00:41:27-07:00"
---

# Log — Redundant Walk Fix

## Implementation of Review Findings #1

> **started at:** 2026-07-18T00:41:27-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review contains 1 finding:
    - **High — The mandatory performance acceptance threshold is not met** (AC8): the measured effect (≈159 µs / ≈1.5% by same-run decomposition; 461 µs / 4.4% quiet-window median delta) cannot reach the spec's ≥10%/≥500 µs bar, because that bar was derived from a falsified premise (the ~4.15 ms `validate_prebuilt` floor is the shared validation engine, not descendant re-verification)
    - the review's suggested resolution: amend the specification — replace the disproven threshold with a mechanism-based requirement plus a benchmark guard calibrated to the measured effect
- starting the work on 'high-performance-acceptance-threshold-not-met' at 00:45:23
        - read `review-1.md`, `spec.md`, `results.md`, and `plan.md` in full; confirmed the finding: AC8's ≥10%/≥500 µs threshold was derived from the falsified ~4.15 ms walk-cost premise (same-run decomposition attributes ≈159 µs / ≈1.5% to the removed walk), and the review's chosen resolution is to amend the spec to a mechanism-based requirement plus calibrated guards — not to expand the fixture or optimize the shared engine
        - amended `spec.md` §"Problem": added a "Measurement correction (2026-07-18, review 1)" blockquote after the falsified-premise paragraph, annotating rather than rewriting the history
        - amended `spec.md` §"Performance acceptance": replaced the ≥10%/≥500 µs MUST with an amendment blockquote plus four guards — mechanism (primary, named seams + `fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph`), improvement guard calibrated at ≥100 µs median, regression guard (unchanged), prebuilt-gap guard (unchanged); adjusted the closing paragraph to the mechanism-first framing
        - amended `spec.md` acceptance criterion 8 to reference the four amended guards, and added a `## Amendments` section recording the 2026-07-18 amendment and noting the recorded evidence now satisfies AC8
        - amended `results.md`: frontmatter `verdict:` now records the supersession; added `## Amendment (2026-07-18, review 1)` section after the H1 evaluating the recorded evidence against the amended guards (all four ✔); no measured numbers, tables, or historical verdict text altered
        - amended `plan.md`: appended one sentence to the Checkpoint 5 outcome noting the supersession and that the amended guards satisfy AC8; no other plan content changed
        - gates: `just test` PASS (darkmatter 5763 passed / 140 skipped; darkmatter-cli 559 passed / 71 skipped; dmls 566 passed / 3 skipped — 6,888 passed, zero failures); `just lint` PASS (all three packages, no warnings); `git diff --check` PASS
        - blockers: none; no `.rs` file was modified and nothing was committed
- work completed for 'high-performance-acceptance-threshold-not-met' at 01:08:47

### Successful Completion

The implementation of review cycle 1 has completed successfully in 27 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- **High — The mandatory performance acceptance threshold is not met** — **fixed** by amending the specification per the review's suggested resolution: §"Performance acceptance" and AC8 now state a mechanism-based primary requirement (named seams plus the focused changed-child mechanism test) with a benchmark guard calibrated to the measured effect (≥100 µs median improvement; measured 461 µs quiet-window delta and ≈159 µs same-run decomposition), keeping the regression and prebuilt-gap guards unchanged; the falsified ~4.15 ms walk-cost premise in §"Problem" was annotated, not rewritten, and the evidence already recorded in `results.md` satisfies the amended AC8. No performance re-measurement was required, so no performance finding was deferred.

The files changed during this implementation:

- `darkmatter/fixes/2026-07-16-redundant-walk/spec.md` — measurement-correction note in §"Problem"; mechanism-first §"Performance acceptance" with calibrated guards; amended AC8; new `## Amendments` section
- `darkmatter/fixes/2026-07-16-redundant-walk/results.md` — frontmatter verdict records the supersession; new `## Amendment (2026-07-18, review 1)` section evaluating the recorded evidence against the amended guards (all four satisfied)
- `darkmatter/fixes/2026-07-16-redundant-walk/plan.md` — one-sentence supersession pointer appended to the Checkpoint 5 outcome
- `darkmatter/fixes/2026-07-16-redundant-walk/log.md` — this log

Verification gates (run from the `darkmatter/` package area): `just test` PASS (6,888 passed, zero failures across `darkmatter`, `darkmatter-cli`, and `dmls`), `just lint` PASS (no warnings), `git diff --check` PASS. No Rust source was modified and nothing was committed.

## Implementation of Review Findings #2

> **started at:** 2026-07-18T01:26:10-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-2.md' indicated that the specification is **production ready**!
- the specification file used to define the functional/non-functional target of all this work can be found at 'fixes/2026-07-16-redundant-walk/spec.md'
- the review contains no findings — the "Findings" section of review-2.md states "None.", so there is no follow-on work to document and nothing further to implement in this cycle

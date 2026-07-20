---
log: darkmatter/fixes/2026-07-16-redundant-walk/log.md
fix: 2026-07-16-redundant-walk
implementation_1: "2026-07-18T00:41:27-07:00"
implementation_3: "2026-07-19T20:19:28-07:00"
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

## Implementation of Review Findings #3

> **started at:** 2026-07-19T20:19:28-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains 1 finding:
        - **High — Fragment validation rereads transcluded children instead of using the fresh graph snapshot** (AC5): `validate_graph_contents` calls `collect_composed_heading_slugs` when `validate_fragments` is enabled, and that helper reloads each graph child from disk with `Markdown::try_from` (`darkmatter/lib/src/markdown/reference/validate.rs:820-833`), so a post-build heading edit leaks into the fresh one-step report — violating the snapshot contract in `spec.md:175-182`
        - the review's suggested resolution: retain build-time prepared heading slugs for graph children in a private, non-serialized snapshot owned by `ReferenceGraph` (or equivalent private artifact); make composed-heading collection use that snapshot on the fresh path; keep non-descendant cross-document fragment targets on the existing disk path; extend the paired mechanism test with `validate_fragments: true`, a root fragment link to a transcluded child's original heading, and a post-build heading rename — fresh validation keeps the original heading result while checked validation still returns a changed-dependency `ReferenceGraphMismatch` before flattening
- starting the work on 'high-fragment-validation-rereads-transcluded-children' at 20:21:00
        - dispatched a single implementation subagent (only one finding in this review); it owns its own log sub-items for this finding
        - design decision: retained **build-time prepared heading slugs** (not loaded child content) in a new private, non-serialized `PreparedHeadingSnapshot` owned by `ReferenceGraph`. Rationale: `build_node` already runs InlinePre preparation and parses the TOC for every node (`build_heading_index`), so capturing slugs there reuses the build's single TOC parse at zero extra I/O or parsing cost, matches the codebase's compact-private-artifact ethos (provenance.rs explicitly avoids retaining full `Markdown` values), and produces exactly the slug set `collect_prepared_heading_slugs` computes. Keying is by best-effort canonical path via a shared `heading_slug_key` helper, identical to the existing `HeadingSlugCache` keying (portable across macOS `/var` vs `/private/var`, Linux, Windows)
        - GitNexus impact on `from_build` before editing: MEDIUM risk, 7 direct callers, all crate-internal (`pub(crate)`); no public surface touched
        - changes made:
                - `lib/src/markdown/reference/snapshot.rs` (new): `PreparedHeadingSnapshot` (canonical-path-keyed slug map, `record`/`slugs_for`) plus the shared `heading_slug_key` helper; unit tests for round-trip and cross-spelling key stability
                - `lib/src/markdown/reference/mod.rs`: registered `pub(crate) mod snapshot`
                - `lib/src/markdown/reference/types.rs`: new private `prepared_headings` field on `ReferenceGraph` (mirroring the `provenance` pattern — absent from manual `Debug` and from `ReferenceGraphView` JSON); `from_build` takes it as a new `pub(crate)` parameter; new `pub(crate)` accessor; extended the debug non-leak test to assert `prepared_headings` never renders; updated 3 synthetic-graph test call sites
                - `lib/src/markdown/reference/graph.rs`: `build_heading_index` now also returns the lowercased slugs from the same TOC parse; `build_node` records each file-sourced node's slugs into the snapshot (new `&mut` parameter, threaded through all 4 recursive call sites); `build_graph_inner` assembles the snapshot and passes it to `from_build`; updated 1 test call site
                - `lib/src/markdown/reference/file_tree/model.rs`: updated 2 synthetic-graph test call sites
                - `lib/src/markdown/reference/validate.rs`: `collect_composed_heading_slugs` now reads descendant headings from the graph's snapshot and seeds the run cache with them (so a `path#fragment` target naming a graph descendant resolves to the same snapshot entry), keeping the disk-loading fallback only for snapshot-less synthetic graphs; `cached_prepared_heading_slugs` now keys via the shared `heading_slug_key`; fixed drifted docs (`HeadingSlugCache` no longer claims every entry is disk-produced; `collect_composed_heading_slugs` no longer says "disk-loaded"); added the new paired mechanism test `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings` (`validate_fragments: true`, root transcludes child + links `#child-heading`, post-build heading rename; fresh seam stays valid, checked seam returns changed-dependency `ReferenceGraphMismatch` naming child.md)
        - `cargo check -p darkmatter --lib`: PASS (no warnings)
        - gate results:
                - focused `cargo nextest run -p darkmatter fresh_seam`: PASS — 2/2 (the existing paired test plus the new `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings`)
                - focused reference/validation/graph selection `-E 'test(/reference|validate|fragment|heading|snapshot|graph|file_tree/)'`: PASS — 568/568
                - `just lint` (darkmatter, darkmatter-cli, dmls): PASS, no warnings
                - `just test` (full Level-1 area suite, completed): PASS — darkmatter 5,862 passed / 0 failed (140 skipped), darkmatter-cli 612 passed / 0 failed (71 skipped), dmls 567 passed / 0 failed (3 skipped)
                - `git diff --check`: PASS
                - GitNexus `detect_changes` (scope all): changed reference symbols are exactly the expected set (`build_graph_inner`, `build_node`, `build_heading_index`, `from_build`, `collect_composed_heading_slugs`, `cached_prepared_heading_slugs`, `HeadingSlugCache`, plus test modules); the remaining dirty-worktree files belong to other in-flight features, not this change
                - post-suite formatting pass: hand-wrapped the new `build_heading_index` signature to `main`'s multi-line style (the only fmt drift this change introduced; all other `cargo fmt --check` flags on touched files are pre-existing `main`-vs-local-rustfmt drift on untouched lines, e.g. `errors.rs`, `provenance.rs`, import ordering). After that whitespace-only edit: focused `fresh_seam` tests re-run PASS (2/2), `just lint` re-run PASS, `git diff --check` re-run PASS; the full-suite counts above were recorded immediately before this whitespace-only edit
        - blockers: none
- work completed for 'high-fragment-validation-rereads-transcluded-children' at 21:14:18

### Successful Completion

The implementation of review cycle 3 has completed successfully in 55 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- **High — Fragment validation rereads transcluded children instead of using the fresh graph snapshot** — **fixed** per the review's suggested resolution: `ReferenceGraph` now owns a private, non-serialized `PreparedHeadingSnapshot` of build-time prepared heading slugs (captured from the build's existing single InlinePre/TOC pass at zero added I/O, keyed by best-effort canonical path via a shared `heading_slug_key` helper); `collect_composed_heading_slugs` reads descendant headings from that snapshot and seeds the run cache with them, so neither the composed-heading collection nor a `path#fragment` target naming a graph descendant rereads disk; non-descendant cross-document fragment targets keep the existing disk-loading path; and the new paired mechanism test `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings` proves that with `validate_fragments: true` a post-build heading rename leaves fresh validation reporting the original heading as valid while checked validation returns a changed-dependency `ReferenceGraphMismatch` before flattening. No performance measurement was required by this finding, so no performance finding was deferred.

The files changed during this implementation:

- `darkmatter/lib/src/markdown/reference/snapshot.rs` — new module: `PreparedHeadingSnapshot` plus the shared `heading_slug_key` helper, with unit tests
- `darkmatter/lib/src/markdown/reference/mod.rs` — registered the `snapshot` module
- `darkmatter/lib/src/markdown/reference/types.rs` — private `prepared_headings` field on `ReferenceGraph` (absent from `Debug` and serialized views, mirroring `provenance`), new `from_build` parameter and `pub(crate)` accessor, extended debug non-leak test, updated synthetic-graph test call sites
- `darkmatter/lib/src/markdown/reference/graph.rs` — `build_heading_index` also returns lowercased slugs from the same TOC parse; `build_node` records each file-sourced node's slugs; `build_graph_inner` assembles the snapshot
- `darkmatter/lib/src/markdown/reference/validate.rs` — snapshot-based composed-heading collection with cache seeding, shared cache keying, drift-fixed docs, and the new paired fragment/heading mechanism test
- `darkmatter/lib/src/markdown/reference/file_tree/model.rs` — updated synthetic-graph test call sites
- `darkmatter/fixes/2026-07-16-redundant-walk/log.md` — this log

Verification gates (run from the `darkmatter/` package area): focused `fresh_seam` tests PASS (2/2), focused reference/validation/graph selection PASS (568/568), `just test` PASS in full (darkmatter 5,862 passed / 0 failed / 140 skipped; darkmatter-cli 612 / 0 / 71 skipped; dmls 567 / 0 / 3 skipped), `just lint` PASS (all three packages, no warnings), `git diff --check` PASS, and GitNexus `detect_changes` confirmed the changed symbols are exactly the expected reference/file-tree set. Nothing was committed.

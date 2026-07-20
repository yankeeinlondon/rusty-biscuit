---
log: darkmatter/fixes/2026-07-16-redundant-walk/log.md
fix: 2026-07-16-redundant-walk
implementation_1: "2026-07-18T00:41:27-07:00"
implementation_3: "2026-07-19T21:35:45-07:00"
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

> **started at:** 2026-07-19T21:35:45-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains 1 finding:
        - **High — Fragment validation rereads transcluded children instead of using the fresh graph snapshot** (AC5): `validate_graph_contents` calls `collect_composed_heading_slugs` when `validate_fragments` is enabled, and the helper reloads each graph child from disk with `Markdown::try_from` (`darkmatter/lib/src/markdown/reference/validate.rs:820-833`), so a post-build heading edit leaks into the fresh one-step report — violating the snapshot contract in `spec.md:175-182`
        - the review's suggested resolution: retain build-time prepared heading slugs for graph children in a private, non-serialized snapshot owned by `ReferenceGraph` (or equivalent private artifact); make composed-heading collection use that snapshot on the fresh path; keep non-descendant cross-document fragment targets on the existing disk path; extend the paired mechanism test with `validate_fragments: true`, a root fragment link to a transcluded child's original heading, and a post-build heading rename — fresh validation keeps the original heading result while checked validation still returns a changed-dependency `ReferenceGraphMismatch` before flattening
- orchestrator notes:
        - on entry, the worktree already contained a complete prior iteration of this same finding (commit `f82221e0c feat(darkmatter): add PreparedHeadingSnapshot to ReferenceGraph`, plus the cycle-closing `152ea6b84`). Rather than redo the implementation from scratch, this iteration acts as an independent verification pass that retains the prior source changes when they satisfy the review's suggested resolution and only edits further if a gap is found
        - the prior implementation matches the suggested resolution: a new private `PreparedHeadingSnapshot` is owned by `ReferenceGraph` (`pub(crate)` field, absent from `Debug` and serialized views, mirroring `provenance`); `build_heading_index` returns lowercased slugs from the build's existing single TOC parse; `build_node` records each file-sourced node's slugs; `collect_composed_heading_slugs` reads descendant headings from that snapshot and seeds the run cache via the shared `heading_slug_key` helper; the disk-loading fallback remains only for synthetic graphs without a snapshot; and the paired mechanism test `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings` exercises `validate_fragments: true` with a root fragment link to a transcluded child's original heading and a post-build heading rename
- starting the work on 'high-fragment-validation-rereads-transcluded-children' at 21:37:00
        - dispatched a single verification subagent (`rust-developer`) with instructions to load the `darkmatter`, `rust`, and `rust-testing` skills; independently confirm the implementation against the review's suggested resolution point-by-point; and run the focused `fresh_seam`, broader reference/validate/fragment/heading/snapshot/graph/file_tree selection, and `just lint` gates from the package area root
        - subagent verification result — point-by-point: **all 7 items PASS**
                - private, non-serialized snapshot: `PreparedHeadingSnapshot` is `pub(crate)` (`snapshot.rs:32`); private field `prepared_headings` on `ReferenceGraph` (`types.rs:469`); custom `Debug` emits only `root`+`nodes`; `ReferenceGraphView` JSON serializes only `file`/`source`/`references`/`transclusions`; a unit test asserts `!dbg.contains("prepared_headings")` (`types.rs:1311`)
                - `collect_composed_heading_slugs` uses snapshot on fresh path: `validate.rs:832-848` calls `graph.prepared_headings().slugs_for(path)` first; only on `None` does it fall back to `Markdown::try_from`
                - cross-doc `path#fragment` descendants seeded from snapshot: `validate.rs:836-837` does `cache.insert(heading_slug_key(path), slugs.to_vec())` for snapshot hits; `validate_cross_doc_fragment` → `cached_prepared_heading_slugs` checks cache first (`validate.rs:864-866`), so a descendant target never re-reads disk for its heading set
                - non-descendant fragment targets keep the disk path: code-path inspection confirms — the loop seeds cache only for `graph.nodes()` entries; a path not among them misses cache and falls through to `Markdown::try_from(target_path.as_path())` at `validate.rs:929`, then lazily populates cache via `cached_prepared_heading_slugs`
                - synthetic graphs work via `Default::default()` + disk fallback: empty snapshot ⇒ `slugs_for` returns `None` ⇒ the `else if let Ok(child_md) = Markdown::try_from(path)` branch runs (`validate.rs:839-847`); all synthetic-graph test call sites pass `PreparedHeadingSnapshot::default()` (graph.rs:1330, types.rs:1187/1242/1272, file_tree/model.rs:793/867)
                - paired mechanism test exercises the required scenario: `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings` (`validate.rs:1588-1649`) sets `validate_fragments: true` (1606), roots a fragment link `[link](#child-heading)` to the transcluded child's `# Child Heading` (1597/1599), post-build renames to `# Renamed Heading` (1614), then asserts the fresh seam reports no `MissingFragmentTarget` (1622-1630) while the checked seam returns `ReferenceGraphMismatch`/`Dependency`/`Changed` for `child.md` (1632-1648); `verify_graph_compatibility` runs before `validate_graph_contents` so the mismatch happens before flattening (`validate.rs:372-374`)
                - public signatures, errors, serialized views, CLI wiring unchanged: `from_build` is `pub(crate)`; all public `Markdown` methods and the `ReferenceGraph` accessor surface are unchanged; `ReferenceError`/`ReferenceGraphMismatchError` variants untouched; `ReferenceGraphView` JSON is byte-shape-compatible; CLI `validate` wiring flows through the unchanged public API
        - gate results recorded by the subagent and re-verified directly by the orchestrator:
                - `cargo --color=never nextest run -p darkmatter fresh_seam`: **PASS — 2/2** (`fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph` + `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings`)
                - `cargo --color=never nextest run -p darkmatter -E 'test(/reference|validate|fragment|heading|snapshot|graph|file_tree/)'`: **PASS — 568/568** (20 slow, 5,434 skipped)
                - `just lint` from the `darkmatter/` package area root: **PASS** for `darkmatter`, `darkmatter-cli`, and `dmls` with no warnings (clippy `-D warnings` per `just/devops.just:97`)
        - source edits required: **none** — no gaps, no drifted docs, no missing assertions
        - blockers: none
- work completed for 'high-fragment-validation-rereads-transcluded-children' at 21:54:00

### Successful Completion

The implementation of review cycle 3 has completed successfully in 19 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- **High — Fragment validation rereads transcluded children instead of using the fresh graph snapshot** — **fixed** (verified) per the review's suggested resolution. On entry the worktree already contained the prior implementation of this same finding (commit `f82221e0c feat(darkmatter): add PreparedHeadingSnapshot to ReferenceGraph`); this iteration acted as an independent verification pass rather than a from-scratch redo. Point-by-point verification confirmed: `ReferenceGraph` owns a private `pub(crate) PreparedHeadingSnapshot` (absent from both the custom `Debug` impl and the `ReferenceGraphView` JSON serializer); `collect_composed_heading_slugs` reads descendant headings from the snapshot and seeds the per-run `HeadingSlugCache` via the shared `heading_slug_key` helper, so neither composed-heading collection nor a descendant `path#fragment` target rereads disk on the fresh path; non-descendant cross-document fragment targets keep the existing `Markdown::try_from` disk path; synthetic test graphs still work via `PreparedHeadingSnapshot::default()` and the disk fallback; the paired mechanism test `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings` exercises `validate_fragments: true`, a root fragment link to a transcluded child's original heading, and a post-build heading rename, and asserts the expected fresh/checked divergence; and public signatures, error variants, serialized graph views, and CLI wiring remain unchanged. No performance measurement was required by this finding, so no performance finding was deferred.

The files changed during this implementation:

- `darkmatter/fixes/2026-07-16-redundant-walk/log.md` — this iteration's verification log (no source edits; the reviewed implementation in `snapshot.rs` / `types.rs` / `graph.rs` / `validate.rs` / `mod.rs` / `file_tree/model.rs` was already in place at commit `f82221e0c` and is retained unchanged)

Verification gates (run from the `darkmatter/` package area): focused `fresh_seam` tests PASS (2/2), focused reference/validation/fragment/heading/snapshot/graph/file_tree selection PASS (568/568), `just lint` PASS in `darkmatter`, `darkmatter-cli`, and `dmls` (no warnings, clippy `-D warnings`). No Rust source was modified and nothing was committed.

## Implementation of Review Findings #5

> **started at:** 2026-07-19T22:19:22-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- the review found in 'darkmatter/fixes/2026-07-16-redundant-walk/review-5.md' indicated that the specification is **production ready**!
- the specification file used to define the functional/non-functional target of all this work can be found at 'fixes/2026-07-16-redundant-walk/spec.md'
- while the review found this feature to be production ready, it did have findings worth looking at for follow on work:
        - **Low — The Review 4 cache-first optimization remains unimplemented and untested**
- refer to the review file -- darkmatter/fixes/2026-07-16-redundant-walk/review-5.md -- for more details

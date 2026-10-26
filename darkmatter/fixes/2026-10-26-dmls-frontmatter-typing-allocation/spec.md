---
kind: fix
created: 2026-10-26
status: draft-spec
area: darkmatter
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
human_review: false
related:
    - 2026-10-25-pipeline-work-reduction
    - 2026-10-25-pipeline-improvements
---

# DMLS frontmatter typing clones schema shapes for every entry

A single hover on a 23 KB Claudine document allocates **145 MB**. On a
document ten times larger it allocates **967 MB**, with **418 MB** live at
its peak. A diagnostics pass on the same documents allocates 525 MB and
970 MB. The editor pays this on every hover and every keystroke's
diagnostics, and the cost grows with the document. Smaller documents and
smaller schemas hide it; Claudine's lifecycle schema exposes it.

## How it was found

The `2026-10-25-pipeline-work-reduction` spike measured DMLS's edit loop on a
frozen Claudine document with
`darkmatter/dmls/benches/pipeline_work_a3.rs`. Allocation totals are exact
and were reproduced byte-for-byte.

| Step | Result |
| ---- | ------ |
| Per-provider hover (`A3_PROVIDER_BREAKDOWN=1`) | `FrontmatterProvider::hover`: 144.8 MB (expression), 155.0 MB (verb). Substrate, wiki, and DSL providers: under 1 MB, and all decline |
| CPU sample of that hover (`sample`, release bench) | about 95% of samples under `expression_values → def_at_path_ctx → ShapeMemo::ensure`; most of the rest is `PropertyAtom` clones and their `drop_glue` (about 30% is drop alone) |

## Cause

All of this is in `darkmatter/dmls/src/providers/frontmatter.rs`.

1. **Hover types the whole document to find one value.**
   `expression_value_at` calls `expression_values`, which resolves a
   definition for **every** scalar entry and then keeps the one containing
   the cursor. When that finds nothing (a cursor on a lifecycle verb),
   `stack_expression_hover` calls `expression_values` **again**.
   Diagnostics (`diagnostics/frontmatter.rs`) runs the same walk.
2. **`ShapeMemo` is keyed by the instance path.** Each lifecycle list item
   (`[success, Index(n)]`, `[success, Index(n), then, Index(m)]`, …) is a new
   key. So the memo builds an **owned** `SchemaShape` per item:
   - `descend` clones the selected discriminated arm
     (`selected.clone()`);
   - or it builds a merged copy (`merged_inline_object_shape`).

   The memo prevents repeat work only within one path; it does not share
   shapes between sibling items that resolve to the same schema position.
3. **`def_at_path_ctx` clones each leaf's `PropertyDef`**
   (`Cow::Owned(memo.shape(…)?.properties.get(leaf)?.clone())`), even when
   the caller only checks whether it is an expression type.

The cost is roughly entries × size of the schema shapes reached. Recursive
lifecycle definitions keep those shapes large at every depth.

## Required behavior

### Hover and other point queries resolve only the cursor's path

- Hover locates the entry at the offset and resolves the definition for that
  entry's path only. It never types every entry to find one.
- `expression_hover` and `stack_expression_hover` share one resolution of the
  cursor's entry instead of each walking the document.
- Completion and any other single-position request follow the same rule. The
  plan lists every caller of `expression_values` and records which ones need
  the whole document.

### Shapes are shared, not copied

- Descending into a schema position that is an existing shape (a selected
  discriminated arm, an inline object, an opened recursive definition)
  **borrows** it, or shares it through `Arc`. It never clones it.
- A merged view of several arms is the only shape that must be built. It is
  built **once per schema position and admissible-arm set**, not once per
  instance item. Two list items that select the same arm, or the same merged
  view, get the same shape.
- The memo key separates the two things a step depends on:
  - the schema position;
  - the authored discriminant values that `discriminated_arm_shape` reads at
    that instance path.

  Today both are hidden inside the instance path. Discriminant selection
  still reads the authored siblings at each item, exactly as now.
- `def_at_path_ctx` returns a borrowed definition when its shape is borrowed
  or shared. A clone happens only where a caller keeps an owned value beyond
  the walk.

### Whole-document walks happen once per document version

- Diagnostics needs every entry's definition. The typed entry list
  (`expression_values` and the definitions behind it) is computed once per
  document version and reused by diagnostics, hover, and completion for that
  version. It is held where DMLS keeps other per-version analysis (the
  frontmatter overlay), and it is dropped with that version. It is never kept
  across edits, and it carries no values that depend on file contents or
  runtime state.

### Unchanged

The following stay exactly as today:
- every hover's text, every diagnostic's range, message, and severity, and
  every completion item;
- discriminant-selection rules and merged-arm semantics (the root-union
  policy applied to nested unions);
- `ArrayCrossing` behavior.

## Out of scope

- DMLS resident memory at idle (25–186 MB observed across workspaces). Re-check
  it after this fix lands, before deciding whether it needs work.
- The index store. It opens on the first references, rename, or symbols
  request by design, and is unrelated to this cost.
- Track 5 items X1 (cached alternative matchers) and D5–D11. They are measured
  separately and are small next to this.

## Implementation surfaces

- `darkmatter/dmls/src/providers/frontmatter.rs`: `hover`, `expression_hover`,
  `stack_expression_hover`, `expression_values`, `expression_value_at`,
  `ShapeMemo`, `descend`, `merged_inline_object_shape`, `def_at_path_ctx`
- `darkmatter/dmls/src/diagnostics/frontmatter.rs` (the whole-document walk)
- `darkmatter/dmls/src/overlay/` (where per-version analysis is held)
- `darkmatter/dmls/benches/pipeline_work_a3.rs` (re-measurement)

## Documentation

- `darkmatter/dmls/docs/`: the page describing how frontmatter typing is
  resolved for hover and diagnostics states the per-version lifetime of typed
  entries and that point requests resolve only the cursor's path. Update it if
  it describes the walk today.
- The `darkmatter` skill, if it describes `ShapeMemo`.
- The `ShapeMemo` doc comment, which today says it is keyed per ancestor path
  and links to an old spike. Rewrite it for the new key.

## Acceptance and verification

1. **Work-count regressions (L1, DMLS):** using DMLS's `work_counters` and a
   new count of shapes built by `descend`/merging (the test-only
   `SHAPE_MATERIALIZATIONS` counter already exists; expose or extend it):
   - a hover on one entry of a document with 200 lifecycle items builds at
     most as many shapes as there are distinct schema positions on that
     entry's path, independent of the item count;
   - a diagnostics pass builds each distinct schema position or merged view
     at most once, so doubling identical lifecycle items does not double the
     count;
   - a hover after diagnostics on the same document version runs no
     whole-document walk.
2. **Allocation result:** the A3 bench reports, in the implementation log, the
   hover and diagnostics bytes at `a3x1` and `a3x10` before and after. Target:
   a hover allocates under **1 MB** at both sizes, and diagnostics grow
   linearly with the document. Allocation totals are deterministic, so this
   can be measured on any host.
3. **Behavior unchanged:** the existing DMLS L1 and L2 suites pass unchanged,
   including hover snapshots, discriminant selection, merged-arm completion,
   and lifecycle diagnostics.
4. `just test`, `just test-l2`, and `just lint` pass in `darkmatter/`. No test
   exceeds five seconds.

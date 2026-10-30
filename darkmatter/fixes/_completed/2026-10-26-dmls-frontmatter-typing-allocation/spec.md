---
kind: fix
created: 2026-10-26
status: finalized-spec
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
    next_spec: string -> the `{date}-{name}` of the spec expected to follow this one; absent when none is planned
next_spec: 2026-10-25-fix-now
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-26
review_iterations: 1
completed: true
human_review: false
needs_rulings: false
related:
    - 2026-10-25-pipeline-work-reduction
    - 2026-10-25-pipeline-improvements
    - 2026-10-25-shared-acyclic-definitions
implemented: true
implemented_by: "claude/opus"
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
`darkmatter/dmls/benches/pipeline_work_a3.rs`. Allocation totals were reproduced
byte-for-byte in that measurement's build and environment; compare before and
after with the same build settings on one host rather than treating these byte
counts as portable constants.

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
   Diagnostics (`diagnostics/frontmatter.rs`) runs the same walk. The stack
   fallback also collects expressions across the whole document through
   `stack_expressions`, even after the typing walk is removed.
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
4. **Root and array-item definitions also allocate.** `known_shape` builds an
   owned effective root shape on each call, and `array_item_def` constructs
   owned item definitions. Sharing nested shapes alone leaves these copies
   on the same request paths.

The cost is roughly entries × size of the schema shapes reached. Recursive
lifecycle definitions keep those shapes large at every depth.

## Required behavior

### Hover and other point queries resolve only the cursor's path

- Hover locates the entry at the offset and resolves the definition for that
  entry's path only. It never types every entry to find one.
- `expression_hover` and `stack_expression_hover` share one resolution of the
  cursor's entry instead of each walking the document.
- The lifecycle fallback inspects interpolation spans and guards only in the
  cursor's scalar. It must not call the whole-document `stack_expressions`
  collector merely to find one span. Keep the existing expression parsing,
  scalar projection, event scope, and hover precedence.
- Completion and any other single-position request follow the same rule. The
  plan lists callers of `expression_values`, `known_shape`, and the shared
  schema-path resolver, distinguishing point requests from document-wide
  consumers such as diagnostics and document links. Resolving additional
  properties named by the hovered expression is allowed; typing unrelated
  entries to locate the cursor is not. Existing source-position searches and
  completion candidate enumeration need not become constant-time.

### Shapes are shared, not copied

- Descending into a schema position that is an existing shape (a selected
  discriminated arm, an inline object, an opened recursive definition)
  **borrows** it, or shares it through `Arc`. It never clones it.
- A merged view of several arms is the only shape that must be built. It is
  built **once per schema position and admissible-arm set**, not once per
  instance item. Two list items that select the same arm, or the same merged
  view, get the same shape.
- Assemble the effective root shape at most once per analysis snapshot and
  share it across providers. Preserve baseline, extension, trigger, and
  document-schema precedence. Cache synthesized array-item definitions by
  their source definition and crossing mode, rather than cloning them for
  each sequence item. Root assembly and synthesized definitions are explicit
  exceptions to the rule against constructing existing nested shapes.
- The memo key separates the two things a step depends on:
  - the schema position;
  - the authored discriminant values that `discriminated_arm_shape` reads at
    that instance path.

  Today both are hidden inside the instance path. Discriminant selection
  still reads the authored siblings at each item, exactly as now. Keep that
  instance lookup separate from the shared shape cache: after selection,
  key the shared result by stable schema-node identity, crossing mode, and
  the selected arm or ordered admissible-arm set. Recursive references
  return to the same definition identity, not a new identity for each depth
  or array index. Missing and invalid selections retain the current merged
  fallback; strings, numbers, booleans, and null must remain distinct.
- `def_at_path_ctx` returns a borrowed definition when its shape is borrowed
  or shared. A clone happens only where a caller keeps an owned value beyond
  the walk. Handles that retain a shared owner and identify a property within
  it are also acceptable; do not require self-referential Rust structs or
  unsafe lifetime extensions to satisfy a borrowed return signature.

### Whole-document typing happens lazily once per analysis snapshot

- Diagnostics needs every entry's definition. Compute the typed scalar-entry
  inventory once, on its first document-wide consumer, and reuse it for later
  consumers of that snapshot. Keep expression values in document order with
  their existing decoded text and authored-range projections. Cache
  non-expression and unresolved classifications too, so a miss does not
  trigger another walk.
- Hover and completion may reuse classifications already available, but never
  initialize the document-wide inventory. Before diagnostics, they resolve
  just their requested paths through the same shared schema state.
- Keep reusable analysis in the overlay cache and give each request overlay
  a shared handle to it. A request constructs a fresh `DocumentOverlay`
  today; a field initialized anew on each such wrapper would not provide
  cross-request reuse. Store entry indices and shared owners rather than
  references into another field of the same allocation.
- A snapshot includes the document URI, buffer revision and text, current
  tree versus last-good-tree state, effective schema and named definitions,
  configuration, trigger registry, and captured resolution context. Follow
  existing schema-dependency invalidation, including imported-schema changes
  without a document edit. Reuse requires all these inputs to match. Do not
  retain current cached analysis across an edit or close/reopen; outstanding
  requests may retain their own old snapshot until they finish. Keep no
  unbounded history or process-global schema-shape cache.
- Analysis depends on authored buffer contents and prepared schema inputs;
  it stores no evaluated expressions or runtime results. Its construction
  does no additional file reads, network requests, shell execution, document
  mutation, or context discovery. Preserve existing last-good-tree behavior
  on malformed YAML and never pair cached decoded text or spans with a
  different buffer.

### Unchanged

The following stay exactly as today:
- every hover's text, every diagnostic's range, message, and severity, and
  every completion item;
- discriminant-selection rules and merged-arm semantics (the root-union
  policy applied to nested unions);
- `ArrayCrossing` behavior.
- parsing recovery and last-good-tree behavior, scalar decoding and range
  projection, baseline precedence, and named-type resolution;
- passive editor analysis: no evaluation or new I/O in the typing cache.

## Design Decisions

### Separate point lookup from lazy document-wide typing

- **Decision:** A point request resolves its cursor entry once and examines
  only that scalar for lifecycle expressions. Diagnostics initializes a
  shared typed-entry inventory lazily; point requests can consume an existing
  inventory but cannot create it.
- **Alternatives considered:** Build the entire inventory on the first hover;
  this leaves the first hover proportional to document size.
- **Alternatives considered:** Recompute a separate inventory for every
  request; this repeats the diagnostics cost.
- **Why:** In the `dmls` package, both
  [`expression_value_at`](../../dmls/src/providers/frontmatter.rs) and the lifecycle
  fallback currently enumerate unrelated values. Separating the two access
  patterns satisfies both cursor-only typing and cross-request reuse.
- **Downsides:** The implementation needs a point API and a document-wide API
  over the same classifier, with tests ensuring they agree.

### Share schema nodes by identity after instance-specific selection

- **Decision:** Use snapshot-local schema-node identities and shared-owner
  handles for existing shapes. Cache merged shapes by source identity,
  crossing mode, and ordered admissible arms; an instance path locates authored
  discriminants but does not identify the reusable shape. Preserve the shared
  library's discriminant selector and merge ordering.
- **Alternatives considered:** Remove array indices from instance-path keys;
  siblings with different selected arms could incorrectly share results.
- **Alternatives considered:** Key by cloned or serialized shape contents;
  constructing the keys would repeat the large copies being removed.
- **Why:** In the `dmls` package,
  [`ShapeMemo`](../../dmls/src/providers/frontmatter.rs) currently owns one shape
  per instance path. Recursive schema-node identity allows repeated siblings
  and recursive depths to share without mixing their authored values.
- **Downsides:** Handles must retain their schema owner, and identity must
  distinguish source nodes with different property metadata even when their
  names match. Identities are not persistent or portable between snapshots.

### Include root assembly and synthesized item definitions

- **Decision:** Share the effective root once per snapshot and synthesize an
  array-item definition at most once per source definition and crossing mode.
  Borrow existing nested definitions or return owner-retaining handles;
  owned copies are reserved for synthesis or consumers requiring ownership.
- **Alternatives considered:** Change nested descent alone; repeated root and
  leaf copies would remain on hover, completion, and navigation paths.
- **Why:** In the `dmls` package,
  [`known_shape` and `array_item_def`](../../dmls/src/providers/frontmatter.rs)
  produce owned values today. They must participate in the same sharing
  strategy to meet the allocation goal without moving the cost elsewhere.
- **Downsides:** Consumers that currently accept owned shapes or `Cow` values
  may need internal signature changes. First root assembly and genuine merges
  still allocate, and their precedence and metadata must remain unchanged.

### Own reusable analysis in the cross-request overlay cache

- **Decision:** Keep one current shared analysis snapshot per document in the
  overlay cache. Invalidate on buffer revision or any prepared semantic-input
  change, including schema-file changes, and remove it on document close.
  Preserve current stale-tree behavior with separately matched text and spans.
- **Alternatives considered:** Cache by document version alone; external
  schemas and configuration can change without a buffer edit.
- **Alternatives considered:** Store a lazy cell only on each request wrapper;
  wrappers are rebuilt and would repeat the work across requests.
- **Why:** The `dmls` package's
  [`OverlayState`](../../dmls/src/overlay/mod.rs) already owns cross-request schema
  caches and dependency checks. Reusing their invalidation boundaries avoids
  a second filesystem watcher or a process-global cache.
- **Downsides:** Some memory stays live until invalidation or close, and an
  outstanding request can temporarily retain a previous snapshot. Cache
  construction must avoid recursive initialization or holding mutable cache
  access while returning references to its contents.

### Use work counters for regression tests and the existing bench for bytes

- **Decision:** Count actual root/merge constructions, synthesized item
  definitions, entry classifications, and full typing walks separately from
  descent attempts. Re-run the existing allocation benchmark on one host at
  its existing sizes and settings; add no broader performance spike.
- **Alternatives considered:** Keep counting every descent as a materialized
  shape; a successful borrowed lookup would misleadingly count as a copy.
- **Alternatives considered:** Add cross-host measurements or timing thresholds;
  neither is needed to verify this allocation mechanism.
- **Why:** The current counter increments before descent even knows whether
  it can return a shape. The supplied measurements already identify the
  problem; deterministic work assertions explain whether the fix removes it.
- **Downsides:** Allocation results describe one build and host. Counters prove
  avoided work, not a universal latency bound.

### Preserve behavior through internal API changes

- **Decision:** Keep editor responses and passive analysis unchanged, including
  malformed-buffer recovery, mixed expression unions, array crossing, and
  discriminant fallback. Internal ownership and caching APIs may change.
- **Alternatives considered:** Simplify selection or suppress stale-buffer
  results to make caching easier; both would change editor behavior.
- **Why:** This fix promises reduced allocation with identical editor results;
  existing library parsing and selection remain the semantic authorities.
- **Downsides:** Tests must cover invalid and incomplete input as well as the
  repeated valid lifecycle items used to demonstrate allocation reduction.

## Interaction with shared acyclic definitions

`2026-10-25-shared-acyclic-definitions` stores more named types as shared
definitions, so the shapes this walk visits contain more `$ref`s. Each
reference is a schema position. Under this fix's memo key it is resolved
once and borrowed by every item that reaches it, so the two changes should
compound rather than conflict. This fix lands first; it is the shared step
that precedes both `2026-10-25-fix-now` and
`2026-10-26-sequence-quadratic-frontmatter`. Record this fix's
`cargo bench -p dmls --bench pipeline_work_a3` hover and diagnostics bytes in
its implementation log. `2026-10-25-shared-acyclic-definitions`, which lands
later, re-runs the same bench against them; a rise means it undid part of
this fix.

## Out of scope

- DMLS resident memory at idle (25–186 MB observed across workspaces). Re-check
  it after this fix lands, before deciding whether it needs work.
- The index store. It opens on the first references, rename, or symbols
  request by design, and is unrelated to this cost.
- Broader pipeline optimizations measured by
  `2026-10-25-pipeline-work-reduction`, including cached alternative matchers.
  This exclusion does not remove root assembly, array-item synthesis, or
  lifecycle point lookup required above.

## Implementation surfaces

- `darkmatter/dmls/src/providers/frontmatter.rs`: `hover`, `expression_hover`,
  `stack_expression_hover`, `expression_values`, `expression_value_at`,
  `ShapeMemo`, `descend`, `merged_inline_object_shape`, `def_at_path_ctx`
- `darkmatter/dmls/src/diagnostics/frontmatter.rs` (the whole-document walk)
- `darkmatter/dmls/src/overlay/` (where per-version analysis is held)
- `darkmatter/dmls/src/overlay/event_views.rs` (cursor-local lifecycle
  expression extraction sharing semantics with document-wide extraction)
- `darkmatter/dmls/src/work_counters.rs` (work-count regression instrumentation)
- `darkmatter/dmls/benches/pipeline_work_a3.rs` (re-measurement)

## Documentation

- `darkmatter/dmls/docs/hover.md`, `diagnostics.md`, and `autocomplete.md`:
  explain cursor-only typing, lazy shared inventory, and invalidation on edits
  or schema-input changes. Include a compact example showing two list items
  with different discriminants sharing schema storage without sharing their
  selected results. Document the current behavior when implementation lands;
  these pages must not refer readers to this spec.
- `.claude/skills/darkmatter/dmls.md`: record the new cache ownership and
  lifetime so later provider work uses it consistently.
- The `ShapeMemo` doc comment, which today says it is keyed per ancestor path
  and links to an old spike. Rewrite it for the new key.

## Acceptance and verification

1. **Work-count regressions (L1, DMLS):** using DMLS's `work_counters` and
   counts of actual shape constructions, typed entries, synthesized item
   definitions, and full typing walks (the test-only
   `SHAPE_MATERIALIZATIONS` counter currently counts descent attempts; correct
   or replace that meaning):
   - a hover on one entry of a document with 200 lifecycle items builds at
     most as many shapes as there are distinct schema positions on that
     entry's path, independent of the item count, plus at most one effective
     root assembly. Prove this before diagnostics has built an inventory,
     for expression and lifecycle-verb hovers and a cursor outside typed
     scalar values; none starts a full typing walk;
   - a diagnostics pass builds each distinct schema position or merged view
     at most once, so doubling identical lifecycle items does not double the
     count. Count a schema position by node identity and admissible arms,
     including recursive reuse, rather than by an unrolled instance path;
   - a hover after diagnostics on the same document version runs no
     whole-document typing or stack-expression walk. Repeated diagnostics
     reuses the typed inventory, and completion before diagnostics also
     performs no whole-document typing. This does not forbid other existing
     diagnostic passes from walking the document;
   - an edit, imported-schema change with unchanged buffer version,
     configuration/registry/context change, and close/reopen each prevents
     stale reuse. Cover malformed YAML followed by repair without attaching
     an old projection to new text.
2. **Allocation result:** the A3 bench reports, in the implementation log, the
   expression-hover, verb-hover, and diagnostics bytes at `a3x1` and `a3x10`
   before and after, retaining its existing cold/warm cases and repetitions.
   Use one host, the same build settings, and the frozen input. Target:
   each measured hover allocates under **1,000,000 bytes** at both sizes,
   and diagnostics allocation grows linearly with entries for a fixed schema.
   The existing benchmark runs diagnostics before hover: report that order
   explicitly, and use the cold point-request work tests above to prove that
   meeting the byte target does not depend on eagerly typing the document.
   Explain residual diagnostics growth from required per-entry outputs;
   do not infer arbitrary-schema scaling from two fixed-schema samples.
3. **Behavior unchanged:** the existing DMLS L1 and L2 suites pass unchanged,
   including hover snapshots, discriminant selection, merged-arm completion,
   and lifecycle diagnostics. Add focused L1 coverage of sibling items choosing
   different arms; missing, unknown, duplicate, or conflicting discriminants;
   typed literal distinctions; recursive references; explicit and transparent
   array crossing; and plain, quoted, block, and tagged expression values.
   Use existing fixtures and assertions where they already prove these rules.
   Preserve completion order and edits, hover ranges, and diagnostic codes
   as well as the text and severity listed above.
4. `just test`, `just test-l2`, and `just lint` pass in `darkmatter/`. No
   L1 test exceeds five seconds. Run tests through nextest-backed recipes and
   use the shared test toolkit. L2 tests must not activate terminal or browser
   windows. This is an implementation gate; the spec review itself does not
   require running code suites.

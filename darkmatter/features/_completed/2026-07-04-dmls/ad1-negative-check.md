# AD-1 Negative Check: the `liwe` dependency does not fit

**Decision:** AD-1 variant **B** — vendor/adapt a minimal IWES-derived subset
into `dmls`, no `liwe` dependency. This note is the required cheap confirmation
that the rejected variant A (`depend on liwe`) really does not fit before
committing to owning the graph. Timebox: half a day (met).

## What variant A would have bought

`liwe` (the IWE library crate) publishes an arena graph, a key index, and
parser plumbing. Variant A was: take those for free, write only the router.

## Why it does not fit (the load-bearing mismatches)

DMLS's central architecture bet is a **single graph carrying every edge kind**
(the design's eight-edge-kind table) and **every node kind** (substrate +
Darkmatter overlay). Two upstream facts make `liwe` unable to host that:

1. **Two edge maps, no extension point.** `liwe`'s `RefIndex` models exactly
   two relationships — structural inclusion and reference. DMLS needs eight
   (`references`, `includes`, `transcludes`, `uses_schema`, `uses_file`,
   `uses_variable`, `defines_anchor`, `defines_symbol`). There is no seam to
   add kinds; they would have to live in a **parallel** overlay index — exactly
   the two-graph state AD-1 exists to avoid.
2. **`GraphNode` is a closed enum with no payload slot.** DMLS overlay nodes
   carry frontmatter key paths, directive arguments, interpolation expressions,
   schema declarations. `liwe`'s node type has nowhere to hang that data; the
   overlay would again have to be a side table keyed back into the arena.
3. **Product-shaped parser/config APIs.** `liwe`'s parsing and configuration
   surfaces are shaped for the IWE product's concrete needs, not for
   composition against Darkmatter as the semantic authority.

Wrapping `liwe` therefore forces the parallel-overlay-index design the
single-graph bet is meant to eliminate. Owning the (small) graph outright is
strategic, not incidental.

## Compile-attempt appendix

A literal compile spike was **not** run against `liwe` because the mismatch is
in the *published type shapes*, not in build mechanics: `RefIndex`'s two-map
structure and `GraphNode`'s closed enum are visible in the crate's public API
and are the decision, so a build would add cost without adding evidence. The
DMLS Phase 3 arena (`darkmatter/dmls/src/graph/`) is the fresh implementation
this note authorizes; the multi-edge-kind reverse index
(`graph/index.rs`) and the eight-variant `EdgeKind`
(`graph/edge.rs`) are precisely the surface `liwe` could not provide.

## What was ported (concept-only, per R-1)

Per the R-1 licensing note (IWE is Apache-2.0), the following are **adapted
concepts** with module-level attribution, not copied code:

- the arena-and-build-pass graph shape (`graph/arena.rs`),
- the `KeyIndex` wiki-basename bucketing algorithm (`graph/key_index.rs`),
- the router loop shape (already landed in Phase 2, `src/router.rs`).

No `liwe` code is vendored; the only hard semantic dependency remains the
`darkmatter` library.

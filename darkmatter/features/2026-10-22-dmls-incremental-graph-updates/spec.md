---
area: darkmatter
status: draft-spec
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
reviewed: false
review_iterations: 0
clarified: false
created: 2026-10-22
owner: Ken Snyder <ken@ken.net>
packages:
    - dmls
related:
    - 2026-10-22-dmls-watched-file-flood
human_review: true
needs_rulings: true
---

# DMLS: update the workspace graph incrementally

## Context

After `2026-10-22-dmls-watched-file-flood`, a watched-file notification
rebuilds the workspace graph at most once, and only when a change can
alter it. When it does, `WorkspaceIndex::rebuild` still rebuilds the
whole `WorkspaceGraph` (nodes, edges, reverse and key indexes, wiki
slots, portability collisions) over every listed and parsed document.
Two triggers remain:

1. **A Markdown file joins (or leaves) the file list.** Document ids are
   assigned in path-sorted order, and every index is keyed by them, so
   one insertion renumbers what follows it.
2. **An unlisted file is created where a broken link waits for it**
   (`WorkspaceGraph::awaits`). It does not become a graph document. Only
   the edges waiting for that path change, from unresolved to a file.

Other callers rebuild too: edits to open buffers, re-parses of stale
documents, wiki-root, context, and configuration changes. Each one is a
full rebuild today.

Incremental updates are not free. Case 2 needs a copy-on-write patch of
some edges and their reverse-index entries. Case 1 needs document ids
that do not depend on sort order. This spec decides, from measurement,
whether either is worth building.

## Phase 1: measure (required before any design)

Measure a **release** build, on this repository's real workspace (about
6,000 Markdown files), and on a synthetic workspace an order of
magnitude larger.

- **M1. Cost of one rebuild.** Time `WorkspaceIndex::rebuild`, split
  into its stages: `WikiUniverse::new`, `assemble_listed`, and
  `finalize` (reverse index, key index, portability). The bench harness
  already times the assemble and finalize stages separately (see
  `tests/l1/bench_index.rs`).
- **M2. How often each trigger fires in real use.** Count rebuilds by
  cause over representative sessions: editing in Zed, an agent writing
  specs and code, and a `cargo build` or `just test` writing under
  `target/`. Use the generation counter, with a temporary per-cause tag
  if needed. Record notifications per minute and rebuilds per minute
  for each cause.
- **M3. Where the time goes in a rebuild.** Profile it (for example
  with `sample` or `cargo flamegraph`) to see whether a cheaper full
  rebuild would deliver most of the win without incremental state.
  Candidates are canonicalizing logical paths once per document instead
  of once per rebuild, and reusing the `WikiUniverse` when the listed
  set has not changed.

### Decision rule

Phase 1 ends with a written ruling in this spec:

- If **rebuild cost × rebuild rate** keeps DMLS under about 1% of a core
  during M2's busiest session, and one rebuild in release on this
  repository costs under about 50 ms (no visible editor latency), do not
  build incremental updates. Record the numbers, and close this spec as
  not needed.
- If a cheaper full rebuild (M3) brings the numbers under those bounds,
  do that instead, as its own phase.
- Otherwise, design incremental updates for the trigger that dominates
  M2, case 2 first if it qualifies, since it is the smaller change.

The thresholds are a starting point. Changing them is a ruling for the
owner, recorded here.

## Phase 2 (only if Phase 1 says so)

To be written from Phase 1's findings. Whatever it builds, a graph
updated incrementally must be indistinguishable from a full rebuild. The
existing `wiki_universe_equivalence` and `index_store_oracle` style of
oracle test, which compares the incremental result with a fresh full
rebuild over the same inputs, is the acceptance bar.

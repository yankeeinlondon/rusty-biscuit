# Mermaid gitGraph tag spacing

`biscuit-visualized` renders a `gitGraph` with [Merman](./merman.md), which
draws merges as Mermaid.js does. It adds one thing of its own: **tags never
overlap**. When two tags on different commits would collide, the commits are
spaced just far enough apart to leave a one-em gap between them, and no
further.

This applies to every gitGraph and every caller: `MermaidDiagram` directly,
the `GitGraph` component and `bt git-graph`, and Darkmatter's Mermaid blocks.
There is nothing to opt into and no new option.

## Merges

Merman follows Mermaid.js, so a merge needs no correction:

- A labeled merge keeps both parents. In `merge feat id: "M" tag: "v1"`,
  `M`'s parents are the current branch's head and `feat`'s head, and `M`
  keeps its ID, its tag, and its lane.
- A merge of a branch that has no commit of its own renders: its second
  parent is the commit the branch was created at.

```text
gitGraph
    commit id: "A"
    branch side
    checkout main
    commit id: "B"
    merge side id: "M"
```

Here `M`'s parents are `B` and `A`.

A merge **into** a branch that has no commit yet is still an error, in Merman
and in Mermaid.js alike; the `GitGraph` component never emits one.

## Tag spacing

Merman places each tag above its commit with no collision avoidance and
spaces commits a fixed 50 px apart (`commitSpacing`'s default). A tag wider
than that covers its neighbor's, as `main` and `origin/main` one commit apart
do. So, before rendering, a left-to-right gitGraph with tags is laid out once
at its current spacing, its tags are checked in pairs, and when a pair
collides the smallest spacing that clears every pair is written into the
source:

```text
gitGraph
    commit id: "d1"
    commit id: "d2" tag: "main"
    commit id: "d3" tag: "origin/main"
%%{init: {"gitGraph": {"commitSpacing": 74.5}}}%%
```

(The value is illustrative; it depends on the measured tag widths.) The
directive is appended as the last line. Merman reads directives anywhere in
the source and merges them over frontmatter and earlier directives, so it
changes no other setting.

```mermaid
flowchart TD
    src["gitGraph source"] --> tags{"Contains tag: ?"}
    tags -- no --> keep["Render unchanged"]
    tags -- yes --> layout["One Merman layout pass"]
    layout --> author{"Author set commitSpacing?"}
    author -- yes --> keep
    author -- no --> lr{"Left-to-right?"}
    lr -- no --> keep
    lr -- yes --> pairs{"Any colliding pair<br/>closer than 1 em?"}
    pairs -- no --> keep
    pairs -- yes --> write["Write the smallest clearing<br/>commitSpacing as %%{init}%%"]
    write --> render["Render"]
```

The rules:

- **Which pairs count.** Two tags on **different** commits whose boxes share
  rows (boxes that only touch do not count). Tags stacked on one commit never
  widen the spacing, and neither do tags on lanes far enough apart that their
  rows do not overlap, nor two commits drawn in the same column
  (`parallelCommits`), which no spacing can separate.
- **The gap.** Each counting pair needs one em between the earlier tag's
  right edge and the later tag's left edge. An em is the diagram's
  `themeVariables.fontSize` (16 px in every built-in theme), so a
  `%%{init}%%` that raises the font size widens the gap. The em is a
  readability policy, not a measured margin.
- **The smallest spacing.** Tag boxes move with their commits, so a pair `n`
  columns apart that is `shortfall` px short of the gap needs
  `spacing + shortfall / n`. The diagram gets the largest of these, rounded
  up to 0.01 px, so the tightest pair ends up one em apart (to that rounding)
  and no pair is closer. One layout pass is enough: no second pass is needed
  to verify it.
- **Tag widths.** Each tag is measured with the same measurer and font chain
  Merman lays the diagram out with, at the tag font size
  (`themeVariables.tagLabelFontSize`, 10 px by default), and its box follows
  Merman's own tag-box formula. A theme font override therefore changes the
  gap, not the tag widths.
- **The author wins.** A source that already sets `gitGraph.commitSpacing`,
  in a directive or in frontmatter, is never changed, even when its value
  lets tags overlap.

Two examples in the default theme:

| Graph | Colliding pair | Spacing |
| ----- | -------------- | ------- |
| `main` and `origin/main` stacked on one commit, beside a long history | none (same commit) | 50, the default |
| `main` on one commit, `origin/main` on the next | one column apart, overlapping | 50 + the overlap + 16 |

So a graph with one long, isolated label keeps Merman's density. Only a real
collision widens it, and only as far as that collision needs.

Left alone, laid out with Merman's spacing:

- diagrams that are not gitGraphs;
- gitGraphs with no tags (no layout pass is made for them);
- top-to-bottom and bottom-to-top gitGraphs (`gitGraph TB:` or `BT:`), whose
  tags Merman rotates. **Their tags are not spaced and can still overlap.**

The spacing is one value for the whole diagram, so one colliding pair widens
every column. Callers that fit a width should expect that a colliding graph
may need shrinking. The `GitGraph` component trims commits first and then
shrinks, and never shortens a label.

## Cost and cache identity

The layout pass runs before the [cache check](./merman.md#cache-identity),
under its own [time budget](./merman.md#time-budget), so a tagged gitGraph
costs one Merman layout even when its image is cached. The cache key covers
the source as rewritten, so the written spacing is part of the key. A change
to how spacing is computed changes output for the same input, so it bumps the
`+bv<n>` suffix of the backend id (`cache::file_cache::MERMAID_BACKEND`).

`commitSpacing` is not a setting stock Merman reads. `biscuit-visualized`
pins Merman to a fork that adds it (see [Dependencies](./dependencies.md)).

## Checking a layout

`MermaidDiagram::gitgraph_geometry()` returns the laid-out commits (ID, lane,
parents, placement index and x, tag text and bounds), the commit step, the
tag gap, and the size, built from Merman's layout data for the source as
rewritten, so it is the layout that renders. `tag_boxes()` and
`tag_overlaps()` answer "do any tags overlap?" without rasterizing.
`biscuit_visualized::mermaid::default_gitgraph_commit_step()` gives Merman's
default spacing (50) to compare against. Both are `#[doc(hidden)]` test
support, not a stable API. The `GitGraph` component's tests and worktree tests
use them to prove placement on real layouts, because Mermaid text alone
cannot show an overlap.

## Tests

- `src/src/tests/gitgraph_tests.rs` (L1): both parents of a labeled merge, a
  merge of a branch without its own commit, and real layouts: long labels on
  neighboring commits, `main`/`origin/main` one commit apart, a stack of
  three tags, and cross-lane tags, each checked for no overlap and for the
  tightest pair being one em apart; the default spacing for stacked tags,
  for lanes apart without shared rows, for a real `wt list` graph, for
  untagged graphs, and for vertical graphs; a larger `%%{init}%%` font
  widening only the gap; and that the measured size is the spaced layout.
- `src/src/tests/gitgraph_layout_tests.rs` (L1): reading Merman's layout
  data strictly (missing, mistyped, and duplicate fields, trailing content),
  tag bounds matching the drawn tags in every direction, and the spacing
  separating adjacent tags.

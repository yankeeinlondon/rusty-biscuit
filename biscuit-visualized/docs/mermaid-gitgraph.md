# Mermaid gitGraph Corrections

`biscuit-visualized` renders Mermaid through `mermaid-rs-renderer` 0.3.1. For a `gitGraph` it corrects two renderer behaviors before drawing:

- **Merge repair:** a labeled `merge` gets back the second parent the parser loses.
- **Tag spacing:** commits are spaced so no two tags overlap.

Both happen for every gitGraph and every caller: `MermaidDiagram` directly, biscuit-terminal's `GitGraph` and `bt git-graph`, and Darkmatter's Mermaid blocks. There is nothing to opt into and no new option.

## Where the corrections run

```mermaid
flowchart LR
    text[Mermaid text] --> parse[parse_mermaid]
    parse --> repair[repair merges]
    repair --> theme[resolve theme<br/>and init overrides]
    theme --> layout[lay out<br/>and space tags]
    layout --> size[natural_size]
    layout --> svg[render_svg]
    layout --> geometry[gitgraph_geometry]
```

All three outputs go through the one private `MermaidDiagram::compute_layout`. So the size `biscuit-terminal` measures to fit a terminal is always the size of the image it then draws.

## Merge repair

`mermaid-rs-renderer` 0.3.1 reads everything after `merge` as the source branch's name, attributes included. For `merge feat id: "M" tag: "v1"` it looks for a branch called `feat id: "M" tag: "v1"`, finds none, and gives the merge commit only its first parent. The merge edge disappears from the picture.

After parsing, each merge commit with exactly one parent gets its source lane back:

1. The source is recovered from the parser's commit message, `merged branch {suffix} into {lane}`.
2. A declared lane matches when the suffix **is** its name, or is its name followed by whitespace and `id:`, `tag:`, or `type:`. Exactly one lane must match.
3. The second parent is that lane's last commit before the merge.

For example:

```text
gitGraph
    commit id: "A"
    branch feat
    checkout feat
    commit id: "B"
    checkout main
    merge feat id: "M" tag: "v1"
```

Parsed as-is, `M`'s parents are `[A]`. After the repair they are `[A, B]`, and `M` keeps its ID, its tag, and its lane.

| Situation | Result |
|---|---|
| The merge already has two or more parents (an unlabeled `merge feat`, or a future parser that handles attributes) | Left alone: the repair is a no-op |
| Its single parent already is the source lane's tip (a merge into a lane with no commit of its own yet) | Left alone |
| `merge feat x id: "M"` with lanes `feat` and `feat x` | Resolves to `feat x`: `feat` is followed by `x`, not an attribute key |
| No declared lane matches | `MermaidError::RenderFailed` |
| Two lanes match (lanes `x` and `x tag: "t"` with `merge x tag: "t" id: "M"`) | `MermaidError::RenderFailed` |
| The source lane has no commit before the merge | `MermaidError::RenderFailed`, as mermaid.js refuses to merge an empty branch |

A merge is never drawn with a guessed source. Measurement, geometry, and rendering all report the error.

One parser behavior the repair cannot fix: a **labeled** merge into a lane that has no commit yet is dropped by the parser entirely, so it never reaches the repair. Emit a merge only where its destination lane already has a commit.

A unit test pins the parser's message format. If a dependency update changes it, that test fails instead of the repair silently stopping.

## Tag spacing

The renderer places each tag above its commit with no collision avoidance and spaces commits a fixed `commit_step` apart (34 units in the default theme). A tag wider than that covers its neighbor's, as `main` and `origin/main` one commit apart do.

A left-to-right (`LR`) or right-to-left (`RL`) gitGraph with at least one tag is laid out twice. The second pass sets:

```text
commit_step = max(default commit_step, widest tag width + 1 em)
```

The em is the resolved theme's font size, including `%%{init}%%` overrides. The one-em gap (`TAG_GAP_EM`) is a readability policy, not a measured margin. Tags stay on their original commits and keep their full text.

Everything else is laid out once, unchanged:

- diagrams that are not gitGraphs;
- gitGraphs with no tags;
- vertical gitGraphs (`TB`, `BT`). 0.3.1 honors only a separate `TB` or `direction TB` line, not `gitGraph TB:`;
- gitGraphs with any rotated tag. **Rotated tags are not spaced and can still overlap.**

The step is one value for the whole diagram, taken from the widest tag. A diagram with one long label is therefore much wider than before: up to about 3.8× in the measured cases. Callers that fit a width should expect to shrink the image. biscuit-terminal's `GitGraph` trims commits first and then shrinks, and never shortens a label.

## Cache identity

Neither correction adds a rendering input: each is derived from the Mermaid text and the resolved theme, which the artifact cache key already covers. Because both change the output for the same input, the backend identifier in the key moved from `mermaid-rs-renderer@0.2.x+bv1` to `mermaid-rs-renderer@0.3.x+bv2` (`cache::file_cache::MERMAID_BACKEND`). Artifacts cached before the change are never served again. Bump the `+bv<n>` suffix whenever this layer changes the output.

## Checking a layout

`MermaidDiagram::gitgraph_geometry()` returns the laid-out commits (ID, lane, repaired parents, tag text and bounds), the commit step, and the size, from the same layout that renders. `tag_boxes()` and `tag_overlaps()` answer "do any tags overlap?" without rasterizing. It is `#[doc(hidden)]` test support, not a stable API. biscuit-terminal and worktree tests use it to prove placement on real layouts, because Mermaid text alone cannot show an overlap.

## Tests

`src/src/tests/gitgraph_tests.rs` (L1, all features): the message-format pin, every row of the repair table, a nested-lane merge, merges into non-root lanes, long labels on neighboring commits, `main`/`origin/main` one commit apart, a stack of three tags, cross-lane tags (each with a control that overlaps without spacing), the unchanged single pass for untagged and vertical graphs, and that the measured size is the spaced layout. `cache_tests::cache_key_different_mermaid_backend` pins the backend identifier.

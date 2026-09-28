# GitGraph

Draws git branch topology as a Mermaid `gitGraph`, rendered through [MermaidDiagram](./mermaid_diagram.md). The caller gathers the git facts and hands them over typed; the component runs no git commands. It decides which refs get lanes and which become tags, orders the lanes, draws merges, marks elided commits, works around `mermaid-rs-renderer`'s parser, and fits the graph to the terminal by trimming commits and lanes rather than shrinking it. It never draws something at a commit other than the one it was given: what it cannot draw is reported as incomplete history.

Requires the `image` feature. `wt list` is its main caller; [the `wt list` graph](../../../worktree/docs/git-graph.md) shows how a caller gathers the facts.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

let graph = GitGraph::new(
    "main",
    vec![LaneEntry::Commit(base_sha.clone()), LaneEntry::Elided(4), LaneEntry::Commit(origin_sha.clone())],
)
.with_ref("main", base_sha.clone())
.with_ref("origin/main", origin_sha.clone())
.with_line(
    GraphLine::new("feat/theme")
        .forked_at(base_sha.clone())
        .with_tip(theme_sha.clone())
        .with_entries(vec![LaneEntry::Commit(theme_sha)])
        .with_created_at(created_unix_seconds),
)
.with_line(
    // Merged by the commit `origin/main` points at.
    GraphLine::new("fix/typo")
        .forked_at(base_sha)
        .with_tip(typo_sha.clone())
        .merged_into(origin_sha.clone())
        .with_entries(vec![LaneEntry::Commit(typo_sha)]),
)
.with_pull_request(GraphPullRequest {
    number: 99,
    source_branch: "feat/theme".into(),
    target_branch: "main".into(),
});
// No current branch: the base view, where every line with commits gets a lane.

print!("{}", graph.render(&Terminal::new()));
```

### Input

| Type | Meaning |
|------|---------|
| `LaneEntry::Commit(sha)` | A commit, by **full** SHA. The component shortens it for the label. |
| `LaneEntry::Elided(n)` | `n` commits left out, drawn as one `+n` square. |
| `GraphLine` | A branch and the commits it has that its parent lacks, oldest first; its parent (`None` is the default branch), fork commit, tip, merge commit, creation time, and tip time. |
| `GraphLine::forked_at(sha)` | The commit the lane hangs from. Unset means the connection is unknown: the lane is drawn unconnected. |
| `GraphLine::with_tip(sha)` | The branch's own tip. A line without commits is labeled here; a line with commits defaults to its newest drawn commit. The fork commit is never used as a tip. |
| `GraphLine::merged_into(sha)` | The merge commit, on another lane, that merged this line. Its lane is found by where that commit is drawn. |
| `with_ref(name, sha)` | A ref tip: local branch, remote branch, or tag. |
| `GraphPullRequest` | An open PR: number, source branch, and target branch. |
| `with_current_branch(name)` | The checked-out branch. Unset, or the default branch, is the **base view**. |
| `with_incomplete_history()` | The caller could not establish some of the history it passed; the graph shows the incomplete-history notice. |

### Lanes and tags

- **Lanes:** the default branch, the current branch, its fork parent when that is not the default branch, and `origin/<default>` when it is passed as a line with commits of its own (it has diverged). In the base view, every line with commits of its own gets a lane. A merged branch whose line has commits keeps its lane: the caller passes the branch's own commits, and `merged_into` draws where they were merged.
- **Merges:** a lane with `merged_into` ends in a `merge` at that commit on the lane where it is drawn. Any lane can be the destination: the default lane, a parent's lane, or another branch's.
- **Tags:** every ref whose tip is a drawn commit, except a lane's own tip (its label names it), and every line with no commits of its own, on its `with_tip` commit.
- **PRs:** a tag `PR #n → target` on the source branch's tip.
- **Order:** lanes forking at the same commit follow creation time, oldest first. Lanes without a creation time come last. A lane merged into a sibling (or into a lane hanging from that sibling) is emitted before it, because a merge can only be drawn after the merged lane's commits.

For example, a default lane `1111111 → 2222222 → mmmmmmm → 3333333` (refs `origin/main` at `mmmmmmm`, `main` at `3333333`) and a line `feat/x` forked at `1111111` with commits `aaaaaaa`, `bbbbbbb`, `merged_into(mmmmmmm)` emit (display IDs are the first 7 SHA characters):

```text
gitGraph
    commit id: "1111111"
    branch feat/x
    checkout feat/x
    commit id: "aaaaaaa"
    commit id: "bbbbbbb"
    checkout main
    commit id: "2222222"
    merge feat/x id: "mmmmmmm" tag: "origin/main"
    commit id: "3333333" tag: "main"
```

After `biscuit-visualized`'s repair, `mmmmmmm`'s parents are `2222222` and `bbbbbbb`.

### Nothing undrawn is substituted

The component never draws something at a commit other than the one it was given:

| Input | When | Result |
|---|---|---|
| `forked_at` | unset, or its commit is not drawn | The lane is drawn **unconnected** (declared before the default lane's first commit, so its first commit has no parent), never from the start of another lane |
| a ref, a label-only line's tip, a PR's source tip | its commit is not drawn | The tag is left out |
| `merged_into` | its commit is not drawn, would be emitted before the merged lane's commits, or already merges another lane | A plain `commit`, no merge |

Each of these, and `with_incomplete_history()`, sets `GitGraphPlan::incomplete`, and the rendered graph is followed by the dim line `INCOMPLETE_HISTORY_NOTE` ("Some history is not shown"), after the hidden-lanes note. Tags of lanes the height cap leaves out are counted by that note instead. A PR for a branch the graph does not know has no commit to account for, and is not counted.

### Sizing and fitting

| Method | Description |
|--------|-------------|
| `with_scale(f32)` | Scale relative to terminal text; the default is `DEFAULT_GIT_GRAPH_SCALE` (1.25). |
| `with_width(ImageWidth)` | Overrides the scale-derived width. `ImageWidth::Scale` replaces the scale; any other width is used as given and is never trimmed to. |
| `plan(GraphViewport)` | The fitted Mermaid text plus its columns, rows, hidden lanes, trimmed commits, and `incomplete`. |
| `GraphViewport::for_terminal(term, layout)` | Available columns after margins, terminal rows, and cell size. |
| `mermaid()` | The untrimmed Mermaid text. |

The image is sized with `ImageWidth::Scale` (see [TerminalImage](./terminal_image.md)): at 125%, gitGraph's 10-unit commit IDs and 14–16-unit branch labels read comfortably. Fitting works in two steps:

1. **Height (base view only).** Past half the terminal's rows, the graph keeps the default lane and adds the other lanes most recently active first, each with its drawn ancestors (the lanes holding its fork and merge commits, and its parent's lane), until the next would not fit. A dim line then says `N more worktrees not shown`.
2. **Width.** While the graph is wider than the available columns, commits move into `+N` squares one at a time. A commit beside an existing square goes first, then the oldest commit on the lane showing the most. Lane tips, fork points, merge destinations, and tagged commits are never trimmed. Only when nothing more can be trimmed does the image shrink to fit.

`biscuit-visualized` keeps neighboring tags from overlapping (see [its gitGraph corrections](../../../biscuit-visualized/docs/mermaid-gitgraph.md)). It widens the commit spacing only when two tags on different commits would collide, and only as far as a one-em gap needs. A single long label, such as `main` and `origin/main` stacked on one commit, keeps the default spacing, so trimming narrows the image as usual. When tags do collide, the wider spacing applies to every column, so trimming may not narrow the image enough and it shrinks instead. Labels are never shortened.

Text measurement uses the host's fonts, so exact sizes differ slightly by OS.

### Rendering

- **Terminal:** the fitted graph as an inline image. Without image support, the Mermaid code block. The hidden-lanes note and the incomplete-history notice follow either one.
- **Tree:** the untrimmed graph as `MermaidDiagram` projects it (an image node whose alt text is the Mermaid source).
- **Browser:** the untrimmed graph's SVG as a raw-HTML island, in the default theme unless one is set. If rendering fails, the Mermaid source in a code block.

### Renderer workarounds

`mermaid-rs-renderer` 0.3.1 reads everything after `branch` or `merge` as the branch name, attributes included:

- `branch` statements carry no attributes.
- `merge` statements carry their `id:` and `tag:`, which costs the merge its second parent in the parser. `biscuit-visualized` restores it before layout, so the component emits labeled merges and the drawn edge is exact.
- The parser drops a labeled `merge` into a lane that has no commit yet, so a merge is emitted only where its destination lane already has one. A merge that would be the default lane's first commit is therefore drawn as a plain commit; pass at least one default-lane commit below it (`wt list` passes the merge's first parent).

Its parser also always names the first lane `main`: the default branch is that lane, and its real name appears as a tag when you pass its ref. A non-default branch named `main` is drawn as `main~` (`~` cannot occur in a git branch name). Commit IDs are labels, and parents are looked up by ID, so repeated IDs are made unique. Lane order is the order of the `branch` statements.

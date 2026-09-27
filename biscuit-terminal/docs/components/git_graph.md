# GitGraph

Draws git branch topology as a Mermaid `gitGraph`, rendered through [MermaidDiagram](./mermaid_diagram.md). The caller gathers the git facts and hands them over typed; the component runs no git commands. It decides which refs get lanes and which become tags, orders the lanes, marks elided commits, works around `mermaid-rs-renderer`'s parser, and fits the graph to the terminal by trimming commits and lanes rather than shrinking it.

Requires the `image` feature. The design is item 5 ("Graph View") of the worktree fix `2026-09-24-ux-improvements`.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

let graph = GitGraph::new(
    "main",
    vec![LaneEntry::Commit(base_sha.clone()), LaneEntry::Elided(4), LaneEntry::Commit(origin_sha.clone())],
)
.with_ref("main", base_sha.clone())
.with_ref("origin/main", origin_sha)
.with_line(
    GraphLine::new("feat/theme")
        .forked_at(base_sha)
        .with_entries(vec![LaneEntry::Commit(theme_sha)])
        .with_created_at(created_unix_seconds),
)
.with_pull_request(GraphPullRequest {
    number: 99,
    source_branch: "feat/theme".into(),
    target_branch: "main".into(),
})
.with_current_branch("feat/theme");

print!("{}", graph.render(&Terminal::new()));
```

### Input

| Type | Meaning |
|------|---------|
| `LaneEntry::Commit(sha)` | A commit, by **full** SHA. The component shortens it for the label. |
| `LaneEntry::Elided(n)` | `n` commits left out, drawn as one `+n` square. |
| `GraphLine` | A branch and the commits it has that its parent lacks, oldest first; its parent (`None` is the default branch), fork commit, creation time, and tip time. |
| `with_ref(name, sha)` | A ref tip: local branch, remote branch, or tag. |
| `GraphPullRequest` | An open PR: number, source branch, and target branch. |
| `with_current_branch(name)` | The checked-out branch. Unset, or the default branch, is the **base view**. |

### Lanes and tags

- **Lanes:** the default branch, the current branch, its fork parent when that is not the default branch, and `origin/<default>` when it is passed as a line with commits of its own (it has diverged). In the base view, every line with commits of its own gets a lane.
- **Tags:** every ref whose tip is a drawn commit, except a lane's own tip (its label names it), and every line with no commits of its own (a branch already in its parent).
- **PRs:** a tag `PR #n → target` on the source branch's tip.
- **Order:** lanes forking at the same commit follow creation time, oldest first. Lanes without a creation time come last.

### Sizing and fitting

| Method | Description |
|--------|-------------|
| `with_scale(f32)` | Scale relative to terminal text; the default is `DEFAULT_GIT_GRAPH_SCALE` (1.25). |
| `with_width(ImageWidth)` | Overrides the scale-derived width. `ImageWidth::Scale` replaces the scale; any other width is used as given and is never trimmed to. |
| `plan(GraphViewport)` | The fitted Mermaid text plus its columns, rows, hidden lanes, and trimmed commits. |
| `GraphViewport::for_terminal(term, layout)` | Available columns after margins, terminal rows, and cell size. |
| `mermaid()` | The untrimmed Mermaid text. |

The image is sized with `ImageWidth::Scale` (see [TerminalImage](./terminal_image.md)): at 125%, gitGraph's 10-unit commit IDs and 14–16-unit branch labels read comfortably. Fitting works in two steps:

1. **Height (base view only).** Past half the terminal's rows, the graph keeps the default lane and adds the other lanes most recently active first, each with its drawn ancestors, until the next would not fit. A dim line then says `N more worktrees not shown`.
2. **Width.** While the graph is wider than the available columns, commits move into `+N` squares one at a time. A commit beside an existing square goes first, then the oldest commit on the lane showing the most. Lane tips, fork points, and tagged commits are never trimmed. Only when nothing more can be trimmed does the image shrink to fit.

Text measurement uses the host's fonts, so exact sizes differ slightly by OS.

### Rendering

- **Terminal:** the fitted graph as an inline image. Without image support, the Mermaid code block. The hidden-lanes note follows either one.
- **Tree:** the untrimmed graph as `MermaidDiagram` projects it (an image node whose alt text is the Mermaid source).
- **Browser:** the untrimmed graph's SVG as a raw-HTML island, in the default theme unless one is set. If rendering fails, the Mermaid source in a code block.

### Renderer workarounds

`mermaid-rs-renderer` 0.3.1 reads everything after `branch` or `merge` as the branch name, attributes included, so the component emits no attributes on those statements. Its parser also always names the first lane `main`: the default branch is that lane, and its real name appears as a tag when you pass its ref. A non-default branch named `main` is drawn as `main~` (`~` cannot occur in a git branch name). Commit IDs are labels, and parents are looked up by ID, so repeated IDs are made unique.

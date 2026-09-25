# The `wt list` Graph

On a terminal that shows inline images, `wt list` draws the branch topology below its table. The worktree CLI gathers the git facts; biscuit-terminal's [`GitGraph`](../../biscuit-terminal/docs/components/git_graph.md) component turns them into a Mermaid `gitGraph`, fits it to the terminal, and renders it. The CLI builds no Mermaid text and picks no width of its own.

The design is item 5 ("Graph View") of the fix `2026-09-24-ux-improvements`.

## Who does what

| Piece | Owner | Code |
|---|---|---|
| Which lines to gather, with full commit SHAs, fork points, ref tips, creation and activity times | worktree CLI | [`git_graph.rs`](../cli/src/commands/git_graph.rs) (`gather`, `GraphFacts`) |
| Open PRs (the same list the table's badges use) | worktree library | [`pull_requests.rs`](../lib/src/pull_requests.rs) |
| Lanes versus tags, lane order, `+N` elision squares, unique commit IDs, renderer workarounds | `GitGraph` | `biscuit-terminal/lib/src/components/git_graph.rs` |
| Scale, width cap, trimming, height cap, the "N more worktrees not shown" note | `GitGraph` | `GitGraph::plan` |

`GitGraph` runs no git commands, and `git_graph.rs` makes no layout decisions.

## When the graph is drawn

- Only when stderr is a terminal and `TERM_PROGRAM` (or `KITTY_WINDOW_ID`) names an image-capable terminal.
- Not for a detached current worktree.
- There is no minimum terminal width. `GitGraph` trims commits to fit and, only when even the fully trimmed graph is too wide, shrinks it.

Gathering starts on its own thread right after `parse_worktree_state`, in parallel with the table's status pass and the PR request. The `--perf` stages are `graph gather` and `graph image render (biscuit-terminal)`.

## The default lane

The default lane ends at the descendant of the local default branch and `origin/<default>` (one `merge-base` when both exist and differ). Both tips are passed as refs, so they appear as tags wherever they sit on a drawn commit.

- `origin/<default>` only ahead of the local branch (the PR-driven case): its extra commits are on the default lane, and the gap between the two tags shows the distance.
- Local only ahead (the Gitflow case): the same, the other way round.
- Diverged: the default lane is the local branch, and `origin/<default>` gets a line of its own from their fork point. That is the only case where it gets a lane.

Commits on the default branch (either tip) never count as a line's own commits.

## Views

### Focused view (a feature branch is checked out)

- The current branch's line holds its commits that neither default tip has, forked at its merge base with the default lane.
- When its fork-origin record names a parent that is another existing branch, that parent gets a line too, and the current branch forks from the parent instead. The parent's tip is passed as a ref.
- The default lane shows two shared commits ending at the fork point, then the newest commits the drawn branches lack.

`wt list -v` reuses the same `merge-base` for its "forked from" commit, and lists every commit on the branch that neither default tip has.

### Base view (the default branch is checked out)

- The default lane shows its 10 newest commits.
- Every worktree branch gets a line (gathered in parallel), forked from its recorded parent when that parent is also drawn, otherwise from the default lane.
- A branch already in the default branch has no commits of its own, so `GitGraph` draws it as a tag on its tip rather than a lane.
- When the graph is taller than half the terminal, `GitGraph` keeps the most recently active lanes (each line carries its tip's commit time) and prints "N more worktrees not shown".

### Windows and elision

Each line shows its 5 newest commits. Older ones fold into one `+N` square next to the fork, counted with a `rev-list --count` only when a line fills its window.

## PR tags

Each open PR from origin's own repository whose head is a drawn branch becomes a tag `PR #n → target` on that branch's tip. `GitGraph` matches PRs by branch name alone, so `GraphFacts::to_git_graph` filters them by source repository first (`PrListing::for_branch`); a fork's same-named branch never gets the tag.

## Sizing and `--width`

`GitGraph` draws at 125% scale: at 100%, the diagram's 16-unit body text is one terminal line tall. The width follows from the SVG's natural width and the terminal's cell size (8×16 when unknown), capped at the available columns. Past the cap, commits move into `+N` squares before anything shrinks.

`-w`/`--width` (`70`, `70ch`, `50%`) replaces the scale-derived width, and the graph is then never trimmed to fit it.

## Tests

- [`git_graph/tests.rs`](../cli/src/commands/git_graph/tests.rs): the facts each view hands over (exact full SHAs, fork points, parents, elision, origin ahead and diverged, a criss-cross history), the shared `merge-base`, call counts, PR filtering, and the `--width` override.
- `GitGraph`'s own tests cover the lane/tag rule, trimming, and sizing (`biscuit-terminal/lib/src/components/git_graph/tests.rs`).
- `level2_list_verbose.rs` runs the image path in real terminals.

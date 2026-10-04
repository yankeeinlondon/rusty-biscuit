# `wt list` Git Graph: Gathering, Layout, Tests

Load before changing `cli/src/commands/git_graph.rs`, `git_graph/topology.rs`,
or graph tests. The behavior a user sees is in `worktree/docs/git-graph.md`;
this page holds the implementation facts and traps.

## Division of labor

- `commands/git_graph.rs` only **gathers** `GraphFacts` (full SHAs, fork points,
  merge commits, refs, `incomplete`) and builds a biscuit-terminal `GitGraph`.
- `GitGraph` owns lanes, tags, merges, trimming, and sizing.
- It filters PRs by source repository first, because `GitGraph` matches by
  branch name alone.
- There is no minimum terminal width for the graph.

## Gathering (`git_graph/topology.rs`)

Two stages on scoped threads.

### Snapshot input

- `GatherInput::from_list(list, refs)` takes the tips explicitly: `wt list`
  gathers from the parse step's read during the remote wait, and again from
  the final read when the tips changed ([list.md](list.md#pipeline)). Every
  `log`/`rev-list`/`merge-base` argument is a SHA from `refs` (or a fork-origin
  `base_sha`), never a branch name; keep it that way, or a discarded
  speculative gather could still describe live refs.
- Verbose labels: `DETAIL_FMT` reads `%H` and `%D` with
  `--decorate-refs-exclude=HEAD|refs/heads/|refs/remotes/`, so `%D` carries
  only what the snapshot does not capture (tags, other decorated refs).
  `snapshot_labels` rebuilds `HEAD -> <current>` (at the snapshot's current
  tip), the remote-tracking names plus `RefTips::remote_heads`
  (`origin/HEAD` beside its target's tip), and the other local branches, in
  Git's reverse-refname order. A `--decorate-refs=` include pattern would drop
  tags: Git then decorates only the included refs.
- `verbose_labels_follow_the_snapshot_and_keep_live_tags` pins the spelling to
  Git's own `%D` when nothing moved.

### Stage 1: `History::classify`

Rule R4 with R4-A1: `C` is the leading run of
`rev-list --first-parent --parents T..X` that is in
`rev-list --ancestry-path T..X`.

Per selected branch, candidates in order: its drawn parent's tip, the default
lane tip, then a diverged `origin/<default>` tip. Results:

| Result | Strength | Drawn as |
| ------ | -------- | -------- |
| unmerged | — | lane |
| merged directly | strong | lane `T --not C^1`, `with_merge(T, C)` |
| no separate history | strong | a label at `T`, never an empty lane |
| integrated otherwise | weak | lane, no merge, incomplete |

- A strong result wins at the first candidate that yields one. A weak result
  lets the walk continue; it is returned only when no later candidate is strong.
  Example: a parent that has `T` only by way of `main` loses to `main`'s direct
  merge (`MergedDirectly { after_indirect: true }`).
- A `GatherGap` before any weak result is the result; one after it returns the
  weak result.
- A merged branch's fork is measured against `C^1`, or against its parent's tip
  only when the merge went elsewhere and was not found after an indirect match
  (the parent's tip contains `T` otherwise).
- A deferred direct merge whose fork no lane draws gets its merge edge, an
  unconnected lane, and the notice from `GitGraphPlan::incomplete` (gathering
  itself reports no gap).
  - In `a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment`
    the fork `W1` is drawn after all, because the parent `fix/wt-ux` continued
    after its own merge at `W1` and its lane is extended through it (see
    boundary walk below).

### Stage 2: lanes

- Every branch lane is built in parallel (`first_parent_entries`,
  `log --first-parent`); the default lane last, from what they did not place.
- Every lane is first-parent. Fork, merge, label, and default-ref commits are
  anchors, placed by `rev-list --first-parent --count A..tip` and verified in
  one `log --no-walk=unsorted --ignore-missing` (a plain `rev-parse` batch fails
  on any `tip~d` past the root), with `+N` squares for the exact runs between.
- The default lane runs down to its oldest anchor plus that anchor's first
  parent (from `%P`, no extra call), so a merge is never its first commit. The
  focused view also caps its window there.
- Verbose (`commit_details_since`) keeps its all-parents semantics and shares
  the focused view's one `merge-base`.

### Boundary walk (`place` → `extend`)

A lane's boundary (the first commit below it) is classified too, sharing one
`Classifications` cache per gathering:

- While a candidate merged it directly at `C`: the stops containing it become
  `C^1`, the lane is re-read (`History::lane_window`, whose `LaneWindow` also
  feeds `first_parent_entries`), and `with_merge(B, C)` is added oldest first
  before the tip edge. The fork becomes `merge-base(C^1, B)` of the oldest edge.
- The walk stops at an ordinary fork or indirect integration, and at the
  branch's fork-origin `base_sha` when that is `B` or newer on the tip's
  first-parent chain (`History::chain_distance`). A malformed, unknown, or
  off-chain value is ignored; an unknown one in a shallow clone is a gap. So a
  branch created at an already merged tip stays unconnected, with the notice.
- Cost: an ordinary lane pays two extra Git calls (`--is-ancestor` and one
  first-parent chain); the `--ancestry-path` call is skipped when the boundary
  is on the candidate's first-parent chain.

### Failure handling (never drop a verified fact)

- Git's exit 1 ("no") is read through
  `worktree::git::git_command_allow_no_match` and trusted only when
  `rev-parse --is-shallow-repository` says `false`.
- Any failure, unparseable line, or shallow "no" is a `GatherGap`: it sets
  `GraphFacts::incomplete` and never drops the graph or a verified fact.
  - A failed reread past an accepted earlier merge keeps the previous window
    (the source is its boundary, from `LaneWindow::with_known`).
  - An unreadable default-lane window still places its anchors
    (`first_parent_entries` is infallible); only a lane whose first
    `lane_window` failed is drawn empty.
- Tests fail one Git call with `worktree::git::recorder::fail_matching`
  (feature `count-git`; `git_command`/`git_command_allow_no_match` only), as in
  `git_graph::tests::a_failed_reread_after_an_accepted_merge_keeps_the_edge_and_its_source`.

## Layout (`GitGraph`)

- Every gathered line passes its branch tip (`GraphLine::with_tip`). `GitGraph`
  labels a line without commits at that tip and never at its fork.
- A tag, fork, or merge commit it cannot draw sets `GitGraphPlan::incomplete`
  ("Some history is not shown") **instead of being substituted**.
- `GraphLine::with_merge(source, destination)` (a `LaneMerge`, appended oldest
  first) draws a `merge`. A source in the middle of a lane makes `GitGraph` emit
  that lane in segments: pause after the source and the lanes forked at it,
  `merge` at the destination, resume after it.
  - one merge per destination commit;
  - merged lanes are ordered before the siblings they merge into;
  - a cycle is broken by drawing the later edge as a plain commit (incomplete);
  - the height cap keeps a lane with every merge destination's lane; trimming
    never folds a source or destination.
- `biscuit-visualized` restores the merge's second parent and widens commit
  spacing only as far as colliding tags (different commits, overlapping rows)
  need for a one-em gap, so an isolated long label such as `origin/main` keeps
  the renderer's default step
  (`biscuit_visualized::mermaid::default_gitgraph_commit_step()`).

## Tests

### L1 layout proof (portable)

`git_graph::tests::gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`:

- gathers real-Git fixtures and plans them with the real measurement at 120×40
  and 56×60;
- checks `biscuit-visualized`'s `gitgraph_geometry` (a dev-dependency) for tag
  overlaps, tags on their SHAs' IDs, exact merge parents, unchanged tag text,
  and each fixture's expected `incomplete` and hidden-lane counts (`Evidence`);
- the sparse-lanes fixture is complete and loses one lane to the height cap at
  120×40; `post_merge` checks the commit after each merge source hangs from it;
- `--width 40` is never trimmed (planned at 120×60 so no lane is hidden).

Other L1 facts:

- `observed_sparse_lanes()`'s `commit_on` dates each commit one second after the
  last, because the height cap ranks lanes by tip commit time in whole seconds
  and wall-clock `commit-tree` dates tie on a fast host (build-linux hid a
  different lane than macOS).
- `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` is the
  density proof: at 200×60 the step is `default_gitgraph_commit_step()` and
  every branch lane keeps at least two commits after its last `+N` square,
  except `fix/wt-ux`, whose recent run the trimmer folds first (it prefers
  commits beside a square) because `main` now draws `d6..d12`. In Kitty's real
  cell size it keeps three.

### L2 in a private Kitty (`cli/tests/level2_graph_in_kitty.rs`, macOS only)

- `wt list` runs under `script` so the transmitted APC (`c=` only) and the
  `CSI <rows> B` reservation are recorded; a window screenshot must show the
  PNG's drawn pixels exactly where the reservation starts.
- A short window (100×32) proves the half-height lane cap and its notice; a
  narrow one (56×60) the width clamp.
- Cases and kept artifacts (in the temp directory, for inspection):
  - `level2_graph_draws_a_merged_branch_in_kitty` (`Fixture::merged`): both
    sizes, one window at a time (macOS stops Kitty drawing a window the next one
    covers); `wt-graph-merged-<cols>x<rows>-{screenshot,transmitted}.png`.
  - `level2_graph_restores_lane_density_in_kitty`
    (`Fixture::sparse_lanes()`, 200×60): `wt-graph-sparse-200x60-*.png`, no
    notice (`fix/wt-ux` draws `W1` merged into `M103`, where `fix/sniff` forks).
  - `level2_graph_draws_a_branch_continued_after_its_merge_in_kitty`
    (`Fixture::continued_after_merge()`, the PR #105 shape, 200×60):
    `wt-graph-continued-200x60-*.png`, no notice.
- `Fixture::record_parent(branch, parent, base_sha)` records the commit the
  branch was created at, **never the parent's final tip**, which the
  fork-origin cutoff would read as stale.
- `GraphRun::assert_table_intact` checks row membership, not order, because rows
  follow the parent tree.
- **The screenshot comparison is mandatory** for the five tests that claim what
  Kitty drew; they never pass on text and APC alone.
- Gating:
  - `kitty_pixels_available()` (`KittyInstance::can_launch()` and
    `biscuit_test_harness::screen_capture_permitted`): a host without Screen
    Recording permission skips the whole test with the reason printed;
  - after the run, `graph_drawn_or_skip!` turns an `Unobserved` capture (no
    pixels even where the table is, because Kitty does not render a covered
    window) into the same visible skip via
    `test_toolkit::evaluate_harness(Level::L2, false, Backend::Kitty)`, so
    `BISCUIT_TEST_REQUIRED_BACKENDS=kitty` fails both instead;
  - a capture that shows the table but no graph in its band **fails**.
- Kitty never runs in CI, so `.config/nextest.toml` shows this binary's output
  on success to keep the skip reason visible.
- `level2_sparse_lanes_fixture_has_the_observed_topology` claims no pixels and
  keeps the plain `can_launch()` gate.

### Graph-stage timings (`cli/tests/perf_graph_stages.rs`, Unix only)

- `graph gather` and `graph image render (biscuit-terminal)` exist only when
  stderr is a terminal and `TERM_PROGRAM` names an image emulator, so a captured
  `wt list --perf` never reports them. `graph gather` is a child of the
  `remote wait ‖ local gather` (or `local gather`) group and overlaps the
  wait, so it is not part of the total; the render is a top-level row.
- The test runs `wt list --perf` through `script` in a 120×40 pty over
  `perf_support::graph::GraphFixture` (floor, ordinary, older essential
  connections, multiple selected branches; built by `git fast-import`), and
  separately at 200×60 over `GraphFixture::observed_sparse_lanes()`, printing
  the median and min–max spread.
- That fixture is the observed sparse-lanes history (a branch merged directly
  into `main` after its recorded parent merged `main` back). Its recorded
  parents, with the commits each branch was created at, are written to
  `GraphFixture::fork_store()`. The same history is built at L1 by
  `git_graph::tests::observed_sparse_lanes()` and in Kitty by
  `Fixture::sparse_lanes()`.
- It emulates **Ghostty**, because every other Kitty-protocol emulator waits out
  a 1 s cursor-position query in an unanswering pty, and `--perf` prints
  durations of 1 s or more in tenths of a second, which hides every difference.
- The floor row measures the remaining fixed cost (about 350 ms).
- Record with
  `WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release`.

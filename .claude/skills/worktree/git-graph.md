# `wt list` Git Graph: Gathering, Layout, Tests

Load before changing `lib/src/graph.rs` (gathering), `lib/src/graph/topology.rs`,
`cli/src/commands/git_graph.rs` (conversion and rendering), or graph tests. The behavior a user sees is in `worktree/docs/git-graph.md`;
this page holds the implementation facts and traps.

## Division of labor

- `worktree::graph` (library) only **gathers** `GraphFacts` (full SHAs, fork
  points, merge commits, refs, `incomplete`) in library-owned types
  (`GraphLine`, `LaneEntry`, `LaneMerge`). The library must not depend on
  biscuit-terminal or biscuit-visualized, so never reuse the component's types there.
- `cli/src/commands/git_graph.rs::to_git_graph` maps those facts onto a
  biscuit-visualized `GitGraph` (`biscuit_visualized::components`); `GitGraph` owns lanes, tags, merges, trimming,
  and sizing.
- `to_git_graph` filters PRs by source repository first, because `GitGraph`
  matches by branch name alone.
- Fact tests live in `lib/src/graph/tests.rs`; tests that need the resulting
  `GitGraph` (merges in the Mermaid text, layout) stay in
  `cli/src/commands/git_graph/tests.rs`.
- There is no minimum terminal width for the graph.

## Gathering (`lib/src/graph.rs`, `lib/src/graph/topology.rs`)

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
| integrated otherwise | weak | lane, no merge, `GraphFacts::merged_elsewhere` (not incomplete) |

- A strong result wins at the first candidate that yields one. A weak result
  lets the walk continue; it is returned only when no later candidate is strong.
  Example: a parent that has `T` only by way of `main` loses to `main`'s direct
  merge (`MergedDirectly { after_indirect: true }`).
- A `GatherGap` before any weak result is the result; one after it returns the
  weak result.
- Integrated otherwise is a **verified** answer, never a gap: `place` reports
  it as `Placement::merged_through` (the carrying merge `C` and its lane), and
  `assemble` turns it into `MergedElsewhere`, naming the carrying branch only
  when a drawn placement's own merge (`merges_at`) is `C`. It used to set
  `incomplete`, which produced a "history not shown" note nobody could act on
  (observed 2026-10-04: a branch whose last commit reached `origin/main`
  through another branch's PR).
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
- Cost: an ordinary lane asks two extra questions (`--is-ancestor` and one
  first-parent chain); the `--ancestry-path` question is skipped when the
  boundary is on the candidate's first-parent chain.

### Questions answered in-process (`topology/commits.rs`)

- Every `History` method is a **question named by its Git command**
  (`merge-base --is-ancestor`, `rev-list --first-parent --count A..tip`, …).
  `Commits` (one per gather, a `Mutex` shared by the gather's threads)
  answers it from `gix` commit reads; the command itself runs only when
  `Commits::open` returns `None` (gix cannot open the repository, or a grafts
  file or `refs/replace/` would make Git see other parents). A graph gather
  therefore starts **no git process**; verbose's two `git log` listings
  still do.
- The answers are Git's, not merely equivalent: reachability and merge bases
  port `paint_down_to_common` (commit-date queue, ties first in first out),
  several best bases are ordered newest first with ties after earlier ones
  (Git's `commit_list_insert_by_date`), a `shallow`-file commit has no
  parents, and `A..tip` along a first-parent chain is the leading run of
  commits no stop reaches (monotone, so found by doubling plus bisection).
  Exclusion uses every parent of a stop, as Git's `--first-parent --not`
  does. Don't "simplify" any of these: the parity tests below catch it.
- `shallow` is "the `shallow` file exists", even empty, as Git reads it (gix's
  own `is_shallow` wants it non-empty).
- Every question passes through `crate::questions::ask` whichever way it is
  answered: recorded in test builds, and failed by `recorder::fail_matching`.
  Count history with `questions::start_recording`/`finish_recording`; the
  `recorder` log now shows only real processes.
- Parity: `graph::tests::with_each_history` runs a `History` test against both
  answerers; `in_process_history_agrees_with_git_on_*` compares every view of
  each fixture with `graph::history_through_git::enter()` (forces Git for the
  whole process, `count-git`/tests only); `in_process_merge_bases_pick_the_base_git_prints`
  pins the tie-break; the CLI's `gather_failing` gathers both ways.
- Keep `parallel`/`locate`'s thread fan-out for the Git path only; in-process
  answers run sequentially (they hold one lock).


### Failure handling (never drop a verified fact)

- A "no" (Git's exit 1, read through
  `worktree::git::git_command_in_allow_no_match`, or the in-process `false`)
  is trusted only when the shallow question says `false`. In-process walks
  (`Commits::reaches` inside a range) are not questions and never apply this
  rule; only `History::is_ancestor`/`merge_base` do, as before.
- Any failure, unparseable line, or shallow "no" is a `GatherGap`: it sets
  `GraphFacts::incomplete` and never drops the graph or a verified fact.
  - A failed reread past an accepted earlier merge keeps the previous window
    (the source is its boundary, from `LaneWindow::with_known`).
  - An unreadable default-lane window still places its anchors
    (`first_parent_entries` is infallible); only a lane whose first
    `lane_window` failed is drawn empty.
- Tests fail one question with `worktree::git::recorder::fail_matching`
  (feature `count-git`), matched on the Git command's arguments; it fails the
  in-process answer and the Git process alike, as in
  `git_graph::tests::a_failed_reread_after_an_accepted_merge_keeps_the_edge_and_its_source`.

## Layout (`GitGraph`)

- Every gathered line passes its branch tip (`GraphLine::with_tip`). `GitGraph`
  labels a line without commits at that tip and never at its fork.
- A tag, fork, or merge commit it cannot draw sets `GitGraphPlan::incomplete`
  **instead of being substituted**; `wt list` words it as a closing note
  ([list.md](list.md)).
- `GraphLine::with_merge(source, destination)` (a `LaneMerge`, appended oldest
  first) draws a `merge`. A source in the middle of a lane makes `GitGraph` emit
  that lane in segments: pause after the source and the lanes forked at it,
  `merge` at the destination, resume after it.
  - one merge per destination commit;
  - merged lanes are ordered before the siblings they merge into;
  - a cycle is broken by drawing the later edge as a plain commit (incomplete);
  - the height cap keeps a lane with every merge destination's lane; trimming
    never folds a source or destination.
- Merman lays out and draws the emitted `gitGraph` as Mermaid.js does, so a
  merge keeps both parents with no correction. `biscuit-visualized` writes a
  `commitSpacing` only when colliding tags (different commits, overlapping
  rows) need it for a one-em gap, so an isolated long label such as
  `origin/main` keeps Merman's default step of 50
  (`biscuit_visualized::mermaid::default_gitgraph_commit_step()`).

## Tests

### L1 layout proof (portable)

`git_graph::tests::gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`:

- gathers real-Git fixtures and plans them with the real measurement at 120
  and 56 columns; rows come from the measured whole graph (twice its rows, so
  the half-height cap hides nothing, or one row short of that when `Evidence`
  expects hidden lanes), because Merman's lanes are taller than a fixed 40
  rows allows;
- checks `biscuit-visualized`'s `gitgraph_geometry` (a dev-dependency) for tag
  overlaps, tags on their SHAs' IDs, exact merge parents, unchanged tag text,
  and each fixture's expected `incomplete` and hidden-lane counts (`Evidence`);
- the sparse-lanes fixture is complete and loses one lane to the height cap one
  row short of the whole graph at 120 columns; `post_merge` checks the commit after each merge source hangs from it;
- `--width 40` is never trimmed (planned with the whole graph's rows so no lane
  is hidden).

Other L1 facts:

- `observed_sparse_lanes()`'s `commit_on` dates each commit one second after the
  last, because the height cap ranks lanes by tip commit time in whole seconds
  and wall-clock `commit-tree` dates tie on a fast host (build-linux hid a
  different lane than macOS).
- `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` is the
  density proof: at 200×60 the step is `default_gitgraph_commit_step()`;
  `feat/schema-enhancement` and `fix/sniff` keep a second commit after their
  last `+N` square whenever one more commit step still fits 200 columns
  (derived from the geometry's width and step). At the 8×16 fallback cell
  Merman's layout is too wide for that, so each keeps only its tip. `fix/wt-ux`'s
  recent run is folded first (the trimmer prefers commits beside a square).

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

- `graph_history` and `graph_render` exist only when stderr is a terminal and
  `TERM_PROGRAM` names an image emulator, so a captured `wt list --perf`
  never reports them. `graph_history` is a concurrent child of
  `remote_and_local` (or `local_reads`) and overlaps the wait, so it is not
  part of the total; its sequential steps (`shallow_check`, `default_tips`,
  `focused_merge_base`, `verbose_details`, `lane_assembly`) carry `git_calls`
  that add up to its own. `graph::gather_timed` records them; the whole
  assembly loop, repeats and fork-holder searches included, is one
  `lane_assembly` step. The render is a top-level span.
- The test runs `wt list --perf=json` through `script` in a 120×40 pty over
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
  a 1 s cursor-position query in an unanswering pty, which would swamp the
  render stage.
- The floor row measures the remaining fixed cost (about 350 ms).
- Record with
  `WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release`.

## Focused view: lanes that hold a fork

- `focused_view` loops: after each `assemble`, every `GraphFacts::forked_off_line`
  entry (a fork no drawn lane holds, found by `drawn`, which checks each
  lane's `placed` anchors **and** its drawn entries) looks for a
  `fork_holder`: an undrawn worktree branch whose first-parent chain holds
  the fork (`classify` gives `NoSeparateHistory`), recorded parent first.
  Found holders are selected with their own recorded parents and the view
  is gathered again; the loop ends when nothing new is found.
- **Trap:** `GitGraph::in_view` filters a focused view's lanes itself. It
  used to keep only the current branch and its parent, so a holder `wt`
  passed was silently dropped and its fork reported as undrawn. It now keeps
  the parent chain and any lane holding an in-view lane's fork.
- `git_graph::to_git_graph` (a CLI function; `GraphFacts` is the library's) tags each `MergedElsewhere` tip `in <into>`
  (`with_ref`), only when that tip is one of the branch line's drawn
  entries: an undrawn ref would come back as a `GraphOmission::Tag` note.

## Merges into lanes other than parent or default

- `place` classifies against parent, default, and origin only. When that
  gives `Unmerged` or `IntegratedOtherwise`, `direct_merge_elsewhere` tries
  every other selected branch (not the parent) and accepts only
  `MergedDirectly` (tip is the merge's second parent). It overrides only
  `merge`: the fork and stop stay from the first classification. Measuring
  the fork against the merging lane hung a merged-into-child branch from
  the child.
- Cost: one `--is-ancestor` per (branch, other lane) pair not already in
  `Classifications`, pinned in
  `the_base_view_gives_every_worktree_branch_a_line`.
- `GitGraph`'s emitter used to drop a merge whose destination lane is
  emitted before the source lane (depth-first order). `EmitState::awaiting`
  now parks a lane before such a destination; emitting the source resumes
  it, and the final loop drops only waits that can never end.

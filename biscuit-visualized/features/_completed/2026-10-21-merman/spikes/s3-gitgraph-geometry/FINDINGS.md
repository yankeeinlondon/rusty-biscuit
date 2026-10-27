# Spike S3: gitGraph geometry from Merman's layout data (R-S8, AC-S6)

Run 2026-10-21 on macOS, Merman `482fcde7faffb05a466f198085d615e604075d97`
(features `svg`, `all-diagrams`, `layout-cytoscape`; the ruling-14 set).

## Question

Can `GitGraphGeometry` be built from `RenderRequest::layout_json` output, and
do the assertions in `worktree/cli/src/commands/git_graph/tests.rs` that read
geometry hold on it?

## Which copy ran

Neither of the two routes the spec lists could run as written:

- a `[patch]`ed `biscuit-visualized` cannot be used, because `worktree-cli`
  depends on it by **path**, and Cargo's `[patch]` does not replace path
  dependencies;
- the test file is a private unit-test module of the `wt` binary that calls
  `worktree` internals (`gather`, `to_git_graph`, `GraphViewport`, real Git
  fixtures), so a copy cannot compile outside that crate.

What ran instead, a **replay of captured inputs**:

1. `geometry_of` in `tests.rs` was instrumented for one run (an
   `S3_CAPTURE_DIR` dump of the Mermaid text and the resulting geometry), the
   76 `git_graph::tests` were run (`cargo nextest run -p worktree-cli -E
   'test(/git_graph::tests::/)'`, 76 passed), and the instrumentation was
   reverted with `git checkout` (no diff remains outside `spikes/`).
2. That gave 31 `geometry_of` calls from 14 tests, stored in `inputs/`: each
   holds the exact Mermaid text a test laid out and today's
   (`mermaid-rs-renderer`) geometry for it.
3. `src/main.rs` lays out each text with Merman, builds the geometry from
   `layout.GitGraphDiagram` (our own serde structs; tag bounds from the SVG's
   `tag-label-bkg` polygons, so from Merman's measurer, not ours), and checks
   every property a `worktree` assertion reads.

Every geometry assertion in those tests reads only these properties, so they
are checked as three groups:

| Group | Properties | Tests |
| ----- | ---------- | ----- |
| S | commit ids and order (`index`), `lane`, `parents`, tag texts per commit | all 14 |
| O | `tag_overlaps()` is empty | the 9 `gathered_graph_lays_out_*` tests and the 200×60 test |
| P | `commit_step == default_gitgraph_commit_step()` (line 2193) | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` |

Reproduce: `CARGO_TARGET_DIR=/tmp/s3-target cargo run --release` (writes
`out/results.json`).

## Results

- **S passes for all 31 inputs.** Merman keeps every commit id (including
  `worktree`'s `+N` squares with trailing spaces), the order, the lane, both
  parents of every merge (no repair), and every tag. Commit `x` increases with
  `index` in every input.
- **The commit step is derivable.** Consecutive `pos` differ by exactly 50 in
  every input: Merman's `COMMIT_STEP` (40) plus `LAYOUT_OFFSET` (10).
- **P (line 2193) holds.** The 200×60 observed graph has no colliding tags, so
  its derived step equals Merman's default (50 = 50), with or without D36.
- **O fails for one test, `gathered_graph_lays_out_long_labels`** (both of its
  viewports): at the fixed step, `fix/very-long-exact-branch-reference-beta`
  overlaps `main` and `origin/main` on the next two commits. Today's renderer
  passes only because tag spacing widened its step to 127.6.
- **D36 would fix it, but Merman cannot be told to.** The smallest
  diagram-wide step giving every colliding pair a 16 px gap is 125.9 for
  `long_labels` and 64.2 for `observation_1`/`observation_2` (which have no
  overlap at 50, only less than the gap). Merman's gitGraph step is a
  compile-time constant in `merman-render/src/gitgraph.rs`; the only gitGraph
  config keys it reads are `rotateCommitLabel`, `showCommitLabel`,
  `showBranches`, `diagramPadding`, `parallelCommits`, and `titleTopMargin`.
  Mermaid.js has no such key either. **D36's "wider commit spacing in the
  generated Mermaid (gitGraph config)" is not available at this rev.**
- **Natural widths change.** Merman's diagram width is a median 1.15× today's
  (range 0.52–1.17×). `worktree`'s planner trims commits to fit a viewport
  using the natural size, so the Mermaid text it plans will change at
  cutover; this replay used today's plans and cannot show that effect.

## Go/no-go for "worktree tests unchanged" (AC-C4)

**No-go as specified.** With geometry from layout data, every assertion holds
except `long_labels`' "no overlapping tags", and the mechanism D36 relies on to
keep that one does not exist in Merman. One of these must change before the
gitGraph cutover (Phase 3, Wave 6):

1. get a commit-spacing key into Merman upstream (or carry a pinned fork with
   one) and keep D36;
2. widen spacing in our own post-process of the layout/SVG;
3. accept Mermaid.js's overlap and edit the one `worktree` assertion (D22).

## Limits

- Tag bounds came from Merman's font-free measurer, not ours (D31); real-font
  widths are wider (S2), which makes collisions more likely, not less.
- LR only: every captured input is LR. A TB/BT tag polygon carries a rotate
  transform and is reported as an error by the parser here.

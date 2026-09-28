---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-28-graph-continued-after-merge/spec.md
plan: worktree/fixes/2026-09-28-graph-continued-after-merge/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
    - worktree-cli
    - biscuit-terminal
source_files_during_phase_1:
    - worktree/cli/src/commands/git_graph/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-terminal/lib/src/components/git_graph.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
    - biscuit-terminal/lib/src/prelude.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/tests.rs
docs_updated_during_phase_2:
    - biscuit-terminal/docs/components/git_graph.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/biscuit-terminal/components.md
source_files_during_phase_3:
    - biscuit-terminal/lib/src/components/git_graph.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/topology.rs
    - worktree/cli/src/commands/git_graph/tests.rs
docs_updated_during_phase_3:
    - worktree/docs/git-graph.md
    - biscuit-terminal/docs/components/git_graph.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/biscuit-terminal/components.md
source_files_during_phase_4:
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/tests/level2_graph_in_kitty.rs
    - worktree/cli/tests/perf_support/graph.rs
    - worktree/cli/tests/perf_graph_stages.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/worktree/SKILL.md
---

# Implementation Log for 2026-09-28-graph-continued-after-merge (5 phases)

## Phase 1

Rulings, spikes, fixtures, and baselines. No production code changed.

### Spike S1: renderer segment semantics

`worktree/fixes/2026-09-28-graph-continued-after-merge/spikes/s1-renderer-segments/`
is a standalone crate (its own `[workspace]`, so the monorepo never builds it)
that runs `MermaidDiagram::gitgraph_geometry` on hand-written Mermaid. Run it
with `CARGO_TARGET_DIR=/tmp/wt-spike-s1-target cargo run -q` from that
directory. Output on 2026-09-28 (layout index, lane, id, parents after
`repair_gitgraph_merges`):

| Case | Result |
|---|---|
| 1. `commit B` on `b`, `commit P` on main, `merge b id: "C"`, `checkout b`, `commit N` | `C` parents `[P, B]`; `N` parent `[B]` |
| 2. As 1, with `commit id: "+2" type: HIGHLIGHT` before `N` | `+2` parent `[B]`; `N` parent `[+2]` |
| 3. `b` merged twice (`B1`→`C1`, `B2`→`C2`) with commits between, then `N` | `C1` `[P1, B1]`, `B2` `[B1]`, `C2` `[P2, B2]`, `N` `[B2]`; no `RenderFailed` |
| 4. `branch c` declared right after `commit B` (tagged `fix/sniff-pr`), before the merge | `c1` parent `[B]`; the tag stays on `B`; `C` `[P, B]`; `N` `[B]` |
| 5. `a` merged into sibling `s` (`S3`), `a` resumed (`a2`), `s` later merged into main (`C`) | `S3` `[s2, A]`, `a2` `[A]`, `C` `[P, s4]`; SVG rows main (y=20), `s` (y=92), `a` (y=164), the order of the `branch` statements |

SVGs of cases 1, 3, and 5 are kept beside the spike
(`1-mid-lane-merge.svg`, `3-merged-twice.svg`, `5-merge-into-sibling.svg`).

**D2 and D6 are confirmed.** The repair picks the source lane's newest commit
before the merge, so pausing the source at `B` makes `B` the merge's second
parent; `mermaid-rs-renderer` 0.3.1 leaves the source head at `B`, so a resumed
commit or `+N` square follows `B`; a lane merged twice needs no change.
`biscuit-visualized` needs no change.

### Spike S2: boundary reproduction

`spikes/s2-boundary/boundary.sh` builds each shape in a scratch repository
with `commit-tree`, then walks G2 (boundary), G3 (classification with the fast
path), G5 (stop replacement, fork), and G8 (record cutoff) with plain Git,
counting every Git call. Run it with `./boundary.sh`.

| Shape | Boundaries and answers | Result |
|---|---|---|
| E1 *behind* (main `P`, origin `C`) | `B` at n=1 → `MergedDirectly(C)`, `C^1 = P`; fork `merge-base(P, B) = P`; next boundary `d2` at n=12 → `NoSeparateHistory` (G3 fast path) | edge `(B, C)`, fork `P` ✓ |
| E1 *diverged* (main `P'`) | same; `C` found on candidate 1 (`origin/main`); stops `[P', C]` → `[P, P']` | edge `(B, C)` on the origin line, fork `P` ✓ |
| E1 `fix/sniff-pr` | tip `B` classified `NoSeparateHistory` against `fix/wt-ux` (G3) | label, no reconstruction ✓ |
| Sparse lanes, `fix/wt-ux` | `W1` at n=68 → `MergedDirectly(M103)`, `C^1 = d12`; fork `d5`; next boundary `d5` at n=76 → `NoSeparateHistory` | edge `(W1, M103)`, fork `d5` ✓ |
| Sparse lanes, `fix/wt-ux`, record `base_sha = d5` (E5) | the cutoff fires at the last boundary (`d = n = 76`) | same facts as with no record; two fewer calls |
| Sparse lanes, `fix/wt-ux`, record = parent's final tip (today's L2 fixtures) | not on the chain → stale | same as no record ✓ (so E5 is about realism, not correctness) |
| Sparse lanes, `fix/sniff` (stop `[d13]`) | `W1` at n=94 → `NoSeparateHistory` against `fix/wt-ux` | no edge; fork `W1` on `fix/wt-ux`'s lane, which now draws it ✓ |
| Merged twice, then continued | `b2` → `C2` (fork `b1`), `b1` → `C1` (fork `d1`), `d1` → `NoSeparateHistory` | edges oldest first `(b1, C1)`, `(b2, C2)`, fork `d1` ✓ |
| New branch at merged tip, record at `b1` | `d = n = 1` → cutoff before classifying | no edge ✓ |
| Same, no record (control) | `b1` → `MergedDirectly(C)`; fork `d1` | edge reconstructed ✓ |
| `B` integrated indirectly | `t1` → `IntegratedOtherwise` | no edge ✓ |
| Shallow `--depth 2` of "merged twice" | `b2` → `MergedDirectly(C2)`, then the planned `is_ancestor(b2, C2^1)` is a shallow "no" → gap | **no edge at all** (see the G5 amendment) |
| Same, without that check | edge `(b2, C2)` accepted; `merge-base(p2, b2)` empty (gap); `b2`'s parents are cut (gap) | first edge kept, notice ✓ |

**Git call counts for one ordinary unmerged lane** (whole walk, including the
calls today already makes):

| Lane | Calls | Added over today |
|---|---|---|
| short (3 commits < `LINE_WINDOW`) | 1 `log`, 1 `merge-base --is-ancestor`, 1 `rev-list` | **+2** (+3 without G3: `rev-list --ancestry-path`) |
| long (9 commits) | 2 `log`, 1 `merge-base --is-ancestor`, 2 `rev-list` | **+3** (the count `rev-list` and first `log` exist today) |
| child forked on its recorded parent (candidates parent, main) | 3 | +2 |

### Rulings confirmed or amended

- **A1–A3, D1–D5, G1–G4, G6–G10, E1, E2, E4, E5, X1, X2:** confirmed; no
  spike contradicted their premises.
- **D6 confirmed:** no `biscuit-visualized` change (S1).
- **G5 amended:** the separate `is_ancestor(B, C^1)` check is dropped.
  Classification already proves it (`C` is the oldest first-parent commit
  inside `--ancestry-path B..X`, so `C^1` does not descend from `B`; `C^1 = B`
  would be `NoSeparateHistory`). In a shallow clone its "no" is always a gap,
  which would stop every shallow reconstruction before its first edge and make
  E3's "older boundary fails" fixture impossible. A stop whose `is_ancestor(B,
  stop)` is a shallow gap is kept and sets `gap`.
- **G8 confirmed**, with one observation: a realistic record at the true fork
  (`d5`) fires the cutoff at the final ordinary boundary, which gives the same
  facts as `NoSeparateHistory` there.
- **P1 refined:** +2 / +3 measured; one more `--is-ancestor` for each
  candidate tried before the one holding `B`.
- **P2 refined:** run-to-run drift exceeds one run's spread (below), so the
  regression rule uses the envelope of both baseline runs.
- **E3:** the fixtures are built (next section); the shallow pair uses depth 1
  and depth 2.

The plan's rulings section carries these amendments.

### L1 fixtures

Added to `worktree/cli/src/commands/git_graph/tests.rs`, each with a topology
sanity test that asserts nothing about the fixed behavior:

| Builder | Sanity test |
|---|---|
| `continued_after_merge(LocalMain::{AtMerge, Behind, Diverged})` (E1) | `continued_after_merge_fixture_has_the_observed_topology` (all three variants: merge parents, `S` merging `P`, `origin/main = C`, local `main`, `B` on `fix/wt-ux`'s first-parent chain and not on `origin/main`'s, `merge-base(P, B) = P`, 11 commits up to `B` (> `LINE_WINDOW`), `fix/sniff-pr`'s record at `B`, no `fix/wt-ux` record) |
| `merged_twice_and_continued()` | `merged_twice_fixture_has_two_merges_of_one_lane` |
| `new_branch_at_merged_tip()` | `new_branch_at_merged_tip_fixture_records_its_creation_at_the_merged_tip` (the record is `b1`, one commit below `new`'s tip; `old` is deleted) |
| `indirect_boundary()` | `indirect_boundary_fixture_reaches_main_only_through_another_merge` |
| `shallow_merged_twice(depth)` | `shallow_merged_twice_fixtures_cut_history_where_expected` (depth 1: `b2` absent, `n`'s parent cut; depth 2: `C2`, `p2`, `b2`, `n` present, `b1`, `C1` absent, `b2`'s parent cut) |

New helpers: `forked_at` (a record with a real `base_sha`), `parent_line`,
`repo_with_root`, `set_branches`. Every fixture commit is made with
`commit_on` (E4). These are unit tests in `commands::git_graph::tests` with no
tier marker, so L1 runs them on both the library and binary targets (verified:
both targets ran them).

### Baselines (before)

Host: Apple M4 Max, macOS 27.2 (26B5091g). Profile `release`, 10 samples,
`WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release`
in `worktree/`, on unchanged graph code (HEAD `572ef7d26` plus this worktree's
unrelated `list_table` edits).

Run 1 (the plan's command; the 120×40 test prints medians only):

| Fixture | graph gather (median) | graph image render (median) |
|---|---|---|
| floor (one commit) | 14.1 ms | 353.3 ms |
| ordinary | 54.9 ms | 353.3 ms |
| older essential connections | 140.4 ms | 345.6 ms |
| multiple selected branches | 71.8 ms | 346.7 ms |
| observed sparse lanes, 200×60 | 79.9 ms (74.9–88.8) | 353.3 ms (347.1–357.2) |

Run 2 (the 120×40 test only, with a temporary edit adding min–max to its
table; the edit was reverted with `git checkout` and the file is unchanged):

| Fixture | graph gather (median, min–max) | graph image render (median, min–max) |
|---|---|---|
| floor (one commit) | 10.5 ms (9.8–12.0) | 348.4 ms (342.7–350.8) |
| ordinary | 40.6 ms (38.7–44.8) | 352.4 ms (346.5–353.9) |
| older essential connections | 128.3 ms (123.5–134.4) | 349.3 ms (344.6–351.9) |
| multiple selected branches | 65.2 ms (59.6–72.6) | 349.6 ms (345.6–360.3) |

The gather medians differ between runs by more than one run's spread (floor
14.1 vs 10.5 ms, ordinary 54.9 vs 40.6 ms), so the P2 comparison uses both
runs' envelope: floor gather 9.8–14.1 ms, ordinary gather 38.7–54.9 ms.

**Call counts pinned today.**
`the_base_view_gives_every_worktree_branch_a_line` asserts 5
`merge-base --is-ancestor`, 3 `merge-base`, 4 `log`, 1 `rev-list`.
`observed_sparse_lanes()`, base view (a scratch test, removed after the run):
24 calls in total: 1 `for-each-ref`, 1 `rev-parse`, 5 `log`, 3 `merge-base`,
4 `merge-base --is-ancestor`, 10 `rev-list`; `incomplete = false` in
`GraphFacts` (the notice comes from `GitGraph`).

### Verification

- `just test` in `worktree`: 770 passed, 30 skipped.
- `just lint` in `worktree`: clean.
- No production code, docs, or skill changed; the OS risk is nil for this
  phase (test-only fixtures; the shallow clone URL uses the existing
  Windows-safe `file://` spelling).

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no behavior, so it has no regression test. Each fixture's
sanity test proves the shape a later phase's behavior test depends on:

| Later requirement | Fixture | Proven now |
|---|---|---|
| PR #105 shape, base and focused views | `continued_after_merge(AtMerge)` | topology, records |
| Same before `--ff` (behind, diverged) | `continued_after_merge(Behind / Diverged)` | local main, `origin/main` |
| Merged twice, then continued | `merged_twice_and_continued()` | both merges, both sources off main's chain |
| New branch at a merged tip (+ control) | `new_branch_at_merged_tip()` | record at the boundary, `old` deleted |
| `B` integrated indirectly | `indirect_boundary()` | `t1` reaches main only through `O` |
| Shallow boundary crossing / older boundary fails | `shallow_merged_twice(1 / 2)` | which commits the clone has |

## Phase 2

`GraphLine` merges API. Behavior-neutral (A3): no Mermaid string, snapshot, or
plan assertion changed.

### Changes

- `biscuit-terminal` (`components/git_graph.rs`, `prelude.rs`): new
  `pub struct LaneMerge { source, destination }` (full SHAs), exported beside
  `GraphLine` and from the prelude. `GraphLine::merged_into` (field and
  builder) is removed; `GraphLine::merges: Vec<LaneMerge>` (oldest first) and
  `with_merge(source, destination)` replace it. A private
  `GraphLine::merge_destination()` returns `merges.last()`'s destination, and
  `arrange` (both the sibling-order closure and the merge table) and
  `lane_ancestors` read it, which is exactly the old single tip-sourced edge.
  Phase 3 replaces these three reads when it generalizes emission; `source`
  is not read by the component yet.
- `worktree-cli` `assemble`: `line.with_merge(placement.tip.clone(), merge)`
  (A2: the tip edge's source is the placement's full tip SHA).
- Component tests: the 13 `.merged_into(x)` calls became
  `.with_merge(<that line's tip>, x)`, where the tip is the line's `with_tip`
  SHA or, without one, its newest commit.
- `worktree-cli` tests: the 20 `.merged_into` assertions go through
  `merge_destinations(line) -> Vec<&String>` (defined beside `line()`), as
  the plan asks. `Some(&x)` became `[&x]`, `None` became an empty `Vec`.
  Clippy's `needless_borrow` rejected `merge_destinations(&line)` where the
  binding is already a `&GraphLine`, so every call passes the reference as is.

### Tests added

| Requirement | Test |
|---|---|
| `with_merge` appends edges oldest first (A1) | `biscuit-terminal` `git_graph::tests::with_merge_appends_edges_oldest_first` |
| The gathered tip edge's source is the branch tip (A2) | `worktree-cli` `git_graph::tests::a_merged_current_branch_keeps_its_lane_and_merges_at_its_merge_commit` now asserts `merges == [LaneMerge { source: w2, destination: merge }]` |
| Behavior-neutral (A3) | every existing byte-exact Mermaid assertion in both crates passes unchanged; `git diff` of both test files shows no expected-string edits |

Both run in L1 (`just test`); neither name carries a tier marker.

### Docs and skills

`GraphLine::merged_into` was named in `biscuit-terminal/docs/components/git_graph.md`,
`.claude/skills/worktree/SKILL.md`, and `.claude/skills/biscuit-terminal/components.md`.
Each now names `with_merge`, and the component doc says only a line's latest
merge is drawn, with drawing every merge from its own source marked
**planned** (Phase 5 writes the full behavior). `worktree/docs/git-graph.md`
does not name the API and is unchanged.

### Verification

- `just test` in `biscuit-terminal`: 3345 passed, 55 skipped (49
  `git_graph::tests` ran).
- `just test` in `worktree`: 770 passed, 30 skipped.
- `just lint` in `biscuit-terminal` and `worktree`: clean.
- `rg merged_into` over `*.rs` and `*.md` outside `_completed/` and this fix's
  directory: only unrelated `worktree/lib/src/remove/safety.rs` and
  `worktree/cli/tests/remove.rs` test names, and the `worktree-cli` fixture
  name `nested_parent_merged_into_default` (English, not the API).
- No cross-OS run: the change is a type rename with no path, process, or
  platform code, so it carries no OS-specific risk.

## Phase 3

Emission and reconstruction. Wave 3a (`biscuit-terminal`) and Wave 3b
(`worktree-cli`) ran concurrently in disjoint files.

### Wave 3a — `GitGraph` segmented emission

**Changes** (`biscuit-terminal/lib/src/components/git_graph.rs`):

- **D1:** `arrange` resolves every edge of a drawn lane through
  `drawn_positions` into `Layouted::merges` (destination SHA → `Edge { source:
  (lane, position), destination: (LaneKey, position) }`) and
  `Layouted::sources` (`(lane, position)` → destinations, edge order). An edge
  is undrawable (sets `incomplete`) when its source is not on its own lane, its
  destination is undrawn or on the same lane, or the destination is already
  claimed (first in line order, then edge order, wins). Lanes hidden by the
  height cap are never resolved, so their edges stay uncounted.
- **D2:** `EmitState::finished` is replaced by per-lane cursors, a `paused` set,
  the `declared` lane order, and a `dropped` destination set. The cursor
  advances before an entry's merge and children. After a source and its
  children the lane pauses while a destination is neither emitted nor dropped,
  unless the source is its last entry (so tip edges emit exactly the Phase 2
  text). At a destination whose source is emitted and whose lane has a head,
  `merge` is emitted; a paused, no-longer-blocked source is then resumed between
  `checkout <source>` and `checkout <destination>`. Otherwise the destination is
  a plain commit and the edge is dropped (a paused source then resumes at that
  point). The private `GraphLine::merge_destination()` is removed.
- **D3:** `emit_merged_lanes_first` takes `(source lane, destination lane)`
  pairs and orders siblings by a stable depth-first pass (every sibling that must
  come first is placed before it). **Departure:** it considers every drawable
  edge in a sibling's subtree, not only the sibling's own edges, so a nested
  lane merging into a sibling's subtree is not dropped; own-edge cases order
  exactly as before. A cycle keeps its first lane first and is broken by D4.
- **D4:** after root emission, the first paused lane in declaration order has
  its blocking edges dropped (`incomplete`) and resumes; each round emits at
  least one entry, so no loop bound is needed. "Lane order" is taken as
  declaration order, which follows D3.
- **D5:** `lane_ancestors` adds the lane of every edge's destination.
  `trim_one_commit` pins both ends of every merge of a drawn lane (drawable or
  not), a superset of the old destination pins.
- Module docs gain "Merges and segments" (sources, segments, pause and resume,
  one merge per commit); "Lanes and tags", "Nothing undrawn is substituted",
  and `plan_with` are updated.

**Tests added** (`git_graph::tests`, all L1, Mermaid text plus exact parents
via `biscuit-visualized` geometry):

| Requirement | Test |
|---|---|
| mid-lane merge: `C` `[C^1, B]`, next commit's parent `B` | `a_lane_merged_from_its_middle_pauses_at_the_source_and_resumes_after_the_merge` |
| `+N` folded after `B` hangs from `B` | `a_square_folded_after_the_merge_source_hangs_from_the_source` |
| children forked before and after `B`; source resumes after `C` | `children_forked_before_and_after_the_merge_source_hang_from_their_own_commits` |
| child forked at `B` declared before the merge, label at `B` | `a_child_forked_at_the_merge_source_is_declared_before_the_merge` |
| merged twice and continued, source order, no repeated ID | `a_lane_merged_twice_and_continued_draws_both_merges_in_source_order` |
| merge into a sibling that merges into the default lane | `a_merge_into_a_sibling_that_merges_into_the_default_lane_emits_each_source_first` |
| cycle: terminates, one edge, `incomplete`, each commit once | `a_cycle_of_merges_draws_one_and_reports_the_other` |
| destination before its source: plain commit, no stranded pause | `a_destination_emitted_before_its_source_is_a_plain_commit_and_the_source_does_not_pause` |
| D1: source not on its lane | `a_merge_whose_source_is_not_on_its_lane_is_left_out_and_reported` |
| height cap keeps a mid-lane destination lane with its source | `the_height_cap_keeps_a_mid_lane_merges_destination_lane_with_its_source` |
| height cap hides source and destination lanes together, counted by the lane note | `the_height_cap_hides_a_source_lane_and_its_destination_lane_together` |
| width trimming never folds `B` or `C` | `trimming_never_folds_a_merge_source_or_its_destination` |

**Regression (byte-identical):** every existing expected string is unchanged,
and all 49 pre-existing tests pass. In addition, a temporary dump in the test
helpers (`mermaid`, `plan`, `geometry_of`) captured every Mermaid text and plan
of those 49 tests under the HEAD emitter and the new one; `diff -r` over the
40 dumped outputs was identical. The dump was reverted.

### Wave 3b — Boundary reconstruction

**Changes:**

- `topology.rs`:
  - `LaneWindow` (newest `LINE_WINDOW` commits, length, whether the length was
    counted, and verified `known` boundaries) is read once by
    `History::lane_window` (today's `newest` plus, for a full window, today's
    `count_first_parent`). `Extent::Until` now carries it, so
    `first_parent_entries` repeats neither call, and places a known boundary
    at its verified distance without `locate`. `Extent::Open` carries the
    default lane's window (the separate `window` argument is gone).
  - G2 `History::boundary`: the first parent of the oldest shown commit when
    the window is the whole lane, otherwise one `log --no-walk=unsorted
    --ignore-missing --format=%H %P tip~(length-1)`. No parent is a root in a
    complete clone and a gap in a shallow one; an uncounted length is a gap.
  - G3: `integration_into` returns `NoSeparateHistory` when the oldest commit
    of the first-parent chain `T..X` has `T` as its first parent, skipping
    `--ancestry-path`.
  - G8 `History::chain_distance(tip, recorded)`: `is_object_id` first (no Git
    call for a malformed value), then `rev-list --first-parent --count
    --ignore-missing tip --not <recorded>` (so an unknown object counts the
    whole chain instead of failing, and a failure is a real one) and `log
    --no-walk=unsorted --ignore-missing --format=%H tip~d`: on the chain only
    when that is `recorded`. Off the chain is stale in a complete clone; in a
    shallow clone `cat-file -e` decides between stale (the object exists) and
    a gap (missing, possibly past the cut).
- `git_graph.rs`:
  - `place()` classifies the tip as before (through the G4 cache), then
    `extend()` walks boundaries: classify `B` against the same candidates;
    `MergedDirectly` → G8 cutoff (computed lazily, once, only when an edge is
    about to be accepted, so a lane with an ordinary boundary pays nothing
    for its record) → replace each stop containing `B` with `C^1` (answers
    classification already has are reused: the winning candidate contains
    `B`, and without `after_indirect` every earlier one does not), dedup, and
    re-read the window. Distances must strictly increase and never revisit a
    boundary, else gap. `NoSeparateHistory`/`IntegratedOtherwise` stop
    cleanly; `Unmerged`/gap set `gap`.
  - The fork is `merge-base(C^1, B)` of the oldest accepted edge, expected on
    the parent's lane if recorded, else `C`'s lane. With no edge, today's fork
    rule runs unchanged; it is now computed after the walk, so a lane with an
    edge never pays for the fork it replaces.
  - `Shape::Lane { window, fork, earlier: Vec<EarlierMerge { source, merge,
    lane }>, merge }` (a named struct instead of the plan's tuple).
    `Placement::anchors` adds `(own lane, B)` and `(C's lane, C)`;
    `assemble` emits `with_merge(B, C)` oldest first, then the tip edge (A2).
  - G4 `Classifications`: `Mutex<HashMap<(commit, candidate tips),
    Arc<OnceLock<…>>>>`, created in `assemble`; the map lock is held only to
    fetch the cell, so two lanes asking the same question at once classify
    it once while unrelated classifications stay parallel. The tip
    classification goes through it too.
  - The diverged `origin/<default>` line reads its window inside its
    parallel job.
  - Module `//!` docs describe the boundary classification.

**Git call budget (P1), measured by the updated call-count tests:**

- `the_base_view_gives_every_worktree_branch_a_line` (three lanes, each short,
  each boundary an ordinary fork on the first candidate tried): `--is-ancestor`
  5 → 8, `rev-list` 1 → 4; `merge-base` 3 and `log` 4 unchanged. Exactly +2
  per lane, as ruled.
- `graph_and_verbose_share_one_merge_base`: `merge-base` (all forms) 2 → 3.

**Tests added** (`worktree-cli` `commands::git_graph::tests`, L1, `GraphFacts`
only, exact SHAs; each runs on the library and binary targets):

| Requirement | Test |
|---|---|
| E1 base view: fork `P`, exact `+7`, merges `[(B, C)]`, `fix/sniff-pr` a label at `B` under its parent, `!incomplete`, no commit on two lanes | `a_continued_branch_draws_its_earlier_merge_and_its_child_label_in_the_base_view` |
| E1 focused from `fix/wt-ux` and from `fix/sniff-pr` | `a_continued_branch_draws_its_earlier_merge_in_both_focused_views` |
| E1 *behind* (`C` on the default lane) and *diverged* (`C` on the `origin/main` line) | `a_continued_branch_merges_into_origin_main_before_a_fast_forward` |
| merged twice and continued: both edges oldest first, fork `d1` | `a_branch_merged_twice_draws_both_merges_oldest_first` |
| `B` integrated indirectly: no edge, today's facts | `a_boundary_integrated_through_another_merge_is_not_reconstructed` |
| shallow crossing (no edge, incomplete) and older boundary failing (first edge kept, fork unknown, incomplete) | `a_shallow_boundary_invents_no_merge_and_keeps_the_verified_one` |
| new branch at a merged tip, with the record (no edge) and without (edge) | `a_branch_created_at_a_merged_tip_does_not_claim_the_old_merge` |
| Input Robustness Matrix, every row, plus shallow rows and both controls | `fork_origin_cutoff_matrix` |
| ordinary unmerged lane: identical `GraphFacts` | `an_ordinary_unmerged_branch_gathers_unchanged_facts` |
| P1 budget | `the_base_view_gives_every_worktree_branch_a_line`, `graph_and_verbose_share_one_merge_base` (updated counts) |

`fork_origin_cutoff_matrix` writes the record with `save_atomic`, applies one
textual edit per cell, reads it with `load_from`, and asserts `(merges, fork,
incomplete)` of `new`'s line. Cells: unedited record at `B` (cutoff, control),
no record (edge, control), absent, `null`, `123`, `""`, duplicate key,
trailing garbage, 7-character, uppercase, unknown object, a tree object, off
the chain (the parent's final tip `C`, and the sibling `p`), an ancestor of `B`
on the chain (edge), `n1` newer than `B` (cutoff). For every cell it also
asserts no Git argument was `""`, the abbreviation, or the uppercase value.
Shallow rows (a depth-3 clone): no record (edge, incomplete from the tip's
shallow classification), the record at `B` (cutoff), unknown object (no edge:
`GatherGap`). Every cell passed on its first run; the two controls differ, so
the field is load-bearing.

**Pre-change proof for the literal test:**
`an_ordinary_unmerged_branch_gathers_unchanged_facts` was also run against
unmodified HEAD (`525611399`) in a scratch `git worktree` at `/tmp` (removed
afterward): it passed there too, so the literal is today's output.

**Smell grep (G8):** the added `worktree-cli` code has no `unwrap_or_default()`,
`.ok()`, or `filter_map` on `base_sha` or its Git answers; `ForkOrigin` has
no `#[serde(default)]`.

### Tests marked for Phase 4

Four `worktree-cli` tests assert the old behavior this phase changes. Each
is `#[ignore = "flips in Phase 4"]` with a `// flips in Phase 4: …` note and
still compiles:

- `a_branch_continued_after_its_merge_is_an_unmerged_lane` (`b1` now drawn)
- `a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment`
  (`W1` now on `fix/wt-ux`'s lane)
- `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`: only
  the sparse-lanes entry fails, on `incomplete`. A scratch run with that entry
  set to `incomplete: false` and the merge `(M103, d12, W1)` added passed
  entirely (hidden lanes still `[1, 0]`); the edit was reverted, since Phase 4
  owns it.
- `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`: every
  assertion passes up to `assert!(plan.incomplete)`, which is now false;
  density and `M104`'s exact parents still hold.

With both waves in place, each of the four fails only where its expectation
is the old behavior.

### Drift corrected

Four texts described the behavior this phase removed. Following the rule
that `docs/` is the current record, each got a minimal correction now; the
full rewrites (diagrams, test lists, performance figures) remain Phase 5's:

- `worktree/docs/git-graph.md`: the "merged directly after the parent took it
  indirectly" example said `fix/sniff` hangs from nothing with the notice
  (now connected at `W1` on `fix/wt-ux`'s reconstructed lane); "Continued
  after a merge" said the earlier merge is not reconstructed (now describes
  the boundary walk, the fork against `B`, and the record cutoff); the
  "No reconstruction of earlier merges" limit is replaced by what is still
  not drawn.
- `biscuit-terminal/docs/components/git_graph.md`: the **Planned** marker on
  per-source merges is removed and the "Merges" bullet and the undrawn-merge
  table row describe segments and every undrawable case.
- `.claude/skills/biscuit-terminal/components.md` and
  `.claude/skills/worktree/SKILL.md`: the "only the latest merge is drawn"
  and "continued branch is unconnected" sentences are replaced.

The worktree skill's Kitty-test description (the sparse-lanes notice) is left
for Phase 4/5, which change that test.

### Verification

- `just test` in `worktree`: 780 passed, 38 skipped (Phase 2: 770 and 30;
  +18 new test runs, +8 skips = four ignored tests on two targets).
- `just test` in `biscuit-terminal`: 3357 passed, 55 skipped (61
  `git_graph::tests`).
- `just lint` in `worktree` and `biscuit-terminal`: clean (clippy's
  `type_complexity` required the `Window`, `Classified`, and `Question`
  aliases).
- No cross-OS run in this phase: there is no path, process, or `cfg` code; the
  new shallow-clone helper reuses the existing Windows-safe `file://`
  spelling. Phase 4 runs Linux L1 as planned.

## Phase 4

Integrated evidence: the four tests Phase 3 parked are flipped and run, the
layout proof covers the new fixtures, Kitty draws the continued branch, the
L2 and perf fixtures record realistic creation commits, and the after-perf is
measured.

### Wave 4 — L1 flips (`worktree/cli/src/commands/git_graph/tests.rs`)

No `#[ignore = "flips in Phase 4"]` remains.

- `a_branch_continued_after_its_merge_is_an_unmerged_lane` is now
  `a_branch_continued_after_its_merge_draws_its_earlier_merge`: `b`'s entries
  are `[b1, b2]`, fork `d1`, merges `[(b1, merge)]`, `!incomplete`, no commit
  on two lanes; the plan (120×40, untrimmed) has no notice, `merge`'s parents
  are `[d1, b1]`, `b2`'s parent is `b1`, and the lane starts from `d1`.
- `a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment`:
  `fix/sniff`'s facts are unchanged (fork `W1`, merge `M104`, own run);
  `fix/wt-ux` now draws `W1` with merges `[(W1, M103)]` and fork `d5`; the
  plan (200×60) has no notice, draws both merges, puts `W1` on `fix/wt-ux`,
  and `fix/sniff`'s first laid-out commit hangs from `W1`. Its doc comment,
  which said no lane draws `W1`, is rewritten.
- `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`: `M103` is
  a merge from `fix/wt-ux` whose second parent is `W1`, every branch lane is
  connected (`fix/sniff` at `W1`), and the plan has no notice.
  `after_square` now measures after the **last** `+N` square.
  **Departure (density):** `feat/schema-enhancement` and `fix/sniff` keep ≥ 2
  commits after their square, but `fix/wt-ux` keeps only its tip after `+67`.
  With `W1` and `M103` drawn, `main` also draws `d6..d12` individually (the
  default lane runs down to its oldest anchor, now `fix/wt-ux`'s fork `d5`),
  so the graph is wider and 10 commits are trimmed. `trim_one_commit` folds a
  commit beside an existing square first, so `fix/wt-ux`'s recent run folds
  into `+67` before `main`'s square-less `d6..d12` run is touched. Trim order
  is out of scope (spec), so the test asserts `fix/wt-ux` keeps its tip and
  draws `W1`, with a comment giving the reason. In real Kitty (actual cell
  size, not L1's `CellSize::FALLBACK`) the same history keeps three recent
  `fix/wt-ux` commits after `+65` (see the sparse screenshot below), so the
  loss is only visible at L1's fallback measurement.
- `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`:
  - `Evidence` gains `post_merge: Vec<(child, merge source)>`: the next laid-out
    commit on the source's lane must be `child` (or a `+N` square once
    trimmed) and its only parent the source.
  - sparse lanes: merges add `(M103, d12, W1)`, `post_merge (w9, W1)`,
    `incomplete: false`; `hidden_lanes` re-measured, unchanged at `[1, 0]`.
  - new entries: E1 `AtMerge` base, focused from `fix/wt-ux`, focused from
    `fix/sniff-pr`; E1 `Behind` and `Diverged` base; merged twice (both
    merges and both post-merge parents); new branch at a merged tip with the
    record (`incomplete: true`, no merge) and without (merge `(C, p, b1)`,
    `post_merge (n1, b1)`). In the `Diverged` variant `origin/main` is its own
    lane (named, not tagged), so that tag is expected only for the other two.
  - report lines (this run):

    ```text
    sparse-lanes base 120x40: columns=118 rows=20 trimmed=14 step=34.0 natural_width=753
    sparse-lanes base 56x60: columns=144 rows=26 trimmed=21 step=34.0 natural_width=921
    continued-AtMerge base 120x40: columns=105 rows=14 trimmed=0 step=34.0 natural_width=669
    continued-AtMerge base 56x60: columns=59 rows=14 trimmed=8 step=34.0 natural_width=375
    continued-AtMerge focused wt-ux 120x40: columns=79 rows=14 trimmed=0 step=34.0 natural_width=501
    continued-AtMerge focused wt-ux 56x60: columns=59 rows=14 trimmed=4 step=34.0 natural_width=375
    continued-AtMerge focused sniff-pr 120x40: columns=79 rows=14 trimmed=0 step=34.0 natural_width=501
    continued-AtMerge focused sniff-pr 56x60: columns=59 rows=14 trimmed=4 step=34.0 natural_width=375
    continued-Behind base 120x40: columns=105 rows=13 trimmed=0 step=34.0 natural_width=669
    continued-Behind base 56x60: columns=59 rows=13 trimmed=8 step=34.0 natural_width=375
    continued-Diverged base 120x40: columns=114 rows=19 trimmed=0 step=34.0 natural_width=727
    continued-Diverged base 56x60: columns=68 rows=19 trimmed=8 step=34.0 natural_width=433
    merged-twice base 120x40: columns=74 rows=13 trimmed=0 step=34.0 natural_width=468
    merged-twice base 56x60: columns=74 rows=13 trimmed=3 step=34.0 natural_width=468
    new-at-merged-tip recorded 120x40: columns=47 rows=13 trimmed=0 step=34.0 natural_width=300
    new-at-merged-tip recorded 56x60: columns=47 rows=13 trimmed=0 step=34.0 natural_width=300
    new-at-merged-tip unrecorded 120x40: columns=54 rows=13 trimmed=0 step=34.0 natural_width=342
    new-at-merged-tip unrecorded 56x60: columns=54 rows=13 trimmed=0 step=34.0 natural_width=342
    ```

  The earlier fixtures' lines are unchanged from before this fix.
- Clippy (`cloned_ref_to_slice_refs`) required `std::slice::from_ref` in the
  `post_merge` check.

### Wave 4 — Kitty L2 (`worktree/cli/tests/level2_graph_in_kitty.rs`)

- `Fixture::record_parent(branch, parent, base_sha)` now takes the creation
  commit (E5); `Fixture::sparse_lanes` records `feat/schema-enhancement` at
  `d2`, `fix/wt-ux` at `d5`, `fix/sniff` at `W1`. A new
  `add_branch_worktree` helper creates a branch at a tip in its own worktree.
  `level2_sparse_lanes_fixture_has_the_observed_topology` now asserts each
  record's `(base_branch, base_sha)`.
- `Fixture::continued_after_merge()`: the E1 shape (`main` = `origin/main` =
  `C`), `fix/wt-ux` at `N` and `fix/sniff-pr` at `B` each in its own worktree,
  `fix/sniff-pr` recorded at `B`, `fix/wt-ux` unrecorded.
- `level2_graph_draws_a_branch_continued_after_its_merge_in_kitty` (200×60,
  base view): no "Some history is not shown", no lane left out, table intact,
  APC and reservation checks; evidence kept as `wt-graph-continued-200x60-*`.
- `level2_graph_restores_lane_density_in_kitty` now expects **no** notice.
- Run: `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 level2_graph`: 5/5 passed.
  **Checks that ran:** screen text, table rows and borders, notices, the
  transmitted APC's `c=` and PNG size, and the `CSI <rows> B` reservation
  against the text band. **Skipped:** the pixel comparison, with the
  existing warning ("the screenshot holds no window contents … Kitty had not
  drawn its window") in all five tests; this process has Screen Recording
  permission, but the captures were empty (the documented Kitty-in-background
  case). The transmitted PNGs were inspected by eye: the continued graph
  shows `fix/wt-ux` forking at `P`, `+7`, `x4..x6`, `B` tagged `fix/sniff-pr`
  merged into `C` (tagged `main`, `origin/main`), then `N`; the sparse graph
  shows `W1` merged into `M103`, `fix/sniff` forking at `W1` and merging into
  `M104`, and three recent `fix/wt-ux` commits after `+65`.
- Evidence kept (macOS `$TMPDIR`,
  `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/`):
  `wt-graph-continued-200x60-transmitted.png`,
  `wt-graph-continued-200x60-screenshot.png`,
  `wt-graph-sparse-200x60-transmitted.png`,
  `wt-graph-sparse-200x60-screenshot.png`, and the unchanged merged fixture's
  `wt-graph-merged-{100x32,56x60}-{transmitted,screenshot}.png`. The
  screenshots are blank for the reason above.

### Wave 4 — Perf and records

- `GraphFixture::observed_sparse_lanes` (`perf_support/graph.rs`) records
  `d2` (`main~13`), `d5` (`main~10`), and `W1` (`main~2^2`) instead of each
  parent's final tip (E5);
  `observed_sparse_lanes_graph_fixture_has_the_observed_topology` asserts the
  `(base_branch, base_sha)` of each record.
- `perf_graph_stages_are_reported_for_every_graph_fixture` now prints min–max
  beside each median (the P2 refinement allowed this); the `spread` helper is
  shared with the 200×60 test.
- After-measurement: same host (Apple M4 Max, macOS 27.2), `release`, 10
  samples, `WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile
  release`, run twice:

| Fixture | Baseline gather envelope (both runs) | After run 1 gather | After run 2 gather | Render (after, both runs) |
|---|---|---|---|---|
| floor (one commit) | 9.8–14.1 ms | 10.6 ms (10.2–10.6) | 11.1 ms (10.2–12.9) | 346.0 / 349.3 ms |
| ordinary | 38.7–54.9 ms | 54.2 ms (52.5–56.5) | 52.1 ms (48.8–54.6) | 354.1 / 350.1 ms |
| older essential connections | 123.5–140.4 ms | 177.2 ms (174.6–179.3) | 175.3 ms (163.3–184.1) | 351.9 / 349.6 ms |
| multiple selected branches | 59.6–72.6 ms | 74.0 ms (69.2–75.9) | 72.1 ms (68.6–81.0) | 352.5 / 351.4 ms |
| observed sparse lanes, 200×60 | 74.9–88.8 ms (one run) | 112.2 ms (103.4–118.5) | 109.5 ms (102.5–181.1) | 357.0 / 356.6 ms |

- **P2 verdict:** the floor and ordinary medians are inside the baseline
  envelope, so nothing blocks review (ordinary sits near its top; its
  per-lane cost is P1's +2 calls). Render is unchanged everywhere.
- **Explained increases (not blocking under P2):**
  - older essential connections, +~40 ms: its one lane is merged, so its
    boundary (the fork 5,000 first parents down `main`) is classified. The
    classification's `rev-list --first-parent --parents <B>..<candidate>`
    walks those thousands of commits inside Git. The number of subprocesses
    is fixed (no call per historical commit); only Git's own walk is long.
  - observed sparse lanes, +~30 ms: `fix/wt-ux` now accepts one step
    (`W1` into `M103`), which re-reads its window and classifies the next
    boundary, and every lane classifies its boundary. Call counts (base view,
    scratch recorder test, removed after the run): **24 → 40** — `log` 5 → 11,
    `rev-list` 10 → 16, `merge-base --is-ancestor` 4 → 8, `merge-base` 3 → 3,
    `for-each-ref` 1, `rev-parse` 1; `incomplete` false before and after (the
    notice it used to produce came from `GitGraph`, and is gone).
  - multiple selected branches: +2–3 ms at the median, inside noise.
- The SLA tests are recorded under Wave 5 (`just test-perf`).

### Wave 5 — Whole-area verification

- `worktree`: `just test` 788 passed, 30 skipped (Phase 3: 780 / 38; the four
  un-ignored tests now run on both targets). `just lint` clean. `just test-l2`
  (`BISCUIT_TEST_LEVEL_REQUIRED=2`) 28 passed.
- `biscuit-terminal`: `just test` 3357 passed, 55 skipped; `just lint` clean.
  `just test-l2 --no-fail-fast`: 74 of 76 passed. Two failures, neither in
  code this branch touches (it changes only `git_graph` files in
  biscuit-terminal, and neither test draws a `GitGraph`):
  - `biscuit-terminal-cli::level2 level2_prose_styling::level2_columns_word_wrap_in_pane`:
    **pre-existing, deterministic** (fails alone too). The test's capture
    includes the echoed shell command line, whose long `aa…` argument itself
    wraps, so its row filter finds rows 1 and 3. Not fixed here (out of
    scope).
  - `biscuit-terminal-cli::level2 level2_render_tree_style::level2_render_tree_style_in_wezterm`:
    **flaky**; failed after 38.7 s during the full run (while the Linux
    cross-check was also running), passed alone in 15.6 s.
- `just ci-local --plan`: a pull request executes **11** cells (biscuit-terminal
  ubuntu lint/check/L1/browser and macOS L1; worktree-cli ubuntu lint/L1/L2
  and macOS L1/L2; the test-toolkit archive-path lint guard), which is the
  6–30 band: **30–45 min**, up to about 1.5 h before treating it as stuck.
  A push to `main` adds the Windows L1 cells of both packages, and the
  nightly adds WSL2 L1.
- `just test-perf` in `worktree` (every `perf_` test, `-j 1`): first run
  failed `commands::list::tests::perf_subprocess_counts_meet_sla`, a
  subprocess-count pin of the base-view graph that Phase 3 missed (it is a
  `perf_` test, so `just test` never runs it). The recorded calls were
  exactly P1's budget: per lane, one more `--is-ancestor` and one
  `rev-list --first-parent --parents`. The pin now asserts `2 × lines`
  `--is-ancestor` and `lines` `rev-list` (other counts unchanged), with its
  comment updated. Rerun: **30 passed**, including `perf_command_sla` and the
  other wall-clock SLAs, unchanged.
- **Linux** (`just cross-check <pkg> --os linux`, build-linux, nextest
  archive produced as `ubuntu-latest`): biscuit-terminal **3033 passed, 60
  skipped**; worktree-cli **482 passed, 60 skipped**. Two environment
  failures came first, neither from this change:
  - both packages: `output file …/target/release/deps/*.rmeta is not
    writeable`. This worktree's standing clone
    (`build-linux:/home/build/coding/shazam--fix-wt-ux`) held 362 read-only,
    multiply-linked files in `target/release` (no kache wrapper on that host;
    they look copied as hardlinks from another clone), so rustc could not
    replace them when rebuilding the `ci-build` tool. The read-only files in
    that clone's `target/release` were deleted (a cache; the other links are
    untouched), and the rerun built.
  - worktree-cli: `ld terminated with signal 9 [Killed]` linking the
    `list_flags` test binary (memory; 16 GB host). A retry with the build
    mostly cached linked and passed.
- **Windows and WSL2** were not run locally. There is no `#[cfg(windows)]`,
  path, process, or file-system code in this change (Git history reading and
  in-memory layout only; the shallow-clone helpers reuse the existing
  Windows-safe `file://` spelling). Per the CI schedule, Windows L1 is proven
  on the push to `main` and WSL2 L1 on the nightly.

### Requirement-to-test mapping (spec acceptance rows)

| Acceptance row | Test(s) (all passing) |
|---|---|
| PR #105 shape: fork `P`, `B` merged into `C`, then `N`; `fix/sniff-pr` a label at `B`; `!incomplete` | `a_continued_branch_draws_its_earlier_merge_and_its_child_label_in_the_base_view`, `a_continued_branch_draws_its_earlier_merge_in_both_focused_views`; plan and layout: `gathered_graphs_lay_out_…` (`continued-AtMerge` base / focused ×2) |
| Same before `--ff` (behind; diverged `origin/main` holds the merge) | `a_continued_branch_merges_into_origin_main_before_a_fast_forward`; layout: `continued-Behind`, `continued-Diverged` entries |
| `a_branch_continued_after_its_merge_is_an_unmerged_lane` flips | `a_branch_continued_after_its_merge_draws_its_earlier_merge` |
| `observed_sparse_lanes()`: `W1` into `M103`, `fix/sniff` forks at `W1`, merges into `M104`, no notice | `a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment`, `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` (density departure above), `gathered_graphs_lay_out_…` (sparse entry), Kitty `level2_graph_restores_lane_density_in_kitty` |
| Merged twice, then continued | `a_branch_merged_twice_draws_both_merges_oldest_first`; layout `merged-twice` entry (both merges, both post-merge parents); component `a_lane_merged_twice_and_continued_draws_both_merges_in_source_order` |
| New branch at an already merged tip | `a_branch_created_at_a_merged_tip_does_not_claim_the_old_merge`, `fork_origin_cutoff_matrix`; layout `new-at-merged-tip recorded / unrecorded` |
| Source with two children, before and after the merge | component `children_forked_before_and_after_the_merge_source_hang_from_their_own_commits`, `a_child_forked_at_the_merge_source_is_declared_before_the_merge` |
| Merge into a sibling lane that later merges | component `a_merge_into_a_sibling_that_merges_into_the_default_lane_emits_each_source_first`, `a_cycle_of_merges_draws_one_and_reports_the_other` |
| `B` reached the default branch through another merge | `a_boundary_integrated_through_another_merge_is_not_reconstructed` |
| Shallow boundary crossing the cutoff | `a_shallow_boundary_invents_no_merge_and_keeps_the_verified_one` (depth 1) |
| Git fails on an older boundary | same test (depth 2: first edge kept, fork unknown, `incomplete`) |
| Ordinary unmerged branch: identical facts | `an_ordinary_unmerged_branch_gathers_unchanged_facts`; call pins `the_base_view_gives_every_worktree_branch_a_line`, `graph_and_verbose_share_one_merge_base`, `perf_subprocess_counts_meet_sla` |
| Layout: no overlaps, tags on SHAs, exact merge and post-merge parents | `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags` (`post_merge` added) |
| `GitGraph` component: mid-lane merge, forks before/after, destination hidden by the cap | biscuit-terminal `a_lane_merged_from_its_middle_pauses_at_the_source_and_resumes_after_the_merge`, `the_height_cap_keeps_a_mid_lane_merges_destination_lane_with_its_source`, `the_height_cap_hides_a_source_lane_and_its_destination_lane_together` |
| Kitty: continued graph connected, no notice, screenshot kept | `level2_graph_draws_a_branch_continued_after_its_merge_in_kitty` (pixel check skipped, see above; PNGs kept) |
| Performance: no network, no call per historical commit, no repeated classification, SLAs pass | perf table and call counts above; `just test-perf` 30 passed; G4 cache and P1 pins from Phase 3 |

### Tier placement of added and renamed tests

- `a_branch_continued_after_its_merge_draws_its_earlier_merge` and the four
  flipped tests: unit tests in `commands::git_graph::tests`, no tier marker,
  compiled by the lib and bin targets; `just test` ran each twice.
- `level2_graph_draws_a_branch_continued_after_its_merge_in_kitty`: in the
  existing `level2_graph_in_kitty` binary (`terminal-tests` feature, which
  `worktree-cli`'s `test-l2` recipe enables); `level2_` selects L2, and the
  run above executed it.
- No test reads a new repository file.

### Docs and skills

- `.claude/skills/worktree/SKILL.md` (drift left by Phase 3 for this phase):
  the sparse-lanes Kitty test now expects no notice; the new continued Kitty
  test and `record_parent(branch, parent, base_sha)` are described; the L1
  layout proof's sparse fixture is complete and has `post_merge`; the density
  sentence records the `fix/wt-ux` exception; the perf fixture's records hold
  creation commits. Phase 5 still owns the full rewrite.
- `worktree/docs/` has no statement about these tests' notices, so no docs
  page changed in this phase. The perf figures go into
  `performance-testing.md` in Phase 5, as planned.
- Frontmatter: `human_review: false`. The one surprise, the L1 density
  exception, comes from trim order (explicitly out of scope) and does not
  appear at Kitty's real cell size, so it does not block Phase 5.

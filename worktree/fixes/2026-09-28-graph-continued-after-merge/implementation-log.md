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

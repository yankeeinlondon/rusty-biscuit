---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-27-graph-merged-branch/spec.md
plan: worktree/fixes/2026-09-27-graph-merged-branch/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - worktree/cli/tests/perf_support/graph.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/perf_graph_stages.rs
    - .config/nextest.toml
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/git-discovery.sh
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/render-topologies/src/main.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/render-topologies/run.sh
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/worktree/SKILL.md
packages:
    - worktree-cli
---

# Implementation Log for 2026-09-27-graph-merged-branch (5 phases)

## Phase 1

### Rulings (recorded 2026-09-27)

Copied from the plan's "Necessary Rules". They bind every later phase. An
amendment made by a spike is listed under "Amendments" below the spike results;
where an amendment and a rule disagree, the amendment wins.

- **R1 — Ownership.** `biscuit-visualized` owns two generic gitGraph
  corrections inside `compute_layout`: repairing the merge second parent, and
  collision-free tag spacing. `biscuit-terminal`'s `GitGraph` owns lane
  semantics: explicit tips, merge destinations, pinning essential commits
  during trimming, never substituting an undrawn commit, and the
  incomplete-history notice. `worktree-cli` owns Git discovery: branch
  classification, first-parent lanes, and compressed anchor entries. No
  dependency upgrade, fork, or patch.
- **R2 — Merge repair mechanism (no new API).** After `parse_mermaid`, for each
  gitGraph commit with `commit_type == Merge` and exactly one parent, recover the
  source lane from the merge statement's suffix, available as the IR message
  `merged branch {from} into {branch}`. Match it against the declared
  `gitgraph.branches` names: it must equal a name, or begin with a name followed
  by whitespace and one of `id:`, `tag:`, `type:`; exactly one name may match.
  The second parent becomes the last commit with `branch == from` and
  `seq < merge.seq`. A merge with two or more parents is left alone (no-op once
  the dependency fixes its parser). An unresolvable or ambiguous source, or a
  source lane with no earlier commit, is an explicit `MermaidError::RenderFailed`,
  never silently skipped (mermaid.js refuses to merge an empty branch). A unit
  test pins the dependency's message format. The typed merge-metadata
  alternative is rejected.
- **R3 — Tag spacing policy.** For a gitGraph whose layout has at least one tag
  with `transform == None`, `compute_layout` lays out twice; the second pass sets
  `commit_step = max(default commit_step, widest tag polygon width + TAG_GAP)`.
  `TAG_GAP` equals the theme's `font_size` (one em), is a named constant, and is
  documented as policy, not a measured margin. With no tags, or rotated tags,
  layout is single-pass and unchanged, and the limitation is documented.
  `MERMAID_BACKEND` moves from `mermaid-rs-renderer@0.2.x+bv1` to
  `mermaid-rs-renderer@0.3.x+bv2`.
- **R4 — Branch classification (worktree-cli).** For each selected branch `B`
  with tip `T`, destination candidates `X` are tried in order: (1) the recorded
  parent's lane, when drawn; (2) the default lane tip; (3) the diverged
  `origin/<default>` line's tip, when drawn. The first `X` with
  `merge-base --is-ancestor T X` decides. `C` is the oldest line of
  `git rev-list --first-parent --ancestry-path --parents T..X`.

  | Class | Condition | Drawn as |
  |---|---|---|
  | Unmerged | No candidate contains `T` | Lane of `--first-parent T --not <fork-lane tip>`, fork = `merge-base(fork-lane tip, T)` |
  | Merged directly | `C` has a non-first parent equal to `T` | Lane of `--first-parent T --not C^1`, fork = `merge-base(<recorded parent tip, else C^1>, T)`, `merged_into(C)` |
  | No separate history | `T` is on `X`'s first-parent chain (`C` would be `T`) | Label at `T` on `X`'s lane (anchored if outside the window) |
  | Integrated otherwise | Contained in `X` but neither of the above | Lane of `--first-parent T --not C^1` with its fork, no merge edge, and the incomplete-history notice |
  | Unknown | Any required Git command failed or returned unparseable output | Draw what is verified, mark the connection unknown, show the notice |

  Tip equality is never an input to classification. A merged branch's fork is
  computed against `C^1` (or the recorded parent), never the default tip.
- **R5 — First-parent lanes.** Every lane is its tip's first-parent chain down to
  its fork. `log`/`rev-list` for lane entries and `+N` counts pass
  `--first-parent`. Verbose gathering (`commit_details_since`) keeps its current
  non-first-parent semantics.
- **R6 — Essential anchors and compression.** A lane's anchors are fork commits
  of lanes hanging from it, merge destinations on it, label commits for "no
  separate history" branches, and the local and `origin/<default>` ref commits
  (default lane). Anchors are always `LaneEntry::Commit`; runs between the
  ordinary window and anchors become `LaneEntry::Elided(n)`. Windows unchanged:
  `LINE_WINDOW` 5, `BASE_DEFAULT_WINDOW` 10, `CONTEXT_COMMITS` 2. An anchor's
  position comes from `rev-list --first-parent --count <anchor>..<tip>`,
  verified with one batched `rev-parse <tip>~<d>…`; an anchor that fails
  verification is handled by R8, never placed anyway.
- **R7 — `GraphLine` / `GitGraph` API.** `GraphLine::with_tip(sha)` sets the
  branch's own tip; label placement uses `tip_sha`, then the last drawn commit,
  never `fork_sha`. `GraphLine::merged_into(sha)` sets the destination merge
  commit, whose lane is found by position. `fork_sha == None` means "connection
  unknown". `GitGraph::with_incomplete_history()` reports caller gaps.
  `GitGraphPlan` gains `incomplete: bool`. Existing builders keep their spelling.
- **R8 — Undrawn things are accounted for, never substituted.** A tag or label
  whose commit is not drawn sets `incomplete`. A lane whose fork is not drawn (or
  unknown) is emitted disconnected if S1 confirms the renderer supports it;
  otherwise it is omitted and its tip label placed if drawn. Either way
  `incomplete` is set. A merge whose destination is not drawn, or would be
  emitted before the merged lane's commits, is not drawn and sets `incomplete`.
  The notice is one dim `Prose` line, "Some history is not shown", after the
  "N more worktrees not shown" line.
- **R9 — Unavailable history.** Gathering helpers return
  `Result<Option<T>, GatherGap>`: `Ok(None)` is a definitive "no", `Err` is
  unknown. An unknown result drops only the affected connection and sets
  `GraphFacts::incomplete`; it never drops the whole graph. An unparseable git
  line is a gap, never filtered out. No fetch and no new network request.
- **R10 — Label-only exception.** Applied only when classification positively
  establishes "No separate history"; never on failure, never on tip equality.
- **R11 — Input Robustness Matrix: not applicable.** No new reader of a file
  format, manifest, lockfile, or configuration; Git plumbing output falls under
  R9.
- **R12 — Validation dimensions.** Long labels
  `feature/very-long-exact-branch-reference-alpha` and
  `origin/very-long-exact-branch-reference-beta`, plus `main`/`origin/main`, plus
  `PR #104 → main`. Viewports 120×40 and 56×60. Explicit width `--width 40`.
- **R13 — Evidence split.** Portable L1 (real-Git gathering, `GitGraph` output,
  `biscuit-visualized` layout geometry from the rendering `Layout`); macOS-only
  Kitty screenshot and PNG inspection. Mermaid-text snapshots never count as
  overlap evidence.

### Spike harnesses

All three live under `spikes/` and are never compiled into the workspace.

- `spikes/render-topologies/` (S1, S2): `src/main.rs` plus `run.sh`, which
  builds it as a standalone crate in a temp directory (no Cargo manifest is kept
  in the repository, so no tooling mistakes it for a package). The crate uses
  the repository's `Cargo.lock`, a path dependency on `biscuit-terminal` (for
  real `GitGraph::mermaid()` output), and `mermaid-rs-renderer =0.3.1`. It
  prototypes R2 (`repair`) and R3 (`layout_spaced`) against the renderer's
  public IR. It asserts parents, lanes, and tag membership, counts pairwise
  tag-polygon intersections from the same `Layout` that renders, and writes
  PNGs. Reproduce with
  `worktree/fixes/2026-09-27-graph-merged-branch/spikes/render-topologies/run.sh <out dir>`.
  A rerun reproduced `results.txt` byte for byte. Output:
  `spikes/render-topologies-output/results.txt` plus 30 PNGs (`.svg` and `.mmd`
  pruned, since the harness embeds every source).
- `spikes/git-discovery.sh` (S3) builds each topology in a temp directory
  with fixed timestamps and prints exit codes, stdout (short SHAs), stderr,
  classifications, anchor verifications, and timings. Transcripts:
  `spikes/git-discovery-{macos,linux,windows}.txt`. The macOS transcript predates
  the last line added to the script (`anchor side~3`), which the Linux and
  Windows transcripts include and which verifies on all three.

### S1 — Renderer topologies (result: R8's disconnected-lane option confirmed; R2 refined)

| Topology | Parsed parents (raw) | After R2 prototype | Layout arrows = parent edges | Visual |
|---|---|---|---|---|
| Disconnected lane (`branch lost` before root's first commit) | `L1 = []`, lane `lost` | unchanged | 3 = 3 | Lane floats on its own row, no fabricated edge (`s1-disconnected-fixed.png`) |
| Nested lane merged into root (`fix/sniff` shape) | `M103 = [D1]`, `M104 = [M103]` | `M103 = [D1, W2]`, `M104 = [M103, S1]` | 8 = 8 | Both merges join at the labeled commits (`s1-nested-into-root-fixed.png`) |
| Merge into a non-root lane | `PM = [P1]` | `PM = [P1, C2]`, lane `parent` | 7 = 7 | Correct |
| Merge commit that is also a fork point | `M = [A]`, `L1 = [M]` | `M = [A, S1]` | 5 = 5 | Correct |
| `+N` squares on both sides of a merge | `M = [+7]` | `M = [+7, T]`, `+3 ` after `M` | 7 = 7 | Correct |
| Unlabeled merge | `[A, S1]` | untouched (two parents) | 3 = 3 | Correct |
| Merge into a lane with no head (`merge side` first) | `[S1]` | left alone (single parent already is the source tip) | 2 = 2 | Correct |

Error cases through the prototype: an unknown lane, an empty source lane, and a
genuine ambiguity are all errors; none are silently skipped.

Renderer facts learned from its source (`parser.rs::parse_gitgraph_diagram`):

- `merge <suffix>` looks up the *whole* suffix as the source lane. When neither
  lane has a head, the parser drops the merge statement entirely (no commit).
  `GitGraph` never emits that shape.
- The IR message is exactly `merged branch {suffix} into {current lane}`, so
  the suffix is recovered by stripping the prefix and the `into {commit.branch}`
  tail (`rsplit_once`, since a suffix can itself contain ` into `).
- A lane with no head (declared before any root commit) gives its first commit
  `parents = []`, which the layout draws as an unconnected row.

### S2 — Tag geometry on real `GitGraph` output (result: R3 stands, no lane-spacing term)

Fixtures emitted through `GitGraph::mermaid()`: cross-lane long labels (R12
names, `main`/`origin/main` one apart, `PR #104 → main`), a four-tag stack beside
a neighbor's long tag, tags beside `+N` squares on two lanes, and a base view
with three lanes whose tips each carry a three-tag stack. Each ran on the light
(`mermaid_default`, font 16) and dark (`modern`, font 14) theme geometry, with
`TAG_GAP = theme.font_size`.

| Fixture | Default overlaps (light/dark) | R3 overlaps (light/dark) | R3 commit step (light/dark) | Width default → R3 (light) |
|---|---|---|---|---|
| cross-lane | 2 / 2 | 0 / 0 | 234.7 / 246.5 | 656 → 1,860 |
| stack | 1 / 1 | 0 / 0 | 246.7 / 258.5 | 252 → 854 |
| `+N` squares | 0 / 0 | 0 / 0 | 97.4 / 103.4 | 656 → 1,036 |
| base lanes | 1 / 1 | 0 / 0 | 236.7 / 246.0 | 801 → 3,030 |

- Zero tag-polygon intersections across all fixtures and both themes,
  cross-lane pairs included. Tags stay on their original commit IDs.
- After spacing, no tag polygon covers another commit's dot either (not
  required; recorded because the default layout had several).
- Cost: one global `commit_step` from the widest tag makes diagrams up to 3.8×
  wider (base lanes 801 → 3,030 units). Trimming and shrinking will fire more
  often. Phase 2's real-backend proof and Phase 4's 56-column evidence must look
  at `trimmed_commits` and image scale, not only overlaps. PNG inspected:
  `s2-base-lanes-light-spaced.png` shows separate stacks at a small scale.

### S3 — Git discovery (result: R4's command replaced, R9 gains a shallow rule, R6 confirmed)

Git versions: macOS 2.55.0 (this host), Linux 2.47.3 (`build-linux`), native
Windows 2.55.0.windows.3 (`build-win-native`, Git Bash). Classifications, error
shapes, and shallow behavior are **identical** on all three once SHAs are
normalized. The CI `ubuntu-latest` git version was not measured directly;
`build-linux` stands in for it.

| Topology | Classification (corrected method) | Plan's `--first-parent --ancestry-path` oldest line |
|---|---|---|
| Observation 1 (`fix/wt-ux` vs `origin/main`) | Merged directly at the merge commit | same |
| Default moved on before the merge | Merged directly (C is not the chain's oldest `T..X` commit) | same |
| Observation 2, `fix/sniff` vs parent `fix/wt-ux` | Not contained (next candidate) | — |
| Observation 2, `fix/sniff` vs `origin/main` | Merged directly at M104 | same |
| Observation 2, `fix/wt-ux` vs `origin/main` | Merged directly at M103 | same |
| Merged into non-default parent | Merged directly on the parent lane | same |
| Fast-forward | No separate history (C's first parent is T) | same |
| Equal tips (D at merged C's tip, vs lane C) | No separate history (T is X) | empty |
| Continued after merge | Not contained → Unmerged, fork at the old merged tip | — |
| Indirect integration | **Integrated otherwise** | **empty** (would read as "no separate history") |

Also observed: the plain `git log -10 origin/main` that `base_view` uses today
fills all ten slots with `fix/sniff`'s commits, and `--first-parent` gives
`Merge #104, Merge #103, d1, root`, which reproduces the second observation.

Timing on a 9,000-commit first-parent chain (fork 5,000 back, merge 3,000 back),
per command, 3 rounds:

| Command | macOS | Linux | Windows |
|---|---|---|---|
| `merge-base --is-ancestor` | 20 ms | 13 ms | 71 ms |
| `rev-list --first-parent --ancestry-path --parents T..X` | 22 ms | 13 ms | 72 ms |
| `merge-base` | 15 ms | 8 ms | 60 ms |
| `rev-list --first-parent --count A..tip` | 21 ms | 13 ms | 67 ms |
| `rev-parse tip~5003 tip~3000 tip~0` (batched) | 21 ms | 13 ms | 70 ms |
| `log --first-parent -10` | 11 ms | 5 ms | 58 ms |

Windows costs 45–60 ms more per git process (spawn overhead), so Phase 3 should
keep the number of git calls per lane small and use the planned
`std::thread::scope` stages.

Missing-history error shapes (shallow `--depth 1` and `--shallow-since`
clones from `file://`; the same on every OS):

| Command | Exit | Output | Meaning |
|---|---|---|---|
| `rev-parse --is-shallow-repository` | 0 | `true` | detectable |
| `merge-base --is-ancestor T X` across the boundary | **1** | none, no stderr | **indistinguishable from a real "no"** |
| `merge-base A B` across the boundary | **1** | none, no stderr | **indistinguishable from unrelated histories** |
| `rev-list --first-parent --ancestry-path T..X` | 0 | empty | silently truncated |
| `rev-list --first-parent --count T..X` | 0 | `1` (true value larger) | silently truncated |
| `rev-parse tip~3`, `tip^2` beyond the boundary | 128 | `unknown revision` on stderr | error |
| `cat-file -e <missing>^1` | 128 | stderr | error |

### Amendments (2026-09-27, binding on later phases)

- **R4-A1 — How `C` is found.** `git rev-list --first-parent --ancestry-path
  --parents T..X` is **rejected**: it computes ancestry over first-parent edges
  only and returns nothing when `T` reached `X` through another branch's merge
  (indirect integration), which would then be misread. Instead:
  1. `chain = rev-list --first-parent --parents T..X` (X's first-parent chain,
     newest first, with parents);
  2. `descendants = rev-list --ancestry-path T..X` (every commit descending from `T`);
  3. `C` is the last entry of the chain's leading run of commits in
     `descendants`. Containment is monotone along a first-parent chain, so the
     walk stops at the first commit not in `descendants`.

  Outcomes: `C^1 == T` means no separate history. `T` among `C`'s other parents
  means merged directly. Otherwise the class is integrated otherwise. An empty
  chain when `is-ancestor` is true means `T` is `X`'s own tip, which is the "no
  separate history" class. That identity is a consequence of containment, not an
  input, so R10 holds. A non-empty chain with no descendant at its head is
  impossible when `is-ancestor` holds, and is treated as a gap (Unknown). Both
  lists are sized by `T..X`, about 20 ms for 3,000 commits on macOS.
- **R9-A1 — Shallow repositories.** Gathering reads
  `rev-parse --is-shallow-repository` once. A failure there is a gap. In a
  shallow repository:
  - Every negative or empty answer is a `GatherGap`, never `Ok(None)`: `is-ancestor`
    exit 1, `merge-base` exit 1, an empty ancestry or chain, an anchor that fails
    verification.
  - Every `+N` count is a lower bound and sets `incomplete`.
  - Positive answers stay trusted, because the objects they name exist.

  Errors are classified by exit code, never by stderr text.
- **R2-A1 — Merge repair refinements.**
  - (a) A merge whose single parent already is the source lane's last earlier
    commit is left alone. This is the parser's shape for `merge side` into a lane
    with no head; R2 as written would add a duplicate parent.
  - (b) The source is recovered as the message minus the `merged branch ` prefix
    and the `into {commit.branch}` tail.
  - (c) The Phase 2 test "ambiguous prefix (`feat` vs `feat x`) is rejected" is
    wrong. Under R2's rule, `merge feat x id: "M"` matches only `feat x`,
    because `feat` is followed by `x`, not a key. The test becomes "`feat` vs
    `feat x` resolves to `feat x`". The rejection test uses a genuine ambiguity:
    lanes `x` and `x tag: "t"` with `merge x tag: "t" id: "M"`, where both
    names match.
- **R8-A1 — Disconnected lanes.** Confirmed renderable. Emit `branch <lane>`
  before the root lane's first commit (the lane has no head, so its first commit
  gets no parent), then `checkout <lane>` where its commits belong. Lane order is
  still the order of `branch` statements, so a disconnected lane's row position
  follows its declaration.
- **R3 — unchanged.** No lane-spacing term is needed (S2: zero cross-lane overlaps).
- **R6 — confirmed.** `rev-list --first-parent --count A..tip` followed by a
  batched `rev-parse tip~d` verification places on-chain anchors exactly (fork
  5,000 back and merge 3,000 back verified). Off-chain anchors fail verification
  as intended: a merged branch's tip, or `merge-base(default, T)` once `T` is
  merged (which *is* `T`).

### Baseline performance (before)

Recorded 2026-09-27 on this host (Apple M4 Max, macOS, git 2.55.0) at
`83485b24e` plus only this phase's test-side changes. Release profile, 10
samples per fixture, one warm-up run each, medians:

```sh
WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release
```

| Fixture | Samples | `graph gather` (median) | `graph image render (biscuit-terminal)` (median) |
|---|---|---|---|
| floor (one commit) | 10 | 5.9 ms | 352.4 ms |
| ordinary (3 worktrees, 50 commits, base view) | 10 | 20.2 ms | 351.2 ms |
| older essential connections (fork 5,000 back, merge 3,000 back, focused view of the merged branch) | 10 | 69.5 ms | 353.3 ms |
| multiple selected branches (8 worktrees, 2 merged, base view) | 10 | 32.4 ms | 351.8 ms |

How the numbers were obtained, and what they include:

- `wt` gathers and renders the graph only when stderr is a terminal and
  `TERM_PROGRAM` names an image-capable emulator. The test
  (`worktree/cli/tests/perf_graph_stages.rs`,
  `perf_graph_stages_are_reported_for_every_graph_fixture`) therefore runs
  `wt list --perf` through `script` in a 120×40 pseudo-terminal.
- It emulates **Ghostty**, not Kitty. A pseudo-terminal never answers
  terminal queries, and every Kitty-protocol emulator except Ghostty and Warp
  runs a cursor-position query (1 s timeout) for scroll compensation
  (`biscuit-terminal` `terminal_image::cursor::needs_scroll_compensation`).
  Under Kitty emulation the render stage read about 1.4 s. `--perf` prints
  durations of 1 s or more with one decimal (`1.4s`), a 100 ms resolution that
  hid every difference between fixtures. Under a second it prints tenths of a
  millisecond.
- The render stage still carries about 350 ms of fixed cost in a pseudo-terminal
  (terminal detection that waits for answers that never come). The floor row
  measures it: compare each row against the floor and against its own later
  median. `pre-dispatch` (about 2.1 s, not part of either stage) is the same
  detection cost at startup.
- The first attempt, with Kitty emulation, recorded render medians of 1.4 s on
  every fixture. It is superseded for the resolution reason above.
- The fixture builders are `worktree/cli/tests/perf_support/graph.rs`
  (`GraphFixture::{floor, ordinary, older_connections, multiple_selected, all}`).
  They are built through `git fast-import` with fixed timestamps and verify
  their own topology (fork at `main~5000`, merge at `main~3000`, two merges and
  eight worktrees). Phase 4 reruns the same command unchanged.
- The test asserts only that both stages are reported on every fixture, with no
  numeric threshold (spec: no new numeric budget). It is `#![cfg(unix)]`, since
  `script` is Unix-only. The util-linux form (`script -qec … /dev/null`) was
  checked on `build-linux` (a 40×120 pty with stdin at `/dev/null`). CI never
  runs `perf_` tests: `worktree-cli`'s L1 drops `perf_` (`scripts/ci/consolidation.py`).
  They run only through `just test-perf` / `just all`.
- `.config/nextest.toml` gains a slow-timeout override (60 s × 5) for this test.
  The default single-sample run needs about 12 s and the recorded 10-sample run
  113 s, both near or above the default 30 s ceiling.

### Checkpoint 1

- Every spike result is recorded above. R2, R4, R8, and R9 are amended
  (R2-A1, R4-A1, R8-A1, R9-A1); R3 and R6 are confirmed unchanged.
- The baseline table exists (above).
- No production source changed. Everything outside this fix directory is test
  infrastructure:
  - `worktree/cli/tests/perf_support/graph.rs` (new fixture builders), plus
    `pub mod graph;` in `perf_support/mod.rs`;
  - `worktree/cli/tests/perf_graph_stages.rs` (new `perf_` measurement test);
  - a nextest slow-timeout override for that test in `.config/nextest.toml`;
  - one bullet in `.claude/skills/worktree/SKILL.md` on how graph-stage
    timings are measured.

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no product behavior, so there is no regression test to write.
The evidence is:

| Phase 1 requirement | Evidence |
|---|---|
| Renderer supports the topologies `GitGraph` will emit (S1) | `spikes/render-topologies` assertions (`parents_of`/`lane_of` on every topology; arrows = parent edges) and `render-topologies-output/s1-*.png`, inspected |
| Tag spacing removes every overlap, cross-lane included (S2) | `render-topologies-output/results.txt`: 0 overlaps over 4 fixtures × 2 themes; `s2-base-lanes-light-spaced.png` inspected |
| Git discovery commands and error shapes per OS (S3) | `spikes/git-discovery-{macos,linux,windows}.txt`, identical after SHA normalization |
| Baseline fixtures reproduce their stated shapes | `GraphFixture::{older_connections, multiple_selected}` assert fork/merge distances, merge count, and worktree count at build time |
| Graph stages are measured on every fixture | `perf_graph_stages::perf_graph_stages_are_reported_for_every_graph_fixture` (passes; `perf_` tier, run by `just test-perf`) |

Gates run:

- `just lint` in `worktree`: pass. `cargo clippy -p worktree-cli --test
  perf_graph_stages --test perf_command_sla -- -D warnings`: pass.
- `just test` in `worktree`: 712 passed, 29 skipped (tier-filtered `perf_` and
  L2 tests), exit 0.
- `just test-perf perf_graph`, dev and release profiles: pass.
- `just check-tier-coverage worktree`: 0 stranded.

Skipped or not applicable:

- No cross-OS run of the new test: it is Unix-only by construction, and CI never
  runs `perf_` tests. The Linux `script` syntax was verified directly on
  `build-linux`.
- `biscuit-terminal` and `biscuit-visualized` gates were not run: neither
  package changed.
- No pre-existing failures were seen.

### Notes for Phase 2

- Use the amended R2 (R2-A1) in `repair_gitgraph_merges`. Change the test list
  item "an ambiguous prefix (`feat` vs `feat x`) is rejected" to: `feat` vs
  `feat x` resolves to `feat x`; the lanes `x` and `x tag: "t"` with
  `merge x tag: "t" id: "M"` are rejected as ambiguous; a merge whose single
  parent is already the source tip is untouched.
- `spikes/render-topologies/src/main.rs` (`repair`, `layout_spaced`, `Rect`, and
  the overlap and dot counters) is a working prototype of R2, R3, and the
  planned `tag_boxes(&Layout)` helper.
- R3's global step widens diagrams up to 3.8×. Watch `trimmed_commits` and image
  scale in the Wave 3 real-backend proof.

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
source_files_during_phase_2:
    - biscuit-visualized/src/src/mermaid/gitgraph.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/tests/gitgraph_tests.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-terminal/lib/src/components/git_graph.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/phase2-renders/src/main.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/phase2-renders/run.sh
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
    - worktree/lib/src/git.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/topology.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/src/commands/list/tests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_4:
    - Cargo.lock
    - worktree/cli/Cargo.toml
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/level2_graph_in_kitty.rs
docs_updated_during_phase_4:
    - worktree/docs/performance-testing.md
    - docs/dependencies.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_5:
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
docs_updated_during_phase_5:
    - worktree/docs/git-graph.md
    - worktree/docs/cli/list.md
    - biscuit-terminal/docs/components/git_graph.md
    - biscuit-visualized/README.md
docs_created_during_phase_5:
    - biscuit-visualized/docs/mermaid-gitgraph.md
skills_files_updated_during_phase_5:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
    - .claude/skills/biscuit-terminal/components.md
    - .claude/skills/os/macos.md
packages:
    - worktree-cli
    - biscuit-visualized
    - biscuit-terminal
    - worktree
source_code:
    - worktree/cli/tests/perf_support/graph.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/perf_graph_stages.rs
    - .config/nextest.toml
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/git-discovery.sh
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/render-topologies/src/main.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/render-topologies/run.sh
    - biscuit-visualized/src/src/mermaid/gitgraph.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/tests/gitgraph_tests.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-terminal/lib/src/components/git_graph.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/phase2-renders/src/main.rs
    - worktree/fixes/2026-09-27-graph-merged-branch/spikes/phase2-renders/run.sh
    - worktree/lib/src/git.rs
    - worktree/cli/src/commands/git_graph/topology.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/src/commands/list/tests.rs
    - Cargo.lock
    - worktree/cli/Cargo.toml
    - worktree/cli/tests/level2_graph_in_kitty.rs
documentation:
    - worktree/docs/performance-testing.md
    - docs/dependencies.md
    - worktree/docs/git-graph.md
    - worktree/docs/cli/list.md
    - biscuit-terminal/docs/components/git_graph.md
    - biscuit-visualized/README.md
    - biscuit-visualized/docs/mermaid-gitgraph.md
completed_phase: 5
implemented: true
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

## Phase 2

Shared rendering layers, 2026-09-27. No Git discovery changed beyond one
bridging line in `worktree-cli` (see "Deliberate early touch").

### 2a — `biscuit-visualized` (merge repair, tag spacing, cache identity)

- New private module `biscuit-visualized/src/src/mermaid/gitgraph.rs`:
  - `repair_gitgraph_merges(&mut Graph)` implements R2 + R2-A1 exactly: the
    source lane is the IR message minus `merged branch ` and the
    `into {commit.branch}` tail (`rsplit_once`), matched against declared lanes
    (equal, or name + whitespace + `id:`/`tag:`/`type:`); exactly one match;
    the second parent is the lane's last commit with a smaller `seq`. Two or
    more parents, or a single parent that already is the source tip, are left
    alone. No match, several matches, or an empty source lane is
    `MermaidError::RenderFailed`. The plan named the argument `ParsedGraph`;
    the renderer's type is `mermaid_rs_renderer::Graph` (`ParseOutput.graph`).
  - `layout(&Graph, &Theme)` implements R3: a second pass with
    `commit_step = max(default, widest tag + TAG_GAP_EM × theme.font_size)`.
    `TAG_GAP_EM = 1.0` is the named policy constant (one em), since the gap
    must follow each theme's resolved font size, including `%%{init}%%` and
    `point_label_font_size` overrides.
  - `MermaidDiagram::compute_layout` (shared by `natural_size` and
    `render_svg`) now parses → repairs → applies theme overrides → `layout`.
  - `#[doc(hidden)] MermaidDiagram::gitgraph_geometry()` returns the laid-out
    commits (repaired parents, lane, tag bounds) with `tag_boxes()` and
    `tag_overlaps()`. Chosen over a `test-support` feature to match existing
    practice (`#[doc(hidden)]` surfaces in `biscuit-terminal`); it lets
    `biscuit-terminal` prove placement against the layout that renders.
- `MERMAID_BACKEND` is now `mermaid-rs-renderer@0.3.x+bv2`; its doc and
  `render_options_json`'s doc state the repair and spacing add no key input.

**R3-A1 (amendment, binding).** R3 read "`transform == None` (horizontal
orientation)". That equivalence is false: 0.3.1 draws unrotated tags in
vertical (`TB`/`BT`) graphs too, and the first run spaced a `TB` graph by tag
width along its vertical axis. Spacing now also requires the layout direction
to be `LR` or `RL`; vertical graphs and any rotated tag keep the single pass.
Also learned: 0.3.1 ignores a direction written in the header
(`gitGraph TB:`); only a separate `TB` or `direction TB` line counts.

**R2-A2 (renderer fact, binding on `GitGraph`).** The parser *drops* a labeled
`merge` into a lane that has no head yet (it finds neither the suffix lane nor
a current head), so the repair never sees it. `GitGraph` therefore emits a
merge only where its lane already has a commit (never as the root lane's first
statement or an unconnected lane's first statement).

Tests (`biscuit-visualized/src/src/tests/gitgraph_tests.rs`, L1, all-features):

| Behavior | Test |
|---|---|
| Message format pinned (and 0.3.1 loses the second parent) | `the_parser_message_still_carries_the_whole_merge_suffix` |
| Labeled merge gets `[dest head, source tip]`; tags and lane kept; next commit's parent unchanged | `a_labeled_merge_gets_its_source_tip_as_second_parent` |
| Unlabeled merge and a two-parent merge untouched | `a_merge_the_parser_already_resolved_is_untouched` |
| Single parent already the source tip untouched; labeled merge into a headless lane is dropped by the parser | `a_merge_into_a_lane_without_a_head_keeps_its_single_source_parent` |
| Nested-lane source (`fix/sniff` shape), both merges | `a_nested_lane_merges_into_the_root_lane` |
| Merge into a non-root lane | `a_merge_into_a_non_root_lane_is_repaired` |
| `feat` vs `feat x` resolves to `feat x` (R2-A1c) | `a_lane_name_that_prefixes_another_resolves_to_the_exact_name` |
| Genuine ambiguity (`x` / `x tag: "t"`) rejected | `a_suffix_that_names_two_lanes_is_rejected` |
| Unknown lane rejected | `a_merge_from_an_unknown_lane_is_rejected` |
| Empty source lane rejected | `a_merge_from_an_empty_lane_is_rejected` |
| Rejection surfaces through public `natural_size` and `gitgraph_geometry` | `measurement_and_geometry_report_a_rejected_merge` |
| Geometry carries repaired parents and lanes; `None` for other diagrams | `geometry_carries_the_repaired_parents`, `geometry_is_none_for_other_diagrams` |
| R12 long labels on neighbors: 0 overlaps, tags on original IDs (both themes; control: default layout overlaps) | `long_labels_on_neighboring_commits_do_not_overlap` |
| `main`/`origin/main` one apart (control overlaps) | `main_and_origin_main_one_commit_apart_do_not_overlap` |
| Three-tag stack beside neighbors (control overlaps) | `a_stack_of_three_tags_clears_its_neighbors` |
| Cross-lane tags (control overlaps) | `tags_on_different_lanes_do_not_overlap` |
| No-tag layout equals the single pass | `a_diagram_without_tags_keeps_the_single_pass_layout` |
| Vertical graphs not spaced; `LR` is (R3-A1) | `a_vertical_graph_with_tags_keeps_the_single_pass_layout` |
| Measured size is the spaced layout | `the_spaced_layout_is_the_measured_one` |
| Cache key changes with the backend; value pinned | `cache_tests::cache_key_different_mermaid_backend` |

### 2b — `biscuit-terminal` `GitGraph`

- `GraphLine` gains `tip_sha`/`with_tip` and `merged_into`/`merged_into()`.
  `tip()` is `tip_sha`, else the newest commit, never `fork_sha`. The
  `fork_sha` doc ("a line with no commits of its own has its tip here") was
  drifted by this change and now says `None` is an unknown connection.
- `attach_point` returns `None` instead of the parent-lane or default-lane
  start. Such lanes (and lanes closing an attach cycle) are **unconnected**
  (R8-A1): declared as `branch <lane>` before the root lane's first commit,
  followed by `checkout main`, and their commits are emitted after the root
  lane's first entry, so their first commit has no parent.
- `emit_lane` emits `merge <lane> id: "<id>"` plus tags for a commit that is
  some drawn lane's `merged_into`, only when that lane was already emitted in
  full and the destination lane has a head (R2-A2). Otherwise it emits
  `commit` and marks the graph incomplete. A second lane merged at the same
  commit is never drawn and marks it incomplete.
- New, not in the plan: siblings hanging from one commit are reordered so a
  lane merged into a sibling (or into a lane below that sibling) is emitted
  first (`emit_merged_lanes_first`). Without it, creation order emitted the
  destination lane's merge commit before the merged lane had commits, which
  showed up in the height-cap test. The shape is reachable once Phase 3 sets a
  child's fork against its recorded parent's tip and that fork lies on the
  default lane.
- `tags()` returns `(tags, unplaced)`. A ref, an in-view label-only line, or a
  PR with a known tip whose commit is not drawn sets `unplaced`. Tags of lanes
  the base-view height cap left out are not counted (that note already accounts
  for them), and a PR for a branch the graph does not know has no commit to
  account for.
- `trim_one_commit` pins merge destinations (and no longer pins the root
  entry that only unconnected lanes hang from). `fit_lanes` keeps a lane's
  ancestors by fork commit, merge destination, and parent name
  (`lane_ancestors`).
- `with_incomplete_history()`, `GitGraphPlan::incomplete`, and
  `pub const INCOMPLETE_HISTORY_NOTE = "Some history is not shown"`. The
  notice is a dim `Prose` line after "N more worktrees not shown"
  (`with_notes`, which replaced `with_hidden_lanes_note`).
- Module docs updated in the same change: they now describe merges, the
  no-substitution rule, and the notice, and they no longer name the
  `2026-09-24-ux-improvements` fix.

Existing tests whose expectations changed on purpose (named per plan):

| Test | Change | Why |
|---|---|---|
| `a_branch_already_in_the_default_branch_is_a_tag` | line gains `with_tip` | R7: the fork no longer labels |
| `the_base_view_gives_every_branch_with_commits_a_lane` | `merged` gains `with_tip(2222222)`, fork moved to `1111111` | R7 |
| `a_fork_point_outside_the_drawn_commits_hangs_from_the_lane_start` → `…_is_drawn_unconnected` | asserts unconnected text, no parent, `incomplete` | R8 (was a substitute attachment) |

New tests (`git_graph/tests.rs`, L1, `--features image`):

| Behavior | Test |
|---|---|
| Merge into default (exact text, repaired parents, lane, not incomplete) | `a_lane_merged_into_the_default_lane_is_drawn_merging_at_its_commit` |
| Merge into the recorded parent's lane | `a_lane_merged_into_its_recorded_parent_merges_on_the_parent_lane` |
| `fix/sniff` shape (fork from parent, both merges into default, `+96`) | `a_nested_lane_merged_into_the_default_lane_forks_from_its_parent` |
| Merge commit that is also a fork point | `a_merge_commit_can_also_be_a_fork_point` |
| Merge destination tagged `origin/main` | covered by the default and nested tests (`tag: "origin/main"` on the merge line, placed by geometry in Wave 3) |
| Sibling order puts the merged lane first | `a_lane_merged_into_a_sibling_is_emitted_before_it` |
| Undrawn destination / destination before the lane / at the root start / two lanes at one commit → no merge, `incomplete` | `a_merge_whose_destination_is_not_drawn_…`, `a_merge_that_would_precede_its_lane_…`, `a_merge_at_the_start_of_the_default_lane_…`, `only_one_lane_merges_at_a_commit` |
| Fork omitted from the parent lane: no attachment at the parent start or default start, `incomplete` | `a_fork_commit_missing_from_the_parent_lane_is_not_replaced_by_its_start` |
| Fork omitted from the default lane | `a_fork_point_outside_the_drawn_commits_is_drawn_unconnected` |
| Unknown fork (`None`) | `an_unknown_fork_is_drawn_unconnected_and_reported` |
| Tagged commit omitted → `incomplete` (control: complete) | `a_tag_on_an_undrawn_commit_is_reported` |
| Label-only line at `tip_sha` even with its fork drawn; no tip / undrawn tip → not labeled, `incomplete` | `a_label_only_line_sits_at_its_tip_never_its_fork` |
| PR for an unknown branch is not reported | `a_pull_request_for_a_branch_the_graph_does_not_know_is_not_reported` |
| Caller-reported gap | `the_caller_can_report_incomplete_history` |
| Trimming never elides a merge destination | `trimming_never_elides_a_merge_destination` |
| Base-view cap keeps a merged lane's destination lane | `the_height_cap_keeps_a_merged_lanes_destination_lane` |
| Hidden lanes' tags are not double-reported | `tags_of_lanes_the_height_cap_leaves_out_are_not_reported_twice` |
| Notice renders in a `Terminal` after the hidden-lanes note | `the_incomplete_history_notice_follows_the_lane_note` |

### Deliberate early touch of `worktree-cli`

R7 removes the fork fallback that was the *only* way `wt list` labeled a
merged branch (its lines had no tip). Leaving `worktree-cli` untouched would
have made every merged worktree branch vanish between Phase 2 and Phase 3.
So each `GraphLine` built in `commands/git_graph.rs` (focused current and
parent lines, base-view lines, the diverged `origin/<default>` line) now also
calls `.with_tip(<its tip>)`. Nothing else in gathering changed. Pinned by
`commands::git_graph::tests::the_base_view_gives_every_worktree_branch_a_line`
(`tip_sha` of three lines, and the merged branch's tag in the emitted text).

### Wave 3 — real-backend proof

`git_graph::tests::measured_plans_place_merges_and_tags_without_overlap` plans
four fixtures with the real measurement (`MermaidTheme::Default`) at 120×40 and
56×60 (fallback 8×16 cells) and checks the layout that renders: zero tag
overlaps, every expected tag on its display ID, merge second parent exact
(first parent exact unless trimmed into a `+N` square), not incomplete, and
`columns > viewport` only when planning for one column gives the same text
(fully trimmed). It is named without `real_`, which is a tier marker.

| Fixture | Viewport | Columns | Rows | Trimmed | Commit step | Natural width |
|---|---|---|---|---|---|---|
| observation 1 | 120×40 | 100 | 13 | 0 | 79.0 | 616 |
| observation 1 | 56×60 | 100 | 13 | 2 | 79.0 | 616 |
| observation 2 | 120×40 | 117 | 18 | 0 | 79.0 | 724 |
| observation 2 | 56×60 | 117 | 18 | 1 | 79.0 | 724 |
| long labels (R12) | 120×40 | 291 | 13 | 1 | 246.5 | 1,907 |
| long labels (R12) | 56×60 | 291 | 13 | 1 | 246.5 | 1,907 |
| compressed (fork 5,000 / merge 3,000 back) | 120×40 | 116 | 14 | 1 | 79.0 | 716 |
| compressed | 56×60 | 116 | 14 | 1 | 79.0 | 716 |

(Default theme's default step is 34.) Finding for Phase 4: with R3's global
step, trimming a lone commit into `+1` no longer narrows the image, because the
tag-driven step, not commit labels, sets the width. Trimming then stops with
the graph still wider than the viewport, and the image shrinks (the existing,
accepted behavior). The long-label pair shrinks about 2.4× at 120 columns and
about 5.2× at 56. Phase 4's 56-column evidence should expect
`trimmed_commits > 0` with an unchanged width.

### Consumer regressions and gates

- `just test` in `biscuit-visualized`: 98 passed.
- `just test` in `biscuit-terminal` (lib `--features image`, `l1` including
  `mermaid_parity.rs`, and the `bt` CLI): 3,344 passed, 55 skipped. No snapshot
  changed; no `.snap.new` files.
- `just test` in `darkmatter`: 8,498 passed, 12 skipped. No snapshot changed.
- `just test` in `worktree`: 712 passed, 29 skipped (tier-filtered `perf_` and
  L2), including the added tip assertions.
- `just lint` in `biscuit-visualized`, `biscuit-terminal`, `worktree`: pass.
  `biscuit-terminal`'s lint recipe builds without `image`, which is the feature
  that compiles `git_graph`, so the changed code was also checked with
  `cargo clippy -p biscuit-terminal --features image --all-targets -- -D warnings`
  and `cargo clippy -p biscuit-visualized --all-features --all-targets -- -D warnings`:
  clean.
- Cross-OS: not run in this phase. The changes are pure computation over
  strings and floats (no paths, processes, or filesystem). Phase 4 runs the
  three packages' L1 on Linux and Windows.

### Checkpoint 2 — rendered PNGs

`spikes/phase2-renders/run.sh <out>` (standalone crate, never in the
workspace) plans observation 1, observation 2 (with the real short SHAs), and
the R12 pair through `GitGraph::plan` and renders each plan with
`MermaidDiagram::render` (PNG, scale 2, default theme). Output:
`spikes/phase2-renders-output/{observation-1,observation-2,long-labels}-{120x40,56x60}.{png,mmd}`.
Every plan reported zero overlaps and exact merge parents
(`08cf96a ← [2222222, 2a7298a]`; `08cf96a ← [1314bd6, 2a7298a]` and
`b47a046 ← [08cf96a, b3b17f3]`). Inspected:

- `observation-1-120x40.png`: `fix/wt-ux` has its own lane from `1111111`,
  and the merge edge enters `main` at `08cf96a` (drawn as a merge node, tagged
  `origin/main`). `main` sits on `3333333`. No overlap.
- `observation-1-56x60.png`: the same with `2222222` and `7ba6ea8` trimmed into
  `+1` squares. The merge edge is unchanged.
- `observation-2-120x40.png`: `fix/wt-ux` merges at `08cf96a` (tag `main`), and
  `fix/sniff` forks at `2a7298a` through `+95` and merges at `b47a046` (tag
  `origin/main`). No side commits are on the default lane.
- `long-labels-120x40.png`: all four labels are separate and on their commits;
  `origin/main` and `main` sit one commit apart. Wide image, small scale, as
  predicted by S2.

### Requirement-to-test mapping (Phase 2)

Covered by the three tables above. Skipped or pre-existing failures: none. Not
done here by plan: `docs/` pages (Phase 5). `biscuit-terminal/docs/components/git_graph.md`
still documents merged branches as tags and "no merge statements"; that drift
is Phase 5's to fix, in the same pass that writes the new contract.

## Phase 3

Git discovery in `worktree-cli`, 2026-09-27. Branch selection is unchanged
(`recorded_parent`, the base view's `branches`/`drawn`, and the focused view's
one-level parent rule are the same code).

### What changed

- **`worktree` lib (`lib/src/git.rs`)**: new `git_command_allow_no_match`
  beside `git_command` (both now share `git_command_status`). Exit 1 is
  `Ok(None)`, any other failure `Err`, so `merge-base --is-ancestor`'s "no" and
  `merge-base`'s "no common ancestor" are told apart from errors **by exit code**
  (R9-A1), which `git_command`'s `Err(stderr)` could not do. Every call is still
  counted by the git-call recorder. Test:
  `git::tests::allow_no_match_separates_a_no_from_a_failure_by_exit_code`.
- **New `cli/src/commands/git_graph/topology.rs`**: `GatherGap`, `History`
  (`read` = one `rev-parse --is-shallow-repository`; a failed read is a gap and
  is treated as shallow), `is_ancestor`, `merge_base`, `classify` (R4 + R4-A1
  exactly: `chain` ∩ `descendants`, leading run), and `first_parent_entries`
  (R5 + R6). Every git line is parsed strictly; an unparseable line is a gap.
- **`cli/src/commands/git_graph.rs`**: `focused_view` and `base_view` now build
  a list of `Selected` branches with the unchanged selection code and hand it to
  one `assemble`:
  1. stage 1, every selected branch classified in parallel (`place`);
  2. stage 2a, every branch lane and a diverged `origin/<default>` line built in
     parallel with the anchors routed to them;
  3. stage 2b, the default lane, with its own anchors plus every anchor no
     branch lane placed.
  `GraphFacts` gains `incomplete`, and `to_git_graph` calls
  `with_incomplete_history()`. PR filtering is unchanged. The verbose half is
  unchanged (`commit_details_since` keeps all-parents semantics) and still
  shares the focused view's one `merge-base`.

### How each class is drawn

| Class (`Integration`) | Lane stop (`--not`) | Fork | Merge | Notice |
|---|---|---|---|---|
| `Unmerged` | default tips + parent tip | `merge-base(parent tip, else default lane tip, T)` (reuses verbose's answer when there is no parent) | none | no |
| `MergedDirectly { C }` | `C^1` (+ parent tip when merged elsewhere) | `merge-base(parent tip if merged elsewhere, else C^1, T)` | `merged_into(C)` | no |
| `NoSeparateHistory` | — (label only, `with_tip(T)`) | none | none | no |
| `IntegratedOtherwise` | as merged | as merged | none | **yes** |
| gap (`Err`) | as unmerged | `merge-base` if it answers, else none (unconnected) | none | **yes** |

A shallow "no" (is-ancestor exit 1, merge-base exit 1, an empty chain) is a gap
(R9-A1), and so is a `+N` square in a shallow repository (a lower bound).

### Anchors (R6) and their routing

- Each placement asks for its fork on its fork lane (the parent's, else the lane
  it was measured against), its merge commit on the lane it merged into, and
  a label's tip on the lane containing it. The default lane also anchors the
  local default tip and `origin/<default>` (unless diverged).
- `first_parent_entries` places an anchor already in the shown window for free.
  Otherwise its position is `rev-list --first-parent --count A..tip` (one per
  anchor, on scoped threads), and every candidate position is verified in **one**
  `log --no-walk=unsorted --ignore-missing --format='%H %P' tip~d…`. R6 named a
  batched `rev-parse`; that fails the whole batch with exit 128 when one
  `tip~d` passes the root (an off-chain anchor's count is the whole chain), so
  `--ignore-missing` replaced it (checked on git 2.55: invalid `tip~d` dropped,
  exit 0). An anchor among the answers is at its own distance, since one chain
  position names one commit. A branch lane shorter than its window is fully
  shown, so it asks no anchor distances at all.
- Anchors a branch lane did not place fall back to the default lane (built
  last). A branch lane whose entries came out empty also sends its tip there,
  so it is labeled where it is.

### Deviations from the plan (recorded; the spec is untouched)

- **R4-A2 (amendment).** R4 says a merged branch's fork is
  `merge-base(<recorded parent tip, else C^1>, T)`. When the branch was merged
  **into** that parent, the parent's tip contains `T`, and `merge-base` returns
  `T` itself. The fork is therefore measured against `C^1` whenever the merge's
  lane is the parent's, and against the parent's tip only when the merge went
  elsewhere (the `fix/sniff` shape). The parent's tip is likewise added to the
  lane stop only in that case. Proven by
  `a_child_merged_into_its_parent_merges_on_the_parent_lane` (fork `p1`, not
  `c2`).
- **Stage 2 is 2a (branch lanes, parallel) then 2b (default lane).** The plan
  said "build every lane's entries with its anchors in parallel". A child's
  fork can lie on its parent's lane or below it on the default lane, and only
  the parent lane's build knows which. Building the default lane last with the
  leftovers costs one lane's latency and saves one `rev-list --count` per
  misrouted anchor (each 60–70 ms on Windows, S3).
- **`CONTEXT_COMMITS` is gone as a constant.** The focused view's two context
  commits are now "the oldest anchor and its first parent" (`Extent::Open {
  cap_window: true }`), taken from `%P`, with no extra git call. The base view
  keeps its 10-commit window (`cap_window: false`) and also extends one commit
  below an anchor beyond the window, so a merge destination is never the default
  lane's first statement (Phase 2's R2-A2 constraint). For every pre-existing
  fixture the drawn default lane is identical to before.
- **`DefaultTips::diverged_line` no longer exists.** The diverged
  `origin/<default>` line is one of stage 2a's jobs (first-parent, window
  `LINE_WINDOW`, stop at the local tip) and receives the anchors of branches
  merged into it.
- **The no-network test cannot assert zero `upload-pack` runs.** A graph-free
  listing of the same repository already runs exactly one (the listing's
  live-head worker checks `origin` whenever there is one, with fresh PR and
  live-head stores seeded). The test asserts the graph-drawing listing makes
  exactly as many runs as the graph-free one (`without_graph <= 1`, and the
  difference equals it), which is the claim R9 makes: no *new* request.

### Consequence worth knowing (follows R4 as written)

A branch that **continued after its merge** is `Unmerged` with its fork at the
old merged tip. That commit is the merge's second parent, on no first-parent
lane, so `GitGraph` draws the lane unconnected and shows "Some history is not
shown". This is truthful (the connection through the old merge is not drawn)
and matches the plan's expectation for the fixture, but it is a visible notice
for a common workflow. Phase 5's docs should state it.

### Tests (all L1; `cli/src/commands/git_graph/tests.rs` unless noted)

| Requirement (plan Wave 6 / spec acceptance row) | Test |
|---|---|
| Observation 1 from the merged branch: own lane, `merged_into == merge`, first-parent default lane, local `main` labeled | `a_merged_current_branch_keeps_its_lane_and_merges_at_its_merge_commit` |
| Observation 1 from `main` (base view, tall viewport keeps the lane) | `a_merged_branch_keeps_its_lane_in_the_base_view` |
| Observation 2: `fix/sniff` forks at `fix/wt-ux`'s tip, merged at `m104`, `+7` long side; no side commit on the default lane; `fix/wt-ux` merged at `m103`; local `main` tagged | `a_child_merged_into_the_default_branch_forks_from_its_parent_in_the_base_view` |
| Merged into the parent: `merged_into` on the parent lane, which anchors it | `a_child_merged_into_its_parent_merges_on_the_parent_lane` |
| Recorded parent with and without its own worktree selected; child keeps `with_parent`; base view nests only under a drawn parent | `a_recorded_parent_is_selected_with_or_without_its_own_worktree` |
| Fast-forward: label at the actual commit, no entries, no merge | `a_fast_forwarded_branch_is_a_label_at_its_commit` |
| Equal tips: `c` keeps its lane and merge, `d` is a label at `c`'s tip | `equal_tips_keep_the_merged_lane_and_label_the_new_branch` |
| Continued after merge: unmerged lane at the old merged tip, no merge edge (plan shows `incomplete`) | `a_branch_continued_after_its_merge_is_an_unmerged_lane` |
| Indirect integration: lane without merge, `incomplete` | `an_indirectly_integrated_branch_gets_a_lane_without_a_merge_and_the_notice` |
| Old connections: fork and merge are `Commit` entries, `Elided` counts equal real first-parent distances (focused and base) | `old_forks_and_merges_stay_drawn_with_exact_elided_runs` |
| Shallow clone: `incomplete`, verified lanes and labels remain, graph not `None` | `a_shallow_clone_draws_what_it_can_verify_and_reports_the_rest` |
| Deleted parent: existing fallback unchanged | `a_deleted_recorded_parent_falls_back_to_the_default_branch` |
| A merged lane competes under the base-view cap by the unchanged activity rule | `a_merged_lane_competes_under_the_height_cap_by_activity` |
| Every classification outcome; first candidate wins; a git failure is a gap, never `Unmerged` | `classify_names_every_integration_and_tries_candidates_in_order` |
| Anchors: off-chain not placed; below a branch lane's stop not placed; exact `+N` runs; unreadable tip is the only `Err` | `first_parent_entries_place_only_anchors_on_the_lane` |
| No new network request; table and notice present (binary, pty, Unix) | `cli/tests/list_remote_head.rs::a_shallow_clone_lists_with_the_incomplete_history_notice_and_asks_origin_nothing` |

Existing tests whose expectations changed on purpose:

| Test | Change | Why |
|---|---|---|
| `the_base_view_gives_every_worktree_branch_a_line` | `chore/merged` (fast-forwarded) now has `fork_sha == None` and `merged_into == None`; call counts are split into `--is-ancestor` checks (5), merge bases (3), `log` (4, the label-only branch has no lane), `rev-list` (1, its empty chain); `incomplete == false` | R4/R7: a label-only branch has no fork; classification adds the yes/no checks |
| `graph_and_verbose_share_one_merge_base` | counts merge bases excluding `--is-ancestor` (still 1) and all `merge-base` calls (2) | classification's one yes/no check is not a merge base; the fork reuses verbose's answer |

`criss_cross_lines_and_verbose_details_hold_tip_unique_commits` and
`origin_ahead_extends_the_default_lane_and_diverged_origin_gets_a_line` pass
unchanged: in both fixtures the first-parent lane holds exactly the commits the
all-parents walk did.

Tier and placement: every new unit test is in `git_graph/tests.rs` (compiled by
the lib and bin targets, no tier marker in any path segment). The binary test
is in `list_remote_head.rs` (an auto-discovered test target, no marker) and is
`#[cfg(unix)]`, since `script` is Unix-only, like `perf_graph_stages.rs`.
`just check-tier-coverage worktree`: 0 stranded.

### Gates

- `just test` in `worktree`: 744 passed, 29 skipped (tier-filtered `perf_` and
  L2), exit 0.
- `just lint` in `worktree`: pass. `cargo clippy -p worktree-cli -p worktree
  --all-targets -- -D warnings`: clean.
- `biscuit-terminal` sizing regressions, unchanged
  (`cargo nextest run -p biscuit-terminal --features image --lib`, filtered):
  `the_base_view_height_cap_keeps_the_most_recently_active_lanes`,
  `the_height_cap_adds_lanes_in_activity_order_until_one_does_not_fit`,
  `a_focused_view_is_never_cut_by_the_height_cap`,
  `the_height_cap_keeps_a_merged_lanes_destination_lane`,
  `tags_of_lanes_the_height_cap_leaves_out_are_not_reported_twice`,
  `without_image_support_the_terminal_gets_the_code_block_and_the_lane_note`,
  `the_incomplete_history_notice_follows_the_lane_note`,
  `the_caller_can_report_incomplete_history`: 8 passed.
- **Not verified here: the Kitty L2** (`level2_graph_in_kitty`, run with
  `BISCUIT_TEST_REQUIRED_BACKENDS=kitty`). Both tests reached the screenshot
  step and failed there with a capture that has window chrome but no contents
  at all (not even the table text), which the test's own message attributes to
  the calling terminal lacking macOS Screen Recording permission. This
  non-interactive session cannot grant it. Phase 4 re-runs this test for its
  screenshot evidence and needs a session with that permission.

### A stale `perf_` expectation found on Linux (fixed)

`commands::list::tests::perf_subprocess_counts_meet_sla` (lib and bin targets)
counted `merge-base` calls in the base-view gather as one per line. It is a
`perf_` test, so neither `just test` nor CI's L1 runs it; build-linux's native
path runs everything and failed on it (`left: 2, right: 1`). The count now
splits one shallow check, one `--is-ancestor` per line (each branch in that
fixture has only the default lane as a candidate), and one merge base per line.
It fails the same way on macOS before the fix, so it was never Linux-specific.

### Cross-OS (Checkpoint 3)

| OS | How | Result |
|---|---|---|
| macOS (this host, git 2.55.0) | `just test` in `worktree` | 744 passed, 29 skipped |
| native Windows (`build-win-native`, git 2.55.0.windows.3) | `./scripts/cross-check.sh --os windows worktree-cli` (archive from `windows-latest`) | 420 passed, 46 skipped; includes every new graph test, the `file:///C:/…` shallow clone, and `allow_no_match`'s exit codes |
| Linux (`build-linux`, git 2.47.3) | `./scripts/cross-check.sh --os linux worktree-cli --features terminal-tests` | 493 passed (after the `perf_` fix above), including the `script -qec` pty test |
| Linux (local Docker `rust:1`, arm64, git 2.47.3) | `cargo test` of `worktree` lib `allow_no_match`, `worktree-cli --lib git_graph`, `--test list_remote_head shallow` | 1 + 24 + 1 passed |

- The plain linux leg (archive mode) failed twice before compiling any
  repository code: `target/release/deps/*.rmeta is not writeable` in the
  `fix-wt-ux` standing clone, the stale read-only kache links the `os` skill
  already records for that clone. A build flag takes the native path, which
  ran green; the skill now records the 2026-09-27 recurrence and that the
  native path runs `perf_` tests.
- OS neutrality: every assertion compares full SHAs or entry lists, never paths
  or git's stderr. Git errors are classified only by exit code
  (`git_command_allow_no_match`), never by (possibly localized) stderr. The one
  path-shaped value, the shallow fixture's `file://` URL, is built as
  `file:///C:/…` on Windows. The `os` skill's git notes (CI's depth-1 checkouts,
  Windows verbatim paths) do not apply: every fixture builds its own temporary
  repository. WSL2 is left to the nightly schedule, as the plan says.

### Perf gates (not Phase 4's recorded comparison)

`just test-perf perf_graph` (dev profile, one sample; Phase 4 records the
release medians against the Phase 1 baseline): pass, both stages reported on
every fixture. `just test-perf meets_sla`: all six pass
(`perf_full_command_non_image_meets_sla`, both cache paths, the three
`perf_pr_request` gates).

### Checkpoint 3

- All gathering fixtures pass on macOS, Windows, and Linux.
- Git usage is OS-neutral (above); the `os` skill was consulted and updated.
- Not verified: the Kitty L2 screenshot step (Screen Recording permission, see
  Gates). Phase 4 owns that evidence.

### Requirement-to-test mapping, targeted tests, and skips (Phase 3 summary)

- Mapping: the "Tests" table above; every Wave 6 bullet has a named test.
- Targeted tests added: 15 in `git_graph/tests.rs`, 1 in `lib/src/git.rs`, 1 in
  `cli/tests/list_remote_head.rs`.
- Broader gates: `just test`/`just lint` in `worktree`, clippy `--all-targets`,
  the `biscuit-terminal` sizing tests, the `perf_` SLA and graph gates, and the
  three cross-OS runs above.
- Skipped or failing and not caused by this phase: `level2_graph_in_kitty`
  (environment: no Screen Recording permission); build-linux archive mode
  (environment: stale kache links). No pre-existing test failures otherwise.

## Phase 4

Rendered evidence, cross-OS, and performance, 2026-09-27. No production
source changed in this phase; everything below is tests, a dev-dependency,
evidence files, and docs.

### End-to-end layout evidence (Wave 7, portable L1)

`commands::git_graph::tests::gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`
(`worktree/cli/src/commands/git_graph/tests.rs`). For each real-Git fixture it
runs `gather`, `to_git_graph`, then `plan()` with the real measurement
(`MermaidTheme::Default`) at 120×40 and 56×60 (`CellSize::FALLBACK`), and reads
the layout the image is drawn from through
`biscuit_visualized::mermaid::MermaidDiagram::gitgraph_geometry()`. Per
viewport it asserts: not incomplete, no hidden lane, `tag_overlaps()` empty,
every expected tag on the emitted ID of its exact SHA, both merge parents
exact (the first may be a `+N` square once trimmed), the sorted tag texts
identical at both viewports (no label shortened or dropped), a graph wider than
56 columns at 120 is trimmed or shrunk at 56, and wider-than-viewport only once
fully trimmed. With `ImageWidth::Characters(40)` it asserts `columns == 40`,
`trimmed_commits == 0`, and the planned text equals the untrimmed text.

| Fixture (view) | Viewport | Columns | Rows | Trimmed | Commit step | Natural width |
|---|---|---|---|---|---|---|
| observation 1 (focused, from `fix/wt-ux`) | 120×40 | 103 | 13 | 0 | 81.1 | 657 |
| observation 1 (focused) | 56×60 | 103 | 13 | 2 | 81.1 | 657 |
| observation 1 (base, from `main`) | 120×40 | 103 | 13 | 0 | 81.1 | 657 |
| observation 1 (base) | 56×60 | 103 | 13 | 2 | 81.1 | 657 |
| observation 2 (base) | 120×40 | 131 | 19 | 6 | 81.1 | 835 |
| observation 2 (base) | 56×60 | 131 | 19 | 6 | 81.1 | 835 |
| long labels (base) | 120×40 | 350 | 13 | 3 | 220.8 | 2,234 |
| long labels (base) | 56×60 | 350 | 13 | 3 | 220.8 | 2,234 |

The expected tags and merges, by full SHA:

- observation 1: `main` on `d2`, `origin/main` on the merge; merge parents
  `[d2, w2]`;
- observation 2: `main` on `m103`, `origin/main` on `m104`; merges
  `m103 ← [d1, w2]` and `m104 ← [m103, s12]`;
- long labels (new fixture `long_labels`): `fix/very-long-exact-branch-reference-beta`
  on `d3`, `main` on `d4`, `origin/main` on `d5` (three neighboring
  default-lane commits), and `PR #104 → main` on the tip of
  `feature/very-long-exact-branch-reference-alpha`'s lane.

As Phase 2 predicted, at 56 columns the graph is trimmed and then shrunk
(`columns` > 56 with every trimmable commit folded), and the tag-driven step
keeps the width unchanged by trimming.

Deviations (recorded; the spec is untouched):

- **Placed beside the fixtures, not in `cli/tests/`.** The plan says
  "`worktree-cli` integration test". The real-Git fixtures and `gather` are
  private to `commands::git_graph` and its test module, so the test sits in
  `git_graph/tests.rs` (compiled by the lib and bin targets, so it runs twice;
  no `insta`). It still crosses every boundary the plan named: real Git,
  gathering, `GitGraph::plan` with the real measurement, and the
  `biscuit-visualized` layout.
- **R12's second label is a local branch.** `wt list` draws no remote-tracking
  ref other than `origin/<default>`, so `origin/very-long-exact-branch-reference-beta`
  could never reach the graph. The fixture uses the same-length local branch
  `fix/very-long-exact-branch-reference-beta` as a label on the default lane.
- **New dev-dependency:** `biscuit-visualized` (workspace path, `image`
  feature) in `worktree/cli/Cargo.toml`, for `gitgraph_geometry`. It was
  already built through `biscuit-terminal`; `docs/dependencies.md` records it.

Load-bearing check: before removing a temporary debug print, the emitted
Mermaid for the long-label fixture showed the three long/ref labels on three
adjacent default-lane commits and the PR tag on the lane tip, which are the
cases the overlap assertion is meant to cover.

### Kitty L2 extension (macOS)

`worktree/cli/tests/level2_graph_in_kitty.rs`:

- `Fixture` is now built by `init()` plus `new()` (the unchanged five-lane
  fixture) or `merged()` (a `fix/wt-ux` worktree merged into `main` with a
  merge commit). The table check reads the fixture's worktree names (sorted,
  as `wt list` lists them) instead of the five constant branches.
- New `level2_graph_draws_a_merged_branch_in_kitty` runs at 100×32 and 56×60.
  Both windows' text and APC checks (table intact, rows ≤ half the window, no
  "Some history is not shown") run and both images are kept before either
  screenshot, then each window's image placement is checked. `GraphRun::keep_evidence`
  keeps `wt-graph-merged-<cols>x<rows>-screenshot.png` and writes
  `…-transmitted.png` in the temp directory.
- The existing APC and reservation assertions are unchanged, and so are the
  two existing tests.

Two fixture changes made while bringing the test up, neither a graph defect:

- A second, unmerged worktree was dropped from `merged()`. At 100×32 the base
  view's half-height cap left it out ("1 more worktree not shown") under the
  unchanged activity rule.
- The merged branch was first `feature/very-long-exact-branch-reference-alpha`.
  At 56 columns the **table** (not the graph) then wraps its header cell
  (`-> paren` / `t`), which broke the test's one-row header parsing. The table
  is outside this fix (no phase touched it), so the branch is now `fix/wt-ux`,
  observation 1's name. Long neighboring labels are proven at L1. **Finding
  outside this fix:** at 56 columns with a 46-character branch name, the
  `wt list` table's header wraps inside the `-> parent` cell.

Run: `BISCUIT_TEST_REQUIRED_BACKENDS=kitty cargo nextest run -p worktree-cli
--features terminal-tests -E 'binary(level2_graph_in_kitty)'`.

**Not verified: the screenshot step.** Every check before it passes for both
sizes of the new test (the table, the `c=` columns ≤ window, the PNG's width =
columns × cell width, reserved rows = Kitty's rows, the text band). The window
screenshot again has window chrome and no contents, the calling terminal's
missing macOS Screen Recording permission that Phase 3 recorded, and the two
existing tests fail at the same step. This non-interactive session cannot grant
it. The image placement on screen therefore still needs one run from a
terminal with that permission.

Inspected instead, the PNGs `wt` transmitted to Kitty (the image Kitty is asked
to draw), copied to
`spikes/phase4-kitty-output/wt-graph-merged-{100x32,56x60}-transmitted.png`:

- 100×32: `fix/wt-ux` has its own lane, forks from `main` at `ba05020`
  (`main 2`), holds `e20cd78` and `508303f`, and its merge edge enters `main`
  at `a19e7b6`, the commit labeled `main`. The default lane is
  `851fe51 → ba05020 → bf985a8 → a19e7b6` with no branch commit on it. One
  label; nothing overlaps.
- 56×60: the same, with `851fe51`, `bf985a8`, and `e20cd78` folded into `+1`
  squares. The merge edge and the `main` label are unchanged.

(`main 1`/`main 2` short SHAs differ per run; the fixture has fixed topology
but not fixed timestamps.)

### Cross-OS L1

`./scripts/cross-check.sh` from this worktree (throwaway commit over
`origin/fix/wt-ux`; the banner reads `+ 1238 changed file(s)` because the
remote branch is behind the local one).

| OS | Package | How | Result |
|---|---|---|---|
| Linux (`build-linux`) | biscuit-visualized | `--features image` (native path) | 98 passed |
| Linux | biscuit-terminal | `--features image` | 3,073 passed, 2 skipped |
| Linux | worktree | `--features count-git` | 307 passed |
| Linux | worktree-cli | `--features terminal-tests` | 496 passed (both targets of the new layout test pass; Kitty L2 skips) |
| native Windows (`build-win-native`) | biscuit-visualized | archive mode refused (`the plan resolves no single windows-latest build`: the package is CI-excluded, `gates = false`); rerun with `--features image` | 98 passed |
| native Windows | biscuit-terminal | archive (CI features) | 3,003 passed, 58 skipped |
| native Windows | worktree | archive | 288 passed |
| native Windows | worktree-cli | archive | 422 passed, 47 skipped (new layout test PASS in lib and bin, about 12 s each: Windows process spawns) |
| macOS (this host) | worktree | `just test` | 746 passed, 29 skipped |

- Linux used a build flag on every package to take the native path, because
  archive mode on this clone hits the stale kache links the `os` skill records.
- No Windows `git` path spelling or `LC_ALL` difference surfaced: the new test
  compares SHAs, display-ID prefixes, and tag texts only.
- WSL2 is left to the nightly schedule, as planned.

### Performance (after)

Same command as Phase 1, same fixtures, unchanged:
`WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release`
(release, 10 samples, one warm-up, medians). Run twice: once while the
cross-checks were running remotely, once on its own. The second run is
recorded; the first agreed within 5%.

| Fixture | `graph gather` before | after | Δ | `graph image render` before | after | Δ |
|---|---:|---:|---:|---:|---:|---:|
| floor | 5.9 ms | 10.5 ms | +4.6 ms | 352.4 ms | 347.0 ms | −1.5% |
| ordinary | 20.2 ms | 39.1 ms | +18.9 ms | 351.2 ms | 356.1 ms | +1.4% |
| older essential connections | 69.5 ms | 126.3 ms | +56.8 ms | 353.3 ms | 351.8 ms | −0.4% |
| multiple selected branches | 32.4 ms | 65.3 ms | +32.9 ms | 351.8 ms | 355.2 ms | +1.0% |

(First run: 10.8 / 39.6 / 130.8 / 63.6 ms gather; 348.0 / 353.7 / 350.5 /
356.3 ms render.)

**Explanation of the gather regression**, from a `GIT_TRACE` of one sample per
fixture: each git spawn costs about 5 ms on this host, and gathering is now a
chain of dependent stages rather than one concurrent round.

- floor: `rev-parse --is-shallow-repository` + the default lane's `log` (was
  the `log` alone). The +4.6 ms is that one spawn.
- ordinary (two branches): shallow check → per branch `merge-base
  --is-ancestor` → fork `merge-base` → branch `log` → default `log` → one
  anchor `rev-list --first-parent --count` → one `log --no-walk`. About 10
  calls on a chain of about 7, against about 5 calls on a chain of 2 before.
- older essential connections: as ordinary, plus `rev-list --first-parent
  --parents` and `rev-list --ancestry-path` over the 3,000-commit span of the
  merged branch, and two anchor counts over 5,000 first-parent commits.
- multiple selected branches: about 30 calls (7 `--is-ancestor`, 7 fork
  `merge-base`s, 2 classifications with both `rev-list` walks, 7 branch `log`s,
  counts, and the verifying `log`) on the same chain length.

These are the costs Phase 3's message predicted (one `--is-ancestor` per
candidate per branch, the merged-branch walks, one `rev-list --count` per
out-of-window anchor, and one batched `log`). None of them are avoidable
without dropping a check that R4, R6, or R9 requires. The one latency that
could be cut is the shallow check, which runs before stage 1 but is only
consulted when an answer is "no"; running it concurrently with stage 1 would
save one spawn (about 5 ms). Not done: it is an optimization the plan does not
call for, and the graph has no numeric budget.

**Render:** unchanged within ±1.5%. It is dominated by the pseudo-terminal's
fixed ~350 ms (floor row), which hides the two-pass layout.

**Existing gates:** `just test-perf meets_sla`: 6 passed
(`perf_full_command_non_image_meets_sla` 1.68 s test time, both cache paths,
the three `perf_pr_request` gates). Full `just test-perf` (worktree-cli, serial):
29 passed, including `perf_subprocess_counts_meet_sla` (lib and bin), the
held-check and held-fetch waits, and the 60 s `--ff` deadline case.

`worktree/docs/performance-testing.md` gains a "Graph Stages" section with the
table and explanation. It also corrects two statements Phase 3 left stale: the
"Graph Data Collection" bullet (it described one `merge-base` and one `log` per
branch) and the descriptions of `perf_subprocess_counts_meet_sla` and
`graph_and_verbose_share_one_merge_base` (their counts changed in Phase 3).

### Checkpoint 4

- [x] Layout evidence passes on macOS, Linux, and native Windows (above).
- [ ] **Open:** the Kitty screenshot. The transmitted PNGs are inspected and
  kept (paths above); the on-screen capture needs a terminal with macOS
  Screen Recording permission. Run
  `BISCUIT_TEST_REQUIRED_BACKENDS=kitty cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_graph_in_kitty)'`
  from such a terminal and inspect
  `$TMPDIR/wt-graph-merged-{100x32,56x60}-screenshot.png`.
- [x] The before/after table is recorded (here and in
  `worktree/docs/performance-testing.md`), and the gather regression is
  explained.

### Requirement-to-test mapping, targeted tests, and skips (Phase 4 summary)

| Requirement (plan Wave 7) | Test / evidence |
|---|---|
| Zero tag overlaps on gathered graphs at 120×40 and 56×60 | `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags` (`tag_overlaps()` empty) |
| Every expected label on the emitted ID of its exact SHA | same test, `Evidence::tags` per fixture |
| Merge parents exact | same test, `Evidence::merges` (observation 1, both observation-2 merges) |
| 56 columns: trimmed or shrunk, no label shortened | same test (tag-text sets equal across viewports; trimmed/shrunk assertion) |
| `--width 40` never trimmed | same test (`columns == 40`, `trimmed_commits == 0`, untrimmed text) |
| Merged-branch Kitty run at 100×32 and 56×60, APC and reservation kept | `level2_graph_in_kitty::level2_graph_draws_a_merged_branch_in_kitty` (all pre-screenshot checks pass; screenshot step blocked) |
| Visual inspection | transmitted PNGs in `spikes/phase4-kitty-output/` (inspected above) |
| Cross-OS L1 | the cross-OS table above |
| Performance after, existing gates | the performance table above; `just test-perf` 29 passed |

- Targeted tests added: 1 L1 (runs in both `worktree-cli` targets), 1 L2
  (`level2_` marker; `worktree-cli`'s `test-l2` recipe is live;
  `just check-tier-coverage worktree`: 0 stranded).
- Broader gates: `just test` in `worktree` (746 passed, 29 skipped: the
  tier-filtered `perf_` and L2 tests), `just lint` in `worktree`, `cargo clippy
  -p worktree-cli -p worktree --all-targets --features
  worktree-cli/terminal-tests -- -D warnings` (clean; covers the L2 file),
  `just test-perf`, and the Linux and Windows runs above.
- Failing and not caused by this phase: the three `level2_graph_in_kitty`
  tests at the screenshot step (environment: no Screen Recording permission);
  Windows archive mode for the CI-excluded `biscuit-visualized` (rerun
  natively, green).

## Phase 5

Documentation, skills, and review readiness, 2026-09-27. No behavior changed in
this phase: the source edits are comment-only (see "Comment drift pass").

### Wave 8 — documentation and skills

| File | Change |
|---|---|
| `worktree/docs/git-graph.md` | Rewritten. Removed the collapse of merged branches into tags and the reference to the `2026-09-24-ux-improvements` fix. Now covers: selection (unchanged, both views, one-level parent, deleted-parent fallback), the first-parent default lane, the classification order and `C`, a Mermaid flowchart of the classification, a class table, one example per class (merged directly, the `fix/sniff` fork-from-parent shape and the merged-into-parent fork rule, no separate history, integrated otherwise, continued after a merge), anchors and `+N` compression, the two gathering stages, the default lane's "oldest anchor + first parent" extent, the no-substitution rule and notice, unavailable history (shallow rule, exit codes, no fetch), sizing with tag spacing, and the limits (no squash inference, no reconstruction of earlier merges, no indirect-integration edge, one lane merged per commit) |
| `biscuit-terminal/docs/components/git_graph.md` | Documents `with_tip`, `forked_at` unset = unknown, `merged_into`, `with_incomplete_history`, `GitGraphPlan::incomplete`, `INCOMPLETE_HISTORY_NOTE`, merge emission (with the exact text a component test pins), sibling reordering, the no-substitution table, merge-destination pinning in trimming, lane ancestors under the height cap, and the tag-spacing consequence for width. "Renderer workarounds" now says `merge` is emitted and its second parent is repaired in `biscuit-visualized`. Dropped the fix reference. The example's merged line is in the base view (a focused view would not give it a lane) |
| `biscuit-terminal/lib/src/components/git_graph.rs` module docs | Already current since Phase 2 (merges, no substitution, notice, no fix reference). No edit |
| `biscuit-visualized/docs/mermaid-gitgraph.md` (new) | The merge repair (rule, example, outcome table, the headless-lane parser drop, the pin test), tag spacing (formula, `TAG_GAP_EM` as policy, the vertical and rotated-tag limitation, the width cost), the cache-backend bump `0.2.x+bv1` → `0.3.x+bv2`, `gitgraph_geometry`, and tests. A Mermaid diagram shows the one layout path |
| `biscuit-visualized/README.md` | One paragraph linking the new page |
| `worktree/docs/cli/list.md` | "Git Graph": removed "a branch already in the default branch is a tag rather than a line" and the fixed "two shared commits"; added the first-parent rule, merged lanes, label-only branches, old connections, and the incomplete-history notice (including the continued-after-merge case) |
| `.claude/skills/biscuit-visualized/SKILL.md` | `mermaid-rs-renderer` v0.2 → 0.3.1 with the cache backend id; topic row and deep-dive pointer for the gitGraph corrections |
| `.claude/skills/biscuit-visualized/mermaid-rendering.md` | New "gitGraph Corrections" section and the `gitgraph.rs` source row |
| `.claude/skills/biscuit-terminal/components.md` | New "GitGraph" section (tip, merge, no substitution, pinning, geometry proof); overview row updated |
| `.claude/skills/worktree/SKILL.md` | The `commands/git_graph.rs` bullet was already updated in Phases 3–4 (first-parent lanes, R4 classes, anchors, `incomplete`, the continued-after-merge notice) and matches the code. No edit |
| `.claude/skills/os/macos.md` | Added the `CDPATH` trap that sent this phase's first gate run to the main checkout (see Final gates). No git trap to add: S3 found no platform-specific one (same classifications, exit codes, and shallow behavior on macOS, Linux, and Windows). The Windows per-process cost was recorded in Phase 3 |

A search of the area docs and skills for the old wording ("tag rather than",
"no merge statements", "already in the default branch/its parent") finds
nothing left.

### Comment drift pass

Reviewed `///`, `//!`, and inline comments on `GraphLine` (fields, `with_tip`,
`merged_into`, `tip`), `attach_point`, `tags`, `trim_one_commit`, `fit_lanes`,
`lane_ancestors`, `plan_with`, `focused_view`, `base_view`, `place`,
`assemble`, `topology::{Integration, Extent, first_parent_entries, locate}`,
`compute_layout`, `gitgraph::{repair_gitgraph_merges, layout}`, and
`MERMAID_BACKEND`. All describe current behavior. `line_entries` and
`log_commits` no longer exist (removed in Phase 3).

Drift found and fixed (comment-only): comments that cited planning rulings by
number, which a reader of the code cannot resolve.

| File | Before | After |
|---|---|---|
| `worktree/cli/src/commands/git_graph.rs` (`place`) | "Classifies one selected branch (R4) against" | "Classifies one selected branch against" |
| `biscuit-terminal/lib/src/components/git_graph/tests.rs` (`long_label_neighbors`) | "R12's long labels on neighboring commits" | "Long labels on neighboring commits" |
| same file, `measured_plans_place_merges_and_tags_without_overlap` | "Recorded in the implementation log (R3's wider step shows up here)." | "Per-viewport size, trimming, and the tag-driven commit step, for review." |
| `worktree/cli/src/commands/git_graph/tests.rs` (`BETA`, `long_labels`) | "R12's second long label is …" / "R12's long labels as `wt list` meets them" | "A label as long as `origin/very-long-exact-branch-reference-beta`; …" / "Long labels as `wt list` meets them" |

### Acceptance trace (Wave 9)

Every spec acceptance row, its proof, and its evidence. Test files:
`wt:` is `worktree/cli/src/commands/git_graph/tests.rs`, `bt:` is
`biscuit-terminal/lib/src/components/git_graph/tests.rs`, and `bv:` is
`biscuit-visualized/src/src/tests/gitgraph_tests.rs`.

| Spec case | Proof (tests) | Evidence files |
|---|---|---|
| Current branch merged, worktree retained | `wt: a_merged_current_branch_keeps_its_lane_and_merges_at_its_merge_commit`; `wt: gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags` (observation 1, focused); `bt: a_lane_merged_into_the_default_lane_is_drawn_merging_at_its_commit`; `level2_graph_in_kitty::level2_graph_draws_a_merged_branch_in_kitty` (all checks before the screenshot) | `spikes/phase2-renders-output/observation-1-{120x40,56x60}.png`; `spikes/phase4-kitty-output/wt-graph-merged-{100x32,56x60}-transmitted.png` |
| Same branch from default | `wt: a_merged_branch_keeps_its_lane_in_the_base_view`; layout test (observation 1, base) | as above |
| Recorded non-default parent ± worktree | `wt: a_recorded_parent_is_selected_with_or_without_its_own_worktree` | — |
| Merged into non-default parent | `wt: a_child_merged_into_its_parent_merges_on_the_parent_lane`; `bt: a_lane_merged_into_its_recorded_parent_merges_on_the_parent_lane`; `bv: a_merge_into_a_non_root_lane_is_repaired` | `spikes/render-topologies-output/s1-into-parent-lane-fixed.png` |
| Second observation (fork from parent inside its `+N`, merged into default) | `wt: a_child_merged_into_the_default_branch_forks_from_its_parent_in_the_base_view`; `bt: a_nested_lane_merged_into_the_default_lane_forks_from_its_parent`; `bt: measured_plans_place_merges_and_tags_without_overlap` (real backend); `bv: a_nested_lane_merges_into_the_root_lane`; layout test (observation 2) | `spikes/phase2-renders-output/observation-2-{120x40,56x60}.png` |
| Merged side longer than the base window | `wt: a_child_merged_into_the_default_branch_forks_from_its_parent_in_the_base_view` (default lane holds no side commits, true forks, local `main` tagged); layout test (`main` on `m103`, `origin/main` on `m104`) | as above |
| Fork/label commit undrawn | `bt: a_fork_commit_missing_from_the_parent_lane_is_not_replaced_by_its_start`, `a_fork_point_outside_the_drawn_commits_is_drawn_unconnected`, `an_unknown_fork_is_drawn_unconnected_and_reported`, `a_tag_on_an_undrawn_commit_is_reported`, `a_label_only_line_sits_at_its_tip_never_its_fork` | `spikes/render-topologies-output/s1-disconnected-fixed.png` |
| Continued after merge | `wt: a_branch_continued_after_its_merge_is_an_unmerged_lane` | — |
| Fast-forward / equal tips | `wt: a_fast_forwarded_branch_is_a_label_at_its_commit`, `equal_tips_keep_the_merged_lane_and_label_the_new_branch` | — |
| History unavailable | `wt: a_shallow_clone_draws_what_it_can_verify_and_reports_the_rest`; `cli/tests/list_remote_head.rs::a_shallow_clone_lists_with_the_incomplete_history_notice_and_asks_origin_nothing` (table present, notice shown, no added upload-pack run); `bt: the_incomplete_history_notice_follows_the_lane_note` | — |
| Height constrained | `bt: the_base_view_height_cap_keeps_the_most_recently_active_lanes`, `the_height_cap_adds_lanes_in_activity_order_until_one_does_not_fit`, `a_focused_view_is_never_cut_by_the_height_cap`, `the_height_cap_keeps_a_merged_lanes_destination_lane`; `wt: a_merged_lane_competes_under_the_height_cap_by_activity`; Kitty short-window test (pre-screenshot checks) | — |
| Parent deleted | `wt: a_deleted_recorded_parent_falls_back_to_the_default_branch` | — |
| Old essential connections | `wt: old_forks_and_merges_stay_drawn_with_exact_elided_runs` (focused and base; `+N` equal to real first-parent distances); `bt: measured_plans_place_merges_and_tags_without_overlap` ("compressed" fixture) | `renderer-spike/old-compressed-repaired.png` |
| Long neighboring labels | `bv: long_labels_on_neighboring_commits_do_not_overlap`, `a_stack_of_three_tags_clears_its_neighbors`, `tags_on_different_lanes_do_not_overlap`; layout test (`long_labels` fixture, both viewports) | `spikes/phase2-renders-output/long-labels-{120x40,56x60}.png` |
| `main` / `origin/main` adjacent | `bv: main_and_origin_main_one_commit_apart_do_not_overlap`; layout test (`main` on `d4`, `origin/main` on `d5`) | as above |
| Width too small | layout test (56-column trimmed-or-shrunk and tag texts unchanged; `ImageWidth::Characters(40)` gives `columns == 40`, `trimmed_commits == 0`); `bt: an_explicit_width_is_never_trimmed_to`, `the_width_cap_trims_commits_before_anything_shrinks`; `trimming_never_elides_a_merge_destination` | `spikes/phase2-renders-output/*-56x60.png` |

Every test named above exists in source (checked by name) and ran green in this
phase's `just test` runs, except the Kitty L2 test (see Final gates).

### Final gates (2026-09-27, this worktree)

A first run of the gates `cd`-ed into the areas through the shell's `CDPATH`,
which resolved `worktree` to the **main checkout** (`~/coding/personal/rusty-biscuit`),
and so tested the wrong tree. It was discarded. Every result below is from a
rerun with absolute paths and `CDPATH` unset, and each log's first line was
checked to be this worktree's path.

| Gate | Result |
|---|---|
| `just test` in `worktree` | 746 passed, 29 skipped (tier-filtered `perf_` and L2); the layout test ran in both targets |
| `just test` in `biscuit-terminal` | 3,344 passed, 55 skipped |
| `just test` in `biscuit-visualized` | 98 passed |
| `just test` in `darkmatter` | 8,498 passed, 12 skipped |
| `just lint` in the same four areas | pass |
| `cargo clippy -p biscuit-terminal --features image --all-targets -- -D warnings` | clean |
| `cargo clippy -p biscuit-visualized --all-features --all-targets -- -D warnings` | clean |
| `cargo clippy -p worktree-cli -p worktree --all-targets --features worktree-cli/terminal-tests -- -D warnings` | clean |
| Kitty L2 (`BISCUIT_TEST_REQUIRED_BACKENDS=kitty cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_graph_in_kitty)'`) | 3 failed, **at the screenshot step only**, for the fourth session running: every earlier check passes, and the window screenshot has chrome and no contents (the calling terminal lacks macOS Screen Recording permission, which a non-interactive session cannot grant) |

The transmitted PNG from this run
(`$TMPDIR/wt-graph-merged-100x32-transmitted.png`) was inspected again:
`fix/wt-ux` has its own lane from `74fbc59`, two commits, and a merge edge
into `730ef34`, the commit tagged `main`. The default lane
(`2419a21 → 74fbc59 → 898c330 → 730ef34`) holds no branch commit, and no
labels overlap. The same picture as Phase 4's kept copies in
`spikes/phase4-kitty-output/`.

`just ci-local --plan` (reviewed, nothing pushed): the change inventory
covers the whole branch against `main` (218 paths, including earlier
sniff, schematic, and list-freshness work). Graph-relevant cells: `worktree`
and `worktree-cli` L1 on all four environments, `worktree-cli` L2 on
`ubuntu-latest` and `macos-latest` (tmux, kitty), `biscuit-terminal` L1 on all
four environments plus its `check` cell compiling 15 dependents (Darkmatter
among them). `biscuit-visualized` has no cell: it is CI-excluded
(`gates = false`), as Phase 4 found, so its tests run only locally and
through `biscuit-terminal`'s and `worktree-cli`'s tests of the real backend.

### Requirement-to-test mapping, targeted tests, and skips (Phase 5 summary)

- Behavior changed in this phase: none (docs, skills, comment-only edits). No
  test was added. The acceptance trace above maps every spec row to its tests.
- Broader gates: the Final gates table.
- Failing and not caused by this phase: the three `level2_graph_in_kitty`
  tests at the screenshot step (environment). No other failures or skips
  beyond the tier-filtered ones.
- Still open for review: the on-screen Kitty screenshot. From a terminal with
  macOS Screen Recording permission, run the Kitty command above and inspect
  `$TMPDIR/wt-graph-merged-{100x32,56x60}-screenshot.png`. Performance for
  human review is in `worktree/docs/performance-testing.md` ("Graph Stages")
  and in Phase 4's table above.

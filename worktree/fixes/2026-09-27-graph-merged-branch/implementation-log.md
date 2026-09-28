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
packages:
    - worktree-cli
    - biscuit-visualized
    - biscuit-terminal
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

---
total_phases: 5
created: 2026-09-27
phase: 5
agent: claude/opus
yolo: true
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

# Plan: `wt list` graph hides a merged branch and overlaps neighboring tags

Specification: [spec.md](spec.md). Renderer feasibility evidence: [renderer-spike.md](renderer-spike.md).

## Summary and Definition of Done

### The work

The graph is wrong for four separate reasons, and each belongs to a different layer:

| Defect | Root cause (verified in source) | Owner |
|---|---|---|
| Merged branch's commits drawn as default-lane history | `focused_view` / `base_view` build lanes with a plain `git log`, which walks merges' second parents | `worktree-cli` (`commands::git_graph`) |
| Merged branch has no lane, or no label at all | Lanes are computed from `tip --not <default tips>`, which is empty once merged. `GraphLine::tip` falls back to the fork commit. `GitGraph::tags` silently drops tags on undrawn commits. `attach_point` hangs a lane whose fork is undrawn from a substitute commit | `worktree-cli` + `biscuit-terminal` (`GitGraph`) |
| No merge edge can be drawn | `GitGraph` emits no `merge` statement. `mermaid-rs-renderer` 0.3.1's `merge` parser uses the whole suffix (`X id: "…" tag: "…"`) as the source-branch name, so a labeled merge loses its second parent | `biscuit-terminal` (emission) + `biscuit-visualized` (parser repair) |
| Neighboring tags overlap | `gitgraph.commit_step` is a fixed 34 units. The renderer does no tag collision avoidance | `biscuit-visualized` (layout configuration) |

`biscuit-visualized::mermaid::MermaidDiagram::compute_layout` is the one function that `natural_size` (measurement) and `render_svg` (rendering) both go through. Putting the parser repair and the spacing policy there does three things. Measurement and rendering use exactly the same effective graph and layout. Every gitGraph caller benefits: `GitGraph`, `bt git-graph`, and Darkmatter's Mermaid blocks. And the renderer gains no new input, so artifact-cache identity only needs the backend-version bump the changed algorithm requires.

### What success looks like

Implementation is **complete and ready for review** when all of the following hold:

1. Every row of the specification's acceptance table has a named, passing test (the mapping is in Phase 5, task "Acceptance trace").
2. In both observed situations (focused view after PR #103, base view after PR #104), a real-Git fixture shows:
   - the merged branch has its own lane;
   - the default lane is first-parent history;
   - forks and merges are drawn at the verified commits;
   - the local `main` label is present.
3. No lane is ever attached at a substitute commit, and no label ever disappears silently. Component tests prove both by omitting fork and tag commits.
4. Tag bounds taken from the same layout that renders the image never overlap. This is proven for long labels on neighboring commits, `main`/`origin/main` one commit apart, stacked tags, and cross-lane tags. Rendered PNGs and a Kitty screenshot back it up visually.
5. Existing gates still pass: the base-view height cap and its notice, the focused-view exemption, explicit-width behavior, the full-command one-second gate, and `just lint` in each touched area.
6. Before and after timings of `graph gather` and `graph image render (biscuit-terminal)` are recorded for the three required scenarios.
7. `worktree/docs/git-graph.md`, `biscuit-terminal/docs/components/git_graph.md`, the `biscuit-visualized` docs and skill (including the stale renderer-version guidance), and the `worktree` and `biscuit-terminal` skills describe the new behavior.
8. The spec remains in `fixes/2026-09-27-graph-merged-branch/`. The agent never moves it to `_completed` and never runs `just complete`.

### Out of scope (restated from the spec)

- Squash-merge inference.
- Reconstructing earlier partial or repeated merges.
- New flags, telemetry, network requests, migrations, or dependency upgrades.
- Changing branch selection, the base-view lane cap, or activity ordering.

## Phase 1 — Rulings, Spikes, and Baselines

Goal: lock down every open planning detail, retire the three risks the renderer experiment left open, and capture "before" performance numbers while the old code is still in place.

### Necessary Rules

The specification says no human rulings remain. These are the planning rulings it delegated. Implementers treat them as binding. A spike result that contradicts one is recorded in `implementation-log.md` and the rule is amended there before any code relies on it.

- **R1 — Ownership.**
  - `biscuit-visualized` owns two generic gitGraph corrections, both applied inside `compute_layout`: repairing the merge second parent, and collision-free tag spacing.
  - `biscuit-terminal`'s `GitGraph` owns lane semantics: explicit tips, merge destinations, pinning essential commits during trimming, never substituting an undrawn commit, and the incomplete-history notice.
  - `worktree-cli` owns Git discovery: branch classification, first-parent lanes, and compressed anchor entries.
  - No dependency upgrade, fork, or patch.

- **R2 — Merge repair mechanism (no new API).**
  - After `parse_mermaid`, for each gitGraph commit with `commit_type == Merge` and exactly one parent, recover the source lane from the merge statement's suffix.
  - The suffix is available as the IR message `merged branch {from} into {branch}`. Match it against the declared `gitgraph.branches` names. It must equal a name, or begin with a name followed by whitespace and one of `id:`, `tag:`, `type:`. Exactly one name may match.
  - The second parent becomes the last commit with `branch == from` and `seq < merge.seq`.
  - A merge that already has two or more parents is left alone. This makes the repair a no-op once the dependency fixes its parser.
  - An unresolvable or ambiguous source, or a source lane with no earlier commit, is an explicit `MermaidError::RenderFailed`. It is never silently skipped. This matches mermaid.js, which refuses to merge an empty branch.
  - A unit test pins the dependency's message format and fails loudly if it changes.
  - The typed merge-metadata alternative from the experiment is rejected. It would fix only callers that opt in, add a new cache-identity input, and duplicate facts the Mermaid text already states.

- **R3 — Tag spacing policy.**
  - For a gitGraph whose layout has at least one tag with `transform == None` (horizontal orientation), `compute_layout` lays out twice.
  - The second pass sets `commit_step = max(default commit_step, widest tag polygon width + TAG_GAP)`.
  - `TAG_GAP` equals the theme's `font_size` (one em), is a named constant, and is documented as policy, not a measured margin.
  - With no tags, or with rotated tags, layout is single-pass and unchanged, and the limitation is documented.
  - `MERMAID_BACKEND` moves from the stale `mermaid-rs-renderer@0.2.x+bv1` to `mermaid-rs-renderer@0.3.x+bv2`, so cached artifacts from the old algorithm are never served.

- **R4 — Branch classification (worktree-cli).**
  - For each selected branch `B` with tip `T`, the destination candidates `X` are tried in order:
    1. the recorded parent's lane, when that parent is drawn;
    2. the default lane tip;
    3. the diverged `origin/<default>` line's tip, when one is drawn.
  - The first `X` with `merge-base --is-ancestor T X` decides. `C` is the oldest line of `git rev-list --first-parent --ancestry-path --parents T..X`.

  | Class | Condition | Drawn as |
  |---|---|---|
  | Unmerged | No candidate contains `T` | Lane of `--first-parent T --not <fork-lane tip>`, fork = `merge-base(fork-lane tip, T)` |
  | Merged directly | `C` has a non-first parent equal to `T` | Lane of `--first-parent T --not C^1`, fork = `merge-base(<recorded parent tip, else C^1>, T)`, `merged_into(C)` |
  | No separate history | `T` is on `X`'s first-parent chain (`C` would be `T`) | Label at `T` on `X`'s lane (anchored if outside the window) |
  | Integrated otherwise | Contained in `X` but neither of the above (indirect integration) | Lane of `--first-parent T --not C^1` with its fork, **no** merge edge, and the incomplete-history notice |
  | Unknown | Any required Git command failed or returned unparseable output | Draw what is verified, mark the connection unknown, show the notice |

  Tip equality is never an input to classification. A merged branch's fork is computed against `C^1` (or the recorded parent), never against the default tip, since `merge-base(main, T) == T` once `T` is merged.

- **R5 — First-parent lanes.** Every lane is its tip's first-parent chain down to its fork: default, recorded parent, branch, and diverged `origin/<default>`. `log`/`rev-list` for lane entries and `+N` counts all pass `--first-parent`. Verbose gathering (`commit_details_since`) keeps its current, non-first-parent semantics.

- **R6 — Essential anchors and compression.**
  - A lane's anchors are:
    - fork commits of lanes that hang from it;
    - merge destinations on it;
    - label commits for branches in the "no separate history" class;
    - the local and `origin/<default>` ref commits (default lane).
  - Anchors are always emitted as `LaneEntry::Commit`. Runs between the ordinary window and anchors become `LaneEntry::Elided(n)`. The ordinary windows are unchanged: `LINE_WINDOW` 5, `BASE_DEFAULT_WINDOW` 10, `CONTEXT_COMMITS` 2.
  - An anchor's position comes from `rev-list --first-parent --count <anchor>..<tip>` and is verified with one batched `rev-parse <tip>~<d>…`. An anchor that fails verification is not on that chain; it is handled by R8, never placed anyway.

- **R7 — `GraphLine` / `GitGraph` API.**
  - `GraphLine::with_tip(sha)` sets the branch's own tip. Label placement uses `tip_sha`, then the last drawn commit, and **never** `fork_sha`.
  - `GraphLine::merged_into(sha)` sets the destination merge commit. Its lane is found by position, so no lane name is needed.
  - `fork_sha == None` now means "connection unknown". It no longer means "tip here".
  - `GitGraph::with_incomplete_history()` lets the caller report gaps.
  - `GitGraphPlan` gains `incomplete: bool`, so tests can assert the notice without rendering.
  - Existing builders keep their spelling.

- **R8 — Undrawn things are accounted for, never substituted.**
  - A tag or label whose commit is not drawn sets `incomplete`.
  - A lane whose fork is not drawn (or unknown) is emitted disconnected, *if Spike S1 confirms the renderer supports it*. Otherwise the lane is omitted and its tip label is placed if drawn. Either way `incomplete` is set.
  - A merge whose destination is not drawn, or would be emitted before the merged lane's commits, is not drawn and sets `incomplete`.
  - The notice is one dim `Prose` line, **"Some history is not shown"** (en-US), placed after the existing "N more worktrees not shown" line.

- **R9 — Unavailable history.**
  - Gathering helpers return `Result<Option<T>, GatherGap>` instead of `Option<T>`. `Ok(None)` is a definitive "no", such as unrelated histories. `Err` is unknown.
  - An unknown result drops only the affected connection and sets `GraphFacts::incomplete`. It never drops the whole graph (today `focused_view` returns `None` on a failed `merge_base`).
  - A git output line that does not parse is a gap, never filtered out.
  - No fetch and no new network request.

- **R10 — Label-only exception.** Applied only when classification positively establishes the "No separate history" class. Never on failure, and never because two tips are equal.

- **R11 — Input Robustness Matrix: not applicable.** This fix adds no reader for a file format, manifest, lockfile, or configuration. The only new parsing is Git plumbing output, which falls under R9's "unparseable is a gap" rule.

- **R12 — Validation dimensions.**
  - Long labels: `feature/very-long-exact-branch-reference-alpha` and `origin/very-long-exact-branch-reference-beta` (from the experiment), plus `main`/`origin/main`, plus a PR tag `PR #104 → main`.
  - Viewports: 120×40 (ordinary) and 56×60 (narrow), matching the existing Kitty L2 narrow window.
  - Explicit width: `--width 40` (never trimmed).

- **R13 — Evidence split.**
  - *Portable* (L1, all four OSes): real-Git gathering fixtures, `GitGraph` output assertions, and `biscuit-visualized` layout-geometry assertions (tag polygons and parent lists from the same `Layout` that renders).
  - *Terminal-specific* (macOS): the Kitty L2 screenshot plus PNG inspection recorded in the implementation log.
  - Snapshots of Mermaid text never count as overlap evidence.

### Spikes

Each spike is a throwaway harness under `worktree/fixes/2026-09-27-graph-merged-branch/spikes/` (never compiled into the workspace). It records a result section in `implementation-log.md`.

- **S1 — Renderer topologies `GitGraph` will emit** (feeds R2, R8):
  - A disconnected lane: `branch X` declared before the root lane's first commit, `checkout X` later.
  - A merge from a lane nested under another lane into the root (the `fix/sniff` shape).
  - A merge into a non-root lane.
  - A merge commit that is also a fork point for another lane.
  - `+N` squares on both sides of a merge.
- **S2 — Tag geometry on real `GitGraph` output** (feeds R3): cross-lane tag rows, tag stacks of 3+, tags beside `+N` squares, and PR tags containing `→`. Can tags on different lanes overlap once `commit_step` is widened? If so, R3 gains a lane-spacing term (`gitgraph` branch spacing), and this spike measures the needed value.
- **S3 — Git discovery commands** (feeds R4, R6, R9):
  - Run `rev-list --first-parent --ancestry-path --parents`, the anchor-distance trick, and `merge-base` on:
    - scripted mirrors of both observations;
    - fast-forward;
    - equal tips;
    - a branch continued after a merge;
    - indirect integration;
    - a `--depth 1` / `--shallow-since` clone.
  - Capture exit codes and stderr for the missing-history cases.
  - Confirm behavior on the git versions of the macOS host, the Linux CI image, and Windows Git. Load the `os` skill for host access.
  - Time the commands on a synthetic 9,000-commit first-parent chain.

### Tasks

**Wave 1** (all concurrent)

- [x] **Record rulings**
  - Copy R1–R13 into `implementation-log.md` under "Rulings", dated 2026-09-27.
- [x] **Spike S1: renderer topologies**
  - Build the harness from `renderer-spike/spike.rs` and assert parent lists, lane (branch) assignment, and tag membership for each S1 topology.
  - Save PNGs and inspect them.
  - Outcome: the R8 disconnected-lane decision is confirmed or amended.
- [x] **Spike S2: tag geometry**
  - Emit real `GitGraph` Mermaid (through `GitGraph::mermaid()`) for the S2 fixtures, then apply R3 spacing in the harness.
  - Report every pairwise tag-polygon intersection across the whole layout.
  - Outcome: R3 stands, or it gains a lane-spacing term.
- [x] **Spike S3: git discovery**
  - Write a shell script that builds each topology in a temp dir and prints command outputs and timings.
  - Outcome: R4's command, R6's distance method, and R9's error shapes are confirmed per OS.
- [x] **Baseline performance (before)**
  - Build three fixtures with the existing `perf_support` helpers:
    - *ordinary* (3 worktrees, 50 commits);
    - *older essential connections* (fork 5,000 first-parent commits back, merge 3,000 back);
    - *multiple selected branches* (8 worktrees, 2 merged).
  - On the current code, record 10-sample medians of `graph gather` and `graph image render (biscuit-terminal)` from `wt list --perf`.
  - Save the numbers in `implementation-log.md`. Commit the fixture builders to `worktree/cli/tests/perf_support/` so the "after" run reuses them unchanged.

**Checkpoint 1**

- [x] Every spike's result is in the log, and R2/R3/R4/R6/R8 are confirmed or amended in the log.
- [x] The baseline table exists.
- [x] No production source has changed yet, apart from new `perf_support` fixture builders.

## Phase 2 — Shared Rendering Layers

Goal: the shared layers can represent merged branches, compressed anchors, and nonoverlapping labels, proven without any Git discovery changes. `biscuit-visualized` and `biscuit-terminal` work proceeds concurrently. Their L1 tests don't depend on each other: component tests use Mermaid text and injected measurement.

**Wave 2** (concurrent: 2a in `biscuit-visualized`, 2b in `biscuit-terminal`)

- [x] **2a. Merge-parent repair** (`biscuit-visualized/src/src/mermaid/render.rs`, new private `gitgraph` module)
  - Implement R2 as `repair_gitgraph_merges(&mut ParsedGraph) -> Result<(), MermaidError>`, called in `compute_layout` right after `parse_mermaid`.
  - Tests:
    - a labeled merge gets parents `[dest-head, source-tip]`;
    - an unlabeled merge is untouched;
    - a two-parent merge is untouched;
    - a nested-lane source works;
    - an ambiguous prefix (`feat` vs `feat x`) is rejected;
    - an unknown lane is rejected;
    - an empty source lane is rejected;
    - the message-format pin test.
- [x] **2a. Tag spacing** (same module)
  - Implement R3 (plus any S2 amendment) inside `compute_layout`, so `natural_size` and `render_svg` share it.
  - Add a test helper, `tag_boxes(&Layout)`, that returns `(commit id, tag text, bounds)` from the layout.
  - Tests, all asserting zero pairwise overlaps and tag membership on the original commit IDs:
    - two neighbors with R12 long labels;
    - `main`/`origin/main` one commit apart;
    - a stack of three tags;
    - cross-lane tags;
    - a no-tag diagram whose layout equals the single-pass layout.
- [x] **2a. Cache identity**
  - Bump `MERMAID_BACKEND` per R3.
  - Add a test that the cache key changes with the backend string (if not already covered).
  - Update the `render_options_json` docs to state that the repair and spacing are derived from the instructions and theme, and add no key input.
- [x] **2b. `GraphLine` explicit tip and merge destination** (`biscuit-terminal/lib/src/components/git_graph.rs`)
  - Add `tip_sha` / `with_tip` and `merged_into` / `merged_into()` per R7.
  - `tip()` never returns `fork_sha`.
  - Update the struct and field docs, including the `fork_sha` doc, which currently says "A line with no commits of its own has its tip here" (drifted once this lands).
- [x] **2b. Merge emission**
  - In `emit_lane`, a destination entry whose SHA is some eligible lane's `merged_into` is emitted as `merge <lane name> id: "<id>"` plus its tags, instead of `commit`. This happens only when the merged lane was emitted earlier in the output.
  - Otherwise emit `commit`, record an undrawn merge, and set `incomplete` (R8).
  - Tests (Mermaid-text level):
    - a merge into default;
    - a merge into the recorded parent's lane;
    - the `fix/sniff` shape (fork from a parent lane, merge into default);
    - a merge commit that is also a fork point;
    - a merge destination tagged `origin/main`.
- [x] **2b. No substitution, full accounting**
  - Replace `attach_point`'s fallbacks with R8.
  - `tags()` records every unplaced tag instead of dropping it.
  - `trim_one_commit` pins merge destinations alongside tips, forks, and tagged commits.
  - `fit_lanes` keeps a merged lane's destination lane as an ancestor.
  - Add `with_incomplete_history()`, `GitGraphPlan::incomplete`, and the R8 notice beside `with_hidden_lanes_note`.
  - Tests:
    - a fork commit omitted from the entries yields no attachment at the default start or the parent start, plus `incomplete`;
    - a tagged commit omitted yields `incomplete`;
    - label-only lines are labeled at `tip_sha`, even when `fork_sha` is drawn;
    - trimming never elides a merge destination;
    - the base-view lane cap keeps a merged lane's destination;
    - the notice text renders in a `Terminal` test after the hidden-lanes note.
- [x] **2b. Existing coverage stays green**
  - Run `just test` in `biscuit-terminal`. Update `git_graph/tests.rs` expectations only where R7/R8 intentionally change output (the fork-fallback tag, the substitute attachment), and name each change in the log.

**Wave 3** (after Wave 2)

- [x] **Real-backend component proof** (`biscuit-terminal/lib/src/components/git_graph/tests.rs` or `lib/tests/l1/`)
  - Build `GitGraph` inputs for the two observations, R12's long-label neighbors, and a compressed old fork/merge, then call `plan()` with the real `biscuit-visualized` measurement.
  - Assert through `biscuit-visualized`'s public layout helper (exposed as a `#[doc(hidden)]` test-support function or behind a `test-support` feature; decide by matching existing crate practice):
    - merge parents are exact;
    - tags sit on the right emitted IDs;
    - there are zero tag overlaps;
    - `plan.columns > viewport.columns` only when fully trimmed.
- [x] **Consumer regressions**
  - Run `just test` in `biscuit-visualized`, `biscuit-terminal` (including `lib/tests/l1/mermaid_parity.rs` and the `bt git-graph` CLI tests), and `darkmatter`.
  - Accept snapshot changes that come only from wider `commit_step` or repaired merges. Review each and list them in the log.
- [x] **Lint**
  - `just lint` in `biscuit-visualized` and `biscuit-terminal`.

**Checkpoint 2**

- [x] Wave 2 and Wave 3 tests pass.
- [x] Rendered PNGs of the two observation fixtures and the long-label pair are generated from the new backend path and visually inspected. Paths and findings are in the log.

## Phase 3 — Git Discovery in `worktree-cli`

Goal: `commands::git_graph` supplies first-parent lanes, verified merges, explicit tips, anchors, and gap reporting, while branch selection stays unchanged.

**Wave 4** (concurrent)

- [x] **Topology helpers** (new `worktree/cli/src/commands/git_graph/topology.rs`; tests in `git_graph/tests.rs`, never `insta` because of the lib/bin double compile)
  - Add `classify(branch_tip, candidates) -> Result<Integration, GatherGap>` implementing R4's table.
  - Add `first_parent_entries(tip, stop, window, anchors) -> Result<(Vec<LaneEntry>, Option<i64>), GatherGap>` implementing R5 and R6 (window + anchors + `Elided` runs; distance verification).
  - Every git call goes through the existing `git_command`, so the tests' git-call recorder counts it.
- [x] **Fixture builders** (`git_graph/tests.rs` helpers, reusing `init_repo`/`commit`/`forked`)
  - `merged_via_merge_commit` (observation 1)
  - `nested_parent_merged_into_default` (observation 2: a side longer than `BASE_DEFAULT_WINDOW`, local `main` one merge behind `origin/main`)
  - `merged_into_parent`
  - `fast_forwarded`
  - `equal_tips_with_history` (C merged at M, and D created at C's tip with recorded parent C)
  - `continued_after_merge`
  - `indirect_integration`
  - `old_connections` (fork and merge beyond the windows)
  - `shallow_clone` (a `--depth` clone from a `file://` origin)
  - `deleted_parent`

**Wave 5** (after Wave 4; the two tasks touch different functions and can run concurrently)

- [x] **Focused view rewrite**
  - `focused_view` uses `classify` for the current branch and its recorded parent.
  - The default lane becomes `CONTEXT_COMMITS` before the anchor, then first-parent entries up to `lane_tip` with the merge destinations and ref commits pinned.
  - Lines get `with_tip` and `merged_into`.
  - A failed `merge_base` no longer returns `None` for the whole graph (R9).
  - `gather`'s verbose half keeps its current semantics.
- [x] **Base view rewrite**
  - `base_view` does two stages, each over `std::thread::scope`:
    1. classify every selected branch in parallel;
    2. build every lane's entries with its anchors in parallel.
  - The default lane is first-parent from `lane_tip`, with `BASE_DEFAULT_WINDOW` plus anchors (fork commits, merge destinations, local `main`).
  - `DefaultTips::diverged_line` becomes first-parent.
  - Selection code (`recorded_parent`, `branches`, `drawn`) is untouched.
- [x] **`GraphFacts` plumbing**
  - Add `incomplete: bool`, and have `to_git_graph` call `with_incomplete_history()`.
  - PR filtering is unchanged.

**Wave 6** (after Wave 5)

- [x] **Gathering tests.** One test per fixture, asserting full SHAs of lane entries, forks, `merged_into`, tips, `Elided` counts, `refs`, and `incomplete`:
  - Observation 1, from the merged branch: the branch has a lane, `merged_into == merge SHA`, and the default lane is first-parent.
  - Observation 1, from `main` in the base view with a tall enough viewport: the same lane survives.
  - Observation 2:
    - `fix/sniff` forks at `fix/wt-ux`'s tip, is merged into `b47a046`'s equivalent, and has `+N` for its long side;
    - the default lane holds no side commits;
    - `fix/wt-ux` is merged at `08cf96a`'s equivalent;
    - the local `main` is tagged.
  - Merged into parent: `merged_into` names the parent-lane commit, and the parent lane anchors it.
  - Recorded parent with and without its own worktree is still selected (existing rule), and the child keeps `with_parent`.
  - Fast-forward: the label is at the actual commit, with no line entries and no merge.
  - Equal tips: C keeps its lane and merge, and D is a label at C's tip.
  - Continued after merge: an unmerged lane forked at the old merged tip, and no merge edge.
  - Indirect integration: a lane without a merge, plus `incomplete`.
  - Old connections: the fork and merge commits are present as `Commit` entries, with `Elided` runs whose counts equal the real first-parent distances.
  - Shallow clone: `incomplete` is set, the verified lanes and labels remain, and the graph is not `None`.
  - Deleted parent: the existing fallback output is unchanged.
- [x] **Existing gather tests**
  - Update `criss_cross_lines_and_verbose_details_hold_tip_unique_commits` and `origin_ahead_extends_the_default_lane_and_diverged_origin_gets_a_line` only where first-parent semantics change the lane, and state why in the test docs.
  - `graph_and_verbose_share_one_merge_base` stays green, or its git-call count is updated with a justification.
- [x] **No new network requests**
  - Add a binary test in `cli/tests/list_remote_head.rs` style.
  - Run `wt list` on the shallow fixture, with a fresh PR store and a fresh remote-head store seeded (`seed_remote_head_store`) and `remote_fixture::UploadPackGate` on the origin.
  - Assert zero upload-pack runs, exit 0, the table present, and the notice text in stdout or stderr as rendered.
- [x] **Sizing regressions**
  - `cargo nextest` runs the existing base-view cap and omitted-worktree notice tests and the focused-view exemption tests unchanged.
  - Add one base-view case where a merged lane competes under the cap and is kept or omitted by the unchanged activity rule.
- [x] **Lint and full L1**
  - `just lint` and `just test` in `worktree`.

**Checkpoint 3**

- [x] All gathering fixtures pass on macOS.
- [x] Git usage is confirmed OS-neutral: results are compared as SHAs, never as paths or localized stderr, and the `os` skill has been consulted for any Windows git differences found in S3.

## Phase 4 — Rendered Evidence, Cross-OS, and Performance

Goal: produce evidence the spec says source snapshots cannot provide, and confirm portability and timing.

**Wave 7** (concurrent)

- [x] **End-to-end layout evidence** (portable L1, `worktree-cli` integration test)
  - For the observation fixtures and a long-label fixture (R12 names as branches and worktrees), run gathering, then `to_git_graph`, then `plan()` at 120×40 and 56×60, then the `biscuit-visualized` layout.
  - Assert:
    - zero tag overlaps;
    - every expected label is on the emitted ID of its exact SHA;
    - merge parents are exact;
    - at 56 columns, `trimmed_commits > 0` or the image shrinks, and no label is shortened.
  - With `--width 40`, assert nothing is trimmed (existing explicit-width contract).
- [ ] **Kitty L2 extension** (`cli/tests/level2_graph_in_kitty.rs`, macOS, private window, no focus)
  - Add a merged-branch fixture run at the existing 100×32 and 56×60 windows.
  - Keep the existing APC and reservation assertions.
  - Save the screenshot and inspect it: the merged lane is visible, the merge edge is at the labeled commit, and no labels overlap.
  - Record the file paths in the log.
  - Run with `BISCUIT_TEST_REQUIRED_BACKENDS` set so a missing backend fails instead of skipping.
- [x] **Cross-OS L1**
  - Run `just test` for `biscuit-visualized`, `biscuit-terminal`, and `worktree` on Linux and native Windows through the hosts in the `os` skill (for example `./scripts/cross-check.sh --os windows worktree-cli`). WSL2 is covered by the nightly schedule.
  - Record the results.
  - Watch for Windows `git` path spelling in fixtures and `LC_ALL` differences in git stderr.
- [x] **Performance (after)**
  - Rerun the Phase 1 fixtures unchanged. Record `graph gather` and `graph image render (biscuit-terminal)` medians beside the baseline, with deltas and an explanation of any regression. The two-pass layout and anchor distance calls are the expected costs.
  - Confirm `perf_command_sla` and the other existing perf gates still pass.
  - Add the table to `worktree/docs/performance-testing.md`.

**Checkpoint 4**

- [x] Layout evidence passes on macOS, Linux, and Windows.
- [ ] The Kitty screenshot has been inspected, and its path is recorded.
- [x] The before/after table is recorded and every regression is explained.

## Phase 5 — Documentation, Skills, and Review Readiness

**Wave 8** (concurrent)

- [x] **`worktree/docs/git-graph.md`**
  - Replace the documented collapse of merged branches into tags. Describe:
    - first-parent lanes;
    - R4's classes with a compact example each;
    - forks and merges at verified commits;
    - anchors and compression;
    - the notice and what triggers it;
    - unchanged selection and sizing.
  - Add a Mermaid diagram of the classification flow.
  - List the agreed limits: no squash inference, no reconstruction of earlier merges, no indirect-integration edge.
  - Never name the fix directory.
- [x] **`biscuit-terminal/docs/components/git_graph.md` and module docs**
  - Update "Lanes and tags" and the renderer-workaround list: `merge` is now emitted, and the parser repair lives in `biscuit-visualized`.
  - Document `with_tip`, `merged_into`, `with_incomplete_history`, `GitGraphPlan::incomplete`, the no-substitution rule, and merge-destination pinning.
  - Drop the module doc's reference to the `2026-09-24-ux-improvements` fix, per the rule that docs don't name specs, in the same edit.
- [x] **`biscuit-visualized` docs and skill**
  - Document the gitGraph merge repair (R2) and the tag-spacing policy (R3), including the rotated-tag limitation and the cache-backend bump.
  - Correct the skill's stale "renderer v0.2" guidance to 0.3.1.
- [x] **Skills**
  - `.claude/skills/worktree/SKILL.md`: update the `commands/git_graph.rs` bullet (first-parent lanes, classification, anchors, `incomplete`).
  - `.claude/skills/biscuit-terminal/`: update the `GitGraph` contract notes.
  - If S3 found a platform-specific git trap, add it to `.claude/skills/os/`.
- [x] **`worktree/docs/cli/list.md`**
  - Update any tag or lane wording that described merged branches as tags.

**Wave 9** (after Wave 8)

- [x] **Acceptance trace**
  - In `implementation-log.md`, map each spec acceptance row to its test name(s) and evidence file(s):

  | Spec case | Proof |
  |---|---|
  | Current branch merged, worktree retained | Wave 6 obs-1 focused test; Wave 7 layout test; Kitty screenshot |
  | Same branch from default | Wave 6 obs-1 base test |
  | Recorded non-default parent ± worktree | Wave 6 parent-selection tests |
  | Merged into non-default parent | Wave 6 merged-into-parent test; Wave 2b emission test |
  | Second observation | Wave 6 obs-2 test; Wave 3 real-backend proof |
  | Merged side longer than base window | Wave 6 obs-2 (default lane first-parent, true forks, local `main` tag) |
  | Fork/label commit undrawn | Wave 2b no-substitution tests |
  | Continued after merge | Wave 6 continued test |
  | Fast-forward / equal tips | Wave 6 fast-forward and equal-tips tests |
  | History unavailable | Wave 6 shallow gather test and no-network binary test |
  | Height constrained | Wave 6 sizing regressions |
  | Parent deleted | Wave 6 deleted-parent test |
  | Old essential connections | Wave 6 old-connections test; Wave 3 compressed proof |
  | Long neighboring labels | Wave 2a spacing tests; Wave 7 layout test; PNG and Kitty inspection |
  | `main` / `origin/main` adjacent | Wave 2a test; Wave 7 layout test |
  | Width too small | Wave 7 56-column and `--width 40` assertions |

- [x] **Final gates**
  - `just lint` and `just test` in `worktree`, `biscuit-terminal`, `biscuit-visualized`, and `darkmatter`.
  - `just test-l2` for the Kitty graph binary on macOS.
    - Run on 2026-09-27: every check passes up to the screenshot step, which is blocked by the missing macOS Screen Recording permission. The screenshot stays open under "Kitty L2 extension"; see the implementation log's Phase 5 "Final gates".
  - Review `just ci-local --plan` so CI scope is understood before any push. The agent does not push unless told to.
- [x] **Comment drift pass**
  - Review `///`/`//!` and inline comments on every changed symbol (`GraphLine`, `tip`, `attach_point`, `tags`, `trim_one_commit`, `focused_view`, `base_view`, `line_entries`, `log_commits`, `compute_layout`). Fix or delete drifted comments and record them in the log.
- [x] **Hand-off**
  - Set the spec frontmatter to `status: implemented`, `implemented: true`, and `implemented_by: claude/opus`.
  - Report "implementation complete, ready for review", including the performance table and screenshot paths for the human review of timing and visual evidence.
  - Do not move the fix directory and do not run `just complete`.

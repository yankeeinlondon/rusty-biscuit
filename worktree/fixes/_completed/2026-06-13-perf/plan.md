---
created: 2026-06-13
phases: 6
start_phase: 1
source_files_during_phase_1:
  - worktree/lib/src/git.rs
  - worktree/lib/Cargo.toml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - worktree
source_files_during_phase_2:
  - worktree/lib/src/worktree.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/Cargo.toml
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - worktree/cli/src/commands/git_graph.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - worktree/cli/src/commands/git_graph.rs
  - worktree/cli/src/commands/list.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - worktree/cli/src/commands/git_graph.rs
  - worktree/cli/src/commands/list.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - docs/topics/testing-strategy.md
docs_created_during_phase_6:
  - worktree/docs/performance-testing.md
skills_files_updated_during_phase_6: []
---

# Plan: `wt list` Performance — Subprocess Orchestration

Source spec: [`spec.md`](./spec.md).

## Conventions

- Build/test the worktree area only: `just -d worktree build`, `just -d worktree test`,
  `just -d worktree lint`, `just -d worktree check`. Avoid `cargo` at the repo root.
- The worktree library owns `git_command` / `git_command_in` (`worktree/lib/src/git.rs`);
  the CLI owns graph/verbose data gathering (`worktree/cli/src/commands/git_graph.rs`).
- UX contract is invariant (spec "Reader's note"): status rows always render, verbose
  text still renders on non-image terminals, the graph is an opportunistic image-only
  enhancement. No phase may change what `wt list` means.
- Tests in this crate run inside the `rusty-biscuit` repo (see `repo_info_from_monorepo`
  in `worktree/lib/src/worktree.rs`), so git-backed unit tests have a real worktree set
  to observe.

## Approach

Six phases, ordered to keep the crate green after each and to let every later phase be
*measured* by the instrumentation introduced in Phase 1:

| Phase | Requirements | Risk | Behavior change? |
| --- | --- | --- | --- |
| 1 — Subprocess instrumentation | R9 | low | no (test-only) |
| 2 — `WorktreeList` snapshot | R2 | low | no (public API shape only) |
| 3 — `--reverse` consolidation | R7 | low | no |
| 4 — Gather struct + full-SHA identity | R3, R4 | medium | no (same output) |
| 5 — Orchestration: gating + parallel + shared gather | R1, R5, R6 | medium | yes (perf only) |
| 6 — Performance docs + acceptance | R8 | low | no |

Phases 1–3 are independent refactors that can land in any order but are sequenced so
Phase 4 builds on all three. Phase 5 is the integration phase where the user-visible
speedup appears. Phase 6 documents and closes out.

---

## Phase 1 — Subprocess-count instrumentation (R9)

Goal: add a test-only counter around the `git_command` / `git_command_in` boundary so
every subsequent phase can prove its subprocess-count claims. Lands first because it is
the measurement tool for the rest of the plan.

### Steps

1. In `worktree/lib/Cargo.toml`, add an internal feature:
   ```toml
   [features]
   count-git = []
   ```
   This is a library-internal test affordance, not part of the public surface.

2. In `worktree/lib/src/git.rs`, add a process-wide recorder gated behind
   `#[cfg(any(test, feature = "count-git"))]`:
   - A small `#[derive(Default)]` struct `GitCallLog` holding `Vec<Vec<String>>` (one
     entry per invocation, each entry the argv).
   - A `std::sync::Mutex<Option<GitCallLog>>` static, plus:
     - `pub fn start_recording()` — resets and installs the log.
     - `pub fn finish_recording() -> Vec<Vec<String>>` — drains and returns entries,
       uninstalling the log (returns `vec![]` if recording was not started).
     - A private `record(args: &[&str])` called from `git_command` and
       `git_command_in` that no-ops when no log is installed.
   - The two real functions gain one line each: `#[cfg(any(test, feature = "count-git"))] record(args);`
   - Production builds (no feature, not test) compile to byte-identical behavior: the
     gate removes every trace.

3. Add helper query fns on the returned `Vec<Vec<String>>` (or as free fns in the same
   cfg block), used by later assertions:
   - `count_matching(|args| predicate) -> usize`
   - `has_any_matching(|args| predicate) -> bool`
   Keep these minimal; later phases add the specific predicates they need.

4. Add a characterization unit test in `worktree/lib/src/git.rs` (`#[cfg(test)]`) that
   starts recording, calls `default_branch()` once, finishes, and asserts exactly one
   entry whose argv starts with `["symbolic-ref", ...]` (or the `rev-parse` fallback).
   This pins current behavior and exercises the recorder end-to-end.

### Validation

- `just -d worktree test` green; `just -d worktree check` with no feature enabled shows
  the recorder is elided.
- `cargo build -p worktree --features count-git` compiles (recorder present).
- The new characterization test fails to compile/green only if the recorder wiring is
  broken — it asserts current (pre-optimization) counts, so it is a baseline, not yet a
  regression guard.

---

## Phase 2 — `WorktreeList` snapshot, surface `default_branch` once (R2)

Goal: make the default-branch name resolved inside `list_worktrees` available to the CLI
so `graph_instructions` and `render_verbose` stop re-deriving it. Spec implementation
decision: direct return-type change via a `WorktreeList` snapshot.

### Steps

1. In `worktree/lib/src/worktree.rs`, add:
   ```rust
   pub struct WorktreeList {
       pub default_branch: String,
       pub statuses: Vec<WorktreeStatus>,
   }
   ```

2. Change `list_worktrees()` signature from
   `Result<Vec<WorktreeStatus>, WorktreeError>` to `Result<WorktreeList, WorktreeError>`.
   The existing `default_branch()?` call at `worktree.rs:135` becomes the snapshot's
   `default_branch` field; the rest of the body is unchanged and still parallelizes via
   `std::thread::scope`.

3. The only consumer is `worktree/cli/src/commands/list.rs:20`. Update `run()`:
   - `let list = list_worktrees()?;` then `let statuses = &list.statuses;` and pass
     `&list.default_branch` down.

4. Update the two downstream callers to take `default_branch: &str` instead of calling
   `default_branch()` themselves:
   - `graph_instructions(statuses, default_branch)` (was `graph_instructions(statuses)`
     at `list.rs:57`).
   - `render_verbose(statuses, default_branch, terminal)` (was at `list.rs:92`).

5. Leave `pub fn default_branch()` in place — the unit test at `worktree.rs:532` and any
   external use still call it. R2 is about the `wt list` run, not removing the function.

### Validation

- `just -d worktree build` green (library + CLI compile against the new shape).
- `just -d worktree test` green.
- Using the Phase 1 recorder in a new cli unit test (call `list_worktrees()` once inside
  the repo), assert **exactly one** argv starting with `["symbolic-ref", ...]`. This is
  the first real regression guard for R2 / acceptance criterion 2.
- Manual: `wt list` output is byte-identical to before this phase.

---

## Phase 3 — `--reverse` + oldest-first query contract (R7)

Goal: push "oldest-first" into the `git log` invocations so the Rust-side `.reverse()`
calls disappear, and fold `parse_commit_lines`'s reversal under the same contract. Pure
refactor; no subprocess-count change (same number of calls), but it simplifies Phase 4.

### Steps

In `worktree/cli/src/commands/git_graph.rs`:

1. `ancestor_commits` (`git_graph.rs:200`): add `--reverse` to the argv and drop the
   `v.reverse()` at `git_graph.rs:205`.
2. `commits_since` (`git_graph.rs:178`): add `--reverse`, drop `v.reverse()` at
   `git_graph.rs:192`.
3. `commit_details` (`git_graph.rs:32`): add `--reverse` to the `git log` argv.
4. `commit_details_since` (`git_graph.rs:42`): add `--reverse`.
5. `parse_commit_lines` (`git_graph.rs:50`): drop the `commits.reverse()` at
   `git_graph.rs:69` since the upstream queries now emit oldest-first.

### Validation

- `just -d worktree test` green.
- Add (or extend) a unit test that calls `ancestor_commits(default_branch, 3)` and
  asserts the returned order matches `git log --reverse` exactly — guards against an
  accidental double-reversal.
- Manual: `wt list -v` verbose commit order is unchanged (oldest first within each
  section).

---

## Phase 4 — Graph-data gather struct + full-SHA identity (R3, R4)

Goal: introduce a single per-run gather that owns merge-base results and commit lists
for a branch, consumed by both the graph and verbose paths. Switch identity comparisons
to full SHAs (`%H`) and derive display IDs in Rust via a shared `display_sha` helper,
eliminating the `short_sha()` subprocess (R4) and the redundant `merge-base` calls (R3).

This phase implements the gather **serially and for the current branch only**; parallel
multi-branch collection and `run()` orchestration land in Phase 5. Keeping it serial here
makes correctness easy to verify before adding concurrency.

### Steps

In `worktree/cli/src/commands/git_graph.rs`:

1. Add a shared display helper (R4):
   ```rust
   /// Render a full SHA as a fixed-width display ID. Branch placement must never use
   /// this — compare full SHAs instead, since git's `%h` abbreviation length is
   /// repository-dependent.
   fn display_sha(full: &str) -> &str {
       &full[..7.min(full.len())]
   }
   ```

2. Add the gather struct (R3, names per spec are flexible but the contract is fixed):
   ```rust
   struct BranchGraphData {
       branch: String,
       merge_base_full: String,
       default_context: Vec<String>,      // full SHAs, oldest-first
       default_after_base: Vec<String>,   // full SHAs on default since base, oldest-first
       branch_after_base: Vec<String>,    // full SHAs on branch since base, oldest-first
       merge_base_detail: Option<CommitDetail>,
       branch_details: Vec<CommitDetail>,
   }
   ```

3. Add a serial gather fn for one branch:
   ```rust
   fn gather_branch(default_branch: &str, branch: &str, verbose: bool) -> Option<BranchGraphData>
   ```
   - One `git merge-base` call → `merge_base_full` (single source of truth; satisfies R3).
   - `ancestor_commits_full(&merge_base_full, 2)` and `commits_since_full(...)`: new
     `%H`-formatting variants (or add a `full: bool`/format param to the existing
     helpers) returning full SHAs. Default- and branch-side lists populated from these.
   - If `verbose`, populate `merge_base_detail` (one `git log` for the base commit) and
     `branch_details` (one `git log` for branch commits since base). If not verbose,
     leave them `None` / empty — do not query what won't be displayed.
   - Returns `None` only if the merge-base itself is unavailable; partial log failures
     degrade to empty lists (feeds R5's per-branch resilience, fully wired in Phase 5).

4. Delete `short_sha()` (`git_graph.rs:218`) and the `git rev-parse --short` call path.
   Its only caller was `base_graph` placement, which Phase 5 rewrites to use full-SHA
   equality against a `%H`-formatted main-commits list.

5. Rewrite `worktree_graph` (`git_graph.rs:225`) to consume a `&BranchGraphData` and emit
   Mermaid lines using `display_sha(...)` for commit IDs. No git calls inside this fn —
   it is now pure formatting. Output must be byte-identical to today for the same inputs.

6. Rewrite `merge_base_commit` (`git_graph.rs:13`) and `branch_commits_detail`
   (`git_graph.rs:20`) to read from `BranchGraphData` instead of calling `get_merge_base`
   themselves. They become cheap accessors; the gather fn did the single `merge-base`.

### Validation

- `just -d worktree test` green.
- Recorder-backed unit test: gather one branch with `verbose=true`, assert **exactly
  one** argv starting with `["merge-base", ...]` for that branch (R3 / acceptance
  criterion 2), and **zero** `["rev-parse", "--short", ...]` (R4 / criterion 3).
- Byte-equality check: capture `worktree_graph` output before (current main) and after
  this phase for a fixed branch and diff — must be identical. (A snapshot test or a
  manual `wt list` screenshot comparison suffices.)

---

## Phase 5 — Orchestration: image-gating + parallel `base_graph` + shared gather (R1, R5, R6)

Goal: wire Phases 2–4 together in `run()` and `base_graph`. This is where the
user-visible speedup lands: non-image terminals skip all graph git calls (R1),
`base_graph` queries branches concurrently with deterministic ordering (R5), and graph
+ verbose share a single gather per the spec's case table (R6).

### 5A — Early image-support / width gating in `run()` (R1)

In `worktree/cli/src/commands/list.rs` `run()`:

1. Compute image support and width **before** any graph data is gathered, reusing the
   existing cheap env logic. Refactor `detect_image_support_from_env` (`list.rs:155`) and
   the stderr-is-tty check so the same detection feeds both the gate and `image_terminal`.
2. Parse `width_spec` early into an `ImageWidth`.
3. Determine `needs_graph`:
   - `image_support != ImageSupport::None`, AND
   - for `ImageWidth::Characters(_)`: `terminal.width() >= MIN_GRAPH_TERMINAL_WIDTH`
     (`list.rs:17`); for `Percent` / `Fill`: always eligible.
   - Only when `needs_graph` is true may any graph-data git call run.
4. `--verbose` is **not** gated by image support (spec R1). `needs_verbose` is the `-v`
   flag set on a non-main current worktree. When `!needs_graph && needs_verbose`, the
   verbose text section still renders using only the verbose data it needs.

### 5B — Implement the gather cases (R6)

Combine `needs_graph` and `needs_verbose` into exactly one gather decision per the spec:

| `needs_graph` | `needs_verbose` | current checkout | Gather |
| --- | --- | --- | --- |
| yes | yes | feature branch | current branch once: graph IDs **and** verbose details |
| yes | no  | feature branch | current branch: graph IDs only |
| yes | *   | base (main)     | base-graph: all branches (graph IDs); verbose is a no-op on base |
| no  | yes | feature branch | current branch: verbose details only |
| no  | no  | *               | nothing |

- When `needs_graph` and the current checkout is the base branch, call the (parallelized,
  5C) base-graph gather over all branches.
- Never gather base-graph branch data when image rendering is unavailable, even if
  verbose is set (verbose on base is already a documented no-op; on a feature branch it
  only needs the current branch).
- `graph_instructions` and `render_verbose` both receive the already-gathered data; they
  no longer call into `git_graph.rs` query helpers directly.

### 5C — Parallelize `base_graph` with deterministic order + per-branch resilience (R5)

In `worktree/cli/src/commands/git_graph.rs`:

1. Add `gather_base_graph(default_branch, branch_names, verbose) -> Vec<BranchGraphData>`
   that collects each branch's data concurrently using `std::thread::scope`, mirroring
   the pattern in `list_worktrees` (`worktree.rs:137`).
2. Per-branch error degradation: a branch whose `merge-base` is unavailable or whose log
   query fails yields no `BranchGraphData` (filtered out), never panics and never
   suppresses sibling branches (spec R5).
3. Deterministic ordering: after collecting, sort results by
   `(merge_base_idx, branch name)` where `merge_base_idx` is computed by full-SHA
   equality against the `%H`-formatted main-commits list (R4). Branches whose merge-base
   falls outside the window anchor at index 0, preserving today's behavior. Output must
   not depend on thread scheduling.
4. Rewrite `base_graph` (`git_graph.rs:268`) to take `&[BranchGraphData]` plus the
   main-commits list and emit Mermaid lines via `display_sha(...)`. Pure formatting, no
   git calls.

### Validation

- `just -d worktree test` and `just -d worktree lint` green.
- Recorder-backed unit tests (Phase 1 tooling) asserting:
  - With `ImageSupport::None` forced and no `--verbose`: **zero** git calls whose argv
    starts with `merge-base` or `log` originate from the graph path (R1 / acceptance
    criterion 1).
  - `--verbose` on a non-image terminal still issues the verbose-path `git log` calls
    and produces the textual commit section (acceptance criterion 8).
  - One `merge-base` argv for the current branch when both graph and verbose are needed
    (acceptance criterion 2).
  - `base_graph` over N branches issues its per-branch queries concurrently (order of
    recorded argv is non-deterministic across runs but the rendered Mermaid is stable —
    assert byte-identical output across two gathers).
- Manual smoke: run `wt list` and `wt list -v` in an image-capable terminal (Kitty /
  Ghostty / iTerm) and in a plain `TERM=dumb` pipe; confirm identical tables/graph/
  verbose text where applicable and that the non-image run is markedly faster.

---

## Phase 6 — Performance testing contract + acceptance (R8)

Goal: add the missing `worktree/docs/performance-testing.md` (an implicit opt-out under
the repo testing strategy otherwise) and run the final acceptance pass.

### Steps

1. Create `worktree/docs/performance-testing.md` with at least these H2 sections (spec R8):
   - `## List Status Collection` — covers `list_worktrees` parallel status/dirty/merge
     work; states the `core.untrackedCache=true` warm-cache assumption.
   - `## Graph Data Collection` — covers `gather_branch` / `gather_base_graph`; states
     that graph data is only gathered when image support is present and width fits.
   - `## Verbose Commit Details` — covers the verbose gather path and that it runs
     independently of image support.
2. State explicitly that **rasterization is excluded** from worktree-owned benchmarks
   (it lives in `biscuit-terminal` / `biscuit-visualized` and is tracked separately).
3. Document the intended Criterion bench surfaces (list-status, graph-data,
   verbose-details) keyed off the Phase 1 recorder so a follow-up can add benches without
   rediscovering scope. Note: the worktree `justfile` has no `bench` recipe today; if
   Criterion is added in this implementation, add a `bench` recipe mirroring the shared
   `/just` bench pattern, otherwise leave the recipe for a follow-up and say so here.
4. Cross-link this doc from any repo-level testing index if one exists (grep
   `performance-testing.md` to find siblings; match their cross-link style).

### Final acceptance pass

Walk every item in spec §Acceptance Criteria:

1. Non-image `wt list` → recorder shows zero graph-path git calls.
2. `wt list -v` → one `default_branch`, one `merge-base` for the current branch.
3. `short_sha()` removed; no `rev-parse --short` in recorder output.
4. `base_graph` concurrent + deterministic (byte-stable across runs).
5. Image-capable `wt list` / `wt list -v` output byte-identical to pre-change baseline.
6. All pre-existing unit tests pass; changed-signature helpers have updated tests.
7. Warm-cache wall-clock for `wt list` (non-image) and `wt list -v` (image) under the
   1-second SLA from `sniff/fixes/_completed/2026-04-21-performance/spec.md`, excluding
   rasterization.
8. `wt list -v` on a non-image terminal prints the verbose section.
9. `worktree/docs/performance-testing.md` exists with the required sections.

### Validation

- `just -d worktree test`, `just -d worktree lint`, `just -d worktree check` all green.
- `just -d worktree doctest` if the worktree recipe supports it (check justfile; skip
  with a note if absent).

---

## Cross-cutting Validation Checkpoints

- After Phase 1: recorder compiles under `count-git`, elided otherwise; baseline
  characterization test green.
- After Phase 2: one `default_branch` call per `wt list`; output unchanged.
- After Phase 3: oldest-first order comes from `git log --reverse`; output unchanged.
- After Phase 4: one `merge-base` per branch; no `short_sha` subprocess; graph output
  byte-identical.
- After Phase 5: non-image runs skip graph git calls; `base_graph` parallel + stable;
  full speedup realized.
- After Phase 6: perf doc in place; all acceptance criteria signed off.

## Risk Notes

- **Public API change (R2):** `list_worktrees()` return type changes. This package is
  pre-1.0 and the CLI is the only consumer (verified: `list.rs:20`). Still, call it out
  in the commit message; `default_branch()` itself stays public.
- **SHA abbreviation correctness (R4):** the current code compares a `rev-parse --short`
  merge-base against `%h` main commits. Switching to full-SHA equality is more correct,
  but verify the anchor-at-index-0 fallback (`git_graph.rs:285` region) still triggers in
  the same cases, or branch placement could shift. The Phase 4 byte-equality check is the
  guard.
- **Concurrency + shared mutable recorder (R5):** the Phase 1 recorder is process-wide
  under a `Mutex`, so concurrent `base_graph` queries will interleave argv entries. Tests
  must assert on *counts/presence*, never on recorded order, for the parallel path.
- **Feature-flag leakage:** ensure `count-git` never slips into a release profile or a
  non-test dependency edge. The `#[cfg(any(test, feature = "count-git"))]` gate plus not
  enabling the feature in `[dependencies]` covers this; double-check `worktree-cli`'s
  dev-dependency enables it only under `dev-dependencies`.

## Out of Scope / Confirmed Excluded

- Mermaid→SVG→image rasterization in `biscuit-terminal` / `biscuit-visualized` (spec
  Non-Goals).
- Changing "dirty" badge semantics / untracked-file walk (spec Non-Goals).
- Long-lived `git` process or `git2` binding (spec Non-Goals).
- Adding Criterion benches themselves — Phase 6 documents the surfaces; the benches and a
  `just bench` recipe are an explicit follow-up unless the implementer chooses to include
  them (then wire through the shared `/just` bench pattern).

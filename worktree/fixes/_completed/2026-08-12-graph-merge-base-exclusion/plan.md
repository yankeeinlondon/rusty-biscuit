---
title: Exclude opposite tips from git graph lanes
status: ready
created: 2026-08-12
phase: 4
total_phases: 4
agent: codex/default
yolo: true
spec: worktree/fixes/2026-08-12-graph-merge-base-exclusion/spec.md
packages:
  - worktree-cli
source_files_during_phase_1:
  - worktree/cli/src/commands/git_graph.rs
docs_updated_during_phase_1:
  - worktree/fixes/2026-08-12-graph-merge-base-exclusion/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - worktree/cli/src/commands/git_graph.rs
docs_updated_during_phase_2:
  - worktree/fixes/2026-08-12-graph-merge-base-exclusion/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - worktree/docs/cli/list.md
  - worktree/docs/performance-testing.md
  - worktree/docs/git-graph.md
  - worktree/fixes/2026-08-12-graph-merge-base-exclusion/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - worktree/fixes/2026-08-12-graph-merge-base-exclusion/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - worktree/cli/src/commands/git_graph.rs
documentation:
  - worktree/docs/cli/list.md
  - worktree/docs/performance-testing.md
  - worktree/docs/git-graph.md
  - worktree/fixes/2026-08-12-graph-merge-base-exclusion/plan.md
---

# Execution plan

## Objective

Make focused and base-overview git graph lanes, elision counts, and verbose
branch details represent commits reachable from one tip but not the opposite
tip. Preserve the selected merge-base only as the shared-context endpoint,
fork anchor, and verbose merge-base detail so graph counts agree with the
already-correct `wt list` ahead/behind table even in criss-cross histories.

## Execution constraints

- Limit production changes to `worktree-cli`; the `worktree` library's
  ahead/behind cache and table calculation are already correct and are out of
  scope.
- Keep the existing two-lane Mermaid model. Do not add merge edges, first-parent
  filtering, or multiple-merge-base anchor selection.
- Preserve the gather subprocess budget: one `merge-base` per branch, the same
  `git log` count, the same `rev-list --count` count, and no new
  `rev-parse --short` calls.
- Treat plain `git merge-base A B` as nondeterministic when multiple best bases
  exist. Tests must work across Git versions and macOS, Windows, and Linux
  without depending on hashes, timestamps, or merge-base output order.
- Use the package-area `just` recipes and nextest-backed test path. Do not run
  `cargo test` or `cargo fmt`.
- Tasks within a phase are dependency-ordered unless explicitly marked
  parallelizable. Phase 3 documentation tasks may run in parallel after the
  production field names and semantics in Phase 2 are settled.

## Phase 1 — Establish selection-independent regression coverage

### Requirement-to-test map

- Fixture topology and merge-base selection independence:
  `criss_cross_fixture_has_two_incomparable_best_bases`.
- Focused lane membership, disjoint rendering, and exact dot/elision counts:
  `criss_cross_focused_graph_uses_tip_unique_commits`.
- Verbose oldest-first feature uniqueness:
  `criss_cross_verbose_details_use_feature_unique_commits`.
- Base-overview feature uniqueness and selected merge-base insertion:
  `criss_cross_base_graph_uses_unique_commits_at_selected_base`.

These are Level 1 tests because they exercise the real Git/filesystem boundary
without requiring a terminal, browser, or host input. This phase changes no
parser, schema, template, prompt, configuration artifact, or persisted value,
so passive-corpus and read/write/read coverage are not applicable.

- [x] **Task 1.1 — Add reusable git-fixture helpers.** In the
  `git_graph.rs` test module, factor only the setup needed to create commits and
  non-interactive merges, retaining the repository-local identity,
  `commit.gpgsign=false`, `gc.auto=0`, `core.fsmonitor=false`, and
  `core.commitGraph=false` settings used by existing fixtures. Ensure merge
  inputs modify separate files so fixture construction never opens an editor or
  conflict-resolution prompt.

- [x] **Task 1.2 — Build `temp_repo_with_criss_cross_merge`.** Construct two
  incomparable histories that are each incorporated into both `main` and the
  feature tip, including the feature-line back-merge needed to reproduce shared
  history outside either single selected base. Return or resolve symbolic refs
  and full SHAs inside the test rather than hard-coding generated commit IDs.

- [x] **Task 1.3 — Prove the fixture's bug preconditions.** Add assertions that
  `git merge-base --all main <feature>` yields exactly two distinct best bases
  as an unordered full-SHA set, neither base is an ancestor of the other, and
  the base selected by plain `git merge-base` leaves at least one commit that is
  reachable from both tips but not from that selected base. This prevents a
  topology mistake from making the regression tests vacuous.

- [x] **Task 1.4 — Add full-SHA lane inspection helpers.** Parse focused and
  base-overview Mermaid branch segments structurally (checkout/branch
  boundaries and commit nodes), and compare their IDs through the gathered
  `CommitId.full` values. Do not use substring searches that can confuse a SHA
  appearing on both lanes; keep elision nodes and the synthetic `HEAD` node
  distinct from real commits.

- [x] **Task 1.5 — Add focused-graph and verbose regression tests.** On the
  criss-cross fixture, assert that `BranchGraphData` contains exactly
  `git log --format=%H --reverse <tip> --not <opposite-tip> --` for each lane,
  that the rendered lane sets are disjoint, each rendered dot count equals
  `git rev-list --count <tip> --not <opposite-tip> --`, no elision marker is
  present when both unique counts are at most five, and verbose
  `branch_details` match the feature-only oldest-first log with no commit
  reachable from `main`.

- [x] **Task 1.6 — Add base-overview regression coverage.** Gather and render
  the feature through `gather_base_graph`/`base_graph`; assert its lane contains
  only feature-unique commits and that, when the plain selected merge-base is in
  the ten-commit main window, the branch block is inserted at that selected
  base's derived index. Never assume which of the two best bases Git selects.

- [x] **Validation checkpoint 1.7 — Record the red proof.** Run the new
  criss-cross tests alone with the worktree area's nextest-backed `just test`
  filtering support. Confirm the topology/precondition assertions pass while
  the lane, elision, verbose, and/or base-overview assertions fail against the
  current merge-base-exclusion implementation for the expected reason. Do not
  weaken the fixture to accommodate the old behavior.

### Phase 1 evidence

- `just test criss_cross_fixture_has_two_incomparable_best_bases` passed in
  both `worktree-cli` test targets, proving the fixture topology independently
  of plain merge-base selection.
- `just test criss_cross` passed the two topology instances and failed all six
  desired-behavior instances (three tests in both CLI targets). Each failure
  showed one shared best-base commit in the actual lane/detail data but absent
  from the opposite-tip-excluded expectation.
- The three desired-behavior tests are ignored with Phase 2-specific reasons
  after recording the red proof, matching the repository's phased fail-first
  convention while keeping the ordinary Level 1 suite green. Phase 2 removes
  those ignores without changing the assertions.
- Package-area `just test` passed: 37 `worktree` tests and 103 active
  `worktree-cli` tests. The three fail-first contracts are ignored in each of
  the CLI's library and binary test targets.
- Package-area `just lint` passed for `worktree` and `worktree-cli`.

## Phase 2 — Change graph gathering to symmetric tip exclusion

- [x] **Task 2.1 — Rename the gathered lane contract.** In
  `BranchGraphData`, replace `default_after_base` and `branch_after_base` with
  `default_unique` and `branch_unique` (or equally explicit final names), and
  rename their hidden-count fields if needed so every symbol describes unique
  reachability rather than time or position after a merge-base. Update all
  focused/base renderer consumers atomically.

- [x] **Task 2.2 — Switch the full gather queries.** In the `GatherScope::Full`
  path, gather default-lane commits and hidden count using
  `default_branch --not branch`, and gather branch-lane commits and hidden count
  using `branch --not default_branch`. Leave `default_context` rooted at the
  selected merge-base and leave `merge_base_detail` unchanged.

- [x] **Task 2.3 — Switch base-overview and verbose queries.** Make
  `GatherScope::BaseOverview` use `branch --not default_branch` for both its
  window and hidden count, and make verbose `branch_details` use the same
  exclusion. Ensure both scopes still share the one already-resolved
  `merge_base_full` for anchor placement without issuing another merge-base
  query.

- [x] **Task 2.4 — Repair source documentation drift.** Rename the
  `hidden_since` exclusion parameter from `merge_base` to `exclude` and update
  `BranchGraphData`, `GatherScope`, gather-function, accessor, helper, and
  renderer docs/comments to distinguish unique lane/detail data from
  merge-base context and anchoring. Delete stale “since merge-base,” “since
  divergence,” and “post-divergence” claims wherever they describe the changed
  data.

- [x] **Task 2.5 — Update existing expectations and linear guards.** Change
  `worktree_graph_uses_in_process_display_ids` and any query-derived expected
  values to exclude the opposite tip. Keep
  `worktree_graph_marks_elided_commits` asserting `7 - 5 = 2` and `6 - 5 = 1`,
  and add explicit characterization that the existing
  `temp_repo_with_branches` and `temp_repo_over_window` focused graph output is
  byte-identical under legacy single-base and new opposite-tip set selection
  because those histories have a single dominating merge-base.

- [x] **Task 2.6 — Cover elision beyond the display window.** Using the
  over-window fixture (or a narrowly extended criss-cross variant), assert each
  `+N` is exactly
  `rev-list --count <tip> --not <opposite-tip> - 5`, appears once, and remains
  fork-adjacent before the five displayed commits on its lane.

- [x] **Validation checkpoint 2.7 — Run focused correctness tests.** Run the
  complete `git_graph.rs` unit-test module through `just test`. Confirm the new
  criss-cross focused, verbose, base-overview, and elision tests are green;
  existing deterministic rendering, display-ID, placeholder, and linear
  fixture tests remain green.

- [x] **Validation checkpoint 2.8 — Verify the process budget.** Confirm
  `gather_branch_uses_one_merge_base_and_no_short_sha` and
  `base_graph_subprocess_count_is_bounded` pass with their existing exact
  counts: no additional merge-base, log, rev-list, or short-SHA subprocesses.

### Phase 2 evidence

- The three selection-independent criss-cross regressions pass in both
  `worktree-cli` targets after removing their Phase 2 ignores: focused lanes,
  verbose details, and base-overview rendering now exclude the opposite tip.
- `linear_focused_graph_is_unchanged_by_opposite_tip_exclusion` and
  `over_window_focused_graph_is_unchanged_by_opposite_tip_exclusion` prove
  byte-identical focused output for the two existing single-base fixtures.
- `worktree_graph_marks_elided_commits` derives both hidden counts from
  opposite-tip `rev-list --count`, asserts one marker per lane, and verifies
  each marker precedes exactly five displayed commits.
- `just test git_graph` passed all 30 module-test executions across the CLI
  library and binary targets. The Full-scope budget guard passed with one
  merge-base, five log, two rev-list, and zero short-SHA calls; the base scope
  retained one merge-base, one branch log, and one rev-list per branch plus its
  single main-window log.
- Package-area `just test` passed 37 `worktree` tests and 113 active
  `worktree-cli` tests; 14 non-Level-1 tests were skipped by the canonical
  filter. Package-area `just lint` passed for both packages.

## Phase 3 — Align user and forward-looking documentation

- [x] **Task 3.1 — Update focused-list semantics.** In
  `worktree/docs/cli/list.md`, define each focused lane as commits unique to its
  tip (not reachable from the opposite tip), retain the merge-base description
  only for the two shared context commits, and define verbose feature details
  as the full oldest-first feature-unique sequence. **Parallelizable with Tasks
  3.2 and 3.3.**

- [x] **Task 3.2 — Update performance terminology.** In
  `worktree/docs/performance-testing.md`, replace post-divergence and
  `default_after_base` language with unique-tip collection terminology while
  preserving the documented gather boundaries and subprocess counts. Recompute
  this Markdown file's frontmatter hash with Darkmatter (`md hash <file>`) after
  its body is final. **Parallelizable with Tasks 3.1 and 3.3.**

- [x] **Task 3.3 — Correct the future `GitGraph` reference model.** In
  `worktree/docs/git-graph.md`, update focused/base-overview algorithms, proposed
  data-field comments, verbose semantics, placeholders, edge cases, and test
  expectations so a later extraction accepts already-filtered unique commits
  and cannot reintroduce merge-base exclusion. Preserve merge-base anchor and
  context roles and the document's explicitly simplified two-lane rendering.
  **Parallelizable with Tasks 3.1 and 3.2.**

- [x] **Validation checkpoint 3.4 — Audit terminology and hashes.** Use `rg`
  across `git_graph.rs` and the three affected documents for “after_base,”
  “since the merge-base,” “since divergence,” and “post-divergence.” Review each
  remaining match and retain it only when it truthfully describes merge-base
  context/anchoring or historical material, then verify Darkmatter reports the
  hashed performance document as current.

### Phase 3 evidence

- `worktree/docs/cli/list.md` now defines focused lanes as commits unique to
  each tip, selected merge-base commits as shared context/anchors, and verbose
  feature details as the full oldest-first feature-unique sequence.
- `worktree/docs/performance-testing.md` now describes unique-tip collection
  without changing the documented gather boundaries or subprocess counts. Its
  Darkmatter hash was refreshed with `md hash --save`, and `md hash --diff`
  reported no semantic changes afterward.
- `worktree/docs/git-graph.md` now requires callers and collectors to provide
  already-filtered unique commits, keeps the selected merge-base limited to
  context/detail/anchor roles, and covers zero-unique placeholders and
  selection-independent multiple-best-base behavior.
- The required case-insensitive `rg` audit across `git_graph.rs` and all three
  documents found no occurrences of `after_base`, `since the merge-base`,
  `since divergence`, or `post-divergence`.

## Phase 4 — Package validation and acceptance audit

- [x] **Task 4.1 — Run the worktree unit suite.** From the package area, run
  `just test` and require all `worktree` and `worktree-cli` Level 1 tests to
  pass under nextest, including the selection-independent criss-cross fixture.

- [x] **Task 4.2 — Run static validation.** Run `just check` and `just lint` in
  the worktree package area. Resolve errors without broad formatting or
  unrelated cleanup; do not run `cargo fmt`.

- [x] **Task 4.3 — Audit every acceptance criterion.** Map AC1–AC8 from
  `spec.md` to named passing tests or documentation diffs. Explicitly verify
  disjoint lanes, exact unique counts, verbose ordering, selected-base anchor
  placement, unchanged linear graphs, unchanged subprocess bounds, corrected
  terminology, and cross-platform/selection-independent fixture behavior.

- [x] **Task 4.4 — Review scope and working-tree diff.** Confirm production
  code changes are confined to `worktree-cli`, no dependencies or lockfiles
  changed, no real merge-edge or cache behavior was introduced, and only the
  specified source, tests, documentation, and plan files were touched. Record
  the commands and results for implementation handoff.

## Completion criteria

- [x] Both focused and base-overview lanes are the symmetric difference of the
  default and feature tips, and graph/elision counts agree with the `Commits`
  table by construction.
- [x] Verbose branch details contain only feature-unique commits in oldest-first
  order; merge-base context, anchor placement, and merge-base detail retain
  their prior roles.
- [x] The criss-cross fixture proves multiple incomparable best bases and stays
  correct regardless of the base selected by plain `git merge-base`.
- [x] Linear-topology graph output and exact git subprocess budgets are
  unchanged.
- [x] `just test`, `just check`, and `just lint` pass, affected documentation is
  semantically current, and Markdown hashes validate.

### Phase 4 evidence

- `just test` passed 37 `worktree` tests and 113 active `worktree-cli` tests;
  the canonical Level 1 filter skipped 14 higher-tier tests.
- `just check` and `just lint` passed for both packages in the worktree area.
- AC1–AC8 map to the named criss-cross, elision, linear-characterization, and
  subprocess-budget tests recorded in the Phase 1 requirement-to-test map and
  Phase 2 evidence. All passed in the package-area suite.
- `rg` found no remaining `after_base`, `since the merge-base`,
  `since divergence`, or `post-divergence` terminology in the changed source
  and documentation. `md hash --diff worktree/docs/performance-testing.md`
  reported no semantic changes; the other changed documents do not declare a
  stored hash.
- `git diff --check -- worktree` passed. The tracked diff contains only
  `worktree/cli/src/commands/git_graph.rs` and the three specified public and
  forward-looking documents; the fix directory contains only `plan.md` and
  `spec.md`. No library source, manifest, dependency, or lockfile changed.
- GitNexus change detection reported low risk and no affected execution flows,
  though its repository index was two commits stale and refresh failed on an
  invalid-UTF-8 indexing error. `sniff repo package-dependencies worktree-cli`
  corroborated the package boundary; the edited graph implementation remains
  private to `worktree-cli`.

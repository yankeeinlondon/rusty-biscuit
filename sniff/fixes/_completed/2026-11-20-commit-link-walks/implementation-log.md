---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/sniff/fixes/2026-11-20-commit-link-walks/spec.md"
plan: "sniff/fixes/2026-11-20-commit-link-walks/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - sniff
    - sniff-detection-cli
source_files_during_phase_2:
    - sniff/lib/src/filesystem/git/commit_links.rs
    - sniff/lib/src/filesystem/git/remote_refresh.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/src/bin/render_git_status_fixture.rs
docs_updated_during_phase_2:
    - sniff/docs/topics/repo/recent-commits.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/sniff/architecture.md
source_files_during_phase_3:
    - sniff/lib/src/error.rs
    - sniff/lib/src/filesystem/mod.rs
    - sniff/lib/src/filesystem/git/mod.rs
    - sniff/lib/src/filesystem/git/types.rs
    - sniff/lib/src/filesystem/git/discovery.rs
    - sniff/lib/src/filesystem/git/remote_refresh.rs
    - sniff/lib/src/filesystem/git/status_report.rs
    - sniff/lib/src/filesystem/git/status_report_tests.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/src/filesystem/repo/scope.rs
    - sniff/lib/src/filesystem/repo/aggregate_view.rs
    - sniff/cli/src/commands/mod.rs
    - sniff/cli/src/commands/repo.rs
    - sniff/cli/src/output/repo_json.rs
    - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_3:
    - sniff/docs/cli/repo_git-status.md
    - sniff/docs/topics/repo/recent-commits.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/sniff/architecture.md
    - .claude/skills/sniff/performance.md
decisions_for_review:
    - |-
      A selected linked worktree whose refs cannot be proven shared normally
      takes one fresh ref snapshot. I skip that snapshot when the history
      count is zero, because nothing is decorated or linked then. The plan
      did not address the zero-count worktree case. The alternative is to
      always take it (one extra ref read, no visible difference). Reversing
      it is a one-line change in status_report.rs selected_worktree_refs.
    - |-
      The generic text renderer and the generic JSON builder keep their
      git-status branches (the renderer passes no link map), although the
      CLI now always dispatches git-status to the focused handler first, so
      those branches are not reached from the CLI. The plan says the
      generic renderer keeps using the shared heading helper, so I kept
      them as working code rather than unreachable!() markers. Turning them
      into unreachable!() would make an accidental fall-through loud;
      nothing else would need redoing.
    - |-
      The new library tests live in status_report_tests.rs, attached to
      status_report.rs as its test module with #[path], rather than inline
      in status_report.rs as the plan worded it. The file is about 600
      lines, and the sibling *_tests.rs pattern already exists in git/.
      Moving them inline is mechanical.
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
    - sniff/lib/src/filesystem/git/commit_links.rs
    - sniff/lib/src/filesystem/git/remote_refresh.rs
    - sniff/lib/src/filesystem/git/types.rs
    - sniff/lib/src/filesystem/git/discovery.rs
    - sniff/lib/src/filesystem/git/mod.rs
    - sniff/lib/src/filesystem/git/status_report.rs
    - sniff/lib/src/filesystem/git/status_report_tests.rs
    - sniff/lib/src/filesystem/mod.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/src/filesystem/repo/scope.rs
    - sniff/lib/src/filesystem/repo/aggregate_view.rs
    - sniff/lib/src/error.rs
    - sniff/cli/src/commands/mod.rs
    - sniff/cli/src/commands/repo.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - sniff/cli/src/output/repo_json.rs
    - sniff/cli/src/bin/render_git_status_fixture.rs
    - sniff/cli/tests/l1/cli.rs
documentation:
    - sniff/docs/cli/repo_git-status.md
    - sniff/docs/topics/repo/recent-commits.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/sniff/performance.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-11-20-commit-link-walks (4 phases)

## Phase 1

### Capture baseline (complete)

No source files were edited in this phase. Everything below was measured before any Phase 2 change.

**Build.** `cargo build --release --locked -p sniff-detection-cli --bin sniff` finished as already fresh (0.87 s) against the current source. Binary: `/home/build/wt/rusty-biscuit/fix-biscuit-terminal/target/release/sniff`, invoked with `--base /home/build/wt/rusty-biscuit/fix-biscuit-terminal`.

**Repository state at sampling.**

- Source HEAD: `fce8e78bba99d2c874b39eae068f09cc753f1016` on `fix/biscuit-terminal`. This differs from the `fb35e1684` noted at planning time: the branch has gained commits since then, but the workload shape below is unchanged.
- Remote refs: 26 under `refs/remotes/origin/`, including the symbolic `origin/HEAD -> refs/remotes/origin/main` (`36097348949d…`), which `remote_branch_tips` excludes. The full name/ID list is saved as `target/commit-link-walks-baseline/remote-refs.txt` (md5 `ab99d39475562ab63a939bef11d32d4c`). Phase 4 should diff against this file to prove the tips are still pinned.
- Distinct reachable commits over the 25 non-symbolic remote tips: **16,682** (`git rev-list --count <tips>`). HEAD alone reaches 16,683.
- Ten displayed targets (`git log -10 HEAD`): **10 of 10 are absent** from every remote branch (`git branch -r --contains` returned 0 for each).
- Commit graph: **absent**. `/home/build/coding/rusty-biscuit/.git/objects/info/` (the common dir) holds no `commit-graph` file or `commit-graphs/` chain.

**Samples.** I ran one warm-up per mode, then three interleaved runs each of `repo git-status --perf` and `repo git-status --json --perf`. Wall time is measured externally (`date +%s%N` around the process). Load average was 0.82 before and 1.22 after, on 16 cores. Raw stdout, stderr and wall times are in `target/commit-link-walks-baseline/`, which is ignored by `.gitignore` `**/target/*`.

| Run | text `--perf` wall | JSON `--json --perf` wall |
| --- | --- | --- |
| warm-up | 3524 ms | 150 ms |
| 1 | 3494 ms | 146 ms |
| 2 | 3537 ms | 150 ms |
| 3 | 3539 ms | 147 ms |
| **median** | **3537 ms** | **147 ms** |

- Reported in-process `Total` (text): 136.7 / 131.8 / 137.4 ms. JSON `total_duration_ms`: 136.3 / 140.7 / 137.7 ms. Text and JSON report the same detection cost, so the ~3.4 s text excess is entirely post-snapshot renderer work.
- Reported `git.commit_visits`: **84** in every text run, every JSON stdout `performance` object, and every JSON stderr diagnostic. The other counters are identical in both modes: `ref_walks` 1, `repository_discoveries` 1, `repository_opens` 1, `status_walks` 2.
- Linking stage: **absent** in both modes. The stages are `detect.filesystem`, `detect.total`, `filesystem.docs`, `filesystem.git`, `filesystem.inventory`, `filesystem.repo`, and no `filesystem.git.commit_links` appears. As the plan expects, the baseline counters exclude the renderer's `commit_links_at` lookups: they run after `CliPerf::build_report`, so the ~3.4 s of independent ancestry walks is invisible to the counters.
- The acceptance target for Phase 4 is terminal status within 0.5 s of focused JSON. The baseline gap is about 3.39 s (3537 − 147 ms).

### Gates

The phase changed only the plan and this log. I ran this from the repository root after saving all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

It reported `packages: []` and `change_class: documentation`, so this phase skipped `just test` and `just lint` in `sniff/`. Phase 1 added no tests, so there is no requirement-to-test mapping for it. Phase 2 owns the first behavior tests.

### Cross-OS rigs

No local test rigs are configured. `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL` and `BUILD_MACOS` are all unset, so the baseline is Linux-only, as the plan specifies.

## Phase 2

### Prune completed ancestry (complete)

- `remote_refresh.rs`: added `walk_ancestry_with_filter(repo, tip, budget, completed: Option<&HashSet<ObjectId>>, visit)`. It builds the same newest-first, commit-graph-enabled walk and calls `all()` without a set or `selected(|id| !completed.contains(..))` with one. gix checks the predicate before queueing, so pruned commits cost neither a visit nor an object read. `walk_ancestry` is now a thin wrapper and stays the deep tier's independent walk (`populate_recent_commit_remotes_from_snapshot` is unchanged). Budget and counter semantics are unchanged: `GIT_COMMIT_VISITS` counts only budgeted yields, and a walk whose last yield spends the last visit still reports `Completed`.
- The filter looks up `id.to_owned()` and not the borrowed `&oid`, because the `Hash` impls of `ObjectId` and `oid` are not guaranteed to agree.
- `RefSnapshot` gained a private `remote_branches_observed` flag, set by `observe` from its `remote_branches` argument, and a crate-private `has_remote_branch_tips()`. `link_commits_from_snapshot` now returns `CommitLink::default()` (unknown) for every target when the flag is false. Every production caller already observed remote branches (`link_commits_with_budget`; `types.rs:1303` for the aggregate), so production output is unchanged.
- `commit_links.rs`: the loop moved into private `link_commits_from_snapshot_with_walk(.., share_visits, walk)`. With sharing on and more than one ordered tip, it allocates the completed set, skips a covered tip before walking, buffers the current walk's IDs in a reused `Vec`, and merges them only on `Completed`. On `Failed` it drops the set (`completed = None`), so the remaining tips walk independently. `OverBudget` stops the pass, and `Stopped` publishes nothing. Doc comments were updated for sharing, the coverage guard, and `walk_ancestry`'s new role.

### Consume captured links (complete)

- `build_git_status_items` and `render_git_section` take a final `Option<&HashMap<String, CommitLink>>` and look up each displayed commit by full SHA. `commit_links_at` is no longer called while rendering. Unit callers and `render_git_status_fixture` pass `None`.
- The heading calculation moved from the `render_text` `GitStatus` arm into crate-private `output/mod.rs::render_repo_git_status`, which the generic arm calls with `None`. Phase 3's focused handler will supply the map. Until then the generic terminal path renders hashes unlinked; see `message_to_agent`.

### Requirement-to-test mapping

| Behavior | Test (L1) | Boundary / fidelity |
| --- | --- | --- |
| Exact visits equal the distinct reachable remote history; absence is proven at that budget, and one visit short is unknown. Preferred and later URLs, duplicate tip, ancestor tip, duplicate target in order | `commit_links::tests::containment::shared_visits_prove_absence_within_the_distinct_reachable_history` | crate-internal linking over a real git2 repository (F2) |
| Independent walks exhaust the same budget (control) | same test, `share_visits = false` | F2 |
| A failed walk publishes nothing: later walks get no filter, a winner from the failed walk survives, a later tip finds the missed ancestor, another absent target stays unknown. The control (no failure) gives origin the ancestor and proves absence | `…::a_failed_walk_leaves_later_tips_walking_independently` | real walker with an injected stop, translated to `Failed` |
| Shallow boundary (ODB-written commit with its parent omitted, plus a `shallow` file) is recognized by gix; shared and independent results match; shared visits 3 vs independent 4 | `…::shallow_boundaries_link_alike_shared_and_independent` | F2 |
| Coverage guard: default, decoration-only and local-only snapshots give unknown with zero visits; an observed snapshot links; an observed empty set proves absence | `…::snapshots_without_remote_tip_coverage_link_unknown` | F2 |
| Existing priority, default branch, timestamp-skew, snapshot and budget coverage | the existing `containment` tests, unchanged and passing | F2 |
| Renderer uses captured URLs by full SHA (two IDs sharing a seven-character prefix), falls back to the hash for the other, lists oldest first, and does zero Git work (nonexistent `/repo`, fresh collector, all counters zero; the old lookup incremented `git.discoveries`) | `output::filesystem::tests::git_status_rendering::captured_links_match_full_ids_and_rendering_reads_no_repository` | CLI crate renderer boundary, markup level |
| Compact and synthetic terminal tests | existing `compact_git_status_omits_meta_section` and the others in `git_status_rendering`, passing | — |

The Input Robustness Matrix does not apply: no parser or configuration loader changed.

### Gates

- Targeted: `cargo nextest run -p sniff --lib -E 'test(commit_links) | test(remote_refresh)'` passed 53 of 53, and the CLI `git_status_rendering` slice passed 26 of 26.
- `just test` (in `sniff/`): **3630 passed, 33 skipped, exit 0**, with `🐢 L1 run recorded: 3630 test(s), none at or above 5.0s`. Load average was about 3.3 on 16 cores.
- `just lint`, `cargo clippy -p sniff --all-targets -- -D warnings` and `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`: all clean.
- `git diff --check`: clean.
- The scope calculator was not used to skip gates, because source files changed.
- Dependents, run serially and targeted, because only `RecentCommits::collect` reaches the changed loop from outside Sniff: Darkmatter `-E 'test(recent_commits)'` passed 9 of 9. Claudine and claudine-cli `-E 'test(history) | test(recent)'` passed 13 of 13. No dependent uses `render_git_section`; the only crate that depends on `sniff-detection-cli` is Sniff's own CLI.
- No skipped or pre-existing failures.

### Docs and skills

- `sniff/docs/topics/repo/recent-commits.md`: added an example of shared visits, the unchanged winner, partial and failed walks, and shallow boundaries.
- `.claude/skills/sniff/architecture.md`: the linking bullets now describe the filtered walker, the fallback after a failed walk, the coverage guard and the test seam. The rest of the guide updates belong to Phase 3.

### Cross-OS rigs

No local rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL` and `BUILD_MACOS` are unset). The changes have no OS-specific paths; the shallow fixture writes `.git/shallow` through `repo.path()`.

## Phase 3

### Prepare library status (complete)

- `filesystem::detect_repo_git_status(root, &GitStatusOptions) -> Result<PreparedGitStatus>` is the new focused entry point. `GitStatusOptions` and `PreparedGitStatus` live in the new `git/status_report.rs` and are re-exported from `git` and `filesystem`. It builds the same request the CLI used to (full, or deep with `refresh_remotes`; structure-tier repo detection only for a package or area selector; no docs, formatting or inventory) and records `detect.filesystem` and `detect.total` as `detect_repo_aggregate` does.
- `detect_filesystem_with_request_inner` takes a private `FilesystemGitMode` (`Standard`, `Aggregate`, `Status(&options)`) instead of the aggregate bool, and returns `FilesystemDetection { filesystem, aggregate_git, commit_links }` (renamed from `AggregateFilesystemDetection`; `aggregate_view.rs` reads `aggregate_git`). In status mode the Git worker records `filesystem.git.initial`, and `filesystem.git` stays unmeasured so the perf tree sums its children.
- `GitRepo::detect_with_request` now wraps crate-private `detect_with_ref_snapshot(request, include_link_tips)`, which returns the snapshot it took. Link demand (`include_link_tips && wants_commits`) is folded into both the snapshot predicate and its remote-branches argument after any refresh fetch, so detached `HEAD` still observes every tip in the one ref walk.
- `RefSnapshot` gained `has_decorations()`, `local_branch_tip`, `remote_branch_tip`, `with_head_branch` (re-marks `is_head` and re-sorts) and `is_worktree_independent()`. That last flag is computed during `observe`: true only when every observed symbolic ref is a `refs/remotes/<r>/HEAD` alias of a directly observed branch of the same remote.
- `discovery::get_commits_for_branch_from_snapshot` starts from captured local or remote tips and reuses captured decorations. A miss (a symbolic remote `HEAD`, a SHA, a branch missing from the snapshot) takes the old targeted lookup and `resolve_remote_or_sha`. `get_commits_for_branch_fallible` now delegates to it with no snapshot, so the two cannot drift.
- `status_report::finish_status` runs after the worker and repo detection join. It resolves package and area scope through the moved `repo/scope.rs::resolve_package_and_area` (now `SniffError::RepositorySelection`, same messages), even with no Git facts. It then applies path history and working-tree narrowing, a different branch (clearing working-tree changes), and the worktree selector, in that order, under `filesystem.git.selection`. Finally it links the first `history_count` entries of the final list under `filesystem.git.commit_links`. Linking is skipped (empty map, no stage) when it is disabled, the display is empty, the snapshot is missing, or the snapshot did not observe remote tips.
- Worktree selection keeps the branch-or-basename match, the `Worktree not found: … (available: …)` message, and one `GitRepo::discover`. History is best effort, as before. Status comes from one `get_repo_status_with_changes(repo, false)` instead of two scans. The initial snapshot is reused when both handles have equal canonicalized common dirs (raw `canonicalize` on both sides, compared only with each other, per the `os` skill), equal namespaces and equal effective replacements (`objects.store_ref().replacements()`, empty when `ignore_replacements`), and neither linked handle has a private `refs/` directory (a failed probe counts as unproven), and the snapshot is worktree-independent. Otherwise the selected handle takes one snapshot (decorations, plus tips when linking). A failed one gives undecorated history and no links. A worktree that disappeared keeps the prepared history and status, updates root and branch, and links nothing. URLs always come from the selected handle's config.

### Update behavior guides (complete)

- `sniff/docs/cli/repo_git-status.md`: documented `--branch` and `--worktree` (flags table, usage, selector order with examples); the commit-link pipeline (collected during the request after selection, plain and zero-count skips, the perf stages); and the focused JSON contract (one value, no link fields, use `recent-commits --json`). I corrected drift in the same pass: file lines are commits, staged, unstaged, untracked, then conflicted (last, not first); an unlinked hash is dim italic, not plain bold; the `blast_radius` paths named files that no longer exist (`args.rs`, `commands.rs`, `output/filesystem.rs`, `git.rs`).
- `sniff/docs/topics/repo/recent-commits.md`: explained why shallow clones need no separate path (the boundary is repository-wide), and why the `--refresh-remotes` membership walk stays independent. Shared visits, failed-walk fallback and tri-state containment were already there from Phase 2.
- `.claude/skills/sniff/architecture.md`: the focused-status pipeline, the snapshot companion, the branch reader, the scope resolver and the worktree ref-sharing rules. `.claude/skills/sniff/performance.md`: the three stages, when the status snapshot is taken, the unmeasured parent, and what the shared visit counter means.

### Wire focused command (complete)

- `RepoAction::GitStatus` is now dispatched from the early arm in `commands/mod.rs::run` to `commands/repo.rs::handle_repo_git_status`. The base is the canonicalized `--base`, or the current directory with `detect_with_plan_inner`'s fallback. Linking is requested for text with a positive count unless `--plain`, whether or not stdout is a terminal. The library call runs inside one `CliPerf::collect`, and `build_report` is taken once afterwards for every output: text perf on stdout, and JSON's embedded report plus the stderr diagnostic. Absent Git prints `{}` for JSON and nothing for text.
- I removed the generic status-plan arm, the GitStatus special cases for refresh and history in `run`, and the three post-detection projections (scope, branch, worktree). The remaining `--package`/`--package-area` validation in `run` and the two callers in `commands/repo.rs` use the library resolver, and the CLI copies of the resolvers are deleted. `repo_json::git_status_value` takes `Option<&GitInfo>` and serves both the focused and the generic paths.

### Verify command paths (complete)

Added in `cli/tests/l1/cli.rs`: a fixture (`status_views_fixture`) built on `create_cli_monorepo_distinct_area_and_package`, with origin at an alpha commit, an unpushed beta HEAD, a `side` branch, and a linked worktree `wt` checked out in `wt-dir`. I also added text perf parsing that ignores escapes, glyphs and alignment.

### Requirement-to-test mapping

| Behavior | Test | Boundary / tier |
| --- | --- | --- |
| Default status: links keyed by exactly the displayed full IDs (unpushed `Some(false)`, pushed URL), one discovery, one ref walk, the `initial`/`selection`/`commit_links`/`detect.*` stages, no measured `filesystem.git` | `status_report::tests::default_status_links_the_displayed_history_with_one_discovery_and_ref_walk` | library public entry, real git2 repo (F2), L1 |
| Detached HEAD observes the remote tips in the one ref walk | `…::detached_head_still_observes_remote_tips_in_the_one_ref_walk` | F2 |
| Disabled linking and `history_count 0` give an empty map and no stage | `…::disabled_or_zero_count_linking_runs_no_linking_stage` | F2 |
| Bare/empty/current branch is a no-op (history and working tree kept); another branch replaces history, clears the working tree, links only its commits, reuses the captured tips and decorations (1 ref walk); `origin/main` resolves | `…::a_bare_or_current_branch_keeps_the_selection_and_another_branch_replaces_it` | F2 |
| Case-insensitive package, area prefix with canonical casing, intersection, intersection error, unknown package/area messages, narrowed changes and counts, excluded commits never linked, decorations reused | `…::package_and_area_select_path_history_and_narrow_the_working_tree` | F2 + real structure detection |
| Selector validation outside a repository; no-repo result is unlinked | `…::selectors_are_validated_outside_a_repository` | F2 |
| History zero keeps every matching path commit (uncapped), links none, path reader does its own decoration read | `…::zero_history_keeps_every_matching_path_commit_and_links_none` | F2 |
| Selected worktree: its history, status and changes, `HEAD` re-marked on its branch, root and branch, basename selector, missing-worktree message, shared refs = 1 ref walk | `…::a_selected_worktree_supplies_history_status_and_head_from_shared_refs` | F2, real linked worktree |
| `extensions.worktreeConfig` URL from the selected worktree over shared refs | `…::worktree_config_supplies_the_selected_worktrees_urls_over_shared_refs` | F2 |
| A shared symbolic branch targeting per-worktree `refs/worktree/pin` forces a fresh snapshot (2 ref walks) with the worktree's own decoration | `…::a_branch_resolving_through_per_worktree_refs_takes_a_fresh_snapshot` | F2 |
| Scope + branch + worktree together: final display only, earlier selector errors (unknown package, corrupt branch ref) survive a later selector | `…::combined_selectors_apply_in_order_and_link_only_the_final_display` | F2 |
| Missing snapshot gives an unlinked report | `…::a_missing_snapshot_produces_an_unlinked_report` | crate-internal `finish_status` |
| `--refresh-remotes` against a bare remote whose tip moved (fetch rewritten with `url.<path>.insteadOf`): links and deep `remotes` use post-fetch refs, 1 ref walk; the stale run leaves the commit unpushed | `…::explicit_refresh_links_against_post_fetch_refs_without_another_ref_walk` | F2, real `git fetch` subprocess |
| CLI default, compact, bare and current `--branch`, `-p`, `-a`: final history, short-hash fallback, no URL when piped | `cli::test_repo_git_status_views_show_their_final_history` | spawned CLI (F3), L1 |
| CLI `--branch side`, package + branch, `--worktree`, area + worktree by basename, Clap conflict, unknown worktree | `cli::test_repo_git_status_later_selectors_replace_earlier_history` | F3 |
| Text perf, ordinary and detached: one linking stage, one discovery, one ref walk | `cli::test_repo_git_status_text_perf_measures_linking_with_one_discovery_and_ref_walk` | F3 |
| Scoped (`-p`) linked text vs `--plain` vs `--json` perf: visit delta = distinct reachable remote history (2); plain and JSON have no linking stage; JSON equals plain; JSON perf diagnostic on stderr | `cli::test_repo_git_status_scoped_linking_adds_exactly_the_remote_history` | F3 |
| `--history 0` text and JSON visit parity, with and without `-p`; no linking stage | `cli::test_repo_git_status_zero_history_links_nothing_with_or_without_scope` | F3 |
| Focused JSON with and without `--perf` is one value, keeps the GitInfo shape, has no link/url fields, writes no stderr without perf | `cli::test_repo_git_status_json_is_one_unlinked_git_info_value` | F3 |
| Renderer consumes the supplied URL by full ID with no repository access | Phase 2's `captured_links_match_full_ids_and_rendering_reads_no_repository` (unchanged) | renderer boundary |

The Input Robustness Matrix does not apply: no parser or configuration loader changed. All new tests are L1, live in declared targets (`sniff` lib unit tests; `sniff-detection-cli` `l1` binary), and none carries a tier marker.

### Gates

- Targeted: `cargo nextest run -p sniff --features remote -E 'test(status_report)'` passed 13 of 13. The CLI slice `-E 'test(test_repo_git_status_)'` passed 15 of 15. The broader status slice (git_status, corrupt, absent, aggregate_commit_families, branch, worktree, package, recent_commits, commit_links, remote_refresh, discovery) passed 888 of 888.
- `just test` (in `sniff/`): **3649 passed, 33 skipped, exit 0**, with `🐢 L1 run recorded: 3649 test(s), none at or above 5.0s`. Load average about 2 on 16 cores.
- `just lint`, `cargo clippy -p sniff --all-targets -- -D warnings` and `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`: clean. The first strict run flagged `large_enum_variant` on `LinkSource`, so the worktree handle is now boxed.
- `git diff --check`: clean apart from a trailing blank line at the end of this log, now removed.
- I did not use the scope calculator to skip gates, because source files changed.
- Dependents, run serially and targeted at the callers of the changed detection and history paths: Darkmatter `test(git_context)` 9/9; Claudine `test(invocation_context) | test(environment) | test(system_prompt)` 160/160; claudine-cli `test(package_context)` 1/1; rendezvous-daemon `test(repos)` 4/4. `cargo check` of the other direct dependents (worktree, git-wt-cli, tree-hugger-cli, messenger, model-citizen, playa, biscuit-speaks, research, unchained-ai, simplified-schema, biscuit-terminal-cli, darkmatter-lsp) was clean. The one downstream `match` on `SniffError` (Darkmatter `provider.rs`) has a wildcard arm, so the new variant breaks nothing.
- No skipped or pre-existing failures.

### Quick timing sample

This is informational; Phase 4 owns acceptance. Release build at HEAD `cba9ab138` plus the uncommitted Phase 2 and 3 tree, load average about 5. Text `repo git-status --perf`: 293 and 267 ms. JSON: 154 and 151 ms. Text reports `commit_visits` 16,768 against JSON's 86; the delta, 16,682, equals the Phase 1 union. Ref walks 1, discoveries 1, linking stage about 109 ms. The baseline was 3,537 ms against 147 ms.

### Cross-OS rigs

No local rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL` and `BUILD_MACOS` are unset), so everything above ran on Linux. The OS-sensitive pieces are the common-dir comparison (canonicalized on both sides and compared only with each other) and the refresh test's `insteadOf` value (a forward-slash native path). CI's macOS and the nightly Windows and WSL2 legs will exercise them.

## Phase 4

### Run Sniff gates (complete)

Phase 4 changed no source, docs or skills files; it changed only this log, the plan and the spec frontmatter. The gates ran against HEAD `45075c7a4`, which is the committed Phase 2 and 3 tree, with a clean working tree. Load average was 1.4 to 3.5 on 16 cores throughout.

- `just test` (in `sniff/`): **3649 passed, 33 skipped, exit 0**, with `🐢 L1 run recorded: 3649 test(s), none at or above 5.0s`. No SLOW, FAIL or TIMEOUT lines.
- `just lint`: exit 0, no warnings.
- `cargo clippy -p sniff --all-targets -- -D warnings`: clean. `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`: clean.
- `git diff --check fce8e78bb HEAD` (the whole fix, from the Phase 1 baseline commit): clean.
- Docs and comments: I grepped the Sniff docs, the skill and the CLI source for the removed renderer lookup (`commit_links_at`). The one hit is `.claude/skills/sniff/performance.md`, which describes the old behavior as history ("Earlier the renderer's `commit_links_at` ran…") and is accurate. No drift was found, so nothing was edited.
- Dependents: nothing changed after Phase 3's serial, targeted dependent runs (Darkmatter, Claudine, claudine-cli, rendezvous-daemon, plus `cargo check` of the other direct dependents), so I did not repeat them.

### Measure status speed (complete)

**Pinning.** `git for-each-ref refs/remotes/origin/` is byte-identical to `target/commit-link-walks-baseline/remote-refs.txt` (md5 `ab99d39475562ab63a939bef11d32d4c`). The union over the 25 non-symbolic tips is still **16,682**, all 10 displayed targets are still absent from every remote branch, and the common dir still has no commit graph.

HEAD is `45075c7a4`, not the baseline's `fce8e78bb`. This is unavoidable: the fix itself is two commits on top of the baseline. The displayed history, ten unpushed commits, and the remote tips have the same shape. The JSON visit count rose from 84 to 88 for the extra history depth, which is why the comparison below uses the linked-minus-JSON delta rather than absolute counts.

**Sample.** Release build (`cargo build --release --locked -p sniff-detection-cli --bin sniff`, already fresh) at `/home/build/wt/rusty-biscuit/fix-biscuit-terminal/target/release/sniff`, with `--base` set to the worktree. One warm-up per mode, then three interleaved runs, first without perf flags (acceptance), then with `--perf` (diagnostics). Wall time was measured externally. Load average was 3.05 before and after. Raw stdout, stderr and wall times are in `target/commit-link-walks-after/` (ignored).

| Run | text | JSON | text `--perf` | JSON `--perf` |
| --- | --- | --- | --- | --- |
| warm-up | 260 ms | 143 ms | 256 ms | 147 ms |
| 1 | 256 ms | 144 ms | 264 ms | 144 ms |
| 2 | 259 ms | 138 ms | 255 ms | 143 ms |
| 3 | 254 ms | 145 ms | 263 ms | 145 ms |
| **median** | **256 ms** | **144 ms** | **263 ms** | **144 ms** |

- **Acceptance met:** terminal status is 112 ms over focused JSON (median 256 vs 144 ms), against a 0.5 s allowance. The baseline gap was about 3,390 ms (3,537 vs 147 ms), and text status is about 13.8× faster.
- Visits: text `--perf` reports `git.commit_visits` **16,770** in all three runs; JSON reports **88** in all three. The delta, **16,682**, equals the pre-counted distinct reachable remote history exactly, so each reachable commit is visited once across the 25 tips.
- `ref_walks` 1 and `repository_discoveries` 1 in both modes.
- Linking stage `filesystem.git.commit_links`: 113.7, 109.8 and 113.9 ms; `filesystem.git.initial` 131 to 135 ms; selection under 1 µs. In-process totals: text 252.0, 244.1 and 251.7 ms; JSON 135.4, 134.3 and 135.0 ms. JSON's stages are `detect.filesystem`, `detect.total`, `filesystem.docs`, `filesystem.git.initial`, `filesystem.git.selection`, `filesystem.inventory` and `filesystem.repo`, with no linking stage. The baseline recorded no linking stage either, and the 3.4 s spent in the renderer went uncounted.
- Output channels: without perf, piped stderr is empty in both modes, and JSON stdout parses as one value. With `--json --perf`, stdout is one JSON value with an embedded `performance` object, and the human diagnostic is on stderr.
- **Saving over independent walks** comes from the isolated tests, which I reran: `shared_visits_prove_absence_within_the_distinct_reachable_history` proves absence at exactly the distinct reachable budget, where independent mode exhausts it. `shallow_boundaries_link_alike_shared_and_independent` gives 3 shared visits against 4 independent. `a_failed_walk_leaves_later_tips_walking_independently` also passes (3 of 3). On the motivating workload, 25 independent walks would each revisit most of the shared `main` history. That is the roughly 3.4 s baseline cost.

No timing miss, so no breakdown or repeat sample was needed.

### Record review evidence (complete)

- Fixture markup and fallback are proven by Phase 2's renderer test, `captured_links_match_full_ids_and_rendering_reads_no_repository`. On this workload, the piped text output carries no hyperlinks (`0` OSC 8 sequences), as expected: all ten displayed targets are unpushed, so each line takes the short-hash fallback.
- Departures from the plan: none in this phase. Earlier departures are in `decisions_for_review`.

### Requirement-to-test mapping

Phase 4 added no tests and changed no behavior. Acceptance evidence is the release sample above. The behavior-to-test mapping is in the Phase 2 and Phase 3 sections.

### Gate-skip check

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Run after all Phase 4 edits were saved, it reported `packages: []` and `change_class: documentation`. The gates ran anyway (above), so this result was not used to skip them.

### Cross-OS rigs

No local rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL` and `BUILD_MACOS` are unset), so every Phase 4 measurement and gate ran on Linux. CI's macOS leg and the nightly Windows and WSL2 legs provide the other environments.

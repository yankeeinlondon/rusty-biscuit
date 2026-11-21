---
total_phases: 4
created: 2026-11-20
status: planned
phase: 4
agent: codex/gpt-6.2-sol
yolo: true
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
---

# Reuse shared Git history when resolving commit links

## Summary and Definition of Done

Reuse completed ancestry visits within one library link pass. Resolve status links against the final displayed history using the repository and refs already acquired for the request. Render the captured links by full commit ID. Preserve the history and JSON contracts, selection behavior and independent deep remote-membership walk.

The fix is done when tests prove exact visit savings, preference and shallow-history parity, failed-walk recovery, selection precedence, full-ID URL association, and zero Git acquisition during rendering. Ordinary and detached status must each perform one discovery and ref enumeration. Plain, zero-display and focused JSON requests skip linking. The final performance report includes scoped preparation and linking. All four Sniff gates must pass, and terminal status must finish within 0.5 s of focused JSON on the Linux checkout below. Record the evidence in implementation-log.md beside this plan.

## Phase 1 — Establish the baseline

### Necessary Rules

**Ancestry**

- Keep `git/commit_links.rs::link_commits_from_snapshot` as the single linking authority. Its callers are `link_commits_with_budget` through `git/api.rs::commit_links_at` and `commit_browser_url` (including CLI `repo hash`), and `git/recent_commits/collect.rs` for the focused and aggregate commit families. Preserve `preferred_remote_order` and the stable default-branch-first ordering via `RefSnapshot::remote_default_branch`. The first remote to reach a target wins even without a browser URL. Emit results in original target order, including duplicates.
- Add `remote_refresh.rs::walk_ancestry_with_filter` taking the existing arguments plus an optional borrowed `HashSet<gix::ObjectId>` of completed visits. Keep `walk_ancestry` as the independent wrapper. Construct the same newest-first, commit-graph-enabled platform, using `all()` with no set and `selected()` with membership filtering otherwise. gix 0.90.0 `revision/walk.rs::Platform::selected` forwards the predicate to `gix_traverse::commit::Simple::filtered`, and `commit/simple.rs::insert_into_seen_and_queue` checks it before timestamp lookup and queue insertion. Use the same algorithm in shallow repositories, with the pinned walk's existing shallow boundary handling. Keep `populate_recent_commit_remotes_from_snapshot` on the independent wrapper, since it records every containing remote and applies its own branch limit.
- Allocate a completed set only for multiple ordered tips. Skip already-covered tips before constructing walks. Buffer yielded IDs for the current tip in a reusable vector, then merge only on `Completed`. On `Failed`, discard the buffer and completed set, retain matches, mark the pass incomplete, and run every remaining tip independently. On `OverBudget`, stop with unresolved targets unknown. On `Stopped`, all targets have winners and the pass ends without publishing the buffer. Keep the five-million-visit limit and increment `GIT_COMMIT_VISITS` only for budgeted successful yields. A walk that uses the last visit on its last yield can still complete. These are the existing `AncestryWalkEnd` and budget semantics in `remote_refresh.rs`.
- Factor the loop behind private `link_commits_from_snapshot_with_walk`, accepting the normal inputs, a sharing flag, and a walker callable taking the repo, tip, mutable budget, optional completed set, and mutable visitor. Production enables sharing. Tests disable it for the independent control or inject a deterministic failed walk. This gives the failure test access to a real successful later walk through the same commit.
- Add a private remote-tip coverage flag to `RefSnapshot`, set by `observe` from its remote-branches argument and readable by a crate-private `has_remote_branch_tips()` accessor. A decoration-only or default snapshot returns unknown links from `link_commits_from_snapshot`. A successfully observed empty list proves absence. The standalone and aggregate link callers already ask for remote tips.

**Status collection**

- Add `filesystem::detect_repo_git_status(root: &Path, options: &GitStatusOptions<'_>) -> sniff::Result<PreparedGitStatus>`. Define the types in new `git/status_report.rs` and re-export them through `git/mod.rs` and `filesystem/mod.rs`. Options carry `history_count: usize`, `refresh_remotes: bool`, `include_commit_links: bool`, and `package`, `package_area`, `branch`, and `worktree` as `Option<&str>`. The result carries `git: Option<GitInfo>` and `commit_links: HashMap<String, CommitLink>`, with no serialization contract. Keep `GitInfo`, `CommitInfo`, `DetectionPlan`, `GitRequest` presets, and public history APIs compatible. These shapes serve other request and serialization paths.
- Construct the same focused filesystem request as `cli/src/commands/mod.rs::run`. Use full or deep Git with the requested count, structure-only repo detection for a package or area selector, and disable docs, formatting, and inventory. In `filesystem/mod.rs::detect_filesystem_with_request_inner`, replace the aggregate bool with private `FilesystemGitMode` variants `Standard`, `Aggregate`, and `Status(&GitStatusOptions)`. Rename `AggregateFilesystemDetection` to `FilesystemDetection`, with `aggregate_git` and `commit_links` companions. Update `repo/aggregate_view.rs::detect_repo_aggregate` to use the renamed field. This retains the existing `FilesystemObservation` discovery and the observed Git root for package scans.
- In `git/types.rs`, keep `GitRepo::detect_with_request` as a wrapper over crate-private `detect_with_ref_snapshot(request, include_link_tips) -> Result<(GitInfo, Option<RefSnapshot>)>`. The status Git worker returns the pair. Other modes return normal Git facts with no snapshot companion. Fold positive-count link demand into both the existing snapshot predicate and its remote-branches argument, after the existing fetch. `detect_with_request` skips tracking demand on detached HEAD, and `RefSnapshot::observe` treats decorations and tips separately. The full and deep status requests already enumerate branches, and their positive-count history already peels remote refs for decorations, so the flag adds no separately fallible read. Existing status and history failures propagate. Unavailable link input yields an unlinked report.
- Finish selection in `status_report.rs` after joining the Git worker and obtaining the package catalog. Reuse `FilesystemObservation::with_repository` and the captured snapshot. Move `commands/mod.rs::resolve_package_and_area` and its private helpers to `filesystem/repo/scope.rs` and re-export the resolver via `repo/mod.rs`. Use `SniffError::RepositorySelection(String)` with verbatim display for invalid selections, including a missing worktree. Preserve case-insensitive exact package, area prefix, trailing slash, intersection, valid-name lists, and validation even without Git facts. The remaining CLI validation in `commands/mod.rs` and the two callers in `commands/repo.rs` use this shared resolver.
- Keep projection order from `commands/mod.rs::run`: path history and changed-file/count filtering, explicit different branch history with cleared working-tree changes, then selected worktree history and status. Run earlier readers and preserve their errors even when a later projection replaces their result. A bare, empty, or current branch selector keeps the current selection. In `discovery.rs` add `get_commits_for_branch_from_snapshot` alongside the public-path fallible reader, using captured local and remote branch tips for matches and the existing targeted lookup and `resolve_remote_or_sha` for misses, including symbolic remote HEAD and SHA lookups. Reuse the captured decorations in both the path and branch readers. At history zero preserve the path reader's uncapped count with its existing scan limit and preserve branch resolution and errors. Scoped readers with a zero-count snapshot lacking decorations do their existing lazy read. Text display and linking still take zero entries. The Clap declaration in `args/repo.rs` rejects simultaneous branch and worktree flags. Preserve that guard and test the full precedence chain through the library entry point.
- Keep the worktree selector match on branch or directory basename, the missing-worktree message, and one `GitRepo::discover(&info.filepath)` for the selected checkout. Use that handle for history, status and URL configuration, via `with_cached_gix`. Keep the selected worktree's best-effort HEAD history reader and replace the two status accessors with one `get_repo_status_with_changes(repo, false)`, defaulting both results on failure as before. The former accessors ran the same scan. Discovery errors propagate. A disappeared checkout preserves the already prepared history and status and updates the selected root and branch as before. Its links are unavailable. Keep deep containment at the initial detection step with the 50-tip preset limit. Scoped path, branch, and worktree readers replace the initial recent list and preserve their current lack of per-commit deep membership, as in the old CLI projection.
- Share refs across worktrees when both handles have equal canonical common directories, namespaces, and effective object replacements, and the captured refs have no worktree-dependent targets. Compare replacements through `Repository::objects.store_ref().replacements()`, treating `ignore_replacements` as an empty sequence. Require no private admin `refs` directory that could shadow shared entries, and require that observed relevant symrefs are remote HEAD aliases to directly observed branches of the same remote. Record that property when taking the snapshot. gix's `store/file/overlay_iter.rs` can overlay checkout-specific refs, while branches, remote-tracking refs and tags normally come from the common Git directory. Unproven equality, including a failed path probe, takes one snapshot on the selected handle for both decorations and requested link tips. Shared refs reuse the captured decorations with `is_head` re-marked and sorted for the selected handle's `head_name()`. A failed selected-worktree snapshot keeps the existing best-effort history with empty decorations and no links, matching `GitRepo::recent_commits` and `ref_decorations`. Use the selected handle's effective config for URLs even when sharing refs.
- Invoke linking after final selection for the first `history_count` records in `GitInfo.recent`, using the final handle and snapshot. Keep the original history list for serialization. Store links by full SHA, using a default link for an invalid ID. An empty display, disabled linking or unavailable link input produces an empty map and no linking stage. A captured empty remote tip list yields known absence. Time the actual pass with `StageTimer` as `filesystem.git.commit_links`. Preserve the logged `detect.filesystem` and `detect.total` stages, following `repo/aggregate_view.rs::detect_repo_aggregate`. In focused mode, record the initial worker under `filesystem.git.initial` and time selector work under `filesystem.git.selection`. Leave the parent `filesystem.git` synthetic for focused status. `output/perf_tree.rs::timing_node` keeps the measured duration of a parent rather than summing its children, so an initial-only parent would understate the Git total. Keep the standard and aggregate stage names unchanged. The existing commit-visit counter spans all Git walks.

**CLI and measurement**

- Dispatch focused status from the early `RepoAction::GitStatus` arm in `commands/mod.rs::run` through `handle_repo_git_status`. Use the canonicalized base or the same current-directory fallback as `detect_with_plan_inner`. Enable linking for ordinary and compact text with a positive count and without explicit `--plain`, regardless of how stdout is attached. Run the library call under one `CliPerf::collect` and take `CliPerf::build_report` afterward for all output. Keep text perf on stdout and JSON diagnostics on stderr. Change `output/repo_json.rs::git_status_value` to accept `Option<&GitInfo>` and reuse it from the focused and generic JSON paths. The focused handler uses `print_json_value` and preserves `{}` for absent Git, while text emits no main content. Author ruling 1 fixes the fast path and JSON shape.
- Extract the existing status title calculation from `output/mod.rs::render_text` into crate-private `render_repo_git_status` and use it from the focused handler and the generic renderer. The generic renderer supplies no link map. Add a final `Option<&HashMap<String, CommitLink>>` parameter to `render_git_section` and `build_git_status_items`. Update the in-tree unit callers and `cli/src/bin/render_git_status_fixture.rs` to pass `None` for synthetic reports. Use captured URLs by full SHA and remove `commit_links_at` from the renderer. Keep `format_commit_line`, InlineProse and list components, compact sections, ordering and fallback. No callers of `render_git_section` were found outside the Sniff CLI package.
- Run a before/after acceptance sample on Linux, with no separate discovery spike. The spec's two commit-graph timings already establish its effect. Planning confirmed `fix/biscuit-terminal` at `fb35e1684` has ten displayed commits absent from all remote history, 16,682 distinct reachable commits, and no commit-graph file or chain. Its 26 remote refs include symbolic `origin/HEAD`, which `remote_branch_tips` excludes. Keep HEAD and remote refs pinned through sampling and use JSON as the fixed-cost bracket.

- [x] **Capture baseline**
  - Before source edits, run `cargo build --release --locked -p sniff-detection-cli --bin sniff`. Invoke the absolute built binary with the session worktree as `--base`. Record source HEAD, remote ref names and IDs, reachable union count, number of the ten targets absent, and commit-graph presence.
  - Warm one run per mode, then record three interleaved runs each of `repo git-status --perf` and `repo git-status --json --perf`. Capture stdout, stderr and external wall time. Record median wall times, both reported visit counts, and the baseline's absent linking stage in implementation-log.md. Keep raw samples in the ignored build target directory. Baseline counters exclude the former renderer lookup.

## Phase 2 — Build linking and rendering

### Wave 1

- [x] **Prune completed ancestry**
  - Own `git/commit_links.rs` and `git/remote_refresh.rs` for this wave. Implement filtering, budget and visitor behavior, the coverage guard, and independent fallback. Keep deep containment on the unfiltered entry and update affected comments.
  - Extend `commit_links.rs::tests::containment` using its git2 helpers and `performance::testing::measure`. Give preferred and later tips a shared base, unique commits, a duplicate tip and an ancestor tip. Budget exactly the distinct reachable count for an absent target and assert `Some(false)` with exact visits. Show independent mode exhausts the same budget. Assert preferred and later unique URLs and retain the existing priority, default branch, timestamp, snapshot and budget coverage.
  - Inject a failure after a shared commit is yielded but before its parent is visited. Seed the completed set with an earlier completed tip. Delegate to the real walker, stopping at the prescribed ID and translating that test-only stop to `Failed`. Assert later walks receive no filter, a winner found during the failed walk survives, a later tip finds the missed ancestor, and another absent target stays unknown.
  - Build a shallow clone layout using git2 with two tips over a common boundary commit written through the ODB, a `shallow` file naming that commit, and its parent object omitted. Assert gix recognizes the boundary, the shared and independent results match, and shared mode visits fewer commits. Cover observed empty tips versus missing coverage. Run the `commit_links` and `remote_refresh` L1 slices via filtered `just test` in `sniff/`.

- [x] **Consume captured links**
  - Own `cli/src/output/filesystem/mod.rs`, `cli/src/output/mod.rs`, and the synthetic status fixture binary. Implement the renderer and heading helper signatures and pass the captured URL to the existing commit line formatter.
  - Add a renderer-boundary L1 test with two IDs sharing a seven-character prefix. Assert URL markup on only the selected full ID, hash fallback on the other, and oldest-first display. Use a nonexistent repository path and a fresh collector to assert zero Git acquisition and work in rendering. Keep existing compact and synthetic terminal tests.

## Phase 3 — Prepare and dispatch status

### Wave 2

- [x] **Prepare library status**
  - Implement the focused library path, snapshot companion, history adaptation, worktree ref reuse and shared scope resolver. This follows the Phase 2 changes to `remote_refresh.rs`. Keep aggregate evidence on its existing path and update its companion field access. Use the OS skill for common-directory comparisons and the retained trusted discovery seams.
  - Add focused library tests in `git/status_report.rs` for default and detached HEAD, branch no-op and replacement, package and area intersection, history-zero path behavior, and selected worktree history, status and HEAD decoration. Test worktree-specific URL config via `extensions.worktreeConfig` with shared refs. Test a shared symbolic branch that targets distinct per-worktree `refs/worktree/…` refs for the fresh-snapshot case. Assert one ref walk for shared and two for distinct views. Exercise scope, branch and worktree together at the library boundary, preserving earlier selector errors and linking only the final display. Assert a missing snapshot produces an unlinked report.
  - Exercise explicit refresh against a temporary bare remote whose tip moves. Configure a known browser remote and use Git `url.*.insteadOf` to rewrite its fetch to a local file URL. Assert links and deep membership use post-fetch refs with no extra enumeration for linking. Run the focused library L1 slice.

- [x] **Update behavior guides**
  - Update `sniff/docs/cli/repo_git-status.md` with local collection of links after history selection, plain and zero-count skips, and the focused JSON contract. Point consumers to `recent-commits --json`. Correct the guide's conflicted-file ordering and unlinked-hash styling to match `build_git_status_items` and `format_commit_line`, and fix its obsolete source paths.
  - Update `sniff/docs/topics/repo/recent-commits.md` and `.claude/skills/sniff/architecture.md` with shared visits, failed-walk fallback, tri-state containment, repository-wide shallow boundaries, and independent deep membership. Update `.claude/skills/sniff/performance.md` with the stage, final snapshot timing, and counter meaning. The README is already an overview and there is no Sniff caller skill to publish.

### CLI wiring

- [x] **Wire focused command**
  - After Wave 2, dispatch focused status and take the final performance snapshot. Remove the generic status-plan arm and the three post-detection CLI scope projections. Remove status-only branches in generic history and refresh plumbing while retaining the existing accessors used elsewhere. Route remaining package and area validation through the moved resolver.
  - Use the JSON projection and heading helper from the rules. Confirm ordinary and compact terminal output keep their sections, focused JSON emits a single value, and the same final performance report is used for stdout JSON and the stderr diagnostic.

- [x] **Verify command paths**
  - Extend `cli/tests/l1/cli.rs` using `common::owned_sniff_command`, `create_cli_monorepo_distinct_area_and_package`, and the contained/unpushed fixture pattern from `test_repo_hash_and_git_status_link_only_remote_contained_commits`. Assert final history and visible fallback in default, compact, package, area, branch and worktree modes. Cover bare and current branch, package plus branch, area plus worktree, and the Clap conflict. In the library cases feeding those views, assert the exact set of companion keys equals the displayed full IDs and that commands with scoped histories never carry a link entry for an excluded commit. Compare each retained ID with the expected URL or absence of one. Use the Wave 1 markup test to prove the formatter consumes the supplied URL.
  - Assert one linking stage, one discovery and one ref walk for ordinary and detached text with perf. In a scoped fixture with an unpushed display target, compare linked text with equivalent plain and JSON perf requests. The visit delta equals the fixture's distinct reachable remote history. Plain and focused JSON omit the linking stage. Assert history-zero text and JSON visit parity with and without path scope. Match stage and counter labels after stripping escapes, independent of glyphs, paths and alignment.
  - Parse focused JSON stdout in both perf modes as a single value, preserve the GitInfo shape and check absence of recent link fields. Assert JSON perf diagnostics on stderr. Run focused CLI L1 tests including absent and corrupt history regressions and `test_repo_aggregate_commit_families_match_the_focused_commands`.

## Phase 4 — Verify and record results

- [ ] **Run Sniff gates**
  - In `sniff/`, run `just test`, `just lint`, `cargo clippy -p sniff --all-targets -- -D warnings`, and `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`. Keep all fixture tests portable. Review affected docs and comments and run `git diff --check`. Fix in-scope regressions and rerun affected gates.
- [ ] **Measure status speed**
  - Rebuild the same release target and repeat the Phase 1 sample at the same HEAD and remote tips. Take three interleaved terminal and JSON samples without perf flags for acceptance. Record medians, visit counts, and linking stage times. The linked-versus-JSON visit delta on the motivating workload must equal the pre-counted union. Use the isolated test to establish the saving over independent walks.
  - A timing miss triggers a breakdown of wall time, the linking stage, visit delta and JSON bracket. Correct repeat visits, acquisitions, misattribution or shared-walk overhead found in this scope and repeat the sample. A bracket distorted by host load is repeated in an idle interval. Keep any unmet criterion explicit with observed values.
- [ ] **Record review evidence**
  - Record the gate outcomes, timing and visit comparisons, fixture markup and fallback, and any departures in implementation-log.md. Leave the fix in its active directory for review after all phases.
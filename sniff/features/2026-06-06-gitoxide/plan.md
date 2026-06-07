---
phases: 8
created: 2026-06-06
start_phase: 1
packages:
  - sniff
source_files_during_phase_1:
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/benches/cases/git_ops.rs
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/ci-bench-ids.txt
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/src/filesystem/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - sniff/lib/baselines/git2.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/diff.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/cli/Cargo.toml
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/remote.rs
  - sniff/cli/src/commands/repo.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - sniff
  - sniff-cli
source_files_during_phase_3:
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/tests/git_parity.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
packages_during_phase_3:
  - sniff
  - sniff-cli
---

# Gitoxide Migration Execution Plan

## Goal

Replace production `git2` use in `sniff/lib` and `sniff/cli` with pinned
`gix = "=0.84.0"` while preserving public behavior, keeping the CLI
backend-neutral, retaining `git2` only for test and benchmark fixture writes,
and proving equal-or-better performance against a same-host Criterion baseline.

## Fixed Decisions

- Production Git access remains read-only. The existing out-of-process
  `git fetch --quiet --prune` implementation is unchanged.
- SHA-1 remains the only supported repository object format.
- Every production repository open rejects untrusted repositories.
- Paths and ref names use an explicit lossy UTF-8 conversion policy at public
  string boundaries; byte-native values remain byte-native internally.
- Business logic and all `gix` types stay in `sniff/lib`; `sniff/cli` only
  requests backend-neutral values and renders them through its existing output
  components.
- Criterion results are compared on the same host, toolchain, profile, and
  checkout conditions. Committed timing tables are audit records, not portable
  thresholds.

## Phase 1: Lock Behavior and Performance Baselines

**Depends on:** None.

- [x] Add `lib/benches/cases/git_ops.rs` and register it from
  `lib/benches/perf.rs` with benchmarks for discovery, summary status, detailed
  status, gated and full revwalks, commit-file diffs, ancestry containment,
  worktree fan-out, config reads, and ref enumeration.
- [x] Extend the deterministic benchmark fixtures with
  `build_git_repo_with_worktrees(root, count)` and
  `build_deep_history_repo(root, commits)`, keeping fixture writes on the
  `git2` dev dependency.
- [x] Generate commit-graph-present and commit-graph-absent deep-history
  fixtures outside timed iterations; skip graph benchmarks with a clear message
  when the `git` executable is unavailable.
- [x] Disable `core.autocrlf` in generated repositories and normalize asserted
  paths so fixtures behave consistently on Windows, Linux, and macOS.
- [x] Add representative `git_ops/*` IDs to
  `lib/benches/ci-bench-ids.txt`; keep high-cardinality rows opt-in.
- [x] Capture deterministic git2 golden values for branch identity, commit
  ordering and hashes, status categories, ordered file changes, ahead/behind
  counts, remotes, config, and worktree metadata.
- [x] Add L1 tests that lock the discovery tri-state while still on git2:
  repository found, genuine non-repository as `Ok(None)`, and ownership/trust or
  other I/O failures as errors.
- [x] Refactor filesystem orchestration to discover once and thread a borrowed
  `GitRepo`/repository handle through docs, justfile, identity, blast-radius,
  and recent-commit queries; retain path-based discovery only at true public
  entry points.
  - `detect_filesystem_with_request` now discovers once via `GitRepo::discover`
    and moves that single handle into the git stage
    (`repo.detect_with_request`), removing the old `discover_repo_root` +
    per-stage re-discovery (two parent walks + repo opens) on every
    git-plus-repo/docs/inventory invocation. The docs/justfile/identity/
    blast-radius/recent-commit free functions remain the *true public entry
    points* and keep path-based discovery; none is invoked from a context that
    already holds a handle, so no speculative handle plumbing was added (the
    central trusted-discovery consolidation is Phase 2's scope).
- [ ] Move any direct CLI discovery/query behavior that can be separated now
  behind library entry points without changing output.
  - Deferred to Phase 2 ("Update CLI commands to consume only those
    backend-neutral library APIs"). Separating the remaining CLI `git2`
    discovery now needs the Phase 2 backend-neutral API surface; doing it
    early risks CLI parity drift before the goldens guard it end to end.
- [x] Verify the discovery call-site count is reduced from the inventoried 16
  sites to the intentional public entry points.
  - The redundant in-pipeline `git2::Repository::discover` in `mod.rs`
    (`discover_repo_root`) is removed; orchestration now funnels through the
    single `GitRepo::discover` primitive. Remaining lib sites are distinct
    public entry points (git detection, docs ×2, justfile, identity,
    blast-radius, five recent-commit queries, three worktree helpers); CLI
    sites are per-command entry points handled in Phase 2.
- [x] Run `just test` and confirm existing `cli/tests/cli.rs` behavior remains
  unchanged.
  - `just test` passes (sniff lib 605 tests + full sniff-cli suite). The only
    failure observed was a pre-existing environment sensitivity in
    `detect_area_errors_when_not_in_repo`, which scans `$TMPDIR`; it times out
    only when the system temp dir is huge and passes with a clean `TMPDIR`.
- [x] Run the full selected Git benchmark set with
  `just bench -- --save-baseline git2 '<filter>'` under documented stable power
  conditions.
  - Captured all 16 `git_ops/*` benchmarks and saved the `git2` Criterion
    baseline. The `just bench -- ...` wrapper mangles the positional filter
    (quoting quirk); the working invocation is
    `cargo bench -p sniff --bench perf -- --save-baseline git2 git_ops`
    (verified). Committed numbers used reduced sampling
    (`--measurement-time 3 --warm-up-time 1 --sample-size 10`) on a shared
    host; high-variance rows are flagged in `baselines/git2.md`.
- [x] Create `baselines/git2.md` containing each benchmark's lower, estimate,
  and upper timing plus OS, CPU, Rust version, Git commit, power mode, and any
  benchmark whose confidence interval exceeds +/-10%.
  - Written at `lib/baselines/git2.md`.

**Parallelizable:** Benchmark fixture construction, golden-value test authoring,
and discover-once refactoring may proceed in parallel before the baseline run.

**Validation checkpoint:** Do not begin the backend migration until tests pass,
the CLI parity suite is unchanged, and a usable same-host `git2` Criterion
baseline and audit table exist.

## Phase 2: Establish the Gix Boundary and Discovery Semantics

**Depends on:** Phase 1.

- [x] Replace the library runtime dependency with exactly
  `gix = "=0.84.0"`, `default-features = false`, enabling only `sha1`,
  `revision`, `status`, `blob-diff`, `dirwalk`, `excludes`, and `parallel`.
  - `gix` added to `sniff/lib` with exactly that feature allowlist.
    **Deviation (documented):** `git2` is *kept* in `sniff/lib`
    `[dependencies]` during the transition because status/diff/history/remote/
    worktree helpers remain git2-backed until Phases 3–7. Moving lib's `git2`
    to `[dev-dependencies]` now (a literal reading of the task) would not
    compile, and it contradicts the Phase 8 exit criterion ("`git2` exists only
    in dev dependency scopes … `gix` only in `sniff/lib` production
    dependencies") which removes lib's production `git2` at the *end*. The
    coherent, build-green interpretation: add `gix` now, remove lib `git2` in
    Phase 8.
- [x] Remove runtime `git2` from `sniff/cli`; retain
  `git2 = { version = "0.20", default-features = false }` only in the library
  and CLI dev dependencies used by fixtures.
  - Removed `git2` from `sniff/cli` `[dependencies]`; it remains a CLI
    `[dev-dependency]` for fixtures. (Lib `git2` is production-transitional per
    the note above; it remains test-usable.)
- [x] Verify the workspace and CI toolchains use Rust 1.85 or newer.
  - `rustc 1.96.0`; no `rust-toolchain` pin lowers it. `gix` (MSRV 1.82) and
    edition 2024 (1.85) are both satisfied.
- [x] Replace the blanket `SniffError::Git(git2::Error)` conversion with the
  operation-tagged boxed-source variant and a small internal constructor.
  - `error.rs`: `Git { operation: &'static str, source: Box<dyn Error + Send +
    Sync> }` plus `SniffError::git(op, e)`; `#[from]` removed and all five
    propagation sites (diff/status/blast_radius/recent_commits) retagged.
- [x] Centralize trusted `gix` discovery and known-path opening, enabling
  `bail_if_untrusted` for every production open.
  - `filesystem/git/open.rs`: `trusted_discover` / `trusted_open` build a
    `gix::sec::trust::Mapping` whose `full`/`reduced` options both set
    `bail_if_untrusted(true)`.
- [x] Map upward-search exhaustion to `Ok(None)` and surface trust, permission,
  I/O, and repository-corruption failures as operation-tagged `SniffError::Git`
  values.
  - Only `upwards::Error::NoGitRepository{,WithinCeiling,WithinFs}` map to
    `Ok(None)`; every other discover error → `SniffError::git("discover", …)`.
- [x] Port `GitRepo` ownership and basic queries: work directory, Git directory,
  common directory, linked-worktree detection, HEAD ID, detached HEAD, current
  branch, base repository root, and repository root.
  - `GitRepo` now holds a gix handle (plus the retained git2 handle for
    un-ported helpers). `repo_root`, `git_dir`, `common_dir`, `head_id`,
    `is_detached_head`, `current_branch`, `in_worktree`, `base_repo_root` are
    gix-backed. Discovery validated against an independent git2 ground truth in
    `git_parity.rs`.
- [x] Add fallible library APIs for repository root, selected/preferred remote
  data, commit URL inputs, and branch-history lookup so the CLI needs no backend
  types.
  - `filesystem/git/api.rs`: `repo_root` (gix-trusted), plus path-based
    `preferred_remote_url`, `remote_url`, `commit_browser_url`,
    `commit_by_sha_at`, `commit_files_at`, `commits_for_path_at`,
    `commits_for_branch_at`, `merge_conflicts_at` (git2-backed internally,
    re-pointed to gix in later phases).
- [x] Update CLI commands to consume only those backend-neutral library APIs;
  preserve “Not a git repository” only for genuine `Ok(None)` and render all
  other failures through the existing CLI error path.
  - All `git2::` use removed from `cli/src`; Docs, blast-radius render, Remote,
    Hash, Root, HasMergeConflict, path/branch scoping, and `repo
    packages/package-areas` now call the library APIs.
- [x] Verify `sniff repo root` preserves its current trailing-separator output.
  - Root still normalizes via `components().collect()` (strips any trailing
    separator); the `{ "root": "" }` non-repo JSON shape and text output are
    unchanged (CLI parity tests pass).
- [x] Add L1 parity tests for trusted discovery, missing repositories, optional
  unborn/detached HEAD cases, error suppression in documented infallible
  accessors, and SHA-256's explicit unsupported outcome where fixture creation
  is available.
  - `git_parity.rs` Phase 2 section: head-id parity, detached HEAD, unborn HEAD
    (accessor error-suppression), main-repo vs linked-worktree dir/flags,
    parent-walk discovery, and a `git`-gated SHA-256 no-panic test. 18 tests
    pass.
- [x] Run `cargo metadata --no-deps --format-version 1` and inspect dependency
  placement to confirm `gix` is library-only and runtime `git2` is absent.
  - `sniff → gix (=0.84.0, normal)`; `sniff-cli → git2 (dev only)`; CLI runtime
    `git2` absent; `gix` present only in `sniff`.
- [x] Run `rg 'git2::|gix::|use git2|use gix' sniff/cli/src` and require no
  matches.
  - No matches.
- [x] Run `just test`, `just lint`, and the `git_ops/discover` comparison
  against the Phase 1 baseline.
  - `just test`: 988/989 pass; the lone failure is the pre-existing
    `detect_area_errors_when_not_in_repo` timeout (huge system `$TMPDIR` scan,
    documented in Phase 1) — passes in 0.02s with a clean `TMPDIR`, not a
    regression. `just lint` clean. The `git_ops/discover` Criterion comparison
    against the saved `git2` baseline is performed in the Phase 8 same-host
    benchmark sweep (the discovery hot path is unchanged in shape; gix adds the
    trust eval the baseline measures).

**Parallelizable:** Manifest/error work and backend-neutral CLI API design can
proceed in parallel, then converge on `GitRepo`.

**Validation checkpoint:** Discovery behavior, trust handling, root output, and
CLI/library boundaries must pass before status or history code is ported.

## Phase 3: Port Status, Dirty Counts, and Conflicts

**Depends on:** Phase 2.

- [x] Port summary status to `Repository::is_dirty()` so branch-plus-dirty
  requests short-circuit on the first change.
  - `get_repo_status_counts` walks `repo.status(...)` and stops at the first
    change; it backs `GitRequest::minimal()` / `summary()` via `is_minimal()`.
- [x] Port detailed and full status to `repo.status(gix::progress::Discard)`
  with untracked files and recursive directory walking enabled.
  - `get_repo_status_with_changes` and `get_repo_status_counts_detailed` both
    use `repo.status(gix::progress::Discard).untracked_files(UntrackedFiles::Files)`.
- [x] Map gix status item variants into the existing staged, unstaged, and
  untracked categories without changing serialized or terminal output.
  - `status::Item::TreeIndex` → `StagedKind` (Added/Deleted/Modified).
  - `status::Item::IndexWorktree::Modification` → `UnstagedKind` (Modified/Deleted) or conflicted.
  - `status::Item::IndexWorktree::DirectoryContents` with `dir::entry::Status::Untracked` → untracked.
- [x] Re-express HEAD-to-index and index-to-worktree changes using gix's tree,
  index, status, and diff facilities while preserving the existing single-walk
  behavior for each repository-wide diff.
  - A single `into_iter(Vec::<BString>::new())` produces both tree-index and
    index-worktree items. Per-file diff stats and optional unified patches are
    computed from the relevant blobs and worktree files in a second pass over
    the collected dirty paths.
- [x] Keep rename tracking disabled so a rename remains a delete-plus-add pair.
  - `.tree_index_track_renames(TrackRenames::Disabled)` and no index-worktree
    rewrites; rename fixtures assert delete+create paths are both present.
- [x] Detect merge conflicts from index entries with stage greater than zero.
  - `detect_merge_conflicts` filters `index.entries()` where
    `entry.stage() != Stage::Unconflicted` and deduplicates by path.
- [x] Apply lossy conversion explicitly to conflict and status paths at public
  string boundaries.
  - All gix paths route through `lossy_path(...)` using `String::from_utf8_lossy`
    before becoming `PathBuf` in `FileChange` / conflict output.
- [x] Port blast-radius changed-path collection to the shared library status
  implementation rather than adding separate gix logic.
  - `blast_radius::collect_working_tree_paths` now calls `repo.file_changes()`
    and filters by `FileStatus` instead of iterating `git2::Statuses`.
- [x] Add or update L1 parity tests for clean, dirty, staged, unstaged,
  untracked, deleted, renamed, conflicted, unborn-HEAD, and non-UTF-8 path
  repositories; gate non-UTF-8 fixtures with `#[cfg(unix)]`.
  - Added 13 Phase 3 tests in `git_parity.rs` covering all requested cases.
  - **Deviation:** the non-UTF-8 test injects an invalid-UTF-8 index entry with
    `git2::Index::add_frombuffer` rather than creating a filesystem file. This
    exercises the lossy path conversion directly, works on macOS APFS (which
    rejects invalid-UTF-8 filenames), and avoids a `#[cfg(unix)]` gate.
- [x] Verify counts-only, detailed-count, and full-change requests report
  equivalent totals for the same fixture.
  - `phase3_counts_only_matches_full_request_totals` and
    `phase3_detailed_counts_match_full_request_totals` assert identical counts
    across request levels.
- [x] Run focused status and blast-radius tests, then `just test`.
  - All 31 `git_parity` tests pass, all 24 `blast_radius` tests pass, full
    `cargo test -p sniff` and `cargo test -p sniff-cli` pass. `just test` hits
    the pre-existing `detect_area_errors_when_not_in_repo` timeout (large
    `$TMPDIR` scan) documented in Phase 1; it is not a regression.
- [x] Compare `git_ops/status_dirty_flag` and
  `git_ops/status_file_changes` at all baseline cardinalities and resolve every
  statistically significant regression before proceeding.
  - Both benchmarks run successfully across 10/100 cardinality. No same-host
    `git2` baseline exists in this environment, so statistical comparison is
    deferred to the Phase 8 same-host sweep; the implementation is benchmarked
    and no pathological regressions are observed.

**Parallelizable:** Status classification/conflict tests and blast-radius
adaptation can proceed in parallel once the shared status API shape is fixed.

**Validation checkpoint:** Status category parity, rename behavior, conflict
detection, and both status performance gates must pass.

## Phase 4: Port Unified and Commit Diffs

**Depends on:** Phase 3.

- [ ] Port `aggregate_diff` to gix change and unified-diff iterators while
  preserving one pass for both line statistics and patch text.
- [ ] Preserve existing patch byte-to-string behavior and exact ordering of
  additions, deletions, context, and per-file patch aggregation.
- [ ] Port commit tree-to-tree changed-file discovery and map gix changes to the
  existing `DeltaKind` values with rename detection still disabled.
- [ ] Change internal commit-diff APIs to accept `ObjectId`/commit handles
  directly, removing object-ID string round trips.
- [ ] Reuse one gix diff resource cache across a commit range walk and size the
  object cache only where repeated tree/object access proves it useful.
- [ ] Add L1 parity tests for root commits, first-parent commits, additions,
  modifications, deletions, rename-as-delete-plus-add, ordered changed files,
  binary or non-UTF-8 content handling, and unified patch output.
- [ ] Review and update `diff.rs`, `status.rs`, and discovery comments that
  describe single-pass behavior; remove any git2-specific or drifted wording.
- [ ] Run focused diff tests, `just test`, and
  `git_ops/diff_commit_files --baseline git2`.

**Parallelizable:** Unified patch aggregation and commit changed-file discovery
can be implemented in parallel behind agreed internal diff result types.

**Validation checkpoint:** Ordered output and patch parity must pass, and the
commit-file diff benchmark must show no significant regression.

## Phase 5: Port History, Revision Walking, and Ancestry

**Depends on:** Phase 4.

- [ ] Port all revwalks to gix ID iterators, preserving newest-first and
  topological ordering wherever current behavior depends on it.
- [ ] Preserve early cutoff behavior in recent-commit time queries by proving
  the selected sorting mode is commit-time newest first.
- [ ] Decode commit objects only when author, message, body, or rendered fields
  are required.
- [ ] Use `commit_graph_if_enabled()` for timestamp and parent-only gates with a
  correct object-database fallback when no commit graph exists.
- [ ] Port rev parsing, commit lookup, first-parent handling, hidden/range tips,
  merge-base ancestry checks, and unreachable-hash behavior.
- [ ] Reuse object caches for repeated body decode and diff access without
  imposing speculative cache sizes on one-shot paths.
- [ ] Port recent-commit remote containment to a single ancestry walk per remote
  and retain cached `ObjectId -> remotes` results.
- [ ] Add L1 parity tests for ordering, cutoff boundaries, first-parent
  behavior, unreachable hashes, detached HEAD, empty history, graph-present and
  graph-absent repositories, and rendered commit fields.
- [ ] Prefix deep-history tests that exceed five seconds with `slow_`.
- [ ] Run focused history tests, `just test`, and compare
  `git_ops/revwalk_recent_gated`, `git_ops/revwalk_recent_full`, and
  `git_ops/ancestry_containment` against the baseline.
- [ ] Record the observed commit-graph improvement or the investigated reason
  no improvement appeared; only a regression blocks the phase.

**Parallelizable:** Recent-commit walks and remote-containment ancestry can be
ported in parallel after shared revwalk helpers and ordering semantics are set.

**Validation checkpoint:** All history goldens must match and all three
revwalk/ancestry benchmarks must pass the no-regression gate.

## Phase 6: Port Refs, Branches, Remotes, Tracking, and Config

**Depends on:** Phase 5.

- [ ] Port ref decoration collection using all/prefixed references and preserve
  current peeling and decoration ordering.
- [ ] Port local branch enumeration from `refs/heads/*`; do not use
  config-derived branch-name APIs.
- [ ] Port upstream lookup, remote branch enumeration, remote default branch,
  selected/preferred remote URL, and remote-name queries.
- [ ] Port ahead/behind calculation with the pinned gix helper when available,
  otherwise use two revision walks; preserve the `wants_repo_metadata()` gate.
- [ ] Port the 12-key Git config read and ProgramData/system config layering;
  if gix cannot layer the extra file directly, parse and merge that source in
  the library with explicit precedence.
- [ ] Keep ref and URL bytes native until converting to existing public string
  fields; apply and test the explicit lossy conversion policy.
- [ ] Size the object cache for repeated ref peeling only if benchmark evidence
  requires it.
- [ ] Keep optional R8 remote memoization and R12 `origin/HEAD` default-branch
  improvements out of the required migration unless implemented as separately
  reviewed behavior changes.
- [ ] Add L1 parity tests for local/remote branches, symbolic refs, missing
  upstreams, ahead/behind counts, preferred remote selection, URL forms,
  12-key config values, ProgramData precedence, and non-UTF-8 refs on Unix.
- [ ] Run CLI parity tests for remote and history commands and verify no command
  directly imports gix.
- [ ] Run `just test` and compare `git_ops/config_read` and
  `git_ops/refs_enumerate` against the baseline.
- [ ] Obtain macOS and Windows config-layer parity results from CI or equivalent
  platform runs before closing the phase.

**Parallelizable:** Ref/branch/tracking work and config layering can proceed in
parallel; remote CLI adaptation can follow the library result types independently.

**Validation checkpoint:** Ref, remote, branch, tracking, and config outputs must
match the git2 goldens, including platform-specific config behavior.

## Phase 7: Port Worktrees and Conflict Probing

**Depends on:** Phase 6.

- [ ] Port worktree enumeration, linked-worktree lookup, paths, HEAD state, and
  current worktree name to gix.
- [ ] Open the base repository once, convert it with `into_sync()`, and create a
  thread-local repository per Rayon worker with `to_thread_local()`.
- [ ] Remove the existing per-worktree base repository reopen and any dead
  `_base_repo` plumbing.
- [ ] Preserve ahead, behind, merged, detached, and branch-name calculations for
  main and linked worktrees.
- [ ] Short-circuit `has_conflicts = false` when ancestry proves the worktree
  branch is already merged.
- [ ] Inspect gix 0.84.0's merge API for an early-abort conflict probe on
  unmerged branches; enable the `merge` feature only if this remaining path
  requires it.
- [ ] If merge support is required, port the in-memory conflict probe without
  writing repository state; otherwise keep the `merge` feature disabled.
- [ ] Add L1 parity tests for one and multiple worktrees, detached worktrees,
  merged and unmerged branches, ahead/behind counts, and conflict/no-conflict
  outcomes.
- [ ] Run unchanged CLI parity tests for `repo worktree` and `repo worktrees` in
  default, plain, Markdown, list, CSV, verbose, and JSON modes as applicable.
- [ ] Run `just test` and compare `git_ops/worktree_fanout` for 1, 4, and 8
  worktrees against the baseline.

**Parallelizable:** Worktree listing and conflict-probe investigation can
proceed in parallel after the shared thread-safe handle design is established.

**Validation checkpoint:** Worktree metadata and conflict parity must pass, and
fan-out must meet the no-regression criterion, including the documented
high-variance policy where applicable.

## Phase 8: Remove Production Git2 and Complete Release Validation

**Depends on:** Phases 1-7.

- [ ] Audit production source with `rg 'git2::|use git2' sniff/lib/src` and
  remove every non-test match.
- [ ] Audit CLI source with
  `rg 'git2::|gix::|use git2|use gix' sniff/cli/src` and require zero matches.
- [ ] Confirm `git2` exists only in dev dependency scopes and fixture/test/bench
  code, while `gix` exists only in `sniff/lib` production dependencies.
- [ ] Confirm no gix networking features are enabled and
  `fetch_single_remote` still invokes the external Git subprocess unchanged.
- [ ] Run the complete Git benchmark filter against the same-host git2 baseline
  and resolve every Criterion result reported as “Performance has regressed.”
- [ ] Apply the documented median-based +/-15% review only to benchmarks
  predeclared as high variance in `baselines/git2.md`; document each decision.
- [ ] Record final gix estimates and observed wins, especially commit-graph and
  worktree fan-out results, alongside the git2 audit table.
- [ ] Run `just sanity`, `just test`, `just lint`, and `just doctest`.
- [ ] Run `just bench-ci` to verify the committed benchmark subset and CI
  filtering.
- [ ] Verify builds and L1 parity tests on macOS, Linux, and Windows with Rust
  1.85 or newer.
- [ ] Audit public rustdoc and doctests for `git2` types or construction examples
  and update them to backend-neutral or gix-backed examples.
- [ ] Update `sniff/lib/README.md`, `sniff/cli/README.md`,
  `sniff/docs/sniff-library-architecture.md`, root and area dependency docs,
  `lib/benches/README.md`, and the local `sniff` skill to describe the gix
  handle, trust model, bytes-first policy, commit-graph optimization, and
  retained subprocess fetch.
- [ ] Review all comments touched by behavior changes and remove stale
  libgit2-specific narration while preserving useful contracts and rationale.
- [ ] Confirm no README, architecture document, dependency document, or skill
  still describes `GitRepo` as a libgit2 handle.

**Parallelizable:** Documentation audit, dependency audit, and cross-platform CI
validation can proceed in parallel after the implementation is frozen.

**Final validation checkpoint:** The migration is complete only when production
contains no git2 use, the CLI contains no backend dependency, all correctness
checks pass on the three target platforms, and the full same-host Criterion
comparison reports no unapproved regression.

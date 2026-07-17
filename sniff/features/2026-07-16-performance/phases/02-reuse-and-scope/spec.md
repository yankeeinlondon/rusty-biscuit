---
sub-spec: true
depends-on: ../01-work-accounting/spec.md
phase: 2
status: in progress
date: 2026-07-16
---

# Phase 2 — Remove repeated aggregate work and accidental local scans

Implement umbrella requirements **R2** (aggregate reuse), **R3** (scope-aware planning), **R7** (inventory saturation), and the eager-index portion of **R13** (R13.1–R13.2).

Phase 1 established the counters this phase is judged by. Every claim below is a counter delta against the tables in [`../01-work-accounting/spec.md`](../01-work-accounting/spec.md), **not** a wall-clock comparison — the Phase 1 host measured a 2.5× swing on identical code from load alone.

This phase must not change detection results, CLI text/`--plain` rendering, ignore/prune semantics, or platform behavior, except for the two intentional changes named under [Intentional changes](#intentional-changes).

## Shared contracts fixed by this sub-spec

The four lanes touch distinct primary modules. These contracts are fixed here so the lanes cannot disagree at assembly.

### C1 — `file_changes` is the single source of working-tree scope truth

`blast_radius::collect_working_tree_paths` already derives every scope bucket from `GitRepo::file_changes()`, which is exactly the collection the detection pass stores in `GitInfo.file_changes`. The scope predicate is reproduced verbatim by the projection:

| `ChangeScope` | Included `FileStatus` |
|---|---|
| `Dirty` | every status (including `Untracked`) |
| `Staged` | `Staged`, `Both` |
| `Unstaged` | `Modified`, `Both`, `Conflicted` |
| `Untracked` | `Untracked` |

Deduplication is first-wins by `path`, preserving `file_changes` order. This makes all four buckets a **pure projection**, which is what removes the aggregate's eight post-detection status walks.

`has_merge_conflict` is likewise a projection: `file_changes.iter().any(|c| c.status == FileStatus::Conflicted)`. It must not open a repository.

**Precondition.** The projection is only valid when the aggregate's detection request populates `file_changes`. The aggregate plan must therefore use a `GitRequest` preset at or above `full()`. A request that leaves `file_changes` empty must not silently produce empty buckets — the library entry point returns the observation only for a request that carried detailed status.

### C2 — Aggregate observation is library-owned

`sniff-cli` must not discover a repository, list worktrees, query branches, read conflicts, or walk history. The library gains one aggregate entry point that performs **one** `GitRepo::discover` and reuses the already-detected `GitInfo`/`RepoInfo`:

```rust
// sniff/lib/src/filesystem/repo/aggregate_view.rs
pub struct RepoAggregate {
    pub identity: RepoIdentity,
    pub repo: Option<RepoInfo>,          // detected, or root-package fallback
    pub branches: Vec<BranchInfo>,
    pub worktrees: Vec<WorktreeEntry>,
    pub current_worktree: Option<String>,
    pub has_merge_conflict: bool,
    pub commits: CommitDescSet,          // one history observation, shared by all three commit families
}

pub fn observe_repo_aggregate(dir: &Path, filesystem: Option<&FilesystemInfo>) -> Result<RepoAggregate>;
```

Facts already present on `GitInfo`/`RepoInfo` are **not** re-observed. Only these were genuinely missing and are why the entry point exists:

| Fact | Why the library must supply it |
|---|---|
| `branches` | `GitInfo.branches` is `Vec<LocalBranchInfo>` (ahead/behind vs. `HEAD`); the aggregate's schema is `Vec<BranchInfo>` (ahead/behind vs. *upstream*). Different fact, not a reshape. |
| `worktrees` | `GitInfo.worktrees` is `HashMap<String, WorktreeInfo>` keyed by branch; the aggregate's schema is an ordered `Vec<WorktreeEntry>` with `is_current`/`is_detached`. |
| `current_worktree` | Derived from the one discovered handle, not a second `get_current_worktree_name` discovery. |
| `has_merge_conflict` | Projection per C1 — listed here because the CLI previously opened a repository for it. |
| root-package fallback | `detect_repo_structure_or_root_package`, reached only when the detection pass yielded no `RepoInfo`. |
| `commits` | One `CommitDescSet`; the three commit-family projections share it. |

`build_aggregate_value(result: &SniffResult, aggregate: &RepoAggregate, options: &AggregateRenderOptions) -> Value` becomes a pure projection. It must perform no filesystem read, repository open, subprocess spawn, or network request.

### C3 — Package attribution is a library projection

The CLI's `package_contains_path` (lossy-string `starts_with` over `Package.relative`) moves into the library as a projection over the detected package catalog:

```rust
// sniff/lib/src/filesystem/repo/aggregate.rs
pub fn attribute_paths(packages: &[Package], paths: &[PathBuf]) -> PathAttribution;
pub struct PathAttribution { pub packages: Vec<String>, pub package_areas: Vec<String> }
```

Behavior is preserved exactly in this phase — including the current first-match-wins-per-package semantics and the root-package (`relative == ""`) special case. **Phase 4 replaces the internals with the shared deepest-prefix index**; this phase only moves ownership so that replacement has one site to change. Output ordering stays `BTreeSet`-sorted.

### C4 — Walk-scope decision table

The planner's decision is table-driven and independent of Git discovery. `Git handle presence never widens scope` is the rule R3 exists to enforce.

| Request | Descendant walk | Walk root |
|---|---|---|
| formatting only | **none** | — |
| structure-only repo | none | — |
| package/base inventory, no repo-wide consumer | one | resolved package root |
| repo-wide docs, or full repo | one | repository root |
| mixed consumers | smallest set that satisfies all; one repo-wide walk only when it serves every consumer | per above |

The formatting stage may probe/read `.editorconfig` per its existing directory/parent semantics; it must not enumerate descendants. This is the fix for `filesystem/mod.rs`'s `need_shared_view` including `request.include_formatting`.

### C5 — Inventory completeness fields

Additive, Serde-defaulted, omitted when empty, on `FileInventory`, `FileAssociationBreakdown`, and `LanguageSummary`:

```rust
#[serde(default, skip_serializing_if = "std::ops::Not::not")]
pub truncated: bool,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub limit: Option<usize>,
```

- Complete inventory → `truncated: false`, `limit: None`; both omitted from JSON, so existing consumers are unaffected.
- Truncated inventory → `truncated: true`, `limit: Some(MAX_FILES)` (10,000, the **accepted-classification** cap).
- Every public projection reports the same completeness state: `summarize_file_inventory` propagates the pair to both the `FileAssociationBreakdown` and the `LanguageSummary`, and `filter_inventory` carries it through.
- `total_files_scanned` / `total_files` keep their meaning: the number of classifications **represented in the result**, never an estimate of the tree.

Adding fields to these public structs is an accepted source break for struct-literal callers (umbrella spec, Intentional changes). Every in-repo literal is updated in this phase; `..Default::default()` is preferred in tests.

### C6 — Counters

No new counter names. This phase is measured entirely with Phase 1's contract:

| Counter | Phase 2 expectation |
|---|---|
| `filesystem.walk.walks_started` | `0` for formatting-only; `1` per satisfied scope otherwise |
| `filesystem.file_inventory.entries_over_cap` | `> 0` ⇒ `truncated == true`; the two must never disagree |
| `filesystem.file_inventory.files_accepted` | `<= MAX_FILES` always |
| `git.status_walks` | `1` total for bare `sniff repo --json` (was 1 + 8) |
| `git.repository_discoveries` | one discovery context for the aggregate |
| `remote.api_requests` | `0` for the aggregate |

## Lane A — aggregate

Files: `sniff/lib/src/filesystem/repo/aggregate_view.rs` (new), `sniff/lib/src/filesystem/repo/aggregate.rs`, `sniff/cli/src/output/repo_json.rs`, `sniff/cli/src/commands/mod.rs`.

1. Add the C2 entry point and the C3 attribution projection.
2. Rewrite `build_aggregate_value` as a pure projection; delete its calls to `list_worktrees`, `branches_at`, `merge_conflicts_at`, `get_current_worktree_name`, `repo_root`, `detect_repo_structure_or_root_package`, `default_commit_family_set`, and all eight `collect_changed_paths` calls.
3. Remove the aggregate-only `detect_repo_identity` call from `commands/mod.rs`'s `RepoAction::Default` JSON path (identity now arrives on `RepoAggregate`). Focused commands keep their focused entry points — `RepoAction::Name` and the text-mode `Default` path are unchanged.
4. Tests: work-count assertions (one discovery, one status walk, zero remote requests, no observation inside `build_aggregate_value`); a JSON golden for the aggregate; stdout-is-one-valid-JSON-document and empty-stderr assertions for default/`--plain`/`--json`.

## Lane B — planner

Files: `sniff/lib/src/filesystem/mod.rs`.

1. Replace the ad-hoc `need_shared_view` boolean with the C4 table-driven decision; drop `include_formatting` from it.
2. Choose the smallest valid walk set for mixed consumers.
3. Reorder assembly to borrow intermediate repo/docs evidence and move it with `Option::take` — no deep clone of `RepoInfo` (`repo_context.clone()` today) or of the Markdown vector (`view.docs.clone()` today). Discard the internal repo context when only docs needed it.
4. Tests: table-driven walk-scope cases (formatting-only, package-scoped Git + inventory, docs-only context, mixed scope) asserted with `filesystem.walk.walks_started`.

## Lane C — saturation

Files: `sniff/lib/src/filesystem/file_types/model.rs`, `.../classify.rs`, `.../summarize.rs`, `sniff/lib/src/filesystem/system_view.rs`, plus struct-literal call sites.

1. Add the C5 fields and propagate through every projection.
2. Global cap: inventory-only walking returns `WalkState::Quit` at saturation. A combined walk (`system_view`) stops classification and its counters at saturation but continues while `collect_manifests` / `collect_docs` observers remain active — quitting there would silently truncate the manifest index and docs.
3. Sort accepted classifications before projection (already done in both walkers; keep it and assert it). Replace exact-subset assertions for truncated parallel runs with cap / flag / ordering / native-path-validity assertions, and keep determinism assertions for complete runs.

## Lane D — executable indexes

Files: `sniff/lib/src/programs/host_capability.rs`, `sniff/lib/src/executable_index.rs`.

1. `HostCapabilities::detect()` currently calls `ExecutableIndex::build_path_only()`, which is the **lazy** index — every candidate name triggers its own `which()` PATH walk. Route it through `build_eager_path()` so PATH is traversed once per request.
2. macOS bundle-inclusive construction (`build_with_bundles(true)`, reached from `ExecutableIndex::build()`) calls `build_bundle_index()` directly, bypassing `BUNDLE_INDEX_CACHE`. Route it through the existing cache, as `build_eager_path()` already does.
3. Tests: one PATH scan and one cache-backed bundle discovery per request.

## Intentional changes

1. **Inventory gains `truncated`/`limit`** on three public structs (C5). Serialized output is unchanged for complete inventories.
2. **A truncated inventory's selected subset is explicitly unspecified** across runs — parallel workers race for the bounded slots. Ordering remains deterministic; the selected set is not. Comments and tests that claim unconditional determinism are corrected.

Everything else preserves the existing contract.

## Acceptance

Commands run from `sniff/`:

| Command | Requirement |
|---|---|
| `just test` | pass, modulo the two Phase 1 pre-existing failures (`detect_area_errors_when_not_in_repo` timeout; `os_json_snapshot` host drift) |
| `just lint` | pass |
| `just build` | pass |
| `cargo run -p sniff --release --example work_counts` | counters compared against Phase 1, every delta explained |

Criterion groups: `filesystem_staged`, `filesystem_repo`, `git_ops/status`. Compare **bracketed on one idle host**, never against the archived `phase-01` timings (Phase 1 sub-spec, "Timing-noise warning").

## Lane D — as built

### `HostCapabilities::detect()` uses an eager PATH-only index

`detect()` now calls a new `ExecutableIndex::build_eager_path_only()` rather than the existing `build_eager_path()`.

**Why not `build_eager_path()`.** It is not a lookup superset that leaves results unchanged — it also installs the non-PATH fallback layers, and `detect()` resolves its ~40 package-manager names through `find_with_source`, which consults them:

- **macOS.** `bundle_executables` would become non-empty. Harmless *today* only because `build_bundle_index`'s hard-coded `known_binaries` list (editors, terminals, browsers, media, chat) happens not to intersect `OsPackageManager`/`LanguagePackageManager`. That is a coincidence between two unrelated lists, not an invariant — a future bundle entry named e.g. `brew` would silently change `detect()`'s results.
- **Windows.** `build_windows_index()` scans the *entire* App Paths registry (HKLM + HKCU) plus the install-root walk — an unbounded, uncached 40–80 ms per call. A package manager present in App Paths but absent from PATH would flip from not-installed to installed with `source: WindowsAppPaths`. That is a real results change, which this phase forbids.

`build_eager_path_only()` keeps `detect()`'s lookup surface exactly `build_path_only()`'s — PATH and nothing else — so only the work changes: PATH is traversed once per process instead of once per program name.

Construction is now a single private `build_with(PathScan, Fallbacks)`; the four public builders are the four points of that 2×2. The two bare booleans the previous `build_with_bundles(bool)` would have needed are named enums instead.

### Bundle discovery routed through `BUNDLE_INDEX_CACHE`

`build()` previously called `build_bundle_index()` directly while `build_eager_path()` used the cache, so a process mixing the two paid for bundle discovery twice. Both now read `BUNDLE_INDEX_CACHE`, making it once per process.

**Not done (out of lane):** `build_eager_path()`'s Windows `build_windows_index()` call remains uncached, even though `windows_apps::get_or_build_windows_index()` exists. The equivalent one-call-per-index fix is available and worth a follow-up.

### Proof

No new counter names (C6 honored). Counters cannot prove this lane: the lazy arm's PATH walks happen inside the `which` crate, which Sniff does not instrument, so `filesystem.io.read_dirs` reads zero on *both* arms and cannot distinguish them. Instrumenting `scan_path_executables` would also only ever count the first call per process (`OnceLock`), making any counter assertion order-dependent. The proofs are therefore behavioral, per the lane brief. No Rayon closure gained a counter, so no `performance::pooled_worker` guard was required.

| Test | Proves |
|---|---|
| `executable_index::eager_path_is_traversed_once_per_process` | Builds once under a PATH holding a shim, then rebuilds under a PATH sharing no directory with the first. The maps are equal, which a re-scan could not produce. Order-independent: it holds whether or not an earlier test populated the cache. |
| `host_capability::detect_traverses_path_through_the_shared_eager_index` | `detect()` leaves `EAGER_PATH_CACHE` populated — it cannot have used per-name `which` walks. Uses a `#[cfg(test)]` `eager_path_cache_is_populated()` seam. |
| `host_capability::detect_agrees_with_a_lazy_path_only_index` | `detect()` and `detect_with_index(&build_path_only())` agree on `has_bash`, both package-manager catalogs, and the default OS PM — the results-unchanged claim. |
| `executable_index::eager_path_only_agrees_with_lazy_path_only` | Eager and lazy PATH-only lookups agree on a shim, a real binary, and an absent name. |
| `executable_index::eager_path_only_index_has_no_fallback_layers` | The PATH-only surface: no bundles (macOS), no App Paths / install roots (Windows). |
| `executable_index::bundle_index_is_discovered_once_per_process` | `BUNDLE_INDEX_CACHE` is populated after `build()`, and `build()` / `build_eager_path()` share one bundle index. macOS-gated. |

Both routing changes were mutation-checked: reverting `detect()` to `build_path_only()` fails `detect_traverses_path_through_the_shared_eager_index`, and restoring the direct `build_bundle_index()` call fails `bundle_index_is_discovered_once_per_process`.

Test-process isolation note: the two cache-population assertions are exact under `cargo nextest` (process per test). Under a single-process `cargo test` they could pass vacuously if an earlier test populated the cache. The repo mandates nextest.

### Behavior risk

`detect()` now reads a process-global PATH map frozen at first use, where it previously re-read PATH per call. A process that **narrows** `PATH` between two `detect()` calls could see a stale hit, because `eager_lookup` falls back to `which` on a miss but never revalidates a hit. Widening PATH is safe (the `which` fallback still finds new entries). No in-repo caller mutates PATH between `detect()` calls; `ProgramsInfo::detect` has carried this same cache since `build_eager_path` was introduced.

## Lane A — as built

Implements R2. `build_aggregate_value` is now a pure projection; every fact it renders is observed before it runs.

### Counter deltas

Measured with `performance::testing::measure` on a 2-package Cargo-workspace fixture with one file in each of the four scopes, comparing the **post-detection** observation set before and after. Both arms ran in the same process on the same tree, so this is a drift-free comparison rather than a cross-run one.

| counter | before | after |
|---|---:|---:|
| `git.repository_discoveries` | 16 | **1** |
| `git.status_walks` | 8 | **0** |
| `remote.api_requests` | 0 | 0 |
| `filesystem.io.read_dirs` | 31 | **8** |
| `filesystem.io.metadata_probes` | 358 | **95** |
| `filesystem.io.file_opens` | 34 | **13** |
| `filesystem.repo.manifest_parses` | 24 | **8** |
| `filesystem.repo.package_enrichments` | 8 | **2** |

Command totals: **one** status walk (detection's, projected from thereafter) and **one** aggregate discovery context. The eight post-detection status walks are gone.

The non-Git reductions are a second-order win that C2 did not predict: `default_commit_family_set` reached `get_recent_commits_by_duration`, which ran its own `detect_repo` over the whole tree. `get_recent_commits_by_duration_with_repo` takes the already-detected `RepoInfo`, deleting that entire second repository detection.

### Contract deviations

1. **`GitRequest::full()` confirmed as the C1 floor.** Bare `repo --json` is neither `lightweight_repo_action` nor `changes_only_repo_action`, so `select_git_request` returns `full()`, which sets `include_file_changes`. The precondition is enforced rather than assumed: `ensure_detailed_status` rejects a `GitInfo` whose status reports dirty while carrying no `file_changes` — the one below-`full()` case that is detectable by inspection. A clean tree legitimately has neither and passes.

2. **`git.repository_discoveries == 1` required a change beyond C2's table.** `detect_repo_identity_with_repo` → `resolve_name` → `remote_basename` called the *path-based* `preferred_remote_url`, a second discovery on every repository whose root carries no manifest name. `remote_basename` now reads through the caller's handle via `api::preferred_remote_url_with_repo`, which routes to the same `resolve_origin_or_first` so preferred-remote semantics cannot drift.

3. **One intentional value change: `name`.** The old path called `detect_repo_identity(dir)` with the CLI's relative default `"."`; gix then reports a relative workdir whose `file_name()` is `None`, so `resolve_name`'s directory-name fallback returned `"unknown"`. `observe_repo_aggregate` absolutizes before discovery (the guard `git::api::repo_root` already applied, and which the old aggregate applied for `structure.root` but not for identity), so the fallback now yields the real directory name. Only reachable for a repository with **no root manifest name and no remote** — where the old value was meaningless. Verified: repositories with a remote (including rusty-biscuit itself) are byte-identical before and after. Schema and ordering are unchanged.

### Golden

`sniff-cli::snapshots::repo_aggregate_json_snapshot`, captured from clean HEAD (`acc4f3da4`) **before** any Lane A edit. Its strength is that the committed snapshot was replayed against clean-HEAD code in a detached worktree and **passed** — it pins pre-Phase-2 behavior, not post-Phase-2 behavior.

Normalized for host/run variance: commit ids and timestamps, temp paths (`redact_base_paths` handles the macOS `/var` → `/private/var` realpath), and gitconfig.

`git_status.file_changes` **order is nondeterministic on clean HEAD** — the gix status walk is parallel, reproduced 6/6 runs on the unmodified binary. The golden sorts that one field rather than dropping it. This is pre-existing and not a Lane A regression, but it means the aggregate's `file_changes` order is not a contract. The four `ScopeBucket`s are unaffected: `scope_paths` preserves `file_changes` order and the projection sorts afterward, matching `collect_changed_paths`'s sort/dedup byte-for-byte.

### Known residual — `build_aggregate_value` is not counter-silent

R2.7 says the builder performs no filesystem read. It records **4 `filesystem.io.canonicalizations`** and nothing else: the cwd-relative `context` block resolves the invoking directory against the package catalog through `output::filesystem`. No repository open, status walk, file read, subprocess, or request remains. `build_aggregate_value_performs_no_observation_beyond_path_normalization` allows exactly that one counter and fails on any other. **R6.2** (Phase 3) replaces those lookups with lexical prefix/depth comparisons over reusable normalized keys; tighten the assertion to `counters.is_empty()` then.

### Escalation — status walks were invisible through the planner (RESOLVED)

Lane A escalated that `git.status_walks` read `0` for a request that demonstrably walks status once. The claim was verified and is real; the cause was **not** a missing collector — `filesystem/mod.rs` and `lib.rs` both install one via `with_current_collector` on every stage thread.

**Root cause.** `increment_counter` writes to a *thread-local* `COUNTER_BUFFER`. Only two things ever drain it: `PerformanceCollector::snapshot`, which merges the **snapshotting** thread, and `WorkerCollector`/`PooledWorkerGuard` on drop. `with_current_collector` installed the collector and restored the previous one but **never flushed**, so any thread that used it and then exited — every `std::thread::scope` stage thread in the planner and every domain thread in `detect_with_plan` — silently discarded everything it recorded.

Phase 1 closed this for `ignore::build_parallel` workers and Rayon pools and did not reach `with_current_collector`. The result is the exact failure R1 exists to prevent, and it reached the archived baseline: **Phase 1's counter tables under-report every git counter for the integrated filesystem cases.**

**Fix.** `with_current_collector` now flushes the installed collector's thread-local buffers before restoring the previous collector (`sniff/lib/src/performance.rs`). Regression test: `filesystem::planner_counter_propagation::git_stage_counters_survive_the_scoped_thread`.

**Consequence for baselines.** Counters that were previously absent now appear. This is newly-*visible* work, not new work — every non-git counter is byte-identical to Phase 1 (see [Phase 2 counters](#phase-2-counters)). A later phase comparing against Phase 1's archived git counters must use the Phase 2 table below instead.

### Phase 2 counters

`cargo run -p sniff --release --example work_counts`, same host and fixtures as Phase 1. Every counter Phase 1 recorded is **unchanged**; the rows below are the ones Phase 1 could not see.

| Case | Phase 1 | Phase 2 |
|---|---|---|
| `staged_filesystem_summary_git_plus_repo` | all FS counters; `git.repository_discoveries: 1` | identical, **plus `git.status_walks: 1`** |
| `staged_filesystem_full_all_stages` | all FS counters; `git.repository_discoveries: 1` | identical, **plus `git.status_walks: 1`, `git.blob_loads: 4`, `git.file_diffs: 2`, `git.ref_walks: 2`, `git.commit_visits: 10`** |
| `repo_structure_huge_375_packages` | 13274 probes / 702 read_dirs / 454 parses / 300 enrichments | identical |
| `repo_full_huge_375_packages` | 4685 entries / 3755 accepted / 25975 probes / 754 parses | identical |
| `git_status_*_100_dirty` | 200/400 blob_loads, 100 diffs | identical, `git.status_walks: 1` now visible |

The staged cases are unchanged by design: neither exercises a scope the planner now narrows. `WalkScope`'s wins land on requests the baseline does not cover — a formatting-only request (`filesystem.walk.walks_started: 0`) and package-scoped inventory — both asserted by test rather than by this table. Lane A's wins are in the CLI aggregate, which this harness does not drive.

### Files

Primary: `repo/aggregate_view.rs` (new), `repo/aggregate.rs`, `repo/mod.rs`, `cli/output/repo_json.rs`, `cli/commands/mod.rs`.

Reuse seams added outside the primary set (all `*_with_repo` variants per the lane brief): `git/types.rs` (`GitRepo::with_cached_gix`), `git/worktree.rs`, `git/recent_commits.rs`, `git/api.rs`, `git/mod.rs`, `repo/identity.rs`. `cli/output/recent_commits.rs` lost `default_commit_family_set`, whose only caller was the aggregate.

### Files

Primary: `repo/aggregate_view.rs` (new), `repo/aggregate.rs`, `repo/mod.rs`, `cli/output/repo_json.rs`, `cli/commands/mod.rs`.

Reuse seams added outside the primary set (all `*_with_repo` variants per the lane brief): `git/types.rs` (`GitRepo::with_cached_gix`), `git/worktree.rs`, `git/recent_commits.rs`, `git/api.rs`, `git/mod.rs`, `repo/identity.rs`. `cli/output/recent_commits.rs` lost `default_commit_family_set`, whose only caller was the aggregate.

## Lane B — as built

### Walk scope is decided by consumers, never by Git

`filesystem/mod.rs` gained `WalkConsumers` (repo_full / docs / inventory) and `WalkScope` (`None` / `Repository` / `Package`). Two defects closed:

1. **Formatting started a walker for nothing.** `need_shared_view` included `request.include_formatting`, yet `detect_formatting` reads the `.editorconfig` chain directly and consumes nothing the walk produces. Formatting is now absent from the decision entirely (R3.1).
2. **A Git handle widened the scope.** The walk root was `discovered_git.repo_root()` whenever git was requested, so a package-scoped inventory request inside a monorepo enumerated the **whole repository**. `WalkScope::Package` now walks the resolved package root.

### Ordering is the mechanism, not an accident

`WalkScope::Package` runs structure-only repo detection **before** the walk, because resolving which package owns `root` requires membership — and structure-only detection needs no descendant walk of its own. `WalkScope::Repository` keeps the original order (walk, then detection consumes its evidence).

This is why the walk moved off its scoped thread onto the calling thread: it now runs concurrently with the git and formatting threads anyway (its root comes from the discovery handle, not the git *result*), so the spawn bought nothing and would have forced a join before detection.

### Clones removed (R3.4/R3.5)

- `repo_context.clone()` → `Option::take` at assembly. A request that needed `RepoInfo` only as docs context now drops it instead of cloning it.
- `view.docs.clone()` → `view.docs.take()`.

### Tests

`walk_scope_table` asserts the real `WalkScope::of` across all eight C4 rows. It replaces five `need_shared_view_*` tests that **re-derived the boolean inside the test body** and therefore passed regardless of what the planner did — they would not have caught either defect above. `formatting_only_request_starts_no_walker` asserts `filesystem.walk.walks_started == 0` via the counter, so it still fails if the decision is right but some stage walks anyway.

## Lane C — as built

### Completeness fields (C5)

`truncated: bool` + `limit: Option<usize>` on `FileInventory`, `FileAssociationBreakdown`, `LanguageSummary`; propagated through `summarize_file_inventory`, `filter_inventory`, `project_package_inventory`, and the CLI's `filter_file_breakdown`. `LanguageSummary::with_completeness_of(&inventory)` stamps the pair; `build_language_summary` keeps its signature because `repo::detection` calls it with maps that have no inventory behind them.

`truncated` is derived as `scanned > MAX_FILES`, not `>=`: a file only observes an over-cap index because the count already passed the cap, so a tree of exactly `MAX_FILES` files is complete.

### Saturation (R7.1/R7.2)

- Inventory-**only** walk (`scan_inventory_parallel`) returns `WalkState::Quit` at the cap — nothing else is observing, so enumerating the rest of the tree is pure waste.
- Combined walk (`system_view`) stops classification and its counters at the cap but **continues** while `collect_manifests`/`collect_docs` observers remain active, gated on `SharedWalkOptions::has_observer_beyond_inventory`. Quitting here would silently truncate the manifest index and docs — turning an inventory bound into a *wrong repository structure*.

### Tests

`inventory_only_walk_stops_at_the_cap_and_reports_truncation` and `combined_walk_keeps_going_past_saturation_for_its_other_observers` build a >`MAX_FILES` tree with a manifest per directory; the second asserts every manifest is still found, which is precisely what a walk that quit at the cap would lose. Truncated-run assertions cover cap, flag, ordering, and native-path validity — **not** an exact selected subset (R7.5). `completeness_serialization` proves complete results omit both fields, truncated results carry `true` + the cap, and legacy JSON without the fields still deserializes as complete.

## Acceptance

Run from `sniff/`:

| Command | Result |
|---|---|
| `just lint` | pass |
| `just test` (lib) | 1332/1333 |
| `cargo nextest run -p sniff-cli -E '!test(level2_)'` | 780/781 |
| `cargo run -p sniff --release --example work_counts` | table above |

The two failures are the **same two Phase 1 documented as pre-existing on clean HEAD** and are unrelated to this phase:

- `sniff filesystem::repo::area::tests::detect_area_errors_when_not_in_repo` — 30 s timeout × 4 retries.
- `sniff-cli::snapshots os_json_snapshot` — host drift; the snapshot pins macOS `26.5.1`, the host runs `26.5.2`.

## Gate for Phase 3

R2, R3, R7, and R13.1–R13.2 are implemented and asserted by counter. The planner's counter-propagation defect is closed, so Phase 3 can assert walk counts end-to-end. Two items are deliberately carried forward:

- `build_aggregate_value` still records 4 `canonicalize` calls via the cwd-relative `context` block (Lane A). That is R6.2's target; its test allows exactly that counter and fails on any other, and should tighten to `is_empty()` once Phase 3/4 land the normalized-path boundary.
- `attribute_paths` preserves the current first-match/lossy-string semantics verbatim. **Phase 4 replaces its internals with the deepest-prefix index** (R6.4/R6.5); this phase only moved ownership so that replacement has one site to change.

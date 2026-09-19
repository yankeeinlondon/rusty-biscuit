---
sub-spec: true
depends-on: ../../../spec.md
phase: 1
status: implemented
date: 2026-07-16
---

# Phase 1 — Trustworthy work accounting and baselines

Establish requirement **R1** of the umbrella specification before any optimization phase is evaluated. This phase adds instrumentation and baselines only. It must not change detection results, public output, ignore/prune semantics, or platform behavior.

The problem it closes: before this phase, work performed inside `ignore::build_parallel()` workers and Rayon pool workers was silently **absent** from every `PerformanceReport`. A worker recorded into its thread's buffers only if a collector was installed on that thread — which never happened — and a pooled thread parks rather than exits, so nothing drained it. A counter that reads zero because nobody counted is worse than no counter at all: a later phase could "prove" it removed work that was never measured.

## Stable counter names

Names live in `sniff/lib/src/performance/counters.rs` and are a contract: renaming one invalidates every archived baseline that used it. A counter absent from a report means zero — recording never writes a zero.

### Filesystem walking and classification

| Constant | Name | Unit |
|---|---|---|
| `FS_WALK_STARTS` | `filesystem.walk.walks_started` | One shared descendant walk started |
| `FS_WALK_ENTRIES` | `filesystem.walk.entries_visited` | One entry yielded by the walker (any file type) |
| `FS_INVENTORY_ACCEPTED` | `filesystem.file_inventory.files_accepted` | One file accepted into the inventory and classified |
| `FS_INVENTORY_SATURATED` | `filesystem.file_inventory.entries_over_cap` | One entry rejected because the cap was already met; nonzero ⇒ truncated |
| `FS_DOCS_PARSED` | `filesystem.docs.documents_parsed` | One Markdown document parsed for metadata |

### Filesystem primitives Sniff controls

| Constant | Name | Unit |
|---|---|---|
| `FS_FILE_OPENS` | `filesystem.io.file_opens` | One file Sniff opened or read whole |
| `FS_BYTES_READ` | `filesystem.io.bytes_read` | Bytes read by Sniff-initiated reads |
| `FS_METADATA_PROBES` | `filesystem.io.metadata_probes` | One `metadata`/`symlink_metadata`/`exists`/`is_dir` probe |
| `FS_READ_DIRS` | `filesystem.io.read_dirs` | One directory enumeration outside the shared walker |
| `FS_CANONICALIZATIONS` | `filesystem.io.canonicalizations` | One `canonicalize` call |

### Repository structure

| Constant | Name | Unit |
|---|---|---|
| `REPO_MANIFEST_PARSES` | `filesystem.repo.manifest_parses` | One package manifest parsed |
| `REPO_LOCKFILE_PARSES` | `filesystem.repo.lockfile_parses` | One lockfile parsed |
| `REPO_CONFIG_PARSES` | `filesystem.repo.config_parses` | One root-scoped tool/test-runner config parsed |
| `REPO_PACKAGE_ENRICHMENTS` | `filesystem.repo.package_enrichments` | One package boundary enriched |

### Git

| Constant | Name | Unit |
|---|---|---|
| `GIT_DISCOVERIES` | `git.repository_discoveries` | One upward repository discovery |
| `GIT_OPENS` | `git.repository_opens` | One repository open at a known path |
| `GIT_STATUS_WALKS` | `git.status_walks` | One working-tree status walk |
| `GIT_BLOB_LOADS` | `git.blob_loads` | One blob or worktree file side loaded for diffing |
| `GIT_FILE_DIFFS` | `git.file_diffs` | One per-file content diff |
| `GIT_REF_WALKS` | `git.ref_walks` | One ref enumeration (per enumeration, **not** per ref) |
| `GIT_COMMIT_VISITS` | `git.commit_visits` | One commit visited by a revision/ancestry walk |
| `GIT_WORKTREE_OPENS` | `git.worktree_opens` | One linked worktree opened as a repository |

### Subprocess, remote, network

| Constant | Name | Unit |
|---|---|---|
| `PROC_SPAWNS` | `process.spawns` | One child process spawned |
| `PROC_TIMEOUTS` | `process.timeouts` | One child killed for exceeding its deadline |
| `REMOTE_REQUESTS` | `remote.api_requests` | One outbound provider API request |
| `remote_operation(op)` | `remote.api_requests.<op>` | Same request, by operation slug |
| `NETWORK_WAN_ATTEMPTS` | `network.wan_ip.endpoint_attempts` | One WAN IP endpoint attempt |

Operation slugs in use: `metadata`, `tree`, `contents`, `workflow_runs`, `pulls`, `issues`, `tags`, `releases`.

## Owned instrumentation boundaries

A counter is placed where **one increment corresponds to one meaningful unit of work**, never around every standard-library call. Where a chokepoint exists, only the chokepoint is counted — double counting silently corrupts a baseline and is worse than not counting.

| Boundary | Counted at | Deliberately not counted |
|---|---|---|
| Shared filesystem walk | `system_view::build_filesystem_system_view` worker callback | — |
| Standalone inventory walk | `file_types::classify::scan_inventory_parallel` worker callback | — |
| File classification | `classify_file` / `read_prefix` | Per-registry lookups (in-memory) |
| Manifest reads | `repo::detection::read_counted_manifest`, `repo::identity::read_counted` | ~25 `ManifestCache` accessor call sites and all 8 `aggregate.rs` read sites — they route through the chokepoints, so counting them would report cache *asks* rather than unique reads |
| Package enrichment | `repo::detection::create_package` | `refresh_package_boundaries` — it re-derives languages for boundaries `create_package` already produced; counting both would report every boundary twice |
| Metadata probes | `repo::detection::probe_exists` / `probe_is_dir` | `is_generated_manifest` gets `FS_*` but not a parse counter — it substring-scans, it does not parse |
| Canonicalization | `repo::detection::canonicalize_path` | 8 `types.rs` call sites routing through it |
| Repository discovery/open | `git::open::trusted_discover` / `trusted_open`, counted *before* the call so a failed discovery still shows its cost | `git::api` — every boundary routes through `trusted_discover` |
| Status walks | `get_repo_status_with_changes`, `is_repo_dirty`, `get_repo_status_counts_detailed` | — |
| Blob loads | `status::read_blob_or_empty` plus the two worktree-side reads | `line_count_of_blob_content` / `diff_blobs` — both route through `read_blob_or_empty` |
| Subprocesses | the four timeout helpers (`os/time.rs`, `programs/schema.rs`, `programs/host_capability.rs`, `hardware/audio.rs`) plus direct spawn sites | `execute_install`/`execute_versioned_install` — they delegate to already-counted variants |
| Remote requests | `remote::mod::count_api_request`, called at each request site | — |

### Known accounting asymmetries

Recorded here so a later phase does not misread them as regressions:

- **`GIT_WORKTREE_OPENS` is a labeled subset of `GIT_OPENS`.** The two are not disjoint.
- **Bitbucket's `tree` count reads higher than GitHub's for equivalent work.** Bitbucket has no recursive-tree flag, so callers walk directory-by-directory and each call is genuinely one tree request. The asymmetry is real, not an artifact.
- **GitLab's `get_repo_metadata` counts as `tree`, not `metadata`.** Absent `GetProject`, its metadata path is implemented as a full tree fetch; naming it `metadata` would hide a tree fetch from the tree counter.
- **Retries count as separate requests.** The authenticated→anonymous fallback puts two requests on the wire and increments twice.
- **Python `tox.ini`/`setup.cfg` land in `REPO_MANIFEST_PARSES`, not `REPO_CONFIG_PARSES`.** They share the untyped-manifest cache; splitting at the caller would over-count on cache hits.

## Worker accounting

Two mechanisms, both in `sniff/lib/src/performance.rs`:

- **`WorkerCollector`** — constructed with `inherit()` on the spawning thread, `activate()`d from the first callback that runs on the worker thread, and flushed on drop. It must be owned by per-worker state that drops on the worker thread; `WorkerBuffers` and `LocalClassifications` own it next to the result buffers they append, so a worker's counters and its results land in the request together.
- **`pooled_worker(collector) -> PooledWorkerGuard`** — the Rayon form. Rayon threads park rather than exit, so the guard flushes per closure call. It is inert when nothing is collecting, and it defers to an already-installed collector because Rayon runs some items inline on the calling thread, whose live buffers must not be cleared.

Applied at: the two `ignore::build_parallel()` walkers, `docs.rs`'s five `par_iter` sites, `host_capability::detect_verified_lang_pkg_mgrs`, and `remote_refresh`'s worktree fan-out.

**Remaining gap:** `executable_index.rs`, `find_program.rs`, `just.rs`, and `programs/mod.rs`'s `rayon::join` pairs are not wrapped. They perform no counted work today, so they read zero correctly rather than misleadingly. Any phase that adds a counter inside those closures must add a guard in the same change.

## Default-path gating

`performance::is_collecting()` is a single relaxed atomic load over a process-wide count of installed collectors, `true` unconditionally under the `metrics` feature. Every recording entry point checks it first, so with nothing collecting:

- no clock is read (`StageTimer::start` and `classify_file` hold `Option<Instant>`);
- no thread-local storage is touched;
- no counter name is formatted.

This replaced the previous `#[cfg(feature = "metrics")]` gates in `classify.rs` and `scan_inventory_parallel`. Those gates made the default path cheap by making structured collection **blind** — exactly the R1 defect. The runtime gate keeps the default path cheap while letting `--json` performance output see the same work.

## Test-access strategy

`performance::testing` is `#[cfg(test)]` and crate-private. `measure(f) -> (T, WorkCounts)` runs `f` under a private collector that dies with the call. No production type carries test-only state and nothing is cached process-globally. Integration tests outside the crate use the ordinary public `with_current_collector` + `snapshot` API.

## Tests added

`sniff/lib/src/filesystem/system_view.rs`:

- `parallel_walk_counts_every_accepted_file_exactly_once` — the counter equals the classifications in the result. Reads short if a worker's buffer is dropped when its thread parks; reads long if a worker flushes twice.
- `parallel_walk_visits_at_least_every_file_it_accepts` — entries ≥ accepted, entries cover every file and directory, exactly one walk per request.
- `parallel_and_serial_walks_agree_on_accepted_work` — the parallel walker's accounting matches an independently-walked serial total.
- `worker_counts_do_not_leak_into_a_later_request` — a request that walks nothing records nothing, proving no stale worker buffer survives.
- `uncollected_walk_records_nothing_into_a_later_request` — the disabled path changes no results and records no counters or stages.

`sniff/lib/src/performance.rs`:

- `nothing_is_collecting_by_default`, `recording_outside_a_collector_is_dropped_not_buffered`, `stage_timer_reads_no_clock_when_inactive`, `measure_scopes_counts_to_its_own_call`, `pooled_worker_attributes_thread_work_to_the_request`, `pooled_worker_on_the_calling_thread_does_not_discard_pending_counts`.

## Baseline

### Environment

| Property | Value |
|---|---|
| Host | macOS 26.5.2, arm64 (Apple Silicon), 16 logical cores |
| Toolchain | rustc 1.96.0 (ac68faa20 2026-05-25), profile `release` |
| Base commit | `acc4f3da4` |
| Captured | 2026-07-16 |
| Harness | `cargo run -p sniff --release --example work_counts` |

Warm-cache, synthetic fixtures on APFS. **Directional only** — never compare timings across hosts or runner classes. Counters are the comparison keys.

### Fixtures

| Fixture | Size |
|---|---|
| `large_monorepo` | 60 Rust + JS/Python packages, 2 dirty files |
| `huge_monorepo` | **375 packages** — 200 Rust, 100 JS, 50 Python, 25 Go; 10 files/package |
| `git_repo_with_dirty_files(100)` | 100 modified files |

### Counter values

`staged_filesystem_summary_git_plus_repo` — `GitRequest::summary()` + `RepoRequest::structure()`, no docs/formatting/inventory:

| counter | value |
|---|---:|
| `filesystem.io.metadata_probes` | 3984 |
| `filesystem.io.read_dirs` | 212 |
| `filesystem.io.file_opens` | 127 |
| `filesystem.io.bytes_read` | 8718 |
| `filesystem.io.canonicalizations` | 90 |
| `filesystem.repo.manifest_parses` | 124 |
| `filesystem.repo.package_enrichments` | 90 |
| `git.repository_discoveries` | 1 |

`staged_filesystem_full_all_stages` — `FilesystemRequest::new()`:

| counter | value |
|---|---:|
| `filesystem.walk.entries_visited` | 638 |
| `filesystem.file_inventory.files_accepted` | 395 |
| `filesystem.docs.documents_parsed` | 182 |
| `filesystem.io.metadata_probes` | 7885 |
| `filesystem.io.read_dirs` | 422 |
| `filesystem.io.file_opens` | 370 |
| `filesystem.io.canonicalizations` | 271 |
| `filesystem.repo.manifest_parses` | 214 |
| `filesystem.repo.package_enrichments` | 180 |
| `git.repository_discoveries` | 1 |

`repo_structure_huge_375_packages` — `RepoRequest::structure()`:

| counter | value |
|---|---:|
| `filesystem.io.metadata_probes` | 13274 |
| `filesystem.io.read_dirs` | 702 |
| `filesystem.io.file_opens` | 457 |
| `filesystem.io.bytes_read` | 31908 |
| `filesystem.io.canonicalizations` | 300 |
| `filesystem.repo.manifest_parses` | 454 |
| `filesystem.repo.package_enrichments` | 300 |

`repo_full_huge_375_packages` — `RepoRequest::full()`:

| counter | value |
|---|---:|
| `filesystem.walk.entries_visited` | 4685 |
| `filesystem.walk.walks_started` | 1 |
| `filesystem.file_inventory.files_accepted` | 3755 |
| `filesystem.io.metadata_probes` | 25975 |
| `filesystem.io.read_dirs` | 1403 |
| `filesystem.io.file_opens` | 758 |
| `filesystem.io.bytes_read` | 47708 |
| `filesystem.io.canonicalizations` | 600 |
| `filesystem.repo.manifest_parses` | 754 |
| `filesystem.repo.package_enrichments` | 600 |

`git_status_file_changes_100_dirty` (`GitRequest::full()`) and `git_status_unified_diffs_100_dirty` (`GitRequest::deep()`):

| counter | full | deep |
|---|---:|---:|
| `git.status_walks` | 1 | 1 |
| `git.blob_loads` | 200 | 400 |
| `git.file_diffs` | 100 | 100 |
| `git.ref_walks` | 2 | 3 |
| `git.commit_visits` | 1 | 1 |
| `git.repository_discoveries` | 1 | 1 |

### Directional timings

| Case | Elapsed |
|---|---:|
| `staged_filesystem_summary_git_plus_repo` | 53.3 ms |
| `staged_filesystem_full_all_stages` | 47.5 ms |
| `repo_structure_huge_375_packages` | 86.1 ms |
| `repo_full_huge_375_packages` | 152.1 ms |
| `git_status_file_changes_100_dirty` | 6.7 ms |
| `git_status_unified_diffs_100_dirty` | 8.8 ms |

### Criterion IDs for Phases 2–7

Archived under the Criterion baseline name **`phase-01`**:

```text
cargo bench -p sniff --features network --bench perf -- \
    --save-baseline phase-01 "^(filesystem_staged|filesystem_repo|git_ops/status)"
```

Compare from a later phase with `--baseline phase-01`.

| Criterion ID | `phase-01` median |
|---|---:|
| `filesystem_staged/staged_filesystem_summary_git_plus_repo` | 34.6 ms |
| `filesystem_staged/staged_filesystem_full_all_stages` | 59.8 ms |
| `filesystem_repo/repo_structure_huge_375_packages` | 107.0 ms |
| `filesystem_repo/repo_full_huge_375_packages` | 238.5 ms |
| `filesystem_repo/package_boundary_refresh_huge` | 887 µs |
| `git_ops/status_dirty_flag/100` | 2.16 ms |
| `git_ops/status_file_changes/100` | 10.1 ms |

The `huge_500_packages` ids were renamed to `huge_375_packages` in this phase. The fixture holds 375 packages and always did; the name was wrong, and a benchmark id that misstates its own workload invalidates every baseline compared against it. **Phase 2 must not compare against any archived `huge_500` result** — the workload is unchanged, but only post-rename runs are on record here.

### Timing-noise warning — read before comparing

These timings were captured at **load average ~82** on a 16-core host with concurrent build agents running. They are unusable as an absolute reference. The evidence for how unusable:

`filesystem_repo/repo_structure_huge_375_packages` measured **270.6 ms** in one run and **107.0 ms** in the next, on identical code and an identical fixture, minutes apart. That is a **2.5× swing from host load alone** — larger than most optimizations this feature will attempt.

Consequences for later phases:

- **Never compare a Phase-N timing against the `phase-01` timing above.** Re-measure both arms back-to-back on the same idle host (a drift bracket), or compare counters.
- The `phase-01` Criterion baseline is retained for *shape* and for same-session bracketed comparison, not as a threshold.
- The counter tables above are the real baseline. They are exact, host-independent, and load-independent — which is the entire reason R1 exists.

## What the baseline already shows

Findings a later phase should expect, recorded now so they are not mistaken for regressions introduced later:

1. **`RepoRequest::structure()` enriches packages.** `staged_filesystem_summary_git_plus_repo` records 90 `package_enrichments` and the 375-package structure case records 300, despite structure mode's documented shallow contract. This is direct evidence for **R5.4** and quantifies it.
2. **The structure/full ratio on the 375-package fixture is ~1.8×** (86.1 ms → 152.1 ms), not 10–50×. This corroborates the umbrella spec's instruction to retire the unqualified "10–50×" claim (`docs.rs::detect_repo_packages` still states it).
3. **Metadata probes dominate.** 25,975 probes for the 375-package full case — roughly 69 per package, and ~35× the file-open count. **R4/R5's** index work targets exactly this.
4. **Structure-only detection starts no shared walk** (`filesystem.walk.*` absent) but performs 702 `read_dirs`, confirming the separate-enumeration problem **R4** addresses.
5. **Rich status loads two blob sides per dirty file and four under `deep()`** (200 / 400 loads for 100 files) while computing 100 diffs — the reload **R8** targets.

## Acceptance

Commands run from `sniff/`:

| Command | Result |
|---|---|
| `just lint` | pass |
| `just build` | pass |
| `just test` | 1343/1344 lib; 772/773 cli |
| `cargo run -p sniff --release --example work_counts` | baseline above |

Two failures are **pre-existing and reproduce identically on clean HEAD (`acc4f3da4`)**, verified in a detached worktree rather than assumed:

- `sniff filesystem::repo::area::tests::detect_area_errors_when_not_in_repo` — times out at 30 s × 4 retries.
- `sniff-cli::snapshots os_json_snapshot` — host drift; the snapshot pins macOS `26.5.1`, the host runs `26.5.2`.

Neither is caused by this phase and neither is in its scope.

## Gate for Phase 2

Parallel-worker counters are visible and repeatable on the same fixture, and the disabled path is proven to record nothing. Phase 2 may begin.

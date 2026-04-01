# Sniff Library Review: Ergonomics And Performance

Date: 2026-04-01

Scope: `sniff/lib` only. I did not review `sniff/cli` except where it helps illustrate caller needs.

## Executive Summary

The library is moving in the right direction. There are already several good precedents for selective, cheaper collection:

- `detect_repo_structure()` exists as a fast path instead of always doing full package scans (`sniff/lib/src/filesystem/repo.rs:388-395`).
- `detect_hardware_summary()` exists as a fast path instead of always paying for audio, storage, and GPU (`sniff/lib/src/hardware/mod.rs:134-188`).
- `detect_docs_with_packages()` exists to avoid redundant repo detection (`sniff/lib/src/filesystem/docs.rs:142-151`).
- Many subsystems already expose smaller atomic queries, especially `GitRepo` (`sniff/lib/src/filesystem/git.rs:522-618`).

The main issue is that the top-level and grouped APIs still mostly expose "full group" collection, so callers still pay for expensive subgroup members unless they drop down into module-specific APIs themselves. In practice that means the library has the building blocks for ergonomic selective collection, but the default composition layers do not yet expose them cleanly.

The highest-value direction is:

1. Replace coarse skip flags with per-group request types or selectors.
2. Make grouped collectors staged DAGs that share intermediate state and parallelize only independent work.
3. Split "summary" from "full detail" for git, repo/filesystem, OS, network, and hardware.

## What Is Working Well

- Package-boundary-aware filesystem scanning is the right abstraction for real monorepos. `detect_filesystem()` already avoids nested package double-counting for the active package (`sniff/lib/src/filesystem/mod.rs:68-86`).
- Git internals are reasonably modular. `GitRepo` already exposes smaller building blocks like `branches()`, `config()`, `tracking_status()`, and `file_changes()` (`sniff/lib/src/filesystem/git.rs:576-617`).
- Program detection already parallelizes within a category via Rayon (`sniff/lib/src/programs/find_program.rs:30-40`, `111-118`).
- The codebase is aware of expensive paths and has started introducing lighter alternatives. The missing piece is making those alternatives first-class for callers.

## Highest-Priority Findings

### 1. The top-level API is still too coarse for callers with mixed cost requirements

Evidence:

- `SniffConfig` only exposes whole-section skips plus `deep`, `commit_count`, and `include_cpu_usage` (`sniff/lib/src/lib.rs:57-74`).
- `detect_with_config()` runs OS, hardware, network, and filesystem sequentially (`sniff/lib/src/lib.rs:179-218`).

Why this matters:

- A caller cannot say "filesystem, but only git summary and repo structure".
- A caller cannot say "hardware summary, not audio devices".
- A caller cannot say "network interfaces, but never WAN IP".
- Expensive subgroup members stay coupled to broad group entry points, so callers must abandon the ergonomic top-level API and manually compose lower-level functions.

Recommendation:

- Replace the current coarse config with per-domain request types, for example:

```rust
let result = sniff::detect_with_plan(
    DetectionPlan::new()
        .os(OsRequest::summary())
        .hardware(HardwareRequest::summary())
        .network(NetworkRequest::interfaces_only())
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::summary())
                .repo(RepoRequest::structure())
                .docs(false)
        )
);
```

- Keep `detect()` as the "full/default" convenience path, but make `detect_with_plan()` the real ergonomic API for library callers.
- Execute independent top-level sections concurrently once request shapes are explicit.

### 2. `detect_filesystem()` still composes expensive work sequentially and leaves obvious shared-work savings unused

Evidence:

- `detect_filesystem()` always runs git, repo, inventory scan, formatting, and docs in sequence (`sniff/lib/src/filesystem/mod.rs:61-100`).
- It calls `detect_docs(root)`, which re-discovers the repo and re-runs `detect_repo()` even though repo/package info is already available (`sniff/lib/src/filesystem/mod.rs:90-91`, `sniff/lib/src/filesystem/docs.rs:83-110`, `136-151`).
- `summarize_file_inventory()` recomputes `summarize_languages()` twice (`sniff/lib/src/filesystem/file_types/aggregate.rs:29-35`).
- `detect_filesystem()` separately calls `summarize_languages()` and `summarize_file_inventory()` on the same inventory, so today the same inventory is summarized three times (`sniff/lib/src/filesystem/mod.rs:87-88`, `sniff/lib/src/filesystem/file_types/aggregate.rs:29-35`).

Why this matters:

- The docs fast path already exists but is not used.
- Triple summarization is a pure waste on every filesystem call.
- Grouped collection should be the place where shared work is most aggressively reused.

Recommendation:

- Change `detect_filesystem()` to:
    - use `detect_docs_with_packages()` when repo info is available,
    - compute language + framework summaries once,
    - return a combined summary struct derived from one pass over the inventory.
- Make filesystem collection a staged DAG:
    - stage 1: repo root / git handle
    - stage 2: repo structure + current-package selection
    - stage 3: shared file inventory
    - stage 4: docs / formatting / summaries in parallel where independent

### 3. Git "full detection" eagerly computes the most expensive shape, even for cheap callers

Evidence:

- `detect_full()` eagerly gathers recent commits, full repo status, remotes, worktrees, config, branches, and tracking every time (`sniff/lib/src/filesystem/git.rs:622-662`).
- `get_repo_status_with_changes()` computes diff stats for each dirty file, then builds full unified diffs for each dirty file (`sniff/lib/src/filesystem/git.rs:1207-1498`).
- `get_worktrees()` calls `get_repo_status_with_changes()` for every worktree just to get `dirty` and `changed_files` (`sniff/lib/src/filesystem/git.rs:1834-1931`, especially `1906-1912`).
- Deep mode performs `git fetch --quiet --prune <remote>` sequentially per remote (`sniff/lib/src/filesystem/git.rs:1697-1716`).

Why this matters:

- Many callers only need repo root, branch, dirty counts, or remotes.
- Full per-file patch material is much more expensive than dirty counts.
- Worktree summary currently pays for diff generation it immediately throws away.

Recommendation:

- Split git collection into explicit levels:
    - `GitRequest::summary()` -> repo root, branch, dirty counts, maybe remotes without branch lists
    - `GitRequest::status_counts()`
    - `GitRequest::file_changes()` -> per-file stats, no unified diffs
    - `GitRequest::patches()` -> full diffs only when requested
    - `GitRequest::worktrees_summary()` -> dirty flag and counts without patch generation
- Refactor `get_repo_status_with_changes()` into layered helpers:
    - one status enumeration pass
    - optional line stats
    - optional unified diff materialization
- Make remote refresh explicitly opt-in and rename `deep` to something precise like `refresh_remote_tracking`.

### 4. Monorepo repo detection still over-scans the tree and repeats work across packages

Evidence:

- `detect_repo_inner()` sequentially probes each workspace system (`sniff/lib/src/filesystem/repo.rs:397-423`).
- In full mode, it walks each discovered workspace package tree again via `discover_packages_from_manifests_in_tree()` (`sniff/lib/src/filesystem/repo.rs:429-443`, `1907-1968`).
- After that, `refresh_package_boundaries()` scans every package tree again with `scan_file_inventory_with_exclusions()` (`sniff/lib/src/filesystem/repo.rs:1857-1895`, `1631-1645`).
- Each inventory scan has its own `MAX_FILES` budget of 10,000 (`sniff/lib/src/filesystem/file_types/classify.rs:13-16`, `52-59`), so a large monorepo can pay that budget repeatedly across overlapping package scans.

Why this matters:

- The current architecture is effectively "detect package boundaries, then rescan each package", which becomes expensive on large nested workspaces.
- Nested packages worsen the multiplier effect.
- The code already has the right conceptual model, but the execution model is still scan-heavy.

Recommendation:

- Build a single manifest index for the repo root, then derive package boundaries from that index rather than repeatedly walking each package tree.
- Build one repo-level `FileInventory`, then project package-level summaries out of it by relative-path prefix instead of re-scanning the filesystem for every package.
- Cache canonicalized paths instead of recomputing them repeatedly inside package merge/boundary logic (`sniff/lib/src/filesystem/repo.rs:1614-1616`, `1761-1777`, `1857-1863`).
- Parallelize package-level summarization once the shared inventory exists.

### 5. Network detection always includes WAN IP lookup, even though that is the most expensive and least local part of the group

Evidence:

- `detect_network()` calls `detect_wan_ip()` before local interface enumeration (`sniff/lib/src/network/mod.rs:98-100`).
- WAN detection spawns a thread, builds a Tokio runtime, performs HTTP, and then immediately blocks on `join()` (`sniff/lib/src/network/mod.rs:202-235`).

Why this matters:

- A caller asking for local interfaces or primary NIC should not pay for an external HTTP call.
- The current implementation is synchronous from the caller’s perspective, even though it spawns a thread internally.
- WAN IP is a classic expensive/optional subgroup member.

Recommendation:

- Move WAN IP behind an explicit `NetworkRequest::include_wan_ip(bool)` or separate `detect_wan_ip()` API.
- If WAN lookup remains in grouped detection, do it concurrently with local interface enumeration and return it as optional late data rather than blocking the whole call.
- Consider a TTL cache instead of a process-lifetime `OnceLock`, since WAN IP can change during long-lived processes.

### 6. OS detection includes optional expensive probes on the hot path

Evidence:

- `detect_os()` always performs package-manager detection and time detection (`sniff/lib/src/os/mod.rs:176-217`).
- On Linux, `detect_ntp_status()` can spend up to 10 seconds across two `timedatectl` calls (`sniff/lib/src/os/time.rs:293-315`).
- Linux package manager detection scans every known package manager executable across PATH (`sniff/lib/src/os/package_manager.rs:819-852`), using repeated per-command filesystem checks (`336-420`).

Why this matters:

- For most callers, OS name/version/kernel/arch are cheap and enough.
- NTP status is useful, but it should not be hidden inside the default OS group if it can stall for seconds.
- Package-manager inventory is a nice subgroup, not always a default requirement.

Recommendation:

- Split OS into:
    - core identity,
    - locale/timezone,
    - NTP status,
    - package managers.
- Lower the Linux NTP timeout aggressively or make it opt-in.
- Add a cheap `detect_os_summary()` and wire it into the top-level planner.

### 7. Hardware already has a fast path, but the top-level API does not expose it

Evidence:

- `detect_hardware()` always includes audio, storage, and GPU (`sniff/lib/src/hardware/mod.rs:71-132`).
- `detect_hardware_summary()` already exists and explicitly skips those expensive parts (`sniff/lib/src/hardware/mod.rs:134-188`).
- `SniffConfig` exposes `include_cpu_usage`, but there is no matching "summary/full" hardware selector (`sniff/lib/src/lib.rs:57-74`).
- `detect_hardware_with_usage()` does not currently add CPU usage at all; it just calls `detect_hardware()` (`sniff/lib/src/hardware/mod.rs:190-210`).

Why this matters:

- The library already knows the cost split; callers just cannot ask for it ergonomically.
- `include_cpu_usage` is also misleading today because it suggests a cost/behavior change that does not actually occur.

Recommendation:

- Expose `HardwareRequest::{summary, full}` at the top level.
- Rename `include_cpu_usage` until real usage sampling exists, or implement the sampling.
- Consider parallelizing hardware subgroup collection after CPU/memory if platform APIs permit it.

### 8. Programs detection is not actually parallel across categories and repeats the same search work

Evidence:

- `ProgramsInfo::detect()` claims category-parallel detection, but constructs each category sequentially (`sniff/lib/src/programs/mod.rs:168-183`).
- Each category separately calls `find_programs_with_source_parallel()` with its own program list (`sniff/lib/src/programs/editors.rs:86-99`, `sniff/lib/src/programs/utilities.rs:94-140`, similar pattern in other categories).
- `find_program_with_source()` does PATH lookup and then macOS app-bundle lookup per program (`sniff/lib/src/programs/find_program.rs:71-87`).

Why this matters:

- PATH traversal and macOS app-bundle discovery are repeated across categories.
- The grouped API is less efficient than it looks from the docs.
- This is also an ergonomics issue: the API suggests a single coherent programs scan, but the implementation is many separate scans.

Recommendation:

- Build one shared program index per detection run:
    - snapshot PATH once,
    - optionally scan macOS app bundles once,
    - resolve all known aliases from that index.
- Then derive category views from the shared index.
- Either actually parallelize categories or remove the misleading doc comment.

### 9. Performance intent is visible, but the current docs/tests do not enforce it tightly

Evidence:

- The integration test comment says fast path should complete in `<300ms`, but the assertion allows `20000ms` (`sniff/lib/tests/integration.rs:47-58`).
- The README still documents the crate as `sniff-lib` / `sniff_lib` even though the package and tests use `sniff` (`sniff/lib/README.md:3`, `22-27`, `35`, `80`, `89`, `95`).

Why this matters:

- The library needs sharper guardrails if performance regressions are a design concern.
- Outdated docs make the selective API story harder for callers to trust.

Recommendation:

- Replace the single broad elapsed-time test with smaller targeted tests around known hot paths and expensive toggles.
- Add perf-oriented regression tests for:
    - repo structure vs. full repo scan,
    - git summary vs. git patches,
    - network interfaces vs. WAN lookup,
    - hardware summary vs. full hardware.
- Bring the README in sync with the actual crate name and current selective APIs.

## Concrete Refactor Plan

### Phase 1: Quick wins

- Use `detect_docs_with_packages()` inside `detect_filesystem()`.
- Fix `summarize_file_inventory()` so it does not call `summarize_languages()` twice.
- Stop using full dirty-file diffs when `get_worktrees()` only needs a dirty boolean and count.
- Correct the misleading "runs detection in parallel" doc comment in `ProgramsInfo::detect()`.
- Clarify or rename `include_cpu_usage`.

### Phase 2: Better grouped requests

- Introduce `OsRequest`, `HardwareRequest`, `NetworkRequest`, `FilesystemRequest`, and `GitRequest`.
- Keep current convenience functions as wrappers over request defaults.
- Replace `deep: bool` with narrower options:
    - `refresh_remote_tracking`
    - `include_remote_branch_details`
    - `include_commit_remote_containment`
    - `include_dependency_registry_enrichment`

### Phase 3: Shared-work architecture

- Repo/filesystem:
    - one manifest index,
    - one shared repo-level inventory,
    - package projections from shared state.
- Programs:
    - one executable index per run,
    - category views built from the same data.
- Git:
    - layered status pipeline so counts, line stats, and patches are independently selectable.

## Suggested API Direction

The design pressure here is consistent across domains: callers need grouped results, but they also need to exclude expensive subgroup members. The cleanest model is "grouped requests with explicit detail levels".

The main abstraction should be:

- grouped result structs for ergonomic consumption,
- request structs for cost control,
- internal shared-state builders so grouped requests do not duplicate work.

That gives callers three good modes:

1. `detect()` for "give me a sensible default".
2. `detect_with_plan(...)` for "give me only the subsets I want".
3. module-level direct calls for expert/manual composition.

## Bottom Line

The library already contains most of the ingredients needed to satisfy both performance-sensitive callers and convenience-first callers. The main work left is to move that selectivity up into the public grouped APIs and to restructure grouped collectors around shared intermediate state.

If I had to prioritize only three changes, I would do these first:

1. Introduce request/detail types for top-level, filesystem, git, network, OS, and hardware.
2. Refactor filesystem/repo collection around one shared repo inventory and one shared manifest index.
3. Split git status into summary, file stats, and patch detail so callers stop paying for unified diffs by default.

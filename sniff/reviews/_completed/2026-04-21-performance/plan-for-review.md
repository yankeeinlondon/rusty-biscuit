---
phases: 4
starting_phase: 4
status: complete
---

# Performance Review Implementation Plan

**Review Date:** 2026-04-21  
**Source:** `sniff/reviews/2026-04-21-performance/review.md`  
**Total Issues:** 23  

---

## Phase 1: Critical Fixes (Issues 1–2)

**Goal:** Eliminate severe slowdowns and pathological scaling that affect all platforms.

| # | Issue | Files | Approach |
|---|-------|-------|----------|
| 1 | Performance collector mutex contention | `performance.rs`, `classify.rs` | Replace `Mutex<BTreeMap>` with thread-local `HashMap` aggregation + merge in `snapshot()`. Accept `&'static str` for known stage names. |
| 2 | Windows index rebuilt per lookup | `find_program.rs` | Cache `build_windows_index()` in a `OnceLock` or route all lookups through the existing `ExecutableIndex`. |

**Deliverables:**
- [ ] Thread-local performance collector with merge step
- [ ] Windows index cached across program lookups
- [ ] Criterion bench showing improvement on 10k-file walk

---

## Phase 2: High-Impact Algorithmic & I/O Fixes (Issues 3–6)

**Goal:** Fix O(n²) scaling, redundant I/O, and platform-specific serialization.

| # | Issue | Files | Approach |
|---|-------|-------|----------|
| 3 | O(n²) package graph resolution | `repo/detection.rs` | Build `HashSet<&str>` of package names once; use O(1) lookups in nested loops. |
| 4 | Repeated HEAD tree resolution | `git/detection.rs` | Resolve HEAD tree once in `get_repo_status_with_changes` and pass `&Tree` into `get_file_diff_stats`. |
| 5 | macOS audio serialization blocks GPU | `hardware/mod.rs` | Re-test IOKit GPU path for CoreAudio safety; if clean, run audio/storage/GPU in parallel on macOS. |
| 6 | Redundant filesystem walks | `filesystem/mod.rs`, `system_view.rs`, `classify.rs` | Expand `need_shared_view` to include `include_formatting`; pass classified inventory to repo detection; eliminate per-package re-scans. |

**Deliverables:**
- [ ] HashSet-based package name lookups
- [ ] Single HEAD tree resolution per status call
- [ ] macOS hardware parallelism restored (if testing confirms safety)
- [ ] Unified shared walk for all filesystem-scanning features

---

## Phase 3: Medium-Effort Scalability & Caching (Issues 7–15)

**Goal:** Reduce per-call overhead and improve scaling for monorepos and repeated detections.

| # | Issue | Files | Approach |
|---|-------|-------|----------|
| 7 | Missing directory exclusions | `classify.rs` | Expand `should_skip_directory_name` list; consider `.gitignore`-aware exclusions or `FilesystemRequest` config. |
| 8 | Hyperpolyglot unguarded fallback | `classify.rs` | Add extension whitelist before calling `hyperpolyglot::detect`; skip generic extensions (`.txt`, `.log`, `.tmp`). |
| 9 | Per-device `diskutil` subprocess | `storage.rs` | Batch device names into single `diskutil info` call, or parallelize with `rayon`, or switch to IOKit query. |
| 10 | Ref decoration recomputation | `git/detection.rs`, `git/types.rs` | Cache ref decorations on `GitRepo` struct; invalidate lazily or cache for `GitRepo` lifetime. |
| 11 | Sequential file inventory fallback | `classify.rs` | Use `build_parallel()` in `scan_file_inventory_with_exclusions` when `rayon` feature is enabled. |
| 12 | Manifest files read multiple times | `repo/detection.rs` | Add `HashMap<PathBuf, ParsedManifest>` cache inside `detect_repo_inner_with_shared`. |
| 13 | `refresh_package_boundaries` O(n²) | `repo/detection.rs` | Sort packages by path depth; only check descendants instead of all pairs. |
| 14 | Package manager PATH re-scan | `os/package_manager.rs` | Accept optional `&ExecutableIndex`; build once at top level and pass down. |
| 15 | Git worktree base repo reopen | `git/detection.rs` | Open base repo once before parallel section; share read-only state. |

**Deliverables:**
- [ ] Expanded directory exclusion list
- [ ] Hyperpolyglot extension guard
- [ ] Batched or parallelized `diskutil` calls
- [ ] Cached ref decorations on `GitRepo`
- [ ] Parallel inventory walker
- [ ] Manifest read cache
- [ ] O(n log n) package boundary refresh
- [ ] Shared `ExecutableIndex` for package managers
- [ ] Single base repo open for worktree detection

---

## Phase 4: Low-Effort Polish & Micro-Optimizations (Issues 16–23)

**Goal:** Clean up remaining inefficiencies with minimal risk.

| # | Issue | Files | Approach |
|---|-------|-------|----------|
| 16 | WAN IP thread + runtime spawn | `network/mod.rs` | Use `reqwest::blocking` or cache runtime/client in `OnceLock`. |
| 17 | `getifaddrs` not cached | `network/mod.rs` | Cache in `OnceLock` with 1-second TTL. |
| 18 | `canonicalize` per package | `repo/detection.rs` | Use path normalization without symlink resolution where safe. |
| 19 | NTP timeout polling | `os/time.rs` | Combine `timedatectl` calls; reduce timeout to 3s. |
| 20 | `Instant::now` overhead | `classify.rs` | Gate micro-recording behind `metrics` feature or sample every 100th entry. |
| 21 | `filter_inventory` cloning | `filesystem/mod.rs` | Use `Arc<[FileClassification]>` or `Cow` to avoid clones. |
| 22 | `determine_shared_walk_root` git discovery | `filesystem/mod.rs` | Skip `git2::Repository::discover()` when `request.git.is_none()`. |
| 23 | `get_path_dirs` PATH scan | `os/` | Cache result in `OnceLock<Vec<PathBuf>>`. |

**Deliverables:**
- [ ] WAN IP uses blocking client or cached runtime
- [ ] Interface list cached with short TTL
- [ ] Reduced `canonicalize` usage
- [ ] Combined NTP `timedatectl` call
- [ ] Sampled or feature-gated `Instant::now` recording
- [ ] Zero-copy or reference-based inventory filtering
- [ ] Conditional git discovery in shared walk root
- [ ] Cached PATH directory list

---

## Cross-Cutting Concerns

### Testing Strategy
- Run `cargo bench -p sniff` before and after each phase to measure impact.
- Add targeted Criterion benchmarks for:
  - 10k-file walk with/without mutex contention
  - 200-package monorepo dependency resolution
  - Windows program detection with cached index

### Risk Mitigation
- **Issue 5 (macOS audio/GPU):** Requires manual testing on macOS hardware before merging.
- **Issue 6 (redundant walks):** May change behavior for callers relying on independent scans; verify with existing tests.
- **Issue 1 (thread-local collector):** Ensure `snapshot()` remains deterministic (sort merged results if needed).

### Dependencies
- Phase 1 must complete before Phase 2 benchmarks are meaningful (mutex contention skews all measurements).
- Issue 14 (package manager index) depends on `ExecutableIndex` pattern established in Phase 1.

---

*Plan generated from 2026-04-21 performance review. Update this file as issues are resolved or reprioritized.*

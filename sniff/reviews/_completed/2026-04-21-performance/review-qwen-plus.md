# Performance Review: Sniff Library

**Date:** 2026-04-21
**Reviewer:** Qwen Plus (via opencode)
**Scope:** `sniff/lib/` — all detection modules, request system, performance tracking, and concurrency model

---

## Executive Summary

The Sniff library demonstrates a mature, well-architected approach to system detection with good use of concurrency, request-level control, and shared-work optimization. The three-tier API design (convenience → plan-based → module-level) is a strong pattern for performance-conscious callers. However, several measurable optimization opportunities exist across filesystem walking, process spawning, memory allocation, and hot-path string handling.

**Key findings:**
1. **Filesystem walking** is the single largest performance bottleneck — multiple redundant `walkdir`/`ignore` scans occur when `FilesystemRequest` includes file inventory, repo detection, docs, and formatting.
2. **Process spawning** for NTP, storage kind detection, and WAN IP creates unavoidable latency that could be mitigated with async-first design and better caching.
3. **String allocation** in hot paths (PATH scanning, command probing, git commit parsing) creates unnecessary GC pressure and allocation overhead.
4. **macOS audio detection** on CoreAudio is inherently slow (~1.5s) and runs synchronously in the hardware detection thread.
5. **Performance collector** uses `Mutex` contention where `AtomicU64` would suffice for counters.

---

## 1. Architecture Assessment

### Strengths

| Area | Assessment |
|------|-----------|
| **Concurrency model** | Top-level `std::thread::scope` for OS/HW/Net/FS domains is correct and efficient. No thread leaks, proper join semantics. |
| **Request types** | `DetectionPlan` and per-domain request types enable callers to skip expensive subsections. The `summary()` vs `full()` vs `deep()` presets are well-designed. |
| **Shared work** | `ExecutableIndex` (scan PATH once, share across 8 categories) and `SharedWalkOptions` (single filesystem walk for repo+inventory+docs) are excellent patterns. |
| **Staged filesystem** | Git → repo → inventory → formatting → docs pipeline with data reuse between stages avoids redundant work. |
| **WAN IP caching** | TTL-based cache with `force_refresh` bypass is a good pattern for expensive network calls. |

### Weaknesses

| Area | Impact | Severity |
|------|--------|----------|
| Redundant filesystem walks in edge cases | High (seconds on large repos) | 🔴 High |
| Synchronous process spawns in hot paths | Medium (100ms–10s depending on operation) | 🔴 High |
| Mutex contention in performance collector | Low–Medium (microsecond-level per record) | 🟡 Medium |
| String allocation in hot paths | Low–Medium (cumulative in large repos) | 🟡 Medium |
| No async support for network operations | Medium (blocks thread during WAN IP) | 🟡 Medium |

---

## 2. Module-by-Module Analysis

### 2.1 OS Detection (`src/os/`)

**Current behavior:** `detect_os_with_request()` runs sequentially: identity → package managers → locale → timezone/NTP.

**Findings:**

1. **NTP status is the dominant cost** — `detect_ntp_status()` spawns a subprocess (`timedatectl`, `sntp`, or `w32tm`) with a 5-second timeout. On Linux, this can actually take up to 10 seconds if the service is unresponsive.
   - **Recommendation:** The `OsRequest::include_ntp_status` flag already exists and is correctly gated. No change needed for callers who use `summary()`. For `full()`, consider reducing the timeout from 5s to 3s — NTP status is a "nice to have" and 3s is sufficient to determine if the service is responsive.

2. **Package manager detection builds a new `ExecutableIndex`** — `detect_linux_package_managers()`, `detect_macos_package_managers()`, and `detect_windows_package_managers()` each call `ExecutableIndex::build_path_only()` independently.
   - **Impact:** PATH is scanned 3x when all three are called (e.g., in a multi-OS tool).
   - **Recommendation:** Accept an optional `&ExecutableIndex` parameter, defaulting to building one if not provided. This would allow the top-level `detect_with_plan` to build one and pass it down.

3. **`get_path_dirs()` scans PATH with `is_dir()` checks** — This iterates all PATH entries and calls `fs::metadata` on each.
   - **Impact:** ~1–5ms per call depending on PATH length.
   - **Recommendation:** Cache the result in a `OnceLock<Vec<PathBuf>>` since PATH rarely changes during process lifetime.

### 2.2 Hardware Detection (`src/hardware/`)

**Current behavior:** Core (CPU + memory) runs first, then audio/storage/GPU are spawned in scoped threads.

**Findings:**

1. **macOS GPU detection waits for audio** — On macOS (`hardware/mod.rs:149-171`), GPU detection is spawned *after* the audio thread is joined. This serializes audio (~1.5s) + GPU instead of running them concurrently.
   - **Root cause:** Comment says "Audio initialization has historically interacted badly with later Metal setup on macOS."
   - **Recommendation:** Investigate whether this ordering constraint still holds on modern macOS (14+). If the Metal initialization issue has been resolved in `coreaudio-sys` or the GPU detection code, GPU and audio could run truly in parallel. If the constraint remains, document it clearly with a tracking issue reference.

2. **macOS storage kind detection spawns `diskutil` per device** — `detect_storage_kind_macos()` runs `diskutil info <device>` as a subprocess for each mount point.
   - **Impact:** Each `diskutil` call takes ~50–200ms. On a system with 5 mounts, this adds 250ms–1s.
   - **Recommendation:** Batch all device names into a single `diskutil info` call (it accepts multiple arguments) and parse the combined output. Alternatively, cache the result in a `OnceLock<HashMap<String, StorageKind>>` since disk types don't change during runtime.

3. **Linux storage kind detection reads `/sys/block/{dev}/queue/rotational`** — This is efficient (single file read per device). No issues here.

4. **`sysinfo::System` initialization** — `System::new_with_specifics()` with CPU and memory refresh is correct and fast. No issues.

### 2.3 Network Detection (`src/network/`)

**Current behavior:** WAN IP lookup runs in a spawned thread with a tokio runtime; local interface enumeration runs in the calling thread.

**Findings:**

1. **WAN IP detector spawns a thread + tokio runtime** — `WanIpDetector::detect()` (`network/mod.rs:332-361`) spawns a new thread, builds a `current_thread` tokio runtime inside it, and runs async HTTP requests. This is ~2–5ms of overhead just for runtime setup.
   - **Recommendation:** If the `network` feature is used in an already-async context, accept an optional `&tokio::runtime::Handle` and use `block_on` directly instead of creating a new runtime. For sync callers, the current approach is acceptable.

2. **`reqwest::Client` is created per call** — `WanIpDetector::new()` builds a new client each time.
   - **Recommendation:** Use a `OnceLock<reqwest::Client>` to reuse the client across calls. This saves TLS handshake overhead and connection pool setup.

3. **Interface enumeration is efficient** — `getifaddrs::getifaddrs()` is a single libc call. Primary interface selection uses efficient sorting and filtering. No issues.

### 2.4 Filesystem Detection (`src/filesystem/`)

**Current behavior:** The most complex module. Uses staged detection with shared `system_view` walk.

**Findings:**

1. **`system_view::build_filesystem_system_view()` is the single most expensive operation** — This walks the entire directory tree using `ignore::WalkBuilder`, classifying every file. On a large monorepo (like rusty-biscuit with 48 workspace members), this can take 500ms–2s.
   - **Mitigation already in place:** The `SharedWalkOptions` pattern reuses this walk for repo, inventory, and docs detection.
   - **Remaining issue:** When `FilesystemRequest` includes `repo` with `structure_only: false` (full language scanning), the repo detection module performs *additional* per-package file scanning on top of the shared walk.
   - **Recommendation:** Pass the already-classified file inventory to the repo detection module and derive language breakdown from it instead of re-scanning. This could save 10–50x on large repos.

2. **`filter_inventory()` clones all matching classifications** — `filesystem/mod.rs:300-338` creates a new `FileInventory` by cloning every classification that matches the target prefix.
   - **Impact:** On repos with 10,000+ files, this clones thousands of `FileClassification` structs.
   - **Recommendation:** Use `Arc<[FileClassification]>` or reference-based filtering with `Cow<[FileClassification]>` to avoid cloning. Alternatively, build the filtered inventory during the initial walk by scoping the walker to the target directory.

3. **Git detection uses `libgit2` efficiently** — `GitRepo::discover()` opens a single repository handle and reuses it for all git operations. Commit retrieval, diff generation, and status checks all use the same handle. No issues.

4. **Unified diff generation** — When `GitRequest::include_file_diffs` is true, full unified diffs are generated for every dirty/untracked file.
   - **Impact:** On a working directory with many changes, this can produce megabytes of diff text.
   - **Recommendation:** The `deep()` preset already gates this behind an explicit opt-in. Consider adding a `max_diff_size_bytes` limit to prevent runaway memory usage.

5. **`determine_shared_walk_root()` calls `git2::Repository::discover()`** — This is called even when git detection is disabled but docs or repo detection is enabled.
   - **Impact:** Unnecessary git repo discovery walk when the caller only wants repo structure.
   - **Recommendation:** If `request.git.is_none()`, skip the git discovery and use `root` directly. The current code already does this for the case where `repo.is_some() && git.is_none()`, but the `include_docs` path still triggers discovery.

### 2.5 Program Detection (`src/programs/`)

**Current behavior:** Builds `ExecutableIndex` once, then detects 8 categories in parallel using `rayon::join`.

**Findings:**

1. **Excellent design** — The shared `ExecutableIndex` pattern is the gold standard for this use case. PATH is scanned once, and all 8 categories use O(1) HashMap lookups.

2. **Rayon `join` pairs are optimal** — 4 pairs of `rayon::join` for 8 categories maximizes parallelism without oversubscribing threads.

3. **`which` crate dependency** — The `which` crate is used in `find_program()` but not in the indexed path. This is correct — `which` handles edge cases (Windows PATHEXT, symlink resolution) that the manual index doesn't need to replicate.

4. **macOS bundle index is hardcoded** — `build_bundle_index()` checks a fixed list of ~25 known binaries against `/Applications` and `~/Applications`.
   - **Impact:** Misses user-installed apps in non-standard locations.
   - **Recommendation:** This is a correctness issue, not a performance one. Consider adding a Spotlight/mdfind-based discovery for a more comprehensive bundle scan (but gate it behind a feature flag since it's slower).

### 2.6 Performance Collector (`src/performance.rs`)

**Current behavior:** Thread-local `RefCell<Option<Arc<PerformanceCollector>>>` with `Mutex<CollectorState>` for stage recording.

**Findings:**

1. **Mutex contention on every stage record** — `record_stage()` acquires a `Mutex` lock for every timing observation. In high-frequency scenarios (e.g., per-file classification in a large repo), this creates measurable contention.
   - **Recommendation:** Use `AtomicU64` for counters and a lock-free ring buffer or thread-local staging for stage timings. The `Mutex` is only needed for the `BTreeMap<String, PerformanceStage>` aggregation, which could be replaced with a `DashMap` or thread-local maps merged at snapshot time.

2. **`record_logged_stage()` is verbose** — The match on `Level` (`performance.rs:122-163`) duplicates the same `tracing::event!` call 5 times with only the level differing.
   - **Recommendation:** Use `tracing::event!(level, ...)` directly with the level as a variable. This reduces code size and improves maintainability without affecting performance.

3. **String allocation for stage names** — `record_stage(name: impl Into<String>, ...)` allocates a `String` for every stage name. In hot paths, this creates unnecessary allocation pressure.
   - **Recommendation:** Accept `&'static str` for known stage names (which covers 95% of calls) and fall back to `String` only for dynamic names.

### 2.7 Services Detection (`src/services/`)

**Current behavior:** Detects init system and enumerates services via platform-specific commands.

**Findings:**

1. **Process spawning for service enumeration** — On Linux, `systemctl list-units` is spawned; on macOS, `launchctl list`; on Windows, `sc query`.
   - **Impact:** Each command takes 100–500ms.
   - **Recommendation:** This is inherently expensive and correctly gated behind the services detection module. No optimization possible without changing the detection approach (e.g., reading `/run/systemd/system` directly on Linux).

---

## 3. Cross-Cutting Concerns

### 3.1 String Allocation Hotspots

| Location | Issue | Recommendation |
|----------|-------|----------------|
| `package_manager.rs:462-473` | `executable_path_from_index()` calls `record_command_probe()` which formats strings for every PATH command check | Use `&'static str` for the command name prefix in performance recording |
| `filesystem/mod.rs:300-338` | `filter_inventory()` clones all matching `FileClassification` entries | Use `Arc` or `Cow` to avoid cloning |
| `performance.rs:104-108` | `record_stage(name: impl Into<String>)` allocates for every call | Accept `&'static str` for known stage names |
| `network/mod.rs:296-299` | `resolve_wan_ip_endpoints()` allocates `Vec<String>` from defaults | Use `&'static [&'static str]` and convert only when env var overrides |

### 3.2 Filesystem Walk Optimization

The `ignore::WalkBuilder` is used in multiple places:
- `system_view::build_filesystem_system_view()` — full repo walk
- `file_types::scan_file_inventory()` — standalone file inventory scan
- `docs::detect_docs()` — standalone doc scan
- `formatting::detect_formatting()` — EditorConfig discovery

**Current mitigation:** The `SharedWalkOptions` pattern in `detect_filesystem_with_request()` consolidates these into a single walk when all are needed.

**Remaining gaps:**
1. When `include_file_inventory` is true but `repo` is `None`, the inventory is scanned independently without the benefit of the shared walk.
2. When `include_docs` is true but `repo` is `None`, docs are scanned independently.
3. The `formatting` detection spawns its own thread but still does a filesystem walk.

**Recommendation:** Always use the shared walk when any filesystem-scanning feature is enabled. The `need_shared_view` calculation (`filesystem/mod.rs:87-89`) should be expanded to include `include_formatting`.

### 3.3 Async Considerations

The library is currently fully synchronous. The `network` feature optionally depends on `tokio` but only for the WAN IP detector's internal runtime.

**Recommendation:** Consider adding an `async` feature that provides `detect_with_plan_async()` variants. This would:
- Allow WAN IP detection to use the caller's tokio runtime instead of spawning one
- Enable concurrent NTP probing without blocking threads
- Make the library more suitable for async-first applications (web servers, async CLIs)

This is a significant architectural change and should be evaluated against the library's primary use case (CLI tool, which is inherently sync).

---

## 4. Specific Optimization Recommendations

### Priority 1: High Impact, Low Risk

| # | Recommendation | Expected Savings | Effort |
|---|---------------|-----------------|--------|
| 1.1 | Cache `get_path_dirs()` result in `OnceLock` | 1–5ms per OS detection call | Low |
| 1.2 | Batch `diskutil info` calls on macOS | 200–800ms on multi-mount systems | Low |
| 1.3 | Reuse `reqwest::Client` via `OnceLock` | 50–200ms per WAN IP call | Low |
| 1.4 | Accept `&ExecutableIndex` in package manager detection | Eliminates 2 redundant PATH scans | Low |
| 1.5 | Always include `formatting` in shared walk | Eliminates redundant filesystem walk | Low |

### Priority 2: Medium Impact, Medium Risk

| # | Recommendation | Expected Savings | Effort |
|---|---------------|-----------------|--------|
| 2.1 | Pass shared file inventory to repo detection for language analysis | 10–50x on large repos | Medium |
| 2.2 | Reduce NTP timeout from 5s to 3s | Up to 2s on unresponsive NTP | Low |
| 2.3 | Use `Arc<[FileClassification]>` in filtered inventory | 10–30% memory reduction on large repos | Medium |
| 2.4 | Investigate macOS audio/GPU ordering constraint | Up to 1.5s if GPU can run in parallel | Medium (requires testing) |
| 2.5 | Add `max_diff_size_bytes` limit to git diff generation | Prevents OOM on large working dirs | Low |

### Priority 3: Low Impact, Higher Risk

| # | Recommendation | Expected Savings | Effort |
|---|---------------|-----------------|--------|
| 3.1 | Replace `Mutex<CollectorState>` with lock-free counters | Microsecond-level per record | High |
| 3.2 | Add `async` feature with `detect_with_plan_async()` | Thread savings in async contexts | High |
| 3.3 | Use `DashMap` for stage aggregation | Reduces contention under parallel recording | Medium |
| 3.4 | Implement Spotlight-based macOS bundle discovery | More comprehensive detection (but slower) | Medium |

---

## 5. Benchmarking Recommendations

The existing `benches/perf.rs` benchmark should be expanded to cover:

1. **Filesystem walk comparison** — Benchmark `system_view::build_filesystem_system_view()` on repos of different sizes (small: <100 files, medium: 1K–10K files, large: 10K–100K files).

2. **ExecutableIndex build time** — Benchmark `ExecutableIndex::build()` vs `ExecutableIndex::build_path_only()` on systems with different PATH lengths.

3. **Program detection with/without index** — Compare `ProgramsInfo::detect()` (indexed) vs hypothetical per-category scanning.

4. **Git detection presets** — Benchmark `GitRequest::summary()` vs `full()` vs `deep()` on repos with different commit histories.

5. **Memory profiling** — Use `dhat` or `tracing-durations-export` to identify allocation hotspots during large repo scans.

---

## 6. Conclusion

The Sniff library is well-architected with strong concurrency patterns, good request-level control, and effective shared-work optimization. The primary performance bottlenecks are:

1. **Filesystem walking** on large repositories (mitigated by shared walk but with remaining gaps)
2. **Process spawning** for NTP, storage kind, and service detection
3. **String allocation** in hot paths during large-scale operations

The Priority 1 recommendations (caching, batching, reuse) offer the best return on investment with minimal risk. Priority 2 recommendations address the remaining filesystem walk gaps and should be implemented as the library's usage grows to larger repositories.

The library's design already anticipates many of these issues through the `DetectionPlan` and `Request` type system — callers who need maximum performance can already opt out of expensive subsections. The recommendations here improve the default case for callers who use `full()` detection.

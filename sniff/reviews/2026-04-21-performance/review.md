# Sniff Library Performance Review

**Date:** 2026-04-21
**Reviewers:** Kimi (AI Agent), Qwen Plus (via opencode)
**Scope:** `sniff/lib` — cross-platform system detection library
**Methodology:** Static code analysis of hot paths, concurrency patterns, algorithmic complexity, and I/O amplification. No runtime profiling was performed; recommendations are based on code structure and known performance characteristics of the dependencies and OS APIs involved.

---

## Executive Summary

The Sniff library demonstrates a **mature, well-architected approach** to system detection. The four top-level domains (OS, hardware, network, filesystem) run in parallel via `std::thread::scope`, the programs module uses Rayon for category-level parallelism, and the `DetectionPlan` / `Request` type system provides excellent caller control over detection depth. The `ExecutableIndex` shared across program categories and `SharedWalkOptions` for consolidated filesystem walks are exemplary patterns.

However, there are **numerous medium-to-high impact inefficiencies** at the next level of detail. Estimated aggregate impact: **full detection on a large monorepo could be 20–50% slower than necessary**, with larger wins available for repeated calls (caching) and Windows program detection.

**Key findings:**
1. **Mutex contention in the performance hot path** — every classified file triggers a mutex lock.
2. **Algorithmic inefficiency in package graph resolution** — O(n²) nested loops with `Vec::contains` in monorepo detection.
3. **Repeated I/O and parsing** — HEAD tree resolution per dirty file, manifest re-reading, and per-device `diskutil` subprocesses.
4. **Platform-specific regressions** — Windows index rebuild on every lookup, audio-before-GPU serialization on macOS.
5. **Missing directory exclusions** — the file walker visits many cache/build directories that could be skipped.

---

## Severity Legend

| Severity | Meaning |
|----------|---------|
| **Critical** | Causes severe slowdowns (>1s) or pathological scaling; fix immediately |
| **High** | Clear performance bug or significant overhead; fix in next sprint |
| **Medium** | Noticeable overhead at scale; fix when convenient |
| **Low** | Micro-optimization or defensive improvement; backlog |

---

## Critical Issues

### 1. Performance Collector Mutex Contention

**Location:** `src/performance.rs:66-86`, `src/filesystem/file_types/classify.rs:259-260`

**Problem:** `PerformanceCollector` uses a `Mutex<BTreeMap>` for stage recording. Every file classification calls `record_stage`, which locks the mutex, mutates the map, and unlocks. On a 10,000-file repository with the parallel walker, this becomes a **global serialization point** across all worker threads.

**Code:**

```rust
fn record_stage(&self, name: &str, duration: Duration) {
    let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let stage = state.stages.entry(name.to_string()).or_default();
    // ... mutate
}
```

In `classify_file`, this is called unconditionally for *every* file:

```rust
performance::record_stage(classification_stage_name(&classification), duration);
```

**Impact:** On large repos, worker threads spend measurable time contending on this mutex instead of doing useful work.

**Recommendation:** Use **thread-local aggregation** with a merge step at the end. Each thread accumulates into its own `HashMap`, and `snapshot()` merges them. This is a well-established pattern (e.g., used by `metrics-rs` exporters). If maintaining `BTreeMap` ordering is required, sort only at snapshot time. Additionally, accept `&'static str` for known stage names (which covers 95% of calls) to eliminate per-call string allocation.

**Estimated win:** 10–30% faster file inventory on multi-core machines with large repos.

---

### 2. Windows Index Rebuilt on Every Program Lookup

**Location:** `src/programs/find_program.rs:443-472`

**Problem:** `find_program_with_source` rebuilds the Windows app index **from scratch on every call**:

```rust
#[cfg(target_os = "windows")]
{
    let idx = super::windows_apps::build_windows_index(); // expensive!
    if let Some(path) = idx.app_paths.get(&key) { ... }
}
```

`find_programs_with_source_parallel` calls `find_program_with_source` per program, so for 20 programs, the index is built 20 times. This involves registry enumeration and filesystem walks.

**Recommendation:** Either:
- Use `ExecutableIndex` (which is already built once and shared) for all Windows lookups too, or
- Cache the Windows index in a `OnceLock` or thread-local if `ExecutableIndex` cannot be used directly.

**Estimated win:** On Windows, program detection could go from seconds to milliseconds.

---

## High Issues

### 3. O(n²) Package Graph Resolution

**Location:** `src/filesystem/repo/detection.rs:1244-1289`

**Problem:** `resolve_internal_deps` uses `Vec::contains` inside nested loops:

```rust
let package_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();

for pkg in packages.iter_mut() {
    for dep_list in [...] {
        for dep in dep_list {
            if package_names.contains(&dep.name)      // O(n)
                && !internal_deps.contains(&dep.name) // O(n)
            {
                internal_deps.push(dep.name.clone());
            }
        }
    }
}
```

For a monorepo with 100 packages, this is ~10,000 string comparisons. For 500 packages, ~250,000.

**Recommendation:** Build a `HashSet<&str>` of package names once, and use it for O(1) lookups. The second `contains` check can also use a `HashSet` per package during construction.

**Estimated win:** Near-linear scaling for monorepo dependency resolution. On a 200-package repo, could save 50–100ms.

---

### 4. Repeated HEAD Tree Resolution in Git Status

**Location:** `src/filesystem/git/detection.rs:430-457`

**Problem:** `get_file_diff_stats` resolves the HEAD tree independently for **every dirty file**:

```rust
fn get_file_diff_stats(repo: &Repository, filepath: &Path) -> (usize, usize) {
    // Staged changes
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        // ... diff this file
    }
    // Unstaged changes
    // ... diff this file
}
```

When 50 files are dirty, HEAD is peeled 50 times. `peel_to_tree()` is not free — it walks refs and resolves objects.

**Recommendation:** Resolve HEAD tree **once** in `get_repo_status_with_changes` and pass it (or a cheap handle/reference) into `get_file_diff_stats`. The same applies to the index-to-workdir diff base.

**Estimated win:** 5–20ms per dirty file batch, depending on repo size.

---

### 5. macOS Audio Serialization Blocks GPU

**Location:** `src/hardware/mod.rs:112-171`

**Problem:** On macOS, audio and storage run in parallel, but GPU is **explicitly sequenced after audio**:

```rust
#[cfg(target_os = "macos")]
let gpu = {
    let audio_devices = audio_handle.map(|handle| handle.join().unwrap()).unwrap_or_default();
    // ... only THEN detect GPU
};
```

The comment says this is because "audio initialization has historically interacted badly with later Metal setup." However, the GPU detection was changed to use **IOKit FFI** (`detect_gpus_iokit`), not Metal framework initialization. The IOKit path (~200µs) should not conflict with CoreAudio.

**Recommendation:** Re-test whether the audio-first ordering is still necessary now that GPU detection uses IOKit instead of Metal. If safe, run all three (audio, storage, GPU) in parallel on macOS like other platforms.

**Estimated win:** ~1.5s reduction in `HardwareRequest::full()` on macOS (audio no longer blocks GPU).

---

### 6. Redundant Filesystem Walks in Edge Cases

**Location:** `src/filesystem/mod.rs:87-89`, `system_view.rs`, `file_types/classify.rs`, `docs.rs`, `formatting.rs`

**Problem:** The `SharedWalkOptions` pattern consolidates repo, inventory, and docs into a single walk when all are needed, but gaps remain:
1. When `include_file_inventory` is true but `repo` is `None`, the inventory is scanned independently.
2. When `include_docs` is true but `repo` is `None`, docs are scanned independently.
3. The `formatting` detection spawns its own thread but still does a separate filesystem walk.
4. When `repo` with `structure_only: false` (full language scanning) is requested, the repo detection module performs *additional* per-package file scanning on top of the shared walk.

**Recommendation:** 
- Expand the `need_shared_view` calculation to include `include_formatting`.
- Pass the already-classified file inventory to the repo detection module and derive language breakdown from it instead of re-scanning.
- Always use the shared walk when any filesystem-scanning feature is enabled.

**Estimated win:** Eliminates redundant walks; 10–50x on large repos for language analysis.

---

## Medium Issues

### 7. Missing Directory Exclusions in File Walker

**Location:** `src/filesystem/file_types/classify.rs:102-107`

**Problem:** `should_skip_directory_name` only skips 8 directories:

```rust
matches!(name,
    ".git" | ".turbo" | "node_modules" | "target" | "vendor"
    | "dist" | "build" | "__pycache__"
)
```

Many common cache/build directories are traversed unnecessarily:
- `.venv`, `venv`, `.env` (Python virtual environments)
- `.pytest_cache`, `.mypy_cache`, `.tox`, `.ruff_cache`
- `.next`, `.nuxt`, `.output`, `.vercel`, `.parcel-cache`
- `.cache`, `out`, `bin`, `obj` (generic build outputs)
- `.idea`, `.vscode` (IDE metadata)
- `coverage`, `htmlcov`
- `.svelte-kit`, `.astro`

**Impact:** On a typical JS/Python project, 20–40% of files walked may be inside these directories.

**Recommendation:** Expand the exclusion list significantly. Consider making it configurable via `FilesystemRequest` or reading from `.gitignore` (the walker already respects `.gitignore`, but these directories are often *not* gitignored if they live outside the repo root, or if the scan root is a parent directory).

**Estimated win:** 10–30% fewer files to classify in mixed-language projects.

---

### 8. Hyperpolyglot Called Without Extension-Based Guard

**Location:** `src/filesystem/file_types/classify.rs:184-232`

**Problem:** When a file has no recognized extension, exact filename, or binary signature, the code reads the first 8KB and calls `hyperpolyglot::detect(path)`. This is a **heavyweight analysis** that may spawn subprocesses or do complex content inspection.

```rust
let header = read_prefix(path, READ_LIMIT);
if let Some(ref bytes) = header {
    if is_probably_text(bytes) {
        let detection = hyperpolyglot::detect(path); // expensive!
        // ...
    }
}
```

There is **no extension whitelist** before calling hyperpolyglot. For example, a `.log` file or a `.tmp` file will be fully analyzed.

**Recommendation:** Add a lightweight extension whitelist for files worth hyperpolyglot analysis (e.g., known ambiguous extensions like `.h`, `.inc`, `.m`, `.pl`). Skip generic extensions like `.txt`, `.log`, `.tmp`, `.bak`, `.out`, `.cache`.

**Estimated win:** Reduces hyperpolyglot invocations by 30–60% in typical repos.

---

### 9. Per-Device `diskutil` Subprocess on macOS Storage

**Location:** `src/hardware/storage.rs:115-142`

**Problem:** `detect_storage_kind_macos` spawns `diskutil info <dev>` for **every storage device**:

```rust
fn detect_storage_kind_macos(device: &str) -> StorageKind {
    let output = match std::process::Command::new("diskutil")
        .args(["info", dev_name])
        .output() { ... }
    // ...
}
```

On a typical Mac, there may be 5–15 mount entries. Each `diskutil` spawn takes 10–50ms.

**Recommendation:** Batch all device names into a single `diskutil info` call (it accepts multiple arguments) and parse the combined output. Alternatively, parallelize with `rayon` or `std::thread::scope`, or switch to IOKit/registry queries for the "Solid State" property (similar to the GPU IOKit optimization).

**Estimated win:** 50–200ms faster storage detection on macOS.

---

### 10. Ref Decorations Recomputed for Every Commit Query

**Location:** `src/filesystem/git/detection.rs:33-113`, `src/filesystem/git/types.rs:564-565`

**Problem:** `collect_ref_decorations` iterates **all refs** and peels each to a commit. This is called:
- In `get_recent_commits` (for every `detect_git` call)
- In `get_commit_by_sha`
- In `get_commits_for_path`

For repos with many tags (e.g., 1,000+), this is expensive and repeated.

**Recommendation:** Cache ref decorations on the `GitRepo` struct (which already holds the `Repository` handle). Invalidate lazily or cache for the lifetime of the `GitRepo`.

**Estimated win:** 10–100ms per git detection call on repos with many tags.

---

### 11. `scan_file_inventory` Uses Sequential Walker

**Location:** `src/filesystem/file_types/classify.rs:42-51`

**Problem:** `scan_file_inventory_with_exclusions` uses `.build()` (sequential), while `system_view::build_filesystem_system_view` uses `.build_parallel()`:

```rust
let walker = WalkBuilder::new(root)
    // ...
    .build(); // sequential!
```

When `detect_repo_inner_with_shared` falls back to per-package scanning (line 207, 219), it uses the sequential path.

**Recommendation:** Make `scan_file_inventory_with_exclusions` use the parallel walker when the `rayon` feature is available, or at least provide a parallel variant. The shared view already demonstrates the pattern with `WorkerBuffers` and `Drop`-based merging.

**Estimated win:** 2–4x faster per-package inventory scans on multi-core machines.

---

### 12. Manifest Files Read Multiple Times During Repo Detection

**Location:** `src/filesystem/repo/detection.rs` (various `read_*` helpers)

**Problem:** Each manifest-reading helper (`read_cargo_package_name`, `read_npm_package_name`, `read_pyproject_package_name`, `read_go_module_name`) reads and parses the file independently. During `resolve_package_name` and `resolve_package_version`, the same files may be read 2–4 times.

**Recommendation:** In the monorepo detection flow, read each manifest once and extract all needed fields. A simple `HashMap<PathBuf, ParsedManifest>` cache within `detect_repo_inner_with_shared` would eliminate redundant I/O.

**Estimated win:** 20–50ms on monorepos with hundreds of packages, especially Node.js projects with many `package.json` files.

---

### 13. `refresh_package_boundaries` is O(n²) in Package Count

**Location:** `src/filesystem/repo/detection.rs:1564-1608`

**Problem:** For each package, this function iterates all other packages to find nested roots:

```rust
for (index, package) in packages.iter_mut().enumerate() {
    for (other_index, other_root) in package_roots.iter().enumerate() {
        if index == other_index { continue; }
        if other_root.starts_with(package_root) {
            nested_roots.push(...);
        }
    }
    // ... then scan languages
}
```

**Recommendation:** Pre-build a tree or prefix map of package paths so nested lookups are O(log n) or O(1). A simple approach: sort packages by path depth and only check descendants.

**Estimated win:** Significant for monorepos with >100 packages.

---

### 14. Package Manager Detection Builds New `ExecutableIndex`

**Location:** `src/os/package_manager.rs` (various `detect_*_package_managers`)

**Problem:** `detect_linux_package_managers()`, `detect_macos_package_managers()`, and `detect_windows_package_managers()` each call `ExecutableIndex::build_path_only()` independently. When all three are called (e.g., in a multi-OS tool), PATH is scanned 3x.

**Recommendation:** Accept an optional `&ExecutableIndex` parameter, defaulting to building one if not provided. This would allow the top-level `detect_with_plan` to build one and pass it down.

**Estimated win:** Eliminates 2 redundant PATH scans.

---

### 15. Git Worktree Detection Reopens Base Repo Per Task

**Location:** `src/filesystem/git/detection.rs:993-1089`

**Problem:** In `get_worktrees`, each parallel task reopens the base repository:

```rust
worktree_paths.par_iter().filter_map(|(name, worktree_path)| {
    let worktree_repo = Repository::open(worktree_path).ok()?;
    // ...
    let base_repo = Repository::open(&base_repo_path).ok(); // reopened N times
    // ...
}).collect()
```

Opening a git repository involves reading refs, configs, and object databases. Doing it N times for N worktrees is wasteful.

**Recommendation:** Open the base repo once before the parallel section and share read-only references, or use `git2::Repository::open` once and clone the handle (if `Repository` supports cheap cloning — it does not, so this may require restructuring). Alternatively, accept the cost since worktrees are typically few (<10).

---

## Low Issues

### 16. WAN IP Detector Spawns a Thread + Tokio Runtime Per Call

**Location:** `src/network/mod.rs:328-361`

**Problem:** `WanIpDetector::detect` spawns a new thread and builds a new Tokio runtime on every call:

```rust
std::thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async move { ... })
}).join()
```

Additionally, `reqwest::Client` is created per call.

**Recommendation:** Use `reqwest::blocking` instead, or cache the runtime/client in a `OnceLock`. If the `network` feature is used in an already-async context, accept an optional `&tokio::runtime::Handle` and use `block_on` directly instead of creating a new runtime.

**Estimated win:** 5–10ms per WAN IP lookup; 50–200ms from client reuse.

---

### 17. `getifaddrs` Results Not Cached for Repeated Calls

**Location:** `src/network/mod.rs:215-271`

**Problem:** `detect_local_interfaces` calls `getifaddrs::getifaddrs()` and rebuilds the interface map from scratch every time. Network interfaces rarely change during a single process lifetime.

**Recommendation:** Cache the result in a `OnceLock` or `std::sync::OnceLock` with a short TTL (e.g., 1 second) for the duration of repeated `detect_network` calls.

**Estimated win:** 1–5ms per repeated network detection call.

---

### 18. `merge_packages` Uses `std::fs::canonicalize` Per Package

**Location:** `src/filesystem/repo/detection.rs:1309-1311`, `1468-1484`

**Problem:** `canonicalize_path` calls `std::fs::canonicalize` for every package during merging:

```rust
pub(crate) fn canonicalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
```

`canonicalize` resolves symlinks and requires filesystem access. In a 100-package monorepo, that's 100 syscalls.

**Recommendation:** If symlink resolution is not strictly necessary for deduplication, use `std::env::current_dir()` normalization or path comparison without canonicalization. If it is necessary, consider doing it only when paths might be ambiguous.

**Estimated win:** 5–20ms on large monorepos.

---

### 19. NTP Detection Timeout Polling Inefficiency

**Location:** `src/os/time.rs:71-108`

**Problem:** `run_command_with_timeout` busy-polls the child process with 10ms sleeps:

```rust
loop {
    match child.try_wait() { ... }
    std::thread::sleep(Duration::from_millis(10));
}
```

More importantly, on Linux `detect_ntp_status` calls `timedatectl` **twice** sequentially:
1. `timedatectl show --property=NTPSynchronized --value`
2. `timedatectl show --property=NTP --value`

**Recommendation:** Combine into a single `timedatectl show --property=NTPSynchronized,NTP --value` call, or use `wait_timeout` crates if available. Also consider reducing the timeout from 5s to 3s — NTP status is a "nice to have" and 3s is sufficient to determine if the service is responsive.

**Estimated win:** 5–20ms on Linux NTP detection.

---

### 20. `Instant::now()` Overhead in Tight Classification Loop

**Location:** `src/filesystem/file_types/classify.rs:62-68`, `109-110`

**Problem:** `Instant::now()` is called for every file during both scanning and classification. While cheap (~10–30ns), it adds up for 10,000 files and is amplified by the performance recording overhead.

```rust
let callback_started = Instant::now();
classifications.push(classify_file(root, entry.path()));
performance::record_stage("filesystem.file_inventory.walk.entry", callback_started.elapsed());
```

**Recommendation:** Gate performance micro-recording behind the `metrics` feature or a compile-time flag, or sample it (e.g., record every 100th entry). The high-level stage timers (`filesystem.file_inventory.scan`) already provide sufficient visibility.

**Estimated win:** Minimal for wall-clock time, but reduces code bloat and mutex contention from issue #1.

---

### 21. `filter_inventory()` Clones All Matching Classifications

**Location:** `src/filesystem/mod.rs:300-338`

**Problem:** `filter_inventory()` creates a new `FileInventory` by cloning every classification that matches the target prefix. On repos with 10,000+ files, this clones thousands of `FileClassification` structs.

**Recommendation:** Use `Arc<[FileClassification]>` or reference-based filtering with `Cow<[FileClassification]>` to avoid cloning. Alternatively, build the filtered inventory during the initial walk by scoping the walker to the target directory.

**Estimated win:** 10–30% memory reduction on large repos.

---

### 22. `determine_shared_walk_root()` Calls `git2::Repository::discover()` Unnecessarily

**Location:** `src/filesystem/mod.rs`

**Problem:** `determine_shared_walk_root()` calls `git2::Repository::discover()` even when git detection is disabled but docs or repo detection is enabled.

**Impact:** Unnecessary git repo discovery walk when the caller only wants repo structure.

**Recommendation:** If `request.git.is_none()`, skip the git discovery and use `root` directly. The current code already does this for the case where `repo.is_some() && git.is_none()`, but the `include_docs` path still triggers discovery.

---

### 23. `get_path_dirs()` Scans PATH with `is_dir()` Checks

**Location:** `src/os/` (PATH scanning)

**Problem:** `get_path_dirs()` iterates all PATH entries and calls `fs::metadata` on each. PATH rarely changes during process lifetime.

**Recommendation:** Cache the result in a `OnceLock<Vec<PathBuf>>`.

**Estimated win:** 1–5ms per OS detection call.

---

## Positive Performance Design Patterns (Keep These)

The following patterns are **exemplary** and should be preserved/extended:

1. **Top-level scoped thread parallelism** (`lib.rs:278-353`) — clean, safe, effective.
2. **ExecutableIndex shared across program categories** (`programs/mod.rs:200-240`) — eliminates redundant PATH scans.
3. **Shared filesystem walk with worker buffers** (`system_view.rs:67-144`) — `Drop`-based accumulation into a shared `Mutex` is a good pattern for parallel walkers.
4. **Request-based cost gating** (`request.rs`) — `GitRequest::summary()` vs `full()` vs `deep()` provides excellent caller control.
5. **WAN IP TTL caching** (`network/mod.rs:365-402`) — prevents repeated HTTP calls.
6. **macOS GPU IOKit FFI** (`hardware/gpu.rs:126-232`) — avoiding the 14+ second Metal framework initialization in non-GUI contexts is a critical optimization.
7. **Git2 handle reuse via `GitRepo`** (`filesystem/git/types.rs:475-506`) — opening the repo once and reusing it across queries is correct and efficient.
8. **Criterion benchmark suite** (`benches/`) — having dedicated benches for each domain enables regression detection.
9. **Rayon `join` pairs for program categories** — 4 pairs of `rayon::join` for 8 categories maximizes parallelism without oversubscribing threads.
10. **Staged filesystem pipeline** — Git → repo → inventory → formatting → docs with data reuse between stages avoids redundant work.

---

## Recommended Priority Order

| # | Issue | Severity | Effort | File(s) |
|---|-------|----------|--------|---------|
| 1 | Performance collector mutex contention | Critical | Medium | `performance.rs`, `classify.rs` |
| 2 | Windows index rebuilt per lookup | Critical | Low | `find_program.rs` |
| 3 | O(n²) package graph resolution | High | Low | `repo/detection.rs` |
| 4 | Repeated HEAD tree resolution | High | Medium | `git/detection.rs` |
| 5 | macOS audio serialization | High | Low | `hardware/mod.rs` |
| 6 | Redundant filesystem walks | High | Medium | `filesystem/mod.rs`, `system_view.rs` |
| 7 | Missing directory exclusions | Medium | Low | `classify.rs` |
| 8 | Hyperpolyglot unguarded fallback | Medium | Low | `classify.rs` |
| 9 | Per-device `diskutil` subprocess | Medium | Medium | `storage.rs` |
| 10 | Ref decoration recomputation | Medium | Medium | `git/detection.rs`, `git/types.rs` |
| 11 | Sequential file inventory fallback | Medium | Medium | `classify.rs` |
| 12 | Manifest re-reading | Medium | Medium | `repo/detection.rs` |
| 13 | `refresh_package_boundaries` O(n²) | Medium | Medium | `repo/detection.rs` |
| 14 | Package manager PATH re-scan | Medium | Low | `os/package_manager.rs` |
| 15 | Git worktree base repo reopen | Medium | Low | `git/detection.rs` |
| 16 | WAN IP thread+runtime spawn | Low | Low | `network/mod.rs` |
| 17 | `reqwest::Client` created per call | Low | Low | `network/mod.rs` |
| 18 | NTP timeout polling | Low | Low | `os/time.rs` |
| 19 | `canonicalize` per package | Low | Low | `repo/detection.rs` |
| 20 | Interface cache | Low | Low | `network/mod.rs` |
| 21 | `Instant::now` overhead | Low | Low | `classify.rs` |
| 22 | `filter_inventory` cloning | Low | Medium | `filesystem/mod.rs` |
| 23 | `get_path_dirs` PATH scan | Low | Low | `os/` |
| 24 | `determine_shared_walk_root` git discovery | Low | Low | `filesystem/mod.rs` |

---

## Appendix: Quick Wins (Copy-Pasteable)

### A. HashSet for package name lookups

In `resolve_internal_deps`:

```rust
// Before
let package_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
if package_names.contains(&dep.name) { ... }

// After
use std::collections::HashSet;
let package_names: HashSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
if package_names.contains(dep.name.as_str()) { ... }
```

### B. Skip more directories

In `should_skip_directory_name`:

```rust
matches!(name,
    ".git" | ".turbo" | "node_modules" | "target" | "vendor"
    | "dist" | "build" | "__pycache__"
    | ".venv" | "venv" | ".env"
    | ".pytest_cache" | ".mypy_cache" | ".tox" | ".ruff_cache"
    | ".next" | ".nuxt" | ".output" | ".vercel" | ".parcel-cache"
    | ".cache" | "out" | "bin" | "obj"
    | ".idea" | ".vscode"
    | "coverage" | "htmlcov"
    | ".svelte-kit" | ".astro"
)
```

### C. Cache Windows index

In `find_program_with_source`:

```rust
#[cfg(target_os = "windows")]
{
    use std::sync::OnceLock;
    static WINDOWS_INDEX: OnceLock<super::windows_apps::WindowsIndex> = OnceLock::new();
    let idx = WINDOWS_INDEX.get_or_init(super::windows_apps::build_windows_index);
    // ... use idx
}
```

### D. Cache `get_path_dirs()`

```rust
use std::sync::OnceLock;

static PATH_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();

fn get_path_dirs() -> &'static [PathBuf] {
    PATH_DIRS.get_or_init(|| {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|p| std::env::split_paths(&p))
            .filter(|p| p.is_dir())
            .collect()
    })
}
```

---

## Benchmarking Recommendations

The existing `benches/perf.rs` benchmark should be expanded to cover:

1. **Filesystem walk comparison** — Benchmark `system_view::build_filesystem_system_view()` on repos of different sizes (small: <100 files, medium: 1K–10K files, large: 10K–100K files).

2. **ExecutableIndex build time** — Benchmark `ExecutableIndex::build()` vs `ExecutableIndex::build_path_only()` on systems with different PATH lengths.

3. **Program detection with/without index** — Compare `ProgramsInfo::detect()` (indexed) vs hypothetical per-category scanning.

4. **Git detection presets** — Benchmark `GitRequest::summary()` vs `full()` vs `deep()` on repos with different commit histories.

5. **Memory profiling** — Use `dhat` or `tracing-durations-export` to identify allocation hotspots during large repo scans.

---

## Architectural Considerations

### Async Support

The library is currently fully synchronous. The `network` feature optionally depends on `tokio` but only for the WAN IP detector's internal runtime.

**Recommendation:** Consider adding an `async` feature that provides `detect_with_plan_async()` variants. This would:
- Allow WAN IP detection to use the caller's tokio runtime instead of spawning one
- Enable concurrent NTP probing without blocking threads
- Make the library more suitable for async-first applications (web servers, async CLIs)

This is a significant architectural change and should be evaluated against the library's primary use case (CLI tool, which is inherently sync).

### String Allocation Strategy

Several hot paths allocate strings unnecessarily:
- `record_stage(name: impl Into<String>)` allocates for every call
- `executable_path_from_index()` formats strings for every PATH command check
- `resolve_wan_ip_endpoints()` allocates `Vec<String>` from defaults

**Recommendation:** Adopt a convention of accepting `&'static str` for known constant names and only falling back to `String` for dynamic values. Use `&'static [&'static str]` for default endpoint lists.

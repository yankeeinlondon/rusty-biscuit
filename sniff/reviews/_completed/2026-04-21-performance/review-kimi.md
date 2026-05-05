# Sniff Library Performance Review

**Date:** 2026-04-21
**Reviewer:** Kimi (AI Agent)
**Scope:** `sniff/lib` — cross-platform system detection library
**Methodology:** Static code analysis of hot paths, concurrency patterns, algorithmic complexity, and I/O amplification. No runtime profiling was performed; recommendations are based on code structure and known performance characteristics of the dependencies and OS APIs involved.

---

## Executive Summary

The Sniff library has a **well-designed high-level concurrency model** — the four top-level domains (OS, hardware, network, filesystem) run in parallel via `std::thread::scope`, and the programs module uses Rayon for category-level parallelism. There is also good **request-tier granularity** that lets callers skip expensive work.

However, there are **numerous medium-to-high impact inefficiencies** at the next level of detail:

1. **Mutex contention in the performance hot path** — every classified file triggers a mutex lock.
2. **Algorithmic inefficiency in package graph resolution** — O(n²) nested loops with `Vec::contains` in monorepo detection.
3. **Repeated I/O and parsing** — HEAD tree resolution per dirty file, manifest re-reading, and per-device `diskutil` subprocesses.
4. **Platform-specific regressions** — Windows index rebuild on every lookup, audio-before-GPU serialization on macOS.
5. **Missing directory exclusions** — the file walker visits many cache/build directories that could be skipped.

Estimated aggregate impact: **full detection on a large monorepo could be 20–50% slower than necessary**, with larger wins available for repeated calls (caching) and Windows program detection.

---

## Severity Legend

| Severity | Meaning |
|----------|---------|
| **Critical** | Causes severe slowdowns (>1s) or pathological scaling; fix immediately |
| **High** | Clear performance bug or significant overhead; fix in next sprint |
| **Medium** | Noticeable overhead at scale; fix when convenient |
| **Low** | Micro-optimization or defensive improvement; backlog |

---

## 1. Performance Collector Mutex Contention (Critical)

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

**Recommendation:** Use **thread-local aggregation** with a merge step at the end. Each thread accumulates into its own `HashMap`, and `snapshot()` merges them. This is a well-established pattern (e.g., used by `metrics-rs` exporters). If maintaining `BTreeMap` ordering is required, sort only at snapshot time.

**Estimated win:** 10–30% faster file inventory on multi-core machines with large repos.

---

## 2. O(n²) Package Graph Resolution (High)

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

## 3. Repeated HEAD Tree Resolution in Git Status (High)

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

## 4. Windows Index Rebuilt on Every Program Lookup (Critical on Windows)

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

## 5. Missing Directory Exclusions in File Walker (Medium)

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

## 6. macOS Audio Serialization Blocks GPU (High on macOS)

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

## 7. Hyperpolyglot Called Without Extension-Based Guard (Medium)

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

## 8. Per-Device `diskutil` Subprocess on macOS Storage (Medium)

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

**Recommendation:** Parallelize the storage kind detection with `rayon` or `std::thread::scope`, or switch to IOKit/registry queries for the "Solid State" property (similar to the GPU IOKit optimization).

**Estimated win:** 50–200ms faster storage detection on macOS.

---

## 9. Ref Decorations Recomputed for Every Commit Query (Medium)

**Location:** `src/filesystem/git/detection.rs:33-113`, `src/filesystem/git/types.rs:564-565`

**Problem:** `collect_ref_decorations` iterates **all refs** and peels each to a commit. This is called:
- In `get_recent_commits` (for every `detect_git` call)
- In `get_commit_by_sha`
- In `get_commits_for_path`

For repos with many tags (e.g., 1,000+), this is expensive and repeated.

**Recommendation:** Cache ref decorations on the `GitRepo` struct (which already holds the `Repository` handle). Invalidate lazily or cache for the lifetime of the `GitRepo`.

**Estimated win:** 10–100ms per git detection call on repos with many tags.

---

## 10. `scan_file_inventory` Uses Sequential Walker (Medium)

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

## 11. Manifest Files Read Multiple Times During Repo Detection (Medium)

**Location:** `src/filesystem/repo/detection.rs` (various `read_*` helpers)

**Problem:** Each manifest-reading helper (`read_cargo_package_name`, `read_npm_package_name`, `read_pyproject_package_name`, `read_go_module_name`) reads and parses the file independently. During `resolve_package_name` and `resolve_package_version`, the same files may be read 2–4 times.

**Recommendation:** In the monorepo detection flow, read each manifest once and extract all needed fields. A simple `HashMap<PathBuf, ParsedManifest>` cache within `detect_repo_inner_with_shared` would eliminate redundant I/O.

**Estimated win:** 20–50ms on monorepos with hundreds of packages, especially Node.js projects with many `package.json` files.

---

## 12. `Instant::now()` Overhead in Tight Classification Loop (Low)

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

## 13. `refresh_package_boundaries` is O(n²) in Package Count (Medium)

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

## 14. WAN IP Detector Spawns a Thread + Tokio Runtime Per Call (Low)

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

This is fine for occasional calls, but if `detect_network` is called repeatedly (e.g., in a polling CLI), the thread+runtime creation overhead adds up.

**Recommendation:** Use `reqwest::blocking` instead, or cache the runtime/client in a `OnceLock`. The `reqwest` client itself is already wrapped in an `Option` but the runtime is rebuilt every time.

**Estimated win:** 5–10ms per WAN IP lookup.

---

## 15. `getifaddrs` Results Not Cached for Repeated Calls (Low)

**Location:** `src/network/mod.rs:215-271`

**Problem:** `detect_local_interfaces` calls `getifaddrs::getifaddrs()` and rebuilds the interface map from scratch every time. Network interfaces rarely change during a single process lifetime.

**Recommendation:** Cache the result in a `OnceLock` or `std::sync::OnceLock` with a short TTL (e.g., 1 second) for the duration of repeated `detect_network` calls.

**Estimated win:** 1–5ms per repeated network detection call.

---

## 16. `merge_packages` Uses `std::fs::canonicalize` Per Package (Low)

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

## 17. NTP Detection Timeout Polling Inefficiency (Low)

**Location:** `src/os/time.rs:71-108`

**Problem:** `run_command_with_timeout` busy-polls the child process with 10ms sleeps:

```rust
loop {
    match child.try_wait() { ... }
    std::thread::sleep(Duration::from_millis(10));
}
```

This is functional but slightly wasteful. More importantly, on Linux `detect_ntp_status` calls `timedatectl` **twice** sequentially:
1. `timedatectl show --property=NTPSynchronized --value`
2. `timedatectl show --property=NTP --value`

**Recommendation:** Combine into a single `timedatectl show --property=NTPSynchronized,NTP --value` call, or use `wait_timeout` crates if available. The 10ms polling is acceptable but not ideal.

**Estimated win:** 5–20ms on Linux NTP detection.

---

## 18. Git Worktree Detection Uses `rayon` but Reopens Base Repo Per Task (Medium)

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

---

## Recommended Priority Order

| # | Issue | Severity | Effort | File(s) |
|---|-------|----------|--------|---------|
| 1 | Performance collector mutex contention | Critical | Medium | `performance.rs`, `classify.rs` |
| 2 | O(n²) package graph resolution | High | Low | `repo/detection.rs` |
| 3 | Windows index rebuilt per lookup | Critical | Low | `find_program.rs` |
| 4 | Repeated HEAD tree resolution | High | Medium | `git/detection.rs` |
| 5 | macOS audio serialization | High | Low | `hardware/mod.rs` |
| 6 | Missing directory exclusions | Medium | Low | `classify.rs` |
| 7 | Hyperpolyglot unguarded fallback | Medium | Low | `classify.rs` |
| 8 | Per-device `diskutil` subprocess | Medium | Medium | `storage.rs` |
| 9 | Ref decoration recomputation | Medium | Medium | `git/detection.rs`, `git/types.rs` |
| 10 | Sequential file inventory fallback | Medium | Medium | `classify.rs` |
| 11 | Manifest re-reading | Medium | Medium | `repo/detection.rs` |
| 12 | WAN IP thread+runtime spawn | Low | Low | `network/mod.rs` |
| 13 | NTP timeout polling | Low | Low | `os/time.rs` |
| 14 | `canonicalize` per package | Low | Low | `repo/detection.rs` |
| 15 | Interface cache | Low | Low | `network/mod.rs` |
| 16 | Worktree base repo reopen | Low | Low | `git/detection.rs` |
| 17 | `Instant::now` overhead | Low | Low | `classify.rs` |

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

---

*Review generated by static analysis. Recommend validating fixes with the existing Criterion bench suite (`cargo bench -p sniff`).*

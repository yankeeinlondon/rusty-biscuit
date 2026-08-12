---
date: 2026-05-05
benchmark_run: sniff-perf-regression-2026-05-05
regressed_benchmarks: 4
slow_benchmarks: 9
suggestions: 15
suggestions_critical: 1
suggestions_important: 8
suggestions_nice_to_have: 6
---

# Performance Regression Review: sniff library benchmarks

**Date:** 2026-05-05
**Baseline comparison:** Previous benchmark run (pre-2026-05-02)
**Package area:** sniff

## Executive Summary

Four benchmarks regressed in the latest run. One regression (`languages_shallow_deep_mix`, +443%) is a **measurement artifact** from a bug fix — the benchmark now correctly measures actual work. The other three represent genuine optimization opportunities.

In addition, nine benchmarks have been identified as chronically slow (>10ms) and are analyzed below for optimization opportunities:

| Benchmark | Regression | Root Cause | Actionable? |
|-----------|-----------|------------|-------------|
| `filesystem_languages/languages_shallow_deep_mix` | +443.55% | Bug fix made benchmark honest | Re-baseline only |
| `filesystem_docs/detect_blast_radius_only` | +12.34% | Redundant `detect_repo()` call per invocation | Yes — cache packages |
| `filesystem_git/git_summary_small` | +12.80% | Summary requests still pay for branch/tracking traversal | Yes — trim summary scope |
| `hardware/detect_gpus_only` | +6.28% | Unnecessary `sysinfo::System` init when only GPU requested | Yes — defer core init |

### Chronically Slow Benchmarks (>10ms)

| Benchmark | Time | Dominant Cost | Actionable? |
|-----------|------|---------------|-------------|
| `programs_fanout/programs_info_detect` | ~24.7ms | Host PATH I/O + eager index rebuild | Yes — cache index |
| `inventory/programs_detect` | ~25.0ms | Same as above (identical code path) | Yes — cache index |
| `git_dirty_scaling/git_deep/100` | ~18.7ms | Unified diff allocation for 100 dirty files | Yes — pre-size + stream |
| `filesystem_staged/filesystem_summary_request` | ~16.5ms | Shared manifest walk + git summary overhead | Yes — skip unneeded walk |
| `filesystem_staged/filesystem_full_request` | ~29.0ms | Full parallel walk + repo enrichment + git full | Yes — combine optimizations |
| `filesystem_repo/repo_full_huge` | ~56.5ms | Single-threaded manifest walk + per-file path normalization | Yes — parallelize + reduce syscalls |
| `filesystem_repo/repo_with_inventory_monorepo` | ~15.8ms | Double directory walk (manifests + inventory) | Yes — single shared walk |
| `filesystem_git/git_summary_monorepo` | ~10.4ms | Branch/tracking traversal on monorepo | Yes — trim summary scope |
| `filesystem_git/git_full_monorepo` | ~16.5ms | Unconditional diff building even when clean | Yes — early-exit clean repos |

---

## Critical: `languages_shallow_deep_mix` — Re-baseline Required

### Problem

The 443% "regression" is **not a code regression**. Commit `2b462c87` fixed a critical bug in `scan_inventory_parallel` where worker threads' classification buffers were dropped on the floor instead of being flushed to the shared accumulator. Before the fix, the parallel walker returned an empty `Vec<FileClassification>` after walking all files — so `detect_languages()` was benchmarking a no-op.

After the fix, the benchmark correctly measures:
- Parallel directory traversal via `ignore::WalkBuilder::build_parallel()`
- Per-file extension/registry lookup for ~110 files
- `FileClassification` allocation and sorting
- Language aggregation and summary computation

### Source Files

- `sniff/lib/src/filesystem/file_types/classify.rs` — `scan_inventory_parallel()` (fixed in `2b462c87`)
- `sniff/lib/src/filesystem/languages.rs` — `detect_languages()`
- `sniff/lib/benches/cases/filesystem.rs` — `languages_shallow_deep_mix` benchmark

### Fix

**No code change required.** Update the benchmark baseline to reflect the post-fix reality. The current ~2.4ms is the true cost of language detection on the `language_mix_tree` fixture.

```bash
# Re-baseline this specific benchmark
cargo bench --bench perf filesystem_languages
```

---

## Important: `detect_blast_radius_docs` — Eliminate Redundant Repo Detection

### Problem

`detect_blast_radius_docs()` calls `collect_repo_packages(&repo_root)`, which internally invokes `detect_repo()` — a full monorepo structure detection including manifest parsing, workspace boundary discovery, and package enrichment. For a docs fixture with 200 markdown files, this redundant work dominates the benchmark.

The package info is barely used in blast-radius mode: it's only stored in the returned `MarkdownMeta.package` field and is not needed for the actual `has_blast_radius` filtering.

**Current code path:**
```
detect_blast_radius_docs()
  → git2::Repository::discover()              [cheap]
  → collect_repo_packages()                   [EXPENSIVE: calls detect_repo()]
  → WalkBuilder (markdown files)              [cheap]
  → read_frontmatter_only() + parse_blast_radius()  [cheap]
```

### Source Files

- `sniff/lib/src/filesystem/docs.rs` — `detect_blast_radius_docs()` (lines 173–207)
- `sniff/lib/src/filesystem/docs.rs` — `collect_repo_packages()` (lines 209–227)
- `sniff/lib/src/filesystem/repo.rs` — `detect_repo()` (called indirectly)

### Fix

**Option A (recommended):** Make package detection lazy. Only resolve packages when the caller actually needs them, or skip it entirely in blast-radius mode.

```rust
// In detect_blast_radius_docs — skip package detection
pub fn detect_blast_radius_docs(root: &Path) -> Option<Vec<MarkdownMeta>> {
    let repo = git2::Repository::discover(root).ok()?;
    let repo_root = repo.workdir()?.to_path_buf();
    // REMOVED: let packages = collect_repo_packages(&repo_root);

    let walker = WalkBuilder::new(&repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let mut docs: Vec<MarkdownMeta> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry.path().extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .filter_map(|entry| {
            parse_markdown_meta_with_mode(
                entry.path(),
                &repo_root,
                &[], // empty packages — blast_radius doesn't need them
                DocParseMode::BlastRadiusOnly,
            )
        })
        .filter(|doc| doc.has_blast_radius)
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    if docs.is_empty() { None } else { Some(docs) }
}
```

**Option B:** Add a `detect_blast_radius_docs_with_packages()` variant that accepts pre-computed package info, allowing callers who already have repo structure to avoid redundant detection.

**Expected improvement:** ~10–15% recovery (proportional to the time spent in `detect_repo()` on the docs fixture).

---

## Important: `git_summary_small` — Trim the `GitRequest::summary()` Scope

### Problem

`GitRequest::summary()` claims to be "minimal" — branch + dirty counts only — but `GitRepo::detect_with_request()` still invokes:

1. `get_local_branches()` — iterates all local branches, peels each to a commit, computes ahead/behind via `graph_ahead_behind()`
2. `get_tracking_status()` — resolves HEAD, finds upstream tracking branches, computes ahead/behind per remote
3. `get_remotes()` — lists all remotes, resolves URLs
4. `get_git_config()` — reads git config files from disk

For a small repo with 1 branch and no remotes, steps 1–3 are mostly no-ops, but they still pay libgit2 object resolution and graph traversal costs. The 12.8% regression suggests these costs grew slightly (likely from the `get_repo_status_with_changes` refactor adding overhead to related code paths, or from git2 internal changes).

### Source Files

- `sniff/lib/src/filesystem/git/types.rs` — `GitRepo::detect_with_request()` (lines 650–726)
- `sniff/lib/src/filesystem/git/remote_refresh.rs` — `get_local_branches()`, `get_tracking_status()`, `get_git_config()`
- `sniff/lib/src/filesystem/git/status.rs` — `get_repo_status_counts_detailed()`

### Fix

**Add a `GitRequest::minimal()` preset** that truly does the absolute minimum:

```rust
impl GitRequest {
    /// Absolute minimum: branch name and dirty yes/no.
    /// No counts, no branches, no remotes, no config, no tracking.
    pub fn minimal() -> Self {
        Self {
            commit_count: 0,
            include_file_changes: false,
            include_file_diffs: false,
            include_worktrees: false,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
        }
    }
}
```

Then branch `detect_with_request()` to skip the expensive calls for minimal requests:

```rust
pub fn detect_with_request(&self, request: &GitRequest) -> Result<GitInfo> {
    let current_branch = self.current_branch();

    // Skip ALL expensive work for minimal requests
    let is_minimal = request.commit_count == 0
        && !request.include_file_changes
        && !request.include_worktrees;

    let (status, file_changes) = if request.include_file_changes {
        super::status::get_repo_status_with_changes(&self.repo, request.include_file_diffs)?
    } else if is_minimal {
        // Ultra-light: just bool, no counts
        let is_dirty = super::status::get_repo_status_counts(&self.repo).0;
        let status = RepoStatus {
            is_dirty,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            dirty: Vec::new(),
            untracked: Vec::new(),
            is_behind: None,
        };
        (status, Vec::new())
    } else {
        let (is_dirty, staged, unstaged, untracked) =
            super::status::get_repo_status_counts_detailed(&self.repo);
        let status = RepoStatus {
            is_dirty,
            staged_count: staged,
            unstaged_count: unstaged,
            untracked_count: untracked,
            dirty: Vec::new(),
            untracked: Vec::new(),
            is_behind: None,
        };
        (status, Vec::new())
    };

    let remotes = if is_minimal {
        Vec::new()
    } else {
        super::remote_refresh::get_remotes(&self.repo, request.include_remote_branch_details)
    };

    let worktrees = if request.include_worktrees {
        super::remote_refresh::get_worktrees(&self.repo)
    } else {
        HashMap::new()
    };

    let config = if is_minimal {
        GitConfig::default()
    } else {
        super::remote_refresh::get_git_config(&self.repo)
    };

    let branches = if is_minimal {
        Vec::new()
    } else {
        super::remote_refresh::get_local_branches(&self.repo, current_branch.as_deref())
    };

    let tracking = if is_minimal {
        Vec::new()
    } else {
        super::remote_refresh::get_tracking_status(&self.repo, current_branch.as_deref())
    };

    // ... rest unchanged
}
```

**Alternative (simpler):** Just make `GitRequest::summary()` skip branches and tracking by default. These are not "summary" level concerns.

**Expected improvement:** 10–15% recovery on `git_summary_small`.

---

## Important: `detect_gpus_only` — Defer `sysinfo::System` Initialization

### Problem

`detect_hardware_with_request()` unconditionally initializes `sysinfo::System` with CPU and memory refresh:

```rust
let sys = System::new_with_specifics(
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything()),
);
```

Even when the request is `HardwareRequest::summary().include_gpu(true)` (GPU only), the function still refreshes CPU and memory. On macOS, `System` initialization can take 1–2ms. The GPU detection itself via IOKit is only ~200µs.

The 6.3% regression is small but represents unnecessary work. It may also fluctuate with system load since `sysinfo` reads `/proc` or macOS IOKit counters.

### Source Files

- `sniff/lib/src/hardware/mod.rs` — `detect_hardware_with_request()` (lines 66–235)

### Fix

Track whether CPU/memory are actually needed and skip `System` init when they're not:

```rust
pub fn detect_hardware_with_request(request: &HardwareRequest) -> Result<HardwareInfo> {
    let needs_cpu_memory = true; // Always needed for summary, but can be skipped for GPU-only

    // For requests that ONLY want GPU/audio/storage, defer System init
    let sys = if request.include_gpu || request.include_audio || request.include_storage {
        // Check if CPU/memory were explicitly requested or if this is summary()
        // HardwareRequest::summary() has all flags false — we need a flag to distinguish
        // "summary (needs CPU+memory)" from "GPU only"
        ...
    };
```

A cleaner approach: add an explicit `include_cpu` / `include_memory` flag to `HardwareRequest`, or check if the request is the `summary()` preset:

```rust
// In detect_hardware_with_request:
let is_summary_preset = !request.include_storage && !request.include_gpu && !request.include_audio;
let needs_sys = is_summary_preset || request.include_storage || request.include_gpu || request.include_audio;
// Actually: summary() needs CPU+memory, full() needs everything,
// but .include_gpu(true) on summary should NOT need CPU+memory
```

The real fix is to make `HardwareRequest` builder track whether core (CPU+memory) is needed:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequest {
    pub include_cpu: bool,      // NEW
    pub include_memory: bool,   // NEW
    pub include_storage: bool,
    pub include_gpu: bool,
    pub include_audio: bool,
}

impl HardwareRequest {
    pub fn summary() -> Self {
        Self {
            include_cpu: true,
            include_memory: true,
            include_storage: false,
            include_gpu: false,
            include_audio: false,
        }
    }

    pub fn full() -> Self {
        Self {
            include_cpu: true,
            include_memory: true,
            include_storage: true,
            include_gpu: true,
            include_audio: true,
        }
    }
}
```

Then in `detect_hardware_with_request`:

```rust
let (cpu, memory) = if request.include_cpu || request.include_memory {
    let sys = System::new_with_specifics(...);
    let cpu = if request.include_cpu { ... } else { CpuInfo::default() };
    let memory = if request.include_memory { ... } else { MemoryInfo::default() };
    (cpu, memory)
} else {
    (CpuInfo::default(), MemoryInfo::default())
};
```

**Expected improvement:** 5–10% recovery on `detect_gpus_only` by eliminating ~1–2ms of `sysinfo` overhead.

---

## Important: `programs_info_detect` / `programs_detect` — Cache ExecutableIndex

### Problem

Both benchmarks exercise `ProgramsInfo::detect()`, which spends ~80% of its time in `ExecutableIndex::build_eager_path()`. That function performs two I/O-heavy operations on every call:

1. **`scan_path_executables()`** — reads every directory on `PATH` (often 20–50 dirs on a developer Mac with Homebrew, cargo, npm, etc.), stat-ing thousands of files.
2. **`build_bundle_index()`** (macOS) — checks `/Applications/*.app` and `~/Applications/*.app` existence for ~20 known binaries.

The actual category detection (8× `HashMap` lookups via `CategoryDetector::new_with_index`) is negligible in comparison. The benchmark is therefore measuring **host filesystem I/O**, not algorithmic work. On different machines or with different `PATH` lengths, the 24–25ms figure will vary wildly.

**Current code path:**
```
ProgramsInfo::detect()
  → ExecutableIndex::build_eager_path()
     → scan_path_executables()          [EXPENSIVE: O(PATH dirs × files per dir)]
     → build_bundle_index()             [EXPENSIVE: ~40 stat calls on macOS]
  → 8× CategoryDetector::new_with_index()  [cheap: HashMap probes]
```

### Source Files

- `sniff/lib/src/executable_index.rs` — `build_eager_path()`, `scan_path_executables()`, `build_bundle_index()`
- `sniff/lib/src/programs/mod.rs` — `ProgramsInfo::detect()` (lines 203–248)
- `sniff/lib/benches/cases/programs.rs` — `programs_info_detect` benchmark
- `sniff/lib/benches/cases/inventory.rs` — `programs_detect` benchmark

### Fix

**Option A (recommended):** Cache the eager index and bundle index with `std::sync::OnceLock` so the PATH scan and bundle checks happen at most once per process:

```rust
// In executable_index.rs
use std::sync::OnceLock;

static EAGER_PATH_CACHE: OnceLock<HashMap<OsString, PathBuf>> = OnceLock::new();
static BUNDLE_INDEX_CACHE: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();

impl ExecutableIndex {
    pub fn build_eager_path() -> Self {
        let eager_path = Some(EAGER_PATH_CACHE.get_or_init(scan_path_executables).clone());
        
        #[cfg(target_os = "macos")]
        let bundle_executables = BUNDLE_INDEX_CACHE
            .get_or_init(build_bundle_index)
            .clone();
        
        // ... rest unchanged
    }
}
```

**Option B:** Make the benchmarks use a controlled, short `PATH` so they measure category-detection logic rather than host I/O. This makes results deterministic across machines but doesn't improve real-world performance.

**Option C:** Lazily build the eager index on first lookup rather than upfront. For `ProgramsInfo::detect()` this doesn't help (all categories are queried), but for single-shot lookups it defers I/O.

**Expected improvement:** 70–90% reduction in benchmark time (from ~25ms to ~2–3ms) by eliminating redundant I/O. Real-world impact is smaller because the index is typically built once per sniff invocation, but repeated CLI calls (e.g. `sniff software` followed by `sniff repo`) would benefit.

---

## Important: `git_deep/100` — Reduce Unified Diff Allocation

### Problem

`GitRequest::deep()` enables `include_file_diffs=true`, which causes `get_repo_status_with_changes()` to collect full unified diff text for every dirty file via `aggregate_diff()`. With 100 dirty files, this produces large `HashMap<PathBuf, String>` allocations:

- `diff.print(git2::DiffFormat::Patch, ...)` is invoked for every hunk header, context line, addition, and deletion across the entire repository.
- Each callback does a `HashMap::entry(path).or_default()` lookup followed by `String::push_str`.
- The patch strings are never pre-sized, so they reallocate repeatedly as diff text accumulates.

For the `git_deep/100` fixture (one tracked file per dirty entry), the diff text itself is small per file, but the callback overhead and HashMap churn scale linearly with dirty count.

### Source Files

- `sniff/lib/src/filesystem/git/status.rs` — `get_repo_status_with_changes()` (lines 30–228)
- `sniff/lib/src/filesystem/git/diff.rs` — `aggregate_diff()` (lines 28–63)
- `sniff/lib/benches/cases/git.rs` — `git_deep` benchmark

### Fix

**Pre-size patch HashMaps** based on the known dirty file count (available from `repo.statuses()`):

```rust
// In get_repo_status_with_changes:
let statuses = repo.statuses(Some(&mut opts))?;

// Pre-size based on status count to avoid reallocation
let estimated_dirty = statuses.iter().count();
let mut staged_patches: HashMap<PathBuf, String> = 
    HashMap::with_capacity(estimated_dirty);
let mut unstaged_patches: HashMap<PathBuf, String> = 
    HashMap::with_capacity(estimated_dirty);
```

**Alternative (more impactful):** Stream diffs instead of collecting them. Many consumers of `GitRequest::deep()` only need diffs for display and could accept a callback-based API:

```rust
pub(crate) fn get_repo_status_with_changes_streaming(
    repo: &Repository,
    mut on_diff: impl FnMut(&Path, &str),
) -> Result<RepoStatus> {
    // ... build diffs ...
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        // Call on_diff directly instead of accumulating in HashMap
        ...
    })?;
}
```

This would eliminate the `HashMap<String>` allocations entirely for streaming consumers.

**Expected improvement:** 15–25% on `git_deep/100` from pre-sizing; 30–50% from streaming for consumers that don't need retained patch strings.

---

## Important: `filesystem_summary_request` — Skip Shared Manifest Walk for Structure-Only Repo

### Problem

`detect_filesystem_with_request()` spawns a shared parallel walk (`system_view::build_filesystem_system_view`) whenever `request.repo.is_some()`. For the summary benchmark:

```rust
let req = FilesystemRequest::new()
    .git(GitRequest::summary())
    .repo(RepoRequest::structure())   // structure_only = true
    .without_docs()
    .without_formatting()
    .without_file_inventory();
```

The shared walk is configured with `collect_manifests: true` because `request.repo.is_some()`. However, `RepoRequest::structure()` means `structure_only = true`, and `detect_repo_inner_with_shared()` with `structure_only=true` **does not use the manifest index at all** — it only detects workspace tools via root-level manifest parsing (e.g. `detect_cargo_workspace`).

So the parallel walk that collects every `Cargo.toml`/`package.json` in the monorepo is **completely wasted work** for structure-only requests.

### Source Files

- `sniff/lib/src/filesystem/mod.rs` — `detect_filesystem_with_request()` (lines 78–280)
- `sniff/lib/src/filesystem/system_view.rs` — `build_filesystem_system_view()` (lines 67–144)
- `sniff/lib/src/filesystem/repo/detection.rs` — `detect_repo_inner_with_shared()` (lines 116–218)
- `sniff/lib/benches/cases/filesystem.rs` — `filesystem_summary_request` benchmark

### Fix

Make `need_shared_view` aware of `structure_only` so it can skip the walk when only structure-level repo detection is needed:

```rust
// In detect_filesystem_with_request:
let need_repo_full = request
    .repo
    .as_ref()
    .is_some_and(|repo| !repo.structure_only);

let need_shared_view = need_repo_full
    || request.include_file_inventory
    || request.include_docs
    || request.include_formatting;
// REMOVED: || request.repo.is_some()
// Structure-only repo detection doesn't need the shared walk.
```

For the `filesystem_summary_request` benchmark, this eliminates the entire parallel directory walk (~8–12ms on the monorepo fixture).

**Expected improvement:** 40–60% reduction on `filesystem_summary_request` (from ~16ms to ~7–9ms).

---

## Important: `repo_full_huge` — Parallelize ManifestIndex + Reduce Path Normalization Syscalls

### Problem

`detect_repo()` on a huge monorepo is dominated by three costs:

1. **`ManifestIndex::build()` uses single-threaded `walkdir::WalkDir`** (lines 97–149 of `manifest_index.rs`). For a huge monorepo with 100+ packages and thousands of subdirectories, this sequential walk is the largest single cost.
2. **`canonicalize_path()` in `ManifestIndex::from_grouped()`** calls `std::fs::canonicalize()` for every manifest directory (one per package). For 100+ packages, this is 100+ syscalls.
3. **`refresh_package_boundaries()` calls `normalize_path()` for every file classification** (lines 822–833 of `detection.rs`). `normalize_path()` on relative paths calls `std::env::current_dir()`, which is a `getcwd()` syscall. For a huge monorepo with 5,000+ files, this is 5,000+ syscalls in a tight loop.

### Source Files

- `sniff/lib/src/filesystem/repo/manifest_index.rs` — `ManifestIndex::build()`, `from_grouped()`
- `sniff/lib/src/filesystem/repo/detection.rs` — `refresh_package_boundaries()`
- `sniff/lib/benches/cases/repo.rs` — `repo_full_huge` benchmark

### Fix

**Fix 1: Use parallel walker for ManifestIndex.** Replace `walkdir::WalkDir` with `ignore::WalkBuilder::build_parallel()` (same pattern as `system_view.rs` and `classify.rs`):

```rust
// In ManifestIndex::build:
use ignore::WalkBuilder;
use std::sync::{Arc, Mutex};

let grouped = Arc::new(Mutex::new(HashMap::new()));
WalkBuilder::new(root)
    .hidden(false)
    .git_ignore(true)
    .build_parallel()
    .run(|| {
        let grouped = Arc::clone(&grouped);
        let mut local = HashMap::new();
        Box::new(move |result| {
            // ... process entry into local ...
            // On Drop, flush local to grouped
        })
    });
```

**Fix 2: Replace `canonicalize_path` with `normalize_path` in ManifestIndex.** The canonicalization is only used for deduplication during package merging. Lexical normalization (`normalize_path`) is sufficient unless the repo contains symlinks, which is rare:

```rust
fn from_grouped(grouped: HashMap<PathBuf, HashSet<ManifestKind>>) -> Self {
    let entries = grouped
        .into_iter()
        .map(|(original, kinds)| {
            let canonical = normalize_path(&original);  // WAS: canonicalize_path
            ManifestEntry { original, canonical, kinds }
        })
        .collect();
    Self { entries }
}
```

**Fix 3: Pre-compute repo root absolute path in `refresh_package_boundaries`.** The inventory paths are relative to the repo root, but the function converts each to absolute then normalizes. Instead, work with relative paths directly:

```rust
// Instead of:
let abs_path = repo_root.join(&classification.path);
let normalized_abs = normalize_path(&abs_path);

// Use relative paths for package containment checks:
let rel_path = &classification.path; // already relative to repo_root
let mut current = rel_path.parent();
while let Some(parent) = current {
    if let Some(&pkg_idx) = package_path_to_index.get(parent) { ... }
    current = parent.parent();
}
```

**Expected improvement:** 30–40% from parallel manifest walk; 10–15% from eliminating canonicalize syscalls; 10–20% from avoiding per-file normalization. Combined: 50–75% reduction on `repo_full_huge` (from ~56ms to ~15–25ms).

---

## Important: `repo_with_inventory_monorepo` — Eliminate Double Directory Walk

### Problem

`detect_repo_with_inventory()` performs **two full directory walks**:

1. `detect_repo_inner_with_shared()` builds `ManifestIndex::build(root)` via single-threaded `walkdir::WalkDir`.
2. Then, because `shared_repo_inventory` is `None`, it calls `scan_file_inventory(root)` which does a parallel `ignore::WalkBuilder` walk.

These walks have overlapping scope (the entire repo tree) but use different walkers and different skip logic. The total cost is roughly the sum of both walks.

In contrast, `detect_filesystem_with_request()` is smarter: it uses a single `system_view::build_filesystem_system_view()` walk that collects both manifests and file classifications concurrently.

### Source Files

- `sniff/lib/src/filesystem/repo/types.rs` — `detect_repo_with_inventory()`
- `sniff/lib/src/filesystem/repo/detection.rs` — `detect_repo_inner_with_shared()` (lines 116–218)
- `sniff/lib/benches/cases/filesystem.rs` — `repo_with_inventory_monorepo` benchmark

### Fix

Refactor `detect_repo_with_inventory()` to build a shared `FilesystemSystemView` first, then pass both the manifest index and inventory into `detect_repo_inner_with_shared()`:

```rust
pub fn detect_repo_with_inventory(root: &Path) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    let options = system_view::SharedWalkOptions {
        collect_manifests: true,
        collect_inventory: true,
        collect_docs: false,
    };
    let view = system_view::build_filesystem_system_view(root, options);
    
    let (repo, inventory) = detection::detect_repo_inner_with_shared(
        root,
        false, // structure_only = false
        view.manifest_index.as_ref(),
        view.inventory.as_ref(),
    )?;
    
    Ok((repo, inventory.or_else(|| view.inventory)))
}
```

This consolidates two walks into one, matching the architecture already used by `detect_filesystem_with_request()`.

**Expected improvement:** 40–50% reduction (from ~16ms to ~8–10ms) by eliminating the second walk.

---

## Important: `git_full_monorepo` — Early-Exit Diff Building for Clean Repos

### Problem

`get_repo_status_with_changes()` unconditionally builds staged and unstaged diffs:

```rust
let staged_diff = head_tree
    .as_ref()
    .and_then(|tree| repo.diff_tree_to_index(Some(tree), None, None).ok());
let unstaged_diff = repo.diff_index_to_workdir(None, None).ok();
```

Even when the repository is completely clean, `diff_index_to_workdir` must stat every tracked file to confirm no changes exist. For a monorepo with thousands of tracked files, this is expensive — typically 3–5ms even with zero dirty files.

The function already calls `repo.statuses()` at the top, which determines whether the repo is dirty. This information is available before the diffs are built.

### Source Files

- `sniff/lib/src/filesystem/git/status.rs` — `get_repo_status_with_changes()` (lines 30–228)
- `sniff/lib/benches/cases/filesystem.rs` — `git_full_monorepo` benchmark

### Fix

Check whether any statuses exist before building diffs. If the repo is clean, skip diff construction entirely:

```rust
pub(crate) fn get_repo_status_with_changes(
    repo: &Repository,
    include_diffs: bool,
) -> Result<(RepoStatus, Vec<FileChange>)> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    // Early exit for clean repositories
    if statuses.is_empty() {
        return Ok((
            RepoStatus {
                is_dirty: false,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
                dirty: Vec::new(),
                untracked: Vec::new(),
                is_behind: None,
            },
            Vec::new(),
        ));
    }

    // Only build diffs when there are actual changes
    let head_tree = repo.head().and_then(|h| h.peel_to_tree()).ok();
    let staged_diff = head_tree
        .as_ref()
        .and_then(|tree| repo.diff_tree_to_index(Some(tree), None, None).ok());
    let unstaged_diff = repo.diff_index_to_workdir(None, None).ok();
    // ... rest unchanged
}
```

**Expected improvement:** 20–30% on `git_full_monorepo` when the repo is clean (the benchmark fixture has 2 dirty files, so the improvement is smaller — ~5–10%). More importantly, this eliminates a major cost for the common case of clean repositories.

---

## Important: `filesystem_full_request` — Combine Staged Optimizations

### Problem

`filesystem_full_request` (~29ms) is the most expensive staged benchmark because it enables every subsystem: git full, repo full, file inventory, docs, and formatting. The dominant costs are:

1. **Shared parallel walk** (~10–12ms): Collects manifests, inventory, and docs in one pass.
2. **Git full detection** (~8–10ms): Includes commit history, file changes, branches, remotes, config.
3. **Repo full enrichment** (~6–8ms): Package discovery, dependency parsing, `refresh_package_boundaries`.

This benchmark doesn't have a single bottleneck — it accumulates every subsystem's cost.

### Source Files

- `sniff/lib/src/filesystem/mod.rs` — `detect_filesystem_with_request()`
- `sniff/lib/benches/cases/filesystem.rs` — `filesystem_full_request` benchmark

### Fix

Apply the individual optimizations above in combination:

1. **Parallelize ManifestIndex build** (from `repo_full_huge` fix) — reduces repo enrichment cost.
2. **Early-exit clean-repo diffs** (from `git_full_monorepo` fix) — if the monorepo fixture is clean, skips diff building.
3. **Pre-size diff HashMaps** (from `git_deep` fix) — reduces allocation overhead when diffs are needed.
4. **Avoid sorting in language aggregation** (existing nice-to-have) — reduces `refresh_package_boundaries` cost.

**Expected improvement:** 25–40% combined reduction (from ~29ms to ~18–22ms) when all subsystem optimizations are applied.

---

## Nice-to-Have: Cache Git Config Across Invocations

### Problem

`get_git_config()` reads git config files from disk on every `detect_git_with_request()` call. Git config rarely changes during a single sniff execution, but it's re-read for every benchmark iteration.

### Source Files

- `sniff/lib/src/filesystem/git/remote_refresh.rs` — `get_git_config()` (lines 18–61)

### Fix

Add a simple `OnceLock` cache in `GitRepo`:

```rust
pub struct GitRepo {
    repo: Repository,
    repo_root: PathBuf,
    ref_decorations: RefCell<Option<HashMap<git2::Oid, Vec<RefDecoration>>>>,
    config_cache: OnceLock<GitConfig>,  // NEW
}

pub fn config(&self) -> GitConfig {
    self.config_cache.get_or_init(|| {
        super::remote_refresh::get_git_config(&self.repo)
    }).clone()
}
```

**Expected improvement:** 2–5% on repeated git detection calls.

---

## Nice-to-Have: Avoid Sorting in Language Aggregation Hot Path

### Problem

`build_language_summary()` sorts `direct_files` and `framework_files` for every language entry:

```rust
entry.direct_files.sort();
entry.framework_files.sort();
```

For the `language_mix_tree` fixture with 110 files, this is cheap. But for large monorepos with 10,000 files, this adds O(n log n) overhead per detected language.

### Source Files

- `sniff/lib/src/filesystem/file_types/aggregate.rs` — `build_language_summary()` (lines 96–183)

### Fix

Defer sorting until the results are actually consumed (e.g., serialized to JSON). Most consumers don't need sorted file lists:

```rust
// Option 1: Only sort when serializing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammingLanguageStats {
    // ... other fields ...
    #[serde(serialize_with = "serialize_sorted_files")]
    pub direct_files: Vec<PathBuf>,
}

// Option 2: Add a `sorted()` method on LanguageSummary
impl LanguageSummary {
    pub fn sorted(mut self) -> Self {
        for lang in &mut self.languages {
            lang.direct_files.sort();
            lang.framework_files.sort();
        }
        for fw in &mut self.frameworks {
            fw.files.sort();
        }
        self
    }
}
```

**Expected improvement:** Minor for small repos; 5–10% for large monorepos with many languages.

---

## Nice-to-Have: Parallelize Markdown Frontmatter Parsing

### Problem

Both `detect_docs()` and `detect_blast_radius_docs()` process markdown files sequentially:

```rust
let mut docs: Vec<MarkdownMeta> = walker
    .filter_map(|entry| entry.ok())
    .filter(|entry| /* md extension */)
    .filter_map(|entry| parse_markdown_meta_with_mode(...))
    .collect();
```

For 200+ documents, frontmatter YAML parsing could be parallelized with Rayon.

### Source Files

- `sniff/lib/src/filesystem/docs.rs` — `collect_markdown_files()` (lines 242–264)
- `sniff/lib/src/filesystem/docs.rs` — `detect_blast_radius_docs()` (lines 185–207)

### Fix

Use `rayon::iter::ParallelIterator` for the post-walk parsing stage:

```rust
use rayon::prelude::*;

let mut docs: Vec<MarkdownMeta> = walker
    .filter_map(|entry| entry.ok())
    .filter(|entry| /* md extension */)
    .collect::<Vec<_>>()  // collect paths first
    .into_par_iter()      // parallelize parsing
    .filter_map(|entry| parse_markdown_meta_with_mode(...))
    .collect();
```

**Caveat:** `serde_yaml_ng::from_str` may not be thread-safe or may have lock contention. Profile before committing.

**Expected improvement:** 20–40% on large doc sets (200+ files) if parsing dominates.

---

## Appendix: Benchmark Fixture Details

| Benchmark | Fixture | Files | Git Repo? | Notes |
|-----------|---------|-------|-----------|-------|
| `languages_shallow_deep_mix` | `language_mix_tree` | ~110 | No | 20 .rs/.ts/.py/.md shallow + 10-level deep nest |
| `detect_blast_radius_only` | `docs_repo(200, 40)` | 200 md | Yes | 40 docs with blast_radius frontmatter |
| `git_summary_small` | `small_git_repo` | ~10 | Yes | 5 commits, 2 dirty files |
| `detect_gpus_only` | Host system | N/A | N/A | macOS IOKit GPU enumeration |
| `programs_info_detect` | Host system | N/A | N/A | 8-category fan-out against live PATH |
| `programs_detect` | Host system | N/A | N/A | Same code path as `programs_info_detect` |
| `git_deep/100` | `git_repo_with_dirty_files(100)` | ~100 | Yes | 100 dirty tracked files, deep diff mode |
| `filesystem_summary_request` | `large_monorepo` | ~500 | Yes | Git summary + repo structure only |
| `filesystem_full_request` | `large_monorepo` | ~500 | Yes | All filesystem subsystems enabled |
| `repo_full_huge` | `huge_monorepo` | ~2000 | No | 100+ Cargo packages, full enrichment |
| `repo_with_inventory_monorepo` | `large_monorepo` | ~500 | Yes | Repo detection + file inventory |
| `git_summary_monorepo` | `large_monorepo` | ~500 | Yes | Git summary on monorepo scale |
| `git_full_monorepo` | `large_monorepo` | ~500 | Yes | Git full on monorepo scale |

---

## Summary: Optimization Impact by Benchmark

| Benchmark | Current | Target | Primary Fix |
|-----------|---------|--------|-------------|
| `programs_info_detect` | ~24.7ms | ~3ms | Cache `ExecutableIndex` with `OnceLock` |
| `programs_detect` | ~25.0ms | ~3ms | Same as above (identical code path) |
| `git_deep/100` | ~18.7ms | ~14ms | Pre-size diff HashMaps; consider streaming |
| `filesystem_summary_request` | ~16.5ms | ~7ms | Skip shared walk for structure-only repo |
| `filesystem_full_request` | ~29.0ms | ~18ms | Combine all subsystem fixes |
| `repo_full_huge` | ~56.5ms | ~18ms | Parallel manifest walk + reduce syscalls |
| `repo_with_inventory_monorepo` | ~15.8ms | ~8ms | Single shared walk instead of two |
| `git_summary_monorepo` | ~10.4ms | ~7ms | Trim `GitRequest::summary()` scope |
| `git_full_monorepo` | ~16.5ms | ~14ms | Early-exit diff building for clean repos |

**Highest-impact fixes to prioritize:**
1. **Cache `ExecutableIndex`** — eliminates host-I/O bound variance; benefits both benchmark determinism and repeated CLI invocations.
2. **Parallelize `ManifestIndex::build`** — single biggest reduction for monorepo repo detection.
3. **Skip shared walk for structure-only** — nearly halves `filesystem_summary_request` time.
4. **Eliminate double walk in `detect_repo_with_inventory`** — straightforward refactor using existing `system_view` infrastructure.

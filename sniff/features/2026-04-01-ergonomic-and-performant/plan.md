# Sniff Library: Ergonomics & Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure sniff-lib so callers can select exactly the detection they need via per-domain request types, eliminate redundant work in grouped collectors, and expose existing fast paths through the public API.

**Architecture:** Introduce a `DetectionPlan` with per-domain request structs (`OsRequest`, `HardwareRequest`, `NetworkRequest`, `FilesystemRequest`, `GitRequest`) that replace coarse skip flags. Internally, refactor grouped collectors into staged DAGs that share intermediate state (one manifest index, one file inventory, one PATH snapshot). Keep the existing `detect()` and `detect_with_config()` as convenience wrappers over the new plan-based API.

**Tech Stack:** Rust, serde, git2, rayon, sysinfo, thiserror

**Source review:** `sniff/features/2026-04-01-ergonomic-and-performant/review.md`

---

## Phase 1: Quick Wins

Low-risk, self-contained fixes that eliminate waste without changing the public API. Each task is independently shippable.

---

### Task 1: Fix triple summarization in detect_filesystem

**Review finding:** #2 - `detect_filesystem()` calls both `summarize_languages()` and `summarize_file_inventory()` on the same inventory, and `summarize_file_inventory()` internally calls `summarize_languages()` twice.

**Files:**
- Modify: `sniff/lib/src/filesystem/file_types/aggregate.rs:29-35`
- Modify: `sniff/lib/src/filesystem/mod.rs:87-88`
- Test: `sniff/lib/src/filesystem/file_types/aggregate.rs` (existing tests)

- [ ] **Step 1: Refactor `summarize_file_inventory` to accept pre-computed language summary**

In `sniff/lib/src/filesystem/file_types/aggregate.rs`, change `summarize_file_inventory` to compute `summarize_languages` once and reuse the result:

```rust
pub fn summarize_file_inventory(inventory: &FileInventory) -> FileAssociationBreakdown {
    let lang_summary = summarize_languages(inventory);
    FileAssociationBreakdown {
        total_files: inventory.total_files_scanned,
        by_association: summarize_associations(inventory),
        by_language: lang_summary.languages,
        by_framework: lang_summary.frameworks,
    }
}
```

- [ ] **Step 2: Refactor `detect_filesystem` to derive languages from file breakdown**

In `sniff/lib/src/filesystem/mod.rs`, replace the two separate calls with a single `summarize_file_inventory` call, then extract the `LanguageBreakdown` from the already-computed language stats inside `FileAssociationBreakdown`:

```rust
let files = inventory.as_ref().map(file_types::summarize_file_inventory);
let languages = files.as_ref().map(|fab| {
    let summary = file_types::summarize_languages(inventory.as_ref().unwrap());
    LanguageBreakdown {
        primary: summary.primary,
        secondary: summary.secondary,
        total_files_scanned: summary.total_files_scanned,
        total_language_files: summary.total_language_files,
        languages: fab.by_language.clone(),
        frameworks: fab.by_framework.clone(),
    }
});
```

Wait - this still double-computes. The cleanest approach: make `summarize_file_inventory` return both the breakdown AND the `LanguageSummary`:

```rust
pub fn summarize_file_inventory(inventory: &FileInventory) -> (FileAssociationBreakdown, LanguageSummary) {
    let lang_summary = summarize_languages(inventory);
    let breakdown = FileAssociationBreakdown {
        total_files: inventory.total_files_scanned,
        by_association: summarize_associations(inventory),
        by_language: lang_summary.languages.clone(),
        by_framework: lang_summary.frameworks.clone(),
    };
    (breakdown, lang_summary)
}
```

Then in `detect_filesystem`:

```rust
let (files, lang_summary) = match inventory.as_ref() {
    Some(inv) => {
        let (fab, ls) = file_types::summarize_file_inventory(inv);
        (Some(fab), Some(ls))
    }
    None => (None, None),
};
let languages = lang_summary.map(|s| LanguageBreakdown::from(s));
```

This requires adding a `From<LanguageSummary> for LanguageBreakdown` impl or adjusting the field mapping. Check the `LanguageBreakdown` struct to confirm fields align with `LanguageSummary`.

**Alternative (simpler):** If `LanguageBreakdown` and `LanguageSummary` are the same type (or nearly identical), just return the summary directly. If they differ, add a conversion.

- [ ] **Step 3: Verify existing tests pass**

Run: `just test` from `sniff/lib/`
Expected: All tests pass with no behavior change.

- [ ] **Step 4: Add a targeted test confirming single-pass summarization**

Add a test that verifies `summarize_file_inventory` returns consistent data with `summarize_languages` (they should produce identical language/framework data since they now share the computation):

```rust
#[test]
fn summarize_file_inventory_language_data_matches_summarize_languages() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("app.ts"), "const x = 1;").unwrap();
    fs::write(dir.path().join("style.css"), "body {}").unwrap();

    let inventory = scan_file_inventory(dir.path()).unwrap();
    let (breakdown, lang_summary) = summarize_file_inventory(&inventory);

    assert_eq!(breakdown.by_language.len(), lang_summary.languages.len());
    assert_eq!(breakdown.by_framework.len(), lang_summary.frameworks.len());
}
```

- [ ] **Step 5: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `fix(sniff-lib): eliminate triple summarization in detect_filesystem`

---

### Task 2: Use detect_docs_with_packages inside detect_filesystem

**Review finding:** #2 - `detect_filesystem()` calls `detect_docs(root)` which internally re-discovers the repo and re-runs `detect_repo()`, even though repo/package info is already available.

**Files:**
- Modify: `sniff/lib/src/filesystem/mod.rs:91`

- [ ] **Step 1: Replace detect_docs with detect_docs_with_packages**

In `sniff/lib/src/filesystem/mod.rs`, replace line 91:

```rust
// Before:
let docs = detect_docs(root);

// After:
let docs = match (git.as_ref(), repo.as_ref().and_then(|r| r.packages.as_ref())) {
    (Some(git_info), Some(packages)) => {
        let pkg_tuples: Vec<(String, PathBuf)> = packages
            .iter()
            .map(|p| (p.name.clone(), p.path.clone()))
            .collect();
        docs::detect_docs_with_packages(&git_info.repo_root, &pkg_tuples)
    }
    _ => detect_docs(root),
};
```

This avoids the redundant `detect_repo()` call inside `detect_docs()` when repo info is already available.

- [ ] **Step 2: Verify tests pass**

Run: `just test` from `sniff/lib/`
Expected: All tests pass. Doc detection results should be identical since the same data feeds both paths.

- [ ] **Step 3: Commit**

Commit: `perf(sniff-lib): reuse repo info for doc detection in detect_filesystem`

---

### Task 3: Add lightweight status counting for worktrees

**Review finding:** #3 - `get_worktrees()` calls `get_repo_status_with_changes()` for every worktree just to get `dirty` and `changed_files`, but that function computes per-file diff stats and builds full unified diffs.

**Files:**
- Modify: `sniff/lib/src/filesystem/git.rs`

- [ ] **Step 1: Extract a `get_repo_status_counts` function**

Add a new lightweight function that only counts dirty/staged/unstaged/untracked without computing diffs or file change details:

```rust
/// Lightweight status check that only counts files by category.
///
/// Avoids the cost of per-file diff stat computation and unified diff
/// generation. Use this when you only need `is_dirty` and file counts.
fn get_repo_status_counts(repo: &Repository) -> (bool, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let total = staged + unstaged + untracked;
    (total > 0, total)
}
```

- [ ] **Step 2: Update get_worktrees to use the lightweight function**

Replace lines 1906-1912 in `get_worktrees`:

```rust
// Before:
let (dirty, changed_files) = get_repo_status_with_changes(&worktree_repo)
    .map(|(s, _)| {
        let count = s.staged_count + s.unstaged_count + s.untracked_count;
        (s.is_dirty, count)
    })
    .unwrap_or((false, 0));

// After:
let (dirty, changed_files) = get_repo_status_counts(&worktree_repo);
```

- [ ] **Step 3: Run tests**

Run: `just test` from `sniff/lib/`
Expected: All tests pass. Worktree detection should produce identical `dirty` and `changed_files` values.

- [ ] **Step 4: Commit**

Commit: `perf(sniff-lib): use lightweight status counts for worktree dirty checks`

---

### Task 4: Fix ProgramsInfo::detect() misleading doc comment

**Review finding:** #8 - Doc comment claims "runs detection in parallel" but categories are constructed sequentially.

**Files:**
- Modify: `sniff/lib/src/programs/mod.rs:170-171`

- [ ] **Step 1: Update the doc comment**

```rust
// Before:
/// Detect all installed programs across all categories.
///
/// This runs detection in parallel for all program categories.

// After:
/// Detect all installed programs across all categories.
///
/// Each category performs its own internal parallel lookups via Rayon,
/// but categories themselves are constructed sequentially.
```

- [ ] **Step 2: Commit**

Commit: `docs(sniff-lib): correct ProgramsInfo::detect() parallelism claim`

---

### Task 5: Rename include_cpu_usage and clarify detect_hardware_with_usage

**Review finding:** #7 - `include_cpu_usage` is misleading because `detect_hardware_with_usage()` currently just calls `detect_hardware()`. The flag suggests a behavior change that does not occur.

**Files:**
- Modify: `sniff/lib/src/lib.rs:60-61, 104-107, 186-192`
- Modify: `sniff/lib/src/hardware/mod.rs:190-210`
- Modify: `sniff/cli/src/main.rs` (any usage of `include_cpu_usage`)

- [ ] **Step 1: Rename the field and builder method**

In `sniff/lib/src/lib.rs`:

```rust
// In SniffConfig struct:
/// Reserved for future CPU usage sampling (~200ms measurement).
/// Currently has no effect.
#[deprecated(note = "CPU usage sampling is not yet implemented. This field has no effect.")]
pub include_cpu_usage: bool,
```

Alternatively, just remove the field entirely since it does nothing. Check if `sniff-cli` or any client actually reads this field. If no client depends on the behavior (which it can't, since there is none), remove it:

- Remove `include_cpu_usage` from `SniffConfig`
- Remove the `include_cpu_usage()` builder method
- In `detect_with_config()`, always call `detect_hardware()` (remove the `detect_hardware_with_usage` branch)
- Remove or deprecate `detect_hardware_with_usage()` in hardware/mod.rs

- [ ] **Step 2: Search for all usages and update**

Search for `include_cpu_usage` across the codebase and update all call sites. The CLI may pass this flag via args - update accordingly.

- [ ] **Step 3: Run tests**

Run: `just test` from `sniff/`
Expected: All tests pass.

- [ ] **Step 4: Commit**

Commit: `fix(sniff-lib): remove no-op include_cpu_usage config flag`

---

## Phase 2: Request Types & DetectionPlan API

Introduce per-domain request types that give callers fine-grained control over what gets detected, replacing the coarse skip flags. The existing API remains as convenience wrappers.

---

### Task 6: Create the request types module

**Review finding:** #1 - The top-level API is too coarse for callers with mixed cost requirements.

**Files:**
- Create: `sniff/lib/src/request.rs`
- Modify: `sniff/lib/src/lib.rs` (add `pub mod request;` and re-exports)

- [ ] **Step 1: Write tests for request type builder API**

Create request type tests inline in the new module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_plan_defaults_to_all_full() {
        let plan = DetectionPlan::default();
        assert!(plan.os.is_some());
        assert!(plan.hardware.is_some());
        assert!(plan.network.is_some());
        assert!(plan.filesystem.is_some());
    }

    #[test]
    fn detection_plan_skip_sections() {
        let plan = DetectionPlan::new()
            .without_os()
            .without_hardware();
        assert!(plan.os.is_none());
        assert!(plan.hardware.is_none());
        assert!(plan.network.is_some());
        assert!(plan.filesystem.is_some());
    }

    #[test]
    fn os_request_summary_vs_full() {
        let summary = OsRequest::summary();
        assert!(!summary.include_package_managers);
        assert!(!summary.include_time);

        let full = OsRequest::full();
        assert!(full.include_package_managers);
        assert!(full.include_time);
    }

    #[test]
    fn hardware_request_detail_levels() {
        let summary = HardwareRequest::summary();
        assert!(!summary.include_storage);
        assert!(!summary.include_gpu);
        assert!(!summary.include_audio);

        let full = HardwareRequest::full();
        assert!(full.include_storage);
        assert!(full.include_gpu);
        assert!(full.include_audio);
    }

    #[test]
    fn network_request_interfaces_only() {
        let req = NetworkRequest::interfaces_only();
        assert!(!req.include_wan_ip);

        let full = NetworkRequest::full();
        assert!(full.include_wan_ip);
    }

    #[test]
    fn git_request_detail_levels() {
        let summary = GitRequest::summary();
        assert_eq!(summary.commit_count, 0);
        assert!(!summary.include_file_changes);
        assert!(!summary.include_worktrees);
        assert!(!summary.refresh_remote_tracking);

        let full = GitRequest::full();
        assert_eq!(full.commit_count, 10);
        assert!(full.include_file_changes);
        assert!(full.include_worktrees);
    }

    #[test]
    fn filesystem_request_composition() {
        let req = FilesystemRequest::new()
            .git(GitRequest::summary())
            .without_docs()
            .without_repo();
        assert!(req.git.is_some());
        assert!(req.repo.is_none());
        assert!(!req.include_docs);
    }
}
```

- [ ] **Step 2: Define the request types**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level detection plan controlling which domains are collected and at what detail level.
///
/// ## Examples
///
/// ```
/// use sniff::request::{DetectionPlan, OsRequest, HardwareRequest, NetworkRequest};
///
/// // Minimal detection: OS summary + hardware summary, skip network/filesystem
/// let plan = DetectionPlan::new()
///     .os(OsRequest::summary())
///     .hardware(HardwareRequest::summary())
///     .without_network()
///     .without_filesystem();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionPlan {
    /// Base directory for filesystem analysis. Falls back to cwd if None.
    pub base_dir: Option<PathBuf>,
    /// OS detection request. None skips OS detection entirely.
    pub os: Option<OsRequest>,
    /// Hardware detection request. None skips hardware detection entirely.
    pub hardware: Option<HardwareRequest>,
    /// Network detection request. None skips network detection entirely.
    pub network: Option<NetworkRequest>,
    /// Filesystem detection request. None skips filesystem detection entirely.
    pub filesystem: Option<FilesystemRequest>,
}

impl Default for DetectionPlan {
    fn default() -> Self {
        Self {
            base_dir: None,
            os: Some(OsRequest::full()),
            hardware: Some(HardwareRequest::full()),
            network: Some(NetworkRequest::full()),
            filesystem: Some(FilesystemRequest::default()),
        }
    }
}

impl DetectionPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_dir(mut self, path: PathBuf) -> Self {
        self.base_dir = Some(path);
        self
    }

    pub fn os(mut self, request: OsRequest) -> Self {
        self.os = Some(request);
        self
    }

    pub fn hardware(mut self, request: HardwareRequest) -> Self {
        self.hardware = Some(request);
        self
    }

    pub fn network(mut self, request: NetworkRequest) -> Self {
        self.network = Some(request);
        self
    }

    pub fn filesystem(mut self, request: FilesystemRequest) -> Self {
        self.filesystem = Some(request);
        self
    }

    pub fn without_os(mut self) -> Self {
        self.os = None;
        self
    }

    pub fn without_hardware(mut self) -> Self {
        self.hardware = None;
        self
    }

    pub fn without_network(mut self) -> Self {
        self.network = None;
        self
    }

    pub fn without_filesystem(mut self) -> Self {
        self.filesystem = None;
        self
    }
}

/// Controls which OS subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRequest {
    /// Include system package manager detection (can be slow on Linux due to PATH scanning)
    pub include_package_managers: bool,
    /// Include locale detection
    pub include_locale: bool,
    /// Include timezone and NTP status (NTP can take up to 10s on Linux)
    pub include_time: bool,
}

impl OsRequest {
    /// Core identity only: OS type, name, version, kernel, hostname.
    pub fn summary() -> Self {
        Self {
            include_package_managers: false,
            include_locale: false,
            include_time: false,
        }
    }

    /// Everything including package managers, locale, and timezone/NTP.
    pub fn full() -> Self {
        Self {
            include_package_managers: true,
            include_locale: true,
            include_time: true,
        }
    }

    pub fn include_package_managers(mut self, include: bool) -> Self {
        self.include_package_managers = include;
        self
    }

    pub fn include_locale(mut self, include: bool) -> Self {
        self.include_locale = include;
        self
    }

    pub fn include_time(mut self, include: bool) -> Self {
        self.include_time = include;
        self
    }
}

/// Controls which hardware subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequest {
    /// Include storage device inventory
    pub include_storage: bool,
    /// Include GPU detection
    pub include_gpu: bool,
    /// Include audio device enumeration (~1.5s on macOS)
    pub include_audio: bool,
}

impl HardwareRequest {
    /// CPU and memory only. Skips storage, GPU, and audio (~1.5s savings on macOS).
    pub fn summary() -> Self {
        Self {
            include_storage: false,
            include_gpu: false,
            include_audio: false,
        }
    }

    /// Full hardware detection including storage, GPU, and audio devices.
    pub fn full() -> Self {
        Self {
            include_storage: true,
            include_gpu: true,
            include_audio: true,
        }
    }

    pub fn include_storage(mut self, include: bool) -> Self {
        self.include_storage = include;
        self
    }

    pub fn include_gpu(mut self, include: bool) -> Self {
        self.include_gpu = include;
        self
    }

    pub fn include_audio(mut self, include: bool) -> Self {
        self.include_audio = include;
        self
    }
}

/// Controls which network subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    /// Include WAN IP lookup (HTTP call to external service)
    pub include_wan_ip: bool,
}

impl NetworkRequest {
    /// Local interfaces only. No external HTTP call.
    pub fn interfaces_only() -> Self {
        Self {
            include_wan_ip: false,
        }
    }

    /// Full network detection including WAN IP lookup.
    pub fn full() -> Self {
        Self {
            include_wan_ip: true,
        }
    }

    pub fn include_wan_ip(mut self, include: bool) -> Self {
        self.include_wan_ip = include;
        self
    }
}

/// Controls git repository detection detail level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRequest {
    /// Number of recent commits to retrieve (0 = skip commit history)
    pub commit_count: usize,
    /// Include per-file change details (status, line counts)
    pub include_file_changes: bool,
    /// Include worktree enumeration and status
    pub include_worktrees: bool,
    /// Fetch remote tracking refs (requires network)
    pub refresh_remote_tracking: bool,
    /// Include remote branch details (requires refresh_remote_tracking)
    pub include_remote_branch_details: bool,
    /// Check which remotes contain recent commits (requires refresh_remote_tracking)
    pub include_commit_remote_containment: bool,
}

impl GitRequest {
    /// Minimal: repo root, current branch, dirty status counts. No commits, no file details.
    pub fn summary() -> Self {
        Self {
            commit_count: 0,
            include_file_changes: false,
            include_worktrees: false,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
        }
    }

    /// Standard detection with 10 commits, file changes, worktrees, but no remote refresh.
    pub fn full() -> Self {
        Self {
            commit_count: 10,
            include_file_changes: true,
            include_worktrees: true,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
        }
    }

    /// Deep detection: refreshes remote tracking refs and populates remote info on commits.
    /// This is the equivalent of the old `deep: true` flag.
    pub fn deep() -> Self {
        Self {
            commit_count: 10,
            include_file_changes: true,
            include_worktrees: true,
            refresh_remote_tracking: true,
            include_remote_branch_details: true,
            include_commit_remote_containment: true,
        }
    }

    pub fn commit_count(mut self, count: usize) -> Self {
        self.commit_count = count;
        self
    }

    pub fn include_file_changes(mut self, include: bool) -> Self {
        self.include_file_changes = include;
        self
    }

    pub fn include_worktrees(mut self, include: bool) -> Self {
        self.include_worktrees = include;
        self
    }

    pub fn refresh_remote_tracking(mut self, include: bool) -> Self {
        self.refresh_remote_tracking = include;
        self
    }
}

/// Controls repo/monorepo detection detail level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRequest {
    /// When true, only detect workspace structure (tools, package names/paths).
    /// When false, also scan per-package languages, frameworks, and file associations.
    pub structure_only: bool,
}

impl RepoRequest {
    /// Structure only: workspace tools and package list. 10-50x faster than full.
    pub fn structure() -> Self {
        Self {
            structure_only: true,
        }
    }

    /// Full repo detection with per-package language and framework scanning.
    pub fn full() -> Self {
        Self {
            structure_only: false,
        }
    }
}

/// Controls filesystem detection composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemRequest {
    /// Git detection request. None skips git entirely.
    pub git: Option<GitRequest>,
    /// Repo/monorepo detection request. None skips repo detection.
    pub repo: Option<RepoRequest>,
    /// Include file inventory and language breakdown
    pub include_file_inventory: bool,
    /// Include EditorConfig formatting detection
    pub include_formatting: bool,
    /// Include markdown document discovery
    pub include_docs: bool,
}

impl Default for FilesystemRequest {
    fn default() -> Self {
        Self {
            git: Some(GitRequest::full()),
            repo: Some(RepoRequest::full()),
            include_file_inventory: true,
            include_formatting: true,
            include_docs: true,
        }
    }
}

impl FilesystemRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn git(mut self, request: GitRequest) -> Self {
        self.git = Some(request);
        self
    }

    pub fn repo(mut self, request: RepoRequest) -> Self {
        self.repo = Some(request);
        self
    }

    pub fn without_git(mut self) -> Self {
        self.git = None;
        self
    }

    pub fn without_repo(mut self) -> Self {
        self.repo = None;
        self
    }

    pub fn without_docs(mut self) -> Self {
        self.include_docs = false;
        self
    }

    pub fn without_formatting(mut self) -> Self {
        self.include_formatting = false;
        self
    }

    pub fn without_file_inventory(mut self) -> Self {
        self.include_file_inventory = false;
        self
    }

    pub fn include_docs(mut self, include: bool) -> Self {
        self.include_docs = include;
        self
    }
}
```

- [ ] **Step 3: Run tests**

Run: `just test` from `sniff/lib/`
Expected: All new request type tests pass.

- [ ] **Step 4: Commit**

Commit: `feat(sniff-lib): add per-domain request types for detection plan API`

---

### Task 7: Implement plan-aware OS detection

**Review finding:** #6 - `detect_os()` always performs package-manager and time detection, even when callers only need core identity.

**Files:**
- Modify: `sniff/lib/src/os/mod.rs`

- [ ] **Step 1: Add `detect_os_with_request` function**

```rust
/// Detect OS information according to the given request.
pub fn detect_os_with_request(request: &OsRequest) -> Result<OsInfo> {
    let non_empty = |s: String| {
        if s.is_empty() { None } else { Some(s) }
    };

    let os_type = detect_os_type();
    let linux_distro = detect_linux_distro();
    let linux_family = linux_distro.as_ref().map(|d| d.family);

    let system_package_managers = if request.include_package_managers {
        match os_type {
            OsType::Linux => Some(detect_linux_package_managers(linux_family)),
            OsType::MacOS => Some(detect_macos_package_managers()),
            OsType::Windows => Some(detect_windows_package_managers()),
            OsType::FreeBSD | OsType::OpenBSD | OsType::NetBSD => {
                Some(detect_bsd_package_managers(os_type))
            }
            OsType::IOS | OsType::Android | OsType::Other => None,
        }
    } else {
        None
    };

    let locale = if request.include_locale {
        Some(detect_locale())
    } else {
        None
    };

    let time = if request.include_time {
        Some(detect_timezone())
    } else {
        None
    };

    Ok(OsInfo {
        os_type,
        name: System::name().unwrap_or_default(),
        version: System::os_version().unwrap_or_default(),
        long_version: System::long_os_version(),
        distribution: non_empty(System::distribution_id()),
        linux_distro,
        kernel: System::kernel_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
        uptime_seconds: System::uptime(),
        system_package_managers,
        locale,
        time,
    })
}
```

- [ ] **Step 2: Refactor `detect_os()` to delegate**

```rust
pub fn detect_os() -> Result<OsInfo> {
    detect_os_with_request(&OsRequest::full())
}
```

- [ ] **Step 3: Add test for summary mode**

```rust
#[test]
fn test_detect_os_summary_skips_expensive_fields() {
    let request = OsRequest::summary();
    let info = detect_os_with_request(&request).unwrap();
    assert!(!info.name.is_empty());
    assert!(info.system_package_managers.is_none());
    assert!(info.locale.is_none());
    assert!(info.time.is_none());
}
```

- [ ] **Step 4: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `feat(sniff-lib): add plan-aware OS detection with OsRequest`

---

### Task 8: Implement plan-aware hardware detection

**Review finding:** #7 - `detect_hardware_summary()` exists but is not exposed through the top-level API.

**Files:**
- Modify: `sniff/lib/src/hardware/mod.rs`

- [ ] **Step 1: Add `detect_hardware_with_request` function**

```rust
/// Detect hardware information according to the given request.
pub fn detect_hardware_with_request(request: &HardwareRequest) -> Result<HardwareInfo> {
    let audio_devices = if request.include_audio {
        detect_audio_devices()
    } else {
        Vec::new()
    };

    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    let cpu = CpuInfo {
        brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        arch: {
            let arch = System::cpu_arch();
            if arch.is_empty() {
                std::env::consts::ARCH.to_string()
            } else {
                arch
            }
        },
        logical_cores: sys.cpus().len(),
        physical_cores: System::physical_core_count(),
        simd: detect_simd(),
    };

    let available = sys.available_memory();
    let available_bytes = if available == 0 {
        sys.free_memory()
    } else {
        available
    };

    let memory = MemoryInfo {
        total_bytes: sys.total_memory(),
        available_bytes,
        used_bytes: sys.used_memory(),
        total_swap: sys.total_swap(),
        free_swap: sys.free_swap(),
        used_swap: sys.used_swap(),
    };

    let storage = if request.include_storage {
        storage::detect_storage()
    } else {
        Vec::new()
    };

    let gpu = if request.include_gpu {
        detect_gpus()
    } else {
        Vec::new()
    };

    Ok(HardwareInfo {
        cpu,
        memory,
        storage,
        gpu,
        audio_devices,
    })
}
```

- [ ] **Step 2: Refactor existing functions to delegate**

```rust
pub fn detect_hardware() -> Result<HardwareInfo> {
    detect_hardware_with_request(&HardwareRequest::full())
}

pub fn detect_hardware_summary() -> Result<HardwareInfo> {
    detect_hardware_with_request(&HardwareRequest::summary())
}
```

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `feat(sniff-lib): add plan-aware hardware detection with HardwareRequest`

---

### Task 9: Implement plan-aware network detection

**Review finding:** #5 - Network detection always includes WAN IP lookup even though it's the most expensive part.

**Files:**
- Modify: `sniff/lib/src/network/mod.rs`

- [ ] **Step 1: Add `detect_network_with_request` function**

```rust
/// Detect network information according to the given request.
pub fn detect_network_with_request(request: &NetworkRequest) -> Result<NetworkInfo> {
    let wan_ip_address = if request.include_wan_ip {
        detect_wan_ip()
    } else {
        None
    };

    // ... rest of detect_network() logic for interfaces ...
    // (move the existing getifaddrs logic here, replacing the current
    // detect_wan_ip() call at the top with the conditional above)
}
```

- [ ] **Step 2: Refactor `detect_network()` to delegate**

```rust
pub fn detect_network() -> Result<NetworkInfo> {
    detect_network_with_request(&NetworkRequest::full())
}
```

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `feat(sniff-lib): add plan-aware network detection with NetworkRequest`

---

### Task 10: Implement plan-aware git detection

**Review finding:** #3 - `detect_full()` eagerly computes the most expensive shape even for cheap callers.

**Files:**
- Modify: `sniff/lib/src/filesystem/git.rs`

- [ ] **Step 1: Add `detect_with_request` method to GitRepo**

```rust
impl GitRepo {
    /// Detect git information according to the given request.
    pub fn detect_with_request(&self, request: &GitRequest) -> Result<GitInfo> {
        let current_branch = self.current_branch();

        if request.refresh_remote_tracking {
            refresh_remote_tracking_refs(&self.repo);
        }

        let mut recent = if request.commit_count > 0 {
            get_recent_commits(&self.repo, request.commit_count)
        } else {
            Vec::new()
        };

        let (mut status, file_changes) = if request.include_file_changes {
            get_repo_status_with_changes(&self.repo)?
        } else {
            // Lightweight: only counts, no diffs or file change details
            let (is_dirty, staged, unstaged, untracked) =
                get_repo_status_counts_detailed(&self.repo);
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

        let remotes = get_remotes(&self.repo, request.include_remote_branch_details);

        let worktrees = if request.include_worktrees {
            get_worktrees(&self.repo)
        } else {
            HashMap::new()
        };

        let config = get_git_config(&self.repo);
        let branches = get_local_branches(&self.repo, current_branch.as_deref());
        let tracking = get_tracking_status(&self.repo, current_branch.as_deref());

        if request.refresh_remote_tracking {
            status.is_behind = summarize_behind_status(&tracking);
            if request.include_commit_remote_containment {
                populate_recent_commit_remotes(&self.repo, &mut recent);
            }
        }

        let (org, repo) = preferred_remote(&remotes)
            .and_then(|r| r.url.as_deref())
            .map(parse_org_repo)
            .unwrap_or((None, None));

        Ok(GitInfo {
            repo_root: self.repo_root.clone(),
            org,
            repo,
            current_branch,
            branches,
            in_worktree: self.repo.is_worktree(),
            base_repo_root: self.base_repo_root(),
            recent,
            status,
            remotes,
            worktrees,
            config,
            tracking,
            file_changes,
        })
    }
}
```

This requires a helper `get_repo_status_counts_detailed` that returns the individual counts (extending Task 3's `get_repo_status_counts`):

```rust
fn get_repo_status_counts_detailed(repo: &Repository) -> (bool, usize, usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0, 0, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let is_dirty = staged > 0 || unstaged > 0 || untracked > 0;
    (is_dirty, staged, unstaged, untracked)
}
```

- [ ] **Step 2: Refactor `detect_full` to delegate**

```rust
pub fn detect_full(&self, deep: bool, commit_count: usize) -> Result<GitInfo> {
    let request = if deep {
        GitRequest::deep().commit_count(commit_count)
    } else {
        GitRequest::full().commit_count(commit_count)
    };
    self.detect_with_request(&request)
}
```

- [ ] **Step 3: Add `detect_git_with_request` public function**

```rust
/// Detect git information for a path according to the given request.
pub fn detect_git_with_request(path: &Path, request: &GitRequest) -> Result<Option<GitInfo>> {
    match GitRepo::discover(path)? {
        Some(git) => Ok(Some(git.detect_with_request(request)?)),
        None => Ok(None),
    }
}
```

- [ ] **Step 4: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `feat(sniff-lib): add plan-aware git detection with GitRequest`

---

### Task 11: Implement plan-aware filesystem detection

**Review finding:** #2 - `detect_filesystem()` composes expensive work sequentially with no way to skip subsections.

**Files:**
- Modify: `sniff/lib/src/filesystem/mod.rs`

- [ ] **Step 1: Add `detect_filesystem_with_request` function**

```rust
/// Detect filesystem information according to the given request.
pub fn detect_filesystem_with_request(
    root: &Path,
    request: &FilesystemRequest,
) -> Result<FilesystemInfo> {
    // Stage 1: Git detection (if requested)
    let git = match &request.git {
        Some(git_request) => detect_git_with_request(root, git_request)?,
        None => None,
    };

    // Stage 2: Repo detection (if requested)
    let repo_root_path = git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root);
    let repo = match &request.repo {
        Some(repo_request) => {
            if repo_request.structure_only {
                detect_repo_structure(repo_root_path)?
            } else {
                detect_repo(repo_root_path)?
            }
        }
        None => None,
    };

    // Stage 3: File inventory (if requested)
    let (files, languages) = if request.include_file_inventory {
        let inventory = match repo.as_ref().and_then(|repo| repo.package_for_dir(root)) {
            Some(package) => {
                let exclude_roots = repo
                    .as_ref()
                    .and_then(|repo| repo.packages.as_ref())
                    .map(|packages| {
                        packages
                            .iter()
                            .filter(|c| c.path != package.path)
                            .filter(|c| c.path.starts_with(&package.path))
                            .map(|c| c.path.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                file_types::scan_file_inventory_with_exclusions(&package.path, &exclude_roots).ok()
            }
            None => file_types::scan_file_inventory(root).ok(),
        };

        match inventory {
            Some(inv) => {
                let (fab, lang_summary) = file_types::summarize_file_inventory(&inv);
                (Some(fab), Some(LanguageBreakdown::from(lang_summary)))
            }
            None => (None, None),
        }
    } else {
        (None, None)
    };

    // Stage 4: Formatting and docs (independent, could parallelize)
    let formatting = if request.include_formatting {
        detect_formatting(root).ok().flatten()
    } else {
        None
    };

    let docs = if request.include_docs {
        match (git.as_ref(), repo.as_ref().and_then(|r| r.packages.as_ref())) {
            (Some(git_info), Some(packages)) => {
                let pkg_tuples: Vec<(String, PathBuf)> = packages
                    .iter()
                    .map(|p| (p.name.clone(), p.path.clone()))
                    .collect();
                docs::detect_docs_with_packages(&git_info.repo_root, &pkg_tuples)
            }
            _ => detect_docs(root),
        }
    } else {
        None
    };

    Ok(FilesystemInfo {
        languages,
        files,
        git,
        repo,
        formatting,
        docs,
    })
}
```

- [ ] **Step 2: Refactor detect_filesystem to delegate**

```rust
pub fn detect_filesystem(root: &Path, deep: bool, commit_count: usize) -> Result<FilesystemInfo> {
    let git_request = if deep {
        GitRequest::deep().commit_count(commit_count)
    } else {
        GitRequest::full().commit_count(commit_count)
    };
    detect_filesystem_with_request(root, &FilesystemRequest::new().git(git_request))
}
```

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `feat(sniff-lib): add plan-aware filesystem detection with FilesystemRequest`

---

### Task 12: Implement detect_with_plan top-level API

**Review finding:** #1 - Give callers three good modes: `detect()`, `detect_with_plan()`, `module-level direct calls`.

**Files:**
- Modify: `sniff/lib/src/lib.rs`

- [ ] **Step 1: Add `detect_with_plan` function**

```rust
/// Detect system information according to a detailed plan.
///
/// This is the primary ergonomic API for callers who need fine-grained
/// control over what gets detected. Use `detect()` for sensible defaults,
/// or module-level functions for expert manual composition.
///
/// ## Examples
///
/// ```no_run
/// use sniff::{detect_with_plan, request::*};
///
/// let plan = DetectionPlan::new()
///     .os(OsRequest::summary())
///     .hardware(HardwareRequest::summary())
///     .without_network()
///     .filesystem(
///         FilesystemRequest::new()
///             .git(GitRequest::summary())
///             .repo(RepoRequest::structure())
///             .without_docs()
///     );
///
/// let result = detect_with_plan(plan).unwrap();
/// ```
pub fn detect_with_plan(plan: DetectionPlan) -> Result<SniffResult> {
    let os = match plan.os {
        Some(ref request) => Some(os::detect_os_with_request(request)?),
        None => None,
    };

    let hardware = match plan.hardware {
        Some(ref request) => Some(hardware::detect_hardware_with_request(request)?),
        None => None,
    };

    let network = match plan.network {
        Some(ref request) => Some(network::detect_network_with_request(request)?),
        None => None,
    };

    let filesystem = match plan.filesystem {
        Some(ref request) => {
            let base = plan
                .base_dir
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            Some(filesystem::detect_filesystem_with_request(&base, request)?)
        }
        None => None,
    };

    Ok(SniffResult {
        os,
        hardware,
        network,
        filesystem,
    })
}
```

- [ ] **Step 2: Add re-export of request module**

In `sniff/lib/src/lib.rs`, add:

```rust
pub mod request;
pub use request::DetectionPlan;
```

- [ ] **Step 3: Bridge SniffConfig to DetectionPlan**

Implement `From<SniffConfig> for DetectionPlan`:

```rust
impl From<SniffConfig> for DetectionPlan {
    fn from(config: SniffConfig) -> Self {
        let git_request = if config.deep {
            GitRequest::deep().commit_count(config.commit_count)
        } else {
            GitRequest::full().commit_count(config.commit_count)
        };

        DetectionPlan {
            base_dir: config.base_dir,
            os: if config.skip_os { None } else { Some(OsRequest::full()) },
            hardware: if config.skip_hardware { None } else { Some(HardwareRequest::full()) },
            network: if config.skip_network { None } else { Some(NetworkRequest::full()) },
            filesystem: if config.skip_filesystem {
                None
            } else {
                Some(FilesystemRequest::new().git(git_request))
            },
        }
    }
}
```

- [ ] **Step 4: Refactor detect_with_config to use detect_with_plan**

```rust
pub fn detect_with_config(config: SniffConfig) -> Result<SniffResult> {
    detect_with_plan(DetectionPlan::from(config))
}
```

- [ ] **Step 5: Run full test suite**

Run: `just test` from `sniff/`
Expected: All existing tests pass unchanged. The public API is additive - no breaking changes.

- [ ] **Step 6: Add integration test for detect_with_plan**

In `sniff/lib/tests/integration.rs`:

```rust
#[test]
fn test_detect_with_plan_summary_mode() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary())
        .hardware(HardwareRequest::summary())
        .without_network()
        .without_filesystem();

    let start = Instant::now();
    let result = sniff::detect_with_plan(plan).unwrap();
    let elapsed = start.elapsed();

    assert!(result.os.is_some());
    assert!(result.hardware.is_some());
    assert!(result.network.is_none());
    assert!(result.filesystem.is_none());

    // Summary mode should be significantly faster than full detection
    assert!(
        elapsed.as_millis() < 2000,
        "Summary detection took too long: {:?}",
        elapsed
    );
}
```

- [ ] **Step 7: Commit**

Commit: `feat(sniff-lib): add detect_with_plan() top-level API with DetectionPlan`

---

## Phase 3: Shared-Work Architecture

Restructure grouped collectors to share intermediate state and avoid redundant work.

---

### Task 13: Layered git status pipeline

**Review finding:** #3 - Split git status into counts, file stats, and patch detail so callers stop paying for unified diffs by default.

**Files:**
- Modify: `sniff/lib/src/filesystem/git.rs`

- [ ] **Step 1: Extract `get_repo_status_counts_detailed` (already done in Task 10)**

This was introduced in Task 10. Confirm it exists and is working.

- [ ] **Step 2: Extract `get_file_changes_without_diffs`**

Create a function that computes `FileChange` entries (status, action) without calling `get_file_diff_stats()`:

```rust
/// Gathers file change list with status and action but without line-level diff stats.
fn get_file_changes_without_diffs(repo: &Repository) -> Result<Vec<FileChange>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut file_changes = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = match entry.path() {
            Some(p) => PathBuf::from(p),
            None => continue,
        };

        let is_staged =
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_modified() || status.is_wt_deleted();
        let is_untracked = status.is_wt_new();

        if is_untracked {
            file_changes.push(FileChange {
                path,
                status: FileStatus::Untracked,
                action: FileAction::Created,
                lines_added: 0,
                lines_removed: 0,
            });
        } else if is_staged && is_unstaged {
            let action = if status.is_index_new() {
                FileAction::Created
            } else if status.is_index_deleted() {
                FileAction::Deleted
            } else {
                FileAction::Modified
            };
            file_changes.push(FileChange {
                path,
                status: FileStatus::Both,
                action,
                lines_added: 0,
                lines_removed: 0,
            });
        } else if is_staged {
            let action = if status.is_index_new() {
                FileAction::Created
            } else if status.is_index_deleted() {
                FileAction::Deleted
            } else {
                FileAction::Modified
            };
            file_changes.push(FileChange {
                path,
                status: FileStatus::Staged,
                action,
                lines_added: 0,
                lines_removed: 0,
            });
        } else if is_unstaged {
            let action = if status.is_wt_deleted() {
                FileAction::Deleted
            } else {
                FileAction::Modified
            };
            file_changes.push(FileChange {
                path,
                status: FileStatus::Modified,
                action,
                lines_added: 0,
                lines_removed: 0,
            });
        }
    }

    Ok(file_changes)
}
```

- [ ] **Step 3: Wire layers into `detect_with_request`**

The `detect_with_request` method (from Task 10) already selects between full status and counts-only. Add a middle tier:

```rust
// In detect_with_request:
let (status, file_changes) = if request.include_file_changes {
    // Full: status counts + per-file diff stats + dirty file diffs
    get_repo_status_with_changes(&self.repo)?
} else {
    // Lightweight: just counts
    let (is_dirty, staged, unstaged, untracked) =
        get_repo_status_counts_detailed(&self.repo);
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
```

A future `GitRequest` option could add an intermediate level that returns `FileChange` list without line counts, using `get_file_changes_without_diffs`. For now, the two-tier model (counts vs full) covers the main use cases.

- [ ] **Step 4: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `refactor(sniff-lib): layered git status pipeline with counts-only fast path`

---

### Task 14: Single manifest index for monorepo detection

**Review finding:** #4 - `detect_repo_inner()` walks each workspace package tree multiple times.

**Files:**
- Modify: `sniff/lib/src/filesystem/repo.rs`

- [ ] **Step 1: Create `ManifestIndex` struct**

A single-pass manifest scanner that collects all manifest files in the repo:

```rust
/// Pre-built index of all manifest files in a repository.
///
/// Built once via a single directory walk, then used to derive package
/// boundaries without re-scanning the filesystem per package.
struct ManifestIndex {
    /// All manifests found, keyed by parent directory
    manifests: HashMap<PathBuf, Vec<ManifestEntry>>,
}

struct ManifestEntry {
    path: PathBuf,
    kind: ManifestKind,
}

enum ManifestKind {
    CargoToml,
    PackageJson,
    PyprojectToml,
    GoMod,
}

impl ManifestIndex {
    /// Build the index with a single directory walk.
    fn build(repo_root: &Path) -> Self {
        let mut manifests: HashMap<PathBuf, Vec<ManifestEntry>> = HashMap::new();

        for entry in walkdir::WalkDir::new(repo_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !matches!(
                    name.as_ref(),
                    ".git" | "node_modules" | "target" | ".turbo" | "dist" | "build"
                )
            })
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let kind = match entry.file_name().to_string_lossy().as_ref() {
                "Cargo.toml" => ManifestKind::CargoToml,
                "package.json" => ManifestKind::PackageJson,
                "pyproject.toml" => ManifestKind::PyprojectToml,
                "go.mod" => ManifestKind::GoMod,
                _ => continue,
            };
            let parent = entry.path().parent().unwrap_or(repo_root).to_path_buf();
            manifests
                .entry(parent)
                .or_default()
                .push(ManifestEntry {
                    path: entry.path().to_path_buf(),
                    kind,
                });
        }

        Self { manifests }
    }

    /// Get all manifest directories for a given manifest kind.
    fn directories_for(&self, kind: ManifestKind) -> Vec<&Path> {
        self.manifests
            .iter()
            .filter(|(_, entries)| entries.iter().any(|e| std::mem::discriminant(&e.kind) == std::mem::discriminant(&kind)))
            .map(|(dir, _)| dir.as_path())
            .collect()
    }
}
```

- [ ] **Step 2: Integrate ManifestIndex into detect_repo_inner**

Replace the per-workspace-tool `discover_packages_from_manifests_in_tree()` calls with lookups against the pre-built index. The exact integration depends on how each workspace tool currently discovers packages - some use manifest walking, others use workspace config parsing.

The key change: build the `ManifestIndex` once at the start of `detect_repo_inner`, then pass it to workspace-specific discovery functions instead of having each one walk the tree independently.

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `perf(sniff-lib): single manifest index for monorepo package discovery`

---

### Task 15: Shared FileInventory with package projections

**Review finding:** #4 - `refresh_package_boundaries()` rescans each package tree with its own `MAX_FILES` budget.

**Files:**
- Modify: `sniff/lib/src/filesystem/repo.rs`
- Modify: `sniff/lib/src/filesystem/file_types/classify.rs`

- [ ] **Step 1: Add `project_package_inventory` function**

Instead of scanning each package directory independently, build one repo-level inventory and project package-level views:

```rust
/// Project a package-level summary from a shared repo inventory.
///
/// Filters classifications by path prefix to extract the subset
/// belonging to a specific package, avoiding a separate filesystem scan.
pub fn project_package_inventory(
    repo_inventory: &FileInventory,
    package_path: &Path,
    exclude_roots: &[PathBuf],
) -> FileInventory {
    let classifications: Vec<_> = repo_inventory
        .classifications
        .iter()
        .filter(|c| {
            c.path.starts_with(package_path)
                && !exclude_roots.iter().any(|ex| c.path.starts_with(ex))
        })
        .cloned()
        .collect();

    FileInventory {
        total_files_scanned: classifications.len(),
        classifications,
    }
}
```

- [ ] **Step 2: Update `refresh_package_boundaries` to use shared inventory**

Replace the per-package `scan_file_inventory_with_exclusions` calls with `project_package_inventory` calls against a single repo-level scan:

```rust
fn refresh_package_boundaries(
    packages: &mut [Package],
    repo_inventory: &FileInventory,  // NEW: shared inventory
) {
    for package in packages.iter_mut() {
        let nested_roots: Vec<PathBuf> = /* existing nested package logic */;
        let pkg_inventory = project_package_inventory(repo_inventory, &package.path, &nested_roots);
        let lang_summary = summarize_languages(&pkg_inventory);
        // Populate package language/framework fields from lang_summary
        // ...
    }
}
```

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `perf(sniff-lib): shared file inventory with package projections`

---

### Task 16: Shared PATH snapshot for program detection

**Review finding:** #8 - Each program category separately scans PATH and macOS app bundles.

**Files:**
- Modify: `sniff/lib/src/programs/find_program.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Create `PathSnapshot` struct**

```rust
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

/// Pre-built snapshot of all executables available on PATH.
///
/// Built once, then used for O(1) lookups instead of repeated
/// filesystem traversal per program.
pub struct PathSnapshot {
    /// Maps binary name to its resolved path
    executables: HashMap<String, PathBuf>,
}

impl PathSnapshot {
    /// Build the snapshot by scanning all PATH directories once.
    pub fn build() -> Self {
        let mut executables = HashMap::new();

        if let Some(path_var) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_var) {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if let Some(name) = entry.file_name().to_str() {
                            // First occurrence wins (matches PATH precedence)
                            executables
                                .entry(name.to_string())
                                .or_insert_with(|| entry.path());
                        }
                    }
                }
            }
        }

        Self { executables }
    }

    /// Look up a program by name.
    pub fn find(&self, program: &str) -> Option<&PathBuf> {
        self.executables.get(program)
    }
}
```

- [ ] **Step 2: Add snapshot-based lookup to find_program**

Add variants that accept a `PathSnapshot`:

```rust
pub fn find_program_in_snapshot(snapshot: &PathSnapshot, program: &str) -> Option<PathBuf> {
    snapshot.find(program).cloned()
}
```

- [ ] **Step 3: Wire into ProgramsInfo::detect()**

Build the snapshot once, pass it to each category detector:

```rust
impl ProgramsInfo {
    pub fn detect() -> Self {
        let snapshot = PathSnapshot::build();
        Self {
            editors: InstalledEditors::new_with_snapshot(&snapshot),
            utilities: InstalledUtilities::new_with_snapshot(&snapshot),
            // ... etc
        }
    }
}
```

Each `Installed*` type needs a `new_with_snapshot` constructor. This is a larger change - each category module needs updating. The existing `new()` constructors should remain as convenience methods that build their own snapshot.

- [ ] **Step 4: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `perf(sniff-lib): shared PATH snapshot for program detection`

---

### Task 17: Parallelize ProgramsInfo::detect across categories

**Review finding:** #8 - Categories are constructed sequentially despite being independent.

**Files:**
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Add rayon parallelization across categories**

After Task 16 provides the shared `PathSnapshot`, use `rayon::join` or `rayon::scope` to construct categories in parallel:

```rust
impl ProgramsInfo {
    pub fn detect() -> Self {
        let snapshot = std::sync::Arc::new(PathSnapshot::build());

        // Use rayon to parallelize category detection
        let (editors, utilities) = rayon::join(
            || InstalledEditors::new_with_snapshot(&snapshot),
            || InstalledUtilities::new_with_snapshot(&snapshot),
        );

        let (lang_pms, os_pms) = rayon::join(
            || InstalledLanguagePackageManagers::new_with_snapshot(&snapshot),
            || InstalledOsPackageManagers::new_with_snapshot(&snapshot),
        );

        let (tts, terminals) = rayon::join(
            || InstalledTtsClients::new_with_snapshot(&snapshot),
            || InstalledTerminalApps::new_with_snapshot(&snapshot),
        );

        let (headless, ai) = rayon::join(
            || InstalledHeadlessAudio::new_with_snapshot(&snapshot),
            || InstalledAiClients::new_with_snapshot(&snapshot),
        );

        Self {
            editors,
            utilities,
            language_package_managers: lang_pms,
            os_package_managers: os_pms,
            tts_clients: tts,
            terminal_apps: terminals,
            headless_audio: headless,
            ai_clients: ai,
        }
    }
}
```

- [ ] **Step 2: Update doc comment**

Now the doc comment accurately reflects parallel execution:

```rust
/// Detect all installed programs across all categories.
///
/// Builds a shared PATH snapshot once, then detects all 8 categories
/// in parallel using Rayon.
```

- [ ] **Step 3: Run tests and commit**

Run: `just test` from `sniff/lib/`
Commit: `perf(sniff-lib): parallelize program category detection with shared PATH snapshot`

---

## Phase 4: Documentation

Update architecture documentation and the Agent skill to reflect the new API.

---

### Task 18: Write sniff-library-architecture.md

**Files:**
- Create: `sniff/docs/sniff-library-architecture.md`

- [ ] **Step 1: Write the architecture document**

This document should describe:

1. **Design philosophy**: Callers get grouped results for convenience, but can control cost through request types. Three API tiers: `detect()` (defaults), `detect_with_plan()` (selective), module-level (expert).

2. **Detection domains**: OS, Hardware, Network, Filesystem (with Git, Repo, Docs sub-domains), Programs, Services. Each has a request type controlling detail level.

3. **Cost model**: Table showing approximate cost of each subsection:

| Domain | Subsection | Approximate Cost | Default |
|--------|-----------|-----------------|---------|
| OS | Core identity | <10ms | Always |
| OS | Package managers | 50-500ms (Linux) | Full only |
| OS | NTP status | up to 10s (Linux) | Full only |
| Hardware | CPU + Memory | <50ms | Always |
| Hardware | Audio devices | ~1.5s (macOS) | Full only |
| Hardware | Storage + GPU | ~100ms | Full only |
| Network | Interfaces | <10ms | Always |
| Network | WAN IP | 500ms-2s | Full only |
| Filesystem | Git summary | <50ms | Always |
| Filesystem | Git file changes | 50-500ms | Full only |
| Filesystem | Repo structure | <100ms | Always |
| Filesystem | Repo full scan | 200ms-5s | Full only |
| Programs | All categories | 200-800ms | Separate API |

4. **Shared-work architecture**: How grouped collectors share intermediate state:
   - Manifest index: single walk, package projections
   - File inventory: single scan, per-package views
   - PATH snapshot: single scan, all category lookups
   - Git status layers: counts → file list → diff stats → patches

5. **Request type quick reference**: Builder patterns for common caller profiles (CI tool, IDE plugin, compose context, full audit).

- [ ] **Step 2: Review and commit**

Commit: `docs(sniff): add library architecture document`

---

### Task 19: Update the sniff Agent Skill

**Files:**
- Modify: `.claude/skills/sniff/SKILL.md`

- [ ] **Step 1: Rewrite SKILL.md to cover the new API**

The skill should cover:

1. **Quick Start** (updated with `detect_with_plan` examples)
2. **API Tiers**: `detect()`, `detect_with_plan()`, module-level
3. **Request Types**: Table of all request types with their detail levels
4. **Common Patterns**:
   - Fast context gathering (Claudine-style): `OsRequest::summary()` + `HardwareRequest::summary()` + `GitRequest::summary()`
   - Demand-driven (Darkmatter-style): module-level calls with `GitRepo::discover()`
   - Full audit (CLI-style): `DetectionPlan::default()` with `GitRequest::deep()`
5. **CLI** section (existing, update if needed)
6. **Key Types** (updated to include request types)
7. **Links** to detailed docs (programs.md, services.md, extending.md, architecture.md)

Keep within the <200 line limit for SKILL.md entry points.

- [ ] **Step 2: Commit**

Commit: `docs(sniff): update Agent skill for plan-based API`

---

## Phase 5: Client Optimization Review (Research Only)

Review each library consumer to identify optimization opportunities unlocked by the new API. **This phase produces a suggestions document, not code changes.**

---

### Task 20: Audit sniff-cli usage

**Files to read:**
- `sniff/cli/src/main.rs` (full file, especially lines 372-821)
- `sniff/cli/src/args.rs`

**What to look for:**
- Places where the CLI uses `SniffConfig` skip flags that could be replaced with targeted `DetectionPlan` requests
- Subcommands that only need a subset of data (e.g., `sniff hardware` doesn't need full git detection)
- The `OutputFilter`-based skip pattern (line 372-445) - can this be replaced with purpose-built plans?
- Whether `ProgramsInfo::detect()` (called separately) could benefit from the shared PATH snapshot
- Whether `detect_services()` (called separately) should be integrated into `DetectionPlan`
- Dependency enrichment (lines 513-821) - is there a way to make this opt-in through the plan?

Document findings in the suggestions file.

---

### Task 21: Audit darkmatter usage

**Files to read:**
- `darkmatter/lib/src/markdown/compose/context/capture.rs`

**What to look for:**
- Darkmatter already uses atomic functions (`GitRepo::discover`, `detect_repo_structure`, `detect_hardware_summary`). Does the new plan API offer a cleaner alternative?
- The demand-driven `scan_needed_groups` pattern (line 546) - does the new `FilesystemRequest` builder give a cleaner way to express "git summary + repo structure + docs but no inventory"?
- Parallel execution pattern (lines 302-446 using `std::thread::scope`) - can independent captures benefit from request types?
- `detect_os()` is called with full detail - would `OsRequest::summary()` suffice for compose context?
- `detect_hardware_summary()` is already used - confirm this maps cleanly to `HardwareRequest::summary()`
- `detect_gpus()` is called separately from hardware - should `HardwareRequest` support a GPU-only mode?

---

### Task 22: Audit claudine usage

**Files to read:**
- `claudine/lib/src/events/environment.rs` (lines 295-327)
- `claudine/lib/src/system_prompt/context.rs`
- `claudine/lib/src/config/mod.rs`

**What to look for:**
- Two detection modes: standard (lines 295-310) and fast (lines 320-327). Map these to `DetectionPlan`:
  - Standard: `OsRequest::full()` + `HardwareRequest::full()` + no network + full filesystem with `commit_count(1)` and `deep(false)`
  - Fast: filesystem only with `GitRequest::summary().commit_count(0)` + no repo? no docs?
- `system_prompt/context.rs` calls `detect_git()` and `detect_repo()` directly - could these be replaced with a single `detect_with_plan()` call?
- AI client detection via `InstalledAiClients::new()` - any benefit from shared PATH snapshot?
- The `From<SniffResult>` conversion (lines 182-289) - does it need updating for optional subsections?

---

### Task 23: Audit playa, unchained-ai, and research usage

**Files to read:**
- `playa/lib/src/player.rs`
- `research/lib/src/metadata/topic.rs`

**What to look for:**
- **Playa**: Only uses `HeadlessAudio` enum metadata. No detection calls. No optimization needed, but document this as a good example of the "metadata-only" consumer pattern.
- **Unchained-AI**: No sniff dependency found. Confirm and document.
- **Research**: Only uses `LanguagePackageManager` enum for type safety. No detection calls. No optimization needed.

---

### Task 24: Write sniff-client-suggestions.md

**Files:**
- Create: `sniff/features/2026-04-01-ergonomic-and-performant/sniff-client-suggestions.md`

- [ ] **Step 1: Compile findings from Tasks 20-23 into the suggestions document**

Structure:

```markdown
# Sniff Client Optimization Suggestions

Generated from client audit following the ergonomics & performance refactoring.

## Sniff CLI

### Current State
- Uses SniffConfig with OutputFilter-based skip flags
- Calls ProgramsInfo::detect() and detect_services() separately
- Dependency enrichment is always-on when deep mode is used

### Suggested Changes
1. [specific suggestions from Task 20]
2. ...

## Darkmatter Library

### Current State
- Demand-driven context capture using atomic functions
- Already uses detect_repo_structure() and detect_hardware_summary()

### Suggested Changes
1. [specific suggestions from Task 21]
2. ...

## Claudine Library

### Current State
- Two detection modes (standard and fast)
- Direct function calls alongside SniffConfig

### Suggested Changes
1. [specific suggestions from Task 22]
2. ...

## Playa, Unchained-AI, Research

### Current State
- Metadata-only and type-only consumers
- No runtime detection calls

### Suggested Changes
- No changes needed. These consumers are already optimal.
```

- [ ] **Step 2: Commit**

Commit: `docs(sniff): add client optimization suggestions from ergonomics review`

---

## Dependency Graph

```
Phase 1 (independent, can parallelize):
  Task 1 ──┐
  Task 2 ──┤
  Task 3 ──┼── All independent, merge after all complete
  Task 4 ──┤
  Task 5 ──┘

Phase 2 (sequential within, depends on Phase 1):
  Task 6 ──→ Task 7 ──┐
             Task 8 ──┤
             Task 9 ──┼── Tasks 7-11 depend on Task 6 (request types)
             Task 10 ─┤    but are independent of each other
             Task 11 ─┘
                  └──→ Task 12 (depends on all of 7-11)

Phase 3 (depends on Phase 2 for request types):
  Task 13 ─── depends on Task 10 (GitRequest integration)
  Task 14 ─── independent (repo module only)
  Task 15 ─── depends on Task 14 (uses ManifestIndex)
  Task 16 ─── independent (programs module only)
  Task 17 ─── depends on Task 16 (uses PathSnapshot)

Phase 4 (depends on Phases 1-3):
  Task 18 ─── after all implementation tasks
  Task 19 ─── after all implementation tasks

Phase 5 (depends on Phase 4):
  Task 20 ──┐
  Task 21 ──┤
  Task 22 ──┼── All independent, can parallelize
  Task 23 ──┘
       └──→ Task 24 (compile findings from 20-23)
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Request types add API surface complexity | Callers confused by too many options | Keep `detect()` and `detect_with_config()` as simple defaults; request types are opt-in |
| Shared PathSnapshot may not be faster than per-binary `which` | Wasted effort | Benchmark first; PATH scan on typical systems (10-20 dirs) should be <50ms total |
| ManifestIndex single-pass may miss edge cases in workspace detection | Wrong package discovery | Keep existing workspace config parsing; ManifestIndex only replaces the tree-walking step |
| Breaking changes to public types | Downstream compile errors | All changes are additive; `SniffConfig` remains functional via `From<SniffConfig> for DetectionPlan` |
| `RepoStatus` now has empty `dirty`/`untracked` vecs in lightweight mode | Callers that assumed these are populated | Document that these are empty when `include_file_changes` is false; add `has_dirty_details()` method |

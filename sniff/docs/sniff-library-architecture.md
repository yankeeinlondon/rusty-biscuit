## Overview

The sniff library detects system information across four plan-driven domains (OS, hardware, network, filesystem) plus two standalone APIs (programs, services). Every subsection has its own cost profile -- some operations complete in under 10ms while others require seconds of I/O or network access. The library's architecture gives callers three distinct ways to interact with it, arranged from simplest to most flexible, so that every caller gets exactly the data they need without paying for operations they don't.

## Three API Tiers

### Tier 1: Convenience (`detect()`)

A single function call that returns everything at default detail levels. This is the right choice when you want comprehensive system context and don't mind the latency.

```rust
use sniff::detect;

let result = detect()?;
// result.os, result.hardware, result.network, result.filesystem
```

Under the hood, `detect()` builds a `SniffConfig::default()`, converts it into a `DetectionPlan`, and delegates to `detect_with_plan`. The net effect is the same as `DetectionPlan::default()`: all four domains at full detail.

### Tier 2: Plan-Based (`detect_with_plan(DetectionPlan)`)

When you know which domains you need and at what detail level, build a plan. Each domain can be included at a specific detail level or excluded entirely. The plan is also serializable, so callers can accept it as JSON configuration.

```rust
use sniff::{detect_with_plan, request::*};

let plan = DetectionPlan::new()
    .os(OsRequest::summary())
    .hardware(HardwareRequest::summary())
    .without_network()
    .filesystem(
        FilesystemRequest::new()
            .git(GitRequest::summary())
            .repo(RepoRequest::structure())
            .without_docs()
    );

let result = detect_with_plan(plan)?;
```

### Tier 3: Module-Level Functions

For callers who want to compose their own detection pipeline -- calling specific modules directly, combining results manually, or integrating a single detection domain into a larger workflow.

```rust
use sniff::os::detect_os_with_request;
use sniff::hardware::detect_hardware_summary;
use sniff::filesystem::git::{GitRepo, detect_git_with_request};
use sniff::filesystem::repo::detect_repo_structure;
use sniff::request::*;

// Pick exactly what you need
let os = detect_os_with_request(&OsRequest::summary())?;
let hw = detect_hardware_summary()?;
let git = detect_git_with_request(path, &GitRequest::full().commit_count(5))?;
let repo = detect_repo_structure(path)?;
```

This tier is how downstream libraries like darkmatter compose context without pulling in the full `SniffResult` structure.

## Detection Domains and Cost Model

Every subsection has a real-world cost. The table below captures approximate latencies and whether each subsection is included in the default plan. Callers should use this to decide which detail level to request.

| Domain | Subsection | Approx. Cost | Default |
|--------|-----------|:------------:|:-------:|
| **OS** | Core identity (type, name, version, kernel, hostname) | <10ms | Yes |
| **OS** | Package managers | 50-500ms (Linux) | Yes |
| **OS** | Locale | <5ms | Yes |
| **OS** | Timezone + DST + UTC offset | <5ms | Yes |
| **OS** | NTP synchronization status | up to 10s (Linux `timedatectl`) | Yes |
| **Hardware** | CPU + Memory | <50ms | Yes |
| **Hardware** | Storage devices | ~50ms | Yes |
| **Hardware** | GPU detection | ~50ms | Yes |
| **Hardware** | Audio devices | ~1.5s (macOS CoreAudio) | Yes |
| **Network** | Local interfaces | <10ms | Yes |
| **Network** | WAN IP (HTTP call, TTL-cached) | 500ms-2s cold, <1ms warm | Yes |
| **Filesystem** | Git identity (root, branch, HEAD, worktree flag, org/repo) | <10ms | No (use `GitRequest::identity()`) |
| **Filesystem** | Git summary (branch + dirty flag) | <50ms | Yes |
| **Filesystem** | Git file changes (paths + line stats) | 50-500ms | Yes |
| **Filesystem** | Git file diffs (full unified diffs) | 100ms-1s | No (deep only) |
| **Filesystem** | Git remote refresh (branches, behind, containment) | 1-5s (network) | No |
| **Filesystem** | Repo structure (workspace tools + package list) | <100ms | Yes |
| **Filesystem** | Repo full (per-package languages, frameworks) | 200ms-5s | Yes |
| **Filesystem** | File inventory (classification + languages) | 50-500ms | Yes |
| **Filesystem** | EditorConfig | <10ms | Yes |
| **Filesystem** | Markdown documents | 50-200ms | Yes |
| **Programs** | All 9 categories (parallel, shared executable index) | 200-800ms | Separate API |
| **Services** | Init system detection + service list | 50-200ms | Separate API |

Programs and Services are not part of `DetectionPlan` because they have no dependency on the filesystem base directory and their results are system-global rather than project-scoped. They are accessed through `ProgramsInfo::detect()` and `ServiceManager::detect()` respectively.

## Request Types

Each detection domain has a dedicated request type with named constructors for common presets and builder methods for fine-tuning individual flags.

### OsRequest

| Constructor | What it includes |
|------------|-----------------|
| `summary()` | Core identity only: OS type, name, version, kernel, hostname |
| `full()` | Everything: adds package managers, locale, timezone/NTP |

### HardwareRequest

| Constructor | What it includes |
|------------|-----------------|
| `summary()` | CPU and memory only (~1.5s savings on macOS by skipping audio) |
| `full()` | Everything: adds storage, GPU, audio devices |

### NetworkRequest

| Constructor | What it includes |
|------------|-----------------|
| `interfaces_only()` | Local interfaces. No external HTTP call |
| `full()` | Adds WAN IP lookup via external service (TTL-cached) |

WAN IP results are cached per run; call `.force_refresh(true)` to bypass the cache and issue a fresh HTTP lookup.

### GitRequest

| Constructor | What it includes |
|------------|-----------------|
| `identity()` | Repo root, current branch, HEAD id, worktree flag, base repo root, and org/repo from the preferred remote. **No working-tree status walk**, no commits, no branches, no remotes, no config. This is the cheapest git request level and the new floor below `minimal()`/`summary()`. |
| `summary()` | Repo root, current branch, and a dirty yes/no flag. No per-category counts, no commits, no file details, no worktrees. **Currently byte-identical to `minimal()`** |
| `minimal()` | Same field set as `summary()` (repo root, branch, dirty flag) |
| `full()` | 10 commits, per-file change stats (paths + line counts), worktrees. No unified diff payloads, no network |
| `deep()` | Everything in `full()` plus full unified diffs for dirty and untracked files, remote tracking refresh, remote branch details, and per-commit remote containment |

All constructors return a builder, so you can further adjust with methods like `.commit_count(5)`, `.include_worktrees(false)`, `.include_file_diffs(true)`, or `.refresh_remote_tracking(false)`.

### RepoRequest

| Constructor | What it includes |
|------------|-----------------|
| `structure()` | Workspace tools and package list only. 10-50x faster than full |
| `full()` | Per-package language scanning, framework detection, file associations |

### FilesystemRequest

Composes sub-requests for git, repo, file inventory, formatting, and document discovery. Each sub-domain can be individually included or excluded.

```rust
let fs_request = FilesystemRequest::new()
    .git(GitRequest::full().commit_count(5))
    .repo(RepoRequest::structure())
    .without_file_inventory()
    .without_docs();
```

## Shared-Work Architecture

Several detection paths would naturally duplicate expensive I/O if implemented naively. The library uses six internal strategies to avoid this.

### Top-Level Domain Concurrency

`detect_with_plan` runs the four top-level domains (OS, hardware, network, filesystem) concurrently using `std::thread::scope`. Each domain has its own scoped thread with a tracing span, and the results are joined before assembling the final `SniffResult`. Domains are independent and have no shared state, so there is no ordering constraint.

### Staged Filesystem Detection

Within the filesystem domain, `detect_filesystem_with_request` runs a **concurrent prelude** followed by a **sequential reuse phase**, so the working tree is walked at most once and every later stage projects off pre-computed inputs.

**Concurrent prelude** (`std::thread::scope`): up to three workers run in parallel, gated by the request —

1. **Git** (`detect_git_with_request`): discovers the repo root, used to retarget the repo/inventory/docs stages at the actual repo rather than the caller's cwd.
2. **Formatting** (`detect_formatting`): cheap EditorConfig lookup.
3. **Shared filesystem view** (`build_filesystem_system_view`): a single `ignore`-based parallel directory walk that collects exactly what the request needs — manifests, file inventory, and/or docs — selected by `SharedWalkOptions`.

**Sequential reuse phase** (after the prelude joins):

4. **Repo** (`detect_repo_inner_with_shared`): consumes the shared view's manifest index and inventory, so it never re-walks the tree.
5. **File inventory + languages**: reuses the shared view's inventory directly, or projects a package-scoped slice from it (`filter_inventory`) with sibling-package exclusions; falls back to a fresh `scan_file_inventory*` only when no shared inventory was collected.
6. **Docs**: reuses the shared view's markdown set and enriches it with package assignment from the repo stage.

The shared view is itself internally parallel: one `ignore::WalkBuilder::build_parallel()` pass produces manifests, file classifications, and docs together, with per-worker buffers flushed into a shared accumulator on drop.

### Manifest Index

When full repo detection runs, it builds a `ManifestIndex` from a single filesystem walk. This index records every `Cargo.toml`, `package.json`, `pyproject.toml`, and `go.mod` found in the tree. Package boundaries and their ecosystems are then derived entirely from this index rather than walking the tree again per workspace tool.

`detect_repo_structure` skips the manifest index entirely, since it only needs workspace-declared package lists. This is the 10-50x speedup path used by `RepoRequest::structure()`.

### File Inventory Projection

The file-tree walk runs once inside the shared filesystem view and is reused by every stage that needs it. When the caller's base directory falls within a specific package, the inventory stage produces a package-scoped view by filtering the shared inventory with path-prefix exclusions for sibling packages (`filter_inventory`). A single walk serves both the overall language breakdown and any package-scoped view.

### Git Status Layers

Git status collection has four code paths selected by the request (`GitRepo::detect_with_request`):

- **Identity only** (`is_identity_only()`): returns repo root, current branch, HEAD id, worktree flag, base repo root, and org/repo. This path performs **no working-tree status walk** and is the cheapest way to obtain repository identity through the plan API.
- **Dirty flag only** (`is_minimal()` — no commits, no file changes, no worktrees, no remote refresh; this is what `summary()` and `minimal()` request): one gix status walk that resolves only whether the tree is dirty. The per-category `staged_count` / `unstaged_count` / `untracked_count` fields are left at `0`.
- **Counts** (`include_file_changes: false` but not minimal — e.g. a request that also asks for commits or worktrees): one status walk that populates the staged/unstaged/untracked counts, without per-file detail (`get_repo_status_counts_detailed`).
- **Full status** (`include_file_changes: true`): per-file change details including delta kind and line-level diff stats. An additional `include_file_diffs` flag further opts into full unified diff payloads (used by `deep()`).

All status-bearing paths share the same `gix` repository handle opened once by `GitRepo::discover`. The identity-only path is the exception: it returns repository identity without scanning the working tree, and is the only request level below `summary()`/`minimal()`. For bare repo-root access without even branch/HEAD resolution, use the Tier-3 `GitRepo::discover().repo_root()` handle directly.

A **bare repository** (no working directory) discovers successfully — its refs, HEAD, and objects are all readable, so `current_branch()`, `head_id()`, and commit queries stay valid. `GitRepo::is_bare()` reports the distinction, `repo_root()` falls back to the git directory, `try_current_worktree_name()` is `None`, and `merge_conflicts()` is empty (no index). Worktree-dependent APIs (`collect_changed_paths`, `detect_repo_identity_with_repo`) reject a bare repository with `NotARepository` rather than walking the git directory as if it were a source tree.

### Actual and Predicted Merge Conflicts

Sniff deliberately separates live index observation from branch-tip prediction:

- `merge_conflicts_at(path)` returns the sorted repository-relative paths in
  non-zero live-index stages. It observes an in-progress merge, rebase,
  cherry-pick, or revert and returns an empty list outside a repository.
- `merge_conflicts_with_branch_at(path, incoming_branch)` predicts the paths
  produced by merging the exact named local branch (`theirs`) into the attached
  current local branch (`ours`). Missing repository or branch prerequisites are
  errors rather than clean results.

Both prediction and `WorktreeEntry::has_conflicts` use the same commit-pair
merge authority. The probe enables `gix` object-memory storage before merging,
builds its temporary index and attribute stack from the captured `ours` tree,
collects unresolved temporary-index stages, and leaves refs, the live index,
worktree, and on-disk object database unchanged. Applicable external merge
drivers, filters, and renormalization are rejected; no hook, subprocess, fetch,
credential helper, or network operation is invoked. The required plumbing is
re-exported by `gix`, so this boundary adds no direct dependency.

### Parallel Program Detection with Shared Executable Index

`ProgramsInfo::detect()` first builds a single `ExecutableIndex` by scanning every `PATH` directory and macOS app bundle location once. All 8 program categories then share this index via `Arc` and perform O(1) HashMap lookups instead of redundant filesystem traversals. The categories themselves run in parallel using `rayon::join` pairs (editors+utilities, language+OS package managers, TTS+terminals, headless audio+AI clients). Rayon handles the nested parallelism correctly, distributing work across its thread pool without spawning excess threads.

The net effect: the PATH scan runs once instead of eight times, macOS bundle detection runs once instead of eight times, and per-program lookups are HashMap hits rather than `which` invocations.

On Windows, `ExecutableIndex::build()` also populates a `WindowsIndex` holding two HashMaps: `app_paths` (from a HKLM+HKCU registry walk) and `install_roots` (from a one-level directory walk of `Program Files`, `Program Files (x86)`, and `LocalAppData\Programs`). Warm-cache cost: 40–80 ms serial, inside the existing build-once budget. Non-Windows builds do not compile this code.

## Common Caller Profiles

### CI Tool (Fast Context)

Needs OS and hardware identity for build metadata, basic git status, and the package list. No audio devices, no WAN IP, no per-file diffs.

```rust
let plan = DetectionPlan::new()
    .os(OsRequest::summary())
    .hardware(HardwareRequest::summary())
    .without_network()
    .filesystem(FilesystemRequest::new()
        .git(GitRequest::summary())
        .repo(RepoRequest::structure())
        .without_file_inventory()
        .without_docs());
```

Estimated cost: **<200ms**

### IDE Plugin (Session Startup)

Needs git context with recent commits and file changes, plus repo structure. Hardware and network are irrelevant.

```rust
let plan = DetectionPlan::new()
    .without_hardware()
    .without_network()
    .filesystem(FilesystemRequest::new()
        .git(GitRequest::full().commit_count(5))
        .repo(RepoRequest::structure())
        .without_file_inventory());
```

Estimated cost: **200-500ms**

### Full Audit (CLI Default)

The `sniff` CLI's default invocation. Returns everything at full detail.

```rust
let result = detect()?;  // equivalent to DetectionPlan::default()
```

Estimated cost: **2-5s** (dominated by audio detection and WAN IP lookup)

### Library Composition (darkmatter-style)

Downstream libraries that need only a few specific pieces. Uses module-level functions directly, bypassing `SniffResult` entirely.

```rust
use sniff::filesystem::git::GitRepo;
use sniff::filesystem::repo::detect_repo_structure;
use sniff::hardware::detect_hardware_summary;

let git = GitRepo::discover(path)?;
let repo = detect_repo_structure(path)?;
let hw = detect_hardware_summary()?;
```

Estimated cost: **varies by what's called**

## Legacy API

`SniffConfig` predates the plan-based API and uses boolean skip flags and a flat `deep` flag. It remains supported through a `From<SniffConfig> for DetectionPlan` conversion. New code should prefer `DetectionPlan` directly.

```rust
// Legacy style (still works)
let config = SniffConfig::new()
    .skip_network()
    .deep(true)
    .commit_count(5);
let result = detect_with_config(config)?;

// Equivalent plan-based style (preferred)
let plan = DetectionPlan::new()
    .without_network()
    .filesystem(FilesystemRequest::new()
        .git(GitRequest::deep().commit_count(5)));
let result = detect_with_plan(plan)?;
```

## Design Rationale

**Why request types instead of feature flags?** A flat set of booleans doesn't compose. `FilesystemRequest` nests `GitRequest` and `RepoRequest` because filesystem detection genuinely depends on git and repo results (e.g., repo root comes from git discovery, doc detection uses package boundaries). The nesting reflects real data flow.

**Why named constructors?** `summary()`, `full()`, and `deep()` encode the most common usage patterns as discoverable entry points. Callers start from a preset and tune with builder methods rather than constructing from scratch, which prevents "forgot to set a flag" mistakes.

**Why are Programs and Services separate?** They detect globally-installed software, not project-scoped data. Including them in `DetectionPlan` would be misleading since `base_dir` has no effect on their results. They also have no dependency on any other domain's output, so there is no shared-work benefit from co-scheduling them.

**Why `Option<T>` for domain results?** A `None` in `SniffResult.hardware` means "not requested," not "detection failed." This lets callers distinguish between skipped domains and domains that returned empty data, which matters for serialization and downstream display logic.

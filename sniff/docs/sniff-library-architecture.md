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
| **Filesystem** | Git summary (branch, dirty counts) | <50ms | Yes |
| **Filesystem** | Git file changes (paths + line stats) | 50-500ms | Yes |
| **Filesystem** | Git file diffs (full unified diffs) | 100ms-1s | No (deep only) |
| **Filesystem** | Git remote refresh (branches, behind, containment) | 1-5s (network) | No |
| **Filesystem** | Repo structure (workspace tools + package list) | <100ms | Yes |
| **Filesystem** | Repo full (per-package languages, frameworks) | 200ms-5s | Yes |
| **Filesystem** | File inventory (classification + languages) | 50-500ms | Yes |
| **Filesystem** | EditorConfig | <10ms | Yes |
| **Filesystem** | Markdown documents | 50-200ms | Yes |
| **Programs** | All 8 categories (parallel, shared executable index) | 200-800ms | Separate API |
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
| `summary()` | Repo root, current branch, dirty status counts. No commits, no file details, no worktrees |
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

Within the filesystem domain, `detect_filesystem_with_request` runs five stages sequentially so each can reuse work from the previous:

1. **Git**: Discovers the repo root (used to retarget subsequent stages at the actual repo rather than the caller's cwd).
2. **Repo**: Returns both the `RepoInfo` and its internal `FileInventory` via `detect_repo_with_inventory`, so Stage 3 can skip its own walk.
3. **File inventory + languages**: Reuses the repo-level inventory when available, optionally filtered to a package scope with sibling-package exclusions.
4. **Formatting (EditorConfig)**: Cheap local lookup.
5. **Docs**: Markdown discovery, informed by the package list from Stage 2 so doc-to-package association is accurate.

Each stage is strictly cheaper because the previous stage pre-computed its expensive inputs.

### Manifest Index

When full repo detection runs, it builds a `ManifestIndex` from a single filesystem walk. This index records every `Cargo.toml`, `package.json`, `pyproject.toml`, and `go.mod` found in the tree. Package boundaries and their ecosystems are then derived entirely from this index rather than walking the tree again per workspace tool.

`detect_repo_structure` skips the manifest index entirely, since it only needs workspace-declared package lists. This is the 10-50x speedup path used by `RepoRequest::structure()`.

### File Inventory Projection

The file inventory scan runs once at the repo level inside full repo detection and is threaded back out via `detect_repo_with_inventory`. When the caller's base directory falls within a specific package, Stage 3 produces a package-scoped view by filtering the repo-level scan with path-prefix exclusions for sibling packages. A single walk serves both the overall language breakdown and any package-scoped view.

### Git Status Layers

Git status collection has two code paths selected by the request:

- **Counts-only** (`include_file_changes: false`): Walks the libgit2 status list once, incrementing staged/unstaged/untracked counters. Used for worktree summaries and the `summary()` preset.
- **Full status** (`include_file_changes: true`): Collects per-file change details including delta kind and line-level diff stats. Only paid for when the caller actually needs file-level data. An additional `include_file_diffs` flag further opts into full unified diff payloads (used by `deep()`).

Both paths share the same `libgit2` repository handle opened once by `GitRepo::discover`.

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

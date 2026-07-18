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
| **OS** | NTP synchronization status | bounded at 3s; network round trip on macOS only | **No** (opt in via `OsRequest`) |
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

`DetectionPlan::default()` is "all domains at safe defaults", **not** every domain at `full()`: it
builds its OS request as `OsRequest::full().include_ntp_status(false)`, so Tier-1 `detect()`
initiates no implicit network probe. `OsRequest::full()` itself is unchanged — an explicit caller
still gets NTP. Every NTP probe is bounded at 3 seconds (`process::timeouts::NTP`); only the macOS
`sntp` path actually contacts a time server, while Linux `timedatectl` and Windows `w32tm` report
local daemon state.

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
| `structure()` | Workspace topology and minimum package identity only |
| `focused(details)` | Structure plus selected package managers, dependencies, or test runners |
| `full()` | Per-package language scanning, framework detection, file associations |

`structure()` leaves package managers, dependencies, test runners, features, languages, frameworks,
and file lists empty. Callers that need only one manifest-backed fact can use
`RepoRequest::focused(RepoDetailRequest::…)` without enabling the inventory-backed work in `full()`.

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

Several detection paths would naturally duplicate expensive I/O if implemented naively. The library
avoids this under one governing rule: **observe once, project many times**. A request acquires each
expensive host fact once, retains only the compact evidence that request needs, and projects its
public results from that evidence. Reusable state is scoped to the request and never outlives it —
the WAN IP TTL cache is the sole documented exception. The CLI is a renderer; it never rediscovers
host or repository state.

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

### Request-Scoped Observation Index

`FilesystemSystemView` is the request-scoped observation index. A **full** repo detection observes
the tree **once** and projects everything from that one pass. `SharedWalkOptions` gates what the
index retains: the manifest index, the unfiltered manifest directories, nested-workspace markers,
the capped inventory, and Markdown metadata. It never retains `DirEntry` values or file bodies, and
there is no process-global cache.

Both full routes share it. Standalone `detect_repo`/`detect_repo_with_inventory` and integrated
`WalkScope::Repository` build the same index and return equivalent results; `RepoEvidence::from_view`
is the only constructor from a walk, so the evidence and the root it came from cannot drift apart.
Before this, standalone detection walked the tree **three** times (manifest index, nested markers,
inventory) and the integrated route walked twice.

`manifest_dirs` and `manifest_index` are deliberately different collections. `ManifestIndex` drops
generated and fixture manifests because they are not discovery boundaries, but membership **globs**
resolve a boundary by marker presence alone and never applied that exclusion — so globs consume the
unfiltered `manifest_dirs`. Routing them through the filtered index would silently drop a member
whose manifest is marked auto-generated, and only in full mode, making structure and full disagree.

Evidence **presence is proof; absence is not.** Because the index omits generated and fixture
manifests, an empty evidence set means "nothing observed", never "no manifest exists". A
probe-skipping fast path is sound only for kinds known *present*.

Specialized fallback walks are named by their own counters (`filesystem.repo.nested_marker_walks`,
`filesystem.repo.membership_glob_walks`) rather than hidden inside `filesystem.io.read_dirs`, which
five unrelated detectors share.

### Structure-Only Contract

`detect_repo_structure` skips the manifest index entirely (`detection.rs` builds it only when
`!structure_only`), since it only needs workspace-declared package lists. In exchange it pays a
nested-marker fallback walk (`filesystem.repo.nested_marker_walks: 1`) that full detection avoids by
reusing the observation index — which is why structure mode is not simply "full minus work".

Structure mode keeps the smallest evidence set on purpose: it consumes no index, inventory, or docs,
so building the observation index for it would classify every file for evidence it then discards.

`RepoRequest::structure()` stops after membership and minimum package identity. Package managers,
dependencies, test runners, features, frameworks, languages, and file lists remain absent or empty.
The focused `sniff repo package-manager`, `dependencies`, and `test-runner` commands opt into their
corresponding `RepoDetailRequest` instead of relying on accidentally enriched structure results.

### Package Discovery vs. Enrichment

Discovery finishes before enrichment begins. Detectors return cheap `PackageSeed` values (`repo/seed.rs`)
— a normalized key, the `owner_root` the boundary was resolved against, the owning standard,
provenance, and matched evidence kinds — and `merge_seeds` collapses duplicate boundaries **before**
`create_package` runs. Enrichment therefore happens exactly once per unique boundary. Two detectors
resolving the same directory is normal; what was a defect was paying full enrichment for each and
merging afterwards.

`PackageSeed::owner_root` is load-bearing: enrichment runs in the frame the *detector* resolved the
boundary in, because Cargo `version.workspace = true`, npm's root-version fallback, and `Cargo.lock`
resolution all resolve against the owning workspace's root — not the repo root. Only `relative` and
`package_area` are re-framed to the repo-root catalog view. A nested Cargo workspace enriched
against the outer root would report the wrong inherited versions.

### File Inventory Projection

The file-tree walk runs once inside the shared filesystem view and is reused by every stage that needs it. When the caller's base directory falls within a specific package, the inventory stage produces a package-scoped view by filtering the shared inventory with path-prefix exclusions for sibling packages (`filter_inventory`). A single walk serves both the overall language breakdown and any package-scoped view.

When no narrowing is required, `filter_inventory` shares the source's classifications through the
`Arc` rather than copying them; any narrowing filter necessarily copies the retained classifications.

### Inventory Saturation

Classification stops at `MAX_FILES` (10,000 accepted). An inventory-**only** walk quits globally at
the cap. A combined walk keeps going for its still-active manifest, marker, or docs observers —
quitting would silently truncate the manifest index — but stops classification and its counters.

`FileInventory`, `FileAssociationBreakdown`, and `LanguageSummary` each carry `truncated: bool` and
`limit: Option<usize>`, Serde-defaulted and omitted when complete, so existing JSON consumers are
unaffected. When truncated, `limit` is the accepted-classification cap. Every public projection
reports the same completeness state.

`total_files_scanned`/`total_files` count the classifications represented in the result, not the
tree's actual file count. When truncated, callers use the new fields to tell the represented count
from a complete one.

**A truncated subset is unspecified across runs.** Parallel workers race for the bounded slots, so
which paths win is not a contract; making it one would require enumerating and ordering the whole
tree, defeating the early termination the cap exists for. Ordering is always sorted, and a complete
result (`truncated == false`) is fully deterministic for an unchanged tree. Tests for truncated runs
assert the cap, the flags, ordering, and path validity — never exact selected-path equality.

### Git Status Layers

Git status collection has four code paths selected by the request (`GitRepo::detect_with_request`):

- **Identity only** (`is_identity_only()`): returns repo root, current branch, HEAD id, worktree flag, base repo root, and org/repo. This path performs **no working-tree status walk** and is the cheapest way to obtain repository identity through the plan API.
- **Dirty flag only** (`is_minimal()` — no commits, no file changes, no worktrees, no remote refresh; this is what `summary()` and `minimal()` request): one gix status walk that resolves only whether the tree is dirty. The per-category `staged_count` / `unstaged_count` / `untracked_count` fields are left at `0`.
- **Counts** (`include_file_changes: false` but not minimal — e.g. a request that also asks for commits or worktrees): one status walk that populates the staged/unstaged/untracked counts, without per-file detail (`get_repo_status_counts_detailed`).
- **Full status** (`include_file_changes: true`): per-file change details including delta kind and line-level diff stats. An additional `include_file_diffs` flag further opts into full unified diff payloads (used by `deep()`).

All status-bearing paths share the same `gix` repository handle opened once by `GitRepo::discover`. The identity-only path is the exception: it returns repository identity without scanning the working tree, and is the only request level below `summary()`/`minimal()`. For bare repo-root access without even branch/HEAD resolution, use the Tier-3 `GitRepo::discover().repo_root()` handle directly.

One `StatusContext` per status call holds the index, HEAD tree, worktree, object cache, and reusable
diff resources — previously these were resolved once per file *per side*. Each staged or unstaged
side loads its blob or worktree file **once** and runs **one** diff; both the line statistics and the
optional unified hunks are derived from that single diff result. Whole-file adds and deletes are
line-counted rather than diffed, so a stats-only request runs zero diffs, and counts-only,
dirty-flag, and identity requests load no blobs at all.

### Focused Git Metadata

`GitRequest.metadata` is an `Option<GitMetadataRequest>` giving fine-grained control over commits,
ref decorations, branches, divergence, remotes, tracking, config, and worktrees.

**`None` means "derive legacy behavior from the coarse fields", never "collect nothing".** The
`wants_*` accessors are the single place that rule lives; do not re-derive it at a call site.

`#[serde(default, skip_serializing_if = "Option::is_none")]` is the whole compatibility story, and
both halves are load-bearing: every preset leaves the field `None`, so preset JSON stays
**byte-identical**, and plans serialized before the field existed still deserialize.

Controls **narrow, never widen**: `summary().metadata(GitMetadataRequest::all())` still collects no
commits, because `commit_count: 0` wins. `full()` keeps paying for branch divergence by design — the
preset's published `ahead`/`behind` values are contract — so only an explicit
`branch_divergence(false)` skips the two reachability walks per non-current branch.

One request-scoped `RefSnapshot` performs a single ref-store iteration and peels only the local,
remote-tracking, and tag refs needed by that request. Recent-commit decorations, local branches,
remote details, tracking status, and containment reuse that observation; focused combinations never
re-glob remote tips. Linked-worktree paths, branches, detached state, and HEAD IDs come directly from
gix worktree proxies and their administrative `HEAD` files. Metadata-only requests therefore open no
linked checkout as a repository. Full worktree status/details retain a parallel repository-open
fallback only for checkouts whose index and working tree are not represented by the already-open
current handle.

### Bounded Path History

`commits_for_path_at`/`get_commits_for_path` take `PathHistoryOptions` and return a
`PathHistoryResult { commits, commits_scanned, history_exhausted, limit_reached }`. The bound is a
commit count, not wall time, so identical repositories have equivalent completeness on every machine.

**`history_exhausted` and `limit_reached` are not complements.** A walk that satisfied `count` before
either boundary reports both `false`. That third state is why the API is not a bare `Vec<CommitInfo>`:
a short vector cannot distinguish "history exhausted" from "gave up early", and would make an
incomplete answer look authoritative.

The default bound is `DEFAULT_PATH_HISTORY_SCAN_LIMIT` (10,000); a zero bound is rejected in favor of
it, since zero would return an empty history indistinguishable from "path never touched". The bound
caps the **tail** — stopping at `count` matches is what keeps the common case cheap — so sparse or
absent-match paths are what it actually protects.

Deep remote containment builds a target set from the requested commit IDs and stops each remote-tip
ancestry walk once every target reachable on that walk has been found. This is a reachability bound,
not a time bound: date-window history stays correct under skewed commit timestamps, and no age-based
early stop is used.

### Subprocess Deadlines

Every subprocess goes through `process::run_with_timeout` or its builder-capable
`process::run_command_with_timeout` form, which own the deadline, concurrent pipe draining, and
process-tree termination/reaping. The builder form preserves caller-configured working directories
and environments. Unix probes run in a dedicated process group;
Windows probes run in a kill-on-close Job Object. Descendant-held pipe handles therefore cannot
extend the helper's deadline. **Never poll `try_wait()` over a piped stdout you are not draining** —
the child blocks in `write()` past one pipe buffer (64 KiB Linux, ~16 KiB macOS), never exits, and
gets killed at its deadline with its output lost.

Deadlines are policy and live in `process::timeouts`: 3s for service, Windows locale, BurntToast,
program-schema, and NTP commands; 5s for `diskutil`; 2s for host-capability probes; and 30s for
explicit remote refresh. Installation commands use the caller's `InstallOptions::timeout_secs`
(30s by default). Changing one is a policy change, not an incidental refactor. Tests inject short
deadlines rather than sleeping for a production one.

Service enrichment is chunked, not per-service: systemd was `1 + N_running` spawns and runit was `N`;
both now batch at `ENRICHMENT_CHUNK` (128), which bounds command-line length only — not how many
services get reported. A failed or timed-out chunk degrades only its own services and never discards
a healthy chunk. Failure of a primary listing returns the backend's existing unavailable/empty
result; a timeout during enrichment returns the successfully parsed partial list. Diagnostics are
`tracing` events, never terminal noise.

### Remote Snapshot

`RemoteRepoSnapshot` (`remote/snapshot.rs`) is the remote analogue of the observation index:
metadata, default branch, and tree are resolved **once** per `fetch_report`. All four providers
(GitHub, GitLab, Gitea, Bitbucket) are now 1 metadata + 1 tree per report. The three hooks
(`snapshot`, `list_documents_with`, `detect_cicd_with`) are **defaulted**, so a downstream provider
adopting none still works; a provider adopting `list_documents_with` must override `list_documents`
too, or the default re-enters it and recurses.

`RemoteTree::available` is not the same as an empty `files`: `false` means the fetch failed and the
tree carries no evidence, so a projection degrades rather than concluding "absent". Metadata is
required; a tree failure never sinks a report.

A provider-reported truncated tree is not complete evidence. Continuations are counted under
`remote.api_requests.tree_continuation`, separate from `tree`, so a correctness-preserving
continuation stays distinguishable from a duplicate root request.

WAN IP detection reuses one blocking client across the ladder and queries **at least two** default
HTTPS endpoints **sequentially**, stopping at the first strictly parsed IP — so a successful first
attempt discloses the caller's address to exactly one endpoint. Response bodies never reach an error,
log, or counter.

### Parallel Program Detection with Shared Executable Index

`ProgramsInfo::detect()` first builds a single `ExecutableIndex` by scanning every `PATH` directory and macOS app bundle location once. All 8 program categories then share this index via `Arc` and perform O(1) HashMap lookups instead of redundant filesystem traversals. The categories themselves run in parallel using `rayon::join` pairs (editors+utilities, language+OS package managers, TTS+terminals, headless audio+AI clients). Rayon handles the nested parallelism correctly, distributing work across its thread pool without spawning excess threads.

The net effect: the PATH scan runs once instead of eight times, macOS bundle detection runs once instead of eight times, and per-program lookups are HashMap hits rather than `which` invocations.

On Windows, `ExecutableIndex::build()` also populates a `WindowsIndex` holding two HashMaps: `app_paths` (from a HKLM+HKCU registry walk) and `install_roots` (from a one-level directory walk of `Program Files`, `Program Files (x86)`, and `LocalAppData\Programs`). Warm-cache cost: 40–80 ms serial, inside the existing build-once budget. Non-Windows builds do not compile this code.

## Performance Evidence

Performance claims in sniff are judged by **work removed**, not wall time. `sniff::performance`
provides the evidence: stable counter names live in `sniff/lib/src/performance/counters.rs` and are a
contract — renaming one invalidates every archived baseline. An absent counter means zero.
`cargo run -p sniff --release --example work_counts` prints the baseline table for the standard cases.

Read this before quoting any timing:

- **Counters, not wall time.** Hosted runners and a loaded dev box swing timings by 2x+ on identical
  code. A Criterion run at load 57–87 on 16 cores reported **+330%** for a case whose counters were
  byte-identical. Always keep an unchanged case in the run as a drift bracket.
- **Compare only within the same OS and runner class.** There is no universal cross-OS wall-clock
  threshold, and none should be added.
- **A high counter is not by itself duplicate work — attribute before optimizing.**
  `filesystem.io.metadata_probes` sits at 13,275 against 4,685 walked entries on the 375-package
  fixture, which reads like the dominant reuse defect. It is not: per-path attribution found 12,675
  distinct and 600 redundant, only 4.5%. Those probes *are* the detection contract ("does
  `<pkg>/vitest.config.ts` exist?"), asked once each, about paths the walk legitimately does not
  answer.
- **Sampling on macOS costs ~7x.** Read a sampled profile for composition (does this symbol appear at
  all?), never for absolute attribution.
- **Parallel workers need explicit collector propagation or their counts vanish.** This has bitten
  four times. Any new `build_parallel`/`spawn` site must carry a collector, or it silently reports
  zero — a counter that *drops* when you add work is this bug, not the code under test.

Archived-baseline caveats, which are why phase tables are not interchangeable:

- Phase 1's `filesystem.io.file_opens`/`bytes_read` **under-report** every case that built a manifest
  index, because `ManifestIndex::build`'s workers carried no collector.
- `git.file_diffs` **under-reports before Phase 5**: `text_hunks` ran a full diff per patch side
  without incrementing it. Phase 5 collapsed stats+patch onto one diff per side, so the counter does
  not move while the real work halves — `git.blob_loads` is the counter that shows it.

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

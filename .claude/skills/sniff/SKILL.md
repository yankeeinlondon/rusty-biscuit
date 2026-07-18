---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, adding new detection capabilities, or optimizing detection performance.
hash: 3cd50ffff2b8b5db-6ebb32d818204990
last_updated: 2026-07-17
---

# sniff

Cross-platform system detection library and CLI for Rust.

## Platform Support

Supported targets are **macOS**, **Linux**, and **Windows**. **WSL** is treated as
the Linux compile and runtime path: under WSL, sniff uses the same `/proc`-backed
detectors as native Linux (`HostCapabilities` exposes an `is_wsl` flag) and no
detector crosses into native Windows behavior. CI enforces this matrix in
`.github/workflows/test.yml` via the `sniff-cross-platform` job, which runs
`cargo check --all-targets` plus the nextest tiers on all three OSes. Windows
audio detection uses a PowerShell `Get-CimInstance` probe (not `wmic`). Keep
test code portable — gate Unix-only imports (`std::os::unix`) and macOS-only
helpers behind `cfg`, and build `PATH` with `std::env::join_paths` rather than
literal `:`/`;` separators.

## Capabilities

| Category | Detection |
|----------|-----------|
| OS | Distribution, kernel, architecture, package managers, locale, timezone, NTP status |
| Hardware | CPU (with SIMD), GPU (Metal), memory, storage, audio devices |
| Network | Interface enumeration with IPv4/IPv6, WAN IP (TTL-cached) |
| Filesystem | Git repos, monorepos, languages, file types, EditorConfig, docs, blast radius, justfiles, recent commits, package-manager/test-runner usage collapse |
| Programs | 9 categories with macOS bundle support, install + remote-bash consent (parallel via Rayon, shared executable index). The 9th category — test runners — resolves via PATH, project-local bins (`node_modules/.bin`, `vendor/bin`, `.venv/bin`, …), and parent binaries, reporting an `availability` discriminator instead of a bare boolean. |
| Services | 10+ init systems (systemd, launchd, OpenRC, runit, etc.) |
| Packages | 110+ package manager abstraction |
| Remote | GitHub, GitLab, Gitea, Bitbucket repository metadata |

## API Tiers

Three levels of API, from simplest to most flexible:

### Tier 1: Convenience (full defaults)

```rust
use sniff::detect;
let result = detect()?;
```

### Tier 2: Plan-based (fine-grained control)

```rust
use sniff::{detect_with_plan, request::*};

let plan = DetectionPlan::new()
    .os(OsRequest::summary())           // Core identity only
    .hardware(HardwareRequest::summary()) // CPU + memory only
    .without_network()                   // Skip entirely
    .filesystem(
        FilesystemRequest::new()
            .git(GitRequest::summary())  // Branch + dirty flag
            .repo(RepoRequest::structure()) // Workspace tools only
            .without_docs()
    );
let result = detect_with_plan(plan)?;
```

### Tier 3: Module-level (expert composition)

```rust
use sniff::filesystem::git::GitRepo;
use sniff::hardware::detect_hardware_summary;
use sniff::os::detect_os_with_request;

// Pick exactly what you need
let git = GitRepo::discover(path)?;
let hw = detect_hardware_summary()?;
```

### Legacy: SniffConfig (still supported)

```rust
use sniff::{detect_with_config, SniffConfig};
let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)           // Equivalent to GitRequest::deep()
    .commit_count(20)
    .skip_network();
let result = detect_with_config(config)?;
```

## Request Types

| Type | Presets | Controls |
|------|---------|----------|
| `OsRequest` | `summary()`, `full()` | Package managers, locale, timezone, NTP status (bounded at 3s; off in the default plan) |
| `HardwareRequest` | `summary()`, `full()` | Storage, GPU, audio (~1.5s macOS) |
| `NetworkRequest` | `interfaces_only()`, `full()` | WAN IP lookup (HTTP); `.force_refresh(true)` bypasses the TTL cache |
| `GitRequest` | `identity()`, `summary()`, `full()`, `deep()` | Commits, per-file stats, worktrees, unified diffs, remote refresh. `.metadata(GitMetadataRequest)` opts into fine-grained control of commits/decorations/branches/divergence/remotes/tracking/config/worktrees |
| `RepoRequest` | `structure()`, `focused(details)`, `full()` | Focused package managers/dependencies/test runners; full inventory-backed enrichment |
| `FilesystemRequest` | `new()` (default full) | Composes git + repo + file inventory + formatting + docs |
| `DetectionPlan` | `new()` (default full domains, **safe** defaults — NTP off) | Composes all four domains (os, hardware, network, filesystem) |

`structure()` performs membership and minimum package identity only. Package managers, dependencies,
test runners, features, languages, frameworks, and file lists remain empty. Use
`RepoRequest::focused(RepoDetailRequest::…)` for selected manifest-backed facts without enabling the
inventory-backed work in `full()`. Structure still pays a nested-marker fallback walk that full
avoids via the observation index, so it is not simply "full minus work".

**Git preset cheat sheet:**
- `identity()` -- repo root, branch, HEAD id, worktree flag, base repo root, and org/repo only. **No working-tree status walk**, no commits, no branches, no remotes, no config. This is the new floor below `minimal()`.
- `minimal()` / `summary()` -- branch + dirty *flag* only (no per-category counts, no commits, no worktrees). The two presets are currently byte-identical.
- `full()` -- 10 commits, per-file change stats, worktrees; no unified diffs, no network
- `deep()` -- adds full unified diffs, remote refresh, branch details, and per-commit containment
- Every preset except `identity()` runs a working-tree status walk. Use `identity()` when you only need repository identity, or use the Tier-3 `GitRepo::discover().repo_root()` handle for bare repo-root access.

**Focused Git metadata (`GitMetadataRequest`):**
- `GitRequest.metadata` is `Option<GitMetadataRequest>`. **`None` means "derive legacy behavior from the coarse fields", never "collect nothing"** — the `wants_*` accessors (`wants_commits`, `wants_branches`, `wants_branch_divergence`, `wants_config`, …) are the single place that rule lives; don't re-derive it at a call site.
- `#[serde(default, skip_serializing_if = "Option::is_none")]` is the whole compatibility story and both halves are load-bearing: every preset leaves it `None`, so preset JSON stays **byte-identical**, and plans serialized before the field existed still deserialize. Asserted by `preset_json_omits_the_metadata_field` and `legacy_request_json_without_metadata_derives_legacy_behavior`.
- Controls **narrow, never widen**: `summary().metadata(GitMetadataRequest::all())` still collects no commits, because `commit_count: 0` wins.
- `full()` keeps paying for branch divergence by design — the preset's published `ahead`/`behind` values are contract. Only an explicit `branch_divergence(false)` skips the two reachability walks per non-current branch.

**Bounded path history (`PathHistoryOptions` / `PathHistoryResult`):**
- `commits_for_path_at` / `get_commits_for_path` take `PathHistoryOptions` and return `PathHistoryResult { commits, commits_scanned, history_exhausted, limit_reached }`.
- **`history_exhausted` and `limit_reached` are not complements.** A walk that satisfied `count` before either boundary reports both `false`. That third state is the reason the API is not a bare `Vec<CommitInfo>`.
- Default scan bound is `DEFAULT_PATH_HISTORY_SCAN_LIMIT` (10,000) and a zero bound is rejected in favor of it — zero would return an empty history indistinguishable from "path never touched".
- The bound caps the **tail**; stopping at `count` matches is what makes the common case cheap. Sparse/absent-match paths are what the bound protects.

## Key Types

| Type | Description |
|------|-------------|
| `SniffResult` | Top-level: os, hardware, network, filesystem (all `Option<T>`) |
| `DetectionPlan` | Plan-based config with per-domain request types |
| `ProgramsInfo` | 9 category fields with shared `ExecutableIndex` + parallel Rayon detection. `test_runners` carries `InstalledTestRunners` (`Vec<TestRunnerEntry>` with `Availability` discriminators `installed` / `local` / `via_parent` / `not_found`). |
| `ServicesInfo` | Init system + service list (via `ServiceManager::detect()`) |
| `Package` | Package path, languages, managers, dependencies |
| `GitRepo` | `gix::Repository` handle from trusted discovery. All git access (status, diff, history, refs, remotes, config, worktrees) is pure-Rust gix; git2/libgit2 is gone from production and retained only as a dev-dependency for test/bench fixtures. |
| `BranchInfo` | Local branch projection with branch name, current flag, tip SHA, upstream, ahead/behind counts, and whether any locally known remote-tracking ref points at the branch tip. `upstream`/`ahead`/`behind` serialize as `null` when no upstream is configured (distinguishing "no tracking data" from an even `0`). Default branch detection uses known refs only; refresh requires explicit opt-in. |
| `get_current_worktree_name` | Early-return helper: returns the basename of the linked worktree directory, or `None` if in the main worktree |
| `MonorepoStandard` | Standard-based monorepo descriptor (Cargo, pnpm, Nx, Bazel, etc.) with `BinarySpec` and advisory `InvocationTemplate`s |
| `DetectedStandard` | Detected instance of a `MonorepoStandard`, including a `ResolvedBinary` (`Path`, `Wrapper`, or missing) and version satisfaction |
| `MonorepoLayer` | One layer of the repo topology; includes `authority`, `orchestrators`, `provenance`, and `packages` (repo-relative path references matching `Package.relative`) |

## Monorepo Topology Model

`RepoInfo` exposes two additive fields for the standard-based model:

- `monorepo_standards: Vec<DetectedStandard>` — every standard whose root marker matched, each with its resolved acting binary and `DetectionConfidence`.
- `monorepo_layers: Vec<MonorepoLayer>` — membership layers, one per root. Each layer has an `authority` (the standard that defines membership), zero or more `orchestrators` (standards that only run tasks across packages), and `packages` — repo-relative path strings that each resolve to exactly one entry in `RepoInfo.packages`.

A standard can hold one or more `Role`s:

- `DefinesMembership` — declares which packages belong to the monorepo.
- `OrchestratesTasks` — runs tasks across packages.
- `ManagesDependencies` — resolves and installs dependencies.

### Authority vs. Orchestrator

- Authorities own the package list (`CargoWorkspace`, `PnpmWorkspaces`, `GoWorkspace`, `Bazel`, etc.).
- Orchestrators ride on top of an authority and appear in `MonorepoLayer::orchestrators`. A repo with only an orchestrator (e.g. an `nx.json` and no workspace authority) is **not** reported as a monorepo.
- Orchestrator-only standards (`Nx`, `Turborepo`, `Lerna`) never appear as `Package.standard`; they only appear as orchestrators on a layer whose authority owns the packages.
- A layer can have multiple standards when a membership authority and one or more orchestrators share the same root (e.g. pnpm + Nx).

### Package Catalog

The canonical package catalog is `RepoInfo.packages`. Each package carries its owning `standard` and `provenance` directly:

- `Package.standard` — the membership authority that owns this package (`CargoWorkspace`, `PnpmWorkspaces`, etc.).
- `Package.provenance` — how the boundary was derived (`Globbed`, `RootExplicit`, `LeafMarkers`, `Lockfile`, `ManifestScan`, etc.).

`MonorepoLayer.packages` does not duplicate package metadata; it holds repo-relative paths that point into the canonical catalog.

### Unified Topology Model

The legacy `MonorepoTool` enum, `RepoInfo.monorepo_tool` / `workspace_tools`, and `Package.discovery_sources` were removed during the monorepo type unification. The `MonorepoStandard` / `MonorepoLayer` / `Package.standard` / `Package.provenance` model is now the sole surface:

- `RepoInfo.monorepo_standards` lists every detected standard with its acting binary and confidence.
- `RepoInfo.monorepo_layers` lists membership layers; each layer has one `authority` and zero or more `orchestrators`.
- `RepoInfo.packages` is the canonical catalog; each package carries `standard` and `provenance`.
- `MonorepoLayer.packages` contains repo-relative path strings that each resolve to exactly one `RepoInfo.packages[].relative` entry.

CLI text derives the one-liner from `RepoInfo::primary_layer()`. The shared label helper composes `{orchestrator_label} (using {authority_label})` when an orchestrator is present, otherwise `{authority_label}` alone. Both parts read each standard's `spec().label` (e.g. `cargo`, `pnpm workspaces`, `Nx`) — never `display_name`. JSON output no longer contains `monorepo_tool`, `workspace_tools`, or `discovery_sources`.

## Shared-Work Highlights

- **Top-level concurrency**: OS, hardware, network, filesystem run in scoped threads via `detect_with_plan`
- **Staged filesystem**: git → repo → file inventory → formatting → docs; each stage reuses earlier work
- **Walk scope is chosen from consumers, never from Git**: an internal `WalkConsumers`/`WalkScope` pair decides whether the shared descendant walk runs and from where. Formatting-only requests start **no** walker (`.editorconfig` is probed directly). Structure-only repo requests start none either. Inventory alone walks the **resolved package root**, not the repository root — a Git handle being present is not a consumer of repository-wide evidence and no longer widens the scope. Full repo detection and repo-wide docs walk the repository root.
- **Inventory saturation**: classification stops at `MAX_FILES` (10,000 accepted). An inventory-**only** walk quits globally at the cap; a combined walk keeps going for its still-active manifest/docs observers (quitting would silently truncate the manifest index). `FileInventory`, `FileAssociationBreakdown`, and `LanguageSummary` carry `truncated: bool` + `limit: Option<usize>`, Serde-defaulted and omitted when complete, so existing JSON consumers are unaffected. **A truncated subset is unspecified across runs** — workers race for the bounded slots. Ordering is always sorted; a complete result is fully deterministic.
- **One observation index per request**: a **full** repo detection observes the tree **once**. `FilesystemSystemView` is the request-scoped index; `SharedWalkOptions` gates what it retains (manifest index, unfiltered manifest dirs, nested-workspace markers, capped inventory, Markdown metadata). It never retains `DirEntry` values or file bodies, and there is no process-global cache. `RepoEvidence::from_view` borrows it into repo detection — the *only* constructor from a walk, so evidence and the root it came from cannot drift apart.
- **Both full routes share it**: standalone `detect_repo`/`detect_repo_with_inventory` and integrated `WalkScope::Repository` build the same index and return equivalent results. Before, standalone walked the tree **three** times (manifest index + nested markers + inventory) and integrated twice.
- **`manifest_dirs` vs `manifest_index`**: `ManifestIndex` drops generated/fixture manifests because they are not discovery boundaries. Membership **globs** resolve a boundary by marker presence alone and never applied that exclusion, so they consume the unfiltered `manifest_dirs`. Routing globs through the filtered index would silently drop a member whose manifest says `auto-generated` — and only in full mode, making structure and full disagree.
- **Structure mode keeps the smallest evidence set**: it consumes no index, inventory, or docs, so building the observation index for it would classify every file for evidence it discards. It walks for nested markers instead (`filesystem.repo.nested_marker_walks: 1`), which is expected, not a defect.
- **Discovery finishes before enrichment**: detectors return cheap `PackageSeed`s (`repo/seed.rs`) — a normalized `key`, the `owner_root` they were resolved against, standard, provenance, evidence kinds — and `merge_seeds` collapses duplicate boundaries **before** `create_package` runs. Enrichment (manifest parses, ecosystem probes, the per-package test-runner search) therefore happens exactly once per unique boundary. Two detectors resolving the same directory is normal, not a bug; what was a bug is paying full enrichment for each and merging afterwards. `merge_packages`/`merge_package_into`/`dedupe_packages`/`rebase_package_to_root` are **gone** — there is nothing left to reconcile, and re-adding a post-hoc merge would re-open the defect.
- **One manifest store per detection**: `ManifestStore` is request-scoped and keyed by normalized native paths. Cargo/Node/Python/Go manifests, raw manifest text, Cargo/pnpm/uv lockfiles, inherited Cargo workspace manifests, and root-scoped test-runner configuration observations are reused across every package in that detection. Values are `Rc`-backed behind `RefCell` because repo detection is single-threaded; the store never crosses into the parallel filesystem walker. Keep `PackageSeed::owner_root` as the lookup frame so nested workspaces cannot share the wrong root manifest or lockfile.
- **One package ownership index per request**: `PackageOwnershipIndex` is a crate-private companion, not a `RepoInfo` field, so the public Rust and Serde shapes stay unchanged. Detection builds it from the normalized native `PackageSeed::key` values and a lexical repo-root alias, then inventory and integrated document attribution borrow that same instance. Standalone document and commit requests build one companion for their whole request. Lookups ascend `Path` parents to select the deepest component prefix without lossy UTF-8 conversion, string-prefix matching, or ad hoc Windows case folding.
- **`PackageSeed::owner_root` is load-bearing**: enrichment runs in the frame the *detector* resolved the boundary in, because Cargo `version.workspace = true`, npm's root-version fallback, and `Cargo.lock` resolution all resolve against the owning workspace's root — not the repo root. Only `relative`/`package_area` are re-framed to the repo-root catalog view. A nested Cargo workspace enriched against the outer root would silently report the wrong inherited versions.
- **`seed.evidence` presence is proof; absence is not.** The manifest index omits generated (`auto-generated`) and fixture manifests, so an empty evidence set means "nothing observed", never "no manifest exists". A probe-skipping fast path is sound only for kinds known *present*.
- **`ManifestIndex` entries are sorted by `canonical`**; `subtree_range` binary-searches a contiguous range instead of scanning every manifest per query (which was O(packages²)). Membership still tests `Path::starts_with` (componentwise), so `crates/pkg-a` never claims `crates/pkg-a2`.
- **`has_workspace_marker` gate**: full `detect_repo` only observes a root that carries a workspace marker. This is what keeps `detect_repo` over a large non-repo directory (a system temp dir) cheap — do not remove it.
- **Single-pass nested-marker walk**: marker filenames (`package.json`, `pnpm-workspace.yaml`, `*.sln`, …) are matched in-memory against entries the `ignore` walker already yields rather than re-probed per directory. Gitignored markers are intentionally not detected (markers are conventionally committed); see `spec.md` "Intentional Behavior Change" in the `2026-06-20-faster-package-list` feature.
- **Git status layers**: counts-only vs full file changes vs full unified diffs, selectable via `GitRequest` flags
- **Parallel remote fetch**: deep git mode fetches multiple remotes concurrently with bounded parallelism; `GIT_TERMINAL_PROMPT=0` is preserved to avoid interactive hangs
- **Ancestry-walk containment**: per-commit remote containment is computed with a single ancestry walk per remote, cached in a `HashMap<Oid, Vec<remote>>`
- **One ref snapshot per Git request**: focused combinations enumerate the ref store once and reuse peeled local/remote tips for decorations, branch projection, remote details, tracking, and containment. Linked-worktree metadata comes from gix proxies plus administrative `HEAD`; metadata-only requests perform zero linked-repository opens, while full status/details use the counted parallel fallback only for checkouts unavailable through the current handle.
- **ExecutableIndex**: scans `PATH` + macOS bundles once, shared across all 9 program categories via `Arc`. The test-runner category additionally consults a cwd-sensitive `LocalBinIndex` for project-local bin dirs. Four builders span a PATH-scan × fallback-layers 2×2: `build()` (lazy `which`, + bundles/Windows layers, bundle index cache-backed), `build_path_only()` (lazy, no layers), `build_eager_path()` (one PATH traversal, + layers), and `build_eager_path_only()` (one PATH traversal, no layers — used by `HostCapabilities::detect()`). Prefer an eager builder for bulk lookups; `_only` matters on Windows, where the fallback layers scan the App Paths registry and would change *which* programs resolve.
- **One remote snapshot per report**: `RemoteRepoSnapshot` (`remote/snapshot.rs`) is the remote analogue of the filesystem observation index — metadata, default branch, and tree resolved **once** per `fetch_report`. Before it, `list_documents` and `detect_cicd` each re-fetched metadata just to learn the default branch and then re-fetched the identical tree (GitHub/Gitea 3 metadata + 2 tree; GitLab **3 identical tree** calls, since absent `GetProject` its metadata call *is* a tree fetch whose result was discarded). Now all four are **1 metadata + 1 tree**. The three hooks (`snapshot`, `list_documents_with`, `detect_cicd_with`) are **defaulted** — a downstream provider adopting none still works. A provider adopting `list_documents_with` must override `list_documents` too, or the default re-enters it and recurses.
- **`RemoteTree::available` ≠ empty `files`**: `false` means the fetch failed and the tree carries no evidence, so a projection degrades rather than concluding "absent". Metadata is required; a tree failure never sinks a report.
- **`truncated` is real and was ignored**: GitHub/Gitea deserialize it and nothing read it, so a >100k-entry repo reported "no docs, no CI" confidently; Bitbucket ignored `response.next` the same way. Continuations are counted under `remote.api_requests.tree_continuation`, separate from `tree`, so a correctness-preserving continuation stays distinguishable from a duplicate root request. **`CONTINUATION_PREFIXES` entries must be single path components** — the client percent-encodes the whole `branch:prefix` tree_sha into one path segment, so `.github/workflows` would go out as `%2F` and be rejected/normalized by routers. Continue `.github`; the recursive response supplies `workflows/ci.yml`.
- **`categorize_document` lives once**, in `remote/snapshot.rs`. All four providers previously carried byte-identical copies plus four copies of the same test suite.
- **Every subprocess goes through `process::run_with_timeout` or builder-capable `process::run_command_with_timeout`** (`process.rs`), which own the deadline, concurrent pipe draining, process-tree termination, and reaping. The builder form preserves caller-configured cwd and environment. Unix probes use a dedicated process group and Windows probes use a kill-on-close Job Object, so a descendant retaining inherited stdout/stderr cannot extend the deadline during reader joins. **Never poll `try_wait()` over a piped stdout you are not draining** — the child blocks in `write()` past one pipe buffer (64 KiB Linux / ~16 KiB macOS), never exits, and gets killed at its deadline with its output lost. That bug was live in `os/time.rs`, `programs/schema.rs`, and `host_capability.rs`; `Command::output()` drains correctly but blocks forever without a deadline (the former installation/uv/remote-refresh paths were the last bypasses). Deadlines are policy and live in `process::timeouts` (services/Windows-locale/BurntToast 3s, `diskutil` 5s, host-capability 2s, program-schema/NTP 3s, explicit remote refresh 30s); installation commands use `InstallOptions::timeout_secs` (30s by default). Tests inject short deadlines; never sleep for a production one.
- **Service enrichment is chunked, not per-service**: systemd was `1 + N_running` spawns (one `systemctl show` each), runit `N` (one `sv status` each). Both now batch at `ENRICHMENT_CHUNK` (128), which bounds command-line length only — not how many services get reported. A failed/timed-out chunk degrades only its own services (`pid: None` / `(false, None)`) and never discards a healthy chunk.
- **`DetectionPlan::default()` is "all domains at safe defaults", not "every domain at `full()`"**: it uses `OsRequest::full().include_ntp_status(false)`, so Tier-1 `detect()` initiates no implicit network probe. `OsRequest::full()` is unchanged — explicit callers still get NTP.
- **WAN IP**: one blocking client reused across the ladder (was rebuilt per attempt), **two** default HTTPS endpoints (one made the fallback unreachable), queried **sequentially** — a successful first attempt discloses the host address to exactly one endpoint. Bodies are strictly parsed through `IpAddr` and never reach an error, log, or counter. The ladder runs on a spawned thread inside a tokio context, so its counters need an explicit `WorkerCollector` (see the parallel-worker trap below).
- **CLI async model**: the CLI is a single-shot command runner; most paths are synchronous (git, filesystem, subprocess) and run directly in the async entrypoint. `spawn_blocking` is avoided unless true concurrent async work exists

## Testing gotcha: feature-gated tests

`sniff-lib` has `default = []`, and `remote = ["network"]`. **A test behind `#[cfg(feature = "remote")]`
or `#[cfg(feature = "network")]` does not compile, let alone run, unless the feature is on.** The
`sniff/justfile` `test` recipe passes `--features remote` for exactly this reason — until Phase 6 it did
not, and **190 tests silently never ran**, including the entire 65-test `remote_providers` suite.

If you add a test under either feature, confirm it actually executes (`just test` should report a
higher count) rather than trusting a green run. A feature-gated test that never runs is worse than no
test: it reads as coverage.

**The same trap bit CI's compile guard.** `test.yml`'s `sniff-cross-platform` job ran
`cargo check -p sniff --all-targets` with no `--features` — so the guard whose whole purpose is
catching a Unix-only import in test code that never runs on that OS could not see any
`remote`/`network`-gated target. Phase 8 added `--features remote` there. Any new feature gate needs
the same treatment in **both** the justfile recipe and the CI check.

## CLI

```bash
sniff                      # Show help
sniff --json               # Full system info (JSON output)
sniff hardware             # Hardware only (text output)
sniff cpu                  # Just CPU info
sniff audio-devices        # Audio input/output devices
sniff software             # All program categories
sniff software editors     # Just editors
sniff software editors install  # Install an editor (interactive)
sniff software test-runners   # Test runners with availability discriminator
sniff software agents      # AI CLI tools
sniff services             # System services
sniff docs                 # Markdown documents
sniff topics               # Table of available topics
sniff just                 # Justfiles and recipes
sniff repo                 # Repository name (bare `sniff repo` is distinct from `sniff repo name`)
sniff repo name            # Repository name only
sniff repo name -v         # Repository name only (verbose styling)
sniff repo is-monorepo     # Monorepo label (e.g. `cargo`; `false` if not). Exits non-zero when false unless `--no-error`. `--json` emits `{ "is_monorepo": true, "authority": "...", "orchestrators": [...] }` / `{ "is_monorepo": false }`
sniff repo package-count   # Number of discovered packages (`{ "package-count": N }` with --json)
sniff repo version         # Declared version(s) for the current package/area/repo context — scoped like `repo test-runner`; `--all`/`--package`/`--package-area` override the CWD scope; `--json` returns `{ "versions": [ { version, packages, sources: [ { manifest, path, href, inherited, packages } ] } ] }`
sniff repo package-manager # Package manager(s) for the current package/area/repo context (`--csv`, `--list`, `--md`, `--json`)
sniff repo test-runner     # Declared test runner(s) for the current package/area/repo context with evidence (`--csv`, `--list`, `--md`, `--json`)
sniff repo git-status      # Git status with commit history
sniff repo language        # Primary programming language for the repository
sniff repo worktree        # Linked worktree name (exit 1 if main worktree)
sniff repo worktrees       # List all worktrees (default, --md, --list, --csv, --verbose, --json)
sniff repo branches        # Local branches from known refs (`--refresh-remotes` opts into fetch)
sniff repo package-dependencies # Internal workspace dependency graph (`--ui` for diagram)
sniff repo dependencies    # External package dependencies with family filters
sniff repo remote origin   # Inspect remote repository
sniff repo pr              # List open pull requests
sniff repo pr --status merged  # List merged pull requests
sniff repo recent-commits 1w  # Commits from last week
sniff repo source-code-changes today  # Today's source changes
sniff blast-radius         # Docs affected by dirty changes
sniff hardware --json      # Subcommand with JSON output
```

**Output modes:**
- No subcommand: help (use `--json` for full JSON)
- With subcommand: Text (default), `--json` for JSON, `--plain` for unstyled

**`sniff repo --json` aggregate:**
Bare `sniff repo --json` returns the consolidated `SniffRepo` projection with snake_case top-level keys. Identity fields remain top-level (`name`, `version`, `language`, `is_monorepo`, `package_count`, `root`), the repo-wide `package_manager` and `test_runner` facts collapse across all packages to `string | string[] | null`, cwd-relative facts live under `context`, worktrees and branches appear once as top-level arrays, and change data is grouped into four `ScopeBucket` objects: `dirty`, `staged`, `unstaged`, and `untracked`. Each bucket contains `files`, `source_code`, `documentation`, `packages`, and `package_areas` arrays. Aggregate `git_status` is intentionally lean (`current_branch`, `config`, compact `file_changes`, and dirty/staged/unstaged/untracked counts), while focused commands such as `repo git-status --json`, `repo structure --json`, and `repo recent-commits --json` keep their richer contracts. Network-primary subcommands (`remote`, `pr`) and parameterized subcommands (`hash`) are excluded, and no network requests are made by the aggregate.

The aggregate's top-level `version` is the **`AggregateScope::Repo` collapse**: exactly one distinct version across all packages → that string; zero or more-than-one distinct versions → `null` (e.g. a pure-virtual Cargo workspace with uniform member versions now reports the version string instead of `null`). The focused `sniff repo version --json` shape is `{ "versions": [ { version, packages, sources: [ { manifest, path, href, inherited, packages } ] } ] }`.

## Work Counters (performance evidence)

Performance claims in sniff are judged by **work removed**, not wall time — hosted runners and a loaded dev box swing timings by 2×+ on identical code. `sniff::performance` provides the evidence:

```rust
use sniff::performance::{PerformanceCollector, with_current_collector};

let collector = PerformanceCollector::new_shared();
with_current_collector(Some(collector.clone()), || detect_repo(path))?;
let report = collector.snapshot(elapsed); // report.counters: BTreeMap<String, u64>
```

- Stable counter names live in `sniff/lib/src/performance/counters.rs` — treat them as a contract; renaming one invalidates archived baselines. An absent counter means zero.
- Counters cover walk entries, inventory acceptance/saturation, file opens/bytes/metadata probes/canonicalizations, manifest/lockfile/config parses, package enrichments, git discoveries/status walks/blob loads/diffs/ref walks/commit visits, subprocess spawns/timeouts, and remote API operations by slug.
- `filesystem.repo.nested_marker_walks` and `filesystem.repo.membership_glob_walks` name the two repo-detection fallback walks. They exist because `filesystem.io.read_dirs` is shared by five unrelated detectors (`test_runner_usage.rs` alone increments it at five sites) and so cannot answer "was the tree enumerated again?". `filesystem.walk.*` is incremented **only** by `build_filesystem_system_view`, so a removed fallback walk shows up as a single `read_dirs` decrement and badly understates itself.
- `cargo run -p sniff --release --example work_counts` prints the baseline table for the standard cases; baselines are archived in `sniff/features/2026-07-16-performance/phases/01-work-accounting/spec.md`.

**Adding instrumentation:**

- Recording is gated on `performance::is_collecting()` (one relaxed atomic load). Check it before setting up any instrumentation state — reading a clock, formatting a name. Use `StageTimer::start(name)` rather than a bare `Instant::now()`, so the default path reads no clock.
- Count at **one chokepoint per unit of work**, never at both a helper and its callers — double counting silently corrupts a baseline and is worse than not counting.
- **Parallel workers need explicit propagation or their counts vanish.** An `ignore::build_parallel()` worker owns a `performance::WorkerCollector` (`inherit()` on the spawning thread, `activate()` in the first callback, flush on drop) in the same struct as its result buffers. A Rayon closure takes `let _worker = performance::pooled_worker(collector.as_ref());` — pool threads park rather than exit, so nothing would otherwise drain them.
- This has bitten **four times** now: `system_view`'s walker (Phase 1), `with_current_collector`'s scoped stage threads (Phase 2), `ManifestIndex::build`'s walker (Phase 3, which reported *zero* file opens while reading every `Cargo.toml` in the tree), and the WAN IP ladder (Phase 6, once it moved onto the tokio-context thread). **Any new `build_parallel`/`spawn` site must carry a collector, or it silently reports zero.** A counter that drops when you add work — or a "regression" from a change that only removes work — is this bug, not the code under test.
- **`git.file_diffs` under-reports before Phase 5.** `text_hunks` ran a full `diff_with_slider_heuristics` per patch side without incrementing anything, so every `include_diffs` case executed twice the diffs its counter admitted. Phase 5 collapsed stats+patch onto one diff per side; the counter therefore **does not move** while real work halves, and `git.blob_loads` (8 → 4 on a two-sided fixture) is the counter that shows it. Never compare Phase 5+ diff counts to Phase 1–4 archived values.
- **Git status: one load and one diff per dirty file side.** `StatusContext` resolves the HEAD tree and index snapshot once per status call (was once per file *per side*). `each_dirty_side_loads_and_diffs_once` pins the bound — `blob_loads` above 4 on a staged+modified file means the double-observation is back. Whole-file adds/deletes are line-**counted**, not diffed, so a stats-only request runs zero diffs.
- **Archived baseline caveat**: Phase 1's `filesystem.io.file_opens` / `bytes_read` under-report every case that built a manifest index. Compare against the Phase 3 table in `sniff/features/2026-07-16-performance/phases/03-observation-index/spec.md`, not the Phase 1 values. For **full-mode** cases the current baseline is Phase 4's table (`phases/04-package-enrichment-and-ownership/spec.md`), which halved them again.
- **The archived Phase 4 structure/full counts predate shallow structure semantics.** Structure now records zero package enrichments and skips dependency, test-runner, feature, framework, language, and file-list work, so do not use the old near-equality as a regression bound.
- **Counters, not wall time.** Timing on a loaded host is worthless: a Phase 3 Criterion run at load 57-87/16 cores reported +330% for a case whose counters were byte-identical. Always keep an unchanged case in the run as a drift bracket.
- **`work_counts` case order is a page-cache confound.** `repo_structure_huge_375_packages` runs *before* `repo_full_huge_375_packages` over the **same** fixture, so the full case reads a cache the structure case just warmed. Three sequential runs put structure *slower* than full (223/144, 138/112, 124/100 ms) — that is the confound, not a finding. Never read a structure-vs-full ratio off one sequential run.
- **Phase 8 is the final baseline for full-mode and Git cases**, but its structure-mode rows predate the R5 shallow-contract migration and are no longer current. Compare new structure measurements against post-review artifacts, not the Phase 1/8 structure rows.
- **CI collects this nightly across three OSes.** `sniff-performance.yml`'s `sniff-work-counts` matrix uploads the table per OS (90-day retention). Compare **only within one OS and runner class** — artifacts are named per OS and the Criterion baseline is `ci-linux` precisely so a cross-OS diff cannot happen by accident. Cross-OS counter deltas are not regressions; path/case/ignore semantics differ legitimately by platform.
- **A high counter is not by itself duplicate work — attribute before optimizing.** `filesystem.io.metadata_probes` is **13,275** against 4,685 walked entries on the 375-package fixture, which reads like the dominant reuse defect left in the codebase. It isn't: per-path attribution (Phase 7) found **12,675 distinct / 600 redundant — only 4.5%**. Those probes *are* the detection contract ("does `<pkg>/vitest.config.ts` exist?"), asked once each, about paths the walk legitimately does not answer. There is no observe-once win there. Before treating any counter as duplicate work, attribute it by `(call site → path)` with a temporary `#[track_caller]` shim on `probe_exists`/`probe_is_dir`; the aggregate counter cannot tell "expensive" from "repeated".
- **The R14 micro-optimizations were measured and deferred — don't re-litigate them without new evidence.** Phase 7 profiled both hot paths post-structural work and deferred **all nine** candidates (ASCII classification, framework prefix reads, walker-metadata mtime reuse, static regex/maps, path-list merges, rate-limit lowercasing, interface-cache clones, local-bin memoization). Repo detection is **~71% syscalls** on distinct paths with its largest userspace bucket at ~2.9%; `ProgramsInfo::detect` is **~96% Rayon park/spin** — so adding parallelism there buys contention, not throughput. Two review premises were already **stale**: `standard.rs`'s version regex is now `#[cfg(test)]`-only (production leaves `version: None` for the no-subprocess boundary), and `merge_path_lists` was deleted in Phase 4. See `phases/07-profile-guided-cleanup/spec.md` for the keep/defer table and the 5%-of-counter-or-sampled-time threshold.
- **Sampling on macOS costs ~7×.** `sample` on a `detect_repo` loop returned 58 iterations where the unsampled binary did 396 in the same window. Read a sampled profile for **composition** (does this symbol appear at all?) and never for absolute attribution. A symbol absent at 1 ms sampling is decisively cold; a symbol present is not thereby material.

## Detailed Topics

- [Programs](./programs.md) - 9 categories, macOS bundle detection, test-runner availability
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities
- [Architecture](../../../sniff/docs/sniff-library-architecture.md) - Cost model, shared-work design
- If you are working with the `gitoxide` crate -- which is used in Sniff for all **git** operations -- then make sure you use the 'rust-devops' skill!

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference

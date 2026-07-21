---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, adding new detection capabilities, or optimizing detection performance.
hash: 3cd50ffff2b8b5db-4046c57a47c4c558
last_updated: 2026-07-21
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
| `OsRequest` | `summary()`, `full()` | Package managers, locale, timezone, NTP status (up to 10s on Linux) |
| `HardwareRequest` | `summary()`, `full()` | Storage, GPU, audio (~1.5s macOS) |
| `NetworkRequest` | `interfaces_only()`, `full()` | WAN IP lookup (HTTP); `.force_refresh(true)` bypasses the TTL cache |
| `GitRequest` | `identity()`, `summary()`, `full()`, `deep()` | Commits, per-file stats, worktrees, unified diffs, remote refresh |
| `RepoRequest` | `structure()`, `full()` | Per-package language scanning (10-50x slower than `structure()`) |
| `FilesystemRequest` | `new()` (default full) | Composes git + repo + file inventory + formatting + docs |
| `DetectionPlan` | `new()` (default full) | Composes all four domains (os, hardware, network, filesystem) |

**Git preset cheat sheet:**
- `identity()` -- repo root, branch, HEAD id, worktree flag, base repo root, and org/repo only. **No working-tree status walk**, no commits, no branches, no remotes, no config. This is the new floor below `minimal()`.
- `minimal()` / `summary()` -- branch + dirty *flag* only (no per-category counts, no commits, no worktrees). The two presets are currently byte-identical.
- `full()` -- 10 commits, per-file change stats, worktrees; no unified diffs, no network
- `deep()` -- adds full unified diffs, remote refresh, branch details, and per-commit containment
- Every preset except `identity()` runs a working-tree status walk. Use `identity()` when you only need repository identity, or use the Tier-3 `GitRepo::discover().repo_root()` handle for bare repo-root access.

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

## Git Conflict APIs

Keep actual conflict state distinct from prediction:

- `filesystem::git::merge_conflicts_at(path)` observes non-zero stages in the
  live index and returns `[]` when the repository is absent or clean.
- `filesystem::git::merge_conflicts_with_branch_at(path, incoming_branch)`
  merges the exact incoming local branch tip (`theirs`) into the attached
  current local branch tip (`ours`) in probe-local memory. Missing repository,
  branch, history, or safe-configuration prerequisites are errors.

Committed-tip prediction ignores the live index and worktree. Its shared
commit-pair helper enables object-memory storage before merging, derives its
temporary index and attributes from the captured `ours` tree, and rejects
applicable external merge drivers, filters, or renormalization. It never runs a
hook, subprocess, fetch, credential helper, or network operation. Both the
public prediction API and `WorktreeEntry::has_conflicts` must continue to derive
from that one helper.

## Remote Observation and Provider Queries

With the `remote` feature, use `resolve_remote_at` for every configured-remote
selection. `branch_exists_on_remote_at` is live and read-only: HTTP(S) remotes
use Git ref advertisement, supported SSH remotes use an authoritative provider
branch endpoint, and unsupported transports return a capability error rather
than `false`. `remote_vendor_at` classifies locally when possible and probes an
ambiguous self-hosted HTTP(S) host only after exact-host consent.

Use `FocusedProviderClient` for bounded pull-request and CI/CD-job queries. It
retains provider/repository/native identity, follows provider pagination within
hard parent/page/item bounds, rejects unsupported canonical filters, disables
redirects, and checks host policy before client construction, credential reads,
or network I/O. Its production discovery path retains a self-hosted server's
reported version when its documented identity response supplies one, derives
version-sensitive capabilities conservatively when it does not, probes every
ambiguous-host candidate anonymously, and retries authentication only after a
response signature identifies the provider. Such retries use only
`SNIFF_{PROVIDER}_{ENCODED_HOST}_TOKEN`; they
never send global provider tokens to an unidentified self-hosted server. Discovery
also retains this host-bound credential scope for the resulting client's provider
queries. It derives a policy-checked HTTPS origin from the resolved host for neutral
SSH/SCP remotes and never treats an SSH port as an HTTP port. Gitea CI/CD job
operations require stable 1.25.0 or newer;
Forgejo releases through 14.0 lack the required exact/list job endpoints. These
unsupported operations fail before provider I/O with the provider, flavor, and
version preserved in the error. Do not collapse focused malformed, missing,
credential, authorization, rate-limit, capability, or transport errors into
empty results.

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
- **Shared repo inventory**: full repo detection returns its `FileInventory` alongside the `RepoInfo` via `detect_repo_with_inventory`, so the file-inventory stage skips rescanning
- **Manifest Index**: single walk records every `Cargo.toml`/`package.json`/`pyproject.toml`/`go.mod` for full repo mode; structure mode skips it entirely
- **Single-pass nested-marker walk**: `walk_for_nested_markers` matches marker filenames (`package.json`, `pnpm-workspace.yaml`, `*.sln`, …) in-memory against the entries the `ignore` walker already yields, instead of re-probing the filesystem per directory. This collapses a per-directory `exists()`/`read_dir` syscall storm (~21k syscalls on this repo) into the single batched walk. Gitignored markers are intentionally no longer detected (markers are conventionally committed); see `spec.md` "Intentional Behavior Change" in the `2026-06-20-faster-package-list` feature.
- **Git status layers**: counts-only vs full file changes vs full unified diffs, selectable via `GitRequest` flags
- **Parallel remote fetch**: deep git mode fetches multiple remotes concurrently with bounded parallelism; `GIT_TERMINAL_PROMPT=0` is preserved to avoid interactive hangs
- **Ancestry-walk containment**: per-commit remote containment is computed with a single ancestry walk per remote, cached in a `HashMap<Oid, Vec<remote>>`
- **ExecutableIndex**: scans `PATH` + macOS bundles once, shared across all 9 program categories via `Arc`. The test-runner category additionally consults a cwd-sensitive `LocalBinIndex` for project-local bin dirs.
- **CLI async model**: the CLI is a single-shot command runner; most paths are synchronous (git, filesystem, subprocess) and run directly in the async entrypoint. `spawn_blocking` is avoided unless true concurrent async work exists

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

## Detailed Topics

- [Programs](./programs.md) - 9 categories, macOS bundle detection, test-runner availability
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities
- [Architecture](../../../sniff/docs/sniff-library-architecture.md) - Cost model, shared-work design
- If you are working with the `gitoxide` crate -- which is used in Sniff for all **git** operations -- then make sure you use the 'rust-devops' skill!

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference
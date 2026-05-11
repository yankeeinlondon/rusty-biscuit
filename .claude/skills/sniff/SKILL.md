---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, adding new detection capabilities, or optimizing detection performance.
---

# sniff

Cross-platform system detection library and CLI for Rust.

## Capabilities

| Category | Detection |
|----------|-----------|
| OS | Distribution, kernel, architecture, package managers, locale, timezone, NTP status |
| Hardware | CPU (with SIMD), GPU (Metal), memory, storage, audio devices |
| Network | Interface enumeration with IPv4/IPv6, WAN IP (TTL-cached) |
| Filesystem | Git repos, monorepos, languages, file types, EditorConfig, docs, blast radius, justfiles, recent commits |
| Programs | 8 categories with macOS bundle support, install + remote-bash consent (parallel via Rayon, shared executable index) |
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
            .git(GitRequest::summary())  // Branch + dirty counts
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
| `GitRequest` | `summary()`, `full()`, `deep()` | Commits, per-file stats, worktrees, unified diffs, remote refresh |
| `RepoRequest` | `structure()`, `full()` | Per-package language scanning (10-50x slower than `structure()`) |
| `FilesystemRequest` | `new()` (default full) | Composes git + repo + file inventory + formatting + docs |
| `DetectionPlan` | `new()` (default full) | Composes all four domains (os, hardware, network, filesystem) |

**Git preset cheat sheet:**
- `summary()` -- branch + dirty counts only (no commits, no worktrees)
- `full()` -- 10 commits, per-file change stats, worktrees; no unified diffs, no network
- `deep()` -- adds full unified diffs, remote refresh, branch details, and per-commit containment

## Key Types

| Type | Description |
|------|-------------|
| `SniffResult` | Top-level: os, hardware, network, filesystem (all `Option<T>`) |
| `DetectionPlan` | Plan-based config with per-domain request types |
| `ProgramsInfo` | 8 category fields with shared `ExecutableIndex` + parallel Rayon detection |
| `ServicesInfo` | Init system + service list (via `ServiceManager::detect()`) |
| `Package` | Package path, languages, managers, dependencies |
| `GitRepo` | libgit2 handle from `GitRepo::discover(path)` |
| `get_current_worktree_name` | Early-return helper: returns the basename of the linked worktree directory, or `None` if in the main worktree |

## Shared-Work Highlights

- **Top-level concurrency**: OS, hardware, network, filesystem run in scoped threads via `detect_with_plan`
- **Staged filesystem**: git → repo → file inventory → formatting → docs; each stage reuses earlier work
- **Shared repo inventory**: full repo detection returns its `FileInventory` alongside the `RepoInfo` via `detect_repo_with_inventory`, so the file-inventory stage skips rescanning
- **Manifest Index**: single walk records every `Cargo.toml`/`package.json`/`pyproject.toml`/`go.mod` for full repo mode; structure mode skips it entirely
- **Git status layers**: counts-only vs full file changes vs full unified diffs, selectable via `GitRequest` flags
- **Parallel remote fetch**: deep git mode fetches multiple remotes concurrently with bounded parallelism; `GIT_TERMINAL_PROMPT=0` is preserved to avoid interactive hangs
- **Ancestry-walk containment**: per-commit remote containment is computed with a single ancestry walk per remote, cached in a `HashMap<Oid, Vec<remote>>`
- **ExecutableIndex**: scans `PATH` + macOS bundles once, shared across all 8 program categories via `Arc`
- **CLI async model**: the CLI is a single-shot command runner; most paths are synchronous (git, filesystem, subprocess) and run directly in the async entrypoint. `spawn_blocking` is avoided unless true concurrent async work exists

## CLI

```bash
sniff                      # Show help
sniff --json               # Full system info (JSON output)
sniff hardware             # Hardware only (text output)
sniff cpu                  # Just CPU info
sniff audio-devices        # Audio input/output devices
sniff programs             # All programs
sniff editors              # Just editors
sniff editors install      # Install an editor (interactive)
sniff agents               # AI CLI tools
sniff services             # System services
sniff docs                 # Markdown documents
sniff topics               # Table of available topics
sniff just                 # Justfiles and recipes
sniff repo                 # Repository/monorepo structure
sniff repo git-status      # Git status with commit history
sniff repo language        # Primary programming language for the repository
sniff repo worktree        # Linked worktree name (exit 1 if main worktree)
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

## Detailed Topics

- [Programs](./programs.md) - 8 categories, macOS bundle detection
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities
- [Architecture](../../../sniff/docs/sniff-library-architecture.md) - Cost model, shared-work design

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference

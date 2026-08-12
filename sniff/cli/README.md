# Sniff CLI

**Sniff** is a comprehensive system and repository detection tool that provides detailed information about your operating system, hardware, network, and filesystem environment. It's designed to give developers, system administrators, and automation tools a complete snapshot of the execution context.

## Features

- **OS Detection**: Distribution, kernel, architecture, hostname, package managers, locale, timezone, and NTP synchronization status
- **Hardware Detection**: CPU (with SIMD capabilities), GPU (with Metal/Vulkan support), memory, storage, and audio devices
- **Network Detection**: Network interfaces with IPv4/IPv6 addresses, status flags, and WAN IP lookup
- **Filesystem Detection**: Git repository status, monorepo detection, programming language analysis, broad file associations, and EditorConfig formatting rules
- **Scoped Enrichment**: Refresh git remotes with `--refresh-remotes` and check registries with `--latest-versions`
- **Flexible Output**: Text (with verbosity levels) or JSON formats

## Installation

```bash
# Install from workspace root
just install

# Or install directly
cargo install --path sniff/cli
```

## Usage

### Basic Usage

```bash
# Show help (no subcommand, no flags)
sniff

# Full system info as JSON (requires --json without subcommand)
sniff --json

# Detect with a specific base directory
sniff --base /path/to/project

# Enable verbose output (show more details)
sniff -v        # Level 1: more details
sniff -vv       # Level 2: even more details
```

### Output Modes

The CLI has two output modes depending on whether a subcommand is used:

| Mode | Output | Use Case |
|------|--------|----------|
| No subcommand (`sniff`) | Help text | Shows available commands |
| No subcommand + `--json` (`sniff --json`) | JSON (all data) | Programmatic consumption, piping to `jq` |
| With subcommand (`sniff cpu`) | Text (default) | Human-readable output |

```bash
# Show help
sniff

# Full system info as JSON
sniff --json

# Subcommand with text output (default)
sniff cpu

# Subcommand with JSON output
sniff cpu --json
```

### Section Subcommands

Use subcommands to filter output to specific sections.

**Top-Level Sections:**

```bash
sniff os          # OS information (name, kernel, locale, timezone)
sniff runtime     # Native, WSL 1, or WSL 2 runtime
sniff hardware    # Hardware information (CPU, GPU, memory, storage)
sniff network     # Network information (interfaces, local IPs, WAN IP)
sniff filesystem  # Filesystem information (git, languages, monorepo)
```

**Discovery Tools:**

```bash
sniff topics      # Table of subsection topics
sniff structure   # Structural overview of sniff output
```

**Hardware Details:**

```bash
sniff cpu             # CPU information
sniff gpu             # GPU information
sniff memory          # Memory information
sniff storage         # Storage/disk information
sniff audio-devices   # Audio input/output devices
```

**Filesystem Details:**

```bash
sniff repo        # Repository/monorepo structure
sniff language    # Programming language results
sniff files       # Broad file association results
sniff docs        # Repository markdown documents
```

**Blast Radius:**

```bash
sniff blast-radius              # Docs affected by dirty source changes
sniff blast-radius staged       # Docs affected by staged changes
sniff blast-radius last-commit  # Docs affected by last commit
```

**Repository File Listing:**

```bash
sniff repo dirty-source-code     # Dirty source code files
sniff repo staged-source-code    # Staged source code files
sniff repo unstaged-source-code  # Unstaged source code files
sniff repo dirty-files           # All dirty files
sniff repo staged-files          # All staged files
sniff repo unstaged-files        # Unstaged modified files
sniff repo untracked-files       # Untracked files
```

**Repository Identity:**

```bash
sniff repo                 # Repository name (default text dispatch; --json returns the aggregate)
sniff repo name            # Repository name only
sniff repo name -v         # Repository name only (verbose styling; no foreign fields)
sniff repo is-monorepo     # Monorepo label (e.g. `cargo`; `false` if not). Exits non-zero when false unless `--no-error`. `--json` emits `{ "is_monorepo": true, "authority": "...", "orchestrators": [...] }` / `{ "is_monorepo": false }`
sniff repo package-count   # Number of discovered packages
sniff repo version         # Declared version(s) for the current package/area/repo context — scoped like `repo test-runner`; `--all`/`--package`/`--package-area` override the CWD scope; `--csv`/`--list`/`--md`/`--json`/`--verbose`
sniff repo language        # Primary programming language for the repository
```

**Repository Queries:**

```bash
sniff repo git-status                # Git status with commit history
sniff repo git-status --history 20   # Show more commits
sniff repo git-status --compact      # Show only the Status section
sniff repo hash HEAD                 # Show latest commit details
sniff repo remote origin             # Inspect remote repository
sniff repo pr                        # List open pull requests
sniff repo pr --status merged        # List merged pull requests
sniff repo pr --status draft --json  # Draft PRs as JSON
sniff repo pr -v                     # Verbose PR block output
sniff repo branches                  # Local branches from known refs
sniff repo branches --refresh-remotes # Refresh remotes before branch projection
sniff repo package-dependencies      # Internal workspace dependency list
sniff repo package-dependencies --ui # Mermaid dependency diagram
sniff repo dependencies              # External dependency list
sniff repo dependencies --dev-dependencies # External dev dependencies only
sniff repo packages                  # List all package names (CSV)
sniff repo packages --md             # Markdown unordered list
sniff repo packages --list           # Raw list (one per line)
sniff repo packages --package-area A # Restrict to a single area
sniff repo packages --verbose        # Annotate each entry with its root dir
sniff repo package                   # Package name for current directory
sniff repo package-area              # Package area for current directory
sniff repo dirty-packages            # Packages with uncommitted changes
sniff repo worktrees                 # List all worktrees (marks current with *)
sniff repo worktrees --md            # Markdown unordered list (one `- name` per line)
sniff repo worktrees --list          # Newline-delimited list (one name per line)
sniff repo worktrees --csv           # Comma-separated names on a single line
sniff repo worktrees --verbose       # Name, branch, and path for each worktree
sniff repo worktrees --json          # JSON output with full worktree metadata
sniff repo has-merge-conflict        # Check for merge conflicts
```

**Recent Commits and Changes:**

```bash
sniff repo recent-commits             # Commits from last 3 days (default)
sniff repo recent-commits 1w          # Commits from last week
sniff repo recent-commits today       # Today's commits
sniff repo recent-commits 10          # The last 10 commits
sniff repo source-code-changes 1w     # Source code changes in last week
sniff repo documentation-changes 1w   # Documentation changes in last week
# Filter by scope:
sniff repo recent-commits --package sniff
sniff repo recent-commits --package-area sniff
```

**Justfile Detection:**

```bash
sniff just                         # Detect justfiles and recipes
sniff just sniff                   # Filter to justfiles with "sniff" in path
sniff just --with build            # Show justfiles containing a "build" recipe
sniff just --with build --grouped  # Group justfiles sharing identical recipe bodies
sniff just -v                      # Show justfiles with recipe details
```

**File List Formatting:**

File-listing commands (`dirty-source-code`, `staged-files`, `blast-radius`, etc.) support:

```bash
--list           # Bullet list (one item per line with `- ` prefix)
--csv            # Comma-separated on a single line
--no-path        # Show only basename
--no-error       # Exit 0 with no output when no results (default: exit 1)
--on-error MSG   # Custom message when no results found
--package PKG    # Scope to a specific package
--package-area A # Scope to a specific package area
```

**Software Subcommands:**

```bash
sniff software                            # All installed software
sniff software editors                    # Editors (vim, VS Code, etc.)
sniff software utilities                  # CLI utilities (ripgrep, fzf, etc.)
sniff software language-package-managers  # Language package managers (cargo, npm, pip)
sniff software os-package-managers        # OS package managers (homebrew, apt, etc.)
sniff software tts-clients                # TTS clients (say, espeak, piper, etc.)
sniff software terminal-apps              # Terminal apps (alacritty, wezterm, etc.)
sniff software audio-players              # Headless audio players (afplay, pacat, etc.)
sniff software notification-helpers       # Desktop notification helpers
sniff software agents                     # AI agent/CLI tools (claude, kimi, etc.)
sniff software test-runners               # Test runners with availability details
```

**Program Installation:**

Eight of the ten detectable program categories support `install` and `install-plan`. Notification helpers and test runners are report-only and expose neither action.

```bash
sniff software editors install          # Interactive picker
sniff software editors install nvim     # Install a specific program
sniff software install                  # Pick from all installable categories
```

Each install attempt runs under a deadline. If the package manager is still
running when the deadline expires, sniff kills it and reports a **timeout**
rather than an ordinary install failure. The timeout warning is rendered after
the failure status and before any retry prompt, so you see it before deciding
whether to try another installation method.

A timed-out install may leave a **partial install** behind. On Unix, killing
the installer's process tree is best-effort: everything in the installer's
process group is terminated, but a descendant that forks and calls `setsid()`
between sniff's samples escapes that group and can keep running — and keep
modifying your system — after sniff has reported the timeout. Third-party
package managers and remote shell installers do sometimes fork and detach, so
treat a reported timeout as "state unknown" and re-check with
`sniff software <category>` before retrying. On Windows the installer is
confined to a kill-on-close Job Object, so termination is enforced by the
kernel and nothing escapes.

**Software Output:**

```bash
# Text output (default)
sniff software
# JSON output with full metadata
sniff software --json
```

**Services Subcommand:**

```bash
sniff services                   # Running services (default)
sniff services --state all       # All services
sniff services --state running   # Only running services
sniff services --state stopped   # Only stopped services
sniff services --json            # JSON output
```

### Scoped Enrichment Flags

Expensive repository checks are opt-in and scoped to the subcommands that can
actually report the extra data:

```bash
# Refresh local remote-tracking data before reporting git sync status
sniff repo git-status --refresh-remotes

# Query registries for latest dependency versions
sniff repo structure --latest-versions -v

# Combine both on the aggregate filesystem report
sniff filesystem --refresh-remotes --latest-versions
```

- `sniff repo git-status --refresh-remotes` and `sniff filesystem --refresh-remotes` add:

- Remote branch lists for each git remote
- Commit synchronization status across remotes
- Detection of whether local branch is behind remote

- `sniff repo structure --latest-versions` and `sniff filesystem --latest-versions` add:

- Latest version information for dependencies from package registries
- Package-level update summaries in text output
- `latest_version`, `is_updatable`, and `has_major_update` fields in JSON output

### Performance Reporting

The `--perf` flag appends structured performance timings to any command:

```bash
# Rich terminal output: perf appended to stdout
sniff software --perf
sniff repo --perf

# Scriptable text output: perf emitted to stderr (stdout stays clean)
sniff repo packages --perf        # CSV names on stdout, timings on stderr
sniff repo package-area --perf    # Area name on stdout, timings on stderr
sniff repo root --perf            # Path on stdout, timings on stderr

# JSON output: perf embedded in the JSON object
sniff repo packages --perf --json
```

Performance data includes total wall-clock time, per-stage timings (calls, total,
max, last), and aggregated counters (cache hits/misses, etc.).

## Output Examples

### Text Output (Default)

```
=== OS ===
Name: macOS 14.3.1
Kernel: Darwin 25.3.0
Architecture: aarch64
Hostname: macbook.local

Package Managers: Primary: homebrew (3 detected)

Locale: en_US.UTF-8 (UTF-8)

Timezone: America/Los_Angeles (PST, UTC-08:00)

=== Hardware ===
CPU: Apple M1 (8 logical cores)
Physical cores: 8
SIMD: NEON

Memory:
  Total: 16.0 GB
  Available: 8.5 GB
  Used: 7.5 GB

GPUs:
  Apple M1 (Apple, Metal)

Storage:
  / (apfs, SSD)

Network
• Primary interface: en0
• WAN IP address: 203.0.113.10
• Active interfaces: 2 of 12
• IPv4: 192.168.1.100
• IPv6: fe80::1

With `sniff network -v`, the command expands into a per-interface inventory and omits
empty IPv4/IPv6 lines for interfaces that do not carry those address families.

=== Filesystem ===
Languages (212 contributing files out of 1,234 scanned):
  Primary: Rust
  Secondary: TypeScript, JavaScript
  Rust: 145 direct, 0 framework (68.4%)
  TypeScript: 41 direct, 26 framework (31.6%)

Files (1,234 scanned):
  Programming language: 186 files (15.1%)
  Framework file: 26 files (2.1%)
  Configuration: 54 files (4.4%)
  Documentation: 18 files (1.5%)

Git Repository:
  Root: .
  Branch: main
  HEAD: 6e11484b (Ken Snyder)
  Message: style: apply consistent formatting across workspace
  Status: dirty (3 staged, 2 unstaged, 1 untracked)
  Remote origin: GitHub (15 branches)

Monorepo: cargo
  Packages: 8
    sniff-cli v0.1.0 (sniff/cli) [Rust]
    sniff v0.1.0 (sniff/lib) [Rust]
    research-cli v0.1.0 (research/cli) [Rust]
    ...
```

### JSON Output

```bash
# Full system info as JSON (no subcommand)
sniff | jq .
```

Returns a structured JSON object with all detection results:

```json
{
  "os": {
    "name": "macOS",
    "version": "14.3.1",
    "kernel": "Darwin 25.3.0",
    "arch": "aarch64",
    "hostname": "macbook.local",
    "system_package_managers": { ... },
    "locale": { ... },
    "time": { ... }
  },
  "hardware": {
    "cpu": { ... },
    "gpu": [ ... ],
    "memory": { ... },
    "storage": [ ... ]
  },
  "network": {
    "interfaces": [ ... ]
  },
  "filesystem": {
    "languages": { ... },
    "git": { ... },
    "repo": { ... }
  }
}
```

### Software Output

```bash
sniff software --json | jq .
```

Returns rich program entries (including metadata and detection status):

```json
[
  {
    "name": "Neovim",
    "binary_name": "nvim",
    "installed": true,
    "path": "/opt/homebrew/bin/nvim",
    "version": "0.10.4",
    "description": "Hyperextensible Vim-based text editor",
    "website": "https://neovim.io"
  }
]
```

### Services Output

```bash
sniff services --json | jq .
```

Returns init system and service list:

```json
{
  "init_system": "launchd",
  "services": [
    {"name": "com.apple.mDNSResponder", "running": true, "pid": 123},
    {"name": "com.docker.service", "running": true, "pid": 456}
  ]
}
```

### Repo Subcommand JSON Shapes

Every `sniff repo` subcommand emits a focused JSON shape that mirrors
its text-mode output — no two subcommands return identical JSON. The
table summarises the contract; see the per-subcommand docs under
`sniff/docs/cli/` for full schemas and examples.

| Subcommand | JSON shape |
|---|---|
| `repo` (bare, `--json`) | Consolidated `SniffRepo` aggregate with snake_case keys, grouped `context`, top-level `branches`/`worktrees`, and `dirty`/`staged`/`unstaged`/`untracked` scope buckets; see [`sniff/docs/topics/json-output.md`](../../docs/topics/json-output.md) |
| `repo structure` | Full `RepoInfo` blob (`is_monorepo`, `packages`, `dependencies`, ...). Includes `monorepo_standards` and `monorepo_layers` when the repo is a monorepo. |
| `repo name` | `{ "name": "..." }` |
| `repo language` | `{ "language": "..." \| null }` (or full language breakdown with `--breakdown`) |
| `repo is-monorepo` | `{ "is_monorepo": true, "authority": "...", "orchestrators": [...] }` / `{ "is_monorepo": false }` |
| `repo package-count` | `{ "package-count": N }` |
| `repo version` | `{ "versions": [ { version, packages, sources: [ { manifest, path, href, inherited, packages } ] } ] }` |
| `repo git-status` | `GitInfo` object directly (`repo_root`, `status`, `recent`, `branches`, ...) |
| `repo package-dependencies` | `{ packages: [{ name, depends_on, used_by, dependencies, dev_dependencies, ... }] }` |
| `repo dependencies` | `{ dependencies: [{ package, family, name, targeted_version, actual_version }] }` |
| `repo branches` | `[{ name, current, sha, remote_represented, upstream, ahead, behind }]` |
| `repo dirty-packages` | `{ scope: "dirty", kind: "packages", names: [...] }` |
| `repo dirty-package-areas` | `{ scope: "dirty", kind: "package_areas", names: [...] }` |
| `repo staged-packages` | `{ scope: "staged", kind: "packages", names: [...] }` |
| `repo staged-package-areas` | `{ scope: "staged", kind: "package_areas", names: [...] }` |
| `repo unstaged-packages` | `{ scope: "unstaged", kind: "packages", names: [...] }` |
| `repo unstaged-package-areas` | `{ scope: "unstaged", kind: "package_areas", names: [...] }` |
| `repo dirty-files` / `staged-files` / `staged-source-code` / `unstaged-source-code` / `dirty-source-code` | `{ scope, kind, paths: [...] }` |
| `repo packages` | `["pkg-a", "pkg-b", ...]` (array of strings) |
| `repo package-areas` | `["area-a", "area-b", ...]` (array of strings) |
| `repo package` / `package-area` | `{ name: "<value>" }` |
| `repo package-root` / `package-area-root` | `{ root: "<abs-path>" }` (empty string + exit 1 when not in scope) |
| `repo is-current-package-area-dirty` | `{ dirty: bool }` (exit 0 when true, 1 when false) |
| `repo package-area-has-source-code-changes` | `{ has_source_code_changes: bool }` (exit 0/1 mirror) |
| `repo has-merge-conflict` | `{ has_merge_conflict: bool }` (exit 0/1 mirror) |
| `repo recent-commits <period>` | Full `CommitDescSet` (no `filter` field) |
| `repo source-code-changes <period>` | Filtered `CommitDescSet` with `"filter": "source_code"` |
| `repo documentation-changes <period>` | Filtered `CommitDescSet` with `"filter": "documentation"` |
| `repo hash <ref>` | `{ commit: {...}, files: [...] }` |
| `repo root` | `{ root: "<abs-path>" }` |
| `repo remote <url-or-name>` | `RemoteReport` JSON |
| `repo pr` | Array of `PullRequest` JSON objects |
| `repo worktrees` | `{ worktrees: [{ name, branch, path, current, detached }] }` |
| `repo unstaged-files` / `untracked-files` | Array of `FileChange` objects |

`--perf --json` injects a top-level `performance` field into any
object-shaped output (everything above except the array shapes); for
array outputs (`packages`, `package-areas`, `pr`, file lists) the
array is wrapped in `{ data: [...], performance: {...} }`.

## Architecture

### CLI Layer (`sniff/cli`)

The CLI binary provides:

- **Argument Parsing**: Uses `clap` with derive API for clean, type-safe CLI definitions
- **Subcommand Filtering**: Use subcommands like `hardware`, `cpu`, `git` to show specific sections
- **Text Rendering**: Multi-level verbosity with human-readable formatting
- **JSON Serialization**: Full structured output for programmatic use
- **Scoped Enrichment**: Async registry lookups via `--latest-versions` and remote-aware git reporting via `--refresh-remotes`

**Key Files:**

- `main.rs` - CLI argument parsing with clap subcommands, detection flow
- `output.rs` - Output rendering for text and JSON formats

### Library Layer (`sniff/lib`)

The library provides modular detection across six domains:

**OS Module:**

- Distribution and version detection
- Package manager discovery (apt, homebrew, pacman, etc.)
- Locale and timezone information
- NTP synchronization status (macOS: `sntp`, Windows: `w32tm`, Linux: `timedatectl`)

**Hardware Module:**

- CPU: Brand, core count, SIMD capabilities (AVX, SSE, NEON)
- GPU: Metal/Vulkan backend detection, capabilities (raytracing, mesh shaders)
- Memory: Total, available, and used bytes
- Storage: Disk type (SSD/HDD), filesystem, mount points
- Audio: Input/output device enumeration with sample rates (macOS/Linux/Windows)

**Network Module:**

- Interface enumeration with permission handling
- IPv4 and IPv6 address collection
- Interface flags (up/down, loopback)

**Filesystem Module:**

1. **Git Detection** (`filesystem/git/`):
   - Repository root and current branch
   - Commit history with author/message
   - Dirty file tracking with diffs
   - Worktree detection and status
   - Remote provider detection (GitHub, GitLab, etc.)
   - Optional remote refresh: branch inventory, default branch, behind status, commit containment

2. **Repository Detection** (`filesystem/repo/`):
   - Monorepo standard detection (Cargo, pnpm/npm/Yarn/Bun workspaces, uv, Go, Gradle, Maven, .NET, Bazel, Pants, Buck2, Rush Stack, Nx, Turborepo, Lerna)
   - Authority-vs-orchestrator topology via `monorepo_standards` and `monorepo_layers`
   - Resolved acting binary (`Path`, `Wrapper`, or missing) and version satisfaction
   - Package enumeration with glob pattern expansion
   - Per-package language detection
   - Per-package dependency manager detection (cargo, npm, pnpm, yarn, pip, go)
   - Dependency parsing from `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, and `go.mod`

3. **Language Analysis** (`filesystem/languages.rs`):
   - File extension-based language detection
   - Percentage breakdown by file count
   - Primary language identification (excludes markup/config)

4. **File Type Classification** (`filesystem/file_types/`):
   - Broad file association (programming, framework, config, styling, docs, data, image, etc.)
   - Registry-based classification with framework detection

5. **Blast Radius Analysis** (`filesystem/blast_radius.rs`):
   - Impact analysis: which docs are affected by source code changes
   - Supports dirty, staged, and last-commit scopes

6. **Justfile Detection** (`filesystem/just.rs`):
   - Discovers justfiles across the repository
   - Parses recipe names and bodies

7. **Dependency Enrichment** (`package/network.rs`):
   - Async queries to package registries (crates.io, npm, PyPI)
   - Latest version resolution for `--latest-versions`
   - Manager-specific network implementations (Cargo, npm, pnpm, Yarn, Bun)

**Programs Module:**

Detects installed programs across 10 categories with parallel execution:

- **Editors**: vim, VS Code, Cursor, IntelliJ, Sublime, etc.
- **Utilities**: ripgrep, fzf, bat, jq, fd, delta, etc.
- **Language Package Managers**: cargo, npm, pip, poetry, go, etc.
- **OS Package Managers**: homebrew, apt, dnf, pacman, etc.
- **TTS Clients**: say, espeak, piper, etc.
- **Terminal Apps**: alacritty, wezterm, kitty, iTerm2, etc.
- **Headless Audio**: afplay, pacat, aplay, etc.
- **AI CLI Tools**: claude, aider, goose, etc.
- **Notification Helpers**: notify-send, terminal-notifier, dunstify, etc.
- **Test Runners**: cargo test, vitest, pytest, go test, etc.

Features:
- macOS app bundle detection (checks `/Applications` when PATH fails)
- `ExecutableSource` tracking (PATH vs macOS bundle)
- Version extraction via multiple strategies
- Rich metadata: display name, description, website

**Services Module:**

Detects system services across multiple init systems:

- **Supported**: systemd, launchd, OpenRC, runit, S6, Dinit, Windows SCM
- **Capabilities**: Service listing, state filtering (running/stopped), PID tracking
- **Evidence**: Tracks detection method for debugging

### Package Manager Abstraction (`package/mod.rs`)

Unified type system for package managers:

- **`OsPackageManager`**: 40+ system package managers (apt, homebrew, pacman, etc.)
- **`LanguagePackageManager`**: 70+ language ecosystem managers (cargo, npm, pip, etc.)
- **`PackageManager`**: Wrapper enum for unified handling
- **`PackageManagerShape`**: Trait for dyn-compatible package operations

**Registry System** (`package/registry.rs`):

- Global static registry of available package managers
- Runtime availability checking via PATH lookup
- Network-backed version resolution

## Technical Details

### Detection Plan API

The library provides three levels of API for detection control:

```rust
// Tier 1: Convenience (all defaults)
let result = sniff::detect()?;

// Tier 2: Plan-based (fine-grained control)
use sniff::{detect_with_plan, request::*};
let plan = DetectionPlan::new()
    .os(OsRequest::summary())
    .hardware(HardwareRequest::summary())
    .without_network();
let result = detect_with_plan(plan)?;

// Legacy: SniffConfig (coarser boolean toggles, still supported)
let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .skip_network();
let result = sniff::detect_with_config(config)?;
```

See [../docs/sniff-library-architecture.md](../docs/sniff-library-architecture.md) for the per-subsection cost table, shared-work strategies, and common caller profiles.

### Verbosity Levels

Text output supports three verbosity levels:

- **Level 0** (default): Summary information, top 5 items in lists
- **Level 1** (`-v`): Extended details, full package lists, recent commits
- **Level 2** (`-vv`): Maximum detail, file lists, git diffs, EditorConfig sections

### Subcommand-Based Filtering

The CLI uses subcommands for filtering (not flags):

```bash
# Correct: use subcommands
sniff hardware
sniff cpu
sniff git

# Incorrect (old flag-based syntax, no longer supported)
# sniff --hardware
# sniff --cpu
# sniff --git
```

Each subcommand outputs text by default. Use `--json` for JSON output:

```bash
sniff cpu --json
```

### Dependency Enrichment

With `--latest-versions`, sniff enriches dependency information:

1. Parses `Cargo.toml` dependencies (normal, dev, build)
2. Extracts version requirements and features
3. Queries package registries asynchronously
4. Populates `latest_version` field in `DependencyEntry`

Supports:

- **Rust**: crates.io via API
- **JavaScript/TypeScript**: npm registry
- **Python**: PyPI JSON API
- **PHP**: Packagist search API
- **Lua**: LuaRocks HEAD requests
- **Go**: pkg.go.dev HEAD requests

### Remote Refresh

Remote-aware git inspection (`--refresh-remotes`) refreshes local remote-tracking data and reports:

- Fetches branch lists for each remote
- Determines which remotes have each recent commit
- Detects if local branch is behind remote
- Network-bound operations with error handling

### Remote Repository Inspection

The `sniff repo remote` subcommand inspects remote repositories via hosting provider APIs:

```bash
# Inspect a GitHub repository
sniff repo remote https://github.com/rust-lang/cargo

# SSH URLs also work
sniff repo remote git@github.com:rust-lang/cargo.git

# GitLab (including nested groups)
sniff repo remote https://gitlab.com/inkscape/inkscape

# Gitea/Codeberg
sniff repo remote https://codeberg.org/forgejo/forgejo

# Bitbucket
sniff repo remote https://bitbucket.org/atlassian/python-bitbucket

# JSON output
sniff repo remote https://github.com/rust-lang/cargo --json
```

**From within a repository**, inspect a configured remote by name:

```bash
# Fetch remote info for 'origin'
sniff repo remote origin

# Or specify a URL directly
sniff repo remote https://github.com/rust-lang/cargo

# List pull requests
sniff repo pr
sniff repo pr --status merged
sniff repo pr --status draft --json
sniff repo pr -v
```

**Output includes:**

- Repository metadata (stars, forks, language, license)
- Open pull requests/merge requests
- Open issues
- Tags and releases
- CI/CD configuration detection
- Key URLs (issues, PRs, wiki, CI/CD, releases)

**Environment Variables:**

| Provider | Auth Env Vars |
|----------|---------------|
| GitHub | `GITHUB_TOKEN` or `GH_TOKEN` |
| GitLab | `GITLAB_TOKEN` or `GITLAB_PRIVATE_TOKEN` |
| Gitea | `GITEA_TOKEN` (or `CODEBERG_TOKEN` for Codeberg) |
| Bitbucket | `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` |

### Authentication Strategy

Sniff uses an **attempt-unauthenticated-first** strategy for remote repository
inspection (`sniff repo remote`, `sniff repo pr`):

1. The CLI first issues the API request **anonymously**, with no credentials
   attached — even if relevant token environment variables happen to be set
   on the calling shell. This lets public-repo lookups succeed without any
   configuration.
2. Only if the unauthenticated request is rejected with a `401 Unauthorized`,
   `403 Forbidden`, or rate-limit error does the CLI surface a
   `MissingCredentials` error advising the user which environment variable
   to set.
3. If credentials *are* provided but the API rejects them, a separate
   `InvalidCredentials` error is shown.

This means the typical user never has to provide a token to query a public
repository, and only sees the credential prompt when the resource genuinely
requires one (private repos, rate limits, etc.).

### Pull Request Limitations by Provider

| Limitation | Affects | Notes |
|---|---|---|
| Draft PR filter (`--status draft`) | **Bitbucket Cloud** | Returns an empty list. Bitbucket Cloud has no native concept of draft pull requests. |
| Verbose body / description | **Bitbucket Cloud** | The list-pull-requests endpoint does not return PR descriptions. The `body` field is `None` for Bitbucket PRs in both default and verbose output. (A per-PR follow-up fetch is a planned enhancement.) |
| Labels | **Bitbucket Cloud** | The Bitbucket Cloud PR API does not expose labels (labels exist on Issues only). The `labels` field is always empty for Bitbucket PRs. |

### Error Handling

- Library uses `thiserror` for structured error types
- CLI displays user-friendly error messages
- Network errors with `--refresh-remotes` and `--latest-versions` are graceful (shows available data)
- Permission denials for network interfaces are handled explicitly

## Development

### Project Structure

```
sniff/
├── cli/              # Binary crate (this package)
│   ├── src/
│   │   ├── main.rs       # Entry point, tracing initialization
│   │   ├── args.rs       # Clap subcommands and argument parsing
│   │   ├── commands.rs   # Command execution logic
│   │   ├── install.rs    # Program installation interface
│   │   └── output/       # Text/JSON rendering with per-topic modules
│   │       ├── mod.rs, os.rs, hardware.rs, network.rs, filesystem.rs
│   │       ├── programs.rs, services.rs, remote.rs, recent_commits.rs
│   │       └── topics.rs, just.rs
│   └── Cargo.toml
├── lib/              # Library crate
│   ├── src/
│   │   ├── lib.rs            # Public API: detect(), SniffConfig, DetectionPlan
│   │   ├── request.rs        # Fine-grained detection control types
│   │   ├── error.rs          # Error types
│   │   ├── os/               # OS detection (distro, locale, time, package managers)
│   │   ├── hardware/         # CPU, GPU, memory, storage, audio devices
│   │   ├── network/          # Network interfaces
│   │   ├── filesystem/       # Git, repo, languages, docs, blast radius, file types, just
│   │   │   ├── git/          # Git repo detection, recent commits
│   │   │   ├── repo/         # Monorepo and package detection
│   │   │   ├── file_types/   # Broad file type classification
│   │   │   └── ...           # languages, docs, formatting, blast_radius, just
│   │   ├── package/          # Package manager abstraction (110+)
│   │   ├── programs/         # Program detection (10 categories; 8 installable)
│   │   ├── remote/           # Remote repo inspection (GitHub, GitLab, Gitea, Bitbucket)
│   │   └── services/         # Init system and service detection
│   └── Cargo.toml
└── docs/             # Package documentation
```

### Key Dependencies

- **`clap`** (4): Command-line argument parsing with derive API
- **`clap_complete`** (4): Dynamic shell completions
- **`serde/serde_json`** (1.0): Serialization for JSON output and parsing
- **`tokio`** (1): Async runtime for network operations
- **`sniff`** (workspace): Core detection library with network and remote features
- **`biscuit-terminal`** (workspace): Terminal capability detection and styled rendering
- **`darkmatter`** (workspace): Graph/visualization for dependency diagrams
- **`strum`** (0.28): Enum iteration for program categories
- **`inquire`** (0.9): Interactive prompts for program installation
- **`tracing/tracing-subscriber`**: Structured logging with verbosity levels

### Testing

```bash
# Run all tests
cargo test -p sniff-cli
cargo test -p sniff

# Test CLI parsing
cargo test -p sniff-cli cli_parsing
```

## Use Cases

### CI/CD Integration

```bash
# Capture build environment metadata as JSON
sniff --json > build-context.json

# Check if running in a monorepo
if sniff repo is-monorepo --no-error | grep -q '^cargo$'; then
    echo "Detected cargo monorepo"
fi

# The bare aggregate uses the new snake_case key
if sniff repo --json | jq -e '.is_monorepo'; then
    echo "Detected monorepo via aggregate"
fi
```

### Development Environment Setup

```bash
# Check available package managers
sniff os -v | grep "Package Managers"

# Verify GPU support before running ML workloads
sniff gpu --json | jq '.[0].capabilities'
```

### System Inventory

```bash
# Full system report as JSON
sniff --json > system-report.json

# Quick hardware summary (text output)
sniff hardware
```

### Repository Analysis

```bash
# Analyze codebase languages
sniff language -v

# Refresh remote tracking state before printing git status
sniff repo git-status --refresh-remotes

# Inspect dependencies with latest versions
sniff repo structure --latest-versions --json | jq '.packages[].dependencies'

# Recent commits and changes
sniff repo recent-commits 1w
sniff repo source-code-changes today

# Find docs affected by current changes
sniff blast-radius
```

## Shell Completions

Dynamic shell completions are supported for bash, zsh, fish, powershell, and elvish:

```bash
# Bash (add to ~/.bashrc)
source <(COMPLETE=bash sniff)

# Zsh (add to ~/.zshrc)
source <(COMPLETE=zsh sniff)

# Fish (add to ~/.config/fish/config.fish)
COMPLETE=fish sniff | source
```

Completions include subcommand names, flag values, package names, and program names.

## Platform Support

`sniff` supports **macOS**, **Linux**, and **Windows**. **WSL** is treated as the
Linux compile and runtime path — under WSL, sniff uses the same `/proc`-backed
detectors as native Linux and never crosses into native Windows behavior. See the
[library README](../lib/README.md#platform-support) for the per-detector support
matrix.

## Limitations

- **Network detection** requires appropriate permissions (may fail on restricted systems)
- **`--latest-versions`** requires network access to package registries (scoped to `repo` and `filesystem` subcommands)
- **`--refresh-remotes`** may contact configured git remotes (scoped to `repo git-status` and `filesystem` subcommands)
- **Monorepo detection** is limited to known tools (Cargo, npm, pnpm, yarn, Nx, Turborepo, Lerna)
- **Language detection** is file extension-based (no content analysis)

## Future Enhancements

Planned features:

- Lockfile-based resolved version extraction for npm/pnpm/yarn
- GPU detection for Windows (D3D12) and Linux (Vulkan)
- Runtime environment detection (Docker, VM, cloud providers)

## License

Part of the Rusty Biscuit monorepo. See top-level LICENSE file.

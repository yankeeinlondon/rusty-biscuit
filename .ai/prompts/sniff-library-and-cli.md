# Sniff CLI and Library

A new module has been added to this monorepo called Sniff (`/sniff`) which contains both a library module (`/sniff/lib`) and a CLI module (`/sniff/cli`).

- The CLI leverages the `clap` crate for structuring and CLI functionality, then delegates to the Sniff library.
- See `sniff/DESIGN.md` for the complete type system and architecture.

The primary utility of this library is to detect information about the host system in three main areas:

1. Hardware
2. Network
3. Filesystem/Repo Info (based on a directory, CWD as default)

---

## Crate Dependencies

### Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `sysinfo` | 0.37+ | CPU, memory, disk, process info (cross-platform) |
| `getifaddrs` | 0.6+ | Network interface enumeration |
| `git2` | 0.20+ | Git repository operations via libgit2 |
| `hyperpolyglot` | 0.1+ | Programming language detection (replaces rust-code-analysis) |
| `semver` | 1.0+ | Cargo-style version parsing |
| `nodejs-semver` | 4.0+ | npm/pnpm/yarn version range parsing |
| `package-parser` | 0.1+ | Multi-format lockfile parsing |
| `serde` / `serde_json` | 1.0 | Serialization |
| `thiserror` | 2.0+ | Error types |
| `walkdir` | 2.0+ | Directory traversal |
| `url` | 2.5+ | URL parsing for git remotes |
| `reqwest` | 0.12+ | Registry API HTTP client |

### Optional/Feature-Gated

| Crate | Feature | Purpose |
|-------|---------|---------|
| `hardware-query` | `gpu` | Detailed GPU info (driver dependent) |
| `rayon` | `parallel` | Parallel file scanning |
| `tokio` | `async` | Async runtime for registry queries |

### Dependency Changes from Original

| Original | Replacement | Reason |
|----------|-------------|--------|
| `rust-code-analysis` | `hyperpolyglot` | rust-code-analysis requires language to be specified upfront; hyperpolyglot auto-detects using GitHub Linguist heuristics |
| `hardware-query` alone | `sysinfo` + optional `hardware-query` | sysinfo is more mature and cross-platform; hardware-query adds GPU details |

---

## Functionality

### Hardware

**Crates:** `sysinfo` (primary), `hardware-query` (GPU details, feature-gated)

#### Data Gathered

| Category | Fields | Source |
|----------|--------|--------|
| OS | name, version, kernel, arch, hostname | `sysinfo::System` |
| CPU | brand, physical/logical cores, frequency, architecture | `sysinfo::System::cpus()` |
| Memory | total, available, used, swap total/used | `sysinfo::System::memory()` |
| Storage | mount point, fs type, total, available, removable | `sysinfo::System::disks()` |
| GPU | name, vendor, VRAM, driver version | `hardware-query` (feature) |

#### API

```rust
/// Fast hardware detection (no CPU usage sampling)
pub fn detect_hardware() -> Result<HardwareInfo, SniffError>;

/// Hardware detection with CPU usage (~1s sampling delay)
pub fn detect_hardware_with_usage() -> Result<HardwareInfo, SniffError>;
```

#### Implementation Notes

- Use `System::new_with_specifics()` with targeted `RefreshKind` for performance
- CPU usage requires two samples with 1-second delay; make this opt-in via `detect_hardware_with_usage()`
- GPU detection via `hardware-query` is feature-gated (`gpu`) due to driver dependencies
- Expected time: <10ms without CPU usage

---

### Network

**Crate:** `getifaddrs`

#### Data Gathered

| Field | Type | Notes |
|-------|------|-------|
| Interface name | `String` | "en0", "eth0", "wlan0" |
| Interface index | `u32` | OS-assigned index |
| MAC address | `Option<[u8; 6]>` | None on virtual interfaces |
| IPv4 addresses | `Vec<Ipv4Network>` | Address + netmask + broadcast |
| IPv6 addresses | `Vec<Ipv6Network>` | Address + prefix + scope |
| Flags | `InterfaceFlags` | up, loopback, running, multicast, broadcast |

#### API

```rust
/// Enumerate all network interfaces
pub fn detect_network() -> Result<NetworkInfo, SniffError>;

/// Filter interfaces
pub fn detect_network_filtered(filter: InterfaceFilter) -> Result<NetworkInfo, SniffError>;

pub struct InterfaceFilter {
    pub include_loopback: bool,    // default: false
    pub include_down: bool,        // default: false
    pub ipv4_only: bool,
    pub ipv6_only: bool,
}
```

#### Implementation Notes

- `getifaddrs::getifaddrs()` returns an iterator over all interfaces
- Primary interface heuristic: first non-loopback interface with IPv4 address
- Cross-platform: works on Linux, macOS, *BSD, Windows
- Expected time: <5ms

---

### Filesystem/Repo Info

This area is sensitive to a particular directory. By default CWD is used, but the library accepts any path and the CLI uses `--base`/`-b` to override.

#### Programming Languages

**Crate:** `hyperpolyglot` (replaces `rust-code-analysis`)

##### Why hyperpolyglot?

- **Auto-detection**: Detects language from filename, extension, and content heuristics
- **GitHub Linguist compatibility**: Uses same detection rules as GitHub
- **Directory analysis**: `get_language_breakdown()` analyzes entire directories
- **No upfront specification**: Unlike rust-code-analysis, doesn't require language to be specified

##### API

```rust
pub struct LanguageBreakdown {
    pub languages: Vec<LanguageStats>,  // Sorted by file count descending
    pub primary: Option<PrimaryLanguage>,
    pub total_files: usize,
}

pub struct LanguageStats {
    pub language: String,       // "Rust", "TypeScript", "Python"
    pub file_count: usize,
    pub percentage: f32,        // 0.0 - 100.0
    pub detection_method: DetectionMethod,
}

pub enum DetectionMethod {
    Extension,      // Detected by file extension
    Filename,       // Detected by filename (e.g., "Makefile")
    Heuristics,     // Content-based detection
    Classifier,     // ML classifier fallback
}
```

##### Implementation

```rust
use hyperpolyglot::{get_language_breakdown, Detection};

pub fn detect_languages(root: &Path) -> Result<LanguageBreakdown, SniffError> {
    let breakdown = get_language_breakdown(root)?;

    // Filter out vendor directories, generated files
    let filtered = breakdown
        .into_iter()
        .filter(|(_, files)| !is_vendor_path(&files[0].1))
        .collect();

    // Calculate percentages and determine primary
    // Shell scripts are noted but deprioritized unless dominant (>50%)
    // ...
}
```

##### Filtering Rules

- **Exclude**: `vendor/`, `node_modules/`, `target/`, `.git/`, `dist/`, `build/`
- **Exclude generated**: `*.generated.*`, `*.g.dart`, `*.pb.go`
- **Deprioritize**: Shell scripts unless >50% of codebase
- **Early termination**: Stop after 10,000 files for performance

---

#### Code Analysis

**Crate:** `rust-code-analysis`

After detecting languages with `hyperpolyglot`, use `rust-code-analysis` for deeper code metrics. This is a two-phase approach:

1. **Phase 1** (hyperpolyglot): Detect what languages exist and their distribution
2. **Phase 2** (rust-code-analysis): Analyze code quality metrics for detected languages

##### Supported Languages

`rust-code-analysis` uses tree-sitter parsers and supports:

| Language | tree-sitter grammar |
|----------|---------------------|
| Rust | `tree-sitter-rust` |
| C / C++ | `tree-sitter-c`, `tree-sitter-cpp` |
| Python | `tree-sitter-python` |
| JavaScript | `tree-sitter-javascript` |
| TypeScript | `tree-sitter-typescript` |
| Java | `tree-sitter-java` |
| Go | `tree-sitter-go` |
| Kotlin | `tree-sitter-kotlin` |
| Mozjs (SpiderMonkey) | custom |
| Preproc (C preprocessor) | custom |

##### Metrics Provided

| Metric | Description | Granularity |
|--------|-------------|-------------|
| **SLOC** | Source Lines of Code (non-blank, non-comment) | file, function, class |
| **PLOC** | Physical Lines of Code (all lines) | file, function, class |
| **LLOC** | Logical Lines of Code (statements) | file, function, class |
| **CLOC** | Comment Lines of Code | file, function, class |
| **BLANK** | Blank lines | file, function, class |
| **Cyclomatic** | Cyclomatic complexity (decision points + 1) | function |
| **Cognitive** | Cognitive complexity (nesting-aware) | function |
| **NOM** | Number of Methods | class |
| **NARGS** | Number of Arguments | function |
| **NEXITS** | Number of Exit points | function |
| **Halstead** | Halstead metrics (volume, difficulty, effort) | function |
| **MI** | Maintainability Index | file, function |

##### Data Structures

```rust
/// Code analysis results for a codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    /// Per-language metrics
    pub languages: Vec<LanguageMetrics>,
    /// Aggregate metrics across all code
    pub summary: CodeSummary,
    /// Files with concerning metrics (optional, opt-in)
    pub hotspots: Option<Vec<CodeHotspot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetrics {
    pub language: String,
    pub file_count: usize,
    pub metrics: AggregateMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregateMetrics {
    /// Total source lines of code
    pub sloc: usize,
    /// Total physical lines
    pub ploc: usize,
    /// Total comment lines
    pub cloc: usize,
    /// Total blank lines
    pub blank: usize,
    /// Comment ratio (cloc / sloc)
    pub comment_ratio: f32,
    /// Average cyclomatic complexity per function
    pub avg_cyclomatic: f32,
    /// Average cognitive complexity per function
    pub avg_cognitive: f32,
    /// Max cyclomatic complexity (worst function)
    pub max_cyclomatic: u32,
    /// Max cognitive complexity (worst function)
    pub max_cognitive: u32,
    /// Total function count
    pub function_count: usize,
    /// Average function length (SLOC)
    pub avg_function_sloc: f32,
    /// Maintainability index (0-100, higher is better)
    pub maintainability_index: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSummary {
    pub total_sloc: usize,
    pub total_files: usize,
    pub total_functions: usize,
    pub primary_language_sloc_percent: f32,
    pub avg_maintainability: Option<f32>,
}

/// A file or function with concerning metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeHotspot {
    pub path: PathBuf,
    pub function_name: Option<String>,
    pub line: usize,
    pub reason: HotspotReason,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotspotReason {
    HighCyclomatic,     // > 10
    HighCognitive,      // > 15
    LongFunction,       // > 100 SLOC
    LowMaintainability, // MI < 20
    ManyArguments,      // > 5 args
}
```

##### Language Mapping (hyperpolyglot → rust-code-analysis)

```rust
use rust_code_analysis::LANG;

/// Map hyperpolyglot language name to rust-code-analysis Lang
pub fn to_rca_lang(hyperpolyglot_name: &str) -> Option<LANG> {
    match hyperpolyglot_name {
        "Rust" => Some(LANG::Rust),
        "C" => Some(LANG::C),
        "C++" => Some(LANG::Cpp),
        "Python" => Some(LANG::Python),
        "JavaScript" => Some(LANG::Javascript),
        "TypeScript" | "TSX" => Some(LANG::Typescript),
        "Java" => Some(LANG::Java),
        "Go" => Some(LANG::Go),
        "Kotlin" => Some(LANG::Kotlin),
        _ => None, // Language not supported by rust-code-analysis
    }
}
```

##### Implementation

```rust
use rust_code_analysis::{metrics, FuncSpace, LANG};
use std::path::Path;

/// Analyze a single file and return metrics
pub fn analyze_file(path: &Path, lang: LANG) -> Result<FileMetrics, SniffError> {
    let source = std::fs::read(path)?;

    // Parse and compute metrics
    let space = metrics(&source, path, lang)
        .ok_or_else(|| SniffError::ParseFailed(path.to_path_buf()))?;

    // Extract metrics from the root space
    let metrics = extract_metrics(&space);

    Ok(FileMetrics {
        path: path.to_path_buf(),
        language: lang.to_string(),
        metrics,
        functions: extract_function_metrics(&space),
    })
}

fn extract_metrics(space: &FuncSpace) -> AggregateMetrics {
    let m = &space.metrics;
    AggregateMetrics {
        sloc: m.loc.sloc() as usize,
        ploc: m.loc.ploc() as usize,
        cloc: m.loc.cloc() as usize,
        blank: m.loc.blank() as usize,
        comment_ratio: if m.loc.sloc() > 0.0 {
            m.loc.cloc() / m.loc.sloc()
        } else {
            0.0
        },
        avg_cyclomatic: m.cyclomatic.cyclomatic_avg(),
        avg_cognitive: m.cognitive.cognitive_avg(),
        max_cyclomatic: m.cyclomatic.cyclomatic_max() as u32,
        max_cognitive: m.cognitive.cognitive_max() as u32,
        function_count: count_functions(space),
        avg_function_sloc: compute_avg_function_sloc(space),
        maintainability_index: Some(m.mi.mi_original()),
    }
}

/// Analyze entire codebase using detected languages
pub fn analyze_code(
    root: &Path,
    language_breakdown: &LanguageBreakdown,
    config: &AnalysisConfig,
) -> Result<CodeAnalysis, SniffError> {
    let mut languages = Vec::new();

    for lang_stats in &language_breakdown.languages {
        // Skip languages not supported by rust-code-analysis
        let Some(rca_lang) = to_rca_lang(&lang_stats.language) else {
            continue;
        };

        // Analyze files for this language
        let metrics = analyze_language_files(root, &lang_stats.language, rca_lang, config)?;
        languages.push(LanguageMetrics {
            language: lang_stats.language.clone(),
            file_count: lang_stats.file_count,
            metrics,
        });
    }

    let summary = compute_summary(&languages, language_breakdown);
    let hotspots = if config.detect_hotspots {
        Some(find_hotspots(&languages, config)?)
    } else {
        None
    };

    Ok(CodeAnalysis { languages, summary, hotspots })
}
```

##### Configuration

```rust
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Compute detailed per-function metrics (slower)
    pub per_function_metrics: bool,
    /// Find code hotspots (high complexity, low maintainability)
    pub detect_hotspots: bool,
    /// Thresholds for hotspot detection
    pub thresholds: HotspotThresholds,
    /// Max files to analyze (for performance)
    pub max_files: Option<usize>,
    /// Skip test files
    pub skip_tests: bool,
}

#[derive(Debug, Clone)]
pub struct HotspotThresholds {
    pub cyclomatic: u32,        // default: 10
    pub cognitive: u32,         // default: 15
    pub function_sloc: usize,   // default: 100
    pub maintainability: f32,   // default: 20.0
    pub max_args: usize,        // default: 5
}

impl Default for HotspotThresholds {
    fn default() -> Self {
        Self {
            cyclomatic: 10,
            cognitive: 15,
            function_sloc: 100,
            maintainability: 20.0,
            max_args: 5,
        }
    }
}
```

##### Performance Considerations

| Operation | ~Time (1000 files) | Notes |
|-----------|-------------------|-------|
| File-level metrics only | 2-5s | SLOC, CLOC, basic counts |
| With function metrics | 5-10s | Per-function complexity |
| With hotspot detection | 5-10s | Same as above, just filtering |
| Full analysis + Halstead | 10-20s | Expensive token analysis |

**Optimization strategies:**

- **Parallel processing**: Use `rayon` for parallel file analysis
- **Sampling**: For large repos (>10k files), sample representative files
- **Caching**: Cache results keyed by file path + mtime
- **Incremental**: Only re-analyze changed files (integrate with git status)
- **Feature gate**: Make code analysis opt-in (`--analyze` flag)

##### CLI Integration

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// Code quality analysis
    Analyze {
        /// Include per-function metrics
        #[arg(long)]
        functions: bool,

        /// Find code hotspots (high complexity)
        #[arg(long)]
        hotspots: bool,

        /// Only analyze specific language
        #[arg(long)]
        language: Option<String>,

        /// Max files to analyze
        #[arg(long, default_value = "5000")]
        max_files: usize,
    },
}
```

---

#### Git Info

**Crate:** `git2`

##### Data Structure

```rust
pub struct GitInfo {
    pub repo_root: PathBuf,
    pub current_branch: Option<String>,
    pub head_commit: Option<CommitInfo>,
    pub status: RepoStatus,
    pub remotes: Vec<RemoteInfo>,
    pub branches: BranchSummary,
}

pub struct RepoStatus {
    pub is_dirty: bool,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub conflicts_count: usize,
    pub files: Option<Vec<FileStatus>>,  // Opt-in for performance
}

pub struct RemoteInfo {
    pub name: String,               // "origin", "upstream"
    pub url: String,
    pub hosting_provider: HostingProvider,
}

pub enum HostingProvider {
    GitHub,
    GitLab,
    Bitbucket,
    AzureDevOps,
    AwsCodeCommit,
    Gitea,
    Forgejo,
    SourceHut,
    SelfHosted,     // Git server, unknown provider
    Unknown,
}
```

##### Implementation

```rust
use git2::{Repository, StatusOptions};

pub fn detect_git(path: &Path) -> Result<Option<GitInfo>, SniffError> {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => return Ok(None),  // Not a git repo
    };

    let repo_root = repo.workdir()
        .ok_or(SniffError::NotARepository(path.to_path_buf()))?
        .to_path_buf();

    // Current branch
    let head = repo.head().ok();
    let current_branch = head.as_ref()
        .and_then(|h| h.shorthand())
        .map(String::from);

    // Status counts
    let statuses = repo.statuses(Some(
        StatusOptions::new()
            .include_untracked(true)
            .recurse_untracked_dirs(false)
    ))?;

    let status = RepoStatus {
        is_dirty: !statuses.is_empty(),
        staged_count: statuses.iter().filter(|s| s.status().is_index_new() || s.status().is_index_modified()).count(),
        unstaged_count: statuses.iter().filter(|s| s.status().is_wt_modified()).count(),
        untracked_count: statuses.iter().filter(|s| s.status().is_wt_new()).count(),
        conflicts_count: statuses.iter().filter(|s| s.status().is_conflicted()).count(),
        files: None,  // Opt-in
    };

    // Remotes
    let remote_names = repo.remotes()?;
    let remotes: Vec<RemoteInfo> = remote_names.iter()
        .filter_map(|name| name)
        .filter_map(|name| repo.find_remote(name).ok())
        .map(|remote| RemoteInfo {
            name: remote.name().unwrap_or("").to_string(),
            url: remote.url().unwrap_or("").to_string(),
            hosting_provider: HostingProvider::from_url(remote.url().unwrap_or("")),
        })
        .collect();

    // Branches with ahead/behind
    let branches = detect_branches(&repo)?;

    Ok(Some(GitInfo { repo_root, current_branch, head_commit, status, remotes, branches }))
}
```

##### Hosting Provider Detection

```rust
impl HostingProvider {
    pub fn from_url(url: &str) -> Self {
        let normalized = url
            .trim_start_matches("git@")
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("ssh://git@");

        match normalized {
            s if s.starts_with("github.com") => Self::GitHub,
            s if s.starts_with("gitlab.com") => Self::GitLab,
            s if s.starts_with("bitbucket.org") => Self::Bitbucket,
            s if s.contains("dev.azure.com") || s.contains("visualstudio.com") => Self::AzureDevOps,
            s if s.contains("codecommit") && s.contains("amazonaws.com") => Self::AwsCodeCommit,
            s if s.contains("sr.ht") => Self::SourceHut,
            s if s.contains(".") => Self::SelfHosted,  // Has domain, assume git server
            _ => Self::Unknown,
        }
    }
}
```

---

#### Repo Root Detection

```rust
pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    git2::Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(PathBuf::from))
}
```

---

#### Monorepo Analysis

##### Detection Strategy

| Tool | Detection File | Key Indicator |
|------|----------------|---------------|
| pnpm workspaces | `pnpm-workspace.yaml` | `packages:` array |
| npm workspaces | `package.json` | `workspaces` field |
| yarn workspaces | `package.json` | `workspaces` field |
| Nx | `nx.json` | existence |
| Turborepo | `turbo.json` | existence |
| Lerna | `lerna.json` | existence |
| Cargo workspace | `Cargo.toml` | `[workspace]` section |
| Bazel | `WORKSPACE` or `MODULE.bazel` | existence |
| Pants | `pants.toml` | existence |
| Gradle multi-project | `settings.gradle(.kts)` | `include` statements |
| Maven multi-module | `pom.xml` | `<modules>` element |

##### Data Structure

```rust
pub struct MonorepoInfo {
    pub is_monorepo: bool,
    pub tool: Option<MonorepoTool>,
    pub root: PathBuf,
    pub packages: Vec<PackageLocation>,
}

pub enum MonorepoTool {
    // JS/TS
    NpmWorkspaces, PnpmWorkspaces, YarnWorkspaces, BunWorkspaces,
    Nx, Lerna, Turborepo, Rush,
    // Multi-language
    Bazel, Pants, Buck2, Moon,
    // Rust
    CargoWorkspace,
    // JVM
    GradleMultiProject, MavenMultiModule,
    // Other
    Unknown,
}

pub struct PackageLocation {
    pub name: String,
    pub path: PathBuf,              // Relative to repo root
    pub package_manager: Option<PackageManager>,
}
```

##### Implementation

```rust
pub fn detect_monorepo(root: &Path) -> Result<Option<MonorepoInfo>, SniffError> {
    // Check in priority order (more specific tools first)
    if let Some(info) = detect_nx(root)? { return Ok(Some(info)); }
    if let Some(info) = detect_turborepo(root)? { return Ok(Some(info)); }
    if let Some(info) = detect_pnpm_workspace(root)? { return Ok(Some(info)); }
    if let Some(info) = detect_npm_workspace(root)? { return Ok(Some(info)); }
    if let Some(info) = detect_cargo_workspace(root)? { return Ok(Some(info)); }
    if let Some(info) = detect_bazel(root)? { return Ok(Some(info)); }
    // ... etc

    Ok(None)
}

fn detect_pnpm_workspace(root: &Path) -> Result<Option<MonorepoInfo>, SniffError> {
    let workspace_file = root.join("pnpm-workspace.yaml");
    if !workspace_file.exists() { return Ok(None); }

    let content = std::fs::read_to_string(&workspace_file)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    let packages = yaml.get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .flat_map(|glob| expand_glob(root, glob))
                .map(|path| PackageLocation {
                    name: extract_package_name(&path),
                    path: path.strip_prefix(root).unwrap().to_path_buf(),
                    package_manager: Some(PackageManager::Pnpm),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: Some(MonorepoTool::PnpmWorkspaces),
        root: root.to_path_buf(),
        packages,
    }))
}
```

---

#### Dependencies

##### Package Manager Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageManager {
    // JavaScript/TypeScript
    Npm, Pnpm, Yarn, Bun, Deno, Jsr,
    // Rust
    Cargo,
    // Python
    Pip, Poetry, Pdm, Uv, Conda, Pipenv,
    // Ruby
    Bundler,
    // PHP
    Composer,
    // Java/JVM
    Maven, Gradle,
    // Go
    GoMod,
    // .NET
    NuGet, Paket,
    // C/C++
    Vcpkg, Conan,
    // Other
    LuaRocks, SwiftPM, Hex, Unknown,
}

impl PackageManager {
    pub fn primary_language(&self) -> &'static str { /* ... */ }
    pub fn registry_url(&self) -> Option<&'static str> { /* ... */ }
    pub fn parent(&self) -> Option<Self> { /* pnpm/yarn/bun -> npm */ }
    pub fn manifest_files(&self) -> &'static [&'static str] { /* ... */ }
    pub fn lockfiles(&self) -> &'static [&'static str] { /* ... */ }
}
```

##### Dependency Relationship

```rust
pub enum DependencyKind {
    Runtime,        // Production dependency
    Dev,            // Development only
    Peer,           // Peer dependency (npm ecosystem)
    Optional,       // Optional dependency
    Build,          // Build-time only
}
```

##### Dependency Structure

```rust
pub struct Dependency {
    pub name: String,
    pub kind: DependencyKind,
    pub version_requirement: String,    // "^1.2.3", ">=1.0,<2.0"
    pub resolved_version: Option<String>, // From lockfile
    pub package_manager: PackageManager,
    pub metadata: DependencyMetadata,
}

pub struct DependencyMetadata {
    pub registry: Option<String>,       // Non-default registry
    pub git_url: Option<String>,        // Git dependency
    pub git_ref: Option<String>,        // Branch/tag/commit
    pub path: Option<PathBuf>,          // Path dependency
    pub is_workspace: bool,             // Workspace reference
    pub features: Vec<String>,          // Cargo features
    pub alias_for: Option<String>,      // npm alias
}
```

##### Version Parsing

Use `semver` for Cargo-style versions and `nodejs-semver` for npm-style:

```rust
pub struct VersionRequirement {
    raw: String,
    kind: VersionRequirementKind,
}

enum VersionRequirementKind {
    Cargo(semver::VersionReq),
    Npm(nodejs_semver::Range),
    Other(String),
}

impl VersionRequirement {
    pub fn parse(raw: &str, pm: PackageManager) -> Self {
        match pm {
            PackageManager::Cargo => {
                semver::VersionReq::parse(raw)
                    .map(|r| Self { raw: raw.to_string(), kind: VersionRequirementKind::Cargo(r) })
                    .unwrap_or_else(|_| Self { raw: raw.to_string(), kind: VersionRequirementKind::Other(raw.to_string()) })
            }
            PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn | PackageManager::Bun => {
                nodejs_semver::Range::parse(raw)
                    .map(|r| Self { raw: raw.to_string(), kind: VersionRequirementKind::Npm(r) })
                    .unwrap_or_else(|_| Self { raw: raw.to_string(), kind: VersionRequirementKind::Other(raw.to_string()) })
            }
            _ => Self { raw: raw.to_string(), kind: VersionRequirementKind::Other(raw.to_string()) },
        }
    }

    pub fn matches(&self, version: &str) -> bool { /* ... */ }
}
```

##### Lockfile Parsing

Use `package-parser` crate for multi-format support:

- `Cargo.lock` - TOML format
- `package-lock.json` - JSON, npm v2/v3 formats
- `pnpm-lock.yaml` - YAML with content-addressable storage
- `yarn.lock` - Custom format (v1) or YAML (berry)
- `poetry.lock` - TOML format
- `Gemfile.lock` - Custom Bundler format
- `composer.lock` - JSON format

##### Package Manager Detection

```rust
pub fn detect_package_managers(root: &Path) -> Vec<(PackageManager, PathBuf)> {
    let mut found = Vec::new();

    // Check manifest files
    let checks = [
        ("Cargo.toml", PackageManager::Cargo),
        ("package.json", PackageManager::Npm),
        ("pyproject.toml", PackageManager::Poetry),  // Refined by content
        ("Gemfile", PackageManager::Bundler),
        ("composer.json", PackageManager::Composer),
        ("go.mod", PackageManager::GoMod),
    ];

    for (filename, pm) in checks {
        if let Some(path) = find_file(root, filename) {
            let refined = refine_package_manager(&path, pm);
            found.push((refined, path));
        }
    }

    found
}

fn refine_package_manager(manifest: &Path, initial: PackageManager) -> PackageManager {
    match initial {
        PackageManager::Npm => {
            // Check lockfile to determine actual package manager
            let dir = manifest.parent().unwrap();
            if dir.join("pnpm-lock.yaml").exists() { PackageManager::Pnpm }
            else if dir.join("yarn.lock").exists() { PackageManager::Yarn }
            else if dir.join("bun.lockb").exists() { PackageManager::Bun }
            else { PackageManager::Npm }
        }
        PackageManager::Poetry => {
            // Check pyproject.toml for tool section
            let content = std::fs::read_to_string(manifest).unwrap_or_default();
            if content.contains("[tool.poetry]") { PackageManager::Poetry }
            else if content.contains("[tool.pdm]") { PackageManager::Pdm }
            else if content.contains("[tool.uv]") { PackageManager::Uv }
            else { PackageManager::Pip }
        }
        _ => initial,
    }
}
```

##### Registry Query (Async, Opt-in)

```rust
#[async_trait]
pub trait RegistryClient: Send + Sync {
    async fn get_package(&self, name: &str) -> Result<PackageMetadata, RegistryError>;
    async fn get_versions(&self, name: &str) -> Result<Vec<String>, RegistryError>;
    async fn get_advisories(&self, name: &str, version: &str) -> Result<Vec<SecurityAdvisory>, RegistryError>;
}

// Implementations for npm, crates.io, PyPI, etc.
pub fn registry_for(pm: PackageManager) -> Option<Box<dyn RegistryClient>> { /* ... */ }
```

##### Complete Dependency Report

```rust
pub struct DependencyReport {
    pub package_managers: Vec<PackageManager>,
    pub packages: Vec<PackageDependencies>,
    pub summary: DependencySummary,
}

pub struct PackageDependencies {
    pub name: Option<String>,
    pub path: PathBuf,
    pub package_manager: PackageManager,
    pub has_lockfile: bool,
    pub dependencies: Vec<Dependency>,
}

pub struct DependencySummary {
    pub total_dependencies: usize,
    pub runtime_dependencies: usize,
    pub dev_dependencies: usize,
    pub outdated_count: usize,      // Requires registry query
    pub security_issues: usize,     // Requires registry query
    pub unique_dependencies: usize,
}
```

---

### Aggregating All Sniff Categories

The `detect()` utility function in `@sniff/lib/src/lib.rs` aggregates all detection areas:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    pub hardware: HardwareInfo,
    pub network: NetworkInfo,
    pub filesystem: Option<FilesystemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub base_dir: PathBuf,
    pub languages: LanguageBreakdown,
    pub code_analysis: Option<CodeAnalysis>,  // Opt-in, slower
    pub git: Option<GitInfo>,
    pub monorepo: Option<MonorepoInfo>,
    pub dependencies: DependencyReport,
}

#[derive(Default)]
pub struct SniffConfig {
    pub base_dir: Option<PathBuf>,
    pub include_cpu_usage: bool,
    pub include_file_status: bool,
    pub include_code_analysis: bool,    // Opt-in: run rust-code-analysis
    pub analysis_config: Option<AnalysisConfig>,
    pub query_registries: bool,
    pub skip_hardware: bool,
    pub skip_network: bool,
    pub skip_filesystem: bool,
}

/// Fast detection with sensible defaults
pub fn detect() -> Result<SniffResult, SniffError> {
    detect_with_config(SniffConfig::default())
}

/// Configurable detection
pub fn detect_with_config(config: SniffConfig) -> Result<SniffResult, SniffError> {
    let hardware = if config.skip_hardware {
        HardwareInfo::default()
    } else if config.include_cpu_usage {
        detect_hardware_with_usage()?
    } else {
        detect_hardware()?
    };

    let network = if config.skip_network {
        NetworkInfo::default()
    } else {
        detect_network()?
    };

    let filesystem = if config.skip_filesystem {
        None
    } else {
        let base = config.base_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        Some(detect_filesystem(&base, &config)?)
    };

    Ok(SniffResult { hardware, network, filesystem })
}

/// Async version with registry queries
pub async fn detect_async(config: SniffConfig) -> Result<SniffResult, SniffError> {
    // Parallel execution of independent detectors
    // Registry queries for dependency version info
    todo!()
}
```

---

## CLI Design

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "sniff", about = "Detect system and repository information")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Base directory for filesystem analysis
    #[arg(short, long, global = true)]
    base: Option<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "text", value_enum)]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect all information (default)
    All {
        #[arg(long)]
        cpu_usage: bool,
        #[arg(long)]
        check_updates: bool,
    },
    /// Hardware information only
    Hardware { #[arg(long)] cpu_usage: bool },
    /// Network information only
    Network { #[arg(long)] include_loopback: bool },
    /// Filesystem/repo information only
    Filesystem,
    /// Git repository status
    Git { #[arg(long)] files: bool },
    /// Programming language breakdown
    Languages,
    /// Dependency analysis
    Dependencies {
        #[arg(long)] outdated: bool,
        #[arg(long)] audit: bool,
    },
    /// Monorepo structure
    Monorepo,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat { Text, Json, Yaml }
```

---

## Performance Expectations

| Operation | Time |
|-----------|------|
| Hardware (no CPU) | <10ms |
| Network | <5ms |
| Languages (1k files) | <100ms |
| Git status | <50ms |
| Dependencies | <100ms |
| **Total (fast path)** | **<300ms** |

Slow path (opt-in):

| Operation | Time | Flag |
|-----------|------|------|
| CPU usage sampling | ~1000ms | `--cpu-usage` |
| Code analysis (1k files) | 2-10s | `--analyze` |
| Code analysis + hotspots | 5-10s | `--analyze --hotspots` |
| Registry queries (10 deps) | 500-2000ms | `--check-updates` |
| Large monorepo (10k files) | 500-1000ms | N/A |

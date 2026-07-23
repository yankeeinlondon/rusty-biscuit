use super::{
    DEFAULT_COMMIT_COUNT, FileListArgs, PackagesFormat, REPO_AFTER_HELP, RecentCommitActionArg,
};
use clap::Subcommand;

/// Layout direction for `sniff repo package-dependencies` visual rendering.
///
/// Maps to `biscuit_visualized::graph::GraphOrientation` in the renderer.
/// Aliased so users can write the long form (`left-to-right`/`horizontal`,
/// `top-to-bottom`/`vertical`) at the command line, while shell completions
/// suggest the short canonical forms.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OrientationArg {
    /// Left-to-right — hub graphs scroll vertically (default for `repo package-dependencies`).
    #[value(name = "lr", alias = "left-to-right", alias = "horizontal")]
    LeftToRight,
    /// Top-to-bottom — good for deep chain-like graphs.
    #[value(name = "tb", alias = "top-to-bottom", alias = "vertical")]
    TopToBottom,
}

impl OrientationArg {
    /// Canonical short form passed through `RepoAction::PackageDependencies::orientation`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeftToRight => "lr",
            Self::TopToBottom => "tb",
        }
    }
}

/// Normalized repo action — decoupled from clap parse shape.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RepoAction {
    Structure {
        filter: Vec<String>,
        latest_versions: bool,
        package: Option<String>,
        package_area: Option<String>,
    },
    GitStatus {
        history: usize,
        refresh_remotes: bool,
        compact: bool,
        package: Option<String>,
        package_area: Option<String>,
        /// Branch filter: `None` = flag absent, `Some(None)` = flag present
        /// with no value (use current branch), `Some(Some(name))` = explicit
        /// branch name.
        branch: Option<Option<String>>,
        /// Worktree filter: name of a linked worktree (matches the worktree's
        /// branch or its directory basename). Mutually exclusive with `branch`.
        worktree: Option<String>,
    },
    Hash {
        sha: String,
    },
    StagedFiles(FileListArgs),
    UnstagedFiles {
        package: Option<String>,
        package_area: Option<String>,
    },
    UntrackedFiles {
        package: Option<String>,
        package_area: Option<String>,
    },
    Remote {
        remote: Option<String>,
    },
    Branches {
        refresh_remotes: bool,
    },
    PackageDependencies {
        filter: Vec<String>,
        ui: bool,
        /// Emit the raw SVG document instead of a terminal-image render.
        /// Mutually exclusive with `ui`.
        svg: bool,
        package: Option<String>,
        package_area: Option<String>,
        /// Width spec for `--ui` rendering. Accepts `75%`, `120` (chars),
        /// `120ch`, or `fill`. `None` uses the deps-specific default (75%).
        width: Option<String>,
        /// Layout direction for `--ui` / `--svg` rendering. Accepts `lr`
        /// (left-to-right, default) or `tb` (top-to-bottom).
        orientation: Option<String>,
    },
    Dependencies {
        dependencies: bool,
        dev_dependencies: bool,
        peer_dependencies: bool,
        optional_dependencies: bool,
    },
    Packages {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
        format: PackagesFormat,
        no_error: bool,
        on_error: Option<String>,
    },
    PackageAreas {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
        format: PackagesFormat,
        no_error: bool,
        on_error: Option<String>,
    },
    Package {
        no_error: bool,
        on_error: Option<String>,
    },
    PackageArea {
        no_error: bool,
        on_error: Option<String>,
    },
    Area {
        no_error: bool,
        on_error: Option<String>,
    },
    DirtyPackages {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    DirtyPackageAreas {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    StagedPackages {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    StagedPackageAreas {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    UnstagedPackages {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    UnstagedPackageAreas {
        filter: Vec<String>,
        package: Option<String>,
        package_area: Option<String>,
    },
    PackageRoot,
    PackageAreaRoot,
    Root,
    IsCurrentPackageAreaDirty,
    PackageAreaHasSourceCodeChanges,
    DirtySourceCode(FileListArgs),
    StagedSourceCode(FileListArgs),
    UnstagedSourceCode(FileListArgs),
    DirtyFiles(FileListArgs),
    HasMergeConflict,
    RecentCommits {
        period: Option<String>,
        actions: Vec<RecentCommitActionArg>,
        package: Option<String>,
        package_area: Option<String>,
        no_error: bool,
        on_error: Option<String>,
    },
    SourceCodeChanges {
        period: Option<String>,
        actions: Vec<RecentCommitActionArg>,
        package: Option<String>,
        package_area: Option<String>,
        no_error: bool,
        on_error: Option<String>,
    },
    DocumentationChanges {
        period: Option<String>,
        actions: Vec<RecentCommitActionArg>,
        package: Option<String>,
        package_area: Option<String>,
        no_error: bool,
        on_error: Option<String>,
    },
    Pr {
        status: sniff::remote::PullRequestState,
        verbose: bool,
    },
    Language {
        breakdown: bool,
    },
    Worktree {
        no_error: bool,
        on_error: Option<String>,
    },
    Worktrees {
        md: bool,
        list: bool,
        csv: bool,
        verbose: bool,
    },
    Name,
    IsMonorepo {
        no_error: bool,
    },
    PackageCount,
    /// `sniff repo package-manager` — report package manager usage.
    PackageManager {
        csv: bool,
        list: bool,
        md: bool,
    },
    /// `sniff repo version` — report the declared version(s) for the current
    /// repo/package context, scoped like `repo test-runner`.
    Version {
        // Output formats (mutually exclusive: csv | list | md).
        csv: bool,
        list: bool,
        md: bool,
        // Scope overrides (mutually exclusive: all | package | package_area).
        // clap enforces the mutual exclusion; the handler treats them in
        // priority order regardless.
        all: bool,
        package: Option<String>,
        package_area: Option<String>,
        // Empty-result behaviour (preserved from the prior focused command).
        no_error: bool,
        on_error: Option<String>,
    },
    /// `sniff repo test-runner` — report declared test runner usage.
    TestRunner {
        csv: bool,
        list: bool,
        md: bool,
    },
    /// Bare `sniff repo` dispatch marker. Distinct from `Name` so the parent
    /// command can aggregate its children's scopes in `--json` mode while
    /// preserving the existing text fallback.
    Default,
}

// ---------------------------------------------------------------------------
// Shared completion candidates
// ---------------------------------------------------------------------------

/// Completion candidates for `--package` flags.
pub(crate) fn repo_package_candidates() -> Vec<clap_complete::engine::CompletionCandidate> {
    use clap_complete::engine::CompletionCandidate;
    let Ok(Some(info)) =
        sniff::filesystem::repo::detect_repo(&std::env::current_dir().unwrap_or_default())
    else {
        return Vec::new();
    };
    info.packages
        .unwrap_or_default()
        .into_iter()
        .map(|p| CompletionCandidate::new(p.name))
        .collect()
}

/// Completion candidates for `--package-area` flags.
pub(crate) fn repo_package_area_candidates() -> Vec<clap_complete::engine::CompletionCandidate> {
    use clap_complete::engine::CompletionCandidate;
    use std::collections::BTreeSet;
    let Ok(Some(info)) =
        sniff::filesystem::repo::detect_repo(&std::env::current_dir().unwrap_or_default())
    else {
        return Vec::new();
    };
    let areas: BTreeSet<String> = info
        .packages
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.package_area)
        .collect();
    areas.into_iter().map(CompletionCandidate::new).collect()
}

/// Repo-specific subcommands.
#[derive(Subcommand, Debug, Clone)]
#[command(
    disable_help_subcommand = true,
    after_help = REPO_AFTER_HELP,
)]
pub enum RepoSubcommand {
    /// Show repository structure
    Structure {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Query package registries for latest dependency versions and report available updates
        #[arg(long)]
        latest_versions: bool,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Show git status with commit history
    #[command(name = "git-status")]
    GitStatus {
        /// Number of recent commits to display
        #[arg(long, default_value_t = DEFAULT_COMMIT_COUNT)]
        history: usize,
        /// Fetch remotes to check if branches are out of sync
        #[arg(long)]
        refresh_remotes: bool,
        /// Only render the Status section
        #[arg(long)]
        compact: bool,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(short = 'a', long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Show commits from a specific branch (defaults to current branch
        /// when the flag is given without a value)
        #[arg(long, value_name = "BRANCH", conflicts_with = "worktree")]
        branch: Option<Option<String>>,
        /// Show commits and working-tree status from a linked worktree.
        /// Accepts the worktree's branch name or its directory basename.
        #[arg(long, value_name = "WORKTREE")]
        worktree: Option<String>,
    },
    /// Show details for a specific commit hash
    #[command(alias = "commit")]
    Hash {
        /// Git commit SHA (full or short)
        #[arg(value_name = "SHA")]
        sha: String,
    },
    /// List staged files (in index, ready to commit)
    #[command(name = "staged-files")]
    StagedFiles(FileListArgs),
    /// List unstaged files (modified in working tree)
    #[command(name = "unstaged-files")]
    UnstagedFiles {
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// List untracked files (not under version control)
    #[command(name = "untracked-files")]
    UntrackedFiles {
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// List dirty source code files (staged + modified + untracked source files)
    #[command(name = "dirty-source-code")]
    DirtySourceCode(FileListArgs),
    /// List staged source code files
    #[command(name = "staged-source-code")]
    StagedSourceCode(FileListArgs),
    /// List unstaged source code files
    #[command(name = "unstaged-source-code")]
    UnstagedSourceCode(FileListArgs),
    /// List all dirty files (staged + modified + untracked)
    #[command(name = "dirty-files")]
    DirtyFiles(FileListArgs),
    /// Inspect a remote repository (URL, name, or owner/repo shorthand)
    Remote {
        /// Git remote URL, remote name, or owner/repo shorthand
        ///
        /// When omitted inside a git repo, defaults to the origin remote URL
        /// (or the first configured remote if origin is not set).
        #[arg(value_name = "REMOTE")]
        remote: Option<String>,
    },
    /// List local branches
    Branches {
        /// Fetch remotes before calculating upstream ahead/behind counts
        #[arg(long)]
        refresh_remotes: bool,
    },
    /// Render an internal package dependency diagram
    #[command(name = "package-dependencies")]
    PackageDependencies {
        /// Use visual graph rendering instead of text
        #[arg(long, conflicts_with = "svg")]
        ui: bool,
        /// Emit the raw SVG source for the dependency diagram. Useful for
        /// piping to a file, an HTML page, or a browser-side renderer.
        #[arg(long, conflicts_with = "ui")]
        svg: bool,
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Width for `--ui` rendering: percentage (`75%`), column count (`120` or `120ch`), or `fill`. Default: 75%.
        #[arg(long, value_name = "WIDTH", requires = "ui")]
        width: Option<String>,
        /// Layout direction for `--ui` / `--svg` rendering: `lr` (left-to-right, default — hub graphs scroll vertically) or `tb` (top-to-bottom — good for deep chains).
        #[arg(long, value_name = "DIR", value_enum)]
        orientation: Option<OrientationArg>,
    },
    /// List external package dependencies
    Dependencies {
        /// Include runtime dependencies
        #[arg(long)]
        dependencies: bool,
        /// Include development dependencies
        #[arg(long)]
        dev_dependencies: bool,
        /// Include peer dependencies
        #[arg(long)]
        peer_dependencies: bool,
        /// Include optional dependencies
        #[arg(long)]
        optional_dependencies: bool,
    },
    /// Output only package names as a comma-separated list
    Packages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Restrict output to packages in the specified package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Render as a Markdown unordered list (one `- name` per line)
        #[arg(long, conflicts_with = "list")]
        md: bool,
        /// Render as a raw list (one name per line, no bullet)
        #[arg(long, conflicts_with = "md")]
        list: bool,
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,
        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },
    /// Output only package area names as a comma-separated list
    #[command(name = "package-areas")]
    PackageAreas {
        /// Filter by area name; prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Restrict output to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Render as a Markdown unordered list (one `- name` per line)
        #[arg(long, conflicts_with = "list")]
        md: bool,
        /// Render as a raw list (one entry per line, no bullet)
        #[arg(long, conflicts_with = "md")]
        list: bool,
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,
        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },
    /// Output the package name for the current directory
    Package {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Output the package area for the current directory
    PackageArea {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Output the area for the current directory (package name when inside a
    /// package, else the surrounding package-area, else "root")
    Area {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Output only package names that have uncommitted changes
    DirtyPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output only package area names that have uncommitted changes
    DirtyPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output only package names that have staged files
    #[command(name = "staged-packages")]
    StagedPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output only package area names that have staged files
    #[command(name = "staged-package-areas")]
    StagedPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output only package names that have unstaged changes
    #[command(name = "unstaged-packages")]
    UnstagedPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output only package area names that have unstaged changes
    #[command(name = "unstaged-package-areas")]
    UnstagedPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Scope to a specific package
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
    },
    /// Output the root directory of the current package
    PackageRoot,
    /// Output the root directory of the current package area
    PackageAreaRoot,
    /// Output the root directory of the repository
    Root,
    /// Exit 0 if the current package area has uncommitted changes, exit 1 otherwise
    IsCurrentPackageAreaDirty,
    /// Exit 0 if the current package area has source code changes, exit 1 otherwise
    PackageAreaHasSourceCodeChanges,
    /// Exit 0 if merge conflicts are detected, exit 1 otherwise
    #[command(name = "has-merge-conflict")]
    HasMergeConflict,
    /// Show recent commits for a period
    #[command(name = "recent-commits")]
    RecentCommits {
        /// Period: duration (3d, 1w), date (YYYY-MM-DD), hash, count (10), 'today', 'yesterday'
        period: Option<String>,
        /// Filter to conventional commit actions; repeat to OR multiple actions together
        #[arg(long = "action", value_enum, value_name = "ACTION")]
        actions: Vec<RecentCommitActionArg>,
        /// Scope to a specific package
        #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,
        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Show source code changes for a period
    #[command(name = "source-code-changes")]
    SourceCodeChanges {
        /// Period: duration (3d, 1w), date (YYYY-MM-DD), hash, count (10), 'today', 'yesterday'
        period: Option<String>,
        /// Filter to conventional commit actions; repeat to OR multiple actions together
        #[arg(long = "action", value_enum, value_name = "ACTION")]
        actions: Vec<RecentCommitActionArg>,
        /// Scope to a specific package
        #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,
        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Show documentation changes for a period
    #[command(name = "documentation-changes")]
    DocumentationChanges {
        /// Period: duration (3d, 1w), date (YYYY-MM-DD), hash, count (10), 'today', 'yesterday'
        period: Option<String>,
        /// Filter to conventional commit actions; repeat to OR multiple actions together
        #[arg(long = "action", value_enum, value_name = "ACTION")]
        actions: Vec<RecentCommitActionArg>,
        /// Scope to a specific package
        #[arg(long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
        package: Option<String>,
        /// Scope to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,
        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// List pull requests for the current repository's remote
    Pr {
        /// Filter pull requests by state (note: 'draft' returns no results on Bitbucket — drafts are not a Bitbucket Cloud feature)
        #[arg(long, default_value = "open")]
        status: sniff::remote::PullRequestState,
    },
    /// Output the primary programming language for the repository
    Language {
        /// Show detailed per-package language breakdown (like the old `sniff language` command)
        #[arg(long)]
        breakdown: bool,
    },
    /// Output the worktree name for the current directory
    Worktree {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// List all worktrees in the repository
    #[command(name = "worktrees")]
    Worktrees {
        /// Render as a Markdown unordered list (one `- name` per line)
        #[arg(long, conflicts_with_all = ["list", "csv"])]
        md: bool,

        /// Output as a newline-delimited list (one name per line)
        #[arg(long, conflicts_with_all = ["md", "csv"])]
        list: bool,

        /// Output as comma-separated values on a single line
        #[arg(long, conflicts_with_all = ["md", "list"])]
        csv: bool,
    },
    /// Output the repository name (plain text); the rich version + language/monorepo one-liner lives on the parent `repo -v`
    Name,
    /// Output whether the repository is a monorepo.
    ///
    /// Prints the unified monorepo label when inside a monorepo, otherwise
    /// prints `false`. Exits non-zero outside a monorepo unless `--no-error`
    /// is given. Genuine failures (not a git repository, unreadable path, …)
    /// still exit non-zero even with `--no-error`.
    #[command(name = "is-monorepo")]
    IsMonorepo {
        /// Exit 0 with `false` output when not inside a monorepo.
        ///
        /// This only suppresses the predicate failure for non-monorepos;
        /// genuine repository errors still exit non-zero.
        #[arg(long)]
        no_error: bool,
    },
    /// Output the number of packages discovered in the repository
    #[command(name = "package-count")]
    PackageCount,
    /// Report the package manager(s) used by the current repo/package context.
    #[command(name = "package-manager")]
    PackageManager {
        /// Render as comma-separated values on a single line.
        #[arg(long, conflicts_with_all = ["list", "md"])]
        csv: bool,
        /// Render as a newline-delimited list (one package manager per line).
        #[arg(long, conflicts_with_all = ["csv", "md"])]
        list: bool,
        /// Render as a Markdown unordered list (one `- name` per line).
        #[arg(long, conflicts_with_all = ["csv", "list"])]
        md: bool,
    },
    /// Report the declared version(s) for the current repo/package context.
    ///
    /// Scoped to the CWD by default (package / package-area / repo, same as
    /// `repo test-runner`); `--all` / `--package` / `--package-area` override
    /// that resolution. `--json` returns the `{ "versions": [...] }` shape
    /// shared with the other focused repo subcommands. `--csv` / `--list` /
    /// `--md` and `--verbose` follow the `repo test-runner` rendering
    /// conventions.
    #[command(name = "version")]
    Version {
        /// Render as comma-separated values on a single line.
        #[arg(long, conflicts_with_all = ["list", "md"])]
        csv: bool,
        /// Render as a newline-delimited list (one version per line).
        #[arg(long, conflicts_with_all = ["csv", "md"])]
        list: bool,
        /// Render as a Markdown unordered list (one `- version` per line).
        #[arg(long, conflicts_with_all = ["csv", "list"])]
        md: bool,
        /// Scope to every package in the repository, regardless of CWD.
        #[arg(long, conflicts_with_all = ["package", "package_area"])]
        all: bool,
        /// Scope to a specific package by name.
        #[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates), conflicts_with_all = ["all", "package_area"])]
        package: Option<String>,
        /// Scope to a specific package area by name.
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates), conflicts_with_all = ["all", "package"])]
        package_area: Option<String>,
        /// Exit 0 with no output when no version is found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no version is found (text mode only)
        #[arg(long, value_name = "MESSAGE", allow_hyphen_values = true)]
        on_error: Option<String>,
    },
    /// Report the test runner(s) declared by the current repo/package context.
    ///
    /// Host installation is reported by `sniff software test-runners`; this
    /// subcommand reports repository usage (manifest keys, config files,
    /// ecosystem defaults). Output modes: default styled text, `--csv`,
    /// `--list`, `--md`, and `--json`.
    #[command(name = "test-runner")]
    TestRunner {
        /// Render as comma-separated values on a single line.
        #[arg(long, conflicts_with_all = ["list", "md"])]
        csv: bool,
        /// Render as a newline-delimited list (one runner per line).
        #[arg(long, conflicts_with_all = ["csv", "md"])]
        list: bool,
        /// Render as a Markdown unordered list (one `- name` per line).
        #[arg(long, conflicts_with_all = ["csv", "list"])]
        md: bool,
    },
}

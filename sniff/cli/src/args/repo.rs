use clap::Subcommand;
use super::{FileListArgs, PackagesFormat, RecentCommitActionArg, DEFAULT_COMMIT_COUNT, REPO_AFTER_HELP};

/// Normalized repo action — decoupled from clap parse shape.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RepoAction {
    Structure {
        filter: Vec<String>,
        latest_versions: bool,
    },
    GitStatus {
        history: usize,
        refresh_remotes: bool,
        compact: bool,
        package: Option<String>,
    },
    Hash {
        sha: String,
    },
    StagedFiles(FileListArgs),
    UnstagedFiles {
        package: Option<String>,
    },
    UntrackedFiles {
        package: Option<String>,
    },
    Remote {
        remote: String,
    },
    Deps {
        filter: Vec<String>,
        ui: bool,
    },
    Packages {
        filter: Vec<String>,
        package_area: Option<String>,
        format: PackagesFormat,
    },
    PackageAreas {
        filter: Vec<String>,
        package_area: Option<String>,
        format: PackagesFormat,
    },
    Package {
        no_error: bool,
        on_error: Option<String>,
    },
    PackageArea {
        no_error: bool,
        on_error: Option<String>,
    },
    DirtyPackages {
        filter: Vec<String>,
    },
    DirtyPackageAreas {
        filter: Vec<String>,
    },
    StagedPackages {
        filter: Vec<String>,
    },
    StagedPackageAreas {
        filter: Vec<String>,
    },
    UnstagedPackages {
        filter: Vec<String>,
    },
    UnstagedPackageAreas {
        filter: Vec<String>,
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
    /// Show repository structure (default when no subcommand given)
    Structure {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
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
        /// Scope git view to a specific package or package area
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// Show details for a specific commit hash
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
        /// Scope to a specific package or package area
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// List untracked files (not under version control)
    #[command(name = "untracked-files")]
    UntrackedFiles {
        /// Scope to a specific package or package area
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
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
        #[arg(value_name = "REMOTE")]
        remote: String,
    },
    /// Render an internal dependency diagram
    Deps {
        /// Use visual (Mermaid) rendering instead of text
        #[arg(long)]
        ui: bool,
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package names as a comma-separated list
    Packages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
        /// Restrict output to packages in the specified package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Render as a Markdown unordered list (one `- name` per line)
        #[arg(long, conflicts_with = "list")]
        md: bool,
        /// Render as a raw list (one name per line, no bullet)
        #[arg(long, conflicts_with = "md")]
        list: bool,
    },
    /// Output only package area names as a comma-separated list
    #[command(name = "package-areas")]
    PackageAreas {
        /// Filter by area name; prefix with ! to exclude
        filter: Vec<String>,
        /// Restrict output to a specific package area
        #[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
        package_area: Option<String>,
        /// Render as a Markdown unordered list (one `- name` per line)
        #[arg(long, conflicts_with = "list")]
        md: bool,
        /// Render as a raw list (one entry per line, no bullet)
        #[arg(long, conflicts_with = "md")]
        list: bool,
    },
    /// Output the package name for the current directory
    Package {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },
    /// Output the package area for the current directory
    PackageArea {
        /// Exit 0 with no output when no results found (default is exit 1)
        #[arg(long)]
        no_error: bool,

        /// Message to display when no results found
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },
    /// Output only package names that have uncommitted changes
    DirtyPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package area names that have uncommitted changes
    DirtyPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package names that have staged files
    #[command(name = "staged-packages")]
    StagedPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package area names that have staged files
    #[command(name = "staged-package-areas")]
    StagedPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package names that have unstaged changes
    #[command(name = "unstaged-packages")]
    UnstagedPackages {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
    },
    /// Output only package area names that have unstaged changes
    #[command(name = "unstaged-package-areas")]
    UnstagedPackageAreas {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Vec<String>,
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
        #[arg(long, value_name = "MESSAGE")]
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
        #[arg(long, value_name = "MESSAGE")]
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
        #[arg(long, value_name = "MESSAGE")]
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
        #[arg(long, value_name = "MESSAGE")]
        on_error: Option<String>,
    },
}

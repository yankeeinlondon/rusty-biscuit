use chrono::{DateTime, Utc};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{Result, SniffError};

/// Git hosting provider types.
///
/// Identifies the hosting platform for a Git repository based on its remote URL.
/// The `#[non_exhaustive]` attribute allows future additions without breaking changes.
///
/// ## Examples
///
/// ```
/// use sniff_lib::filesystem::git::HostingProvider;
///
/// let provider = HostingProvider::from_url("https://github.com/user/repo");
/// assert_eq!(provider, HostingProvider::GitHub);
///
/// let provider = HostingProvider::from_url("git@gitlab.com:user/repo.git");
/// assert_eq!(provider, HostingProvider::GitLab);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostingProvider {
    /// GitHub (github.com)
    GitHub,
    /// GitLab (gitlab.com)
    GitLab,
    /// Bitbucket (bitbucket.org)
    Bitbucket,
    /// Azure DevOps (dev.azure.com, visualstudio.com)
    AzureDevOps,
    /// AWS CodeCommit
    AwsCodeCommit,
    /// Gitea
    Gitea,
    /// Forgejo
    Forgejo,
    /// SourceHut (sr.ht)
    SourceHut,
    /// Self-hosted Git server
    SelfHosted,
    /// Unknown provider
    Unknown,
}

impl HostingProvider {
    /// Detects the hosting provider from a Git remote URL.
    ///
    /// Supports HTTPS, SSH, and git protocol URLs.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::filesystem::git::HostingProvider;
    ///
    /// assert_eq!(
    ///     HostingProvider::from_url("https://github.com/user/repo"),
    ///     HostingProvider::GitHub
    /// );
    /// assert_eq!(
    ///     HostingProvider::from_url("git@bitbucket.org:user/repo.git"),
    ///     HostingProvider::Bitbucket
    /// );
    /// assert_eq!(
    ///     HostingProvider::from_url("https://git.company.com/repo"),
    ///     HostingProvider::SelfHosted
    /// );
    /// ```
    pub fn from_url(url: &str) -> Self {
        let normalized = url
            .trim_start_matches("git@")
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("ssh://");

        if normalized.starts_with("github.com") {
            Self::GitHub
        } else if normalized.starts_with("gitlab.com") {
            Self::GitLab
        } else if normalized.starts_with("bitbucket.org") {
            Self::Bitbucket
        } else if normalized.contains("dev.azure.com") || normalized.contains("visualstudio.com") {
            Self::AzureDevOps
        } else if normalized.contains("codecommit") && normalized.contains("amazonaws.com") {
            Self::AwsCodeCommit
        } else if normalized.contains("sr.ht") {
            Self::SourceHut
        } else if normalized.contains('.') {
            Self::SelfHosted
        } else {
            Self::Unknown
        }
    }
}

/// Complete Git repository information.
///
/// Contains repository metadata including location, branch, commit history,
/// working tree status, and remote configuration.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false).unwrap();
/// if let Some(info) = git_info {
///     println!("Repository: {:?}", info.repo_root);
///     println!("Branch: {:?}", info.current_branch);
///     println!("Dirty: {}", info.status.is_dirty);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Absolute path to the repository root.
    pub repo_root: PathBuf,
    /// Current branch name (None for detached HEAD).
    pub current_branch: Option<String>,
    /// Whether the current path is inside a worktree (vs main repository).
    pub in_worktree: bool,
    /// Recent commits from HEAD (last 10 commits).
    pub recent: Vec<CommitInfo>,
    /// Working tree status.
    pub status: RepoStatus,
    /// Configured remotes.
    pub remotes: Vec<RemoteInfo>,
    /// Linked worktrees (keyed by branch name).
    pub worktrees: HashMap<String, WorktreeInfo>,
}

/// Represents whether the local branch is behind remote tracking branches.
///
/// Serializes as `false` when not behind any remote, or as an array of remote
/// names when behind one or more remotes. This type is only populated when
/// the `--deep` flag is used.
///
/// ## Examples
///
/// ```
/// use sniff_lib::filesystem::git::BehindStatus;
///
/// // Not behind any remote
/// let status = BehindStatus::NotBehind;
/// assert_eq!(serde_json::to_string(&status).unwrap(), "false");
///
/// // Behind origin and upstream
/// let status = BehindStatus::Behind(vec!["origin".to_string(), "upstream".to_string()]);
/// assert_eq!(serde_json::to_string(&status).unwrap(), r#"["origin","upstream"]"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehindStatus {
    /// Local branch is not behind any remote.
    NotBehind,
    /// Local branch is behind these remotes.
    Behind(Vec<String>),
}

impl Default for BehindStatus {
    fn default() -> Self {
        Self::NotBehind
    }
}

impl Serialize for BehindStatus {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            BehindStatus::NotBehind => serializer.serialize_bool(false),
            BehindStatus::Behind(remotes) => remotes.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BehindStatus {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct BehindStatusVisitor;

        impl<'de> Visitor<'de> for BehindStatusVisitor {
            type Value = BehindStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("false or an array of remote names")
            }

            fn visit_bool<E>(self, value: bool) -> std::result::Result<BehindStatus, E>
            where
                E: de::Error,
            {
                if value {
                    Err(de::Error::custom("expected false, got true"))
                } else {
                    Ok(BehindStatus::NotBehind)
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<BehindStatus, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut remotes = Vec::new();
                while let Some(remote) = seq.next_element()? {
                    remotes.push(remote);
                }
                Ok(BehindStatus::Behind(remotes))
            }
        }

        deserializer.deserialize_any(BehindStatusVisitor)
    }
}

/// Working tree status information.
///
/// Tracks staged, unstaged, and untracked changes in the repository,
/// including detailed information about each modified and untracked file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoStatus {
    /// True if there are any uncommitted changes.
    pub is_dirty: bool,
    /// Number of staged changes.
    pub staged_count: usize,
    /// Number of unstaged modifications.
    pub unstaged_count: usize,
    /// Number of untracked files.
    pub untracked_count: usize,
    /// Detailed information about files with uncommitted changes.
    pub dirty: Vec<DirtyFile>,
    /// Detailed information about untracked files.
    pub untracked: Vec<UntrackedFile>,
    /// Remotes where local branch is behind (only populated with --deep flag).
    ///
    /// When `--deep` is used:
    /// - `Some(BehindStatus::NotBehind)` → serializes as `false`
    /// - `Some(BehindStatus::Behind(vec![...]))` → serializes as array of remote names
    /// When `--deep` is not used: `None` → field is omitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_behind: Option<BehindStatus>,
}

/// Git remote configuration.
///
/// Contains the remote name, URL, and detected hosting provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    /// Remote name (e.g., "origin").
    pub name: String,
    /// Remote URL (if configured).
    pub url: Option<String>,
    /// Detected hosting provider.
    pub provider: HostingProvider,
    /// Branches available on this remote (only populated with --deep flag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,
}

/// Git commit metadata.
///
/// Contains commit hash, message, author, and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Full commit SHA.
    pub sha: String,
    /// Commit message (trimmed).
    pub message: String,
    /// Author name.
    pub author: String,
    /// Commit timestamp.
    pub timestamp: DateTime<Utc>,
    /// Remotes that contain this commit (only populated with --deep flag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remotes: Option<Vec<String>>,
}

/// Git worktree information.
///
/// Contains details about a linked worktree including its branch,
/// location, HEAD commit, and dirty status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    /// Branch name checked out in this worktree.
    pub branch: String,
    /// Absolute path to the worktree directory.
    pub filepath: PathBuf,
    /// HEAD commit SHA in this worktree.
    pub sha: String,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
}

/// A file with uncommitted changes (staged or unstaged).
///
/// Contains path information, unified diff output, and commit references
/// for tracking the state of modified files in the working tree.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false).unwrap().unwrap();
/// for dirty_file in &git_info.status.dirty {
///     println!("Modified: {:?}", dirty_file.filepath);
///     println!("Diff:\n{}", dirty_file.diff);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyFile {
    /// Relative path from repository root.
    pub filepath: PathBuf,
    /// Absolute path to the file.
    pub absolute_filepath: PathBuf,
    /// Full unified diff (like `git diff` output).
    pub diff: String,
    /// HEAD commit SHA.
    pub last_local_commit: String,
    /// Remote tracking branch commit SHA (if available).
    pub origin_commit: Option<String>,
}

/// An untracked file in the repository.
///
/// Represents a file that exists in the working tree but is not
/// tracked by Git (not in the index).
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false).unwrap().unwrap();
/// for untracked in &git_info.status.untracked {
///     println!("Untracked: {:?}", untracked.filepath);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrackedFile {
    /// Relative path from repository root.
    pub filepath: PathBuf,
    /// Absolute path to the file.
    pub absolute_filepath: PathBuf,
}

/// Detects Git repository information for a given path.
///
/// Searches upward from the given path to find a Git repository.
/// Returns `None` if the path is not within a Git repository.
///
/// The `deep` parameter enables network operations for enhanced remote info
/// (e.g., fetching remote branch info, checking if local is behind remote).
/// When `false`, only local repository information is gathered.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let result = detect_git(Path::new("."), false).unwrap();
/// match result {
///     Some(info) => println!("Found repo at: {:?}", info.repo_root),
///     None => println!("Not a git repository"),
/// }
/// ```
///
/// ## Errors
///
/// Returns an error if:
/// - The repository exists but has no working directory (bare repo)
/// - Git operations fail due to filesystem permissions or corruption
pub fn detect_git(path: &Path, deep: bool) -> Result<Option<GitInfo>> {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?
        .to_path_buf();

    let head = repo.head().ok();
    let current_branch = head
        .as_ref()
        .and_then(|h| h.shorthand())
        .map(String::from);

    let in_worktree = repo.is_worktree();
    let recent = get_recent_commits(&repo, 10, deep);
    let mut status = get_repo_status(&repo)?;
    if deep {
        status.is_behind = Some(check_behind_remotes(&repo));
    }
    let remotes = get_remotes(&repo, deep);
    let worktrees = get_worktrees(&repo);

    Ok(Some(GitInfo {
        repo_root,
        current_branch,
        in_worktree,
        recent,
        status,
        remotes,
        worktrees,
    }))
}

/// Gets the last N commits from HEAD using revwalk.
///
/// When `deep` is true, also determines which remotes contain each commit.
fn get_recent_commits(repo: &Repository, count: usize, deep: bool) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };

    if revwalk.push_head().is_err() {
        return commits;
    }

    for oid_result in revwalk.take(count) {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        let remotes = if deep {
            check_commit_on_remotes(repo, oid)
        } else {
            None
        };

        let author = commit.author();
        commits.push(CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").trim().to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
            remotes,
        });
    }

    commits
}

/// Gathers repository status including staged, unstaged, and untracked changes.
fn get_repo_status(repo: &Repository) -> Result<RepoStatus> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    // Recurse into untracked directories to get individual file paths
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked_count = 0;

    // Collect paths for dirty and untracked files
    let mut dirty_paths: Vec<PathBuf> = Vec::new();
    let mut untracked_paths: Vec<PathBuf> = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry.path().map(PathBuf::from);

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
            if let Some(ref p) = path
                && !dirty_paths.contains(p) {
                    dirty_paths.push(p.clone());
                }
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
            if let Some(ref p) = path
                && !dirty_paths.contains(p) {
                    dirty_paths.push(p.clone());
                }
        }
        if status.is_wt_new() {
            untracked_count += 1;
            if let Some(p) = path {
                untracked_paths.push(p);
            }
        }
    }

    // Get HEAD commit SHA and upstream commit
    let (head_sha, origin_commit) = get_commit_refs(repo);

    // Get repository root for absolute paths
    let repo_root = repo.workdir().map(Path::to_path_buf);

    // Build dirty file details with diffs
    let dirty = build_dirty_files(repo, &dirty_paths, &head_sha, &origin_commit, &repo_root)?;

    // Build untracked file details
    let untracked = build_untracked_files(&untracked_paths, &repo_root);

    Ok(RepoStatus {
        is_dirty: staged > 0 || unstaged > 0 || untracked_count > 0,
        staged_count: staged,
        unstaged_count: unstaged,
        untracked_count,
        dirty,
        untracked,
        is_behind: None, // Populated by detect_git when deep=true
    })
}

/// Gets HEAD commit SHA and upstream tracking branch commit SHA.
fn get_commit_refs(repo: &Repository) -> (String, Option<String>) {
    // Get HEAD commit SHA
    let head_sha = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id().to_string())
        .unwrap_or_default();

    // Get upstream tracking branch commit using dynamic remote discovery
    let origin_commit = get_upstream_commit(repo);

    (head_sha, origin_commit)
}

/// Gets the upstream tracking branch commit SHA using dynamic remote discovery.
fn get_upstream_commit(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;

    // Only works for branch references, not detached HEAD
    if !head.is_branch() {
        return None;
    }

    let branch_name = head.shorthand()?;
    let branch = repo.find_branch(branch_name, git2::BranchType::Local).ok()?;

    // Get the upstream branch (this handles dynamic remote discovery)
    let upstream = branch.upstream().ok()?;
    let upstream_commit = upstream.get().peel_to_commit().ok()?;

    Some(upstream_commit.id().to_string())
}

/// Builds detailed information for dirty files including unified diffs.
fn build_dirty_files(
    repo: &Repository,
    paths: &[PathBuf],
    head_sha: &str,
    origin_commit: &Option<String>,
    repo_root: &Option<PathBuf>,
) -> Result<Vec<DirtyFile>> {
    let mut dirty_files = Vec::new();

    for filepath in paths {
        let diff = get_file_diff(repo, filepath)?;
        let absolute_filepath = repo_root
            .as_ref()
            .map(|root| root.join(filepath))
            .unwrap_or_else(|| filepath.clone());

        dirty_files.push(DirtyFile {
            filepath: filepath.clone(),
            absolute_filepath,
            diff,
            last_local_commit: head_sha.to_string(),
            origin_commit: origin_commit.clone(),
        });
    }

    Ok(dirty_files)
}

/// Gets the unified diff for a single file (combined staged + unstaged changes).
fn get_file_diff(repo: &Repository, filepath: &Path) -> Result<String> {
    let mut diff_output = String::new();

    // Get diff for staged changes (HEAD to index)
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(filepath);

        if let Ok(staged_diff) = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))
        {
            let staged_output = diff_to_string(&staged_diff)?;
            if !staged_output.is_empty() {
                diff_output.push_str(&staged_output);
            }
        }
    }

    // Get diff for unstaged changes (index to workdir)
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.pathspec(filepath);

    if let Ok(unstaged_diff) = repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
        let unstaged_output = diff_to_string(&unstaged_diff)?;
        if !unstaged_output.is_empty() {
            if !diff_output.is_empty() {
                diff_output.push('\n');
            }
            diff_output.push_str(&unstaged_output);
        }
    }

    Ok(diff_output)
}

/// Converts a git2::Diff to a unified diff string using the callback-based print API.
fn diff_to_string(diff: &git2::Diff) -> Result<String> {
    let mut output = String::new();

    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        // Add the appropriate prefix based on line origin
        let prefix = match line.origin() {
            '+' | '-' | ' ' => line.origin(),
            _ => ' ',
        };

        // Only add prefix for content lines, not headers
        if matches!(line.origin(), '+' | '-' | ' ') {
            output.push(prefix);
        }

        if let Ok(content) = std::str::from_utf8(line.content()) {
            output.push_str(content);
        }
        true
    })?;

    Ok(output)
}

/// Builds detailed information for untracked files.
fn build_untracked_files(paths: &[PathBuf], repo_root: &Option<PathBuf>) -> Vec<UntrackedFile> {
    paths
        .iter()
        .map(|filepath| {
            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(filepath))
                .unwrap_or_else(|| filepath.clone());

            UntrackedFile {
                filepath: filepath.clone(),
                absolute_filepath,
            }
        })
        .collect()
}

/// Retrieves all configured remotes with their URLs and hosting providers.
///
/// When `deep` is true, also fetches the list of branches from each remote
/// using ls-remote. This requires network access and may fail for remotes
/// that require authentication.
fn get_remotes(repo: &Repository, deep: bool) -> Vec<RemoteInfo> {
    repo.remotes()
        .map(|names| {
            names
                .iter()
                .flatten()
                .filter_map(|name| {
                    repo.find_remote(name).ok().map(|remote| {
                        let url = remote.url().map(String::from);
                        let provider = url
                            .as_ref()
                            .map(|u| HostingProvider::from_url(u))
                            .unwrap_or(HostingProvider::Unknown);

                        let branches = if deep {
                            get_remote_branches(repo, name)
                        } else {
                            None
                        };

                        RemoteInfo {
                            name: name.to_string(),
                            url,
                            provider,
                            branches,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Fetches branch names from a remote using ls-remote.
///
/// Returns `None` if the fetch fails or times out. This is a read-only
/// operation that connects to the remote without authentication.
/// Network failures are handled gracefully.
fn get_remote_branches(repo: &Repository, remote_name: &str) -> Option<Vec<String>> {
    let mut remote = repo.find_remote(remote_name).ok()?;

    // Connect to remote - use default callbacks (no auth)
    // This is a read-only operation (ls-remote)
    remote
        .connect_auth(git2::Direction::Fetch, None, None)
        .ok()?;

    // Get the list of remote refs
    let refs = remote.list().ok()?;

    let branches: Vec<String> = refs
        .iter()
        .filter_map(|head| {
            let name = head.name();
            // Filter to only branch refs (refs/heads/*)
            name.strip_prefix("refs/heads/").map(|branch| branch.to_string())
        })
        .collect();

    if branches.is_empty() {
        None
    } else {
        Some(branches)
    }
}

/// Checks which remotes the local branch is behind.
///
/// Returns `BehindStatus::NotBehind` if local is up-to-date with all remotes,
/// or `BehindStatus::Behind(vec![...])` with the list of remote names where
/// the tracking branch has commits not in local.
fn check_behind_remotes(repo: &Repository) -> BehindStatus {
    let Some(head) = repo.head().ok() else {
        return BehindStatus::NotBehind;
    };
    if !head.is_branch() {
        return BehindStatus::NotBehind;
    }

    let Some(branch_name) = head.shorthand() else {
        return BehindStatus::NotBehind;
    };
    let Ok(local_branch) = repo.find_branch(branch_name, git2::BranchType::Local) else {
        return BehindStatus::NotBehind;
    };
    let Ok(local_commit) = local_branch.get().peel_to_commit() else {
        return BehindStatus::NotBehind;
    };

    let mut behind_remotes = Vec::new();

    // Check each remote for a tracking branch
    if let Ok(remotes) = repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            // Try to find the remote tracking branch (e.g., origin/main)
            let remote_branch_name = format!("{}/{}", remote_name, branch_name);
            if let Ok(remote_ref) =
                repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))
                && let Ok(remote_commit) = remote_ref.peel_to_commit()
                && let Ok((_ahead, behind)) =
                    repo.graph_ahead_behind(local_commit.id(), remote_commit.id())
                && behind > 0
            {
                behind_remotes.push(remote_name.to_string());
            }
        }
    }

    if behind_remotes.is_empty() {
        BehindStatus::NotBehind
    } else {
        BehindStatus::Behind(behind_remotes)
    }
}

/// Checks which remotes contain a specific commit.
fn check_commit_on_remotes(repo: &Repository, commit_oid: git2::Oid) -> Option<Vec<String>> {
    let mut containing_remotes = Vec::new();

    if let Ok(remotes) = repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            // Check all remote refs for this remote
            if let Ok(refs) = repo.references_glob(&format!("refs/remotes/{}/*", remote_name)) {
                for reference in refs.flatten() {
                    if let Ok(target) = reference.peel_to_commit() {
                        // Check if commit is reachable from this remote ref
                        if repo
                            .graph_descendant_of(target.id(), commit_oid)
                            .unwrap_or(false)
                            || target.id() == commit_oid
                        {
                            containing_remotes.push(remote_name.to_string());
                            break; // Found on this remote, no need to check other branches
                        }
                    }
                }
            }
        }
    }

    if containing_remotes.is_empty() {
        None
    } else {
        Some(containing_remotes)
    }
}

/// Retrieves all linked worktrees for the repository.
///
/// Returns a HashMap keyed by branch name. Anonymous worktrees (without a name)
/// are filtered out. For each worktree, opens it as a Repository to access
/// HEAD commit and dirty status.
fn get_worktrees(repo: &Repository) -> HashMap<String, WorktreeInfo> {
    let mut worktrees = HashMap::new();

    let worktree_names = match repo.worktrees() {
        Ok(names) => names,
        Err(_) => return worktrees,
    };

    for name in worktree_names.iter().flatten() {
        let worktree = match repo.find_worktree(name) {
            Ok(wt) => wt,
            Err(_) => continue,
        };

        let worktree_path = worktree.path();
        let worktree_repo = match Repository::open(worktree_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Get branch name from worktree's HEAD
        let branch = worktree_repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| name.to_string());

        // Get HEAD commit SHA
        let sha = worktree_repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .map(|c| c.id().to_string())
            .unwrap_or_default();

        // Check if worktree is dirty
        let dirty = get_repo_status(&worktree_repo)
            .map(|s| s.is_dirty)
            .unwrap_or(false);

        worktrees.insert(
            branch.clone(),
            WorktreeInfo {
                branch,
                filepath: worktree_path.to_path_buf(),
                sha,
                dirty,
            },
        );
    }

    worktrees
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_non_git_directory_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_git_repo_detected() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a branch
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // Use canonicalize to handle /private/var vs /var on macOS
        assert_eq!(
            info.repo_root.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
        assert!(info.current_branch.is_some());
        assert!(!info.recent.is_empty());
    }

    #[test]
    fn test_in_worktree_false_for_normal_repo() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid repo
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // A standard initialized repo is not a worktree
        assert!(!info.in_worktree);
    }

    #[test]
    fn test_hosting_provider_github() {
        assert_eq!(
            HostingProvider::from_url("https://github.com/user/repo"),
            HostingProvider::GitHub
        );
        assert_eq!(
            HostingProvider::from_url("git@github.com:user/repo.git"),
            HostingProvider::GitHub
        );
    }

    #[test]
    fn test_hosting_provider_gitlab() {
        assert_eq!(
            HostingProvider::from_url("https://gitlab.com/user/repo"),
            HostingProvider::GitLab
        );
    }

    #[test]
    fn test_hosting_provider_bitbucket() {
        assert_eq!(
            HostingProvider::from_url("https://bitbucket.org/user/repo"),
            HostingProvider::Bitbucket
        );
    }

    #[test]
    fn test_hosting_provider_azure_devops() {
        assert_eq!(
            HostingProvider::from_url("https://dev.azure.com/org/project"),
            HostingProvider::AzureDevOps
        );
        assert_eq!(
            HostingProvider::from_url("https://org.visualstudio.com/project"),
            HostingProvider::AzureDevOps
        );
    }

    #[test]
    fn test_hosting_provider_aws_codecommit() {
        assert_eq!(
            HostingProvider::from_url("https://git-codecommit.us-east-1.amazonaws.com/v1/repos/repo"),
            HostingProvider::AwsCodeCommit
        );
    }

    #[test]
    fn test_hosting_provider_sourcehut() {
        assert_eq!(
            HostingProvider::from_url("https://git.sr.ht/~user/repo"),
            HostingProvider::SourceHut
        );
    }

    #[test]
    fn test_hosting_provider_self_hosted() {
        assert_eq!(
            HostingProvider::from_url("https://git.company.com/repo"),
            HostingProvider::SelfHosted
        );
    }

    #[test]
    fn test_hosting_provider_unknown() {
        assert_eq!(
            HostingProvider::from_url("unknown"),
            HostingProvider::Unknown
        );
    }

    #[test]
    fn test_repo_status_clean() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert!(!status.is_dirty);
        assert_eq!(status.staged_count, 0);
        assert_eq!(status.unstaged_count, 0);
        assert_eq!(status.untracked_count, 0);
        assert!(status.dirty.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_repo_status_with_untracked() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an untracked file
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.untracked_count, 1);
        assert_eq!(status.untracked.len(), 1);

        let untracked = &status.untracked[0];
        assert_eq!(untracked.filepath, PathBuf::from("test.txt"));
        assert!(untracked.absolute_filepath.ends_with("test.txt"));
    }

    #[test]
    fn test_repo_status_with_unstaged_changes() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit with a file
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial content").unwrap();

        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Modify the file (unstaged change)
        std::fs::write(&file_path, "modified content").unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 1);
        assert_eq!(status.dirty.len(), 1);

        let dirty = &status.dirty[0];
        assert_eq!(dirty.filepath, PathBuf::from("test.txt"));
        assert!(!dirty.last_local_commit.is_empty());
        // Diff should contain the change
        assert!(dirty.diff.contains("initial content") || dirty.diff.contains("modified content"));
    }

    #[test]
    fn test_repo_status_with_staged_changes() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit with a file
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial content").unwrap();

        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Modify and stage the file
        std::fs::write(&file_path, "staged content").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
        }

        let status = get_repo_status(&repo).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.dirty.len(), 1);

        let dirty = &status.dirty[0];
        assert_eq!(dirty.filepath, PathBuf::from("test.txt"));
        // Diff should contain the staged change
        assert!(!dirty.diff.is_empty());
    }

    #[test]
    fn test_repo_status_with_new_staged_file() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial empty commit
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        {
            let mut index = repo.index().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Create and stage a new file
        let file_path = dir.path().join("new_file.txt");
        std::fs::write(&file_path, "new file content").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("new_file.txt")).unwrap();
            index.write().unwrap();
        }

        let status = get_repo_status(&repo).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.dirty.len(), 1);

        let dirty = &status.dirty[0];
        assert_eq!(dirty.filepath, PathBuf::from("new_file.txt"));
    }

    #[test]
    fn test_dirty_file_has_correct_paths() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit with a file
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let file_path = dir.path().join("subdir");
        std::fs::create_dir(&file_path).unwrap();
        let nested_file = file_path.join("nested.txt");
        std::fs::write(&nested_file, "content").unwrap();

        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("subdir/nested.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Modify the nested file
        std::fs::write(&nested_file, "modified").unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert_eq!(status.dirty.len(), 1);

        let dirty = &status.dirty[0];
        // Relative path should be from repo root
        assert_eq!(dirty.filepath, PathBuf::from("subdir/nested.txt"));
        // Absolute path should be full path
        assert!(dirty.absolute_filepath.is_absolute() || dirty.absolute_filepath.starts_with("/"));
        assert!(dirty.absolute_filepath.ends_with("subdir/nested.txt"));
    }

    #[test]
    fn test_untracked_file_has_correct_paths() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create untracked file at root level (simpler case, avoids directory folding)
        std::fs::write(dir.path().join("untracked.txt"), "content").unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert_eq!(status.untracked.len(), 1);

        let untracked = &status.untracked[0];
        assert_eq!(untracked.filepath, PathBuf::from("untracked.txt"));
        assert!(untracked.absolute_filepath.ends_with("untracked.txt"));
    }

    #[test]
    fn test_untracked_nested_file_paths() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit so git doesn't fold directories
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        {
            let mut index = repo.index().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Create nested untracked file
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("untracked.txt"), "content").unwrap();

        // Use recurse_untracked_dirs option to get full paths
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts)).unwrap();
        assert!(statuses.len() >= 1);

        // Verify we can get the nested path
        let entry = statuses.iter().next().unwrap();
        let path = entry.path().unwrap();
        assert!(path.contains("untracked.txt"));
    }

    #[test]
    fn test_diff_contains_unified_format() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit with a file
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        // Modify the file
        std::fs::write(&file_path, "line1\nmodified\nline3\n").unwrap();

        let status = get_repo_status(&repo).unwrap();
        assert_eq!(status.dirty.len(), 1);

        let diff = &status.dirty[0].diff;
        // Should contain diff markers
        assert!(diff.contains('-') || diff.contains('+'));
    }

    #[test]
    fn test_worktrees_empty_for_normal_repo() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid repo
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // A normal repo without linked worktrees should have an empty worktrees map
        assert!(info.worktrees.is_empty());
    }

    #[test]
    fn test_remotes_branches_none_without_deep() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid repo
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Add a remote (even though it won't be reachable, we just need to test the struct)
        repo.remote("origin", "https://github.com/example/repo.git")
            .unwrap();

        // Without deep mode, branches should be None
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.remotes.len(), 1);
        assert_eq!(info.remotes[0].name, "origin");
        assert!(info.remotes[0].branches.is_none());
    }

    #[test]
    fn test_is_behind_none_without_deep() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid repo
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Without deep mode, is_behind should be None
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(info.status.is_behind.is_none());
    }

    #[test]
    fn test_commit_remotes_none_without_deep() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid repo
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Without deep mode, commit remotes should be None
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(!info.recent.is_empty());
        // All commits should have remotes as None when not in deep mode
        for commit in &info.recent {
            assert!(commit.remotes.is_none());
        }
    }

    #[test]
    fn test_recent_commits_returns_multiple() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();

        // Create 7 commits
        let mut parent_commit = None;
        for i in 1..=7 {
            let tree_id = {
                let mut index = repo.index().unwrap();
                // Create a file to make each commit have content
                let file_path = dir.path().join(format!("file{}.txt", i));
                std::fs::write(&file_path, format!("content {}", i)).unwrap();
                index
                    .add_path(Path::new(&format!("file{}.txt", i)))
                    .unwrap();
                index.write().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            let message = format!("Commit {}", i);

            let commit_id = if let Some(parent) = parent_commit {
                let parent_commit_obj = repo.find_commit(parent).unwrap();
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &message,
                    &tree,
                    &[&parent_commit_obj],
                )
                .unwrap()
            } else {
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])
                    .unwrap()
            };
            parent_commit = Some(commit_id);
        }

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();

        // Should have 7 commits (or up to 10 if we had more)
        assert_eq!(info.recent.len(), 7);

        // First commit should be HEAD (most recent = "Commit 7")
        assert_eq!(info.recent[0].message, "Commit 7");

        // Should be in reverse chronological order
        assert_eq!(info.recent[1].message, "Commit 6");
        assert_eq!(info.recent[2].message, "Commit 5");
        assert_eq!(info.recent[6].message, "Commit 1");
    }

    #[test]
    fn test_recent_commits_returns_correct_fields() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Use distinct author names and messages
        let sig1 = git2::Signature::now("Alice Author", "alice@example.com").unwrap();
        let sig2 = git2::Signature::now("Bob Builder", "bob@example.com").unwrap();

        // Create first commit with Alice
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("alice.txt");
            std::fs::write(&file_path, "alice content").unwrap();
            index.add_path(Path::new("alice.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let first_commit_id = repo
            .commit(
                Some("HEAD"),
                &sig1,
                &sig1,
                "  First commit with whitespace  \n",
                &tree,
                &[],
            )
            .unwrap();

        // Create second commit with Bob
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("bob.txt");
            std::fs::write(&file_path, "bob content").unwrap();
            index.add_path(Path::new("bob.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let first_commit_obj = repo.find_commit(first_commit_id).unwrap();
        let second_commit_id = repo
            .commit(
                Some("HEAD"),
                &sig2,
                &sig2,
                "Second commit\n\nWith body",
                &tree,
                &[&first_commit_obj],
            )
            .unwrap();

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.recent.len(), 2);

        // Most recent commit (HEAD) should be Bob's
        let head_commit = &info.recent[0];
        assert_eq!(head_commit.sha, second_commit_id.to_string());
        assert_eq!(head_commit.author, "Bob Builder");
        // Message should be trimmed
        assert_eq!(head_commit.message, "Second commit\n\nWith body");

        // Second commit should be Alice's
        let older_commit = &info.recent[1];
        assert_eq!(older_commit.sha, first_commit_id.to_string());
        assert_eq!(older_commit.author, "Alice Author");
        // Whitespace should be trimmed from message
        assert_eq!(older_commit.message, "First commit with whitespace");

        // Timestamps should be non-zero and reasonable
        assert!(head_commit.timestamp.timestamp() > 0);
        assert!(older_commit.timestamp.timestamp() > 0);
    }

    #[test]
    fn test_recent_commits_empty_for_no_commits() {
        let dir = TempDir::new().unwrap();
        let _repo = Repository::init(dir.path()).unwrap();

        // Repo is initialized but has no commits
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // recent should be empty (not an error) for a repo with no commits
        assert!(info.recent.is_empty());
        // current_branch should also be None for empty repo
        assert!(info.current_branch.is_none());
    }

    #[test]
    fn test_recent_commits_handles_revwalk_correctly() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();

        // Create initial commit on main branch
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("main.txt");
            std::fs::write(&file_path, "main content").unwrap();
            index.add_path(Path::new("main.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let initial_commit_id = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial on main", &tree, &[])
            .unwrap();

        // Get the actual initial branch name (could be "master" or "main" depending on git config)
        let initial_branch_name = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "master".to_string());

        // Create second commit on main
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("main2.txt");
            std::fs::write(&file_path, "main2 content").unwrap();
            index.add_path(Path::new("main2.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let initial_commit = repo.find_commit(initial_commit_id).unwrap();
        let second_main_id = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Second on main",
                &tree,
                &[&initial_commit],
            )
            .unwrap();

        // Create a branch from initial commit (not from HEAD)
        repo.branch(
            "feature",
            &repo.find_commit(initial_commit_id).unwrap(),
            false,
        )
        .unwrap();

        // Switch to feature branch and add a commit
        repo.set_head("refs/heads/feature").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("feature.txt");
            std::fs::write(&file_path, "feature content").unwrap();
            index.add_path(Path::new("feature.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let initial_commit = repo.find_commit(initial_commit_id).unwrap();
        let feature_commit_id = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Commit on feature",
                &tree,
                &[&initial_commit],
            )
            .unwrap();

        // Now detect from feature branch
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();

        // Should only have commits from feature branch ancestry:
        // "Commit on feature" and "Initial on main"
        // Should NOT include "Second on main" (on different branch)
        assert_eq!(info.recent.len(), 2);

        let commit_messages: Vec<&str> = info.recent.iter().map(|c| c.message.as_str()).collect();
        assert!(commit_messages.contains(&"Commit on feature"));
        assert!(commit_messages.contains(&"Initial on main"));
        assert!(!commit_messages.contains(&"Second on main"));

        // Verify order: feature commit should be first (HEAD)
        assert_eq!(info.recent[0].sha, feature_commit_id.to_string());
        assert_eq!(info.recent[1].sha, initial_commit_id.to_string());

        // Verify main branch still has its commits (switch back and check)
        // Use the actual initial branch name we captured earlier
        repo.set_head(&format!("refs/heads/{}", initial_branch_name))
            .unwrap();
        // Also checkout to update the working directory
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .unwrap();

        let result_main = detect_git(dir.path(), false).unwrap().unwrap();
        let main_messages: Vec<&str> = result_main
            .recent
            .iter()
            .map(|c| c.message.as_str())
            .collect();
        assert!(main_messages.contains(&"Second on main"));
        assert!(main_messages.contains(&"Initial on main"));
        assert!(!main_messages.contains(&"Commit on feature"));
        assert_eq!(result_main.recent[0].sha, second_main_id.to_string());
    }

    #[test]
    fn test_worktree_info_serialization() {
        let worktree = WorktreeInfo {
            branch: "feature-branch".to_string(),
            filepath: PathBuf::from("/path/to/worktree"),
            sha: "abc123def456".to_string(),
            dirty: true,
        };

        let json = serde_json::to_string(&worktree).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["branch"], "feature-branch");
        assert_eq!(parsed["filepath"], "/path/to/worktree");
        assert_eq!(parsed["sha"], "abc123def456");
        assert_eq!(parsed["dirty"], true);

        // Verify roundtrip
        let deserialized: WorktreeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.branch, worktree.branch);
        assert_eq!(deserialized.filepath, worktree.filepath);
        assert_eq!(deserialized.sha, worktree.sha);
        assert_eq!(deserialized.dirty, worktree.dirty);
    }

    #[test]
    fn test_remote_info_with_branches_serialization() {
        // Test with branches populated
        let remote_with_branches = RemoteInfo {
            name: "origin".to_string(),
            url: Some("https://github.com/user/repo".to_string()),
            provider: HostingProvider::GitHub,
            branches: Some(vec!["main".to_string(), "develop".to_string()]),
        };

        let json_with = serde_json::to_string(&remote_with_branches).unwrap();
        let parsed_with: serde_json::Value = serde_json::from_str(&json_with).unwrap();

        assert_eq!(parsed_with["name"], "origin");
        assert_eq!(parsed_with["url"], "https://github.com/user/repo");
        assert_eq!(parsed_with["provider"], "GitHub");
        assert!(parsed_with["branches"].is_array());
        let branches = parsed_with["branches"].as_array().unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0], "main");
        assert_eq!(branches[1], "develop");

        // Test with branches as None (should be excluded due to skip_serializing_if)
        let remote_without_branches = RemoteInfo {
            name: "upstream".to_string(),
            url: Some("https://github.com/other/repo".to_string()),
            provider: HostingProvider::GitHub,
            branches: None,
        };

        let json_without = serde_json::to_string(&remote_without_branches).unwrap();
        let parsed_without: serde_json::Value = serde_json::from_str(&json_without).unwrap();

        assert_eq!(parsed_without["name"], "upstream");
        // branches field should be absent (not null)
        assert!(parsed_without.get("branches").is_none());
    }

    #[test]
    fn test_behind_status_serialization() {
        // NotBehind should serialize as false
        let not_behind = BehindStatus::NotBehind;
        let json = serde_json::to_string(&not_behind).unwrap();
        assert_eq!(json, "false");

        // Behind should serialize as array
        let behind = BehindStatus::Behind(vec!["origin".to_string(), "upstream".to_string()]);
        let json = serde_json::to_string(&behind).unwrap();
        assert_eq!(json, r#"["origin","upstream"]"#);

        // Empty Behind should serialize as empty array
        let behind_empty = BehindStatus::Behind(vec![]);
        let json = serde_json::to_string(&behind_empty).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn test_behind_status_deserialization() {
        // false should deserialize to NotBehind
        let not_behind: BehindStatus = serde_json::from_str("false").unwrap();
        assert_eq!(not_behind, BehindStatus::NotBehind);

        // array should deserialize to Behind
        let behind: BehindStatus = serde_json::from_str(r#"["origin","upstream"]"#).unwrap();
        assert_eq!(
            behind,
            BehindStatus::Behind(vec!["origin".to_string(), "upstream".to_string()])
        );

        // empty array should deserialize to Behind with empty vec
        let behind_empty: BehindStatus = serde_json::from_str("[]").unwrap();
        assert_eq!(behind_empty, BehindStatus::Behind(vec![]));

        // true should fail to deserialize
        let result: std::result::Result<BehindStatus, _> = serde_json::from_str("true");
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_status_with_is_behind_serialization() {
        // Test with is_behind populated (behind some remotes)
        let status_behind = RepoStatus {
            is_dirty: false,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            dirty: vec![],
            untracked: vec![],
            is_behind: Some(BehindStatus::Behind(vec![
                "origin".to_string(),
                "upstream".to_string(),
            ])),
        };

        let json_behind = serde_json::to_string(&status_behind).unwrap();
        let parsed_behind: serde_json::Value = serde_json::from_str(&json_behind).unwrap();

        assert_eq!(parsed_behind["is_dirty"], false);
        assert!(parsed_behind["is_behind"].is_array());
        let behind = parsed_behind["is_behind"].as_array().unwrap();
        assert_eq!(behind.len(), 2);
        assert_eq!(behind[0], "origin");
        assert_eq!(behind[1], "upstream");

        // Test with is_behind = NotBehind (should serialize as false)
        let status_not_behind_deep = RepoStatus {
            is_dirty: true,
            staged_count: 1,
            unstaged_count: 0,
            untracked_count: 0,
            dirty: vec![],
            untracked: vec![],
            is_behind: Some(BehindStatus::NotBehind),
        };

        let json_not_behind = serde_json::to_string(&status_not_behind_deep).unwrap();
        let parsed_not_behind: serde_json::Value = serde_json::from_str(&json_not_behind).unwrap();

        assert_eq!(parsed_not_behind["is_dirty"], true);
        // is_behind should be present and equal to false
        assert_eq!(parsed_not_behind["is_behind"], false);

        // Test with is_behind as None (should be excluded due to skip_serializing_if)
        let status_no_deep = RepoStatus {
            is_dirty: true,
            staged_count: 1,
            unstaged_count: 0,
            untracked_count: 0,
            dirty: vec![],
            untracked: vec![],
            is_behind: None,
        };

        let json_no_deep = serde_json::to_string(&status_no_deep).unwrap();
        let parsed_no_deep: serde_json::Value = serde_json::from_str(&json_no_deep).unwrap();

        assert_eq!(parsed_no_deep["is_dirty"], true);
        // is_behind field should be absent (not null) when --deep not used
        assert!(parsed_no_deep.get("is_behind").is_none());
    }

    #[test]
    fn test_commit_info_with_remotes_serialization() {
        use chrono::TimeZone;

        // Test with remotes populated
        let commit_with_remotes = CommitInfo {
            sha: "abc123def456789".to_string(),
            message: "Add feature X".to_string(),
            author: "Jane Doe".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 0).unwrap(),
            remotes: Some(vec!["origin".to_string()]),
        };

        let json_with = serde_json::to_string(&commit_with_remotes).unwrap();
        let parsed_with: serde_json::Value = serde_json::from_str(&json_with).unwrap();

        assert_eq!(parsed_with["sha"], "abc123def456789");
        assert_eq!(parsed_with["message"], "Add feature X");
        assert_eq!(parsed_with["author"], "Jane Doe");
        assert!(parsed_with["remotes"].is_array());
        let remotes = parsed_with["remotes"].as_array().unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0], "origin");

        // Test with remotes as None (should be excluded due to skip_serializing_if)
        let commit_without_remotes = CommitInfo {
            sha: "def789abc123456".to_string(),
            message: "Fix bug Y".to_string(),
            author: "John Smith".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 1, 14, 9, 0, 0).unwrap(),
            remotes: None,
        };

        let json_without = serde_json::to_string(&commit_without_remotes).unwrap();
        let parsed_without: serde_json::Value = serde_json::from_str(&json_without).unwrap();

        assert_eq!(parsed_without["sha"], "def789abc123456");
        assert_eq!(parsed_without["message"], "Fix bug Y");
        // remotes field should be absent (not null)
        assert!(parsed_without.get("remotes").is_none());

        // Verify roundtrip for both cases
        let deserialized_with: CommitInfo = serde_json::from_str(&json_with).unwrap();
        assert_eq!(
            deserialized_with.remotes,
            Some(vec!["origin".to_string()])
        );

        let deserialized_without: CommitInfo = serde_json::from_str(&json_without).unwrap();
        assert_eq!(deserialized_without.remotes, None);
    }

    #[test]
    fn test_detect_git_deep_false_is_default_behavior() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = dir.path().join("test.txt");
            std::fs::write(&file_path, "content").unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Add a remote
        repo.remote("origin", "https://github.com/example/repo.git")
            .unwrap();

        // Call detect_git with deep=false
        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();

        // Verify all network-dependent fields are NOT populated (None)

        // 1. RemoteInfo.branches should be None
        assert!(!info.remotes.is_empty());
        for remote in &info.remotes {
            assert!(
                remote.branches.is_none(),
                "branches should be None when deep=false"
            );
        }

        // 2. RepoStatus.is_behind should be None
        assert!(
            info.status.is_behind.is_none(),
            "is_behind should be None when deep=false"
        );

        // 3. CommitInfo.remotes should be None for all commits
        assert!(!info.recent.is_empty());
        for commit in &info.recent {
            assert!(
                commit.remotes.is_none(),
                "commit.remotes should be None when deep=false"
            );
        }

        // Verify basic fields are still populated correctly
        assert!(info.current_branch.is_some());
        assert!(!info.in_worktree);
        assert!(info.worktrees.is_empty());
        assert_eq!(info.remotes.len(), 1);
        assert_eq!(info.remotes[0].name, "origin");
        assert_eq!(info.remotes[0].provider, HostingProvider::GitHub);
    }

    #[test]
    fn test_recent_commits_limited_to_10() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();

        // Create 15 commits (more than the limit of 10)
        let mut parent_commit = None;
        for i in 1..=15 {
            let tree_id = {
                let mut index = repo.index().unwrap();
                let file_path = dir.path().join(format!("file{}.txt", i));
                std::fs::write(&file_path, format!("content {}", i)).unwrap();
                index
                    .add_path(Path::new(&format!("file{}.txt", i)))
                    .unwrap();
                index.write().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            let message = format!("Commit {}", i);

            let commit_id = if let Some(parent) = parent_commit {
                let parent_commit_obj = repo.find_commit(parent).unwrap();
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &message,
                    &tree,
                    &[&parent_commit_obj],
                )
                .unwrap()
            } else {
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])
                    .unwrap()
            };
            parent_commit = Some(commit_id);
        }

        let result = detect_git(dir.path(), false).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();

        // Should be limited to 10 commits even though 15 exist
        assert_eq!(info.recent.len(), 10);

        // Most recent commits should be included (15 down to 6)
        assert_eq!(info.recent[0].message, "Commit 15");
        assert_eq!(info.recent[9].message, "Commit 6");

        // Commits 1-5 should not be included
        let messages: Vec<&str> = info.recent.iter().map(|c| c.message.as_str()).collect();
        assert!(!messages.contains(&"Commit 1"));
        assert!(!messages.contains(&"Commit 5"));
    }
}

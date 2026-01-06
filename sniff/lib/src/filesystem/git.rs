use chrono::{DateTime, Utc};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
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
/// let git_info = detect_git(Path::new(".")).unwrap();
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
    /// HEAD commit information.
    pub head_commit: Option<CommitInfo>,
    /// Working tree status.
    pub status: RepoStatus,
    /// Configured remotes.
    pub remotes: Vec<RemoteInfo>,
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
/// let git_info = detect_git(Path::new(".")).unwrap().unwrap();
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
/// let git_info = detect_git(Path::new(".")).unwrap().unwrap();
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
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let result = detect_git(Path::new(".")).unwrap();
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
pub fn detect_git(path: &Path) -> Result<Option<GitInfo>> {
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

    let head_commit = get_head_commit(&repo);
    let status = get_repo_status(&repo)?;
    let remotes = get_remotes(&repo);

    Ok(Some(GitInfo {
        repo_root,
        current_branch,
        head_commit,
        status,
        remotes,
    }))
}

/// Extracts HEAD commit information.
fn get_head_commit(repo: &Repository) -> Option<CommitInfo> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let author = commit.author();

    Some(CommitInfo {
        sha: commit.id().to_string(),
        message: commit.message().unwrap_or("").trim().to_string(),
        author: author.name().unwrap_or("Unknown").to_string(),
        timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
    })
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
            if let Some(ref p) = path {
                if !dirty_paths.contains(p) {
                    dirty_paths.push(p.clone());
                }
            }
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
            if let Some(ref p) = path {
                if !dirty_paths.contains(p) {
                    dirty_paths.push(p.clone());
                }
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
fn get_remotes(repo: &Repository) -> Vec<RemoteInfo> {
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
                        RemoteInfo {
                            name: name.to_string(),
                            url,
                            provider,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_non_git_directory_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = detect_git(dir.path()).unwrap();
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

        let result = detect_git(dir.path()).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // Use canonicalize to handle /private/var vs /var on macOS
        assert_eq!(
            info.repo_root.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
        assert!(info.current_branch.is_some());
        assert!(info.head_commit.is_some());
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
}

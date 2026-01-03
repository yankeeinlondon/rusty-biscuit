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
/// Tracks staged, unstaged, and untracked changes in the repository.
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

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    Ok(RepoStatus {
        is_dirty: staged > 0 || unstaged > 0 || untracked > 0,
        staged_count: staged,
        unstaged_count: unstaged,
        untracked_count: untracked,
    })
}

/// Retrieves all configured remotes with their URLs and hosting providers.
fn get_remotes(repo: &Repository) -> Vec<RemoteInfo> {
    repo.remotes()
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name)
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
    }
}

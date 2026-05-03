use chrono::{DateTime, Utc};
use git2::Repository;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{debug, instrument, warn};

use crate::request::GitRequest;
use crate::{Result, SniffError};

const CONVENTIONAL_COMMIT_RE: &str = r"^([a-zA-Z0-9-]+)(?:\(([^)]*)\))?: (.+)$";

/// Git user configuration.
///
/// Contains user identity from git config (user.name, user.email).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitConfig {
    /// User name from git config (user.name).
    pub user_name: Option<String>,
    /// User email from git config (user.email).
    pub user_email: Option<String>,
    /// GPG agent usage (gpg.useAgent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpg_use_agent: Option<bool>,
    /// GPG program path (gpg.program).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpg_program: Option<String>,
    /// Credential helper (credential.helper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_helper: Option<String>,
    /// GPG signing key (user.signingkey).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_key: Option<String>,
    /// Whether commits are signed by default (commit.gpgsign).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sign: Option<bool>,
    /// Whether tags are signed by default (tag.gpgsign).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_sign: Option<bool>,
    /// Configured pager (core.pager).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pager: Option<String>,
    /// Delta syntax theme (delta.syntax-theme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_syntax_theme: Option<String>,
    /// Delta light mode (delta.light).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_light: Option<bool>,
    /// Delta side-by-side mode (delta.side-by-side).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_side_by_side: Option<bool>,
}

/// Parsed conventional commit.
///
/// Represents a commit message following the conventional commits spec:
/// `type(scope): description`
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::git::ConventionalCommit;
///
/// let commit = ConventionalCommit::parse("feat(cli): add new flag");
/// assert_eq!(commit.operation, Some("feat".to_string()));
/// assert_eq!(commit.scope, Some("cli".to_string()));
/// assert_eq!(commit.description, "add new flag");
///
/// let plain = ConventionalCommit::parse("Regular commit message");
/// assert_eq!(plain.operation, None);
/// assert_eq!(plain.description, "Regular commit message");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConventionalCommit {
    /// Commit type (feat, fix, chore, etc.) if conventional format.
    pub operation: Option<String>,
    /// Scope within parentheses if present.
    pub scope: Option<String>,
    /// The commit description (first line after type/scope).
    pub description: String,
}

impl ConventionalCommit {
    /// Parse a commit message into a conventional commit structure.
    ///
    /// Only parses the first line of the message. If the message doesn't
    /// follow conventional commit format, returns a struct with only
    /// the description populated.
    pub fn parse(message: &str) -> Self {
        let first_line = message.lines().next().unwrap_or("").trim();

        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(CONVENTIONAL_COMMIT_RE).expect("valid regex"));

        if let Some(caps) = re.captures(first_line) {
            let scope = caps.get(2).and_then(|m| {
                let s = m.as_str();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            Self {
                operation: Some(caps.get(1).unwrap().as_str().to_string()),
                scope,
                description: caps.get(3).unwrap().as_str().to_string(),
            }
        } else {
            Self {
                operation: None,
                scope: None,
                description: first_line.to_string(),
            }
        }
    }
}

/// File change status in the working tree.
///
/// Distinguishes between staged-only, modified-only, conflicted, and
/// untracked states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// File is staged (in index) but not modified in working tree.
    Staged,
    /// File is modified in working tree but not staged.
    Modified,
    /// File is both staged and has additional modifications.
    Both,
    /// File is in an unmerged merge-conflict state.
    Conflicted,
    /// File is new/untracked.
    Untracked,
}

/// The kind of change applied to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileAction {
    /// Newly created file.
    Created,
    /// Existing file was modified.
    Modified,
    /// File was deleted.
    Deleted,
}

impl FileAction {
    /// Human-readable label for display.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
        }
    }
}

/// A file change with its status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Relative path from repository root.
    pub path: PathBuf,
    /// Status of the file.
    pub status: FileStatus,
    /// The kind of change (created, modified, deleted).
    pub action: FileAction,
    /// Number of lines added in this change.
    pub lines_added: usize,
    /// Number of lines removed in this change.
    pub lines_removed: usize,
}

/// Tracking status for a remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTrackingStatus {
    /// Remote name (e.g., "origin").
    pub remote: String,
    /// Number of commits local is ahead of remote.
    pub ahead: usize,
    /// Number of commits local is behind remote.
    pub behind: usize,
}

/// Information about a local branch.
///
/// Contains the branch name, abbreviated commit hash, and ahead/behind
/// counts relative to the current branch's HEAD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalBranchInfo {
    /// Branch name (e.g., "main", "feature/xyz").
    pub name: String,
    /// Abbreviated commit hash for the branch tip (first 8 chars).
    pub short_hash: String,
    /// Number of commits this branch is ahead of the current branch.
    pub ahead: usize,
    /// Number of commits this branch is behind the current branch.
    pub behind: usize,
}

/// Git hosting provider types.
///
/// Identifies the hosting platform for a Git repository based on its remote URL.
/// The `#[non_exhaustive]` attribute allows future additions without breaking changes.
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::git::GitHostingProvider;
///
/// let provider = GitHostingProvider::from_url("https://github.com/user/repo");
/// assert_eq!(provider, GitHostingProvider::GitHub);
///
/// let provider = GitHostingProvider::from_url("git@gitlab.com:user/repo.git");
/// assert_eq!(provider, GitHostingProvider::GitLab);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GitHostingProvider {
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

/// Static metadata for a Git hosting provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHostingProviderMetadata {
    /// Human-readable provider name.
    pub display_name: &'static str,
    /// Short ASCII symbol used in compact CLI output.
    pub symbol: &'static str,
    /// Whether this provider is typically self-hosted.
    pub self_hosted: bool,
    /// Whether hostname matching is generally enough for detection.
    pub host_based_detection_reliable: bool,
    /// Canonical browser base URL when one exists.
    pub browser_base_url: Option<&'static str>,
}

impl GitHostingProvider {
    /// Returns static metadata for the hosting provider.
    pub const fn metadata(&self) -> GitHostingProviderMetadata {
        match self {
            Self::GitHub => GitHostingProviderMetadata {
                display_name: "GitHub",
                symbol: "[gh]",
                self_hosted: false,
                host_based_detection_reliable: true,
                browser_base_url: Some("https://github.com"),
            },
            Self::GitLab => GitHostingProviderMetadata {
                display_name: "GitLab",
                symbol: "[gl]",
                self_hosted: false,
                host_based_detection_reliable: false,
                browser_base_url: Some("https://gitlab.com"),
            },
            Self::Bitbucket => GitHostingProviderMetadata {
                display_name: "Bitbucket",
                symbol: "[bb]",
                self_hosted: false,
                host_based_detection_reliable: true,
                browser_base_url: Some("https://bitbucket.org"),
            },
            Self::AzureDevOps => GitHostingProviderMetadata {
                display_name: "Azure DevOps",
                symbol: "[az]",
                self_hosted: false,
                host_based_detection_reliable: true,
                browser_base_url: Some("https://dev.azure.com"),
            },
            Self::AwsCodeCommit => GitHostingProviderMetadata {
                display_name: "AWS CodeCommit",
                symbol: "[aws]",
                self_hosted: false,
                host_based_detection_reliable: false,
                browser_base_url: None,
            },
            Self::Gitea => GitHostingProviderMetadata {
                display_name: "Gitea",
                symbol: "[ga]",
                self_hosted: true,
                host_based_detection_reliable: false,
                browser_base_url: None,
            },
            Self::Forgejo => GitHostingProviderMetadata {
                display_name: "Forgejo",
                symbol: "[fj]",
                self_hosted: true,
                host_based_detection_reliable: false,
                browser_base_url: None,
            },
            Self::SourceHut => GitHostingProviderMetadata {
                display_name: "SourceHut",
                symbol: "[sh]",
                self_hosted: false,
                host_based_detection_reliable: true,
                browser_base_url: Some("https://sr.ht"),
            },
            Self::SelfHosted => GitHostingProviderMetadata {
                display_name: "Self-Hosted",
                symbol: "[git]",
                self_hosted: true,
                host_based_detection_reliable: false,
                browser_base_url: None,
            },
            Self::Unknown => GitHostingProviderMetadata {
                display_name: "Unknown",
                symbol: "[?]",
                self_hosted: false,
                host_based_detection_reliable: false,
                browser_base_url: None,
            },
        }
    }

    /// Human-readable provider name.
    pub const fn display_name(&self) -> &'static str {
        self.metadata().display_name
    }

    /// Short ASCII symbol for compact CLI display.
    pub const fn symbol(&self) -> &'static str {
        self.metadata().symbol
    }

    /// Canonical browser base URL when one exists.
    pub const fn browser_base_url(&self) -> Option<&'static str> {
        self.metadata().browser_base_url
    }

    /// Returns the URL path segment used to view a single commit.
    ///
    /// Combined with the browse URL and a SHA to form a full commit link:
    /// `{browse_url}/{commit_path_segment}/{sha}`
    pub const fn commit_path_segment(&self) -> &'static str {
        match self {
            Self::GitLab => "-/commit",
            Self::Bitbucket => "commits",
            Self::SourceHut => "commit",
            // GitHub, Gitea, Forgejo, AzureDevOps, and most others use "commit"
            _ => "commit",
        }
    }

    /// Detects the hosting provider from a Git remote URL.
    ///
    /// Supports HTTPS, SSH, and git protocol URLs.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::filesystem::git::GitHostingProvider;
    ///
    /// assert_eq!(
    ///     GitHostingProvider::from_url("https://github.com/user/repo"),
    ///     GitHostingProvider::GitHub
    /// );
    /// assert_eq!(
    ///     GitHostingProvider::from_url("git@bitbucket.org:user/repo.git"),
    ///     GitHostingProvider::Bitbucket
    /// );
    /// assert_eq!(
    ///     GitHostingProvider::from_url("https://git.company.com/repo"),
    ///     GitHostingProvider::SelfHosted
    /// );
    /// ```
    pub fn from_url(url: &str) -> Self {
        let host = match extract_remote_host(url) {
            Some(host) => host.to_ascii_lowercase(),
            None => return Self::Unknown,
        };

        if host == "github.com" || host == "www.github.com" {
            return Self::GitHub;
        }
        if host == "gitlab.com" || host == "www.gitlab.com" {
            return Self::GitLab;
        }
        if host == "bitbucket.org" || host == "www.bitbucket.org" {
            return Self::Bitbucket;
        }
        if host == "dev.azure.com" || host.ends_with(".visualstudio.com") {
            return Self::AzureDevOps;
        }
        if (host.starts_with("git-codecommit.") || host.contains("codecommit"))
            && host.contains("amazonaws.com")
        {
            return Self::AwsCodeCommit;
        }
        if host == "sr.ht" || host.ends_with(".sr.ht") {
            return Self::SourceHut;
        }
        if host == "codeberg.org" || host.starts_with("forgejo.") || host.contains(".forgejo.") {
            return Self::Forgejo;
        }
        if host.starts_with("gitea.") || host.contains(".gitea.") {
            return Self::Gitea;
        }
        if host.contains('.') {
            Self::SelfHosted
        } else {
            Self::Unknown
        }
    }
}

/// Extract a hostname from common git remote URL formats.
fn extract_remote_host(url: &str) -> Option<&str> {
    let trimmed = url.trim();

    // Scheme-based URLs (https://, http://, ssh://, git://)
    if let Some(without_scheme) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
        .or_else(|| trimmed.strip_prefix("git://"))
    {
        let without_user = without_scheme
            .rsplit_once('@')
            .map(|(_, rest)| rest)
            .unwrap_or(without_scheme);
        let host_port = without_user.split('/').next()?;
        let host = host_port.split(':').next().unwrap_or(host_port);
        return if host.is_empty() { None } else { Some(host) };
    }

    // SCP-style SSH URL: git@host:owner/repo.git
    if let Some((_, after_at)) = trimmed.split_once('@')
        && let Some((host, _)) = after_at.split_once(':')
    {
        return if host.is_empty() { None } else { Some(host) };
    }

    None
}

/// Lightweight handle to a discovered git repository.
///
/// Wraps a `git2::Repository` and exposes atomic query methods so callers
/// can request only the data they need without paying for a full
/// `detect_git` sweep.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::git::GitRepo;
/// use std::path::Path;
///
/// if let Some(repo) = GitRepo::discover(Path::new(".")).unwrap() {
///     println!("root: {:?}", repo.repo_root());
///     let (org, name) = repo.org_and_repo();
///     println!("org={org:?}  repo={name:?}");
/// }
/// ```
pub struct GitRepo {
    repo: Repository,
    repo_root: PathBuf,
    /// Cached ref decorations to avoid recomputing on every commit query.
    ref_decorations: RefCell<Option<HashMap<git2::Oid, Vec<RefDecoration>>>>,
}

impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepo")
            .field("repo_root", &self.repo_root)
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    /// Returns cached ref decorations, computing them once on first access.
    pub(crate) fn ref_decorations(
        &self,
    ) -> std::cell::Ref<'_, HashMap<git2::Oid, Vec<RefDecoration>>> {
        // If not yet computed, compute and cache
        if self.ref_decorations.borrow().is_none() {
            let decorations = super::detection::collect_ref_decorations(&self.repo);
            *self.ref_decorations.borrow_mut() = Some(decorations);
        }
        std::cell::Ref::map(self.ref_decorations.borrow(), |opt| opt.as_ref().unwrap())
    }
}

impl GitRepo {
    /// Discover a repository containing `path`.
    ///
    /// Returns `Ok(None)` when `path` is not inside a git repository.
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn discover(path: &Path) -> Result<Option<Self>> {
        let repo = match Repository::discover(path) {
            Ok(r) => r,
            Err(e) => {
                debug!(error = %e, "not a git repository");
                return Ok(None);
            }
        };
        let repo_root = repo
            .workdir()
            .ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?
            .to_path_buf();
        Ok(Some(Self {
            repo,
            repo_root,
            ref_decorations: RefCell::new(None),
        }))
    }

    /// Absolute path to the repository working directory.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Current branch name (`None` for detached HEAD).
    pub fn current_branch(&self) -> Option<String> {
        self.repo
            .head()
            .map_err(|e| {
                debug!(error = %e, "could not read HEAD");
                e
            })
            .ok()
            .as_ref()
            .and_then(|h| h.shorthand())
            .map(String::from)
    }

    /// Whether the working directory is a linked worktree.
    pub fn in_worktree(&self) -> bool {
        self.repo.is_worktree()
    }

    /// Base repository root when inside a worktree.
    pub fn base_repo_root(&self) -> Option<PathBuf> {
        if self.repo.is_worktree() {
            self.repo.commondir().parent().map(Path::to_path_buf)
        } else {
            None
        }
    }

    /// Organization and repository name parsed from the preferred remote URL.
    pub fn org_and_repo(&self) -> (Option<String>, Option<String>) {
        let remotes = super::detection::get_remotes(&self.repo, false);
        preferred_remote(&remotes)
            .and_then(|r| r.url.as_deref())
            .map(parse_org_repo)
            .unwrap_or((None, None))
    }

    /// File-level working tree status (staged, modified, untracked).
    pub fn file_changes(&self) -> Result<Vec<FileChange>> {
        let (_status, changes) = super::detection::get_repo_status_with_changes(&self.repo, false)?;
        Ok(changes)
    }

    /// Aggregated working tree status.
    pub fn repo_status(&self) -> Result<RepoStatus> {
        let (status, _changes) = super::detection::get_repo_status_with_changes(&self.repo, false)?;
        Ok(status)
    }

    /// Recent commits from HEAD.
    pub fn recent_commits(&self, count: usize) -> Vec<CommitInfo> {
        let decorations = self.ref_decorations();
        super::detection::get_recent_commits_with_decorations(
            &self.repo,
            count,
            Some(&*decorations),
        )
    }

    /// Configured remotes.
    pub fn remotes(&self, include_details: bool) -> Vec<RemoteInfo> {
        super::detection::get_remotes(&self.repo, include_details)
    }

    /// Linked worktrees.
    pub fn worktrees(&self) -> HashMap<String, WorktreeInfo> {
        super::detection::get_worktrees(&self.repo)
    }

    /// Git user configuration.
    pub fn config(&self) -> GitConfig {
        super::detection::get_git_config(&self.repo)
    }

    /// Local branch information.
    pub fn branches(&self) -> Vec<LocalBranchInfo> {
        let current = self.current_branch();
        super::detection::get_local_branches(&self.repo, current.as_deref())
    }

    /// Per-remote tracking status (ahead/behind).
    pub fn tracking_status(&self) -> Vec<RemoteTrackingStatus> {
        let current = self.current_branch();
        super::detection::get_tracking_status(&self.repo, current.as_deref())
    }

    /// Full detection — equivalent to `detect_git()` but reuses
    /// the already-opened repository handle.
    ///
    /// ## Notes
    ///
    /// Both `deep=false` and `deep=true` include full unified diff payloads to
    /// preserve backward compatibility. New callers that want file stats without
    /// diffs should use [`detect_with_request`] with [`GitRequest::full()`] directly.
    pub fn detect_full(&self, deep: bool, commit_count: usize) -> Result<GitInfo> {
        let request = if deep {
            GitRequest::deep().commit_count(commit_count)
        } else {
            // Preserve backward compat: detect_full always included full diffs.
            // New callers should use detect_with_request(GitRequest::full()) to
            // get the cheaper stats-only path.
            GitRequest::full()
                .include_file_diffs(true)
                .commit_count(commit_count)
        };
        self.detect_with_request(&request)
    }

    /// Detect git information according to the given request.
    ///
    /// Controls which expensive operations are performed:
    /// - `commit_count`: 0 skips commit history
    /// - `include_file_changes`: false skips per-file diff stats
    /// - `include_worktrees`: false skips worktree enumeration
    /// - `refresh_remote_tracking`: true fetches remote refs (network)
    pub fn detect_with_request(&self, request: &GitRequest) -> Result<GitInfo> {
        let current_branch = self.current_branch();

        if request.refresh_remote_tracking {
            super::detection::refresh_remote_tracking_refs(&self.repo, 2);
        }

        let mut recent = if request.commit_count > 0 {
            super::detection::get_recent_commits(&self.repo, request.commit_count)
        } else {
            Vec::new()
        };

        let (mut status, file_changes) = if request.include_file_changes {
            super::detection::get_repo_status_with_changes(&self.repo, request.include_file_diffs)?
        } else {
            let (is_dirty, staged, unstaged, untracked) =
                super::detection::get_repo_status_counts_detailed(&self.repo);
            let status = RepoStatus {
                is_dirty,
                staged_count: staged,
                unstaged_count: unstaged,
                untracked_count: untracked,
                dirty: Vec::new(),
                untracked: Vec::new(),
                is_behind: None,
            };
            (status, Vec::new())
        };

        let remotes =
            super::detection::get_remotes(&self.repo, request.include_remote_branch_details);

        let worktrees = if request.include_worktrees {
            super::detection::get_worktrees(&self.repo)
        } else {
            HashMap::new()
        };

        let config = super::detection::get_git_config(&self.repo);
        let branches = super::detection::get_local_branches(&self.repo, current_branch.as_deref());
        let tracking = super::detection::get_tracking_status(&self.repo, current_branch.as_deref());

        if request.refresh_remote_tracking {
            status.is_behind = super::detection::summarize_behind_status(&tracking);
            if request.include_commit_remote_containment {
                super::detection::populate_recent_commit_remotes(
                    &self.repo,
                    &mut recent,
                    request.max_remote_branches,
                );
            }
        }

        let (org, repo) = preferred_remote(&remotes)
            .and_then(|r| r.url.as_deref())
            .map(parse_org_repo)
            .unwrap_or((None, None));

        Ok(GitInfo {
            repo_root: self.repo_root.clone(),
            org,
            repo,
            current_branch,
            branches,
            in_worktree: self.repo.is_worktree(),
            base_repo_root: self.base_repo_root(),
            recent,
            status,
            remotes,
            worktrees,
            config,
            tracking,
            file_changes,
        })
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
/// use sniff::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false, 10).unwrap();
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
    /// Organization or owner name from the preferred remote (e.g., "rust-lang").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    /// Repository name from the preferred remote (e.g., "cargo").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Current branch name (None for detached HEAD).
    pub current_branch: Option<String>,
    /// All local branches with commit hashes and ahead/behind counts.
    pub branches: Vec<LocalBranchInfo>,
    /// Whether the current path is inside a worktree (vs main repository).
    pub in_worktree: bool,
    /// Absolute path to the base repository root (only set when inside a worktree).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_repo_root: Option<PathBuf>,
    /// Recent commits from HEAD (last 10 commits).
    pub recent: Vec<CommitInfo>,
    /// Working tree status.
    pub status: RepoStatus,
    /// Configured remotes.
    pub remotes: Vec<RemoteInfo>,
    /// Linked worktrees (keyed by branch name).
    pub worktrees: HashMap<String, WorktreeInfo>,
    /// Git user configuration (user.name, user.email).
    pub config: GitConfig,
    /// Per-remote tracking status (ahead/behind counts).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tracking: Vec<RemoteTrackingStatus>,
    /// File changes with their status (staged/modified/both/conflicted/untracked).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub file_changes: Vec<FileChange>,
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
/// use sniff::filesystem::git::BehindStatus;
///
/// // Not behind any remote
/// let status = BehindStatus::NotBehind;
/// assert_eq!(serde_json::to_string(&status).unwrap(), "false");
///
/// // Behind origin and upstream
/// let status = BehindStatus::Behind(vec!["origin".to_string(), "upstream".to_string()]);
/// assert_eq!(serde_json::to_string(&status).unwrap(), r#"["origin","upstream"]"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BehindStatus {
    /// Local branch is not behind any remote.
    #[default]
    NotBehind,
    /// Local branch is behind these remotes.
    Behind(Vec<String>),
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
    ///
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
    pub provider: GitHostingProvider,
    /// Branches available on this remote (only populated with --deep flag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,
    /// Default branch for this remote (resolved from refs/remotes/{name}/HEAD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

/// Ref decoration for a commit (branch, tag, or remote tracking ref).
///
/// Represents refs that point to a specific commit, similar to
/// `git log --decorate` output like `(HEAD -> main, origin/main)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefDecoration {
    /// The ref name (e.g., "main", "origin/main", "v1.0.0").
    pub name: String,
    /// The type of ref.
    pub kind: RefKind,
    /// Whether HEAD points to this ref (only true for local branches).
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub is_head: bool,
}

/// Type of git reference.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefKind {
    /// Local branch (refs/heads/*)
    LocalBranch,
    /// Remote tracking branch (refs/remotes/*)
    RemoteBranch,
    /// Tag (refs/tags/*)
    Tag,
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
    /// Refs pointing to this commit (branches, tags, remote tracking refs).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub refs: Vec<RefDecoration>,
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
    /// Number of commits ahead of the base branch.
    pub ahead: usize,
    /// Number of commits behind the base branch.
    pub behind: usize,
    /// The base branch used for ahead/behind comparison (e.g., "main").
    pub base_branch: String,
    /// Whether merging this worktree's HEAD into the base branch would produce conflicts.
    pub has_conflicts: bool,
    /// Whether this worktree's branch is fully merged into the base branch.
    pub merged: bool,
    /// Number of uncommitted files (staged + unstaged + untracked).
    pub changed_files: usize,
}

/// A file with uncommitted changes (staged or unstaged).
///
/// Contains path information, unified diff output, and commit references
/// for tracking the state of modified files in the working tree.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false, 10).unwrap().unwrap();
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
/// use sniff::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let git_info = detect_git(Path::new("."), false, 10).unwrap().unwrap();
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

/// Extracts the organization (owner) and repository name from a remote URL.
///
/// Supports both SSH (`git@github.com:owner/repo.git`) and HTTPS
/// (`https://github.com/owner/repo.git`) URL formats. Strips `.git` suffix
/// and splits the path into owner and repo components.
///
/// Returns `(None, None)` if the URL cannot be parsed.
fn parse_org_repo(url: &str) -> (Option<String>, Option<String>) {
    let path = if url.contains('@') && url.contains(':') {
        // SSH: git@github.com:owner/repo.git
        url.split(':')
            .next_back()
            .map(|s| s.trim_end_matches(".git").to_string())
    } else if url.contains("://") {
        // HTTPS: https://github.com/owner/repo.git
        let parts: Vec<_> = url.split('/').skip(3).collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("/").trim_end_matches(".git").to_string())
        }
    } else {
        None
    };

    match path {
        Some(p) => {
            let segments: Vec<&str> = p.splitn(2, '/').collect();
            match segments.as_slice() {
                [org, repo] if !org.is_empty() && !repo.is_empty() => {
                    (Some(org.to_string()), Some(repo.to_string()))
                }
                _ => (None, None),
            }
        }
        None => (None, None),
    }
}

/// Selects the preferred remote from a list of remotes.
///
/// Preference order:
/// 1. `origin` (always preferred)
/// 2. First alphabetically, excluding `upstream`
/// 3. `upstream` as last resort (only if it's the sole remote)
fn preferred_remote(remotes: &[RemoteInfo]) -> Option<&RemoteInfo> {
    if remotes.is_empty() {
        return None;
    }

    // Prefer "origin"
    if let Some(origin) = remotes.iter().find(|r| r.name == "origin") {
        return Some(origin);
    }

    // First non-upstream remote alphabetically
    let mut candidates: Vec<_> = remotes.iter().filter(|r| r.name != "upstream").collect();
    candidates.sort_by(|a, b| a.name.cmp(&b.name));

    if let Some(first) = candidates.first() {
        return Some(first);
    }

    // Fall back to upstream if it's the only remote
    remotes.first()
}

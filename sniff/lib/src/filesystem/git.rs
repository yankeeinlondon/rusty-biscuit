use chrono::{DateTime, Utc};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Result, SniffError};

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

        // Pattern: type(scope): description  OR  type: description
        // type must be alphanumeric (feat, fix, chore, etc.)
        let mut chars = first_line.chars().peekable();
        let mut operation = String::new();

        // Extract type (alphanumeric characters)
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '-' {
                operation.push(c);
                chars.next();
            } else {
                break;
            }
        }

        // Check if we have a valid type
        if operation.is_empty() {
            return Self {
                operation: None,
                scope: None,
                description: first_line.to_string(),
            };
        }

        // Check for scope in parentheses
        let scope = if chars.peek() == Some(&'(') {
            chars.next(); // consume '('
            let mut scope_str = String::new();
            for c in chars.by_ref() {
                if c == ')' {
                    break;
                }
                scope_str.push(c);
            }
            if scope_str.is_empty() {
                None
            } else {
                Some(scope_str)
            }
        } else {
            None
        };

        // Check for colon
        if chars.peek() != Some(&':') {
            // Not conventional format
            return Self {
                operation: None,
                scope: None,
                description: first_line.to_string(),
            };
        }
        chars.next(); // consume ':'

        // Skip whitespace after colon
        while chars.peek() == Some(&' ') {
            chars.next();
        }

        let description: String = chars.collect();

        Self {
            operation: Some(operation),
            scope,
            description,
        }
    }
}

/// File change status in the working tree.
///
/// Distinguishes between staged-only, modified-only, and both states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// File is staged (in index) but not modified in working tree.
    Staged,
    /// File is modified in working tree but not staged.
    Modified,
    /// File is both staged and has additional modifications.
    Both,
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
        let without_user =
            without_scheme.rsplit_once('@').map(|(_, rest)| rest).unwrap_or(without_scheme);
        let host_port = without_user.split('/').next()?;
        let host = host_port.split(':').next().unwrap_or(host_port);
        return if host.is_empty() {
            None
        } else {
            Some(host)
        };
    }

    // SCP-style SSH URL: git@host:owner/repo.git
    if let Some((_, after_at)) = trimmed.split_once('@')
        && let Some((host, _)) = after_at.split_once(':')
    {
        return if host.is_empty() {
            None
        } else {
            Some(host)
        };
    }

    None
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
    /// File changes with their status (staged/modified/both/untracked).
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
        url.split(':').next_back().map(|s| s.trim_end_matches(".git").to_string())
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
                [
                    org,
                    repo,
                ] if !org.is_empty() && !repo.is_empty() => {
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

/// Detects Git repository information for a given path.
///
/// Searches upward from the given path to find a Git repository.
/// Returns `None` if the path is not within a Git repository.
///
/// The `deep` parameter enables network operations for enhanced remote info
/// (e.g., fetching remote branch info, checking if local is behind remote).
/// When `false`, only local repository information is gathered.
///
/// ## Arguments
///
/// * `path` - The path to search from
/// * `deep` - Enable network operations for enhanced remote info
/// * `commit_count` - Number of recent commits to retrieve
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::git::detect_git;
/// use std::path::Path;
///
/// let result = detect_git(Path::new("."), false, 10).unwrap();
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
pub fn detect_git(path: &Path, deep: bool, commit_count: usize) -> Result<Option<GitInfo>> {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let repo_root =
        repo.workdir().ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?.to_path_buf();

    let head = repo.head().ok();
    let current_branch = head.as_ref().and_then(|h| h.shorthand()).map(String::from);

    let in_worktree = repo.is_worktree();

    // When inside a worktree, resolve the base repository root from commondir.
    // commondir points to the base repo's .git directory; its parent is the workdir.
    let base_repo_root = if in_worktree {
        repo.commondir().parent().map(Path::to_path_buf)
    } else {
        None
    };

    if deep {
        refresh_remote_tracking_refs(&repo);
    }

    let mut recent = get_recent_commits(&repo, commit_count);
    let (mut status, file_changes) = get_repo_status_with_changes(&repo)?;
    let remotes = get_remotes(&repo, deep);
    let worktrees = get_worktrees(&repo);
    let config = get_git_config(&repo);
    let branches = get_local_branches(&repo, current_branch.as_deref());
    let tracking = get_tracking_status(&repo, current_branch.as_deref());

    if deep {
        status.is_behind = summarize_behind_status(&tracking);
        populate_recent_commit_remotes(&repo, &mut recent);
    }

    let (org, repo) = preferred_remote(&remotes)
        .and_then(|r| r.url.as_deref())
        .map(parse_org_repo)
        .unwrap_or((None, None));

    Ok(Some(GitInfo {
        repo_root,
        org,
        repo,
        current_branch,
        branches,
        in_worktree,
        base_repo_root,
        recent,
        status,
        remotes,
        worktrees,
        config,
        tracking,
        file_changes,
    }))
}

/// Collects all refs (branches, remote tracking, tags) pointing to each commit.
///
/// Returns a HashMap from commit OID to a vector of ref decorations.
fn collect_ref_decorations(repo: &Repository) -> HashMap<git2::Oid, Vec<RefDecoration>> {
    let mut decorations: HashMap<git2::Oid, Vec<RefDecoration>> = HashMap::new();

    // Get current HEAD target to mark the active branch
    let head_target = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            h.shorthand().map(String::from)
        } else {
            None
        }
    });

    // Iterate all references
    let Ok(refs) = repo.references() else {
        return decorations;
    };

    for reference in refs.flatten() {
        let Some(name) = reference.name() else {
            continue;
        };

        // Resolve the reference to its target commit
        let Ok(target) = reference.peel_to_commit() else {
            continue;
        };
        let oid = target.id();

        // Determine ref kind and display name
        let (kind, display_name) = if let Some(branch) = name.strip_prefix("refs/heads/") {
            (RefKind::LocalBranch, branch.to_string())
        } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
            (RefKind::RemoteBranch, remote.to_string())
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
            (RefKind::Tag, tag.to_string())
        } else {
            continue; // Skip other refs (notes, stash, etc.)
        };

        // Check if this is the HEAD branch
        let is_head = kind == RefKind::LocalBranch
            && head_target.as_ref().is_some_and(|h| h == &display_name);

        let decoration = RefDecoration {
            name: display_name,
            kind,
            is_head,
        };

        decorations.entry(oid).or_default().push(decoration);
    }

    // Sort decorations: HEAD branch first, then local branches, remote branches, tags
    for refs in decorations.values_mut() {
        refs.sort_by(|a, b| {
            // HEAD branch comes first
            if a.is_head != b.is_head {
                return b.is_head.cmp(&a.is_head);
            }
            // Then by kind: LocalBranch < RemoteBranch < Tag
            match (a.kind, b.kind) {
                (RefKind::LocalBranch, RefKind::LocalBranch) => a.name.cmp(&b.name),
                (RefKind::LocalBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::LocalBranch) => std::cmp::Ordering::Greater,
                (RefKind::RemoteBranch, RefKind::RemoteBranch) => a.name.cmp(&b.name),
                (RefKind::RemoteBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::RemoteBranch) => std::cmp::Ordering::Greater,
                (RefKind::Tag, RefKind::Tag) => a.name.cmp(&b.name),
            }
        });
    }

    decorations
}

/// Gets the last N commits from HEAD using revwalk.
fn get_recent_commits(repo: &Repository, count: usize) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };

    if revwalk.push_head().is_err() {
        return commits;
    }

    // Collect ref decorations once for all commits
    let ref_decorations = collect_ref_decorations(repo);

    for oid_result in revwalk.take(count) {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        // Get refs pointing to this commit
        let refs = ref_decorations.get(&oid).cloned().unwrap_or_default();

        let author = commit.author();
        commits.push(CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").trim().to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}

/// Gathers repository status including staged, unstaged, and untracked changes.
/// Also returns file changes with their status for rich output.
fn get_repo_status_with_changes(repo: &Repository) -> Result<(RepoStatus, Vec<FileChange>)> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    // Recurse into untracked directories to get individual file paths
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    use std::collections::HashSet;

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked_count = 0;

    // Use HashSet for O(1) deduplication instead of Vec::contains which is O(n)
    let mut dirty_set: HashSet<PathBuf> = HashSet::new();
    let mut untracked_paths: Vec<PathBuf> = Vec::new();
    let mut file_changes: Vec<FileChange> = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry.path().map(PathBuf::from);

        let is_staged =
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_modified() || status.is_wt_deleted();
        let is_untracked = status.is_wt_new();

        // Determine the specific action for staged and unstaged changes
        let staged_action = if status.is_index_new() {
            Some(FileAction::Created)
        } else if status.is_index_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_index_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        let unstaged_action = if status.is_wt_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_wt_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        if is_staged {
            staged += 1;
        }
        if is_unstaged {
            unstaged += 1;
        }
        if is_untracked {
            untracked_count += 1;
            if let Some(ref p) = path {
                untracked_paths.push(p.clone());
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Untracked,
                    action: FileAction::Created,
                    lines_added: 0,
                    lines_removed: 0,
                });
            }
        }

        // Add to dirty set if staged or unstaged (but not untracked)
        if let Some(ref p) = path
            && !is_untracked
        {
            if is_staged && is_unstaged {
                // File is both staged and has additional modifications
                let (lines_added, lines_removed) = get_file_diff_stats(repo, p);
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Both,
                    action: staged_action.unwrap_or(FileAction::Modified),
                    lines_added,
                    lines_removed,
                });
                dirty_set.insert(p.clone());
            } else if is_staged {
                let (lines_added, lines_removed) = get_file_diff_stats(repo, p);
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Staged,
                    action: staged_action.unwrap_or(FileAction::Modified),
                    lines_added,
                    lines_removed,
                });
                dirty_set.insert(p.clone());
            } else if is_unstaged {
                let (lines_added, lines_removed) = get_file_diff_stats(repo, p);
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Modified,
                    action: unstaged_action.unwrap_or(FileAction::Modified),
                    lines_added,
                    lines_removed,
                });
                dirty_set.insert(p.clone());
            }
        }
    }

    // Convert HashSet to Vec for downstream processing
    let dirty_paths: Vec<PathBuf> = dirty_set.into_iter().collect();

    // Get HEAD commit SHA and upstream commit
    let (head_sha, origin_commit) = get_commit_refs(repo);

    // Get repository root for absolute paths
    let repo_root = repo.workdir().map(Path::to_path_buf);

    // Build dirty file details with diffs
    let dirty = build_dirty_files(repo, &dirty_paths, &head_sha, &origin_commit, &repo_root)?;

    // Build untracked file details
    let untracked = build_untracked_files(&untracked_paths, &repo_root);

    let repo_status = RepoStatus {
        is_dirty: staged > 0 || unstaged > 0 || untracked_count > 0,
        staged_count: staged,
        unstaged_count: unstaged,
        untracked_count,
        dirty,
        untracked,
        is_behind: None, // Populated by detect_git when deep=true
    };

    Ok((repo_status, file_changes))
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
        let absolute_filepath =
            repo_root.as_ref().map(|root| root.join(filepath)).unwrap_or_else(|| filepath.clone());

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
/// Returns `(lines_added, lines_removed)` for a single file by combining staged and unstaged diffs.
fn get_file_diff_stats(repo: &Repository, filepath: &Path) -> (usize, usize) {
    let mut added: usize = 0;
    let mut removed: usize = 0;

    // Staged changes (HEAD to index)
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(filepath);
        if let Ok(diff) = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))
            && let Ok(stats) = diff.stats()
        {
            added += stats.insertions();
            removed += stats.deletions();
        }
    }

    // Unstaged changes (index to workdir)
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(filepath);
    if let Ok(diff) = repo.diff_index_to_workdir(None, Some(&mut opts))
        && let Ok(stats) = diff.stats()
    {
        added += stats.insertions();
        removed += stats.deletions();
    }

    (added, removed)
}

fn get_file_diff(repo: &Repository, filepath: &Path) -> Result<String> {
    let mut diff_output = String::new();

    // Get diff for staged changes (HEAD to index)
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(filepath);

        if let Ok(staged_diff) =
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))
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

/// Gets git configuration (user info, GPG, signing).
fn get_git_config(repo: &Repository) -> GitConfig {
    let mut config = match repo.config() {
        Ok(c) => c,
        Err(_) => return GitConfig::default(),
    };

    // On macOS, the Developer Tools system gitconfig lives outside libgit2's
    // default search paths. Include it so we pick up credential.helper, etc.
    #[cfg(target_os = "macos")]
    {
        let macos_system = std::path::Path::new(
            "/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig",
        );
        if macos_system.exists() {
            let _ = config.add_file(macos_system, git2::ConfigLevel::ProgramData, false);
        }
    }

    GitConfig {
        user_name: config.get_string("user.name").ok(),
        user_email: config.get_string("user.email").ok(),
        gpg_use_agent: config.get_bool("gpg.use-agent").ok(),
        gpg_program: config.get_string("gpg.program").ok(),
        credential_helper: config.get_string("credential.helper").ok(),
        signing_key: config.get_string("user.signingkey").ok(),
        commit_sign: config.get_bool("commit.gpgsign").ok(),
        tag_sign: config.get_bool("tag.gpgsign").ok(),
        pager: config.get_string("core.pager").ok(),
        delta_syntax_theme: config.get_string("delta.syntax-theme").ok(),
        delta_light: config.get_bool("delta.light").ok(),
        delta_side_by_side: config.get_bool("delta.side-by-side").ok(),
    }
}

/// Gets all local branches with commit hashes and ahead/behind counts.
///
/// For each branch, resolves the tip commit's short hash and computes
/// ahead/behind relative to the current branch's HEAD. The current branch
/// itself gets ahead=0, behind=0.
fn get_local_branches(repo: &Repository, current_branch: Option<&str>) -> Vec<LocalBranchInfo> {
    let mut branches = Vec::new();

    // Resolve HEAD commit OID for ahead/behind calculations
    let head_oid = repo.head().ok().and_then(|h| h.peel_to_commit().ok()).map(|c| c.id());

    if let Ok(branch_iter) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in branch_iter {
            if let Ok((branch, _)) = branch_result
                && let Ok(Some(name)) = branch.name()
            {
                let is_current = current_branch.is_some_and(|cb| cb == name);

                // Get short hash from branch tip commit
                let short_hash = branch
                    .get()
                    .peel_to_commit()
                    .ok()
                    .map(|c| {
                        let id = c.id().to_string();
                        id[..8.min(id.len())].to_string()
                    })
                    .unwrap_or_default();

                // Compute ahead/behind relative to HEAD
                let (ahead, behind) = if is_current {
                    (0, 0)
                } else if let Some(head_id) = head_oid {
                    branch
                        .get()
                        .peel_to_commit()
                        .ok()
                        .and_then(|c| repo.graph_ahead_behind(c.id(), head_id).ok())
                        .unwrap_or((0, 0))
                } else {
                    (0, 0)
                };

                branches.push(LocalBranchInfo {
                    name: name.to_string(),
                    short_hash,
                    ahead,
                    behind,
                });
            }
        }
    }

    branches
}

/// Gets tracking status (ahead/behind) for each remote.
fn get_tracking_status(
    repo: &Repository,
    current_branch: Option<&str>,
) -> Vec<RemoteTrackingStatus> {
    let mut tracking = Vec::new();

    let Some(branch_name) = current_branch else {
        return tracking;
    };

    let Ok(local_branch) = repo.find_branch(branch_name, git2::BranchType::Local) else {
        return tracking;
    };

    let Ok(local_commit) = local_branch.get().peel_to_commit() else {
        return tracking;
    };

    // Check each remote for a tracking branch
    if let Ok(remotes) = repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            // Try to find the remote tracking branch (e.g., origin/main)
            let remote_branch_name = format!("{}/{}", remote_name, branch_name);
            if let Ok(remote_ref) =
                repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))
                && let Ok(remote_commit) = remote_ref.peel_to_commit()
                && let Ok((ahead, behind)) =
                    repo.graph_ahead_behind(local_commit.id(), remote_commit.id())
            {
                tracking.push(RemoteTrackingStatus {
                    remote: remote_name.to_string(),
                    ahead,
                    behind,
                });
            }
        }
    }

    tracking
}

/// Retrieves all configured remotes with their URLs and hosting providers.
///
/// When `include_remote_details` is true, also includes locally known
/// remote-tracking branches and the resolved default branch.
fn get_remotes(repo: &Repository, include_remote_details: bool) -> Vec<RemoteInfo> {
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
                            .map(|u| GitHostingProvider::from_url(u))
                            .unwrap_or(GitHostingProvider::Unknown);

                        let (branches, default_branch) = if include_remote_details {
                            (get_remote_branches(repo, name), get_remote_default_branch(repo, name))
                        } else {
                            (None, None)
                        };

                        RemoteInfo {
                            name: name.to_string(),
                            url,
                            provider,
                            branches,
                            default_branch,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Refresh local remote-tracking refs using the user's configured `git` binary.
///
/// This uses `git fetch --quiet --prune <remote>` per remote and disables
/// terminal prompts so CLI use does not block on credential input.
fn refresh_remote_tracking_refs(repo: &Repository) {
    let Some(repo_root) = repo.workdir() else {
        return;
    };

    let Ok(remotes) = repo.remotes() else {
        return;
    };

    for remote_name in remotes.iter().flatten() {
        let _ = Command::new("git")
            .current_dir(repo_root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .args([
                "fetch",
                "--quiet",
                "--prune",
                remote_name,
            ])
            .status();
    }
}

/// Derive the user-facing behind status from per-remote tracking counts.
fn summarize_behind_status(tracking: &[RemoteTrackingStatus]) -> Option<BehindStatus> {
    if tracking.is_empty() {
        return None;
    }

    let mut behind_remotes: Vec<String> = tracking
        .iter()
        .filter(|status| status.behind > 0)
        .map(|status| status.remote.clone())
        .collect();
    behind_remotes.sort();
    behind_remotes.dedup();

    Some(if behind_remotes.is_empty() {
        BehindStatus::NotBehind
    } else {
        BehindStatus::Behind(behind_remotes)
    })
}

/// Populate commit containment data from locally available remote-tracking refs.
fn populate_recent_commit_remotes(repo: &Repository, commits: &mut [CommitInfo]) {
    let remote_tips = remote_branch_tips(repo);
    if remote_tips.is_empty() {
        return;
    }

    for commit in commits {
        let Ok(commit_oid) = git2::Oid::from_str(&commit.sha) else {
            continue;
        };

        let mut containing_remotes = Vec::new();
        for (remote_name, tip_oid) in &remote_tips {
            let contains = *tip_oid == commit_oid
                || repo.graph_descendant_of(*tip_oid, commit_oid).unwrap_or(false);
            if contains {
                containing_remotes.push(remote_name.clone());
            }
        }

        containing_remotes.sort();
        containing_remotes.dedup();
        if !containing_remotes.is_empty() {
            commit.remotes = Some(containing_remotes);
        }
    }
}

/// Collect the tip OIDs for all remote-tracking branches keyed by remote name.
fn remote_branch_tips(repo: &Repository) -> Vec<(String, git2::Oid)> {
    let Ok(refs) = repo.references_glob("refs/remotes/*") else {
        return Vec::new();
    };

    refs.flatten()
        .filter_map(|reference| {
            let name = reference.name()?;
            let branch = name.strip_prefix("refs/remotes/")?;
            if branch.ends_with("/HEAD") {
                return None;
            }

            let (remote_name, _) = branch.split_once('/')?;
            let target = reference.peel_to_commit().ok()?;
            Some((remote_name.to_string(), target.id()))
        })
        .collect()
}

/// Resolves the default branch for a remote from `refs/remotes/{name}/HEAD`.
///
/// Returns the branch name (e.g., "main") if the symbolic ref exists and can be resolved.
fn get_remote_default_branch(repo: &Repository, remote_name: &str) -> Option<String> {
    let ref_name = format!("refs/remotes/{}/HEAD", remote_name);
    let reference = repo.find_reference(&ref_name).ok()?;
    let target = reference.symbolic_target()?;
    let prefix = format!("refs/remotes/{}/", remote_name);
    target.strip_prefix(&prefix).map(String::from)
}

/// Gets branch names for a remote from local tracking refs (`refs/remotes/<name>/*`).
///
/// Reads locally cached remote branch info (updated on fetch/pull).
/// No network access required.
fn get_remote_branches(repo: &Repository, remote_name: &str) -> Option<Vec<String>> {
    let pattern = format!("refs/remotes/{}/*", remote_name);
    let refs = repo.references_glob(&pattern).ok()?;
    let prefix = format!("refs/remotes/{}/", remote_name);

    let mut branches: Vec<String> = refs
        .flatten()
        .filter_map(|r| {
            let name = r.name()?;
            let branch = name.strip_prefix(&prefix)?;
            if branch == "HEAD" {
                None
            } else {
                Some(branch.to_string())
            }
        })
        .collect();

    branches.sort();

    if branches.is_empty() {
        None
    } else {
        Some(branches)
    }
}

/// Retrieves all linked worktrees for the repository.
///
/// Returns a HashMap keyed by branch name. Anonymous worktrees (without a name)
/// are filtered out. For each worktree, opens it as a Repository to access
/// HEAD commit, dirty status, and ahead/behind counts relative to the base
/// repository's default branch.
fn get_worktrees(repo: &Repository) -> HashMap<String, WorktreeInfo> {
    let mut worktrees = HashMap::new();

    let worktree_names = match repo.worktrees() {
        Ok(names) => names,
        Err(_) => return worktrees,
    };

    // Resolve the base branch name and its commit OID for ahead/behind calculations.
    // Try the base repo's HEAD first; fall back to "main" then "master".
    let (base_branch, base_oid) = resolve_base_branch(repo);

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

        // Get HEAD commit SHA and OID
        let head_commit = worktree_repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let sha = head_commit.as_ref().map(|c| c.id().to_string()).unwrap_or_default();

        // Compute ahead/behind relative to base branch
        let (ahead, behind) = base_oid
            .zip(head_commit.as_ref())
            .and_then(|(base, wt_commit)| repo.graph_ahead_behind(wt_commit.id(), base).ok())
            .unwrap_or((0, 0));

        // Check for merge conflicts by performing an in-memory merge
        let has_conflicts = base_oid
            .zip(head_commit.as_ref())
            .and_then(|(base_id, wt_commit)| {
                let base_commit = repo.find_commit(base_id).ok()?;
                let index = repo.merge_commits(wt_commit, &base_commit, None).ok()?;
                Some(index.has_conflicts())
            })
            .unwrap_or(false);

        // Check if worktree is dirty
        let dirty =
            get_repo_status_with_changes(&worktree_repo).map(|(s, _)| s.is_dirty).unwrap_or(false);

        worktrees.insert(
            branch.clone(),
            WorktreeInfo {
                branch,
                filepath: worktree_path.to_path_buf(),
                sha,
                dirty,
                ahead,
                behind,
                base_branch: base_branch.clone(),
                has_conflicts,
            },
        );
    }

    worktrees
}

/// Resolves the base branch name and its commit OID for ahead/behind calculations.
///
/// When the repo is a worktree, finds the base repo's current branch. Otherwise
/// uses the current HEAD branch. Falls back to "main" or "master" if HEAD is
/// detached or unavailable.
fn resolve_base_branch(repo: &Repository) -> (String, Option<git2::Oid>) {
    // If we're in a worktree, open the base repo to get its HEAD branch
    let base_repo = if repo.is_worktree() {
        repo.commondir()
            .parent()
            .and_then(|p| Repository::open(p).ok())
    } else {
        None
    };
    let effective_repo = base_repo.as_ref().unwrap_or(repo);

    // Try the base repo's current HEAD branch
    if let Ok(head) = effective_repo.head() {
        if let Some(name) = head.shorthand() {
            let oid = head.peel_to_commit().ok().map(|c| c.id());
            return (name.to_string(), oid);
        }
    }

    // Fallback: try "main", then "master"
    for candidate in &["main", "master"] {
        let refname = format!("refs/heads/{candidate}");
        if let Ok(reference) = repo.find_reference(&refname) {
            let oid = reference.peel_to_commit().ok().map(|c| c.id());
            return (candidate.to_string(), oid);
        }
    }

    ("main".to_string(), None)
}

/// Kind of change a file underwent in a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaKind {
    /// File was added.
    Added,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
    /// File was copied.
    Copied,
}

impl std::fmt::Display for DeltaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
            Self::Copied => write!(f, "copied"),
        }
    }
}

impl DeltaKind {
    /// Convert a git2 Delta status to a DeltaKind.
    fn from_delta(delta: git2::Delta) -> Self {
        match delta {
            git2::Delta::Added => Self::Added,
            git2::Delta::Deleted => Self::Deleted,
            git2::Delta::Renamed => Self::Renamed,
            git2::Delta::Copied => Self::Copied,
            _ => Self::Modified,
        }
    }
}

/// Look up a single commit by full or abbreviated SHA.
///
/// Uses `repo.revparse_single()` to resolve abbreviated or full SHA strings,
/// then peels to a commit and builds a `CommitInfo` with ref decorations.
///
/// Returns `None` if the SHA doesn't resolve to a valid commit.
pub fn get_commit_by_sha(repo: &Repository, sha_prefix: &str) -> Option<CommitInfo> {
    let obj = repo.revparse_single(sha_prefix).ok()?;
    let commit = obj.peel_to_commit().ok()?;

    let ref_decorations = collect_ref_decorations(repo);
    let oid = commit.id();
    let refs = ref_decorations.get(&oid).cloned().unwrap_or_default();

    let author = commit.author();
    Some(CommitInfo {
        sha: oid.to_string(),
        message: commit.message().unwrap_or("").trim().to_string(),
        author: author.name().unwrap_or("Unknown").to_string(),
        timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
        remotes: None,
        refs,
    })
}

/// Get the list of files changed by a specific commit.
///
/// Computes a diff between the commit's tree and its first parent's tree.
/// For the initial commit (no parent), diffs against an empty tree.
///
/// Returns a list of `(relative_path, DeltaKind)` pairs.
pub fn get_commit_files(repo: &Repository, full_sha: &str) -> Vec<(PathBuf, DeltaKind)> {
    let Ok(oid) = git2::Oid::from_str(full_sha) else {
        return Vec::new();
    };
    let Ok(commit) = repo.find_commit(oid) else {
        return Vec::new();
    };
    let Ok(tree) = commit.tree() else {
        return Vec::new();
    };

    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None).ok();

    let Some(diff) = diff else {
        return Vec::new();
    };

    diff.deltas()
        .filter_map(|delta| {
            let path = delta.new_file().path().unwrap_or(Path::new(""));
            if path.as_os_str().is_empty() {
                None
            } else {
                Some((path.to_path_buf(), DeltaKind::from_delta(delta.status())))
            }
        })
        .collect()
}

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD, computes diffs for each commit,
/// and includes commits where at least one changed file starts with `path_prefix`.
///
/// Ref decorations are collected once and reused for all matching commits.
pub fn get_commits_for_path(repo: &Repository, path_prefix: &str, count: usize) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
    if revwalk.push_head().is_err() {
        return commits;
    }

    let ref_decorations = collect_ref_decorations(repo);

    for oid_result in revwalk {
        if commits.len() >= count {
            break;
        }
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        let Ok(tree) = commit.tree() else {
            continue;
        };

        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) else {
            continue;
        };

        let touches_path = diff.deltas().any(|delta| {
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            new_path.starts_with(path_prefix) || old_path.starts_with(path_prefix)
        });

        if touches_path {
            let refs = ref_decorations.get(&oid).cloned().unwrap_or_default();
            let author = commit.author();
            commits.push(CommitInfo {
                sha: oid.to_string(),
                message: commit.message().unwrap_or("").trim().to_string(),
                author: author.name().unwrap_or("Unknown").to_string(),
                timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
                remotes: None,
                refs,
            });
        }
    }

    commits
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ============================================================================
    // ConventionalCommit parser tests
    // ============================================================================

    #[test]
    fn test_conventional_commit_feat_with_scope() {
        let commit = ConventionalCommit::parse("feat(cli): add new flag");
        assert_eq!(commit.operation, Some("feat".to_string()));
        assert_eq!(commit.scope, Some("cli".to_string()));
        assert_eq!(commit.description, "add new flag");
    }

    #[test]
    fn test_conventional_commit_fix_without_scope() {
        let commit = ConventionalCommit::parse("fix: resolve memory leak");
        assert_eq!(commit.operation, Some("fix".to_string()));
        assert_eq!(commit.scope, None);
        assert_eq!(commit.description, "resolve memory leak");
    }

    #[test]
    fn test_conventional_commit_chore() {
        let commit = ConventionalCommit::parse("chore(deps): update dependencies");
        assert_eq!(commit.operation, Some("chore".to_string()));
        assert_eq!(commit.scope, Some("deps".to_string()));
        assert_eq!(commit.description, "update dependencies");
    }

    #[test]
    fn test_conventional_commit_plain_message() {
        let commit = ConventionalCommit::parse("Regular commit message");
        assert_eq!(commit.operation, None);
        assert_eq!(commit.scope, None);
        assert_eq!(commit.description, "Regular commit message");
    }

    #[test]
    fn test_conventional_commit_multiline() {
        let commit = ConventionalCommit::parse("feat(api): add endpoint\n\nBody text here");
        assert_eq!(commit.operation, Some("feat".to_string()));
        assert_eq!(commit.scope, Some("api".to_string()));
        assert_eq!(commit.description, "add endpoint");
    }

    #[test]
    fn test_conventional_commit_empty_scope() {
        let commit = ConventionalCommit::parse("feat(): description");
        assert_eq!(commit.operation, Some("feat".to_string()));
        assert_eq!(commit.scope, None);
        assert_eq!(commit.description, "description");
    }

    #[test]
    fn test_conventional_commit_breaking_change() {
        // Breaking change indicator (!) is part of the type
        let commit = ConventionalCommit::parse("feat!: breaking change");
        // The '!' is not alphanumeric so parsing stops there
        assert_eq!(commit.operation, None);
        assert_eq!(commit.description, "feat!: breaking change");
    }

    #[test]
    fn test_conventional_commit_hyphenated_type() {
        let commit = ConventionalCommit::parse("bug-fix(core): fix issue");
        assert_eq!(commit.operation, Some("bug-fix".to_string()));
        assert_eq!(commit.scope, Some("core".to_string()));
        assert_eq!(commit.description, "fix issue");
    }

    #[test]
    fn test_conventional_commit_no_colon() {
        let commit = ConventionalCommit::parse("feat something");
        assert_eq!(commit.operation, None);
        assert_eq!(commit.description, "feat something");
    }

    #[test]
    fn test_conventional_commit_empty() {
        let commit = ConventionalCommit::parse("");
        assert_eq!(commit.operation, None);
        assert_eq!(commit.description, "");
    }

    // ============================================================================
    // FileStatus tests
    // ============================================================================

    #[test]
    fn test_file_status_serialization() {
        assert_eq!(serde_json::to_string(&FileStatus::Staged).unwrap(), "\"Staged\"");
        assert_eq!(serde_json::to_string(&FileStatus::Modified).unwrap(), "\"Modified\"");
        assert_eq!(serde_json::to_string(&FileStatus::Both).unwrap(), "\"Both\"");
        assert_eq!(serde_json::to_string(&FileStatus::Untracked).unwrap(), "\"Untracked\"");
    }

    // ============================================================================
    // Repository detection tests
    // ============================================================================

    #[test]
    fn test_non_git_directory_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = detect_git(dir.path(), false, 10).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let result = detect_git(dir.path(), false, 10).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // Use canonicalize to handle /private/var vs /var on macOS
        assert_eq!(info.repo_root.canonicalize().unwrap(), dir.path().canonicalize().unwrap());
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let result = detect_git(dir.path(), false, 10).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        // A standard initialized repo is not a worktree
        assert!(!info.in_worktree);
    }

    #[test]
    fn test_hosting_provider_github() {
        assert_eq!(
            GitHostingProvider::from_url("https://github.com/user/repo"),
            GitHostingProvider::GitHub
        );
        assert_eq!(
            GitHostingProvider::from_url("git@github.com:user/repo.git"),
            GitHostingProvider::GitHub
        );
    }

    #[test]
    fn test_hosting_provider_gitlab() {
        assert_eq!(
            GitHostingProvider::from_url("https://gitlab.com/user/repo"),
            GitHostingProvider::GitLab
        );
    }

    #[test]
    fn test_hosting_provider_bitbucket() {
        assert_eq!(
            GitHostingProvider::from_url("https://bitbucket.org/user/repo"),
            GitHostingProvider::Bitbucket
        );
    }

    #[test]
    fn test_hosting_provider_azure_devops() {
        assert_eq!(
            GitHostingProvider::from_url("https://dev.azure.com/org/project"),
            GitHostingProvider::AzureDevOps
        );
        assert_eq!(
            GitHostingProvider::from_url("https://org.visualstudio.com/project"),
            GitHostingProvider::AzureDevOps
        );
    }

    #[test]
    fn test_hosting_provider_aws_codecommit() {
        assert_eq!(
            GitHostingProvider::from_url(
                "https://git-codecommit.us-east-1.amazonaws.com/v1/repos/repo"
            ),
            GitHostingProvider::AwsCodeCommit
        );
    }

    #[test]
    fn test_hosting_provider_sourcehut() {
        assert_eq!(
            GitHostingProvider::from_url("https://git.sr.ht/~user/repo"),
            GitHostingProvider::SourceHut
        );
    }

    #[test]
    fn test_hosting_provider_gitea_and_forgejo() {
        assert_eq!(
            GitHostingProvider::from_url("https://gitea.example.com/user/repo"),
            GitHostingProvider::Gitea
        );
        assert_eq!(
            GitHostingProvider::from_url("https://forgejo.example.com/user/repo"),
            GitHostingProvider::Forgejo
        );
        assert_eq!(
            GitHostingProvider::from_url("https://codeberg.org/forgejo/forgejo"),
            GitHostingProvider::Forgejo
        );
    }

    #[test]
    fn test_hosting_provider_metadata_helpers() {
        let github = GitHostingProvider::GitHub.metadata();
        assert_eq!(github.display_name, "GitHub");
        assert_eq!(github.symbol, "[gh]");
        assert!(github.host_based_detection_reliable);
        assert_eq!(github.browser_base_url, Some("https://github.com"));

        let self_hosted = GitHostingProvider::SelfHosted.metadata();
        assert!(self_hosted.self_hosted);
        assert!(!self_hosted.host_based_detection_reliable);
        assert_eq!(GitHostingProvider::SelfHosted.symbol(), "[git]");
    }

    #[test]
    fn test_extract_remote_host() {
        assert_eq!(extract_remote_host("git@github.com:rust-lang/cargo.git"), Some("github.com"));
        assert_eq!(
            extract_remote_host("ssh://git@gitlab.example.com:2222/team/project"),
            Some("gitlab.example.com")
        );
        assert_eq!(
            extract_remote_host("https://bitbucket.org/workspace/repo"),
            Some("bitbucket.org")
        );
        assert_eq!(extract_remote_host("not-a-url"), None);
    }

    #[test]
    fn test_hosting_provider_self_hosted() {
        assert_eq!(
            GitHostingProvider::from_url("https://git.company.com/repo"),
            GitHostingProvider::SelfHosted
        );
    }

    #[test]
    fn test_hosting_provider_unknown() {
        assert_eq!(GitHostingProvider::from_url("unknown"), GitHostingProvider::Unknown);
    }

    #[test]
    fn test_repo_status_clean() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }

        // Modify the file (unstaged change)
        std::fs::write(&file_path, "modified content").unwrap();

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }

        // Modify and stage the file
        std::fs::write(&file_path, "staged content").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("test.txt")).unwrap();
            index.write().unwrap();
        }

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }

        // Create and stage a new file
        let file_path = dir.path().join("new_file.txt");
        std::fs::write(&file_path, "new file content").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("new_file.txt")).unwrap();
            index.write().unwrap();
        }

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }

        // Modify the nested file
        std::fs::write(&nested_file, "modified").unwrap();

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
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
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }

        // Modify the file
        std::fs::write(&file_path, "line1\nmodified\nline3\n").unwrap();

        let (status, _) = get_repo_status_with_changes(&repo).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let result = detect_git(dir.path(), false, 10).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Add a remote (even though it won't be reachable, we just need to test the struct)
        repo.remote("origin", "https://github.com/example/repo.git").unwrap();

        // Without deep mode, branches should be None
        let result = detect_git(dir.path(), false, 10).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Without deep mode, is_behind should be None
        let result = detect_git(dir.path(), false, 10).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Without deep mode, commit remotes should be None
        let result = detect_git(dir.path(), false, 10).unwrap();
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
                index.add_path(Path::new(&format!("file{}.txt", i))).unwrap();
                index.write().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            let message = format!("Commit {}", i);

            let commit_id = if let Some(parent) = parent_commit {
                let parent_commit_obj = repo.find_commit(parent).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit_obj])
                    .unwrap()
            } else {
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[]).unwrap()
            };
            parent_commit = Some(commit_id);
        }

        let result = detect_git(dir.path(), false, 10).unwrap();
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
            .commit(Some("HEAD"), &sig1, &sig1, "  First commit with whitespace  \n", &tree, &[])
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

        let result = detect_git(dir.path(), false, 10).unwrap();
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
        let result = detect_git(dir.path(), false, 10).unwrap();
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
        let initial_commit_id =
            repo.commit(Some("HEAD"), &sig, &sig, "Initial on main", &tree, &[]).unwrap();

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
            .commit(Some("HEAD"), &sig, &sig, "Second on main", &tree, &[&initial_commit])
            .unwrap();

        // Create a branch from initial commit (not from HEAD)
        repo.branch("feature", &repo.find_commit(initial_commit_id).unwrap(), false).unwrap();

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
            .commit(Some("HEAD"), &sig, &sig, "Commit on feature", &tree, &[&initial_commit])
            .unwrap();

        // Now detect from feature branch
        let result = detect_git(dir.path(), false, 10).unwrap();
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
        repo.set_head(&format!("refs/heads/{}", initial_branch_name)).unwrap();
        // Also checkout to update the working directory
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).unwrap();

        let result_main = detect_git(dir.path(), false, 10).unwrap().unwrap();
        let main_messages: Vec<&str> =
            result_main.recent.iter().map(|c| c.message.as_str()).collect();
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
            ahead: 3,
            behind: 1,
            base_branch: "main".to_string(),
            has_conflicts: false,
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
            provider: GitHostingProvider::GitHub,
            branches: Some(vec![
                "main".to_string(),
                "develop".to_string(),
            ]),
            default_branch: Some("main".to_string()),
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
        assert_eq!(parsed_with["default_branch"], "main");

        // Test with branches as None (should be excluded due to skip_serializing_if)
        let remote_without_branches = RemoteInfo {
            name: "upstream".to_string(),
            url: Some("https://github.com/other/repo".to_string()),
            provider: GitHostingProvider::GitHub,
            branches: None,
            default_branch: None,
        };

        let json_without = serde_json::to_string(&remote_without_branches).unwrap();
        let parsed_without: serde_json::Value = serde_json::from_str(&json_without).unwrap();

        assert_eq!(parsed_without["name"], "upstream");
        // branches field should be absent (not null)
        assert!(parsed_without.get("branches").is_none());
        // default_branch field should also be absent
        assert!(parsed_without.get("default_branch").is_none());
    }

    #[test]
    fn test_local_branch_info_serialization() {
        let branch = LocalBranchInfo {
            name: "feature/xyz".to_string(),
            short_hash: "a1b2c3d4".to_string(),
            ahead: 3,
            behind: 1,
        };

        let json = serde_json::to_string(&branch).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["name"], "feature/xyz");
        assert_eq!(parsed["short_hash"], "a1b2c3d4");
        assert_eq!(parsed["ahead"], 3);
        assert_eq!(parsed["behind"], 1);
    }

    #[test]
    fn test_behind_status_serialization() {
        // NotBehind should serialize as false
        let not_behind = BehindStatus::NotBehind;
        let json = serde_json::to_string(&not_behind).unwrap();
        assert_eq!(json, "false");

        // Behind should serialize as array
        let behind = BehindStatus::Behind(vec![
            "origin".to_string(),
            "upstream".to_string(),
        ]);
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
            BehindStatus::Behind(vec![
                "origin".to_string(),
                "upstream".to_string()
            ])
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
            refs: vec![],
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
            refs: vec![],
        };

        let json_without = serde_json::to_string(&commit_without_remotes).unwrap();
        let parsed_without: serde_json::Value = serde_json::from_str(&json_without).unwrap();

        assert_eq!(parsed_without["sha"], "def789abc123456");
        assert_eq!(parsed_without["message"], "Fix bug Y");
        // remotes field should be absent (not null)
        assert!(parsed_without.get("remotes").is_none());

        // Verify roundtrip for both cases
        let deserialized_with: CommitInfo = serde_json::from_str(&json_with).unwrap();
        assert_eq!(deserialized_with.remotes, Some(vec!["origin".to_string()]));

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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Add a remote
        repo.remote("origin", "https://github.com/example/repo.git").unwrap();

        // Call detect_git with deep=false
        let result = detect_git(dir.path(), false, 10).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();

        // Verify all network-dependent fields are NOT populated (None)

        // 1. RemoteInfo.branches should be None
        assert!(!info.remotes.is_empty());
        for remote in &info.remotes {
            assert!(remote.branches.is_none(), "branches should be None when deep=false");
        }

        // 2. RepoStatus.is_behind should be None
        assert!(info.status.is_behind.is_none(), "is_behind should be None when deep=false");

        // 3. CommitInfo.remotes should be None for all commits
        assert!(!info.recent.is_empty());
        for commit in &info.recent {
            assert!(commit.remotes.is_none(), "commit.remotes should be None when deep=false");
        }

        // Verify basic fields are still populated correctly
        assert!(info.current_branch.is_some());
        assert!(!info.in_worktree);
        assert!(info.worktrees.is_empty());
        assert_eq!(info.remotes.len(), 1);
        assert_eq!(info.remotes[0].name, "origin");
        assert_eq!(info.remotes[0].provider, GitHostingProvider::GitHub);
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
                index.add_path(Path::new(&format!("file{}.txt", i))).unwrap();
                index.write().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            let message = format!("Commit {}", i);

            let commit_id = if let Some(parent) = parent_commit {
                let parent_commit_obj = repo.find_commit(parent).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit_obj])
                    .unwrap()
            } else {
                repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[]).unwrap()
            };
            parent_commit = Some(commit_id);
        }

        let result = detect_git(dir.path(), false, 10).unwrap();
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

    // ============================================================================
    // RefDecoration tests
    // ============================================================================

    #[test]
    fn test_ref_decoration_serialization() {
        let local_head = RefDecoration {
            name: "main".to_string(),
            kind: RefKind::LocalBranch,
            is_head: true,
        };

        let json = serde_json::to_string(&local_head).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["name"], "main");
        assert_eq!(parsed["kind"], "LocalBranch");
        assert_eq!(parsed["is_head"], true);

        // Test without is_head (should be omitted due to skip_serializing_if)
        let remote_branch = RefDecoration {
            name: "origin/main".to_string(),
            kind: RefKind::RemoteBranch,
            is_head: false,
        };

        let json = serde_json::to_string(&remote_branch).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["name"], "origin/main");
        assert_eq!(parsed["kind"], "RemoteBranch");
        // is_head should be absent when false
        assert!(parsed.get("is_head").is_none());
    }

    #[test]
    fn test_ref_kind_variants() {
        assert_eq!(serde_json::to_string(&RefKind::LocalBranch).unwrap(), "\"LocalBranch\"");
        assert_eq!(serde_json::to_string(&RefKind::RemoteBranch).unwrap(), "\"RemoteBranch\"");
        assert_eq!(serde_json::to_string(&RefKind::Tag).unwrap(), "\"Tag\"");
    }

    #[test]
    fn test_commit_info_with_refs_serialization() {
        use chrono::TimeZone;

        let commit_with_refs = CommitInfo {
            sha: "abc123".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 0).unwrap(),
            remotes: None,
            refs: vec![
                RefDecoration {
                    name: "main".to_string(),
                    kind: RefKind::LocalBranch,
                    is_head: true,
                },
                RefDecoration {
                    name: "origin/main".to_string(),
                    kind: RefKind::RemoteBranch,
                    is_head: false,
                },
            ],
        };

        let json = serde_json::to_string(&commit_with_refs).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["refs"].is_array());
        let refs = parsed["refs"].as_array().unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0]["name"], "main");
        assert_eq!(refs[1]["name"], "origin/main");

        // Test with empty refs (should be omitted)
        let commit_no_refs = CommitInfo {
            sha: "def456".to_string(),
            message: "Another commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 1, 14, 9, 0, 0).unwrap(),
            remotes: None,
            refs: vec![],
        };

        let json = serde_json::to_string(&commit_no_refs).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // refs field should be absent when empty
        assert!(parsed.get("refs").is_none());
    }

    #[test]
    fn test_commit_has_refs_populated() {
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
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let result = detect_git(dir.path(), false, 10).unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(!info.recent.is_empty());

        // HEAD commit should have ref decorations
        let head_commit = &info.recent[0];
        assert!(!head_commit.refs.is_empty());

        // Should have at least the local branch
        let has_local_branch = head_commit.refs.iter().any(|r| r.kind == RefKind::LocalBranch);
        assert!(has_local_branch, "HEAD commit should have a local branch ref");

        // The HEAD branch should be marked
        let has_head_marker = head_commit.refs.iter().any(|r| r.is_head);
        assert!(has_head_marker, "Current branch should be marked as HEAD");
    }

    #[test]
    fn test_refs_sorted_correctly() {
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
        let commit_id =
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Create additional branches pointing to the same commit
        let commit = repo.find_commit(commit_id).unwrap();
        repo.branch("feature", &commit, false).unwrap();
        repo.branch("develop", &commit, false).unwrap();

        // Create a tag pointing to the same commit
        repo.tag_lightweight("v1.0.0", commit.as_object(), false).unwrap();

        let result = detect_git(dir.path(), false, 10).unwrap();
        let info = result.unwrap();
        let head_commit = &info.recent[0];

        // Should have multiple refs
        assert!(head_commit.refs.len() >= 3);

        // HEAD branch should come first
        assert!(head_commit.refs[0].is_head, "HEAD branch should be first");
        assert_eq!(head_commit.refs[0].kind, RefKind::LocalBranch);

        // Other local branches should come next, then tags
        let first_tag_idx = head_commit.refs.iter().position(|r| r.kind == RefKind::Tag);
        let last_local_idx = head_commit.refs.iter().rposition(|r| r.kind == RefKind::LocalBranch);

        if let (Some(tag_idx), Some(local_idx)) = (first_tag_idx, last_local_idx) {
            assert!(local_idx < tag_idx, "Local branches should come before tags");
        }
    }

    // ============================================================================
    // parse_org_repo tests
    // ============================================================================

    #[test]
    fn test_parse_org_repo_ssh() {
        let (org, repo) = parse_org_repo("git@github.com:rust-lang/cargo.git");
        assert_eq!(org.as_deref(), Some("rust-lang"));
        assert_eq!(repo.as_deref(), Some("cargo"));
    }

    #[test]
    fn test_parse_org_repo_https() {
        let (org, repo) = parse_org_repo("https://github.com/rust-lang/cargo.git");
        assert_eq!(org.as_deref(), Some("rust-lang"));
        assert_eq!(repo.as_deref(), Some("cargo"));
    }

    #[test]
    fn test_parse_org_repo_https_no_git_suffix() {
        let (org, repo) = parse_org_repo("https://gitlab.com/acme/project");
        assert_eq!(org.as_deref(), Some("acme"));
        assert_eq!(repo.as_deref(), Some("project"));
    }

    #[test]
    fn test_parse_org_repo_ssh_no_git_suffix() {
        let (org, repo) = parse_org_repo("git@bitbucket.org:team/repo");
        assert_eq!(org.as_deref(), Some("team"));
        assert_eq!(repo.as_deref(), Some("repo"));
    }

    #[test]
    fn test_parse_org_repo_invalid_url() {
        let (org, repo) = parse_org_repo("not-a-url");
        assert_eq!(org, None);
        assert_eq!(repo, None);
    }

    #[test]
    fn test_parse_org_repo_no_repo_part() {
        let (org, repo) = parse_org_repo("https://github.com/owner-only");
        assert_eq!(org, None);
        assert_eq!(repo, None);
    }

    // ============================================================================
    // preferred_remote tests
    // ============================================================================

    fn make_remote(name: &str) -> RemoteInfo {
        RemoteInfo {
            name: name.to_string(),
            url: Some(format!("https://github.com/{name}/repo.git")),
            provider: GitHostingProvider::GitHub,
            branches: None,
            default_branch: None,
        }
    }

    #[test]
    fn test_preferred_remote_origin_preferred() {
        let remotes = vec![
            make_remote("upstream"),
            make_remote("origin"),
            make_remote("fork"),
        ];
        let preferred = preferred_remote(&remotes).unwrap();
        assert_eq!(preferred.name, "origin");
    }

    #[test]
    fn test_preferred_remote_first_alpha_excluding_upstream() {
        let remotes = vec![
            make_remote("upstream"),
            make_remote("fork"),
            make_remote("backup"),
        ];
        let preferred = preferred_remote(&remotes).unwrap();
        assert_eq!(preferred.name, "backup");
    }

    #[test]
    fn test_preferred_remote_upstream_only() {
        let remotes = vec![make_remote("upstream")];
        let preferred = preferred_remote(&remotes).unwrap();
        assert_eq!(preferred.name, "upstream");
    }

    #[test]
    fn test_preferred_remote_empty() {
        let remotes: Vec<RemoteInfo> = vec![];
        assert!(preferred_remote(&remotes).is_none());
    }

    #[test]
    fn test_preferred_remote_single_non_origin() {
        let remotes = vec![make_remote("my-fork")];
        let preferred = preferred_remote(&remotes).unwrap();
        assert_eq!(preferred.name, "my-fork");
    }
}

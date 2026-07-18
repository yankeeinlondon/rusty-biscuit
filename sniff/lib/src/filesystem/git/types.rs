use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{debug, instrument, warn};

use crate::request::GitRequest;
use crate::{Result, SniffError};

use super::recent_commits::CommitDescSet;
use super::worktree::WorktreeEntry;

const AGGREGATE_COMMIT_WINDOW_DAYS: i64 = 3;

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

/// Local branch projection for repo-level branch listing.
///
/// This is distinct from [`LocalBranchInfo`], whose ahead/behind counts are
/// relative to the current `HEAD`. `BranchInfo` reports each branch against its
/// configured upstream, using only locally known refs unless the caller
/// explicitly refreshes remote-tracking refs before detection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchInfo {
    /// Branch name (e.g., "main", "feature/xyz").
    pub name: String,
    /// Whether this branch is currently checked out.
    pub current: bool,
    /// Full commit hash for the branch tip.
    pub sha: String,
    /// Whether a locally known remote-tracking ref points at this branch tip.
    pub remote_represented: bool,
    /// Configured upstream branch, such as "origin/main".
    ///
    /// Serializes as `null` when the branch has no configured upstream, so
    /// consumers can distinguish "no tracking configured" from a present value.
    pub upstream: Option<String>,
    /// Number of commits this branch is ahead of its configured upstream.
    ///
    /// `None` when no upstream is configured or its tip is not locally known —
    /// distinguishing "no tracking data" from an even `Some(0)`.
    pub ahead: Option<usize>,
    /// Number of commits this branch is behind its configured upstream.
    ///
    /// `None` under the same conditions as [`ahead`](Self::ahead).
    pub behind: Option<usize>,
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
/// Discovery, trust validation, identity queries (root, branch, HEAD, worktree
/// state), working-tree status, history, refs, remotes, config, and worktree
/// queries are all backed by the pure-Rust `gix` handle.
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
    /// Pure-Rust handle backing discovery and all git queries.
    ///
    /// Wrapped in [`RefCell`] so object-cache sizing (a mutating operation on
    /// the gix handle) can be deferred to the first object-intensive call.
    gix: RefCell<gix::Repository>,
    repo_root: PathBuf,
    /// Path to this repository's git directory (the `.git` dir or, for a linked
    /// worktree, its per-worktree git directory).
    git_dir: PathBuf,
    /// Path to the common git directory shared across all worktrees.
    common_dir: PathBuf,
    /// Cached ref decorations to avoid recomputing on every commit query.
    ref_decorations: RefCell<Option<HashMap<gix::ObjectId, Vec<RefDecoration>>>>,
    /// Cached git config so disk reads happen at most once per instance.
    config_cache: OnceLock<GitConfig>,
    /// Whether the object cache has already been sized for this handle.
    cache_configured: Cell<bool>,
}

impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepo")
            .field("repo_root", &self.repo_root)
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    /// Sizes the object cache once, before the first object-intensive operation.
    fn ensure_cache(&self) {
        if !self.cache_configured.get() {
            super::open::configure_cache(&mut self.gix.borrow_mut());
            self.cache_configured.set(true);
        }
    }

    /// Runs `f` against this instance's gix handle with the object cache sized.
    ///
    /// The seam that lets sibling modules add `*_with_repo` query variants which
    /// reuse an already-discovered handle instead of paying a second
    /// `trusted_discover`. `f` must not re-enter a `GitRepo` method: the handle
    /// is held through a [`RefCell`] borrow for the duration of the call.
    pub(crate) fn with_cached_gix<T>(&self, f: impl FnOnce(&gix::Repository) -> T) -> T {
        self.ensure_cache();
        f(&self.gix.borrow())
    }

    /// Returns cached ref decorations, computing them once on first access.
    ///
    /// Errors are suppressed: an unreadable ref store yields an empty map.
    pub(crate) fn ref_decorations(
        &self,
    ) -> std::cell::Ref<'_, HashMap<gix::ObjectId, Vec<RefDecoration>>> {
        // If not yet computed, compute and cache. Collecting decorations peels
        // refs (object decoding), so size the object cache first — this is the
        // lazy alternative to sizing it unconditionally at open time.
        if self.ref_decorations.borrow().is_none() {
            self.ensure_cache();
            let decorations = super::discovery::collect_ref_decorations(&self.gix.borrow());
            *self.ref_decorations.borrow_mut() = Some(decorations);
        }
        std::cell::Ref::map(self.ref_decorations.borrow(), |opt| opt.as_ref().unwrap())
    }

}

impl GitRepo {
    /// Discover a repository containing `path`.
    ///
    /// Discovery walks parent directories and rejects untrusted repositories.
    ///
    /// ## Returns
    ///
    /// `Ok(None)` when `path` is not inside a git repository.
    ///
    /// ## Errors
    ///
    /// Trust/ownership, permission, I/O, and corruption failures surface as
    /// [`SniffError::Git`] rather than being reported as repository absence.
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn discover(path: &Path) -> Result<Option<Self>> {
        let Some(gix) = super::open::trusted_discover(path)? else {
            debug!("not a git repository");
            return Ok(None);
        };
        let repo_root = gix
            .workdir()
            .ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?
            .to_path_buf();
        let git_dir = gix.git_dir().to_path_buf();
        let common_dir = gix.common_dir().to_path_buf();
        Ok(Some(Self {
            gix: RefCell::new(gix),
            repo_root,
            git_dir,
            common_dir,
            ref_decorations: RefCell::new(None),
            config_cache: OnceLock::new(),
            cache_configured: Cell::new(false),
        }))
    }

    /// Absolute path to the repository working directory.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Path to this repository's git directory (the `.git` dir or, for a linked
    /// worktree, its per-worktree git directory).
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Path to the common git directory shared across all worktrees.
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// HEAD commit id as a full hex SHA, or `None` for an unborn HEAD.
    pub fn head_id(&self) -> Option<String> {
        self.gix.borrow().head_id().ok().map(|id| id.to_string())
    }

    /// Whether HEAD is detached (points directly at a commit).
    pub fn is_detached_head(&self) -> bool {
        self.gix
            .borrow()
            .head()
            .map(|h| h.is_detached())
            .unwrap_or(false)
    }

    /// Current branch name (`None` for detached or unborn HEAD).
    ///
    /// Errors are suppressed: an unreadable or malformed HEAD reports `None`.
    /// For error propagation, use [`Self::try_current_branch`].
    pub fn current_branch(&self) -> Option<String> {
        self.try_current_branch().ok().flatten()
    }

    /// Fallible current branch name.
    ///
    /// `Ok(None)` means HEAD is detached or unborn — a branch genuinely is not
    /// checked out. A missing or malformed HEAD, permission, I/O, or corruption
    /// failure surfaces as [`SniffError::Git`] rather than collapsing to `None`.
    ///
    /// Resolves through gix's HEAD query, which reads packed refs, so a branch
    /// that exists only in `packed-refs` (after `git pack-refs --all --prune`)
    /// still reports its name.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::Git`] for any HEAD read or parse failure that is
    /// not a legitimate detached/unborn state.
    pub fn try_current_branch(&self) -> Result<Option<String>> {
        use gix::bstr::ByteSlice;
        use gix::head::Kind;

        let repo = self.gix.borrow();
        let head = repo.head().map_err(|e| SniffError::git("head", e))?;
        let name = match head.kind {
            Kind::Symbolic(reference) => reference.name,
            Kind::Unborn(_) | Kind::Detached { .. } => return Ok(None),
        };
        let full = name.as_bstr().to_str_lossy();
        Ok(full.strip_prefix("refs/heads/").map(str::to_string))
    }

    /// Whether the working directory is a linked worktree.
    pub fn in_worktree(&self) -> bool {
        self.gix.borrow().worktree().is_some_and(|wt| !wt.is_main())
    }

    /// Base repository root when inside a worktree.
    pub fn base_repo_root(&self) -> Option<PathBuf> {
        if self.in_worktree() {
            // Canonicalize first: gix may report a relative common_dir (e.g.
            // `.git/worktrees/wt/../..`) whose `.parent()` does not resolve to
            // the repository root without filesystem resolution.
            std::fs::canonicalize(self.gix.borrow().common_dir())
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        } else {
            None
        }
    }

    /// Organization and repository name parsed from the preferred remote URL.
    pub fn org_and_repo(&self) -> (Option<String>, Option<String>) {
        let remotes = super::remote_refresh::get_remotes(&self.gix.borrow(), false);
        preferred_remote(&remotes)
            .and_then(|r| r.url.as_deref())
            .map(parse_org_repo)
            .unwrap_or((None, None))
    }

    /// File-level working tree status (staged, modified, untracked).
    pub fn file_changes(&self) -> Result<Vec<FileChange>> {
        let (_status, changes) =
            super::status::get_repo_status_with_changes(&self.gix.borrow(), false)?;
        Ok(changes)
    }

    /// Aggregated working tree status.
    pub fn repo_status(&self) -> Result<RepoStatus> {
        let (status, _changes) =
            super::status::get_repo_status_with_changes(&self.gix.borrow(), false)?;
        Ok(status)
    }

    /// Recent commits from HEAD.
    pub fn recent_commits(&self, count: usize) -> Vec<CommitInfo> {
        let decorations = self.ref_decorations();
        super::discovery::get_recent_commits_with_decorations(
            &self.gix.borrow(),
            count,
            Some(&*decorations),
        )
    }

    /// Configured remotes.
    pub fn remotes(&self, include_details: bool) -> Vec<RemoteInfo> {
        super::remote_refresh::get_remotes(&self.gix.borrow(), include_details)
    }

    /// Linked worktrees.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::Git`] if a linked worktree cannot be opened due to
    /// trust, permission, I/O, or corruption failure. The trusted-open invariant
    /// is enforced for every worktree; failures are propagated rather than
    /// silently omitted.
    pub fn worktrees(&self) -> Result<HashMap<String, WorktreeInfo>> {
        self.ensure_cache();
        super::remote_refresh::get_worktrees(
            &self.gix.borrow(),
            true,
            Some(self.repo_root.as_path()),
        )
    }

    /// Git user configuration.
    pub fn config(&self) -> GitConfig {
        self.config_cache
            .get_or_init(|| super::remote_refresh::get_git_config(&self.gix.borrow()))
            .clone()
    }

    /// Local branch information.
    ///
    /// Errors are suppressed: corrupt or unreadable repositories report empty
    /// branch lists. For error propagation, use [`Self::try_branches`].
    pub fn branches(&self) -> Vec<LocalBranchInfo> {
        self.try_branches().unwrap_or_default()
    }

    /// Fallible local branch information.
    ///
    /// Propagates permission, I/O, and corruption failures as
    /// [`SniffError::Git`] rather than suppressing them.
    pub fn try_branches(&self) -> Result<Vec<LocalBranchInfo>> {
        self.ensure_cache();
        let current = self.try_current_branch()?;
        super::remote_refresh::get_local_branches_fallible(&self.gix.borrow(), current.as_deref(), true)
    }

    /// Branch projection for `sniff repo branches`.
    ///
    /// By default this uses only local refs and locally cached remote-tracking
    /// refs. When `refresh_remotes` is true, remote-tracking refs are refreshed
    /// first using the same non-interactive helper as deep git detection.
    pub fn branch_info(&self, refresh_remotes: bool) -> Result<Vec<BranchInfo>> {
        if refresh_remotes {
            super::remote_refresh::refresh_remote_tracking_refs(&self.gix.borrow(), 2);
        }
        self.ensure_cache();
        let current = self.try_current_branch()?;
        super::remote_refresh::get_branch_info_fallible(&self.gix.borrow(), current.as_deref())
    }

    /// Per-remote tracking status (ahead/behind).
    ///
    /// Errors are suppressed: corrupt or unreadable repositories report empty
    /// tracking lists. For error propagation, use [`Self::try_tracking_status`].
    pub fn tracking_status(&self) -> Vec<RemoteTrackingStatus> {
        self.try_tracking_status().unwrap_or_default()
    }

    /// Fallible per-remote tracking status.
    ///
    /// Propagates permission, I/O, and corruption failures as
    /// [`SniffError::Git`] rather than suppressing them.
    pub fn try_tracking_status(&self) -> Result<Vec<RemoteTrackingStatus>> {
        self.ensure_cache();
        let current = self.try_current_branch()?;
        super::remote_refresh::get_tracking_status_fallible(&self.gix.borrow(), current.as_deref())
    }

    /// Full detection — equivalent to `detect_git()` but reuses
    /// the already-opened repository handle.
    ///
    /// ## Notes
    ///
    /// Both `deep=false` and `deep=true` include full unified diff payloads to
    /// preserve backward compatibility. New callers that want file stats without
    /// diffs should use [`Self::detect_with_request`] with [`GitRequest::full()`] directly.
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
    ///
    /// Build the status-free identity payload used by [`GitRequest::identity()`].
    fn identity_only_info(&self, current_branch: Option<String>) -> GitInfo {
        let (org, repo) = self.org_and_repo();
        GitInfo {
            repo_root: self.repo_root.clone(),
            org,
            repo,
            current_branch,
            head_id: self.head_id(),
            branches: Vec::new(),
            in_worktree: self.in_worktree(),
            base_repo_root: self.base_repo_root(),
            recent: Vec::new(),
            status: None,
            remotes: Vec::new(),
            worktrees: HashMap::new(),
            config: GitConfig::default(),
            tracking: Vec::new(),
            file_changes: Vec::new(),
        }
    }

    pub fn detect_with_request(&self, request: &GitRequest) -> Result<GitInfo> {
        // Resolve the branch fallibly: a missing or malformed HEAD must surface
        // for every detection preset, not only the metadata-producing ones.
        // `minimal()`/`summary()` skip branch/tracking collection below, so this
        // is the only point where their HEAD corruption can be detected. The
        // result is reused for branch/tracking collection to avoid re-reading
        // HEAD per accessor.
        let current_branch = self.try_current_branch()?;

        if request.is_identity_only() {
            return Ok(self.identity_only_info(current_branch));
        }

        let is_minimal = request.is_minimal();

        if request.refresh_remote_tracking {
            super::remote_refresh::refresh_remote_tracking_refs(&self.gix.borrow(), 2);
        }

        let wants_tracking_refs = request.wants_tracking() && current_branch.is_some();
        let needs_ref_snapshot = request.wants_ref_decorations()
            || request.wants_branches()
            || wants_tracking_refs
            || (request.wants_remotes() && request.include_remote_branch_details)
            || (request.refresh_remote_tracking && request.include_commit_remote_containment);
        let ref_snapshot = if needs_ref_snapshot {
            self.ensure_cache();
            Some(super::remote_refresh::RefSnapshot::observe(
                &self.gix.borrow(),
                request.wants_branches() || wants_tracking_refs,
                wants_tracking_refs
                    || (request.wants_remotes() && request.include_remote_branch_details)
                    || (request.refresh_remote_tracking
                        && request.include_commit_remote_containment),
                request.wants_ref_decorations(),
            )?)
        } else {
            None
        };

        let mut recent = if request.wants_commits() {
            self.ensure_cache();
            // Decorations are an opt-out: a caller that wants bare commit
            // metadata should not pay for a ref-store walk and peel.
            if request.wants_ref_decorations() {
                super::discovery::get_recent_commits_fallible(
                    &self.gix.borrow(),
                    request.commit_count,
                    ref_snapshot.as_ref().map(|refs| refs.decorations()),
                )?
            } else {
                super::discovery::get_recent_commits_fallible(
                    &self.gix.borrow(),
                    request.commit_count,
                    None,
                )?
            }
        } else {
            Vec::new()
        };

        let (mut status, file_changes) = if request.include_file_changes {
            super::status::get_repo_status_with_changes(
                &self.gix.borrow(),
                request.include_file_diffs,
            )?
        } else if is_minimal {
            // minimal()/summary() only render the dirty flag — short-circuit on
            // the first change instead of counting every file.
            let is_dirty = super::status::is_repo_dirty(&self.gix.borrow())?;
            let status = RepoStatus {
                is_dirty,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
                dirty: Vec::new(),
                untracked: Vec::new(),
                is_behind: None,
            };
            (status, Vec::new())
        } else {
            let (is_dirty, staged, unstaged, untracked) =
                super::status::get_repo_status_counts_detailed(&self.gix.borrow())?;
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

        // Remotes, config, branches, and tracking are repo metadata that a pure
        // file-change query never renders. Each is gated on its own `wants_*`
        // accessor, which derives the legacy `wants_repo_metadata` answer unless
        // the caller supplied explicit controls — so a `summary()
        // .include_file_changes` request still skips the per-branch
        // `graph_ahead_behind` walk that otherwise dominates latency on repos
        // with many local branches.
        let remotes = if request.wants_remotes() {
            super::remote_refresh::get_remotes_from_snapshot(
                &self.gix.borrow(),
                request.include_remote_branch_details,
                ref_snapshot.as_ref(),
            )
        } else {
            Vec::new()
        };

        let worktrees = if request.wants_worktrees() {
            self.ensure_cache();
            super::remote_refresh::get_worktrees_from_snapshot(
                &self.gix.borrow(),
                request.full_worktree_details,
                Some(self.repo_root.as_path()),
                ref_snapshot.as_ref(),
            )?
        } else {
            HashMap::new()
        };

        let config = if request.wants_config() {
            self.config()
        } else {
            GitConfig::default()
        };

        let branches = if request.wants_branches() {
            self.ensure_cache();
            super::remote_refresh::get_local_branches_from_snapshot(
                &self.gix.borrow(),
                current_branch.as_deref(),
                request.wants_branch_divergence(),
                ref_snapshot.as_ref().expect("branch request observes refs"),
            )?
        } else {
            Vec::new()
        };

        let tracking = if request.wants_tracking() {
            if let Some(current_branch) = current_branch.as_deref() {
                self.ensure_cache();
                super::remote_refresh::get_tracking_status_from_snapshot(
                    &self.gix.borrow(),
                    Some(current_branch),
                    ref_snapshot.as_ref().expect("tracking request observes refs"),
                )?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if request.refresh_remote_tracking {
            status.is_behind = super::remote_refresh::summarize_behind_status(&tracking);
            if request.include_commit_remote_containment {
                self.ensure_cache();
                super::remote_refresh::populate_recent_commit_remotes_from_snapshot(
                    &self.gix.borrow(),
                    &mut recent,
                    request.max_remote_branches,
                    ref_snapshot
                        .as_ref()
                        .expect("containment request observes refs"),
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
            // Identity-only requests carry `head_id`; every status-bearing
            // preset leaves it unset so existing JSON shapes (e.g. `sniff repo
            // git-status --json`) gain no new top-level field.
            head_id: None,
            branches,
            in_worktree: self.in_worktree(),
            base_repo_root: self.base_repo_root(),
            recent,
            status: Some(status),
            remotes,
            worktrees,
            config,
            tracking,
            file_changes,
        })
    }

    /// Observe the Git shapes used only by the repository aggregate.
    ///
    /// This companion is deliberately crate-private: aggregate projection is
    /// a library operation, not a metadata flag or a field on [`GitInfo`]. The
    /// retained repository handle lets branch, worktree, and history
    /// observations reuse discovery, with one ref snapshot for branch facts.
    pub(crate) fn observe_aggregate_evidence(&self) -> Result<GitAggregateEvidence> {
        self.ensure_cache();
        let current_branch = self.try_current_branch()?;
        let refs = super::remote_refresh::RefSnapshot::observe(
            &self.gix.borrow(),
            true,
            true,
            false,
        )?;
        let branches = self.with_cached_gix(|repo| {
            super::remote_refresh::get_branch_info_from_snapshot(
                repo,
                current_branch.as_deref(),
                &refs,
            )
        })?;
        let worktrees = self
            .with_cached_gix(super::worktree::list_worktrees_from_gix)
            .map_err(|error| SniffError::SystemInfo {
                domain: "git.worktrees",
                message: error.to_string(),
            })?;
        let current_worktree =
            self.with_cached_gix(super::worktree::current_worktree_name_from_gix);
        let commits = super::recent_commits::get_recent_commits_by_duration_with_repo(
            self,
            Duration::days(AGGREGATE_COMMIT_WINDOW_DAYS),
            &format!("last {AGGREGATE_COMMIT_WINDOW_DAYS}d"),
            None,
        )?;

        Ok(GitAggregateEvidence {
            branches,
            worktrees,
            current_worktree,
            commits,
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
///     match info.status {
///         Some(status) => println!("Dirty: {}", status.is_dirty),
///         None => println!("Status not requested"),
///     }
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
    /// HEAD commit id as a full hex SHA.
    ///
    /// Only populated by [`GitRequest::identity()`]; status-bearing presets
    /// leave it `None` to preserve their existing JSON shape. Also `None` for an
    /// unborn HEAD even in identity mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_id: Option<String>,
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
    ///
    /// `None` means the status was not requested (e.g., an identity-only
    /// request), not that the repository is clean.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RepoStatus>,
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

/// Repository evidence needed only by the bare aggregate projection.
#[derive(Debug, Clone)]
pub(crate) struct GitAggregateEvidence {
    pub(crate) branches: Vec<BranchInfo>,
    pub(crate) worktrees: Vec<WorktreeEntry>,
    pub(crate) current_worktree: Option<String>,
    pub(crate) commits: CommitDescSet,
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
    /// Whether this worktree is the one the current process is running from.
    pub is_current: bool,
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
/// if let Some(status) = &git_info.status {
///     for dirty_file in &status.dirty {
///         println!("Modified: {:?}", dirty_file.filepath);
///         println!("Diff:\n{}", dirty_file.diff);
///     }
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
/// if let Some(status) = &git_info.status {
///     for untracked in &status.untracked {
///         println!("Untracked: {:?}", untracked.filepath);
///     }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        (dir, repo)
    }

    fn corrupt_packed_refs(repo_path: &Path) {
        let packed_refs = repo_path.join(".git").join("packed-refs");
        std::fs::write(
            &packed_refs,
            "# pack-refs with: peeled fully-peeled\n^garbage\n",
        )
        .unwrap();
    }

    #[test]
    fn config_cache_returns_same_value_on_repeated_access() {
        let (_dir, _repo) = setup_repo();
        let git_repo = GitRepo::discover(_dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        let config1 = git_repo.config();
        let config2 = git_repo.config();

        assert_eq!(config1.user_email, config2.user_email);
        assert_eq!(config1.user_name, config2.user_name);
    }

    #[test]
    fn corrupt_ref_try_branches_propagates_error() {
        let (dir, _repo) = setup_repo();
        corrupt_packed_refs(dir.path());

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        assert!(
            git_repo.try_branches().is_err(),
            "corrupt refs must propagate through try_branches"
        );
    }

    #[test]
    fn corrupt_ref_try_tracking_status_propagates_error() {
        let (dir, repo) = setup_repo();
        // Configure a remote so the tracking walk has to look up a
        // remote-tracking ref. The corrupt packed-refs file causes the
        // lookup to fail instead of returning an empty Ok result.
        repo.remote("origin", "https://example.com/repo").unwrap();
        corrupt_packed_refs(dir.path());

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        assert!(
            git_repo.try_tracking_status().is_err(),
            "corrupt refs must propagate through try_tracking_status"
        );
    }

    #[test]
    fn corrupt_ref_detect_with_request_propagates_error() {
        let (dir, _repo) = setup_repo();
        corrupt_packed_refs(dir.path());

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        assert!(
            git_repo.detect_with_request(&GitRequest::full()).is_err(),
            "corrupt refs must propagate through detect_with_request"
        );
    }

    #[test]
    fn branches_convenience_suppresses_errors_to_empty_vec() {
        let (dir, _repo) = setup_repo();
        corrupt_packed_refs(dir.path());

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        assert!(
            git_repo.branches().is_empty(),
            "infallible branches() must suppress errors to an empty vec"
        );
    }

    #[test]
    fn tracking_status_convenience_suppresses_errors_to_empty_vec() {
        let (dir, repo) = setup_repo();
        repo.remote("origin", "https://example.com/repo").unwrap();
        corrupt_packed_refs(dir.path());

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        assert!(
            git_repo.tracking_status().is_empty(),
            "infallible tracking_status() must suppress errors to an empty vec"
        );
    }

    #[test]
    fn identity_request_does_not_walk_status() {
        let (dir, _repo) = setup_repo();
        // Make the repo dirty so a status walk would be observable.
        std::fs::write(dir.path().join("dirty.txt"), "x\n").unwrap();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        // Measure walks for this repo's path as a before/after delta: a global
        // counter would be contaminated by other tests' walks under `cargo test`.
        let before = crate::filesystem::git::status::status_walk_count(dir.path());
        let info = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();

        assert_eq!(
            crate::filesystem::git::status::status_walk_count(dir.path()),
            before,
            "identity() must not trigger a working-tree status walk"
        );
        assert!(info.status.is_none());

        // Prove the gate is real: summary() does trigger a status walk.
        let _ = git_repo
            .detect_with_request(&GitRequest::summary())
            .unwrap();
        assert!(
            crate::filesystem::git::status::status_walk_count(dir.path()) > before,
            "summary() must record a status walk"
        );
    }

    #[test]
    fn identity_request_on_branch_has_expected_fields() {
        let (dir, _repo) = setup_repo();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");
        let info = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();

        assert!(
            info.current_branch
                .as_deref()
                .is_some_and(|b| !b.is_empty()),
            "current_branch should be set on a branch: {:?}",
            info.current_branch
        );
        assert!(!info.in_worktree);
        assert!(info.head_id.is_some());
        assert!(info.status.is_none());
        assert!(info.branches.is_empty());
        assert!(info.remotes.is_empty());
        assert!(info.worktrees.is_empty());
        assert!(info.recent.is_empty());
        assert!(info.file_changes.is_empty());
    }

    #[test]
    fn identity_request_in_linked_worktree_has_expected_fields() {
        let (dir, repo) = setup_repo();
        let worktree_path = dir.path().join("linked-wt");
        repo.worktree("linked", &worktree_path, None).unwrap();

        let git_repo = GitRepo::discover(&worktree_path)
            .unwrap()
            .expect("worktree should be discoverable");
        let info = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();

        assert!(info.in_worktree);
        assert!(info.base_repo_root.is_some());
        assert_eq!(
            info.base_repo_root
                .as_ref()
                .unwrap()
                .canonicalize()
                .unwrap(),
            dir.path().canonicalize().unwrap()
        );
        assert!(info.status.is_none());
    }

    #[test]
    fn identity_request_detached_head_has_expected_fields() {
        let (dir, repo) = setup_repo();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.set_head_detached(commit.id()).unwrap();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");
        let info = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();

        assert_eq!(info.current_branch, None);
        assert!(info.head_id.is_some());
        assert!(info.status.is_none());
    }

    #[test]
    fn identity_request_unborn_head_has_expected_fields() {
        let dir = TempDir::new().unwrap();
        let _repo = git2::Repository::init(dir.path()).unwrap();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");
        let info = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();

        assert_eq!(info.current_branch, None);
        assert_eq!(info.head_id, None);
        assert!(info.status.is_none());
    }

    #[test]
    fn identity_request_serializes_without_status_field() {
        let (dir, _repo) = setup_repo();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");
        let identity = git_repo
            .detect_with_request(&GitRequest::identity())
            .unwrap();
        let identity_json = serde_json::to_string(&identity).unwrap();
        let identity_value: serde_json::Value = serde_json::from_str(&identity_json).unwrap();
        assert!(
            !identity_value.as_object().unwrap().contains_key("status"),
            "identity() JSON must omit the status field"
        );

        let summary = git_repo
            .detect_with_request(&GitRequest::summary())
            .unwrap();
        let summary_json = serde_json::to_string(&summary).unwrap();
        let summary_value: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
        assert!(
            summary_value.as_object().unwrap().contains_key("status"),
            "summary() JSON must include the status field"
        );
    }

    #[test]
    fn existing_presets_still_compute_status() {
        let (dir, _repo) = setup_repo();
        // Make the repo dirty so the dirty-flag contract is observable.
        std::fs::write(dir.path().join("dirty.txt"), "x\n").unwrap();

        let git_repo = GitRepo::discover(dir.path())
            .unwrap()
            .expect("repo should be discoverable");

        for request in [GitRequest::minimal(), GitRequest::summary()] {
            let info = git_repo.detect_with_request(&request).unwrap();
            assert!(
                info.status.is_some(),
                "{:?} must still yield a status object",
                request
            );
            assert!(
                info.status.unwrap().is_dirty,
                "{:?} must report a dirty fixture",
                request
            );
        }

        for request in [GitRequest::full(), GitRequest::deep()] {
            let info = git_repo.detect_with_request(&request).unwrap();
            assert!(
                info.status.is_some(),
                "{:?} must still yield a status object",
                request
            );
        }
    }

    #[test]
    fn focused_ref_consumers_share_one_observation() {
        let (dir, repo) = setup_repo();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        let head = repo.head().unwrap().target().unwrap();
        repo.remote("origin", "https://example.com/acme/project.git")
            .unwrap();
        repo.reference(
            &format!("refs/remotes/origin/{branch}"),
            head,
            true,
            "tracking fixture",
        )
        .unwrap();
        repo.reference_symbolic(
            "refs/remotes/origin/HEAD",
            &format!("refs/remotes/origin/{branch}"),
            true,
            "default branch fixture",
        )
        .unwrap();
        repo.tag_lightweight("v1", &repo.find_commit(head).unwrap().into_object(), true)
            .unwrap();

        let mut request = GitRequest::full().metadata(
            crate::request::GitMetadataRequest::none()
                .commits(true)
                .ref_decorations(true)
                .branches(true)
                .remotes(true)
                .tracking(true),
        );
        request.include_remote_branch_details = true;

        let git_repo = GitRepo::discover(dir.path()).unwrap().unwrap();
        let collector = crate::performance::PerformanceCollector::new_shared();
        let info = crate::performance::with_current_collector(Some(collector.clone()), || {
            git_repo.detect_with_request(&request).unwrap()
        });
        let counters = collector
            .snapshot(std::time::Duration::ZERO)
            .counters;

        assert_eq!(
            counters
                .get(crate::performance::counters::GIT_REF_WALKS)
                .copied()
                .unwrap_or(0),
            1,
            "branches, tracking, remote tips, and decorations share one pass: {counters:?}"
        );
        assert!(info.recent.iter().any(|commit| !commit.refs.is_empty()));
        assert!(info.branches.iter().any(|candidate| candidate.name == branch));
        assert!(info.tracking.iter().any(|status| status.remote == "origin"));
        let origin = info
            .remotes
            .iter()
            .find(|remote| remote.name == "origin")
            .unwrap();
        assert_eq!(origin.default_branch.as_deref(), Some(branch.as_str()));
        assert_eq!(origin.branches.as_deref(), Some(&[branch] as &[String]));
    }

    #[test]
    fn focused_worktree_metadata_opens_no_linked_repositories() {
        let (dir, repo) = setup_repo();
        let linked_path = dir.path().join("linked-native-path");
        repo.worktree("linked", &linked_path, None).unwrap();

        let request = GitRequest::full().metadata(
            crate::request::GitMetadataRequest::none().worktrees(true),
        );
        let git_repo = GitRepo::discover(dir.path()).unwrap().unwrap();
        let collector = crate::performance::PerformanceCollector::new_shared();
        let info = crate::performance::with_current_collector(Some(collector.clone()), || {
            git_repo.detect_with_request(&request).unwrap()
        });
        let counters = collector
            .snapshot(std::time::Duration::ZERO)
            .counters;

        assert_eq!(
            counters
                .get(crate::performance::counters::GIT_WORKTREE_OPENS)
                .copied()
                .unwrap_or(0),
            0,
            "metadata-only linked worktrees must not be opened: {counters:?}"
        );
        let linked = info.worktrees.get("master").or_else(|| {
            info.worktrees
                .values()
                .find(|worktree| {
                    worktree.filepath == std::fs::canonicalize(&linked_path).unwrap()
                })
        });
        let linked = linked.expect("linked worktree is projected from metadata");
        assert_eq!(
            linked.filepath,
            std::fs::canonicalize(linked_path).unwrap()
        );
        assert!(!linked.sha.is_empty());
        assert!(!linked.is_current);
    }

    #[test]
    fn focused_worktrees_reuse_current_linked_repository() {
        let (dir, repo) = setup_repo();
        let linked_path = dir.path().join("current-linked");
        repo.worktree("current-linked", &linked_path, None).unwrap();

        let request = GitRequest::full().metadata(
            crate::request::GitMetadataRequest::none().worktrees(true),
        );
        let git_repo = GitRepo::discover(&linked_path).unwrap().unwrap();
        let collector = crate::performance::PerformanceCollector::new_shared();
        let info = crate::performance::with_current_collector(Some(collector.clone()), || {
            git_repo.detect_with_request(&request).unwrap()
        });
        let counters = collector
            .snapshot(std::time::Duration::ZERO)
            .counters;

        assert_eq!(
            counters
                .get(crate::performance::counters::GIT_WORKTREE_OPENS)
                .copied()
                .unwrap_or(0),
            0,
            "the already-discovered linked repository supplies current details: {counters:?}"
        );
        let current = info
            .worktrees
            .values()
            .find(|worktree| {
                worktree.filepath == std::fs::canonicalize(&linked_path).unwrap()
            })
            .expect("current linked worktree is present");
        assert!(current.is_current);
        assert!(!current.sha.is_empty());
    }
}

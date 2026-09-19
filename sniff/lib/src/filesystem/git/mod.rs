pub mod api;
pub mod commit_links;
pub mod discovery;
mod merge_conflicts;
pub mod open;
pub mod recent_commits;
#[cfg(feature = "network")]
pub mod remote_observation;
pub mod remote_refresh;
pub mod remote_resolver;
pub mod status;
pub mod types;
pub mod worktree;

pub use api::{
    branches_at, commit_browser_url, commit_by_sha_at, commit_files_at, commit_links_at,
    commits_for_branch_at,
    commits_for_path_at, merge_conflicts_at, merge_conflicts_with_branch_at, preferred_remote_url,
    remote_url, repo_root,
};
#[cfg(feature = "network")]
pub use remote_observation::{branch_exists_on_remote_at, remote_vendor_at};
pub use remote_resolver::{ApiFlavor, RemoteEndpoint, ResolvedRemote, resolve_remote_at};

pub use commit_links::{CommitLink, RepositoryLink, commit_url, repository_link};
pub use discovery::{
    DEFAULT_PATH_HISTORY_SCAN_LIMIT, DeltaKind, PathHistoryOptions, PathHistoryResult, detect_git,
    detect_git_with_request, get_commit_by_sha, get_commit_files, get_commit_files_with_cache,
    get_commits_for_branch, get_commits_for_path,
};
pub use recent_commits::{
    DEFAULT_RECENT_COMMIT_COUNT, NamedDate, RecentCommit, RecentCommitAuthor, RecentCommitFile,
    RecentCommitFileKind, RecentCommitFileTypes, RecentCommitPackages, RecentCommits,
    RecentCommitsOptions, RecentCommitsProjection, RecentCommitsVerbosity, Selection,
};
pub use status::detect_merge_conflicts;
pub use types::{
    BehindStatus, BranchInfo, CommitInfo, ConventionalCommit, FileAction, FileChange, FileStatus,
    GitConfig, GitHostingProvider, GitHostingProviderMetadata, GitInfo, GitRepo, LocalBranchInfo,
    RefDecoration, RefKind, RemoteInfo, RemoteTrackingStatus, RepoStatus, UntrackedFile,
    WorktreeInfo, parse_commit_message,
};
pub use worktree::{
    WorktreeEntry, current_worktree_name_with_repo, get_current_worktree_info,
    get_current_worktree_name, list_worktrees, list_worktrees_with_repo,
};

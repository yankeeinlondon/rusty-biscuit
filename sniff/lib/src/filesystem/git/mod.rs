pub mod api;
pub mod discovery;
mod merge_conflicts;
pub mod open;
pub mod recent_commits;
pub mod remote_resolver;
#[cfg(feature = "network")]
pub mod remote_observation;
pub mod remote_refresh;
pub mod status;
pub mod types;
pub mod worktree;

pub use api::{
    branches_at, commit_browser_url, commit_by_sha_at, commit_files_at, commits_for_branch_at,
    commits_for_path_at, merge_conflicts_at, merge_conflicts_with_branch_at, preferred_remote_url,
    remote_url, repo_root,
};
pub use remote_resolver::{ApiFlavor, RemoteEndpoint, ResolvedRemote, resolve_remote_at};
#[cfg(feature = "network")]
pub use remote_observation::{branch_exists_on_remote_at, remote_vendor_at};

pub use discovery::{
    DeltaKind, detect_git, detect_git_with_request, get_commit_by_sha, get_commit_files,
    get_commit_files_with_cache, get_commits_for_branch, get_commits_for_path,
};
pub use recent_commits::{
    CommitDesc, CommitDescSet, CommitFileChange, PeriodSpecifier, get_recent_commits_by_count,
    get_recent_commits_by_date, get_recent_commits_by_duration, get_recent_commits_by_hash,
    get_recent_commits_in_range, parse_period,
};
pub use status::detect_merge_conflicts;
pub use types::{
    BehindStatus, BranchInfo, CommitInfo, ConventionalCommit, FileAction, FileChange, FileStatus,
    GitConfig, GitHostingProvider, GitHostingProviderMetadata, GitInfo, GitRepo, LocalBranchInfo,
    RefDecoration, RefKind, RemoteInfo, RemoteTrackingStatus, RepoStatus, UntrackedFile,
    WorktreeInfo,
};
pub use worktree::{
    WorktreeEntry, get_current_worktree_info, get_current_worktree_name, list_worktrees,
};

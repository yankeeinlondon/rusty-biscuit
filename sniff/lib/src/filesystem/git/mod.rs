pub mod detection;
pub mod recent_commits;
pub mod types;

pub use detection::{
    DeltaKind, detect_git, detect_git_with_request, detect_merge_conflicts, get_commit_by_sha,
    get_commit_files, get_commits_for_path,
};
pub use recent_commits::{
    CommitDesc, CommitDescSet, CommitFileChange, PeriodSpecifier, get_recent_commits_by_date,
    get_recent_commits_by_duration, get_recent_commits_by_hash, get_recent_commits_in_range,
    parse_period,
};
pub use types::{
    BehindStatus, CommitInfo, ConventionalCommit, FileAction, FileChange, FileStatus, GitConfig,
    GitHostingProvider, GitHostingProviderMetadata, GitInfo, GitRepo, LocalBranchInfo,
    RefDecoration, RefKind, RemoteInfo, RemoteTrackingStatus, RepoStatus, UntrackedFile,
    WorktreeInfo,
};

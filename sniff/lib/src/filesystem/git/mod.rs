pub mod detection;
pub mod types;

pub use detection::{
    DeltaKind, detect_git, detect_git_with_request, detect_merge_conflicts, get_commit_by_sha,
    get_commit_files, get_commits_for_path,
};
pub use types::{
    BehindStatus, CommitInfo, ConventionalCommit, FileAction, FileChange, FileStatus, GitConfig,
    GitHostingProvider, GitHostingProviderMetadata, GitInfo, GitRepo, LocalBranchInfo,
    RefDecoration, RefKind, RemoteInfo, RemoteTrackingStatus, RepoStatus, UntrackedFile,
    WorktreeInfo,
};

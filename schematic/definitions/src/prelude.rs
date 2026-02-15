//! Convenient re-exports for working with API definitions.
//!
//! This prelude provides all API definition functions and their associated types.
//!
//! ## Examples
//!
//! ```
//! use schematic_definitions::prelude::*;
//!
//! let api = define_openai_api();
//! assert_eq!(api.name, "OpenAI");
//! ```
//!
//! ```
//! use schematic_definitions::prelude::*;
//!
//! let api = define_github_api();
//! assert_eq!(api.name, "GitHub");
//! assert_eq!(api.endpoints.len(), 14);
//! ```
//!
//! ```
//! use schematic_definitions::prelude::*;
//!
//! let api = define_gitea_api();
//! assert_eq!(api.name, "Gitea");
//! assert_eq!(api.endpoints.len(), 14);
//! ```
//!
//! ```
//! use schematic_definitions::prelude::*;
//!
//! let api = define_bitbucket_api();
//! assert_eq!(api.name, "Bitbucket");
//! assert_eq!(api.endpoints.len(), 14);
//! ```

// API definition functions
pub use crate::bitbucket::define_bitbucket_api;
pub use crate::gitea::define_gitea_api;
pub use crate::github::define_github_api;
pub use crate::openai::define_openai_api;

// Response types for OpenAI
pub use crate::openai::{DeleteModelResponse, ListModelsResponse, Model};

// Response types for GitHub (key top-level types)
pub use crate::github::{
    AnnotatedTagObject, GitRef, GitTreeResponse, IssueSummary, PullRequestSummary, Release,
    RepoTag, RepositoryInfo,
};

// Response types for Gitea (key top-level types, prefixed to avoid collision)
pub use crate::gitea::{
    AnnotatedTagObject as GiteaAnnotatedTagObject, GitRef as GiteaGitRef,
    GitTreeResponse as GiteaGitTreeResponse, IssueSummary as GiteaIssueSummary,
    PullRequestSummary as GiteaPullRequestSummary, Release as GiteaRelease,
    RepoTag as GiteaRepoTag, RepositoryInfo as GiteaRepositoryInfo,
};

// Response types for Bitbucket (key top-level types, prefixed to avoid collision)
pub use crate::bitbucket::{
    Download as BitbucketDownload, Issue as BitbucketIssue,
    PaginatedResponse as BitbucketPaginatedResponse, PullRequest as BitbucketPullRequest,
    Repository as BitbucketRepository, SourceEntry as BitbucketSourceEntry,
    Tag as BitbucketTag, User as BitbucketUser,
};

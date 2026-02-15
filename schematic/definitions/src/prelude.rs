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

// API definition functions
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

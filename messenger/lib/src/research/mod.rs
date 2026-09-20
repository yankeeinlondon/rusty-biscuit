//! Provider research contract: typed loading, semantic validation, and
//! implementation assessment (feature `research`).
//!
//! The pipeline is offline and passive: it reads the roster
//! (`messenger/docs/platforms.yaml`), platform research documents, overrides,
//! and reviewed implementation mappings; validates them against the shipped
//! SimplifiedSchema through Darkmatter's library; and applies the Rust-owned
//! rules listed in `messenger/docs/research/platforms/_rules.md`. It never
//! changes `CapabilitySet` or any delivery behavior, and the ordinary send
//! path does not depend on it.
//!
//! Design record: `messenger/features/2026-09-17-research-metadata-pipeline/architecture.md`.

pub mod assess;
pub mod canonical;
pub mod delta;
pub mod diagnostics;
pub mod error;
pub mod generate;
pub mod load;
pub mod model;
pub mod paths;
pub mod project;
pub mod publish;
pub mod refresh;
pub mod report;
pub mod validate;

pub use diagnostics::{Diagnostic, Rule, sort_diagnostics};
pub use error::ResearchError;
pub use load::{ContractFile, ContractKind, Loaded, Loader};
pub use paths::{RepoPath, Workspace};
pub use validate::{
    Context, DocumentValidation, FactRecord, Scope, ValidatedDocument, coverage_summary, validate_document, validate_fleet,
    validate_mappings, validate_overrides, validate_roster,
};

//! Runtime context capture, formatting, and merge for compose pipeline.
//!
//! This module assembles a rich runtime context from the local clock,
//! environment, repository/monorepo inspection, document discovery,
//! OS, and hardware -- all powered by the `sniff` library.

pub mod catalog;
pub(crate) mod capture;
pub(crate) mod diagnostics;
pub(crate) mod effective_state;
pub(crate) mod format;
pub mod merge;
pub(crate) mod options;
pub(crate) mod report;
pub(crate) mod runtime;

pub use catalog::{context_variable_descriptors, ContextValueType, ContextVariableDescriptor};
pub use capture::{ContextCaptureEvidence, ContextGroup, ContextRequirements};
pub use diagnostics::ContextMergeDiagnostic;
pub use merge::merge_ctx;

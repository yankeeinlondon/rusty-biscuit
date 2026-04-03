//! Composition services for inline and chained document workflows.
//!
//! This module provides the shared logic for Claudine's composition features:
//! - File reference resolution via `biscuit-file::FileReference`
//! - Prompt preparation (inline frontmatter prompt and chained document)
//! - Agent/provider selection with precedence rules
//! - Composition-specific error types

pub mod closure;
mod error;
mod guardrails;
pub mod lifecycle;
pub mod preflight;
mod prepare;
mod resolve;
mod select;
mod types;

pub use error::CompositionError;
pub use lifecycle::{
    LifecycleConfig, LifecycleNotification, LifecycleRuntimeContext, LifecycleRuntimeState,
    LifecycleSignal, emit_lifecycle_signal, parse_lifecycle_config,
};
pub use preflight::{PreFlightResult, resolve_shell_approvals};
pub use prepare::{prepare_direct, prepare_inline};
pub use resolve::{resolve_composition_source, validate_file_permissions};
pub use select::{build_candidate_set, select_provider};
pub use types::{
    CompositionClosurePlan, CompositionExecutionRequest, CompositionMode, InlineClosurePlan,
    PreparedComposition, ResolvedCompositionSource, SelectedProvider, SelectionReason,
};

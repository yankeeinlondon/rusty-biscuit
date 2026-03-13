//! Composition services for inline and chained document workflows.
//!
//! This module provides the shared logic for Claudine's composition features:
//! - File reference resolution via `biscuit-file::FileReference`
//! - Prompt preparation (inline frontmatter prompt and chained document)
//! - Agent/provider selection with precedence rules
//! - Composition-specific error types

mod error;
mod prepare;
mod resolve;
mod select;
mod types;

pub use error::CompositionError;
pub use prepare::{prepare_chained_prompt, prepare_inline_prompt};
pub use resolve::resolve_composition_source;
pub use select::{build_candidate_set, select_provider, select_provider_with_env};
pub use types::{
    CompositionMode, CompositionRequest, PreparedPrompt, ResolvedCompositionSource,
    SelectedProvider, SelectionReason,
};

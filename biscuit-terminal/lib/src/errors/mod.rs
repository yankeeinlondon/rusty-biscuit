//! Shared error rendering primitives.
//!
//! This module exposes the [`BlockError`] trait — a terminal-rendering contract
//! that lets any [`std::error::Error`] produce a consistent `Status` +
//! `StatusBlock` block-style report.
//!
//! ## Convenience prelude
//!
//! The [`prelude`] module re-exports the minimal set of names needed when
//! implementing [`BlockError`]. Use it to reduce boilerplate in error
//! implementations:
//!
//! ```
//! use biscuit_terminal::errors::prelude::*;
//! ```

pub mod block_error;
pub mod prelude;
pub mod source_context;

pub use block_error::{
    BlockError, ErrorHeader, StatusBlockExt, as_block_error, render_with_causes,
};
pub use source_context::{SourceContext, YamlKeyPath};

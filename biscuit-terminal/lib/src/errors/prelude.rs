//! Convenience re-exports for implementing [`BlockError`].
//!
//! This prelude imports the minimal set of names needed inside a typical
//! `BlockError::status_block` implementation, reducing boilerplate from
//! three separate `use` lines to a single `use biscuit_terminal::errors::prelude::*`.
//!
//! ## When to use this prelude
//!
//! Use this prelude when implementing [`BlockError`] on custom error types.
//! For general terminal rendering (building status blocks, prose, or other
//! components outside error contexts), prefer the crate-level
//! [`biscuit_terminal::prelude`](crate::prelude).
//!
//! ## Examples
//!
//! ```
//! use std::error::Error;
//! use std::fmt;
//! use biscuit_terminal::errors::prelude::*;
//!
//! #[derive(Debug)]
//! struct CycleDetected { chain: Vec<String> }
//!
//! impl fmt::Display for CycleDetected {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "cycle detected: {}", self.chain.join(" -> "))
//!     }
//! }
//!
//! impl Error for CycleDetected {}
//!
//! impl BlockError for CycleDetected {
//!     fn status_block(&self, _term: &Terminal) -> StatusBlock {
//!         StatusBlock::new(StatusState::Error)
//!             .error_header(ErrorHeader::new("CycleDetected", "transclusion cycle"))
//!             .body(format!("chain: {}", self.chain.join(" -> ")))
//!             .hint("Break the cycle by removing one of the edges.")
//!     }
//! }
//!
//! let err = CycleDetected { chain: vec!["a.md".into(), "b.md".into(), "a.md".into()] };
//! let out = err.report_block_error_optimistic(Some(80));
//! assert!(out.contains("CycleDetected"));
//! ```

pub use super::{BlockError, ErrorHeader, StatusBlockExt, render_with_causes};
pub use crate::components::status::StatusState;
pub use crate::components::status_block::StatusBlock;
pub use crate::terminal::Terminal;

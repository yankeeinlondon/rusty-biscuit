//! Shared error rendering primitives.
//!
//! This module exposes the [`BlockError`] trait — a terminal-rendering contract
//! that lets any [`std::error::Error`] produce a consistent `Status` +
//! `StatusBlock` block-style report.

pub mod block_error;

pub use block_error::{
    BlockError, ErrorHeader, StatusBlockExt, as_block_error, render_with_causes,
};

//! Code generation utilities for safe Rust code injection.
//!
//! This module provides utilities for safely injecting Rust code (particularly enums)
//! into source files using AST-based manipulation with the `syn` crate.
//!
//! # Safety Guarantees
//!
//! - **Pre/Post Validation:** All code is validated before and after modification
//! - **Atomic Writes:** Uses atomic file operations (tempfile + rename) to prevent partial writes
//! - **Rollback Safety:** Original files remain untouched if any validation fails
//! - **AST-Based:** Uses proper parsing instead of regex for robust handling of all Rust syntax
//!
//! # Examples
//!
//! ```rust,no_run
//! use shared::codegen::inject_enum;
//!
//! let new_enum = r#"
//! pub enum Color {
//!     Red,
//!     Green,
//!     Blue,
//! }
//! "#;
//!
//! inject_enum("Color", new_enum, "src/types.rs")?;
//! # Ok::<(), shared::codegen::CodegenError>(())
//! ```

mod inject;
mod validation;

pub use inject::{inject_enum, inject_enum_variants};
pub use validation::validate_syntax;

use thiserror::Error;

/// Errors that can occur during code generation operations.
#[derive(Debug, Error)]
pub enum CodegenError {
    /// Syntax error in Rust file (pre or post injection).
    #[error("Syntax error in Rust file: {message}")]
    SyntaxError { message: String },

    /// File I/O error during read/write operations.
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Attempted to replace an enum that doesn't exist in the file.
    #[error("Enum not found: {name}")]
    EnumNotFound { name: String },

    /// Failed to persist temporary file atomically.
    #[error("Failed to persist temporary file: {0}")]
    PersistError(#[from] tempfile::PersistError),
}

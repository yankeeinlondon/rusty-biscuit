//! Darkmatter CLI crate.
//!
//! This crate provides the `md` binary and a small programmatic surface for
//! parsing CLI arguments. User-facing command usage and the source module map
//! live in `darkmatter/cli/README.md`; Markdown parsing, composition, and
//! rendering APIs live in the `darkmatter` library crate.

pub mod approval;
pub mod args;
pub mod artifact;
pub mod commands;
pub mod io;
pub mod render;
pub mod style_claims;

// Re-export CLI types for programmatic access
pub use args::{Cli, Command as CliCommand, OutputFormat};

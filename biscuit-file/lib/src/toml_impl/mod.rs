//! TOML parsing, conversion, and validation.
//!
//! This module provides the [`Toml`] struct for working with TOML files,
//! including conversion to JSON and YAML formats, and optional schema validation.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::Toml;
//!
//! let toml = Toml::from_str(r#"
//! [package]
//! name = "example"
//! version = "1.0"
//! "#)?;
//!
//! // Convert to other formats
//! let json = toml.as_json()?;
//! let yaml = toml.as_yaml()?;
//!
//! // Validate
//! let report = toml.validate();
//! # Ok::<(), biscuit_file::TomlError>(())
//! ```

mod types;

pub use types::{Toml, TomlError, TomlSource, ValidationIssue, ValidationLevel, ValidationReport};

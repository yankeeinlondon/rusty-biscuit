//! TOML parsing, conversion, and validation.
//!
//! This module provides the [`Toml`] struct for working with TOML files,
//! including conversion to JSON and YAML formats, and optional schema validation.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use biscuit_file::Toml;
//!
//! // Parse from file
//! let toml = Toml::new("config.toml")?;
//!
//! // Convert to other formats
//! let json = toml.as_json()?;
//! let yaml = toml.as_yaml()?;
//!
//! // Validate
//! let report = toml.validate()?;
//! ```

mod types;

pub use types::{Toml, TomlError, TomlSource, ValidationIssue, ValidationLevel, ValidationReport};

//! File format utilities for the dockhand ecosystem.
//!
//! This crate provides parsing, conversion, and validation for common file formats:
//!
//! - **TOML**: Parse, convert to JSON/YAML, validate against schema
//! - **YAML**: Parse, convert to JSON/TOML, validate against schema
//! - **JSON5**: Parse, convert to JSON/YAML/TOML
//! - **PDF**: Extract text, convert to Markdown, extract table of contents
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `toml` | Yes | TOML parsing and conversion |
//! | `yaml` | Yes | YAML parsing and conversion |
//! | `json5` | Yes | JSON5 parsing and conversion |
//! | `extract` | Yes | PDF text extraction via pdf-extract |
//! | `lopdf` | Yes | PDF TOC extraction via lopdf |
//! | `pdfium` | No | High-fidelity PDF extraction via pdfium-render |
//! | `schema` | No | JSON Schema validation for TOML/YAML |
//! | `full` | No | All features enabled |
//!
//! ## Re-exports
//!
//! For convenience, this crate re-exports types from underlying crates,
//! allowing consumers to use `use biscuit_file::serde_yaml::Value` instead of
//! `use serde_yaml::Value`. This facilitates migrating from direct crate
//! dependencies to this unified library.
//!
//! ## Examples
//!
//! ### TOML Conversion
//!
//! ```rust
//! use biscuit_file::Toml;
//!
//! let toml = Toml::from_str(r#"
//! [package]
//! name = "example"
//! version = "1.0"
//! "#)?;
//! let json = toml.as_json()?;
//! let yaml = toml.as_yaml()?;
//! # Ok::<(), biscuit_file::TomlError>(())
//! ```
//!
//! ### YAML Conversion
//!
//! ```rust
//! use biscuit_file::Yaml;
//!
//! let yaml = Yaml::from_str("name: example\nversion: '1.0'")?;
//! let json = yaml.as_json()?;
//! let toml = yaml.as_toml()?;
//! # Ok::<(), biscuit_file::YamlError>(())
//! ```
//!
//! ### PDF Extraction
//!
//! ```rust,no_run
//! use biscuit_file::Pdf;
//!
//! let pdf = Pdf::new("document.pdf")?;
//! let text = pdf.as_text()?;
//! let markdown = pdf.as_markdown(Default::default())?;
//! let toc = pdf.toc()?;
//! # Ok::<(), biscuit_file::PdfError>(())
//! ```

mod detect;
mod error;

#[cfg(feature = "toml")]
pub mod toml_impl;

#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(feature = "json5")]
pub mod json5;

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub mod pdf;

// Re-export core error type
pub use error::BiscuitFileError;

// Re-export file type detection
pub use detect::{FileType, detect_file_type, detect_file_type_from_bytes};

// Re-export format-specific types
#[cfg(feature = "toml")]
pub use self::toml_impl::{Toml, TomlError, TomlSource};

#[cfg(feature = "yaml")]
pub use self::yaml::{Yaml, YamlError, YamlSource};

#[cfg(feature = "json5")]
pub use self::json5::{Json5, Json5Error, Json5Source};

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub use self::pdf::{Pdf, PdfConfig, PdfError, PdfMarkdown, PdfToc};

// Re-export underlying crate types for convenience
// This allows consumers to use `use biscuit_file::serde_yaml::Value`
// instead of `use serde_yaml::Value`, facilitating migration.

#[cfg(feature = "toml")]
pub use toml as toml_crate;

#[cfg(feature = "yaml")]
pub use serde_yaml_ng;

/// YAML value type for direct manipulation.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Value as YamlValue;

/// YAML mapping type for direct manipulation.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Mapping as YamlMapping;

/// Error type for YAML parsing.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Error as YamlParseError;

// Re-export TOML parsing error type
#[cfg(feature = "toml")]
mod toml_error_reexport {
    pub use toml::de::Error;
}

#[cfg(feature = "toml")]
pub use toml_error_reexport::Error as TomlDeError;

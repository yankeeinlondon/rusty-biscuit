//! File format utilities for the dockhand ecosystem.
//!
//! This crate provides parsing, conversion, and validation for common file formats:
//!
//! - **TOML**: Parse, convert to JSON/YAML, validate against schema
//! - **YAML**: Parse, convert to JSON/TOML, validate against schema
//! - **PDF**: Extract text, convert to Markdown, extract table of contents
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `toml` | Yes | TOML parsing and conversion |
//! | `yaml` | Yes | YAML parsing and conversion |
//! | `extract` | Yes | PDF text extraction via pdf-extract |
//! | `lopdf` | Yes | PDF TOC extraction via lopdf |
//! | `pdfium` | No | High-fidelity PDF extraction via pdfium-render |
//! | `schema` | No | JSON Schema validation for TOML/YAML |
//! | `full` | No | All features enabled |
//!
//! ## Examples
//!
//! ### TOML Conversion
//!
//! ```rust,ignore
//! use biscuit_file::Toml;
//!
//! let toml = Toml::new("config.toml")?;
//! let json = toml.as_json()?;
//! let yaml = toml.as_yaml()?;
//! ```
//!
//! ### YAML Conversion
//!
//! ```rust,ignore
//! use biscuit_file::Yaml;
//!
//! let yaml = Yaml::new("config.yaml")?;
//! let json = yaml.as_json()?;
//! let toml = yaml.as_toml()?;
//! ```
//!
//! ### PDF Extraction
//!
//! ```rust,ignore
//! use biscuit_file::Pdf;
//!
//! let pdf = Pdf::new("document.pdf")?;
//! let text = pdf.as_text()?;
//! let markdown = pdf.as_markdown(Default::default())?;
//! let toc = pdf.toc()?;
//! ```

mod detect;
mod error;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub mod pdf;

// Re-export core error type
pub use error::BiscuitFileError;

// Re-export file type detection
pub use detect::{FileType, detect_file_type, detect_file_type_from_bytes};

// Re-export format-specific types
#[cfg(feature = "toml")]
pub use self::toml::{Toml, TomlError, TomlSource};

#[cfg(feature = "yaml")]
pub use self::yaml::{Yaml, YamlError, YamlSource};

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub use self::pdf::{Pdf, PdfConfig, PdfError, PdfMarkdown, PdfToc};

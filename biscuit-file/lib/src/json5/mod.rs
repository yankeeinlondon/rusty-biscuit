//! JSON5 parsing and conversion.
//!
//! This module provides the [`Json5`] struct for working with JSON5 files,
//! including conversion to JSON, YAML, and TOML formats.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use biscuit_file::Json5;
//!
//! // Parse from file
//! let json5 = Json5::new("config.json5")?;
//!
//! // Convert to other formats
//! let json = json5.as_json()?;
//! let yaml = json5.as_yaml()?;
//! let toml = json5.as_toml()?;
//! ```

mod format;
mod types;

pub use format::{to_json5_compact, to_json5_pretty};
pub use types::{Json5, Json5Error, Json5Source};

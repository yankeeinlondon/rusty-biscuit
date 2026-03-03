//! JSON5 parsing and conversion.
//!
//! This module provides the [`Json5`] struct for working with JSON5 files,
//! including conversion to JSON, YAML, and TOML formats.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::Json5;
//!
//! let json5 = Json5::from_str(r#"{ name: "example", version: "1.0" }"#)?;
//!
//! // Convert to other formats
//! let json = json5.as_json()?;
//! let yaml = json5.as_yaml()?;
//! let toml = json5.as_toml()?;
//! # Ok::<(), biscuit_file::Json5Error>(())
//! ```

mod format;
mod types;

pub use format::{to_json5_compact, to_json5_pretty};
pub use types::{Json5, Json5Error, Json5Source};

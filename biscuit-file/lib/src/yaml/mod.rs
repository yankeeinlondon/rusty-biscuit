//! YAML parsing, conversion, and validation.
//!
//! This module provides the [`Yaml`] struct for working with YAML files,
//! including conversion to JSON and TOML formats, and optional schema validation.
//!
//! ## Conversion Policies
//!
//! YAML conversion is inherently lossy. This module provides configurable policies
//! for handling edge cases:
//!
//! - Non-string map keys (JSON/TOML don't support them)
//! - Null values (TOML has no null type)
//! - Heterogeneous arrays (TOML requires homogeneous arrays)
//! - Non-finite floats like NaN and Infinity (JSON doesn't support them)
//!
//! ## Examples
//!
//! ```rust,ignore
//! use biscuit_file::Yaml;
//!
//! // Parse from file
//! let yaml = Yaml::new("config.yaml")?;
//!
//! // Convert to other formats
//! let json = yaml.as_json()?;
//! let toml = yaml.as_toml()?;
//! ```

mod types;

pub use types::{
    ConversionOutput, ConversionWarning, HeteroArrayPolicy, JsonConversionOptions,
    NonFiniteFloatPolicy, NonStringKeyPolicy, NullPolicy, TomlConversionOptions, Yaml, YamlError,
    YamlSource,
};

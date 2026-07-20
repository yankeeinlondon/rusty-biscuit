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
//! ## Diagnostics and Repairs
//!
//! [`Yaml::analyze`] is the entry point for source-first diagnostics. It
//! yields one [`YamlAnalysis`] that carries the diagnostics, the candidate
//! repairs, and the patched source, so a caller wanting more than one of
//! those views scans the document once. [`Yaml::diagnose`] and
//! [`Yaml::repair_candidates`] remain as single-view shorthand, but each
//! runs its own scan — calling both analyzes the same source twice.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::Yaml;
//!
//! let yaml = Yaml::from_str("name: example\nversion: '1.0'")?;
//!
//! // Convert to other formats
//! let json = yaml.as_json()?;
//! let toml = yaml.as_toml()?;
//! # Ok::<(), biscuit_file::YamlError>(())
//! ```
//!
//! ```rust
//! use biscuit_file::Yaml;
//!
//! let yaml = Yaml::from_str("name: example  \n")?;
//! if let Some(analysis) = yaml.analyze() {
//!     for diagnostic in analysis.diagnostics() {
//!         println!("{}: {}", diagnostic.code, diagnostic.message);
//!     }
//!     let cleaned = analysis.apply().source;
//!     assert_eq!(cleaned, "name: example\n");
//! }
//! # Ok::<(), biscuit_file::YamlError>(())
//! ```

mod analyze;
mod location;
mod types;

pub use analyze::{
    EditAudit, EditRejection, EditSetOutcome, RejectedEdit, YamlAnalysis, YamlCertainty,
    YamlDiagnostic, YamlDiagnosticCode, YamlParseFailure, YamlParseOutcome, YamlPathSegment,
    YamlRepair, YamlValueLocation, analyze_parse_count, analyze_yaml, apply_edit_set,
    locate_yaml_key, locate_yaml_value, reset_analyze_parse_count,
};
pub use location::YamlLocation;
pub use types::{
    ConversionOutput, ConversionWarning, HeteroArrayPolicy, JsonConversionOptions,
    NonFiniteFloatPolicy, NonStringKeyPolicy, NullPolicy, TomlConversionOptions, Yaml, YamlError,
    YamlSource,
};

#[cfg(test)]
mod tests;

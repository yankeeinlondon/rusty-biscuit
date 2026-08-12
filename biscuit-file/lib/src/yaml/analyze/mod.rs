//! Source-first YAML analysis.
//!
//! This module holds the shared diagnostic vocabulary ([`YamlDiagnostic`],
//! [`YamlRepair`], [`YamlCertainty`], [`YamlDiagnosticCode`]), the top-level
//! analysis result ([`YamlAnalysis`]), the shared edit-set utility
//! ([`apply_edit_set`]), and the analyzer entry point [`analyze_yaml`],
//! which produces [`YamlAnalysis`] values from raw source for both parseable
//! and unparseable input.

mod analysis;
mod diagnostic;
mod edit_set;
mod engine;
mod locate;
mod recover;
mod report;
mod scan;

pub use analysis::{YamlAnalysis, YamlParseFailure, YamlParseOutcome};
pub use diagnostic::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair};
pub use edit_set::{EditAudit, EditRejection, EditSetOutcome, RejectedEdit, apply_edit_set};
pub use engine::{analyze_parse_count, analyze_yaml, reset_analyze_parse_count};
pub use locate::{YamlPathSegment, YamlValueLocation, locate_yaml_key, locate_yaml_value};

#[cfg(test)]
mod tests;

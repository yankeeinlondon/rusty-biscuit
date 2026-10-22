//! Phase 1 input-robustness matrix of `2026-10-28-schema-enhancements`: the
//! cells existing code already decides, each walked from an unedited positive
//! control captured from a real producer (see
//! `tests/fixtures/schema_enhancements_matrix/PROVENANCE.md`) with exactly one
//! edit per cell, and asserted through the public schema validation or
//! composition result.

mod envelope;
mod frontmatter;
mod raw_schema;

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::{
    Markdown,
    compose::ComposeSource,
    schemas::{DarkmatterSchemas, SchemaError, ValidationReport},
};

/// Validates `frontmatter` (YAML lines without fences) as `document.md` in
/// `dir` through the public schema API, with no baseline configured.
fn validate_document(dir: &Path, frontmatter: &str) -> Result<ValidationReport, SchemaError> {
    let markdown = Markdown::from(format!("---\n{frontmatter}\n---\nBody\n"))
        .with_source(ComposeSource::File(dir.join("document.md")));
    DarkmatterSchemas::new(FileResolutionContext::new(dir)).validate(&markdown)
}

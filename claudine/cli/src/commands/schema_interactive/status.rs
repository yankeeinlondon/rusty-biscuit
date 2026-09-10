//! Status-report rendering for the schema-required-property flow.
//!
//! Renders the per-property status table (required/optional, valid/missing/
//! invalid) to stderr. Tests should assert on the structured
//! [`SchemaStatusReport`] returned by the library rather than the rendered
//! terminal output.

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{SchemaStatusReport, schema_status_report_prose};

pub(super) use claudine::composition::escape_schema_prose as escape_prose;
#[cfg(test)]
pub(super) use claudine::composition::{
    description_suffix, render_optional_line, render_required_line,
};

use crate::log;

/// Render the schema status report to stderr using `biscuit-terminal::Prose`.
///
/// The report shows every required and optional property in declaration
/// order, with state-specific glyphs:
///
/// - required + valid:   `<green>✓</green>`
/// - required + missing: `<red>⍉</red>`
/// - required + invalid: `!`
/// - required + deferred (inline launch only): `<blue>…</blue>`
/// - optional + valid:   `<green>✓</green>` (dim)
/// - optional + missing: `<grey>⍉</grey>` (dim)
/// - optional + invalid: `<yellow>!</yellow>` (dim)
///
/// When at least one optional property has an invalid value, a trailing
/// note explains that the value will be dropped from the prompt context.
pub fn render_status_report(report: &SchemaStatusReport, term: &Terminal) {
    log::message(&schema_status_report_prose(report).render(term));
}

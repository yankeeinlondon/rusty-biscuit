//! Findings collected across a document tree without stopping at the first.
//!
//! A *finding* is the shared [`Diagnostic`] record that `md --output json`,
//! DMLS, and Claudine already exchange. This module adds what a passive check
//! needs on top of it:
//!
//! - [`Trail`]: how a problem was reached, outermost first. A host (Claudine)
//!   pushes its own constructs as [`TrailFrame::Host`] frames, beside the
//!   transclusion and `::map` / `::filter` frames Darkmatter records.
//! - [`FindingSink`]: an ordered collector grouped by *subject* (a document
//!   for `md`, a task occurrence for Claudine), deduplicating equivalent
//!   findings and recording every check that could not run
//!   ([`NOT_CHECKED_CODE`]) instead of staying silent.
//! - [`FindingReport`]: the finished, ordered report, with severity counts,
//!   the data form, and the human rendering.
//! - [`reference`]: the builder of a confirmed missing-document finding over
//!   biscuit-file's detailed resolution.
//!
//! Severity belongs to the finding; a caller's blocking or exit policy reads
//! it and never changes it. See `darkmatter/docs/errors/message-standard.md`
//! for the four-part message standard every new finding follows.

pub mod reference;
mod report;
mod sink;
mod trail;

pub use crate::markdown::compose::expression::diagnostics::{
    Diagnostic, DiagnosticRelated, DiagnosticSource, ReferenceDetails, Severity,
};
pub use report::{FindingCounts, FindingReport, HumanLayout, ReportEntry};
pub use sink::{FindingId, FindingSink, Subject, SubjectId};
pub use trail::{Trail, TrailFrame};

/// A check that could not run for a subject, and why. Always `info`.
pub const NOT_CHECKED_CODE: &str = "dm.check.not_checked";

/// A document-valued host reference (a Claudine `prompt:`, `task:`,
/// `group:`, or `sequence:`) whose resolution confirmed no match.
pub const MISSING_DOCUMENT_CODE: &str = "dm.reference.missing_document";

/// A composed body that is proven empty.
pub const EMPTY_BODY_CODE: &str = "dm.compose.empty_body";

/// Every code this module's builders produce, with what it reports.
pub const FINDING_CODES: &[(&str, &str)] = &[
    (NOT_CHECKED_CODE, "a check that could not run, with the finding that blocked it"),
    (MISSING_DOCUMENT_CODE, "a document reference whose target does not exist"),
    (EMPTY_BODY_CODE, "a composed body that is proven empty"),
];

/// Why a composed body is empty, as far as the producer has proven.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EmptyBodyCause {
    /// The document has no body at all (a YAML sequence file, or Markdown
    /// with nothing after its frontmatter).
    AuthoredEmpty,
    /// The authored body holds only whitespace.
    Whitespace,
    /// The authored body has content, and the rendered result is empty; no
    /// narrower cause is proven.
    EmptyRendered,
}

impl EmptyBodyCause {
    /// The cause of an empty `rendered` body composed from `authored`, or
    /// `None` when `rendered` has visible content.
    ///
    /// Visible content is judged after embedded render metadata is
    /// stripped, the same predicate Claudine's launch uses.
    pub fn classify(authored: &str, rendered: &str) -> Option<Self> {
        if !crate::markdown::render_tree::embedded::without_embedded_metadata(rendered).trim().is_empty() {
            return None;
        }
        Some(if authored.is_empty() {
            Self::AuthoredEmpty
        } else if authored.trim().is_empty() {
            Self::Whitespace
        } else {
            Self::EmptyRendered
        })
    }

    /// The cause in the author's terms, completing "the body …".
    pub fn phrase(&self) -> &'static str {
        match self {
            Self::AuthoredEmpty => "has no body",
            Self::Whitespace => "contains only whitespace",
            Self::EmptyRendered => "rendered to nothing",
        }
    }

    /// The next step for this cause.
    pub fn hint(&self) -> &'static str {
        match self {
            Self::AuthoredEmpty | Self::Whitespace => "add the prompt text after the frontmatter",
            Self::EmptyRendered => "check the values the body's expressions and blocks read",
        }
    }
}

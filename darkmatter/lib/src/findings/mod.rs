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
    /// Every block of the body was excluded by a condition that evaluated
    /// false, and nothing outside those blocks had content.
    ExcludedBlocks {
        /// The excluded blocks, in document order.
        blocks: Vec<ExcludedBlock>,
    },
    /// The authored body has content, and the rendered result is empty; no
    /// narrower cause is proven.
    EmptyRendered,
}

/// A `::block` whose `when=` condition evaluated false, so its content was
/// left out: the fact an [`EmptyBodyCause::ExcludedBlocks`] is proven from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ExcludedBlock {
    /// The one-based line of the `::block` directive: in the document's file
    /// when the body's map proves it, otherwise in the composed body.
    pub line: usize,
    /// The condition as written.
    pub condition: String,
}

impl EmptyBodyCause {
    /// The cause of an empty `rendered` body composed from `authored`, or
    /// `None` when `rendered` has visible content.
    ///
    /// Visible content is judged after embedded render metadata is
    /// stripped, the same predicate Claudine's launch uses.
    pub fn classify(authored: &str, rendered: &str) -> Option<Self> {
        Self::classify_composed(authored, rendered, &[])
    }

    /// [`Self::classify`] for a composition that left out `excluded` blocks
    /// ([`ComposeReport::excluded_blocks`](crate::markdown::compose::ComposeReport::excluded_blocks)):
    /// an empty result of a body that had blocks left out is
    /// [`Self::ExcludedBlocks`].
    pub fn classify_composed(authored: &str, rendered: &str, excluded: &[ExcludedBlock]) -> Option<Self> {
        if !crate::markdown::render_tree::embedded::without_embedded_metadata(rendered).trim().is_empty() {
            return None;
        }
        Some(if authored.is_empty() {
            Self::AuthoredEmpty
        } else if authored.trim().is_empty() {
            Self::Whitespace
        } else if !excluded.is_empty() {
            Self::ExcludedBlocks { blocks: excluded.to_vec() }
        } else {
            Self::EmptyRendered
        })
    }

    /// The cause in the author's terms, completing "the body …".
    pub fn phrase(&self) -> String {
        match self {
            Self::AuthoredEmpty => "has no body".to_string(),
            Self::Whitespace => "contains only whitespace".to_string(),
            Self::ExcludedBlocks { blocks } => {
                let conditions: Vec<String> = blocks
                    .iter()
                    .map(|block| format!("`{}` (line {})", block.condition, block.line))
                    .collect();
                format!("rendered to nothing: every block was left out because its condition was false: {}", conditions.join(", "))
            }
            Self::EmptyRendered => "rendered to nothing".to_string(),
        }
    }

    /// The next step for this cause.
    pub fn hint(&self) -> &'static str {
        match self {
            Self::AuthoredEmpty | Self::Whitespace => "add the prompt text after the frontmatter",
            Self::ExcludedBlocks { .. } => {
                "pass the values those conditions read, or add body content outside the conditional blocks"
            }
            Self::EmptyRendered => "check the values the body's expressions and blocks read",
        }
    }
}

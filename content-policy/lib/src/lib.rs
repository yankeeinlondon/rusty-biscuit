//! Declare, evaluate, and renew content policies carried in Markdown
//! frontmatter.
//!
//! A policy is a list of rules under a frontmatter key (`content_policy` by
//! default). Evaluation reads an [`EvidenceRecord`], either a document's
//! frontmatter through the library's own [`reader`] or a map the caller
//! already holds, at an explicit evaluation time, and returns a [`Report`]
//! with a status, an effective action, and a result for every entry.
//!
//! See `content-policy/docs/topics/policy-lifecycle.md` for the behavior this
//! crate implements.

mod aggregate;
mod diagnostic;
mod evaluate;
mod grammar;
mod model;
mod normalized;
pub mod reader;
pub mod time;

pub use aggregate::{Aggregate, ResultKind, Status, aggregate};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, EntryField, Invalid, Location, Warning, WarningCode,
};
pub use evaluate::{
    DateEvidence, DateSource, DocumentError, EntryOutcome, EntryResult, EvaluationContext,
    NaiveDateText, PolicySource, PolicySummary, Report, UnknownReason, evaluate_document,
    evaluate_policy, evaluate_record,
};
pub use model::{
    Action, Baseline, Deadline, Duration, DurationUnit, EvidenceRecord, GRAMMAR_VERSION, Policy,
    PolicyEntry, PolicyOptions, Renewal, Rule,
};

//! Declare, evaluate, and renew content policies carried in Markdown
//! frontmatter.
//!
//! A policy is a list of rules under a frontmatter key (`content_policy` by
//! default). Evaluation reads an [`EvidenceRecord`], either a document's
//! frontmatter through the library's own [`reader`] or a map the caller
//! already holds, at an explicit evaluation time, and returns a [`Report`]
//! with a status, an effective action, and a result for every entry.
//!
//! Renewal ([`plan_renewal`], [`apply_renewal`]) records a content update: it
//! plans byte-exact edits that advance every renewable baseline, and applies
//! them only to the bytes it planned from.
//!
//! `FileChanged` rules observe their files through a [`FileProvider`] and a
//! base directory, set with [`EvaluationContext::with_files`]. The bundled
//! `FileAdapter`, behind the off-by-default `file-adapter` feature, reads the
//! local file system; without a provider a file rule is `unknown`.
//!
//! See `content-policy/docs/topics/policy-lifecycle.md` for the behavior this
//! crate implements.

mod aggregate;
mod diagnostic;
mod evaluate;
#[cfg(feature = "file-adapter")]
mod file_adapter;
mod fingerprint;
mod grammar;
mod model;
mod normalized;
mod path_form;
mod provider;
pub mod reader;
mod renew;
pub mod time;

pub use aggregate::{Aggregate, ResultKind, Status, aggregate};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, EntryField, Invalid, Location, Warning, WarningCode,
};
pub use evaluate::{
    DateEvidence, DateSource, DocumentError, EntryOutcome, EntryResult, EvaluationContext,
    FileEvidence, NaiveDateText, PolicySource, PolicySummary, Report, UnknownReason,
    evaluate_document, evaluate_policy, evaluate_record,
};
#[cfg(feature = "file-adapter")]
pub use file_adapter::FileAdapter;
pub use fingerprint::FingerprintScheme;
pub use model::{
    Action, Baseline, Deadline, Duration, DurationUnit, EvidenceRecord, GRAMMAR_VERSION, Policy,
    PolicyEntry, PolicyOptions, Renewal, Rule,
};
pub use provider::{FileObservation, FileProvider, FileRequest};
pub use renew::{
    BaselineChange, BaselineTarget, ChangeKind, Conflict, ConflictKind, EvidenceIssue,
    EvidenceIssueKind, Refusal, RefusalReason,
    RenewalContext, RenewalError, RenewalPlan, TextEdit, apply_renewal, plan_fingerprint,
    plan_renewal,
};

/// The editor schema, `content-policy/schemas/content-policy.yaml`, compiled
/// in so a caller can incorporate it into its own SimplifiedSchema.
///
/// It is a `kind: schema` file declaring the types `short_form` (one compact
/// rule), `long_form` (a `{rule, action}` entry), and `policy` (either form).
/// The schema checks a subset of the grammar; evaluation is the authority.
pub const EDITOR_SCHEMA: &str = include_str!("../../schemas/content-policy.yaml");

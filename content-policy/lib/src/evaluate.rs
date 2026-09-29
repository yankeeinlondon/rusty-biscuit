//! Evaluation: resolve baselines, evaluate every entry, and build a report.
//!
//! Evaluation reads evidence and never writes it: no API here takes anything
//! mutable, so checking a document can never capture or advance a baseline.

use std::fmt;

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::aggregate::{ResultKind, Status, aggregate};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Invalid, Location, Warning, WarningCode};
use crate::grammar::{self, RuleError};
use crate::model::{
    Action, Baseline, Deadline, EvidenceRecord, Policy, PolicyOptions, Renewal, Rule,
    serialize_display,
};
use crate::reader::{ReadError, ReadOutcome, read_frontmatter};
use crate::time::{add_duration, start_of_day};

/// Everything an evaluation needs besides the policy and evidence.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    at: DateTime<Utc>,
    options: PolicyOptions,
    document: Option<String>,
}

impl EvaluationContext {
    /// A context evaluating at `at` with the built-in options.
    #[must_use]
    pub fn new(at: DateTime<Utc>) -> Self {
        Self {
            at,
            options: PolicyOptions::default(),
            document: None,
        }
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn with_options(mut self, options: PolicyOptions) -> Self {
        self.options = options;
        self
    }

    /// Labels the report with the document identity as the caller spells
    /// it, such as the path it was given. It is never canonicalized.
    #[must_use]
    pub fn with_document(mut self, label: impl Into<String>) -> Self {
        self.document = Some(label.into());
        self
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn at(&self) -> DateTime<Utc> {
        self.at
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn options(&self) -> &PolicyOptions {
        &self.options
    }
}

/// Whether the evaluated policy was declared or is the caller's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    Declared,
    Defaulted,
}

/// The policy a report was evaluated under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicySummary {
    pub source: PolicySource,
    pub grammar_version: u32,
    pub identity: String,
}

/// Where a date in a rule came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DateSource {
    /// Written inside the rule.
    Inline,
    /// An `@name` reference.
    Property,
    /// The `ValidFor(<duration>)` shorthand's default date property.
    DefaultProperty,
}

/// A rule's baseline or deadline as resolved for the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DateEvidence {
    pub source: DateSource,
    /// The property read, or `None` for an inline date.
    pub property: Option<String>,
    /// The date, or `None` when the property is absent or `null`.
    pub value: Option<NaiveDateText>,
}

/// A date that serializes as `YYYY-MM-DD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct NaiveDateText(#[serde(serialize_with = "serialize_display")] pub NaiveDate);

/// Why an entry could not be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    /// The baseline or deadline property is absent or `null`.
    MissingBaseline,
    /// The baseline is later than the evaluation date.
    InconsistentBaseline,
}

/// One entry's result: `triggered`, `not_triggered`, or `unknown` with a
/// typed reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryOutcome {
    Triggered,
    NotTriggered,
    Unknown(UnknownReason),
}

impl EntryOutcome {
    #[allow(missing_docs)]
    #[must_use]
    pub fn kind(self) -> ResultKind {
        match self {
            Self::Triggered => ResultKind::Triggered,
            Self::NotTriggered => ResultKind::NotTriggered,
            Self::Unknown(_) => ResultKind::Unknown,
        }
    }
}

/// Serializes as `"result"` plus `"unknown_reason"` (`null` unless unknown),
/// flattened into [`EntryResult`].
impl Serialize for EntryOutcome {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("result", &self.kind())?;
        let reason = match self {
            Self::Unknown(reason) => Some(reason),
            _ => None,
        };
        map.serialize_entry("unknown_reason", &reason)?;
        map.end()
    }
}

/// The evaluation of one policy entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryResult {
    /// Zero-based position in the policy list.
    pub index: usize,
    /// The rule in canonical compact form.
    pub rule: String,
    pub action: Action,
    pub renewal: Renewal,
    #[serde(flatten)]
    pub outcome: EntryOutcome,
    /// `ValidFor`'s starting date.
    pub baseline: Option<DateEvidence>,
    /// `ValidUntil`'s deadline.
    pub deadline: Option<DateEvidence>,
    /// The date the rule triggers from (00:00 UTC), when it could be computed.
    pub due: Option<NaiveDateText>,
    pub reason: String,
}

/// A complete evaluation. Its existence means every entry was valid, so it
/// always carries a verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub document: Option<String>,
    #[serde(serialize_with = "serialize_instant")]
    pub evaluated_at: DateTime<Utc>,
    pub policy: PolicySummary,
    pub status: Status,
    pub action: Option<Action>,
    pub evaluation_complete: bool,
    pub action_resolution_complete: bool,
    pub results: Vec<EntryResult>,
    pub warnings: Vec<Warning>,
}

impl Report {
    /// Serializes the report as pretty-printed JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("reports always serialize")
    }
}

fn serialize_instant<S: Serializer>(at: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&at.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// Why a document could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    /// The frontmatter could not be read.
    Read { document: Option<String>, error: ReadError },
    /// The declaration or its evidence is invalid; no verdict.
    Invalid(Invalid),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { document, error } => match document {
                Some(document) => write!(f, "{document}: {error}"),
                None => write!(f, "{error}"),
            },
            Self::Invalid(invalid) => write!(f, "{invalid}"),
        }
    }
}

impl std::error::Error for DocumentError {}

/// Evaluates the policy an evidence record declares under the configured
/// key, or the default policy when the key is absent. No document is
/// involved: a cache manifest or Darkmatter's parsed frontmatter works as is.
///
/// ## Errors
///
/// Returns [`Invalid`], listing every invalid entry, when the declaration or
/// a resolved evidence value is invalid.
pub fn evaluate_record(
    record: &EvidenceRecord,
    context: &EvaluationContext,
) -> Result<Report, Invalid> {
    let (policy, source) = match record.get(context.options.key()) {
        None => (context.options.default_policy().clone(), PolicySource::Defaulted),
        Some(declaration) => (
            Policy::from_declaration(declaration).map_err(|invalid| label(invalid, context))?,
            PolicySource::Declared,
        ),
    };
    evaluate_with_source(&policy, source, record, context)
}

/// Evaluates an explicit policy against an evidence record.
///
/// ## Errors
///
/// Returns [`Invalid`] when a resolved evidence value is invalid.
pub fn evaluate_policy(
    policy: &Policy,
    record: &EvidenceRecord,
    context: &EvaluationContext,
) -> Result<Report, Invalid> {
    evaluate_with_source(policy, PolicySource::Declared, record, context)
}

/// Reads a Markdown document's frontmatter with the library's reader and
/// evaluates it. A document without frontmatter uses the default policy.
///
/// ## Errors
///
/// Returns [`DocumentError::Read`] for unreadable frontmatter, and
/// [`DocumentError::Invalid`] for an invalid declaration or evidence value,
/// a duplicate key included.
pub fn evaluate_document(
    bytes: &[u8],
    context: &EvaluationContext,
) -> Result<Report, DocumentError> {
    let (record, warnings) = match read_frontmatter(bytes) {
        Ok(ReadOutcome::NoFrontmatter) => (EvidenceRecord::new(), Vec::new()),
        Ok(ReadOutcome::Found(frontmatter)) => {
            let warnings = if frontmatter.tab_repair().is_empty() {
                Vec::new()
            } else {
                vec![Warning {
                    code: WarningCode::TabIndentationRepaired,
                    message: format!(
                        "the frontmatter is indented with tabs, which YAML forbids; it was read \
                         after replacing each indentation tab with two spaces ({} line(s)); \
                         `policy renew` offers the same repair as an edit",
                        frontmatter.tab_repair().len()
                    ),
                }]
            };
            (frontmatter.record().clone(), warnings)
        }
        Err(ReadError::DuplicateKey { key, message }) => {
            return Err(DocumentError::Invalid(duplicate_key_invalid(
                key,
                &message,
                context.document.clone(),
            )));
        }
        Err(error) => {
            return Err(DocumentError::Read {
                document: context.document.clone(),
                error,
            });
        }
    };
    match evaluate_record(&record, context) {
        Ok(mut report) => {
            report.warnings = warnings;
            Ok(report)
        }
        Err(mut invalid) => {
            invalid.warnings = warnings;
            Err(DocumentError::Invalid(invalid))
        }
    }
}

/// The validation error for a [`ReadError::DuplicateKey`], shared by
/// evaluation and renewal.
pub(crate) fn duplicate_key_invalid(
    key: Option<String>,
    message: &str,
    document: Option<String>,
) -> Invalid {
    let location = match key {
        Some(name) => Location::Property { name, entry: None },
        None => Location::Frontmatter,
    };
    let mut invalid = Invalid::new(vec![Diagnostic::new(
        DiagnosticCode::DuplicateKey,
        location,
        format!("a key is defined twice, and YAML tools disagree about which copy wins: {message}"),
    )]);
    invalid.document = document;
    invalid
}

fn label(mut invalid: Invalid, context: &EvaluationContext) -> Invalid {
    invalid.document.clone_from(&context.document);
    invalid
}

fn evaluate_with_source(
    policy: &Policy,
    source: PolicySource,
    record: &EvidenceRecord,
    context: &EvaluationContext,
) -> Result<Report, Invalid> {
    let mut diagnostics = Vec::new();
    let mut results = Vec::with_capacity(policy.entries().len());
    for (index, entry) in policy.entries().iter().enumerate() {
        match evaluate_entry(index, &entry.rule, record, context) {
            Ok(evaluated) => results.push(EntryResult {
                index,
                rule: entry.rule.to_string(),
                action: entry.action,
                renewal: entry.rule.renewal(),
                outcome: evaluated.outcome,
                baseline: evaluated.baseline,
                deadline: evaluated.deadline,
                due: evaluated.due.map(NaiveDateText),
                reason: evaluated.reason,
            }),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    if !diagnostics.is_empty() {
        return Err(label(Invalid::new(diagnostics), context));
    }
    let verdict = aggregate(results.iter().map(|result| (result.action, result.outcome.kind())));
    Ok(Report {
        document: context.document.clone(),
        evaluated_at: context.at,
        policy: PolicySummary {
            source,
            grammar_version: policy.grammar_version(),
            identity: policy.identity(),
        },
        status: verdict.status,
        action: verdict.action,
        evaluation_complete: verdict.evaluation_complete,
        action_resolution_complete: verdict.action_resolution_complete,
        results,
        warnings: Vec::new(),
    })
}

struct Evaluated {
    outcome: EntryOutcome,
    baseline: Option<DateEvidence>,
    deadline: Option<DateEvidence>,
    due: Option<NaiveDate>,
    reason: String,
}

impl Evaluated {
    fn plain(outcome: EntryOutcome, reason: &str) -> Self {
        Self {
            outcome,
            baseline: None,
            deadline: None,
            due: None,
            reason: reason.to_string(),
        }
    }
}

/// Resolves a date position: `Ok(None)` for missing evidence (absent or
/// `null`), a diagnostic for a present value that is not a date.
fn resolve_property(
    index: usize,
    name: &str,
    record: &EvidenceRecord,
) -> Result<Option<NaiveDate>, Diagnostic> {
    let location = Location::Property {
        name: name.to_string(),
        entry: Some(index),
    };
    match record.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => grammar::date_error(text)
            .map(Some)
            .map_err(|error| Diagnostic::new(error.code, location, error.message)),
        Some(other) => Err(Diagnostic::new(
            DiagnosticCode::WrongType,
            location,
            format!("expected a `YYYY-MM-DD` date string, found {}", grammar::type_name(other)),
        )),
    }
}

fn evaluate_entry(
    index: usize,
    rule: &Rule,
    record: &EvidenceRecord,
    context: &EvaluationContext,
) -> Result<Evaluated, Diagnostic> {
    let at = context.at;
    match rule {
        Rule::Evergreen => Ok(Evaluated::plain(EntryOutcome::NotTriggered, "Never expires")),
        Rule::TimeSensitive => Ok(Evaluated::plain(
            EntryOutcome::Triggered,
            "Time-sensitive content always needs a refresh",
        )),
        Rule::ValidFor { duration, baseline } => {
            let (source, property, date) = match baseline {
                Baseline::Inline(date) => (DateSource::Inline, None, Some(*date)),
                Baseline::Reference(name) => (
                    DateSource::Property,
                    Some(name.clone()),
                    resolve_property(index, name, record)?,
                ),
                Baseline::Defaulted => {
                    let name = context.options.date_property();
                    (
                        DateSource::DefaultProperty,
                        Some(name.to_string()),
                        resolve_property(index, name, record)?,
                    )
                }
            };
            let evidence = DateEvidence {
                source,
                property,
                value: date.map(NaiveDateText),
            };
            let Some(date) = date else {
                return Ok(Evaluated {
                    baseline: Some(evidence),
                    ..Evaluated::plain(
                        EntryOutcome::Unknown(UnknownReason::MissingBaseline),
                        "No baseline date is recorded",
                    )
                });
            };
            let due = add_duration(date, *duration).ok_or_else(|| {
                let RuleError { code, message } = grammar::out_of_range(&rule.to_string());
                Diagnostic::new(code, Location::Entry { index }, message)
            })?;
            let (outcome, reason) = if start_of_day(date) > at {
                (
                    EntryOutcome::Unknown(UnknownReason::InconsistentBaseline),
                    "The baseline date is later than the evaluation date",
                )
            } else if at >= start_of_day(due) {
                (EntryOutcome::Triggered, "Validity interval elapsed")
            } else {
                (EntryOutcome::NotTriggered, "Within its validity interval")
            };
            Ok(Evaluated {
                outcome,
                baseline: Some(evidence),
                deadline: None,
                due: Some(due),
                reason: reason.to_string(),
            })
        }
        Rule::ValidUntil { deadline } => {
            let (source, property, date) = match deadline {
                Deadline::Inline(date) => (DateSource::Inline, None, Some(*date)),
                Deadline::Reference(name) => (
                    DateSource::Property,
                    Some(name.clone()),
                    resolve_property(index, name, record)?,
                ),
            };
            let evidence = DateEvidence {
                source,
                property,
                value: date.map(NaiveDateText),
            };
            let Some(date) = date else {
                return Ok(Evaluated {
                    deadline: Some(evidence),
                    ..Evaluated::plain(
                        EntryOutcome::Unknown(UnknownReason::MissingBaseline),
                        "No deadline date is recorded",
                    )
                });
            };
            let (outcome, reason) = if at >= start_of_day(date) {
                (EntryOutcome::Triggered, "Fixed deadline reached")
            } else {
                (EntryOutcome::NotTriggered, "Before its fixed deadline")
            };
            Ok(Evaluated {
                outcome,
                baseline: None,
                deadline: Some(evidence),
                due: Some(date),
                reason: reason.to_string(),
            })
        }
    }
}

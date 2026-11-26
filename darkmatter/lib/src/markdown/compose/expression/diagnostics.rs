//! The shared expression diagnostics: one record per finding, with its code,
//! severity, message, authored position, and (for an argument) expected and
//! actual type. Composition, the `md` CLI, and DMLS all take these records from
//! here and only adapt them (an editor range, a warning row); none re-derives a
//! code, a severity, or a message.
//!
//! Two shapes exist:
//!
//! - [`ExpressionDiagnostic`] is a finding of one analyzed expression, ranged
//!   in that expression's text. [`expression_call_diagnostics`] and
//!   [`CallCheck::diagnostics`] produce it.
//! - [`Diagnostic`] is a finding located in its authored file, with one-based
//!   line and character column ([`ExpressionDiagnostic::locate`]), and the
//!   record every family reports through. An editor translates the position to
//!   its own coordinates.
//!
//! Severity is a property of the finding, not of a command's policy: a
//! blocking policy changes a command's outcome, never a diagnostic's code or
//! severity. Producing a diagnostic reads only the analysis result, so it
//! never evaluates an expression, reads a value, or touches the filesystem,
//! and no record holds an evaluated value.

use std::path::{Path, PathBuf};

use serde::Serialize;

use simplified_schema::expression::ast::SpannedExpr;
use super::call_check::{ALWAYS_FALSE_CALL_CODE, ArgumentCategory, CallCheck, check_expression_calls};
use super::iteration::IterationPath;
use super::host::{INCOMPLETE_ANALYSIS_CODE, IncompleteHostCall, incomplete_host_calls};
use super::scope::TypeScope;
use crate::markdown::reference::types::ReferenceTrailStep;
use crate::markdown::span::SourceSpan;

/// A call the design-time check proves wrong: argument category 5, a wrong
/// argument count, or an ambiguous overload. Always an error; composition
/// rejects the document. A `load_data` contract the check proves wrong uses
/// its `dm.data.*` code instead ([`CallCheck::data_problem`]).
pub const INCOMPATIBLE_CALL_CODE: &str = "dm.expression.incompatible_call";

/// A call argument that might not bind: category 3, and under
/// `expressions.strict` also categories 2 (may be `null`) and 4 (type
/// unknown). Always a warning.
pub const UNCERTAIN_CALL_CODE: &str = "dm.expression.uncertain_call";

/// A member read the known object type of its base does not declare:
/// every non-null alternative is a closed object without that name, and the
/// read does not handle its absence. Always a warning.
pub const UNDECLARED_MEMBER_CODE: &str = "dm.expression.undeclared_member";

/// How serious a diagnostic is, independent of any blocking policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// The lowercase name a report writes (`"error"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// The severity of an argument category: 5 is always an error, 3 always a
/// warning, 2 and 4 a warning only under `strict` (DMLS's
/// `expressions.strict`), and 1 never shown. Composition is never strict.
pub fn call_severity(category: ArgumentCategory, strict: bool) -> Option<Severity> {
    match category {
        ArgumentCategory::Incompatible => Some(Severity::Error),
        ArgumentCategory::Uncertain => Some(Severity::Warning),
        ArgumentCategory::NullableMatch | ArgumentCategory::Unknown if strict => Some(Severity::Warning),
        ArgumentCategory::Compatible | ArgumentCategory::NullableMatch | ArgumentCategory::Unknown => None,
    }
}

/// One finding of an analyzed expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionDiagnostic {
    pub code: &'static str,
    pub severity: Severity,
    /// Names the function and, for an argument, the parameter and both types.
    pub message: String,
    /// The argument's span, or the call's for a call-level problem, in the
    /// analyzed expression's text.
    pub span: SourceSpan,
    /// The parameter's declared type as written, for an argument finding.
    pub expected_type: Option<String>,
    /// The argument's design-time type, for an argument finding.
    pub actual_type: Option<String>,
}

impl ExpressionDiagnostic {
    /// This finding located in `file` at the one-based `position` of its
    /// span's start.
    pub fn at(&self, file: Option<PathBuf>, position: Option<(usize, usize)>) -> Diagnostic {
        Diagnostic {
            code: self.code.to_string(),
            severity: self.severity,
            message: self.message.clone(),
            source: position.map(|(line, column)| DiagnosticSource::at(file, line, column)),
            expected_type: self.expected_type.clone(),
            actual_type: self.actual_type.clone(),
            span: None,
            reference_details: None,
            data_location: None,
            grants_needed: None,
            schema_default: None,
            related: Vec::new(),
            hint: None,
            notes: Vec::new(),
            trail: crate::findings::Trail::default(),
        }
    }

    /// This finding located where its span starts when the analyzed text
    /// begins at byte `offset` of `text`, the authored file's content.
    pub fn locate(&self, file: Option<PathBuf>, text: &str, offset: usize) -> Diagnostic {
        self.at(file, one_based_position(text, offset + self.span.start))
    }
}

/// A finding located in its authored source.
///
/// Serializes with `iteration_path` and `included_via` projected from
/// [`Self::trail`], which is the only stored copy of that context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    /// Where the authored range starts; `None` when no surface located it.
    pub source: Option<DiagnosticSource>,
    pub expected_type: Option<String>,
    pub actual_type: Option<String>,
    /// The byte range of the authored construct in `source.file`, when the
    /// producer proved one; `None` otherwise.
    pub span: Option<std::ops::Range<usize>>,
    /// What a reference finding adds, written beside the fields above; `None`
    /// for every other family.
    pub reference_details: Option<ReferenceDetails>,
    /// Where in a loaded data source the problem is, for a
    /// `dm.data.malformed` or `dm.data.contract_mismatch` record; `None`
    /// (and not serialized) for every other record.
    pub data_location: Option<DataLocation>,
    /// The approvals any one of which would allow the read a
    /// `dm.schema.slot_approval_required` record is about, in the order an
    /// approval prompt offers them; `None` (and not serialized) for every
    /// other record.
    pub grants_needed: Option<Vec<GrantNeeded>>,
    /// The schema default that supplied the value a schema problem is
    /// about: no authored text holds that value, so `source` names the
    /// document without a line or column. `None` (and not serialized) for
    /// every other record.
    pub schema_default: Option<SchemaDefaultOrigin>,
    /// Other locations involved in the finding: the other declarations of a
    /// conflict, or the finding that blocked a check (`dm.check.not_checked`).
    pub related: Vec<DiagnosticRelated>,
    /// The next step that fixes the problem; required for a new actionable
    /// finding ([`Diagnostic::actionable`]) and optional for a projection of
    /// an older one.
    pub hint: Option<String>,
    /// Facts the producer established that support the message, one per
    /// line (such as each link of a provider chain), shown under it. Unlike
    /// [`Self::related`], a note has no location of its own.
    pub notes: Vec<String>,
    /// How the problem was reached, outermost first: host constructs (a
    /// Claudine task), transclusions, and `::map` / `::filter` items.
    ///
    /// Serialized as `trail` (every frame) only when it holds a host frame;
    /// its iteration and included frames are always serialized as
    /// `iteration_path` and `included_via`, so a trail of Darkmatter frames
    /// alone adds no field.
    pub trail: crate::findings::Trail,
}

impl Diagnostic {
    /// A new actionable finding: what failed and why in `message`, the next
    /// step in `hint`. Locate it with [`Self::with_source`] and give its
    /// context with [`Self::with_trail`].
    pub fn actionable(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::projected(code, severity, message, Some(hint.into()))
    }

    /// A finding projected from an older diagnostic, whose hint is optional.
    pub fn projected(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            source: None,
            expected_type: None,
            actual_type: None,
            span: None,
            reference_details: None,
            data_location: None,
            grants_needed: None,
            schema_default: None,
            related: Vec::new(),
            hint,
            notes: Vec::new(),
            trail: crate::findings::Trail::default(),
        }
    }

    /// This finding located at `source`.
    #[must_use]
    pub fn with_source(mut self, source: DiagnosticSource) -> Self {
        self.source = Some(source);
        self
    }

    /// This finding with the authored byte range `span` of `source.file`.
    #[must_use]
    pub fn with_span(mut self, span: std::ops::Range<usize>) -> Self {
        self.span = Some(span);
        self
    }

    /// This finding reached through `trail`: its frames go outside the ones
    /// the finding already carries (a host's construct encloses the
    /// transclusions and `::map` / `::filter` items Darkmatter recorded).
    #[must_use]
    pub fn with_trail(mut self, trail: crate::findings::Trail) -> Self {
        self.trail = std::mem::take(&mut self.trail).within(trail);
        self
    }

    /// The iteration frames of [`Self::trail`]: the original positions of the
    /// items being processed when a composition failed, outermost first.
    /// Empty for a static finding, a warning, or a failure outside every
    /// `::map` / `::filter` body.
    pub fn iteration_path(&self) -> IterationPath {
        self.trail.iteration_path()
    }

    /// Replaces the iteration frames of [`Self::trail`] with `path`'s.
    pub fn set_iteration_path(&mut self, path: &IterationPath) {
        self.trail = std::mem::take(&mut self.trail).with_iteration(path);
    }

    /// This finding with its iteration frames replaced by `path`'s.
    #[must_use]
    pub fn with_iteration_path(mut self, path: &IterationPath) -> Self {
        self.set_iteration_path(path);
        self
    }

    /// The inclusion path to a reference finding's document, outermost
    /// first, from the included frames of [`Self::trail`]: empty for the
    /// root document, `None` for a record that reports no inclusion path
    /// ([`ReferenceDetails::reports_inclusion`]).
    pub fn included_via(&self) -> Option<Vec<ReferenceTrailStep>> {
        self.reference_details
            .as_ref()
            .filter(|details| details.reports_inclusion)
            .map(|_| self.trail.included_steps())
    }

    /// This finding with one more related location.
    #[must_use]
    pub fn with_related(mut self, related: DiagnosticRelated) -> Self {
        self.related.push(related);
        self
    }

    /// This finding with one more supporting line.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// This finding with the reference it is about.
    #[must_use]
    pub fn with_reference_details(mut self, details: ReferenceDetails) -> Self {
        self.reference_details = Some(details);
        self
    }
}

/// A secondary location of a diagnostic, with its own authored source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticRelated {
    /// What the location contributes to the diagnostic.
    pub message: String,
    /// The file and position of the related declaration.
    pub source: DiagnosticSource,
}

/// Where a schema default that composition or validation inserted is
/// declared, as a data output writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaDefaultOrigin {
    /// The dotted property path the default stands in for.
    pub path: String,
    /// JSON Pointer of the `default` keyword in the lowered schema, such as
    /// `/properties/doc/default`.
    pub schema_location: String,
    /// The schema file that declares the default (the imported file for a
    /// default inside a `Name@file` type); omitted when the document's own
    /// inline schema or a compiled-in schema declares it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_file: Option<PathBuf>,
}

impl From<&simplified_schema::AppliedDefault> for SchemaDefaultOrigin {
    fn from(applied: &simplified_schema::AppliedDefault) -> Self {
        Self {
            path: applied.path.to_string(),
            schema_location: applied.pointer.clone(),
            schema_file: applied.origin.as_ref().and_then(simplified_schema::SchemaOrigin::file).map(Path::to_path_buf),
        }
    }
}

/// One approval that would allow a schema slot's read, as a data output
/// writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrantNeeded {
    /// `file` or `url`.
    pub kind: &'static str,
    /// The file's path or the exact URL the approval covers.
    pub target: String,
    /// `once` (this run) or `persisted` ("always allow" a file: saved to the
    /// user's `permissions.file-read` and applied for the run).
    pub scope: &'static str,
}

impl GrantNeeded {
    /// The data-output form of `grant`.
    pub fn of(grant: &crate::markdown::schemas::slots::grants::Grant) -> Self {
        use crate::markdown::schemas::slots::grants::GrantScope;
        Self {
            kind: grant.target.kind_word(),
            target: grant.target.to_string(),
            scope: match grant.scope {
                GrantScope::Once => "once",
                GrantScope::Persisted => "persisted",
            },
        }
    }
}

/// A position inside a data source read by `load_data`, as opposed to the
/// call site in [`Diagnostic::source`].
///
/// Every field other than `file` is a fact a parser or locator proved; an
/// unknown one is `None` and omitted from output, never a placeholder (a
/// pointer is never `""` to mean "unknown"). `line` and `column` are
/// 1-based and count characters of the BOM-stripped text, with LF or CRLF
/// as the only line breaks. `byte_offset` is set only for local text that is
/// not valid UTF-8; it is zero-based into the original bytes, and `line` and
/// `column` are then absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataLocation {
    /// The resolved data file as a portable path, or the normalized URL.
    pub file: String,
    /// RFC 6901 pointer to the offending value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<usize>,
}

/// The start of a diagnostic's authored range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticSource {
    /// The authored file, or a remote document's URL; `None` for input with no
    /// named origin.
    pub file: Option<PathBuf>,
    /// One-based authored line; `None` when no position in `file` is proven,
    /// such as a document built in memory or a line counted in rewritten text.
    /// Never a placeholder number.
    pub line: Option<usize>,
    /// One-based authored column, in characters; `None` as for `line`.
    pub column: Option<usize>,
    /// The composed anchor slug of the innermost heading at or before the
    /// problem; omitted above the first heading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// An expression, a shell step, or a `replace:` value wrote the problem's
    /// text, so `line` and `column` name that construct.
    pub inserted: bool,
}

impl DiagnosticSource {
    /// An authored position with no section, in text the document wrote.
    pub fn at(file: Option<PathBuf>, line: usize, column: usize) -> Self {
        Self { file, line: Some(line), column: Some(column), section: None, inserted: false }
    }

    /// `file` when nothing narrower is known.
    pub fn file_only(file: Option<PathBuf>) -> Self {
        Self { file, line: None, column: None, section: None, inserted: false }
    }
}

/// The reference a [`Diagnostic`] is about. Each field is omitted when absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceDetails {
    /// The reason phrase, such as `file not found`.
    pub reason: Option<String>,
    /// The construct phrase, such as `hyperlink` or `::code`.
    pub construct: Option<String>,
    /// The reference as written.
    pub reference: Option<String>,
    /// The file-reference failure class, such as `no-match`.
    pub failure: Option<String>,
    /// Whether the record reports the inclusion path to its reference's
    /// document as `included_via` (the diagnostic's included trail frames,
    /// `[]` for the root document); `false` for a failure with no one
    /// reference, which omits the field.
    pub reports_inclusion: bool,
    /// The directory a relative reference was resolved against, as a
    /// portable path.
    pub base: Option<String>,
    /// The candidate the reference resolved to when resolution proposed one
    /// (an explicit relative reference's joined path), as a portable path.
    pub resolved: Option<String>,
    /// Every location the resolver actually checked, in its search order.
    pub searched: Option<Vec<String>>,
    /// Nearby files that might be the intended target, at most three; never
    /// selected automatically.
    pub suggestions: Option<Vec<String>>,
}


/// The serialized form of a [`Diagnostic`]: the stored fields in their
/// published order, with `iteration_path` and `included_via` projected from
/// the trail.
#[derive(Serialize)]
struct DiagnosticWire<'a> {
    code: &'a str,
    severity: Severity,
    message: &'a str,
    source: Option<&'a DiagnosticSource>,
    expected_type: Option<&'a str>,
    actual_type: Option<&'a str>,
    span: Option<&'a std::ops::Range<usize>>,
    iteration_path: IterationPath,
    #[serde(flatten)]
    reference_details: Option<ReferenceWire<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_location: Option<&'a DataLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grants_needed: Option<&'a [GrantNeeded]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_default: Option<&'a SchemaDefaultOrigin>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    related: &'a [DiagnosticRelated],
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'a str>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    trail: Option<&'a crate::findings::Trail>,
}

/// The reference fields of a [`DiagnosticWire`], each omitted when absent.
#[derive(Serialize)]
struct ReferenceWire<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    construct: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    included_via: Option<Vec<ReferenceTrailStep>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    searched: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestions: Option<&'a [String]>,
}

impl Serialize for Diagnostic {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        DiagnosticWire {
            code: &self.code,
            severity: self.severity,
            message: &self.message,
            source: self.source.as_ref(),
            expected_type: self.expected_type.as_deref(),
            actual_type: self.actual_type.as_deref(),
            span: self.span.as_ref(),
            iteration_path: self.iteration_path(),
            reference_details: self.reference_details.as_ref().map(|details| ReferenceWire {
                reason: details.reason.as_deref(),
                construct: details.construct.as_deref(),
                reference: details.reference.as_deref(),
                failure: details.failure.as_deref(),
                included_via: self.included_via(),
                base: details.base.as_deref(),
                resolved: details.resolved.as_deref(),
                searched: details.searched.as_deref(),
                suggestions: details.suggestions.as_deref(),
            }),
            data_location: self.data_location.as_ref(),
            grants_needed: self.grants_needed.as_deref(),
            schema_default: self.schema_default.as_ref(),
            related: &self.related,
            hint: self.hint.as_deref(),
            notes: &self.notes,
            trail: self.trail.has_host_frame().then_some(&self.trail),
        }
        .serialize(serializer)
    }
}

/// The one-based line and character column of byte `offset` in `text`;
/// `None` when the offset is out of bounds or splits a character.
pub fn one_based_position(text: &str, offset: usize) -> Option<(usize, usize)> {
    let before = text.get(..offset)?;
    crate::note_work!(PositionPrefixScanBytes, offset);
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    Some((before.matches('\n').count() + 1, before[line_start..].chars().count() + 1))
}

impl CallCheck {
    /// The shown findings of this call as diagnostics: each category the
    /// [`call_severity`] ladder shows under `strict` (a data problem's finding
    /// under its own `dm.data.*` code), then the always-false
    /// warning ([`ALWAYS_FALSE_CALL_CODE`]) under either setting.
    pub fn diagnostics(&self, strict: bool) -> Vec<ExpressionDiagnostic> {
        let mut found: Vec<_> = self
            .findings()
            .into_iter()
            .filter_map(|finding| {
                let severity = call_severity(finding.category, strict)?;
                let data = self.data_problem.as_ref().filter(|problem| finding.argument == Some(problem.argument));
                let code = if matches!(self.problem, Some(super::call_check::CallProblem::InvalidCallback { .. })) {
                    simplified_schema::expression::parser::INVALID_CALLBACK_CODE
                } else if matches!(self.problem, Some(super::call_check::CallProblem::InvalidSpreadOperand { .. })) {
                    super::call_check::INVALID_SPREAD_OPERAND_CODE
                } else if let Some(problem) = data {
                    problem.code
                } else if finding.category == ArgumentCategory::Incompatible {
                    INCOMPATIBLE_CALL_CODE
                } else {
                    UNCERTAIN_CALL_CODE
                };
                let argument = finding.argument.and_then(|index| self.arguments.iter().find(|argument| argument.index == index));
                Some(ExpressionDiagnostic {
                    code,
                    severity,
                    message: finding.message,
                    span: finding.span,
                    expected_type: argument.map(|argument| argument.parameter_type.clone()),
                    actual_type: argument.map(|argument| argument.argument_type.to_string()),
                })
            })
            .collect();
        found.extend(self.always_false_finding().map(|finding| ExpressionDiagnostic {
            code: ALWAYS_FALSE_CALL_CODE,
            severity: Severity::Warning,
            message: finding.message,
            span: finding.span,
            expected_type: None,
            actual_type: None,
        }));
        found
    }

    /// The diagnostic composition rejects the document with: the first
    /// category 5 finding. `None` for a call that is not rejected.
    pub fn rejection_diagnostic(&self) -> Option<ExpressionDiagnostic> {
        self.diagnostics(false).into_iter().find(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

impl IncompleteHostCall {
    /// The incomplete-analysis warning ([`INCOMPLETE_ANALYSIS_CODE`]) for this
    /// call, ranged on the whole call, under both strictness settings.
    pub fn diagnostic(&self) -> ExpressionDiagnostic {
        ExpressionDiagnostic {
            code: INCOMPLETE_ANALYSIS_CODE,
            severity: Severity::Warning,
            message: self.message(),
            span: self.span.clone(),
            expected_type: None,
            actual_type: None,
        }
    }
}

/// Every shown call diagnostic of `expr`, in every branch, each argument
/// typed at its own site under `site`: [`CallCheck::diagnostics`] for each
/// call, then an incomplete-analysis warning for each call to a host function
/// whose analysis is incomplete.
pub fn expression_call_diagnostics(expr: &SpannedExpr, site: &TypeScope<'_>, strict: bool) -> Vec<ExpressionDiagnostic> {
    let mut found: Vec<_> =
        check_expression_calls(expr, site).iter().flat_map(|check| check.diagnostics(strict)).collect();
    found.extend(incomplete_host_calls(expr).iter().map(IncompleteHostCall::diagnostic));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    use ArgumentCategory::{Compatible, Incompatible, NullableMatch, Uncertain, Unknown};

    #[test]
    fn the_severity_ladder_follows_the_category_table() {
        let warning = Some(Severity::Warning);
        let error = Some(Severity::Error);
        for (category, default, strict) in [
            (Compatible, None, None),
            (NullableMatch, None, warning),
            (Uncertain, warning, warning),
            (Unknown, None, warning),
            (Incompatible, error, error),
        ] {
            assert_eq!(call_severity(category, false), default, "{category:?} default");
            assert_eq!(call_severity(category, true), strict, "{category:?} strict");
        }
    }

    #[test]
    fn positions_are_one_based_lines_and_character_columns() {
        let text = "ab\nçd{{ x }}\n";
        assert_eq!(one_based_position(text, 0), Some((1, 1)));
        assert_eq!(one_based_position(text, 3), Some((2, 1)));
        // `ç` is two bytes and one column.
        assert_eq!(one_based_position(text, 6), Some((2, 3)));
        assert_eq!(one_based_position(text, 4), None, "inside a character");
        assert_eq!(one_based_position(text, 99), None);
    }
}

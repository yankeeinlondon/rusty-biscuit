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

use std::path::PathBuf;

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
            iteration_path: IterationPath::empty(),
            reference_details: None,
            data_location: None,
        }
    }

    /// This finding located where its span starts when the analyzed text
    /// begins at byte `offset` of `text`, the authored file's content.
    pub fn locate(&self, file: Option<PathBuf>, text: &str, offset: usize) -> Diagnostic {
        self.at(file, one_based_position(text, offset + self.span.start))
    }
}

/// A finding located in its authored source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// The original positions of the items being processed when a composition
    /// failed, outermost first. Empty for a static finding, a warning, or a
    /// failure outside every `::map` / `::filter` body.
    pub iteration_path: IterationPath,
    /// What a reference finding adds, written beside the fields above; `None`
    /// for every other family.
    #[serde(flatten)]
    pub reference_details: Option<ReferenceDetails>,
    /// Where in a loaded data source the problem is, for a
    /// `dm.data.malformed` or `dm.data.contract_mismatch` record; `None`
    /// (and not serialized) for every other record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_location: Option<DataLocation>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ReferenceDetails {
    /// The reason phrase, such as `file not found`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The construct phrase, such as `hyperlink` or `::code`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construct: Option<String>,
    /// The reference as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The file-reference failure class, such as `no-match`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// The inclusion path to the reference's document, outermost first; empty
    /// for the root document, `None` for a failure with no one reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_via: Option<Vec<ReferenceTrailStep>>,
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

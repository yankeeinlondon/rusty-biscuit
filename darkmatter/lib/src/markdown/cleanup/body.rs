//! Guarded body cleanup: the formatting rules as bounded source edits, then
//! one fidelity check of the final candidate.
//!
//! The candidate is checked once, after every rule (including fixed-width
//! re-wrapping) has run. If it is not equivalent to the input under the
//! resolved rules, or the input holds a malformed directive region, or the
//! edits cannot be applied, the input comes back unchanged with a
//! `clean.fidelity-fallback` finding. Counted parses: one of the input (shared
//! by planning and the comparison), one of the final candidate, and one more
//! for the re-wrap stage when a width is in effect; an unchanged input skips
//! the verification parse.

use super::compare::{Snapshot, tightness_exempt_lists};
use super::diagnostic::{CleanDiagnostic, FallbackReason};
use super::protect::{Lines, ParseView, Protection};
use super::rules::ResolvedMarkdownRules;
use super::structure::{self, Model};
use super::{edits, transform, wrap};

const BYTE_ORDER_MARK: &str = "\u{FEFF}";

/// What a clean run did with its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanOutcome {
    /// Nothing needed to change.
    Unchanged,
    /// The text was formatted.
    Changed,
    /// Cleanup refused and returned the input unchanged.
    Declined,
    /// The frontmatter is invalid YAML that safe repair could not resolve.
    /// Body cleanup never returns this.
    Invalid,
}

/// Why and where cleanup declined.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Fallback {
    pub reason: FallbackReason,
    /// Byte offset of the first affected input block: in the cleaned text
    /// for a [`BodyCleanup`], in the whole document once carried by a
    /// [`CleanDiagnostic`].
    pub offset: usize,
    /// What differed, for the diagnostic message.
    pub detail: String,
}

impl Fallback {
    /// The `clean.fidelity-fallback` finding for a document whose cleaned
    /// text starts at `text_offset`: a whole-document span, with the first
    /// affected block named by 1-based line and byte column in the message.
    #[must_use]
    pub fn diagnostic(&self, document: &str, text_offset: usize) -> CleanDiagnostic {
        let offset = (text_offset + self.offset).min(document.len());
        let lines = Lines::new(document);
        // Past the last terminator is the start of the line after the last.
        let (line, column) = if lines.len() == 0 {
            (1, 1)
        } else if offset == document.len() && document.ends_with(['\n', '\r']) {
            (lines.len() + 1, 1)
        } else {
            let index = lines.index_of(offset);
            (index + 1, offset.saturating_sub(lines.get(index).start) + 1)
        };
        let located = Self { offset, ..self.clone() };
        CleanDiagnostic::fidelity_fallback(0..document.len(), located.message(line, column), located)
    }

    /// The finding's message with the first affected block at `line` and
    /// `column` (1-based, in whatever column unit the caller reports).
    #[must_use]
    pub fn message(&self, line: usize, column: usize) -> String {
        let reason = match self.reason {
            FallbackReason::MalformedRegion => "a malformed directive region",
            FallbackReason::SerializationFailure => "the formatted text could not be produced",
            FallbackReason::SemanticMismatch => "formatting would change the document's meaning",
        };
        format!(
            "cleanup left the document unchanged because of {reason} at line {line}, column {column}: {}",
            self.detail
        )
    }
}

/// The result of [`clean_body`].
#[derive(Debug, Clone, PartialEq)]
pub struct BodyCleanup {
    /// The cleaned text; the input itself unless `outcome` is `Changed`.
    pub text: String,
    pub outcome: CleanOutcome,
    /// Findings in the body's own coordinates.
    pub diagnostics: Vec<CleanDiagnostic>,
    /// Set when `outcome` is `Declined`, so a caller embedding the body in a
    /// document can re-anchor the finding with [`Fallback::diagnostic`].
    pub fallback: Option<Fallback>,
}

/// Formats a Markdown body (no frontmatter) under `rules`.
///
/// Directive lines, expressions, code, and every byte no enabled rule
/// touches come through unchanged; with every body rule off the output is the
/// input. Never returns [`CleanOutcome::Invalid`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::{CleanOutcome, clean_body};
/// use darkmatter::markdown::cleanup::rules::ResolvedMarkdownRules;
///
/// let cleaned = clean_body("```\ncode\n```\n", &ResolvedMarkdownRules::defaults());
/// assert_eq!(cleaned.outcome, CleanOutcome::Changed);
/// assert_eq!(cleaned.text, "```text\ncode\n```\n");
/// ```
#[must_use]
pub fn clean_body(body: &str, rules: &ResolvedMarkdownRules) -> BodyCleanup {
    let (bom, text) = match body.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (BYTE_ORDER_MARK, rest),
        None => ("", body),
    };
    match run(text, rules) {
        Ok(Some(candidate)) => BodyCleanup {
            text: format!("{bom}{candidate}"),
            outcome: CleanOutcome::Changed,
            diagnostics: Vec::new(),
            fallback: None,
        },
        Ok(None) => BodyCleanup {
            text: body.to_string(),
            outcome: CleanOutcome::Unchanged,
            diagnostics: Vec::new(),
            fallback: None,
        },
        Err(mut fallback) => {
            fallback.offset += bom.len();
            BodyCleanup {
                text: body.to_string(),
                outcome: CleanOutcome::Declined,
                diagnostics: vec![fallback.diagnostic(body, 0)],
                fallback: Some(fallback),
            }
        }
    }
}

/// One parsed side: everything planning and comparison read.
struct Side {
    protection: Protection,
    lines: Lines,
    view: ParseView,
}

impl Side {
    fn new(source: &str) -> Result<Self, super::protect::MalformedRegion> {
        let protection = Protection::scan(source)?;
        let view = ParseView::new(source, &protection);
        Ok(Self { lines: Lines::new(source), protection, view })
    }
}

fn run(source: &str, rules: &ResolvedMarkdownRules) -> Result<Option<String>, Fallback> {
    if source.is_empty() {
        return Ok(None);
    }
    let input = Side::new(source).map_err(|malformed| Fallback {
        reason: FallbackReason::MalformedRegion,
        offset: malformed.offset,
        detail: malformed.message,
    })?;
    let parsed = structure::parse(&input.view);
    let model = Model::build(source, &input.lines, &parsed.events);
    let planned = transform::plan(source, &input.lines, &input.protection, &parsed.events, &model, rules);
    let mut candidate = if planned.is_empty() {
        None
    } else {
        Some(edits::apply(source, planned, &input.protection).map_err(conflict)?)
    };

    if let Some(width) = rules.wrap_columns() {
        let width = usize::from(width);
        candidate = match candidate {
            // Nothing changed yet: the input parse serves the re-wrap stage too.
            None => {
                let wrapped = wrap::plan(source, &input.lines, &input.protection, &parsed.events, &model, width);
                (!wrapped.is_empty())
                    .then(|| edits::apply(source, wrapped, &input.protection).map_err(conflict))
                    .transpose()?
            }
            Some(stage) => {
                let side = Side::new(&stage).map_err(|malformed| Fallback {
                    reason: FallbackReason::SerializationFailure,
                    offset: malformed.offset.min(source.len()),
                    detail: malformed.message,
                })?;
                let stage_parsed = structure::parse(&side.view);
                let stage_model = Model::build(&stage, &side.lines, &stage_parsed.events);
                let wrapped = wrap::plan(&stage, &side.lines, &side.protection, &stage_parsed.events, &stage_model, width);
                if wrapped.is_empty() {
                    Some(stage)
                } else {
                    Some(edits::apply(&stage, wrapped, &side.protection).map_err(conflict)?)
                }
            }
        };
    }

    #[cfg(any(test, feature = "cleanup-test-hooks"))]
    if let Some(mutated) = test_hooks::apply(candidate.as_deref().unwrap_or(source)) {
        candidate = Some(mutated);
    }

    let Some(candidate) = candidate.filter(|candidate| candidate != source) else {
        return Ok(None);
    };

    let exempt = tightness_exempt_lists(&model, rules);
    let before = Snapshot::new(
        source,
        &input.lines,
        &input.protection,
        &parsed.events,
        &parsed.definitions,
        &model,
        rules,
        &exempt,
    );
    let output = Side::new(&candidate).map_err(|malformed| Fallback {
        reason: FallbackReason::SemanticMismatch,
        offset: malformed.offset.min(source.len()),
        detail: format!("the formatted text has a malformed directive region: {}", malformed.message),
    })?;
    let output_parsed = structure::parse(&output.view);
    let output_model = Model::build(&candidate, &output.lines, &output_parsed.events);
    let after = Snapshot::new(
        &candidate,
        &output.lines,
        &output.protection,
        &output_parsed.events,
        &output_parsed.definitions,
        &output_model,
        rules,
        &exempt,
    );
    before.compare(&after, rules).map_err(|divergence| Fallback {
        reason: FallbackReason::SemanticMismatch,
        offset: divergence.offset,
        detail: divergence.detail,
    })?;
    Ok(Some(candidate))
}

fn conflict(conflict: edits::EditConflict) -> Fallback {
    Fallback { reason: FallbackReason::SerializationFailure, offset: conflict.offset, detail: conflict.message }
}

/// A test-only seam that corrupts the cleanup candidate before the fidelity
/// check, so tests in this crate and in consumers (with the
/// `cleanup-test-hooks` feature) can prove a broken pass is refused.
/// Production builds compile none of it.
#[cfg(any(test, feature = "cleanup-test-hooks"))]
pub mod test_hooks {
    use std::sync::Mutex;

    /// A corruption of the candidate text.
    pub type BrokenPass = fn(&str) -> String;

    static BROKEN_PASS: Mutex<Option<BrokenPass>> = Mutex::new(None);

    /// Removes the installed pass when dropped.
    #[must_use = "the broken pass is removed when the guard drops"]
    pub struct BrokenPassGuard(());

    impl Drop for BrokenPassGuard {
        fn drop(&mut self) {
            *BROKEN_PASS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }

    /// Installs `pass` process-wide: every body cleanup applies it to its
    /// candidate (or, when no rule changed anything, to the input) until the
    /// guard drops.
    pub fn install_broken_pass(pass: BrokenPass) -> BrokenPassGuard {
        *BROKEN_PASS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(pass);
        BrokenPassGuard(())
    }

    pub(super) fn apply(candidate: &str) -> Option<String> {
        let pass = *BROKEN_PASS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        pass.map(|pass| pass(candidate))
    }
}

//! The join between the passive member check and the evaluator's reach.
//!
//! An undeclared-member warning exists in composition only where two facts
//! hold for one authored selector: the passive precheck found it (a
//! [`MemberCandidate`], computed once per document under the schema the
//! precheck used) and the evaluator reached it (an ordinal an
//! [`Observation`](super::expression::absence::Observation) recorded).
//!
//! ```mermaid
//! flowchart LR
//!     P[precheck: member findings] -->|candidate per site and ordinal| C[MemberCandidates]
//!     E[evaluator: reached ordinals] --> J{same site key<br/>same expression text?}
//!     C --> J
//!     J -->|yes| W[warning, once per authored site]
//!     J -->|no| N[nothing]
//! ```
//!
//! A site is keyed where it was authored ([`MemberSiteKey`]), so a repeated
//! `::map` copy or a second include of one child names the same candidate,
//! and its warning collapses by identity when it is added to the report.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use super::ComposeWarning;
use super::body_origin::BodyOrigin;
use super::expression::absence::Observation;

/// Where one evaluated expression was authored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MemberSiteKey {
    /// The frontmatter key path label of the leaf holding it; `None` for a
    /// body site.
    pub(crate) scope: Option<String>,
    /// For a body site, the loaded-document byte offset of the construct's
    /// first byte (the `{{` of an interpolation, the first byte of a `when`
    /// condition or a collection gate or callback); for a frontmatter site,
    /// the byte offset of the `{{` in the authored leaf; for a `$( … )`
    /// piece, its [`ShellPiece`].
    pub(crate) start: usize,
}

impl MemberSiteKey {
    /// The key of the body construct starting at `offset` of the text
    /// `origin` describes; `None` when the byte was not copied from the
    /// loaded document (an unknown position yields no warning).
    pub(crate) fn body(origin: &BodyOrigin, offset: usize) -> Option<Self> {
        let start = origin.project(&(offset..offset + 1))?.start;
        Some(Self { scope: None, start })
    }

    /// The key of one piece of the frontmatter `$( … )` ternary of the
    /// top-level `key`; the piece stands in for an offset because the
    /// evaluating stage holds the pieces, not their positions.
    pub(crate) fn shell(key: &str, piece: ShellPiece) -> Self {
        Self {
            scope: Some(format!("$(/{key})")),
            start: piece as usize,
        }
    }

    /// The key of the `{{` at `offset` of the frontmatter leaf `scope` labels.
    pub(crate) fn leaf(scope: String, offset: usize) -> Self {
        Self {
            scope: Some(scope),
            start: offset,
        }
    }
}

/// A piece of a frontmatter `$( … )` ternary that is evaluated as an
/// expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellPiece {
    Condition = 0,
    Then = 1,
    Else = 2,
}

/// One undeclared member the precheck found at an authored site.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MemberCandidate {
    /// The trimmed text of the expression the finding was computed from.
    pub(crate) expression: String,
    /// The member name's offset in [`Self::expression`]; orders the warnings
    /// of one expression.
    pub(crate) order: usize,
    /// The finished warning, located where the member was authored.
    pub(crate) warning: ComposeWarning,
}

/// A document's member candidates, by site and node ordinal.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MemberCandidates {
    sites: HashMap<(MemberSiteKey, usize), MemberCandidate>,
}

impl MemberCandidates {
    /// Records the candidate of the access node `ordinal` of the expression
    /// authored at `key`.
    pub(crate) fn insert(&mut self, key: MemberSiteKey, ordinal: usize, candidate: MemberCandidate) {
        self.sites.insert((key, ordinal), candidate);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// The warnings of the `reached` ordinals of the expression authored at
    /// `key`, in authored order. A candidate whose recorded expression text
    /// differs from `expression` (text a stage rewrote) yields nothing, so an
    /// ordinal is never joined to another expression's finding.
    pub(crate) fn warnings(&self, key: &MemberSiteKey, expression: &str, reached: &[usize]) -> Vec<ComposeWarning> {
        if self.sites.is_empty() || reached.is_empty() {
            return Vec::new();
        }
        let expression = expression.trim();
        let mut found: Vec<&MemberCandidate> = reached
            .iter()
            .filter_map(|ordinal| self.sites.get(&(key.clone(), *ordinal)))
            .filter(|candidate| candidate.expression == expression)
            .collect();
        found.sort_by_key(|candidate| candidate.order);
        found.into_iter().map(|candidate| candidate.warning.clone()).collect()
    }
}

/// A body surface's view of its document's member candidates: the
/// conditions and collection callbacks a stage evaluates directly.
pub(crate) struct BodyMembers<'a> {
    candidates: Arc<MemberCandidates>,
    origin: &'a BodyOrigin,
}

impl<'a> BodyMembers<'a> {
    /// The view of `candidates` for the body `origin` describes; `None` when
    /// there is nothing to join, no proven origin, or the pass only discovers
    /// (`defer_expression_failures`), which reports no member reads.
    pub(crate) fn new(
        candidates: &Arc<MemberCandidates>,
        origin: Option<&'a BodyOrigin>,
        options: &super::ComposeRequest,
    ) -> Option<Self> {
        let origin = origin?;
        (!candidates.is_empty() && !options.defer_expression_failures).then(|| Self {
            candidates: Arc::clone(candidates),
            origin,
        })
    }

    /// The warnings of the selectors `observation` recorded for the
    /// expression `expression` starting at `start` of the current body.
    /// Always clears the record.
    pub(crate) fn warnings(&self, start: usize, expression: &str, observation: &mut Observation) -> Vec<ComposeWarning> {
        let reached = observation.take_reached();
        match MemberSiteKey::body(self.origin, start) {
            Some(key) => self.candidates.warnings(&key, expression, &reached),
            None => Vec::new(),
        }
    }
}

/// How one interpolating surface names the sites it evaluates.
#[derive(Debug)]
pub(crate) enum MemberFrame<'a> {
    /// Body text `origin` describes.
    Body(&'a BodyOrigin),
    /// A frontmatter leaf; its label is set before each leaf is rewritten.
    Leaf(std::cell::RefCell<Option<String>>),
}

impl MemberFrame<'_> {
    /// The key of the `{{ … }}` spanning `span` of the scanned text.
    pub(crate) fn key(&self, span: &Range<usize>) -> Option<MemberSiteKey> {
        match self {
            Self::Body(origin) => MemberSiteKey::body(origin, span.start),
            Self::Leaf(label) => label.borrow().clone().map(|label| MemberSiteKey::leaf(label, span.start)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::context::report::{ExpressionOrigin, WarningIdentity, WarningSubject};

    fn warning(message: &str) -> ComposeWarning {
        ComposeWarning::new("interpolation", message)
    }

    fn candidates() -> MemberCandidates {
        let mut candidates = MemberCandidates::default();
        let key = MemberSiteKey::leaf("/title".to_string(), 0);
        for (ordinal, order, message) in [(2, 9, "second"), (1, 3, "first")] {
            candidates.insert(
                key.clone(),
                ordinal,
                MemberCandidate {
                    expression: "[a.b, a.c]".to_string(),
                    order,
                    warning: warning(message),
                },
            );
        }
        candidates
    }

    fn messages(warnings: Vec<ComposeWarning>) -> Vec<String> {
        warnings.into_iter().map(|warning| warning.message).collect()
    }

    #[test]
    fn keys_compare_by_scope_and_start() {
        assert_eq!(MemberSiteKey::leaf("/a".into(), 3), MemberSiteKey::leaf("/a".into(), 3));
        assert_ne!(MemberSiteKey::leaf("/a".into(), 3), MemberSiteKey::leaf("/b".into(), 3));
        assert_ne!(MemberSiteKey::leaf("/a".into(), 3), MemberSiteKey::leaf("/a".into(), 4));
        assert_ne!(
            MemberSiteKey { scope: None, start: 3 },
            MemberSiteKey::leaf("/a".into(), 3)
        );
    }

    #[test]
    fn reached_candidates_are_returned_in_authored_order() {
        let key = MemberSiteKey::leaf("/title".to_string(), 0);
        let found = candidates().warnings(&key, " [a.b, a.c] ", &[2, 1, 7]);
        assert_eq!(messages(found), ["first", "second"]);
        assert!(candidates().warnings(&key, "[a.b, a.c]", &[]).is_empty());
    }

    #[test]
    fn another_site_or_rewritten_text_yields_nothing() {
        let other = MemberSiteKey::leaf("/title".to_string(), 1);
        assert!(candidates().warnings(&other, "[a.b, a.c]", &[1, 2]).is_empty());
        let key = MemberSiteKey::leaf("/title".to_string(), 0);
        assert!(candidates().warnings(&key, "[a.b, a.d]", &[1, 2]).is_empty());
    }

    #[test]
    fn repeated_reaches_of_one_site_collapse_in_a_report() {
        let key = MemberSiteKey::leaf("/title".to_string(), 0);
        let mut candidates = MemberCandidates::default();
        candidates.insert(
            key.clone(),
            1,
            MemberCandidate {
                expression: "a.b".to_string(),
                order: 2,
                warning: ComposeWarning {
                    source: Some(ComposeWarning::EXPRESSION_SOURCE.to_string()),
                    code: Some(crate::markdown::compose::expression::UNDECLARED_MEMBER_CODE.to_string()),
                    identity: Some(WarningIdentity {
                        subject: WarningSubject::Expression {
                            document: None,
                            scope: Some("/title".to_string()),
                            origin: ExpressionOrigin::Authored(2..3),
                        },
                    }),
                    ..warning("Member 'b' is not declared")
                },
            },
        );
        let mut report = super::super::ComposeReport::default();
        for _ in 0..3 {
            report.add_warnings(candidates.warnings(&key, "a.b", &[1]));
        }
        assert_eq!(report.warnings.len(), 1);
    }
}

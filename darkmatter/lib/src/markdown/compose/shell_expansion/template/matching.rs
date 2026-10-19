//! Whitelist and blacklist matching for `template` rules.
//!
//! A whitelist rule covers a candidate shape when both have the same segments,
//! operators, ordered redirections, argument count, byte-equal literals, holes
//! at the same positions, and each candidate hole's type is covered by the
//! rule's ([`rule_covers`]).
//!
//! A blacklist rule errs toward refusing: holes match any hole whatever its
//! type, redirections are ignored, and a single-segment rule matches any
//! segment of a chain. Against a filled command, a hole matches any run of its
//! containing argument, so a command with no authored holes is blocked too.

use super::{ApprovalType, FilledChain, FilledSegment, ShapeArg, ShapePart, ShapeSegment, TemplateShape};
use crate::markdown::compose::shell_expansion::types::{ShellRuleEntry, ShellRuleSet};
use crate::markdown::schemas::{is_string_like, is_subtype};

/// Whether a rule hole of type `rule` covers a candidate hole of type
/// `declared`.
///
/// A broad `string` rule covers every hole. A narrower rule covers only a
/// string-like declaration that the core subtype relation places inside it; it
/// never covers a hole whose approval type is the broad `string`.
pub fn rule_covers(rule: &ApprovalType, declared: &ApprovalType) -> bool {
    match (rule, declared) {
        (ApprovalType::BroadString, _) => true,
        (ApprovalType::StringLike(rule), ApprovalType::StringLike(declared)) => {
            // `is_subtype` holds for any identical pair; only a string-like
            // rule may reach it (identical numbers or arrays are refused).
            is_string_like(rule.atom()) && is_string_like(declared.atom()) && is_subtype(declared.atom(), rule.atom())
        }
        (ApprovalType::StringLike(_), ApprovalType::BroadString) => false,
    }
}

impl TemplateShape {
    /// Whether this whitelist rule covers `candidate`.
    pub fn covers(&self, candidate: &TemplateShape) -> bool {
        self.segments.len() == candidate.segments.len()
            && self.segments.iter().zip(&candidate.segments).all(|(rule, candidate)| {
                rule.operator == candidate.operator
                    && rule.redirects == candidate.redirects
                    && segment_matches(rule, candidate, rule_covers)
            })
    }

    /// Whether this blacklist rule matches `candidate` by shape alone.
    pub fn blocks_shape(&self, candidate: &TemplateShape) -> bool {
        let same_shape = |rule: &ShapeSegment, candidate: &ShapeSegment| segment_matches(rule, candidate, |_, _| true);
        match self.segments.as_slice() {
            [single] => candidate.segments.iter().any(|segment| same_shape(single, segment)),
            segments => {
                segments.len() == candidate.segments.len()
                    && segments
                        .iter()
                        .zip(&candidate.segments)
                        .all(|(rule, candidate)| rule.operator == candidate.operator && same_shape(rule, candidate))
            }
        }
    }

    /// Whether this blacklist rule matches the filled `chain`.
    pub fn blocks_filled(&self, chain: &FilledChain) -> bool {
        match self.segments.as_slice() {
            [single] => chain.segments.iter().any(|segment| filled_matches(single, segment)),
            segments => {
                segments.len() == chain.segments.len()
                    && segments
                        .iter()
                        .zip(&chain.segments)
                        .all(|(rule, filled)| rule.operator == filled.operator && filled_matches(rule, filled))
            }
        }
    }
}

/// Same program, argument count, literals, and hole positions; `hole_matches(rule, candidate)` judges each pair of holes.
fn segment_matches(
    rule: &ShapeSegment,
    candidate: &ShapeSegment,
    hole_matches: impl Fn(&ApprovalType, &ApprovalType) -> bool,
) -> bool {
    rule.program == candidate.program
        && rule.args.len() == candidate.args.len()
        && rule.args.iter().zip(&candidate.args).all(|(rule, candidate)| {
            rule.parts.len() == candidate.parts.len()
                && rule.parts.iter().zip(&candidate.parts).all(|parts| match parts {
                    (ShapePart::Literal(rule), ShapePart::Literal(candidate)) => rule == candidate,
                    (ShapePart::Hole(rule), ShapePart::Hole(candidate)) => hole_matches(rule, candidate),
                    _ => false,
                })
        })
}

fn filled_matches(rule: &ShapeSegment, filled: &FilledSegment) -> bool {
    rule.program == filled.program
        && rule.args.len() == filled.args.len()
        && rule.args.iter().zip(&filled.args).all(|(rule, value)| arg_matches_text(rule, value))
}

/// Whether `text` is one value of `rule`: its literal runs in order, each hole
/// matching any (possibly empty) run between them.
fn arg_matches_text(rule: &ShapeArg, text: &str) -> bool {
    // The literals split by holes. Holes act as `*`, so a greedy left-to-right
    // search for the middle literals is exact.
    let mut runs: Vec<&str> = vec![""];
    for part in &rule.parts {
        match part {
            ShapePart::Literal(literal) => *runs.last_mut().expect("non-empty") = literal.as_str(),
            ShapePart::Hole(_) => runs.push(""),
        }
    }
    let [first, middle @ .., last] = runs.as_slice() else {
        return text == runs[0];
    };
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    for literal in middle {
        match rest.find(literal) {
            Some(index) => rest = &rest[index + literal.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

impl ShellRuleSet {
    /// Whether a `template` whitelist rule covers `candidate`.
    pub fn template_covers(&self, candidate: &TemplateShape) -> bool {
        self.templates().any(|rule| rule.covers(candidate))
    }

    /// Whether a `template` blacklist rule matches `candidate` by shape.
    pub fn template_blocks_shape(&self, candidate: &TemplateShape) -> bool {
        self.templates().any(|rule| rule.blocks_shape(candidate))
    }

    /// Whether any blacklist rule blocks the filled `chain`: an `exact` rule
    /// for a segment's normalized command, a `prefix` rule for its program, or
    /// a `template` rule matching the filled arguments.
    pub fn blocks_filled(&self, chain: &FilledChain) -> bool {
        chain.segments.iter().any(|segment| {
            self.matches_exact(&super::super::policy::normalize_command(&segment.program, &segment.args))
                || self.matches_prefix(&segment.program)
        }) || self.templates().any(|rule| rule.blocks_filled(chain))
    }

    fn templates(&self) -> impl Iterator<Item = &TemplateShape> {
        self.entries.iter().filter_map(|entry| match entry {
            ShellRuleEntry::Template(shape) => Some(shape),
            ShellRuleEntry::Exact(_) | ShellRuleEntry::Prefix(_) => None,
        })
    }
}

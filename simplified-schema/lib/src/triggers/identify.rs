//! Identifying triggers: a schema trigger that decides which host program a
//! document belongs to.
//!
//! A host registration names one ordinary `schema-trigger` as its
//! identifying trigger. Before it is used, [`admit_identifying_trigger`]
//! proves structurally that every way the expression can hold requires at
//! least one non-null top-level key the caller does not own (a **witness**).
//! A shared key alone (an event every host understands), an optional key, a
//! `$path` predicate, or a `none:` exclusion never identifies a host. This
//! is a proof over the match tree, not general schema satisfiability: an
//! expression it cannot prove is refused with a repair hint.
//!
//! [`identifies`] then evaluates an admitted trigger. It differs from
//! ordinary [`matches`](super::matcher::matches) in one way: a positive
//! required property condition tests only that the key is present and not
//! null, whatever type it declares, so a host key holding a wrongly typed
//! value still selects the host whose schema can report the type error.
//! Ordinary trigger matching is unchanged.
//!
//! Passive: neither function reads a file. A `$path` predicate is judged
//! against the known document path only, as ordinary matching judges it.

use std::collections::BTreeSet;

use serde_json::Value;

use super::grammar::{MatchConditions, MatchExpr};
use super::matcher::{PathSubject, property_matches};

/// Code of the warning for an identifying trigger that could hold on keys
/// the caller owns alone.
pub const IDENTIFY_TOO_BROAD_CODE: &str = "dm.host.identify_too_broad";

/// An identifying trigger [`admit_identifying_trigger`] proved: every
/// alternative requires a non-null key outside the caller's owned names.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifyingTrigger {
    conditions: MatchConditions,
}

impl IdentifyingTrigger {
    /// The admitted `match:` conditions.
    #[must_use]
    pub const fn conditions(&self) -> &MatchConditions {
        &self.conditions
    }
}

/// Why an identifying trigger was refused ([`IDENTIFY_TOO_BROAD_CODE`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifyTooBroad {
    /// The first sub-expression that supplies no witness, written from
    /// `match` down (`match[0].group[2]`); `match` for an empty expression.
    pub field: String,
    /// What the expression lacks.
    pub message: String,
}

impl IdentifyTooBroad {
    /// The repair to suggest beside [`Self::message`].
    #[must_use]
    pub const fn hint() -> &'static str {
        "make every alternative require a non-null top-level key only this host owns, \
         for example `sequence: object(required)`"
    }
}

impl std::fmt::Display for IdentifyTooBroad {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "`{}` {}; {}", self.field, self.message, Self::hint())
    }
}

impl std::error::Error for IdentifyTooBroad {}

/// Admits `conditions` as an identifying trigger.
///
/// `owned` holds the top-level names the caller owns (for Darkmatter: its
/// baseline properties, events, and globals). A positive property condition
/// that requires its key ([`PropertyAtom::is_required`]) with a type that
/// does not admit `null` ([`PropertyAtom::admits_null`], so a nullable or
/// `unknown` atom never counts) on a name outside `owned` is a witness. `all:` (and the top-level list) needs one witnessing child;
/// every `group:` alternative must witness; `min-match: n` witnesses only
/// when `n` is greater than the number of children without a witness, so
/// any `n` children that hold include one. `none:` and `$path` supply none.
///
/// [`PropertyAtom::is_required`]: crate::simplified::PropertyAtom::is_required
/// [`PropertyAtom::admits_null`]: crate::simplified::PropertyAtom::admits_null
///
/// ## Errors
///
/// [`IdentifyTooBroad`] naming the first sub-expression without a witness.
pub fn admit_identifying_trigger(
    conditions: &MatchConditions,
    owned: &BTreeSet<String>,
) -> Result<IdentifyingTrigger, IdentifyTooBroad> {
    let root = MatchExpr::All(conditions.0.clone());
    match unwitnessed(&root, owned, "match") {
        None => Ok(IdentifyingTrigger { conditions: conditions.clone() }),
        Some(refusal) => Err(refusal),
    }
}

/// The refusal for `expr` at `field`, or `None` when it witnesses.
fn unwitnessed(expr: &MatchExpr, owned: &BTreeSet<String>, field: &str) -> Option<IdentifyTooBroad> {
    let refuse = |message: &str| Some(IdentifyTooBroad { field: field.to_string(), message: message.to_string() });
    match expr {
        MatchExpr::Property { name, atom } => {
            // `required` is presence only: a type that admits `null` is
            // met by a `null` value, which identifies nothing.
            if !atom.is_required() || atom.admits_null() {
                refuse(&format!("does not require `{name}` to be present and non-null"))
            } else if owned.contains(name) {
                refuse(&format!("requires only `{name}`, which the caller owns"))
            } else {
                None
            }
        }
        MatchExpr::Path(_) => refuse("is a `$path` predicate, which cannot identify a host"),
        MatchExpr::None(_) => refuse("is a `none:` exclusion, which cannot identify a host"),
        MatchExpr::All(children) => {
            if children.is_empty() {
                return refuse("has no condition");
            }
            if let [only] = children.as_slice() {
                return unwitnessed(only, owned, &child_field(field, expr, 0));
            }
            let field = |index: usize| child_field(field, expr, index);
            if children.iter().enumerate().any(|(index, child)| unwitnessed(child, owned, &field(index)).is_none()) {
                None
            } else {
                // Report the first child's reason: each lacks a witness.
                unwitnessed(&children[0], owned, &field(0))
                    .map(|first| IdentifyTooBroad { message: format!("has no condition that identifies a host ({})", first.message), ..first })
            }
        }
        MatchExpr::Group(children) => {
            if children.is_empty() {
                return refuse("has no alternative");
            }
            children
                .iter()
                .enumerate()
                .find_map(|(index, child)| unwitnessed(child, owned, &child_field(field, expr, index)))
        }
        MatchExpr::MinMatch { count, of } => {
            let without: Vec<usize> = of
                .iter()
                .enumerate()
                .filter(|(index, child)| unwitnessed(child, owned, &child_field(field, expr, *index)).is_some())
                .map(|(index, _)| index)
                .collect();
            if *count > without.len() {
                None
            } else {
                refuse(&format!(
                    "can hold on {count} condition(s) without one that identifies a host ({} of {} lack one)",
                    without.len(),
                    of.len()
                ))
            }
        }
    }
}

/// The field of `parent`'s child `index`: the top-level list's entries are
/// `match[i]`, a combinator's are `<field>.<combinator>[i]`. A property
/// condition keeps its parent's field: the grammar parses a mapping of
/// property keys as an `all` the author never wrote, and the refusal message
/// names the key.
fn child_field(field: &str, parent: &MatchExpr, index: usize) -> String {
    let combinator = match parent {
        MatchExpr::All(_) if field == "match" => return format!("match[{index}]"),
        MatchExpr::All(children) if matches!(children.get(index), Some(MatchExpr::Property { .. })) => {
            return field.to_string();
        }
        MatchExpr::All(_) => "all",
        MatchExpr::Group(_) => "group",
        MatchExpr::None(_) => "none",
        MatchExpr::MinMatch { .. } => "min-match",
        MatchExpr::Property { .. } | MatchExpr::Path(_) => return field.to_string(),
    };
    format!("{field}.{combinator}[{index}]")
}

/// Whether `trigger` identifies the document whose parsed frontmatter is
/// `frontmatter` and whose path is in `subject`.
///
/// Evaluated as ordinary matching, except that a required property
/// condition in positive position (under no `none:`, or under an even number
/// of them) tests only that the key is present and not null. A guard (no
/// `required`) and a condition a `none:` negates keep their ordinary
/// meaning.
#[must_use]
pub fn identifies(trigger: &IdentifyingTrigger, frontmatter: &Value, subject: &PathSubject<'_>) -> bool {
    trigger.conditions.0.iter().all(|condition| holds(condition, frontmatter, subject, true))
}

fn holds(expr: &MatchExpr, frontmatter: &Value, subject: &PathSubject<'_>, positive: bool) -> bool {
    match expr {
        MatchExpr::All(children) => children.iter().all(|child| holds(child, frontmatter, subject, positive)),
        MatchExpr::Group(children) => children.iter().any(|child| holds(child, frontmatter, subject, positive)),
        MatchExpr::None(children) => !children.iter().any(|child| holds(child, frontmatter, subject, !positive)),
        MatchExpr::MinMatch { count, of } => {
            of.iter().filter(|child| holds(child, frontmatter, subject, positive)).count() >= *count
        }
        MatchExpr::Property { name, atom } if positive && atom.is_required() => {
            frontmatter.get(name).is_some_and(|value| !value.is_null())
        }
        MatchExpr::Property { name, atom } => property_matches(frontmatter, name, atom),
        MatchExpr::Path(globs) => subject.admits(globs),
    }
}

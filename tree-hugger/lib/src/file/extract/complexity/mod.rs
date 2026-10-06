//! Cyclomatic complexity (`metrics.cyclomatic`): one iterative, pruned walk
//! of a callable's body ([`CallableBody`]), driven by a per-language rule
//! table of grammar node kinds, fields and operator tokens.
//!
//! A language without a reviewed table has no value (`None`); there is no
//! approximate fallback. Only the body is visited, so parameter defaults,
//! decorators, attributes and return types never count.

use super::*;

mod javascript;
mod rust;
mod typescript;

/// A language's reviewed counting rules.
pub(crate) struct Rules {
    /// Nested callables and types: their subtrees are another declaration's
    /// code, so they are neither visited nor checked for errors.
    boundaries: &'static [&'static str],
    /// Part of the body but not syntax the rules can read (a Rust macro's
    /// token tree): not visited and contributes 0, but a syntax error inside
    /// still makes the total unavailable.
    opaque: &'static [&'static str],
    decisions: &'static [Decision],
}

/// What a node of `kind` adds to the total.
struct Decision {
    kind: &'static str,
    counts: Counts,
}

enum Counts {
    /// +1 for every node of the kind.
    Always,
    /// +1 when the node has this field (a Rust guard's `condition`).
    WithField(&'static str),
    /// +1 when the node's `operator` field is one of these tokens.
    Operator(&'static [&'static str]),
    /// +1 per unnamed child token in the set (the `&&` of a Rust `let`
    /// chain, which has no field).
    Tokens(&'static [&'static str]),
    /// +1 unless no later named sibling has the same kind: every arm but the
    /// last.
    UnlessLast,
    /// +1 unless the field holds a node of this kind (a JavaScript
    /// `for (;;)`, whose `condition` is an `empty_statement`).
    UnlessFieldKind { field: &'static str, kind: &'static str },
    /// +1 for an arm that has this field, or that no other arm follows:
    /// labels grouped over one body (`case 1: case 2: …`, `case 1:
    /// default: …`) are one arm, and only the last has the body.
    Arm(&'static str),
}

fn rules_for(language: ProgrammingLanguage) -> Option<&'static Rules> {
    match language {
        ProgrammingLanguage::Rust => Some(&rust::RULES),
        ProgrammingLanguage::JavaScript => Some(&javascript::RULES),
        ProgrammingLanguage::TypeScript => Some(&typescript::RULES),
        _ => None,
    }
}

/// `1 +` the decisions `language`'s rules count in `body`. `None` when the
/// language has no reviewed rules, a visited node is an error or missing
/// node, or the total does not fit in `u32`.
pub(crate) fn cyclomatic(body: CallableBody<'_>, language: ProgrammingLanguage) -> Option<u32> {
    count(body, rules_for(language)?)
}

fn count(body: CallableBody<'_>, rules: &Rules) -> Option<u32> {
    let mut total = Tally(1);
    match body {
        CallableBody::Node(node) => walk(node, rules, &mut total)?,
        CallableBody::Run { first, last } => {
            let mut node = first;
            loop {
                walk(node, rules, &mut total)?;
                if node.id() == last.id() {
                    break;
                }
                node = node.next_sibling()?;
            }
        }
        CallableBody::EmptyAt(_) => {}
    }
    Some(total.0)
}

/// A running total that refuses to wrap.
struct Tally(u32);

impl Tally {
    fn add(&mut self, amount: u32) -> Option<()> {
        self.0 = self.0.checked_add(amount)?;
        Some(())
    }
}

/// Visits `root`'s subtree with one cursor (no recursion, so nesting depth
/// never reaches the call stack).
fn walk(root: Node<'_>, rules: &Rules, total: &mut Tally) -> Option<()> {
    let mut cursor = root.walk();
    let mut depth = 0usize;
    loop {
        if visit(cursor.node(), rules, total)? && cursor.goto_first_child() {
            depth += 1;
            continue;
        }
        loop {
            if depth == 0 {
                return Some(());
            }
            if cursor.goto_next_sibling() {
                break;
            }
            cursor.goto_parent();
            depth -= 1;
        }
    }
}

/// Adds `node`'s own contribution; `Some(true)` when its children are to be
/// visited, `None` when the total is unavailable.
fn visit(node: Node<'_>, rules: &Rules, total: &mut Tally) -> Option<bool> {
    if node.is_error() || node.is_missing() {
        return None;
    }
    let kind = node.kind();
    if rules.boundaries.contains(&kind) {
        return Some(false);
    }
    if rules.opaque.contains(&kind) {
        return (!node.has_error()).then_some(false);
    }
    for decision in rules.decisions.iter().filter(|decision| decision.kind == kind) {
        total.add(decision.counts.of(node))?;
    }
    Some(true)
}

impl Counts {
    fn of(&self, node: Node<'_>) -> u32 {
        match self {
            Self::Always => 1,
            Self::WithField(field) => u32::from(node.child_by_field_name(field).is_some()),
            Self::Operator(tokens) => u32::from(
                node.child_by_field_name("operator")
                    .is_some_and(|operator| tokens.contains(&operator.kind())),
            ),
            Self::Tokens(tokens) => {
                let mut cursor = node.walk();
                let matching = node
                    .children(&mut cursor)
                    .filter(|child| !child.is_named() && tokens.contains(&child.kind()))
                    .count();
                u32::try_from(matching).unwrap_or(u32::MAX)
            }
            Self::UnlessLast => {
                let mut sibling = node.next_named_sibling();
                while let Some(next) = sibling {
                    if next.kind() == node.kind() {
                        return 1;
                    }
                    sibling = next.next_named_sibling();
                }
                0
            }
            Self::UnlessFieldKind { field, kind } => {
                u32::from(node.child_by_field_name(field).is_none_or(|value| value.kind() != *kind))
            }
            Self::Arm(field) => {
                let mut sibling = node.next_named_sibling();
                while let Some(next) = sibling {
                    if next.kind() != "comment" {
                        return u32::from(node.child_by_field_name(field).is_some());
                    }
                    sibling = next.next_named_sibling();
                }
                1
            }
        }
    }
}

#[cfg(test)]
mod tests;

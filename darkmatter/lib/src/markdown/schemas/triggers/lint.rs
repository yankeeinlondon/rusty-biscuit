//! Vacuous-trigger lint.
//!
//! Per arm, the lint statically detects that **no satisfiable path** through
//! the boolean tree contains a presence-requiring condition. A
//! presence-requiring condition is a `required` gate or a `$path` predicate
//! that sits inside an `all`/`any`/`min-match` (not under `none`). A vacuous
//! arm matches every document, and because arms OR together, one vacuous arm
//! makes the whole trigger vacuous — so a vacuous arm is a **load-time
//! error**.
//!
//! ## Why a load-time error
//!
//! Consistent with the fail-loud posture of `example(...)` validation: a
//! trigger that fires on every document is almost always an authoring
//! mistake (a missing `required` gate), and silent activation on look-alike
//! documents is exactly the false-positive surface the typed grammar exists
//! to design away.

use crate::markdown::schemas::errors::SchemaError;

use super::grammar::MatchArms;

/// Runs the vacuous-trigger lint over every arm.
///
/// Returns [`SchemaError::TriggerVacuousArm`] for the first vacuous arm
/// found. Because arms OR together, one vacuous arm is sufficient to reject
/// the whole trigger.
pub fn lint(arms: &MatchArms) -> Result<(), SchemaError> {
    for arm in &arms.0 {
        if is_vacuous_arm(arm) {
            return Err(SchemaError::TriggerVacuousArm);
        }
    }
    Ok(())
}

/// Returns `true` when an arm has no satisfiable path through the tree that
/// contains a presence-requiring condition.
///
/// An arm is vacuous when it is satisfiable (not structurally contradictory)
/// **and** has no presence requirement. A tree is "presence-requiring" when
/// some satisfiable evaluation forces a `required` key to be present or a
/// `$path` to match.
fn is_vacuous_arm(expr: &super::grammar::MatchExpr) -> bool {
    // The arm is vacuous iff it can be satisfied without any presence
    // requirement. `requires_presence` returns true when the expression
    // forces at least one gate; if an arm is satisfiable AND has no presence
    // requirement, it matches every document.
    !requires_presence(expr)
}

/// Returns `true` when the expression forces at least one presence-requiring
/// condition along every satisfiable path. A presence-requiring condition is:
///
/// - A `required` gate on a property condition.
/// - A `$path` predicate, when it is not the sole child of a `none`.
///
/// Under `none`, presence requirements invert: a `required` inside `none`
/// does **not** make the arm presence-requiring (it forbids a key, which is
/// satisfied by absence).
fn requires_presence(expr: &super::grammar::MatchExpr) -> bool {
    use super::grammar::MatchExpr;
    match expr {
        // `all`: every child must hold, so the presence requirement of any
        // child is a real requirement.
        MatchExpr::All(children) => children.iter().any(requires_presence),
        // `any`: at least one child holds. The arm can still be satisfied by a
        // non-presence-requiring child, so `any` is presence-requiring only
        // when *every* child is presence-requiring (whichever arm fires, a
        // gate is present). If any child lacks a presence requirement, the
        // arm can be satisfied vacuously through that child.
        MatchExpr::Any(children) => !children.is_empty() && children.iter().all(requires_presence),
        // `none`: a child holding is forbidden. Presence requirements inside
        // `none` do not contribute to the arm's presence requirement (they
        // forbid rather than require). However, `none` with a presence-
        // requiring child is still "satisfiable without presence" — the
        // document simply lacks the forbidden key.
        MatchExpr::None(_) => false,
        // `min-match`: N of M must hold. The arm can be satisfied without any
        // presence requirement when the number of non-presence-requiring
        // children is at least `count` (the quota can be filled with guards
        // alone). So it is presence-requiring iff fewer than `count` children
        // lack a presence requirement — forcing at least one gate to fire.
        MatchExpr::MinMatch { count, of } => {
            let non_present = of.iter().filter(|c| !requires_presence(c)).count();
            non_present < *count
        }
        // A property condition is presence-requiring iff it is a gate
        // (`required`). A guard (no `required`) can be satisfied by absence.
        MatchExpr::Property { atom, .. } => is_gate(atom),
        // `$path` is inherently a gate (the path always exists).
        MatchExpr::Path(_) => true,
    }
}

fn is_gate(atom: &crate::markdown::schemas::simplified::PropertyAtom) -> bool {
    use crate::markdown::schemas::simplified::Constraint;
    atom.constraints.iter().any(|c| matches!(c, Constraint::Required))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::{Constraint, PropertyAtom, SimplifiedType, TypeExpr};
    use crate::markdown::schemas::triggers::grammar::{MatchArms, MatchExpr, PathGlobs};

    fn required_prop(name: &str) -> MatchExpr {
        MatchExpr::Property {
            name: name.into(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            },
        }
    }

    fn guard_prop(name: &str) -> MatchExpr {
        MatchExpr::Property {
            name: name.into(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![],
                array_constraints: vec![],
                description: None,
            },
        }
    }

    #[test]
    fn rejects_guard_only_arm() {
        let arms = MatchArms(vec![guard_prop("maybe")]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }

    #[test]
    fn accepts_required_arm() {
        let arms = MatchArms(vec![required_prop("must")]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn accepts_path_arm() {
        let arms = MatchArms(vec![MatchExpr::Path(PathGlobs {
            patterns: vec!["**/*.md".into()],
        })]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn any_with_one_guard_child_is_vacuous() {
        // `any: [required, guard]` — the guard child can satisfy the arm
        // without presence, so the whole arm is vacuous.
        let arms = MatchArms(vec![MatchExpr::Any(vec![
            required_prop("must"),
            guard_prop("maybe"),
        ])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }

    #[test]
    fn any_with_all_required_children_is_ok() {
        let arms = MatchArms(vec![MatchExpr::Any(vec![
            required_prop("a"),
            required_prop("b"),
        ])]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn none_does_not_contribute_presence() {
        // `none: [required]` forbids the key; the arm is satisfied by absence,
        // so it is vacuous unless there is another presence-requiring sibling.
        let arms = MatchArms(vec![MatchExpr::None(vec![required_prop("forbidden")])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }

    #[test]
    fn all_with_none_and_required_is_ok() {
        let arms = MatchArms(vec![MatchExpr::All(vec![
            MatchExpr::None(vec![required_prop("forbidden")]),
            required_prop("must"),
        ])]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn one_vacuous_arm_rejects_whole_trigger() {
        let arms = MatchArms(vec![
            required_prop("must"),
            guard_prop("maybe"),
        ]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }

    #[test]
    fn empty_arm_is_vacuous() {
        // An `all` with no children matches everything.
        let arms = MatchArms(vec![MatchExpr::All(vec![])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }

    #[test]
    fn min_match_with_enough_required_is_ok() {
        // min-match count=1, of=[required, required] — whichever fires, a gate
        // is present.
        let arms = MatchArms(vec![MatchExpr::MinMatch {
            count: 1,
            of: vec![required_prop("a"), required_prop("b")],
        }]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn min_match_with_skippable_guard_is_vacuous() {
        // min-match count=1, of=[required, guard] — the arm can fire through
        // the guard alone.
        let arms = MatchArms(vec![MatchExpr::MinMatch {
            count: 1,
            of: vec![required_prop("a"), guard_prop("b")],
        }]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousArm)));
    }
}

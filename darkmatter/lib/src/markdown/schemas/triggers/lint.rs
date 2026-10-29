//! Vacuous-trigger lint.
//!
//! Over the whole match expression (the top-level conditions AND'd), the lint
//! statically detects that **some way of satisfying it** includes no
//! presence-requiring condition. A presence-requiring condition is a
//! `required` gate or a `$path` predicate that sits inside an
//! `all`/`group`/`min-match` (not under `none`). Such a trigger can match a
//! document that has none of its keys, so it is a **load-time error**.
//!
//! ## Why a load-time error
//!
//! Consistent with the fail-loud posture of `example(...)` validation: a
//! trigger that fires on every document is almost always an authoring
//! mistake (a missing `required` gate), and silent activation on look-alike
//! documents is exactly the false-positive surface the typed grammar exists
//! to design away.

use crate::markdown::schemas::errors::SchemaError;

use super::grammar::MatchConditions;

/// Runs the vacuous-trigger lint over the whole match expression.
///
/// Returns [`SchemaError::TriggerVacuousMatch`] when the AND of `conditions`
/// can be satisfied without any presence requirement.
pub fn lint(conditions: &MatchConditions) -> Result<(), SchemaError> {
    if requires_presence(&conditions.to_expr()) {
        Ok(())
    } else {
        Err(SchemaError::TriggerVacuousMatch)
    }
}

/// Returns `true` when the expression forces at least one presence-requiring
/// condition along every satisfiable path. A presence-requiring condition is:
///
/// - A `required` gate on a property condition.
/// - A `$path` predicate, when it is not the sole child of a `none`.
///
/// Under `none`, presence requirements invert: a `required` inside `none`
/// does **not** make the expression presence-requiring (it forbids a key,
/// which is satisfied by absence).
fn requires_presence(expr: &super::grammar::MatchExpr) -> bool {
    use super::grammar::MatchExpr;
    match expr {
        // `all`: every child must hold, so the presence requirement of any
        // child is a real requirement.
        MatchExpr::All(children) => children.iter().any(requires_presence),
        // `group`: at least one child holds. The group can be satisfied by a
        // non-presence-requiring child, so it is presence-requiring only
        // when *every* child is presence-requiring (whichever alternative
        // holds, a gate is present).
        MatchExpr::Group(children) => !children.is_empty() && children.iter().all(requires_presence),
        // `none`: a child holding is forbidden. Presence requirements inside
        // `none` do not contribute to a presence requirement (they
        // forbid rather than require). However, `none` with a presence-
        // requiring child is still "satisfiable without presence" — the
        // document simply lacks the forbidden key.
        MatchExpr::None(_) => false,
        // `min-match`: N of M must hold. It can be satisfied without any
        // presence requirement when the number of non-presence-requiring
        // children is at least `count` (the quota can be filled with guards
        // alone). So it is presence-requiring iff fewer than `count` children
        // lack a presence requirement — forcing at least one gate to fire.
        MatchExpr::MinMatch { count, of } => {
            let non_present = of.iter().filter(|c| !requires_presence(c)).count();
            non_present < *count
        }
        // A property condition is presence-requiring iff it is a gate
        // (effectively `required`). A guard, including `unknown(required)` and
        // `null(required)`, can be satisfied by absence.
        MatchExpr::Property { atom, .. } => atom.is_required(),
        // `$path` is inherently a gate (the path always exists).
        MatchExpr::Path(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::{Constraint, PropertyAtom, SimplifiedType, TypeExpr};
    use crate::markdown::schemas::triggers::grammar::{MatchConditions, MatchExpr, PathGlobs};

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
        let arms = MatchConditions(vec![guard_prop("maybe")]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn accepts_required_arm() {
        let arms = MatchConditions(vec![required_prop("must")]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn accepts_path_arm() {
        let arms = MatchConditions(vec![MatchExpr::Path(PathGlobs::new(vec!["**/*.md".into()]).unwrap())]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn group_with_one_guard_child_is_vacuous() {
        // `group: [required, guard]` — the guard child can satisfy the
        // group without presence, so the whole expression is vacuous.
        let arms = MatchConditions(vec![MatchExpr::Group(vec![
            required_prop("must"),
            guard_prop("maybe"),
        ])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn group_with_all_required_children_is_ok() {
        let arms = MatchConditions(vec![MatchExpr::Group(vec![
            required_prop("a"),
            required_prop("b"),
        ])]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn none_does_not_contribute_presence() {
        // `none: [required]` forbids the key; the arm is satisfied by absence,
        // so it is vacuous unless there is another presence-requiring sibling.
        let arms = MatchConditions(vec![MatchExpr::None(vec![required_prop("forbidden")])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn all_with_none_and_required_is_ok() {
        let arms = MatchConditions(vec![MatchExpr::All(vec![
            MatchExpr::None(vec![required_prop("forbidden")]),
            required_prop("must"),
        ])]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn top_level_list_is_and_so_one_gate_covers_a_guard_sibling() {
        let arms = MatchConditions(vec![
            required_prop("must"),
            guard_prop("maybe"),
        ]);
        assert!(lint(&arms).is_ok());
        let guards = MatchConditions(vec![guard_prop("maybe"), guard_prop("other")]);
        assert!(matches!(lint(&guards), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn empty_all_is_vacuous() {
        // An `all` with no children matches everything.
        let arms = MatchConditions(vec![MatchExpr::All(vec![])]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn min_match_with_enough_required_is_ok() {
        // min-match count=1, of=[required, required] — whichever fires, a gate
        // is present.
        let arms = MatchConditions(vec![MatchExpr::MinMatch {
            count: 1,
            of: vec![required_prop("a"), required_prop("b")],
        }]);
        assert!(lint(&arms).is_ok());
    }

    #[test]
    fn min_match_with_skippable_guard_is_vacuous() {
        // min-match count=1, of=[required, guard] — it can hold through
        // the guard alone.
        let arms = MatchConditions(vec![MatchExpr::MinMatch {
            count: 1,
            of: vec![required_prop("a"), guard_prop("b")],
        }]);
        assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)));
    }

    #[test]
    fn null_admitting_required_condition_is_a_guard_but_a_required_list_is_a_gate() {
        for ty in [SimplifiedType::Unknown, SimplifiedType::Null] {
            let arms = MatchConditions(vec![MatchExpr::Property {
                name: "steps".into(),
                atom: PropertyAtom {
                    ty: TypeExpr::Primitive(ty),
                    is_array: false,
                    constraints: vec![Constraint::Required],
                    array_constraints: vec![],
                    description: None,
                },
            }]);
            assert!(matches!(lint(&arms), Err(SchemaError::TriggerVacuousMatch)), "{ty:?}");
        }
        let list = MatchConditions(vec![MatchExpr::Property {
            name: "steps".into(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::Unknown),
                is_array: true,
                constraints: vec![],
                array_constraints: vec![Constraint::Required],
                description: None,
            },
        }]);
        assert!(lint(&list).is_ok(), "`unknown[](required)` is a gate");
    }
}

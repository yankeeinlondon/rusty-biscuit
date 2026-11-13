//! Launch-phase reports completed with the defaults composition applies.

use super::*;

/// Completes a [`SchemaPhase::Launch`] `report` with the defaults
/// composition applies.
///
/// The launch projection drops `required` from a property that is not eager,
/// so its launch report neither lists such a property's `default(...)` nor
/// refuses a broken one, while composition fills it from the unphased
/// contract. When an absent property of that kind declares a default, this
/// judges `instance` unphased and copies, for those properties only, its
/// defaults into `report.defaults` and its other problems (a refused default)
/// into `report.problems`. Reports of any other phase are left as they are.
pub(super) fn complete_launch_defaults(
    effective: &EffectiveSchema,
    phase: Option<SchemaPhase>,
    instance: &serde_json::Value,
    report: &mut ValidationReport,
) {
    if phase != Some(SchemaPhase::Launch) {
        return;
    }
    let Some(SchemaRoot::Single(shape)) = effective.simplified.as_ref() else {
        return;
    };
    let hidden: Vec<&str> = shape
        .properties
        .iter()
        .filter(|(name, def)| {
            def.is_required()
                && !is_eager(Some(shape), name)
                && instance.get(name.as_str()).is_none()
                && any_atom_constraint(def, |c| matches!(c, Constraint::Default(_)))
        })
        .map(|(name, _)| name.as_str())
        .collect();
    if hidden.is_empty() {
        return;
    }
    // Unphased, `validate_stored` never fails.
    let Ok(unphased) = effective.validate_stored(instance, &PositionMap::new(), None) else {
        return;
    };
    let at_hidden = |name: Option<&str>| name.is_some_and(|name| hidden.contains(&name));
    report.defaults.extend(
        unphased
            .defaults
            .into_iter()
            .filter(|applied| at_hidden(applied.path.segments().first().map(String::as_str))),
    );
    let refused: Vec<ValidationProblem> = unphased
        .problems
        .into_iter()
        .filter(|problem| {
            !matches!(problem.kind, ValidationProblemKind::Missing)
                && at_hidden(top_level_pointer_segment(&problem.path).as_deref())
        })
        .collect();
    if !refused.is_empty() {
        report.problems.extend(refused);
        report.valid = false;
    }
}

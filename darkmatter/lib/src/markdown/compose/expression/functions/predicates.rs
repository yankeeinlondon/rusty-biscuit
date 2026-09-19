use super::{EvaluationMode, FunctionBinding, FunctionHandler};

/// The absence predicates: a direct bare-variable argument to one of these
/// states that the author expects the value may be missing, so it is not an
/// unknown-identifier warning (spec Requirement 4). Membership in this group
/// is the marker; walkers ask [`super::is_absence_predicate`] and never spell
/// the names.
///
/// Interim (spec Resolved Decision 9): the expression type-system successor
/// replaces this with "any parameter whose type admits null".
pub(super) const ABSENCE_PREDICATES: &[FunctionBinding] = &[
    FunctionBinding { canonical: "is_null", aliases: &["isnull"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_null)) },
    FunctionBinding { canonical: "is_empty", aliases: &["isempty"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_empty_fn)) },
];

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "is_string", aliases: &["isstring"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_string)) },
    FunctionBinding { canonical: "is_number", aliases: &["isnumber"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_number)) },
    FunctionBinding { canonical: "is_array", aliases: &["isarray"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_array)) },
    FunctionBinding { canonical: "is_object", aliases: &["isobject"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_object)) },
    FunctionBinding { canonical: "is_positive", aliases: &["ispositive"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_positive)) },
    FunctionBinding { canonical: "is_negative", aliases: &["isnegative"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_negative)) },
    FunctionBinding { canonical: "is_integer", aliases: &["isinteger"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_integer)) },
    FunctionBinding { canonical: "min", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::min_fn)) },
    FunctionBinding { canonical: "max", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::max_fn)) },
    FunctionBinding { canonical: "abs", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::abs_fn)) },
    FunctionBinding { canonical: "round", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::round_fn)) },
];

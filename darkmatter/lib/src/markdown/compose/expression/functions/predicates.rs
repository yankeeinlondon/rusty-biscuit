use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "is_string", aliases: &["isstring"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_string)) },
    FunctionBinding { canonical: "is_number", aliases: &["isnumber"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_number)) },
    FunctionBinding { canonical: "is_array", aliases: &["isarray"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_array)) },
    FunctionBinding { canonical: "is_null", aliases: &["isnull"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_null)) },
    FunctionBinding { canonical: "is_object", aliases: &["isobject"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_object)) },
    FunctionBinding { canonical: "is_empty", aliases: &["isempty"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_empty_fn)) },
    FunctionBinding { canonical: "is_positive", aliases: &["ispositive"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_positive)) },
    FunctionBinding { canonical: "is_negative", aliases: &["isnegative"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_negative)) },
    FunctionBinding { canonical: "is_integer", aliases: &["isinteger"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_integer)) },
    FunctionBinding { canonical: "min", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::min_fn)) },
    FunctionBinding { canonical: "max", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::max_fn)) },
    FunctionBinding { canonical: "abs", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::abs_fn)) },
    FunctionBinding { canonical: "round", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::round_fn)) },
];

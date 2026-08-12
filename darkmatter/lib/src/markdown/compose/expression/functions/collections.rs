use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "first", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::first_fn)) },
    FunctionBinding { canonical: "last", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::last_fn)) },
    FunctionBinding { canonical: "has_key", aliases: &["haskey"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::has_key_fn)) },
    FunctionBinding { canonical: "contains", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::contains_fn)) },
    FunctionBinding { canonical: "length", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::length_fn)) },
    FunctionBinding { canonical: "number", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::number_fn)) },
    FunctionBinding { canonical: "as_line_separated", aliases: &["aslineseparated"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_line_separated)) },
    FunctionBinding { canonical: "as_csv", aliases: &["ascsv"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_csv)) },
    FunctionBinding { canonical: "as_tsv", aliases: &["astsv"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_tsv)) },
    FunctionBinding { canonical: "as_space_separated", aliases: &["asspaceseparated"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_space_separated)) },
    FunctionBinding { canonical: "as_unordered_list", aliases: &["asunorderedlist"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_unordered_list)) },
    FunctionBinding { canonical: "as_ordered_list", aliases: &["asorderedlist"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::as_ordered_list)) },
];

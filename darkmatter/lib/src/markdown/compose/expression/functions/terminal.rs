use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "terminal", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::terminal)) },
];

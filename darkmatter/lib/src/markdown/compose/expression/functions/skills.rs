use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "has_skill", aliases: &["hasskill"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::has_skill_fn)) },
    FunctionBinding { canonical: "has_local_skill", aliases: &["haslocalskill"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::has_local_skill_fn)) },
];

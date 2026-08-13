use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "starts_with", aliases: &["startswith"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::starts_with)) },
    FunctionBinding { canonical: "ends_with", aliases: &["endswith"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::ends_with)) },
    FunctionBinding { canonical: "lower", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::lower)) },
    FunctionBinding { canonical: "upper", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::upper)) },
    FunctionBinding { canonical: "capitalize", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::capitalize)) },
    FunctionBinding { canonical: "kebab_case", aliases: &["kebabcase"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::kebab_case)) },
    FunctionBinding { canonical: "snake_case", aliases: &["snakecase"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::snake_case)) },
    FunctionBinding { canonical: "camel_case", aliases: &["camelcase"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::camel_case)) },
    FunctionBinding { canonical: "pascal_case", aliases: &["pascalcase"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::pascal_case)) },
    FunctionBinding { canonical: "title_case", aliases: &["titlecase"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::title_case)) },
    FunctionBinding { canonical: "without_date", aliases: &["withoutdate"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::without_date)) },
    FunctionBinding { canonical: "ensure_leading", aliases: &["ensureleading"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::ensure_leading)) },
    FunctionBinding { canonical: "ensure_trailing", aliases: &["ensuretrailing"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::ensure_trailing)) },
    FunctionBinding { canonical: "replace", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::replace)) },
    FunctionBinding { canonical: "replace_first", aliases: &["replacefirst"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::replace_first)) },
    FunctionBinding { canonical: "replace_last", aliases: &["replacelast"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::replace_last)) },
    FunctionBinding { canonical: "raw_markdown", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::raw_markdown)) },
];

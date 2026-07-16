use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "frontmatter", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::frontmatter_fn)) },
    FunctionBinding { canonical: "markdown_body_empty", aliases: &["markdownbodyempty"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::markdown_body_empty_fn)) },
    FunctionBinding { canonical: "markdown_title", aliases: &["markdowntitle"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::markdown_title_fn)) },
    FunctionBinding { canonical: "validate_schema", aliases: &["validateschema"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::validate_schema_fn)) },
];

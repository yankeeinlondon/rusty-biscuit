use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "absolute", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::absolute_fn)) },
    FunctionBinding { canonical: "relative", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::relative_fn)) },
    FunctionBinding { canonical: "file_exists", aliases: &["fileexists"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::file_exists_fn)) },
    FunctionBinding { canonical: "has_command", aliases: &["hascommand"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::has_command_fn)) },
    FunctionBinding { canonical: "is_indexed_file", aliases: &["isindexedfile"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::is_indexed_file_fn)) },
    FunctionBinding { canonical: "file_index", aliases: &["fileindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::file_index_fn)) },
    FunctionBinding { canonical: "increment_file_index", aliases: &["incrementfileindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::increment_file_index_fn)) },
    FunctionBinding { canonical: "decrement_file_index", aliases: &["decrementfileindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::decrement_file_index_fn)) },
    FunctionBinding { canonical: "find_first_index", aliases: &["findfirstindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::find_first_index_fn)) },
    FunctionBinding { canonical: "find_last_index", aliases: &["findlastindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::find_last_index_fn)) },
    FunctionBinding { canonical: "basename", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::basename_fn)) },
    FunctionBinding { canonical: "basename_without_index", aliases: &["basenamewithoutindex"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::basename_without_index_fn)) },
    FunctionBinding { canonical: "dirname", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::dirname_fn)) },
    FunctionBinding { canonical: "ext", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::ext_fn)) },
    FunctionBinding { canonical: "parent_dir", aliases: &["parentdir"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::parent_dir_fn)) },
    FunctionBinding { canonical: "file_trailing", aliases: &["filetrailing"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::file_trailing_fn)) },
    FunctionBinding { canonical: "dir_leading", aliases: &["dirleading"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::dir_leading_fn)) },
    FunctionBinding { canonical: "join", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::join_fn)) },
    FunctionBinding { canonical: "link", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(super::link_fn)) },
];

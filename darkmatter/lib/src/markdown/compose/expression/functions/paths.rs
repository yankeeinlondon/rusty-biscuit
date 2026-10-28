use super::FunctionBinding;
use crate::markdown::compose::expression::call_binding::FactKind::TrueImplies;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("absolute", &[], super::absolute_fn),
    FunctionBinding::context("relative", &[], super::relative_fn),
    // The handler returns false for `null`, every non-text value, and
    // malformed reference text, which it never looks up. Only text a file
    // parameter can take is ever true, so the true side proves `file`, as
    // `is_file_type` does; the false side proves nothing, since a valid
    // reference may name a file that is absent.
    FunctionBinding::context("file_exists", &["fileexists"], super::file_exists_fn)
        .narrowing("value", TrueImplies, Some("file"))
        .false_for_malformed_reference("value"),
    FunctionBinding::context("find_files", &["findfiles"], super::find_files_fn),
    FunctionBinding::context("has_command", &["hascommand"], super::has_command_fn),
    // `has_binary` is a second documented name for the same probe (R16), not an
    // alias: both names must appear in the descriptor catalog.
    FunctionBinding::context("has_binary", &["hasbinary"], super::has_command_fn),
    FunctionBinding::context("is_indexed_file", &["isindexedfile"], super::is_indexed_file_fn),
    FunctionBinding::context("file_index", &["fileindex"], super::file_index_fn),
    FunctionBinding::context("increment_file_index", &["incrementfileindex"], super::increment_file_index_fn),
    FunctionBinding::context("decrement_file_index", &["decrementfileindex"], super::decrement_file_index_fn),
    FunctionBinding::context("find_first_index", &["findfirstindex"], super::find_first_index_fn),
    FunctionBinding::context("find_last_index", &["findlastindex"], super::find_last_index_fn),
    FunctionBinding::context("basename", &[], super::basename_fn),
    FunctionBinding::context("basename_without_index", &["basenamewithoutindex"], super::basename_without_index_fn),
    FunctionBinding::context("dirname", &[], super::dirname_fn),
    FunctionBinding::context("ext", &[], super::ext_fn),
    FunctionBinding::context("parent_dir", &["parentdir"], super::parent_dir_fn),
    FunctionBinding::context("file_trailing", &["filetrailing"], super::file_trailing_fn),
    FunctionBinding::context("dir_leading", &["dirleading"], super::dir_leading_fn),
    FunctionBinding::context("join", &[], super::join_fn),
    FunctionBinding::context("link", &[], super::link_fn),
    FunctionBinding::context("code_link", &["codelink"], super::code_link_fn),
];

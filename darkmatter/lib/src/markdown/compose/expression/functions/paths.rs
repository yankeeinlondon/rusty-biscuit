use super::{FunctionBinding, ReadKind, SlotClass};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("absolute", &[], super::absolute_fn).reads(ReadKind::Probe),
    FunctionBinding::context("relative", &[], super::relative_fn).reads(ReadKind::Probe),
    // The handler returns false for `null`, every non-text value, and
    // malformed reference text, which it never looks up. Only text a file
    // parameter can take is ever true, so the true side proves `file`, as
    // `is_file_type` does; the false side proves nothing, since a valid
    // reference may name a file that is absent.
    FunctionBinding::context("file_exists", &["fileexists"], super::file_exists_fn).reads_or_fetches(ReadKind::Probe)
        .false_for_malformed_reference("value"),
    FunctionBinding::context("find_files", &["findfiles"], super::find_files_fn).reads(ReadKind::Listing),
    FunctionBinding::context("has_command", &["hascommand"], super::has_command_fn).slot(SlotClass::Captured),
    // `has_binary` is a second documented name for the same probe (R16), not an
    // alias: both names must appear in the descriptor catalog.
    FunctionBinding::context("has_binary", &["hasbinary"], super::has_command_fn).slot(SlotClass::Captured),
    FunctionBinding::context("is_indexed_file", &["isindexedfile"], super::is_indexed_file_fn).reads(ReadKind::Probe),
    FunctionBinding::context("file_index", &["fileindex"], super::file_index_fn).reads(ReadKind::Probe),
    FunctionBinding::context("increment_file_index", &["incrementfileindex"], super::increment_file_index_fn).reads(ReadKind::Probe),
    FunctionBinding::context("decrement_file_index", &["decrementfileindex"], super::decrement_file_index_fn).reads(ReadKind::Probe),
    FunctionBinding::context("find_first_index", &["findfirstindex"], super::find_first_index_fn).reads(ReadKind::Listing),
    FunctionBinding::context("find_last_index", &["findlastindex"], super::find_last_index_fn).reads(ReadKind::Listing),
    FunctionBinding::context("basename", &[], super::basename_fn).reads(ReadKind::Probe),
    FunctionBinding::context("basename_without_index", &["basenamewithoutindex"], super::basename_without_index_fn).reads(ReadKind::Probe),
    FunctionBinding::context("dirname", &[], super::dirname_fn).reads(ReadKind::Probe),
    FunctionBinding::context("ext", &[], super::ext_fn).reads(ReadKind::Probe),
    FunctionBinding::context("parent_dir", &["parentdir"], super::parent_dir_fn).reads(ReadKind::Probe),
    FunctionBinding::context("file_trailing", &["filetrailing"], super::file_trailing_fn).reads(ReadKind::Probe),
    FunctionBinding::context("dir_leading", &["dirleading"], super::dir_leading_fn).reads(ReadKind::Probe),
    FunctionBinding::context("join", &[], super::join_fn).reads(ReadKind::Probe),
    FunctionBinding::context("link", &[], super::link_fn).reads(ReadKind::Probe),
    FunctionBinding::context("code_link", &["codelink"], super::code_link_fn).reads(ReadKind::Probe),
];

use super::{FunctionBinding, ReadKind};
use crate::markdown::compose::expression::data::load;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("load_data", &["loaddata"], load::load_data_fn).reads_or_fetches(ReadKind::Content)
        .refining_binding_errors(load::refine_binding_error),
];

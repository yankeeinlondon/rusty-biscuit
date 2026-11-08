use super::{FunctionBinding, ReadKind};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("frontmatter", &[], super::frontmatter_fn).reads_or_fetches(ReadKind::Content),
    FunctionBinding::context("try_frontmatter", &["tryfrontmatter"], super::try_frontmatter_fn).reads_or_fetches(ReadKind::Content),
    FunctionBinding::context("markdown_body_empty", &["markdownbodyempty"], super::markdown_body_empty_fn).reads_or_fetches(ReadKind::Content),
    FunctionBinding::context("markdown_title", &["markdowntitle"], super::markdown_title_fn).reads_or_fetches(ReadKind::Content),
    FunctionBinding::context("validate_schema", &["validateschema"], super::validate_schema_fn).reads_or_fetches(ReadKind::Content),
];

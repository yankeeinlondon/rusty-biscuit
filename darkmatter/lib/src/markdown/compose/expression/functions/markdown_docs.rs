use super::FunctionBinding;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("frontmatter", &[], super::frontmatter_fn),
    FunctionBinding::context("try_frontmatter", &["tryfrontmatter"], super::try_frontmatter_fn),
    FunctionBinding::context("markdown_body_empty", &["markdownbodyempty"], super::markdown_body_empty_fn),
    FunctionBinding::context("markdown_title", &["markdowntitle"], super::markdown_title_fn),
    FunctionBinding::context("validate_schema", &["validateschema"], super::validate_schema_fn),
];

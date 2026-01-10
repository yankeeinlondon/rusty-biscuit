
/// The **SmartConcat** struct is a functional grouping operation
/// which takes 2 or more markdown documents and creates a single
/// document by concatenating them together.
///
/// Concatenation is achieved using the following logic:
///
/// 1. Normalize to `Markdown` struct - "Documents" can come in as a proper
///    `Markdown` struct or as a string variant (`String` or `&str`) but the
///    first step will be to convert them all into the `Markdown` type.
pub struct SmartConcat {}

/// Stateless tree-sitter node extraction helpers used by `tree_file`.
pub(crate) mod extract;
/// Tree-sitter-backed file parsing utilities.
pub mod tree_file;

/// Revision of the symbol-extraction rules. Bump it in any change that alters
/// extracted records, so persistent caches keyed on it are invalidated and a
/// record's `provenance.extractor_version` names the rules that produced it.
pub const EXTRACTOR_REVISION: &str = "1";

/// Stateless tree-sitter node extraction helpers used by `tree_file`.
pub(crate) mod extract;
/// Tree-sitter-backed file parsing utilities.
pub mod tree_file;

/// Revision of the symbol-extraction rules. Bump it in any change that alters
/// extracted records, so persistent caches keyed on it are invalidated and a
/// record's `provenance.extractor_version` names the rules that produced it.
///
/// That includes repairs on an unreleased branch (a warm cache built from an
/// earlier commit would otherwise keep serving the old records), `.scm` query
/// edits and grammar crate upgrades: the CLI key holds no query hash or
/// grammar version, so this constant is the only component that moves.
pub const EXTRACTOR_REVISION: &str = "4";

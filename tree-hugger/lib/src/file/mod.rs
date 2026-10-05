/// Stateless tree-sitter node extraction helpers used by `tree_file`.
pub(crate) mod extract;
/// Selective symbol queries (`SymbolQuery`, `Capability`).
pub mod query;
/// A source file read but not yet parsed (`SourceFile`).
pub mod source_file;
/// Tree-sitter-backed file parsing utilities.
pub mod tree_file;

/// Revision of the symbol-extraction rules. Bump it in any change that alters
/// extracted records, so persistent caches keyed on it are invalidated and a
/// record's `provenance.extractor_version` names the rules that produced it.
///
/// That includes repairs on an unreleased branch (a warm cache built from an
/// earlier commit would otherwise keep serving the old records). The cache
/// key already digests the grammar and the effective query text, so this
/// constant is what moves for a change in Rust extraction code.
pub const EXTRACTOR_REVISION: &str = "8";

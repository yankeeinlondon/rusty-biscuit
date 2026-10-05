//! Per-symbol metrics (`Capability::Metrics`).

use super::*;
use crate::shared::MetricsFacet;

/// The metrics of a function-like or other declaration.
///
/// `parameters` is the length of the declaration's own parameter list,
/// without a Rust `self` receiver (a Go receiver has its own list and is
/// never counted); C, C++ and Go count unnamed parameters too
/// ([`declared_parameter_count`]). `None` for kinds that are not functions
/// or where no list is found ([`declared_parameters`]). `has_docs` is whether `doc` (from
/// [`extract_doc_comment`]) is present; `None` in Lua, Perl, Bash and Zsh,
/// which have no doc-comment detection.
pub(crate) fn symbol_metrics(
    kind: SymbolKind,
    declaration: Node<'_>,
    name: Node<'_>,
    doc: Option<&str>,
    language: ProgrammingLanguage,
    source: &str,
) -> MetricsFacet {
    let parameters = kind
        .is_function()
        .then(|| {
            declared_parameter_count(declaration, name, language, source).or_else(|| {
                declared_parameters(declaration, name, language, source).map(|parameters| {
                    let receivers = usize::from(
                        language == ProgrammingLanguage::Rust
                            && parameters.first().is_some_and(|first| first.name == "self"),
                    );
                    parameters.len() - receivers
                })
            })
        })
        .flatten()
        .map(|count| u32::try_from(count).unwrap_or(u32::MAX));
    let has_docs = match language {
        ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => None,
        _ => Some(doc.is_some()),
    };
    MetricsFacet { parameters, has_docs }
}

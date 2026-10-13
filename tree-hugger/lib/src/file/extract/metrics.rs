//! Per-symbol metrics (`Capability::Metrics`).

use super::*;
use crate::shared::MetricsFacet;

/// The metrics of a function-like or other declaration.
///
/// `parameters` is the length of the declaration's own parameter list (all
/// curried Scala lists), whatever binds each parameter (an identifier, a
/// destructuring pattern or `_`), without a Rust `self` receiver (a Go
/// receiver has its own list, and Java and TypeScript receiver parameters
/// are never listed); C, C++ and Go count unnamed parameters too
/// ([`declared_parameter_count`]). `None` for kinds that are not functions
/// or where no list is found ([`declared_parameters`]). `has_docs` is whether `doc` (from
/// [`extract_doc_comment`]) is present; `None` in Lua, Perl, Bash and Zsh,
/// which have no doc-comment detection. `body` is the callable's
/// [`callable_body`] (`None` for other kinds): `lines` is [`body_lines`] of
/// its span, the record's `source.body_span`, and `cyclomatic` is
/// [`cyclomatic`] over its nodes, so both measure the same body.
pub(crate) fn symbol_metrics(
    kind: SymbolKind,
    declaration: Node<'_>,
    name: Node<'_>,
    doc: Option<&str>,
    body: Option<CallableBody<'_>>,
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
    MetricsFacet {
        parameters,
        has_docs,
        lines: body_lines(kind, body.map(|body| body.span(source)).as_ref()),
        cyclomatic: body.filter(|_| kind.is_function()).and_then(|body| cyclomatic(body, language)),
    }
}

/// The physical lines a function, method or constructor's body span
/// ([`CallableBody::span`]) touches: `0` for an empty span, otherwise
/// `end.line - start.line`, plus one unless the span ends at column 1 (it
/// then stops before that line's first byte). With one-based locations,
/// CRLF and LF sources count the same. `None` for other kinds, even with a
/// body span, and for a callable without a body.
pub(crate) fn body_lines(kind: SymbolKind, body_span: Option<&TextSpan>) -> Option<u32> {
    let span = body_span.filter(|_| kind.is_function())?;
    if span.start_byte == span.end_byte {
        return Some(0);
    }
    Some(span.end.line - span.start.line + u32::from(span.end.column > 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: (u32, u32, u32), end: (u32, u32, u32)) -> TextSpan {
        TextSpan {
            start: TextPoint { line: start.0, column: start.1 },
            end: TextPoint { line: end.0, column: end.1 },
            start_byte: start.2,
            end_byte: end.2,
        }
    }

    /// The spec's examples: `{}` on one line is 1, lines 3 to 6 ending at
    /// column 1 is 3, an empty span is 0, other kinds and absent bodies are
    /// `None`.
    #[test]
    fn body_lines_follow_the_spec_formula() {
        let braces = span((1, 10, 9), (1, 12, 11));
        let to_column_one = span((3, 5, 40), (6, 1, 90));
        let to_column_two = span((3, 5, 40), (6, 2, 91));
        let empty = span((2, 3, 20), (2, 3, 20));
        for kind in [SymbolKind::Function, SymbolKind::Method, SymbolKind::Constructor] {
            assert_eq!(body_lines(kind, Some(&braces)), Some(1));
            assert_eq!(body_lines(kind, Some(&to_column_one)), Some(3));
            assert_eq!(body_lines(kind, Some(&to_column_two)), Some(4));
            assert_eq!(body_lines(kind, Some(&empty)), Some(0));
            assert_eq!(body_lines(kind, None), None);
        }
        for kind in [SymbolKind::Class, SymbolKind::Module, SymbolKind::Variable, SymbolKind::Type] {
            assert_eq!(body_lines(kind, Some(&to_column_two)), None, "{kind:?}");
        }
    }
}

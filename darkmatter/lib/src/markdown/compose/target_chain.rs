//! Passive grammar and selection rule for a directive's file target.
//!
//! `::file` and `::code` name one file. `::toc-linking` names a fallback
//! chain: alternatives separated by `|`, optionally ending in `false`
//! (case-insensitive), as in `::toc-linking "./a.md | ./b.md | false"`. The
//! first alternative that resolves to an existing file wins; when none does,
//! a trailing `false` suppresses the directive's output, and otherwise the
//! directive fails with the first alternative's failure class.
//!
//! Composition and the language server both read targets through this module,
//! so an editor projection never resolves a whole chain as one filename. The
//! split and the selection are passive: [`select_target`] resolves only
//! through the caller's closure.

use biscuit_file::ResolutionFailure;

use super::directives_api::{DirectiveKind, ParsedDirective};
use crate::markdown::span::SourceSpan;

/// One alternative of a target chain: its text and its byte span in the
/// scanned document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAlternative {
    /// The alternative as authored, trimmed.
    pub value: String,
    /// Byte span of `value` in the document. When the authored token cannot
    /// be mapped back exactly, this is the whole target token.
    pub span: SourceSpan,
}

/// A directive target read through its family's grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetChain {
    /// The file alternatives in authored order; never contains `false`.
    pub alternatives: Vec<TargetAlternative>,
    /// The chain ends in `false`: when no alternative exists the directive
    /// intentionally produces nothing.
    pub suppress_not_found: bool,
}

/// Which alternative a chain selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSelection<T> {
    /// The alternative at `index` resolved to `resolved`.
    Found { index: usize, resolved: T },
    /// No alternative resolved and the chain ends in `false`.
    Suppressed,
    /// No alternative resolved. `failure` is the first (authored)
    /// alternative's class, `None` when that failure was not a
    /// file-reference failure.
    Unresolved { failure: Option<ResolutionFailure> },
}

/// A chain that does not follow the grammar; composition rejects the
/// directive with this message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetChainError(pub String);

/// An alternative's text and its byte range within the raw chain.
type RawAlternative<'a> = (&'a str, std::ops::Range<usize>);

/// Splits a raw (unquoted) chain into its alternatives and whether it ends
/// in `false`.
pub(crate) fn split_target_chain(
    raw: &str,
) -> Result<(Vec<RawAlternative<'_>>, bool), TargetChainError> {
    let parts: Vec<RawAlternative<'_>> = {
        let mut parts = Vec::new();
        let mut start = 0;
        for piece in raw.split('|') {
            let leading = piece.len() - piece.trim_start().len();
            let trimmed = piece.trim();
            let begin = start + leading;
            parts.push((trimmed, begin..begin + trimmed.len()));
            start += piece.len() + 1;
        }
        parts
    };
    let last = parts.len() - 1;
    let mut alternatives = Vec::new();
    let mut suppress = false;
    for (index, (part, range)) in parts.into_iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if part.eq_ignore_ascii_case("false") {
            if index != last {
                return Err(TargetChainError(
                    "'false' can only appear as the last item in a fallback chain".to_string(),
                ));
            }
            suppress = true;
        } else {
            alternatives.push((part, range));
        }
    }
    if alternatives.is_empty() && !suppress {
        return Err(TargetChainError("No file targets specified".to_string()));
    }
    Ok((alternatives, suppress))
}

/// Applies the chain's selection rule: the first alternative `resolve`
/// accepts wins; `resolve` returns the failure class of a rejected one.
pub fn select_target<'a, T>(
    alternatives: impl IntoIterator<Item = &'a str>,
    suppress_not_found: bool,
    mut resolve: impl FnMut(&str) -> Result<T, Option<ResolutionFailure>>,
) -> TargetSelection<T> {
    // The authored (first) alternative's class is the one reported: the
    // reader acts on what they wrote, and a fallback's miss must not hide it.
    let mut first_failure = None;
    let mut any = false;
    for (index, alternative) in alternatives.into_iter().enumerate() {
        any = true;
        match resolve(alternative) {
            Ok(resolved) => return TargetSelection::Found { index, resolved },
            Err(failure) if index == 0 => first_failure = failure,
            Err(_) => {}
        }
    }
    if suppress_not_found || !any {
        TargetSelection::Suppressed
    } else {
        TargetSelection::Unresolved { failure: first_failure }
    }
}

impl TargetChain {
    /// [`select_target`] over this chain's alternatives.
    pub fn select<T>(
        &self,
        resolve: impl FnMut(&str) -> Result<T, Option<ResolutionFailure>>,
    ) -> TargetSelection<T> {
        select_target(
            self.alternatives.iter().map(|alternative| alternative.value.as_str()),
            self.suppress_not_found,
            resolve,
        )
    }
}

impl ParsedDirective {
    /// This directive's file target read through its family's grammar:
    /// `::toc-linking` splits its fallback chain; `::file` and `::code` are
    /// one alternative (a `|` is part of the filename). `None` for a family
    /// without a single-file target (or with no target); `Err` for a chain
    /// composition rejects.
    pub fn target_chain(&self, source: &str) -> Option<Result<TargetChain, TargetChainError>> {
        let target = self.target.as_ref()?;
        match self.kind {
            DirectiveKind::File | DirectiveKind::Code => Some(Ok(TargetChain {
                alternatives: vec![TargetAlternative {
                    value: target.value.clone(),
                    span: target.span.clone(),
                }],
                suppress_not_found: false,
            })),
            DirectiveKind::TocLinking => {
                // A quoted token's value starts one byte in; anything else
                // that does not map back exactly spans the whole token.
                let authored = source.get(target.span.clone()).unwrap_or_default();
                let value_offset = if authored == target.value {
                    Some(target.span.start)
                } else if authored.len() == target.value.len() + 2
                    && authored.get(1..authored.len() - 1) == Some(target.value.as_str())
                {
                    Some(target.span.start + 1)
                } else {
                    None
                };
                Some(split_target_chain(&target.value).map(|(alternatives, suppress)| TargetChain {
                    alternatives: alternatives
                        .into_iter()
                        .map(|(value, range)| TargetAlternative {
                            value: value.to_string(),
                            span: value_offset.map_or_else(
                                || target.span.clone(),
                                |offset| offset + range.start..offset + range.end,
                            ),
                        })
                        .collect(),
                    suppress_not_found: suppress,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::directives_api::scan_darkmatter_directives;

    fn chain(source: &str) -> Result<TargetChain, TargetChainError> {
        let directives = scan_darkmatter_directives(source);
        directives[0].target_chain(source).expect("a file target")
    }

    #[test]
    fn toc_linking_chain_keeps_each_alternative_span() {
        let source = "# Doc\n\n::toc-linking \"&missing.md | &target.md\" level=h2\n";
        let chain = chain(source).unwrap();
        let values: Vec<&str> = chain.alternatives.iter().map(|alt| alt.value.as_str()).collect();
        assert_eq!(values, ["&missing.md", "&target.md"]);
        for alternative in &chain.alternatives {
            assert_eq!(&source[alternative.span.clone()], alternative.value);
        }
        assert!(!chain.suppress_not_found);
    }

    #[test]
    fn trailing_false_suppresses_and_is_never_a_filename() {
        let source = "::toc-linking \"&missing.md | FALSE\"\n";
        let chain = chain(source).unwrap();
        assert_eq!(chain.alternatives.len(), 1);
        assert_eq!(chain.alternatives[0].value, "&missing.md");
        assert!(chain.suppress_not_found);
    }

    #[test]
    fn false_before_the_end_is_rejected() {
        assert!(chain("::toc-linking \"a.md | false | b.md\"\n").is_err());
    }

    #[test]
    fn file_and_code_targets_are_one_alternative() {
        for keyword in ["::file", "::code"] {
            let source = format!("{keyword} \"a.md | b.md\"\n");
            let chain = chain(&source).unwrap();
            assert_eq!(chain.alternatives.len(), 1, "{keyword}");
            assert_eq!(chain.alternatives[0].value, "a.md | b.md");
        }
    }

    #[test]
    fn bare_chain_maps_spans_without_quotes() {
        let source = "::toc-linking a.md|b.md\n";
        let chain = chain(source).unwrap();
        assert_eq!(&source[chain.alternatives[1].span.clone()], "b.md");
    }

    #[test]
    fn selection_takes_the_first_existing_and_reports_the_first_failure() {
        let exists = |name: &str| -> Result<String, Option<ResolutionFailure>> {
            if name == "b" { Ok(name.to_string()) } else { Err(Some(ResolutionFailure::NoMatch)) }
        };
        assert_eq!(
            select_target(["a", "b"], false, exists),
            TargetSelection::Found { index: 1, resolved: "b".to_string() }
        );
        assert_eq!(
            select_target(["b", "a"], false, exists),
            TargetSelection::Found { index: 0, resolved: "b".to_string() }
        );
        assert_eq!(select_target(["a"], true, exists), TargetSelection::Suppressed);
        let first_invalid = |name: &str| -> Result<(), Option<ResolutionFailure>> {
            Err(Some(if name == "a" { ResolutionFailure::InvalidReference } else { ResolutionFailure::NoMatch }))
        };
        assert_eq!(
            select_target(["a", "c"], false, first_invalid),
            TargetSelection::Unresolved { failure: Some(ResolutionFailure::InvalidReference) }
        );
    }
}

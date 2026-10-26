//! Grammar loading utilities for syntax highlighting.
//!
//! Provides functions to load syntax sets from two-face and syntect,
//! including support for extended language grammars.

use lazy_static::lazy_static;
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use two_face::syntax::extra_newlines as extra_syntax_set;

use crate::markdown::language_grammar::LanguageGrammar;

/// The vendored, flattened Mermaid syntax (MIT). `assets/syntaxes/mermaid/SOURCE.md`
/// records its upstream commit and how it differs from upstream.
pub(crate) const MERMAID_SYNTAX: &str =
    include_str!("../../../assets/syntaxes/mermaid/Mermaid.sublime-syntax");

lazy_static! {
    /// Lazily loaded syntax set from two-face with extended grammars.
    ///
    /// This includes all default syntect grammars plus additional languages
    /// from the bat project (TypeScript, TOML, Dockerfile, etc.).
    static ref SYNTAX_SET: SyntaxSet = extra_syntax_set();

    /// The vendored Mermaid syntax and plain text, in a set of their own.
    ///
    /// Adding Mermaid to two-face's set would relink every two-face syntax,
    /// which forces all of their lazily serialized contexts to load: about
    /// 0.5 s per process in a debug build against 25 ms for the set alone.
    /// The Mermaid syntax references no other syntax, so a separate set
    /// highlights it identically for about a millisecond.
    static ref MERMAID_SYNTAX_SET: SyntaxSet = build_mermaid_syntax_set();
}

fn build_mermaid_syntax_set() -> SyntaxSet {
    let mermaid = SyntaxDefinition::load_from_str(MERMAID_SYNTAX, true, Some("Mermaid"))
        .expect("the vendored Mermaid syntax parses; a test guards it");
    let mut builder = SyntaxSetBuilder::new();
    builder.add_plain_text_syntax();
    builder.add(mermaid);
    builder.build()
}

/// Returns the syntax set that carries `grammar`'s syntax.
///
/// A [`syntect::parsing::SyntaxReference`] only highlights against the set
/// it was resolved from, so callers resolve and highlight against this one
/// set.
pub(crate) fn syntax_set_for(grammar: &LanguageGrammar) -> &'static SyntaxSet {
    if matches!(grammar, LanguageGrammar::Mermaid) {
        &MERMAID_SYNTAX_SET
    } else {
        &SYNTAX_SET
    }
}

/// Loads the syntax set with extended grammars.
///
/// This returns a reference to a lazily-loaded static syntax set that includes
/// both syntect's default grammars and additional grammars from two-face (curated
/// by the bat project).
///
/// ## Examples
///
/// ```
/// # use darkmatter::markdown::highlighting::CodeHighlighter;
/// # let highlighter = CodeHighlighter::default();
/// let syntax_set = highlighter.syntax_set();
/// let rust_syntax = syntax_set.find_syntax_by_extension("rs");
/// assert!(rust_syntax.is_some());
/// ```
pub(crate) fn load_syntax_set() -> &'static SyntaxSet {
    &SYNTAX_SET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_syntax_set() {
        let syntax_set = load_syntax_set();
        assert!(!syntax_set.syntaxes().is_empty());
    }

    #[test]
    fn test_find_rust_syntax() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set.find_syntax_by_extension("rs");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap().name, "Rust");
    }

    #[test]
    fn test_find_typescript_syntax() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set.find_syntax_by_extension("ts");
        assert!(syntax.is_some());
    }

    #[test]
    fn test_find_python_syntax() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set.find_syntax_by_extension("py");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap().name, "Python");
    }

    #[test]
    fn test_find_toml_syntax() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set.find_syntax_by_extension("toml");
        assert!(syntax.is_some());
    }

    #[test]
    fn test_find_yaml_syntax() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set.find_syntax_by_extension("yaml");
        assert!(syntax.is_some());
    }

    #[test]
    fn test_syntax_set_pointer_stability() {
        let a = load_syntax_set();
        let b = load_syntax_set();
        assert!(
            std::ptr::eq(a, b),
            "load_syntax_set should return the same static instance"
        );
    }
}

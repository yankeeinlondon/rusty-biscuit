//! Grammar loading utilities for syntax highlighting.
//!
//! Provides functions to load syntax sets from two-face and syntect,
//! including support for extended language grammars.

use lazy_static::lazy_static;
use syntect::parsing::SyntaxSet;
use two_face::syntax::{extra_newlines as extra_syntax_set};

lazy_static! {
    /// Lazily loaded syntax set from two-face with extended grammars.
    ///
    /// This includes all default syntect grammars plus additional languages
    /// from the bat project (TypeScript, TOML, Dockerfile, etc.).
    static ref SYNTAX_SET: SyntaxSet = extra_syntax_set();
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
/// # use shared::markdown::highlighting::CodeHighlighter;
/// # let highlighter = CodeHighlighter::default();
/// let syntax_set = highlighter.syntax_set();
/// let rust_syntax = syntax_set.find_syntax_by_extension("rs");
/// assert!(rust_syntax.is_some());
/// ```
pub(super) fn load_syntax_set() -> SyntaxSet {
    SYNTAX_SET.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_syntax_set() {
        let syntax_set = load_syntax_set();
        assert!(syntax_set.syntaxes().len() > 0);
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
}

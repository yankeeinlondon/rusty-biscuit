//! Typed language grammar resolver for fenced code blocks.
//!
//! [`LanguageGrammar`] is a typed convenience over
//! [`syntect::parsing::SyntaxSet`]. Common languages have named variants that
//! resolve through tested lookup paths; arbitrary fence tokens fall through to
//! dynamic extension/name/token lookup. Resolution is failible: the syntax set
//! may not carry every grammar a caller names, and the caller decides whether
//! to error or fall back to a `plain-text` syntax.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::language_grammar::LanguageGrammar;
//!
//! let yaml = LanguageGrammar::Yaml;
//! assert_eq!(format!("{:?}", yaml), "Yaml");
//! ```

use std::fmt;
use thiserror::Error;

/// Errors that can occur when resolving a [`LanguageGrammar`] to a
/// [`syntect::parsing::SyntaxReference`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LanguageGrammarError {
    /// The grammar could not be found in the loaded [`SyntaxSet`].
    #[error("Unknown grammar: {0:?} (not in syntax set; check spelling or load a custom .sublime-syntax)")]
    UnknownGrammar(String),
}

/// Typed convenience resolver for fenced code-block languages.
///
/// Common languages have named variants whose resolution is covered by
/// unit tests; arbitrary fence tokens (`"rs"`, `"py"`, `"yaml"`, custom
/// project grammars) flow through [`LanguageGrammar::OtherByExtension`],
/// [`LanguageGrammar::OtherByName`], or [`LanguageGrammar::OtherByToken`]
/// and resolve dynamically against a [`SyntaxSet`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::language_grammar::LanguageGrammar;
///
/// // The "yml" alias is mapped to YAML by `from_fence_token` (per spec).
/// let resolved = LanguageGrammar::from_fence_token("yml");
/// assert!(matches!(resolved, LanguageGrammar::Yaml));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LanguageGrammar {
    /// Rust source code.
    Rust,
    /// JavaScript source code.
    JavaScript,
    /// TypeScript source code.
    TypeScript,
    /// Go source code.
    Go,
    /// PHP source code.
    Php,
    /// Python source code.
    Python,
    /// Bash / shell scripts.
    Bash,
    /// HTML markup.
    Html,
    /// CSS stylesheets.
    Css,
    /// Markdown source.
    Markdown,
    /// YAML configuration / data.
    Yaml,
    /// JSON data.
    Json,
    /// TOML configuration.
    Toml,
    /// Look up by file extension. Resolves against
    /// [`SyntaxSet::find_syntax_by_extension`].
    OtherByExtension(String),
    /// Look up by display name. Resolves against
    /// [`SyntaxSet::find_syntax_by_name`].
    OtherByName(String),
    /// Look up by raw fence token. Resolves by trying the token as
    /// extension, then as name (case-insensitive), then via the alias map.
    OtherByToken(String),
}

impl LanguageGrammar {
    /// Resolves a fenced code-block language token to a typed
    /// [`LanguageGrammar`].
    ///
    /// Common variants use canonical, tested lookup paths. Tokens
    /// syntect resolves natively (`"rs"`, `"py"`, `"js"`, `"ts"`, `"yaml"`,
    /// `"md"`, `"json"`, `"toml"`, `"go"`, `"css"`, `"html"`, `"php"`) flow
    /// through [`LanguageGrammar::OtherByExtension`].
    ///
    /// The first implementation preserves the seven aliases the existing
    /// resolver already special-cases
    /// (`code_block.rs:357`):
    /// `shell`/`zsh` → `bash`, `c++` → `cpp`, `dockerfile` → `Dockerfile`,
    /// `makefile`/`make` → `Makefile`, `javascript` → `js`, `typescript` →
    /// `ts`, `python3` → `py`, and adds four gap-fills: `sh` → `bash`,
    /// `tsx` → TypeScript, `python` → `py`, `yml` → `yaml`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::language_grammar::LanguageGrammar;
    ///
    /// assert!(matches!(LanguageGrammar::from_fence_token("rust"), LanguageGrammar::Rust));
    /// assert!(matches!(LanguageGrammar::from_fence_token("py"), LanguageGrammar::Python));
    /// assert!(matches!(LanguageGrammar::from_fence_token("yml"), LanguageGrammar::Yaml));
    /// ```
    pub fn from_fence_token(token: impl AsRef<str>) -> Self {
        let token = token.as_ref().trim();
        if token.is_empty() {
            return LanguageGrammar::OtherByToken(String::new());
        }
        let lower = token.to_ascii_lowercase();
        // The 11 guaranteed aliases — the seven the existing resolver
        // already special-cases plus the four gap-fills the spec requires.
        match lower.as_str() {
            "rust" | "rs" => LanguageGrammar::Rust,
            "javascript" | "js" => LanguageGrammar::JavaScript,
            "typescript" | "ts" => LanguageGrammar::TypeScript,
            "tsx" => LanguageGrammar::TypeScript,
            "go" | "golang" => LanguageGrammar::Go,
            "php" => LanguageGrammar::Php,
            "python" | "py" | "python3" => LanguageGrammar::Python,
            "bash" | "sh" | "shell" | "zsh" => LanguageGrammar::Bash,
            "html" | "htm" => LanguageGrammar::Html,
            "css" => LanguageGrammar::Css,
            "markdown" | "md" => LanguageGrammar::Markdown,
            "yaml" | "yml" => LanguageGrammar::Yaml,
            "json" => LanguageGrammar::Json,
            "toml" => LanguageGrammar::Toml,
            "c++" | "cpp" => LanguageGrammar::OtherByExtension("cpp".to_string()),
            "dockerfile" => LanguageGrammar::OtherByName("Dockerfile".to_string()),
            "makefile" | "make" => LanguageGrammar::OtherByName("Makefile".to_string()),
            _ => {
                // Common tokens that resolve natively in syntect fall
                // through to OtherByExtension; syntect will find them by
                // extension.
                LanguageGrammar::OtherByToken(token.to_string())
            }
        }
    }

    /// Resolves this grammar against a [`SyntaxSet`], returning the
    /// matching [`SyntaxReference`].
    ///
    /// Resolution order:
    ///
    /// 1. **Named variants** ([`Rust`], [`Yaml`], etc.) prefer the canonical
    ///    extension or display name first, then fall back to the same alias
    ///    map [`from_fence_token`] uses.
    /// 2. **OtherByExtension** and **OtherByName** call the corresponding
    ///    `find_syntax_by_*` directly.
    /// 3. **OtherByToken** tries extension, then name, then the alias map.
    ///
    /// [`Rust`]: LanguageGrammar::Rust
    /// [`Yaml`]: LanguageGrammar::Yaml
    ///
    /// ## Errors
    ///
    /// Returns [`LanguageGrammarError::UnknownGrammar`] if the syntax set
    /// does not carry a grammar matching this resolver.
    pub fn resolve<'a>(
        &self,
        syntax_set: &'a syntect::parsing::SyntaxSet,
    ) -> Result<&'a syntect::parsing::SyntaxReference, LanguageGrammarError> {
        match self {
            LanguageGrammar::Rust => find_first(
                syntax_set,
                &[("extension", "rs"), ("name", "Rust")],
            ),
            LanguageGrammar::JavaScript => find_first(
                syntax_set,
                &[("extension", "js"), ("name", "JavaScript")],
            ),
            LanguageGrammar::TypeScript => find_first(
                syntax_set,
                &[("extension", "ts"), ("name", "TypeScript")],
            ),
            LanguageGrammar::Go => find_first(
                syntax_set,
                &[("extension", "go"), ("name", "Go")],
            ),
            LanguageGrammar::Php => find_first(
                syntax_set,
                &[("extension", "php"), ("name", "PHP")],
            ),
            LanguageGrammar::Python => find_first(
                syntax_set,
                &[("extension", "py"), ("name", "Python")],
            ),
            LanguageGrammar::Bash => find_first(
                syntax_set,
                &[("extension", "sh"), ("name", "Bourne Again Shell (bash)")],
            ),
            LanguageGrammar::Html => find_first(
                syntax_set,
                &[("extension", "html"), ("name", "HTML")],
            ),
            LanguageGrammar::Css => find_first(
                syntax_set,
                &[("extension", "css"), ("name", "CSS")],
            ),
            LanguageGrammar::Markdown => find_first(
                syntax_set,
                &[("extension", "md"), ("name", "Markdown")],
            ),
            LanguageGrammar::Yaml => find_first(
                syntax_set,
                &[("extension", "yaml"), ("extension", "yml"), ("name", "YAML")],
            ),
            LanguageGrammar::Json => find_first(
                syntax_set,
                &[("extension", "json"), ("name", "JSON")],
            ),
            LanguageGrammar::Toml => find_first(
                syntax_set,
                &[("extension", "toml"), ("name", "TOML")],
            ),
            LanguageGrammar::OtherByExtension(ext) => syntax_set
                .find_syntax_by_extension(ext)
                .ok_or_else(|| LanguageGrammarError::UnknownGrammar(format!("extension:{ext}"))),
            LanguageGrammar::OtherByName(name) => {
                let direct = syntax_set
                    .find_syntax_by_name(name)
                    .ok_or_else(|| LanguageGrammarError::UnknownGrammar(format!("name:{name}")))?;
                Ok(direct)
            }
            LanguageGrammar::OtherByToken(token) => find_via_token(syntax_set, token),
        }
    }
}

impl fmt::Display for LanguageGrammar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageGrammar::Rust => f.write_str("rust"),
            LanguageGrammar::JavaScript => f.write_str("javascript"),
            LanguageGrammar::TypeScript => f.write_str("typescript"),
            LanguageGrammar::Go => f.write_str("go"),
            LanguageGrammar::Php => f.write_str("php"),
            LanguageGrammar::Python => f.write_str("python"),
            LanguageGrammar::Bash => f.write_str("bash"),
            LanguageGrammar::Html => f.write_str("html"),
            LanguageGrammar::Css => f.write_str("css"),
            LanguageGrammar::Markdown => f.write_str("markdown"),
            LanguageGrammar::Yaml => f.write_str("yaml"),
            LanguageGrammar::Json => f.write_str("json"),
            LanguageGrammar::Toml => f.write_str("toml"),
            LanguageGrammar::OtherByExtension(ext)
            | LanguageGrammar::OtherByName(ext)
            | LanguageGrammar::OtherByToken(ext) => f.write_str(ext),
        }
    }
}

/// Looks up a syntax by trying multiple `("kind", "value")` pairs in order.
///
/// `kind` is `"extension"` or `"name"`. The first match wins.
fn find_first<'a>(
    syntax_set: &'a syntect::parsing::SyntaxSet,
    candidates: &[(&str, &str)],
) -> Result<&'a syntect::parsing::SyntaxReference, LanguageGrammarError> {
    for (kind, value) in candidates {
        let result = match *kind {
            "extension" => syntax_set.find_syntax_by_extension(value),
            "name" => syntax_set.find_syntax_by_name(value),
            other => {
                return Err(LanguageGrammarError::UnknownGrammar(format!(
                    "internal: unknown lookup kind {other}"
                )));
            }
        };
        if let Some(syntax) = result {
            return Ok(syntax);
        }
    }
    Err(LanguageGrammarError::UnknownGrammar(
        candidates
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(", "),
    ))
}

/// Resolves a raw fence token against the syntax set: extension first,
/// then case-insensitive name match, then the alias map. Mirrors the
/// existing terminal code-block lookup at `code_block.rs:330` so dynamic
/// fence tokens behave identically.
fn find_via_token<'a>(
    syntax_set: &'a syntect::parsing::SyntaxSet,
    token: &str,
) -> Result<&'a syntect::parsing::SyntaxReference, LanguageGrammarError> {
    if token.is_empty() {
        return Err(LanguageGrammarError::UnknownGrammar(String::new()));
    }
    if let Some(syntax) = syntax_set.find_syntax_by_extension(token) {
        return Ok(syntax);
    }
    if let Some(syntax) = syntax_set.find_syntax_by_name(token) {
        return Ok(syntax);
    }
    let lower = token.to_ascii_lowercase();
    for syntax in syntax_set.syntaxes() {
        if syntax.name.to_ascii_lowercase() == lower {
            return Ok(syntax);
        }
    }
    let alias = match lower.as_str() {
        "shell" | "zsh" | "sh" => "bash",
        "c++" => "cpp",
        "dockerfile" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        "javascript" => "js",
        "typescript" => "ts",
        "python" | "python3" => "py",
        "tsx" => "TypeScript",
        "yml" => "yaml",
        _ => return Err(LanguageGrammarError::UnknownGrammar(token.to_string())),
    };
    syntax_set
        .find_syntax_by_extension(alias)
        .or_else(|| syntax_set.find_syntax_by_name(alias))
        .ok_or_else(|| LanguageGrammarError::UnknownGrammar(token.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::highlighting::grammars::load_syntax_set;

    fn syntax_set() -> &'static syntect::parsing::SyntaxSet {
        load_syntax_set()
    }

    #[test]
    fn from_fence_token_returns_named_variants() {
        assert!(matches!(
            LanguageGrammar::from_fence_token("rust"),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("yaml"),
            LanguageGrammar::Yaml
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("json"),
            LanguageGrammar::Json
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("toml"),
            LanguageGrammar::Toml
        ));
    }

    #[test]
    fn from_fence_token_handles_native_syntect_tokens() {
        let rs = LanguageGrammar::from_fence_token("rs");
        let py = LanguageGrammar::from_fence_token("py");
        let js = LanguageGrammar::from_fence_token("js");
        let ts = LanguageGrammar::from_fence_token("ts");
        // Native resolution paths may be either a named variant or
        // OtherByExtension; either way they must resolve cleanly.
        assert!(matches!(rs.resolve(syntax_set()), Ok(_)));
        assert!(matches!(py.resolve(syntax_set()), Ok(_)));
        assert!(matches!(js.resolve(syntax_set()), Ok(_)));
        assert!(matches!(ts.resolve(syntax_set()), Ok(_)));
    }

    #[test]
    fn from_fence_token_handles_existing_aliases() {
        // The seven aliases the existing resolver already special-cases.
        assert!(matches!(
            LanguageGrammar::from_fence_token("shell"),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("zsh"),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("c++"),
            LanguageGrammar::OtherByExtension(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("dockerfile"),
            LanguageGrammar::OtherByName(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("makefile"),
            LanguageGrammar::OtherByName(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("javascript"),
            LanguageGrammar::JavaScript
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("typescript"),
            LanguageGrammar::TypeScript
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("python3"),
            LanguageGrammar::Python
        ));
    }

    #[test]
    fn from_fence_token_handles_new_aliases() {
        // The four gap-fill aliases the spec requires.
        assert!(matches!(
            LanguageGrammar::from_fence_token("sh"),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("tsx"),
            LanguageGrammar::TypeScript
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("python"),
            LanguageGrammar::Python
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("yml"),
            LanguageGrammar::Yaml
        ));
    }

    #[test]
    fn from_fence_token_is_case_insensitive() {
        assert!(matches!(
            LanguageGrammar::from_fence_token("RUST"),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_fence_token("Rust"),
            LanguageGrammar::Rust
        ));
    }

    #[test]
    fn from_fence_token_trims_whitespace() {
        assert!(matches!(
            LanguageGrammar::from_fence_token("  rust  "),
            LanguageGrammar::Rust
        ));
    }

    #[test]
    fn from_fence_token_empty_returns_other_by_token() {
        let resolved = LanguageGrammar::from_fence_token("");
        assert!(matches!(resolved, LanguageGrammar::OtherByToken(ref s) if s.is_empty()));
    }

    #[test]
    fn from_fence_token_unknown_returns_other_by_token() {
        let resolved = LanguageGrammar::from_fence_token("nosuchgrammar_xyz");
        assert!(matches!(resolved, LanguageGrammar::OtherByToken(_)));
    }

    #[test]
    fn resolve_named_variants_succeed() {
        for grammar in [
            LanguageGrammar::Rust,
            LanguageGrammar::JavaScript,
            LanguageGrammar::TypeScript,
            LanguageGrammar::Go,
            LanguageGrammar::Php,
            LanguageGrammar::Python,
            LanguageGrammar::Bash,
            LanguageGrammar::Html,
            LanguageGrammar::Css,
            LanguageGrammar::Markdown,
            LanguageGrammar::Yaml,
            LanguageGrammar::Json,
            LanguageGrammar::Toml,
        ] {
            assert!(
                grammar.resolve(syntax_set()).is_ok(),
                "expected {grammar:?} to resolve, got {:?}",
                grammar.resolve(syntax_set()).err()
            );
        }
    }

    #[test]
    fn resolve_other_by_extension_succeeds_for_known_extension() {
        let rust = LanguageGrammar::OtherByExtension("rs".to_string());
        let resolved = rust.resolve(syntax_set()).expect("rs resolves");
        assert_eq!(resolved.name, "Rust");
    }

    #[test]
    fn resolve_other_by_name_succeeds_for_known_name() {
        let dockerfile = LanguageGrammar::OtherByName("Dockerfile".to_string());
        let resolved = dockerfile.resolve(syntax_set()).expect("Dockerfile resolves");
        assert_eq!(resolved.name, "Dockerfile");
    }

    #[test]
    fn resolve_other_by_token_falls_back_through_paths() {
        let via_token = LanguageGrammar::OtherByToken("rust".to_string());
        let resolved = via_token.resolve(syntax_set()).expect("rust resolves via token");
        assert_eq!(resolved.name, "Rust");

        // sh resolves via the alias map.
        let sh_token = LanguageGrammar::OtherByToken("sh".to_string());
        let sh_resolved = sh_token.resolve(syntax_set()).expect("sh resolves via alias");
        assert!(!sh_resolved.name.is_empty());

        // yml resolves via the alias map to YAML.
        let yml_token = LanguageGrammar::OtherByToken("yml".to_string());
        let yml_resolved = yml_token
            .resolve(syntax_set())
            .expect("yml resolves via alias");
        assert!(!yml_resolved.name.is_empty());
    }

    #[test]
    fn resolve_unknown_grammar_returns_error() {
        let unknown = LanguageGrammar::OtherByExtension("nosuchgrammar_xyz".to_string());
        let err = unknown.resolve(syntax_set()).expect_err("unknown must error");
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));

        let unknown_token = LanguageGrammar::OtherByToken("nosuchgrammar_xyz".to_string());
        let err = unknown_token
            .resolve(syntax_set())
            .expect_err("unknown token must error");
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn display_round_trips_named_variants() {
        for (grammar, expected) in [
            (LanguageGrammar::Rust, "rust"),
            (LanguageGrammar::JavaScript, "javascript"),
            (LanguageGrammar::TypeScript, "typescript"),
            (LanguageGrammar::Yaml, "yaml"),
            (LanguageGrammar::Json, "json"),
            (LanguageGrammar::Toml, "toml"),
        ] {
            assert_eq!(format!("{grammar}"), expected);
        }
    }
}

//! Typed language grammar resolver for fenced code blocks.
//!
//! [`LanguageGrammar`] is Darkmatter's authoritative grammar lookup API. All
//! production code that turns a user-provided language, filename, extension, or
//! syntect name into a syntax grammar routes through this type instead of calling
//! [`syntect::parsing::SyntaxSet::find_syntax_by_extension`] or
//! [`find_syntax_by_name`](syntect::parsing::SyntaxSet::find_syntax_by_name)
//! directly.
//!
//! Common languages have named variants that resolve through tested lookup paths;
//! arbitrary fence tokens, extensions, and names fall through to dynamic
//! variants. Resolution is fallible for callers that want to surface unknown
//! grammars, and infallible helpers fall back to [`LanguageGrammar::PlainText`].
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::language_grammar::LanguageGrammar;
//!
//! let yaml = LanguageGrammar::yaml();
//! assert_eq!(format!("{}", yaml), "yaml");
//!
//! let rust = LanguageGrammar::try_from("rust title=\"hi\"").unwrap();
//! assert!(matches!(rust, LanguageGrammar::Rust));
//! ```

use std::fmt;
use std::str::FromStr;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use thiserror::Error;

use crate::markdown::highlighting::grammars::load_syntax_set;

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
/// `LanguageGrammar` is the single production grammar authority in Darkmatter.
/// Callers that need to resolve a fence token, extension, name, or filename to a
/// syntect grammar should use this type rather than invoking
/// `SyntaxSet::find_syntax_by_*` directly.
///
/// ## Construction
///
/// - Use [`from_token`](Self::from_token) for Markdown fence info strings.
/// - Use [`from_extension`](Self::from_extension), [`from_name`](Self::from_name),
///   or [`from_filename`](Self::from_filename) when the caller knows exactly what
///   kind of input it holds.
/// - Use [`TryFrom<&str>`] or [`FromStr`] for general user input that may be a
///   token, name, extension, or filename.
/// - Use the infallible [`from_lossy`](Self::from_lossy) or
///   [`from_token_or_plain_text`](Self::from_token_or_plain_text) when an unknown
///   grammar should silently fall back to plain text.
/// - Use the named constructors [`yaml`](Self::yaml), [`rust`](Self::rust),
///   [`json`](Self::json), [`toml`](Self::toml), [`markdown`](Self::markdown),
///   [`plain_text`](Self::plain_text), or [`text`](Self::text) for guaranteed
///   grammars.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::language_grammar::LanguageGrammar;
///
/// // Fence info metadata is ignored.
/// let grammar = LanguageGrammar::from_token("rust title=\"hi\"").unwrap();
/// assert!(matches!(grammar, LanguageGrammar::Rust));
///
/// // Filenames resolve by extension.
/// let grammar = LanguageGrammar::from_filename("src/main.rs").unwrap();
/// assert!(matches!(grammar, LanguageGrammar::Rust));
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
    /// JSON5 data (JSON with comments and relaxed syntax).
    ///
    /// Highlights through the JSON grammar (syntect ships no JSON5 grammar) but
    /// is a recognized language so tooling never flags it as unknown.
    Json5,
    /// TOML configuration.
    Toml,
    /// Mermaid diagram source.
    ///
    /// Darkmatter renders Mermaid as a diagram rather than highlighting it, and
    /// syntect ships no Mermaid grammar, so this resolves to plain text. It is a
    /// recognized language so tooling never flags it as unknown.
    Mermaid,
    /// Plain text / no grammar.
    ///
    /// This variant is the infallible fallback. Its [`Display`] representation
    /// is the empty string, so HTML and Markdown emission treat it as an
    /// unclassified code block.
    PlainText,
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
    /// Returns the plain-text grammar used as the infallible fallback.
    pub fn plain_text() -> Self {
        LanguageGrammar::PlainText
    }

    /// Alias for [`plain_text`](Self::plain_text).
    pub fn text() -> Self {
        LanguageGrammar::PlainText
    }

    /// Returns the YAML grammar.
    ///
    /// This constructor is infallible because YAML is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn yaml() -> Self {
        LanguageGrammar::Yaml
    }

    /// Returns the Rust grammar.
    ///
    /// This constructor is infallible because Rust is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn rust() -> Self {
        LanguageGrammar::Rust
    }

    /// Returns the Markdown grammar.
    ///
    /// This constructor is infallible because Markdown is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn markdown() -> Self {
        LanguageGrammar::Markdown
    }

    /// Returns the JSON grammar.
    ///
    /// This constructor is infallible because JSON is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn json() -> Self {
        LanguageGrammar::Json
    }

    /// Returns the TOML grammar.
    ///
    /// This constructor is infallible because TOML is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn toml() -> Self {
        LanguageGrammar::Toml
    }

    /// Parses a Markdown fence-style language token.
    ///
    /// Unquoted input is read only up to the first ASCII whitespace character so
    /// metadata such as `title="hi"` is ignored. Quoted input uses the quoted
    /// value as the token. Returns an error when the normalized token is empty
    /// or cannot be resolved by Darkmatter's default grammar set.
    pub fn from_token(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError> {
        let token = normalize_input_token(input.as_ref())?;
        resolve_token(&token)
    }

    /// Resolves a file extension to a Darkmatter grammar.
    ///
    /// The input may include or omit a leading dot. Lookup is case-insensitive
    /// for user ergonomics, and the returned grammar stores the canonical
    /// lowercase spelling so downstream `Display` emits a stable fence token.
    pub fn from_extension(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError> {
        let ext = input.as_ref().trim();
        if ext.is_empty() {
            return Err(LanguageGrammarError::UnknownGrammar(String::new()));
        }
        let ext = ext.strip_prefix('.').unwrap_or(ext);
        let lower = ext.to_ascii_lowercase();

        if let Some(grammar) = grammar_from_alias(&lower) {
            return Ok(grammar);
        }

        // syntect's `find_syntax_by_extension` happens to be case-insensitive
        // in practice, but Darkmatter's contract is to store the canonical
        // lowercase spelling regardless of input case.
        if load_syntax_set().find_syntax_by_extension(&lower).is_some() {
            return Ok(LanguageGrammar::OtherByExtension(lower));
        }

        Err(LanguageGrammarError::UnknownGrammar(lower))
    }

    /// Resolves a syntect display name to a Darkmatter grammar.
    ///
    /// This is for callers that know they have a grammar name such as `Rust`,
    /// `YAML`, `Dockerfile`, or `Bourne Again Shell (bash)`. Prefer
    /// [`from_token`](Self::from_token) for Markdown fence input and
    /// [`from_extension`](Self::from_extension) for file suffixes.
    pub fn from_name(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError> {
        let name = input.as_ref().trim();
        if name.is_empty() {
            return Err(LanguageGrammarError::UnknownGrammar(String::new()));
        }

        if let Some(syntax) = load_syntax_set().find_syntax_by_name(name) {
            return Ok(LanguageGrammar::OtherByName(syntax.name.clone()));
        }

        let lower = name.to_ascii_lowercase();
        for syntax in load_syntax_set().syntaxes() {
            if syntax.name.to_ascii_lowercase() == lower {
                return Ok(LanguageGrammar::OtherByName(syntax.name.clone()));
            }
        }

        Err(LanguageGrammarError::UnknownGrammar(name.to_string()))
    }

    /// Resolves a filename or path to a Darkmatter grammar.
    ///
    /// Paths with extensions resolve by their final extension. Extensionless
    /// well-known filenames such as `Makefile` and `Dockerfile` resolve through
    /// the same alias table used by token lookup.
    pub fn from_filename(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError> {
        let raw = input.as_ref().trim();
        if raw.is_empty() {
            return Err(LanguageGrammarError::UnknownGrammar(String::new()));
        }

        // Reject input that looks like fence info rather than a single filename.
        if raw.contains(|c: char| c.is_ascii_whitespace()) {
            return Err(LanguageGrammarError::UnknownGrammar(raw.to_string()));
        }

        let path = std::path::Path::new(raw);
        let basename = path
            .file_name()
            .and_then(|os| os.to_str())
            .unwrap_or(raw);

        if let Some(ext) = path.extension().and_then(|os| os.to_str()) {
            let ext = ext.trim();
            if !ext.is_empty() {
                return Self::from_extension(ext);
            }
        }

        // Extensionless well-known filenames resolve through the shared alias
        // tables used by token resolution. Named aliases win first; remaining
        // well-known source filenames (Dockerfile, Makefile, ...) resolve
        // through the extension/name alias map.
        let lower = basename.to_ascii_lowercase();
        if let Some(grammar) = grammar_from_alias(&lower) {
            return Ok(grammar);
        }

        let syntax_set = load_syntax_set();
        if let Some(alias) = extensionless_alias(&lower)
            && (syntax_set.find_syntax_by_extension(alias).is_some()
                || syntax_set.find_syntax_by_name(alias).is_some())
        {
            return Ok(LanguageGrammar::OtherByToken(basename.to_string()));
        }

        Err(LanguageGrammarError::UnknownGrammar(raw.to_string()))
    }

    /// Parses user-provided grammar text and falls back to plain text when no
    /// Darkmatter grammar matches.
    ///
    /// This is the preferred infallible entry point for UI, CLI, and rendering
    /// paths that should never fail just because a user supplied an unknown
    /// language token. It performs full resolution including filename/path
    /// detection.
    pub fn from_lossy(input: impl AsRef<str>) -> Self {
        match Self::try_from(input.as_ref()) {
            Ok(grammar) => grammar,
            Err(_) => LanguageGrammar::PlainText,
        }
    }

    /// Parses a Markdown fence-style language token and falls back to plain
    /// text when no Darkmatter grammar matches.
    ///
    /// Unlike [`from_lossy`](Self::from_lossy), this name makes the fence-token
    /// behavior explicit: unquoted input is read only up to the first ASCII
    /// whitespace character, while quoted input uses the quoted token. Filename
    /// detection is **not** performed.
    pub fn from_token_or_plain_text(input: impl AsRef<str>) -> Self {
        match Self::from_token(input.as_ref()) {
            Ok(grammar) => grammar,
            Err(_) => LanguageGrammar::PlainText,
        }
    }

    /// Resolves this grammar against Darkmatter's default two-face syntax set.
    ///
    /// Known variants such as [`Yaml`](LanguageGrammar::Yaml) and
    /// [`Rust`](LanguageGrammar::Rust) are expected to resolve successfully
    /// because Darkmatter treats them as guaranteed grammars. [`PlainText`](LanguageGrammar::PlainText)
    /// always resolves. Dynamic variants can still fail if they were constructed
    /// manually with an unsupported extension, name, or token.
    pub fn resolve_default(
        &self,
    ) -> Result<&'static SyntaxReference, LanguageGrammarError> {
        self.resolve(load_syntax_set())
    }

    /// Resolves this grammar against a [`SyntaxSet`], returning the
    /// matching [`SyntaxReference`].
    ///
    /// Resolution order:
    ///
    /// 1. **Named variants** ([`Rust`](LanguageGrammar::Rust), [`Yaml`](LanguageGrammar::Yaml),
    ///    etc.) prefer the canonical extension or display name first, then fall
    ///    back to the alias map.
    /// 2. **OtherByExtension** and **OtherByName** call the corresponding
    ///    `find_syntax_by_*` directly.
    /// 3. **OtherByToken** tries extension, then name, then the alias map.
    /// 4. **PlainText** always returns the plain-text syntax reference.
    ///
    /// ## Errors
    ///
    /// Returns [`LanguageGrammarError::UnknownGrammar`] if the syntax set
    /// does not carry a grammar matching this resolver.
    pub fn resolve<'a>(
        &self,
        syntax_set: &'a SyntaxSet,
    ) -> Result<&'a SyntaxReference, LanguageGrammarError> {
        match self {
            LanguageGrammar::PlainText => Ok(syntax_set.find_syntax_plain_text()),
            LanguageGrammar::Rust => find_first(syntax_set, &[("extension", "rs"), ("name", "Rust")]),
            LanguageGrammar::JavaScript => {
                find_first(syntax_set, &[("extension", "js"), ("name", "JavaScript")])
            }
            LanguageGrammar::TypeScript => {
                find_first(syntax_set, &[("extension", "ts"), ("name", "TypeScript")])
            }
            LanguageGrammar::Go => find_first(syntax_set, &[("extension", "go"), ("name", "Go")]),
            LanguageGrammar::Php => find_first(syntax_set, &[("extension", "php"), ("name", "PHP")]),
            LanguageGrammar::Python => {
                find_first(syntax_set, &[("extension", "py"), ("name", "Python")])
            }
            LanguageGrammar::Bash => find_first(
                syntax_set,
                &[("extension", "sh"), ("name", "Bourne Again Shell (bash)")],
            ),
            LanguageGrammar::Html => {
                find_first(syntax_set, &[("extension", "html"), ("name", "HTML")])
            }
            LanguageGrammar::Css => find_first(syntax_set, &[("extension", "css"), ("name", "CSS")]),
            LanguageGrammar::Markdown => {
                find_first(syntax_set, &[("extension", "md"), ("name", "Markdown")])
            }
            LanguageGrammar::Yaml => find_first(
                syntax_set,
                &[("extension", "yaml"), ("extension", "yml"), ("name", "YAML")],
            ),
            LanguageGrammar::Json => {
                find_first(syntax_set, &[("extension", "json"), ("name", "JSON")])
            }
            LanguageGrammar::Json5 => {
                find_first(syntax_set, &[("extension", "json"), ("name", "JSON")])
            }
            LanguageGrammar::Toml => {
                find_first(syntax_set, &[("extension", "toml"), ("name", "TOML")])
            }
            LanguageGrammar::Mermaid => Ok(syntax_set.find_syntax_plain_text()),
            LanguageGrammar::OtherByExtension(ext) => syntax_set
                .find_syntax_by_extension(ext)
                .ok_or_else(|| LanguageGrammarError::UnknownGrammar(format!("extension:{ext}"))),
            LanguageGrammar::OtherByName(name) => syntax_set
                .find_syntax_by_name(name)
                .ok_or_else(|| LanguageGrammarError::UnknownGrammar(format!("name:{name}"))),
            LanguageGrammar::OtherByToken(token) => find_via_token(syntax_set, token),
        }
    }
}

impl TryFrom<&str> for LanguageGrammar {
    type Error = LanguageGrammarError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let token = normalize_input_token(input)?;

        // Input containing unquoted whitespace is fence info, not a filename.
        let is_fence_info = !input.trim().starts_with(['"', '\'']) && input.trim().contains(|c: char| c.is_ascii_whitespace());

        if !is_fence_info
            && looks_like_filename(input.trim())
            && let Ok(grammar) = Self::from_filename(input.trim())
        {
            return Ok(grammar);
        }

        resolve_token(&token)
    }
}

impl TryFrom<String> for LanguageGrammar {
    type Error = LanguageGrammarError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        LanguageGrammar::try_from(input.as_str())
    }
}

impl FromStr for LanguageGrammar {
    type Err = LanguageGrammarError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        LanguageGrammar::try_from(input)
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
            LanguageGrammar::Json5 => f.write_str("json5"),
            LanguageGrammar::Toml => f.write_str("toml"),
            LanguageGrammar::Mermaid => f.write_str("mermaid"),
            LanguageGrammar::PlainText => Ok(()),
            LanguageGrammar::OtherByExtension(ext)
            | LanguageGrammar::OtherByName(ext)
            | LanguageGrammar::OtherByToken(ext) => f.write_str(ext),
        }
    }
}

/// Trims input and extracts the first fence token.
///
/// - Quoted input (`"rust"` or `'rust'`) uses the content up to the matching
///   closing quote.
/// - Unquoted input is read up to the first ASCII whitespace character.
/// - Empty normalized input returns `UnknownGrammar(String::new())`.
fn normalize_input_token(input: &str) -> Result<String, LanguageGrammarError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LanguageGrammarError::UnknownGrammar(String::new()));
    }

    let first = trimmed.chars().next().expect("non-empty trimmed input");
    if first == '"' || first == '\'' {
        let mut token = String::new();
        for ch in trimmed[first.len_utf8()..].chars() {
            if ch == first {
                break;
            }
            token.push(ch);
        }
        return Ok(token);
    }

    Ok(trimmed
        .split(|c: char| c.is_ascii_whitespace())
        .next()
        .unwrap_or("")
        .to_string())
}

/// Returns `true` when the input looks like a filename or path rather than a
/// language token.
fn looks_like_filename(input: &str) -> bool {
    if input.contains(['/', '\\']) {
        return true;
    }

    let path = std::path::Path::new(input);
    let basename = path.file_name().and_then(|os| os.to_str()).unwrap_or(input);

    // Basename with a dot and a non-empty suffix.
    if let Some(dot) = basename.rfind('.')
        && dot > 0
        && dot + 1 < basename.len()
    {
        return true;
    }

    // Extensionless well-known filenames.
    matches!(basename.to_ascii_lowercase().as_str(), "makefile" | "dockerfile" | "gemfile" | "rakefile")
}

/// Returns the named [`LanguageGrammar`] for a canonical alias token, if any.
fn grammar_from_alias(token: &str) -> Option<LanguageGrammar> {
    match token {
        "rust" | "rs" => Some(LanguageGrammar::Rust),
        "javascript" | "js" => Some(LanguageGrammar::JavaScript),
        "typescript" | "ts" | "tsx" => Some(LanguageGrammar::TypeScript),
        "go" | "golang" => Some(LanguageGrammar::Go),
        "php" => Some(LanguageGrammar::Php),
        "python" | "py" | "python3" => Some(LanguageGrammar::Python),
        "bash" | "sh" | "shell" | "zsh" => Some(LanguageGrammar::Bash),
        "html" | "htm" => Some(LanguageGrammar::Html),
        "css" => Some(LanguageGrammar::Css),
        "markdown" | "md" => Some(LanguageGrammar::Markdown),
        "yaml" | "yml" => Some(LanguageGrammar::Yaml),
        "json" => Some(LanguageGrammar::Json),
        "json5" => Some(LanguageGrammar::Json5),
        "toml" => Some(LanguageGrammar::Toml),
        "mermaid" | "mmd" => Some(LanguageGrammar::Mermaid),
        _ => None,
    }
}

/// Returns the canonical syntect extension or display name that an
/// extensionless filename or token alias resolves to.
///
/// Shared between [`LanguageGrammar::from_filename`] and `find_via_token` so
/// extensionless well-known source filenames (`Dockerfile`, `Makefile`) and
/// their token aliases (`dockerfile`, `make`, `c++`) resolve identically.
fn extensionless_alias(token: &str) -> Option<&'static str> {
    match token {
        "shell" | "zsh" | "sh" => Some("bash"),
        "c++" | "cpp" => Some("cpp"),
        "dockerfile" => Some("Dockerfile"),
        "makefile" | "make" => Some("Makefile"),
        "javascript" => Some("js"),
        "typescript" => Some("ts"),
        "python" | "python3" => Some("py"),
        "tsx" => Some("TypeScript"),
        "yml" => Some("yaml"),
        _ => None,
    }
}

/// Returns `true` for tokens that explicitly request plain text and should
/// preserve the original token as a dynamic variant.
fn is_plain_text_token(token: &str) -> bool {
    matches!(token, "txt" | "text" | "plain" | "plaintext" | "plain-text")
}

/// Central token resolver.
///
/// Resolution order:
///
/// 1. Explicit aliases to named variants.
/// 2. Plain-text token preservation.
/// 3. Extension lookup.
/// 4. Exact name lookup.
/// 5. Case-insensitive name lookup.
/// 6. Extensionless alias map (`make` → Makefile, `dockerfile` → Dockerfile).
/// 7. Otherwise `UnknownGrammar`.
///
/// Steps 3-6 return [`OtherByToken`](LanguageGrammar::OtherByToken) preserving
/// the caller's spelling so downstream `Display` echoes the original token.
fn resolve_token(
    token: &str,
) -> Result<LanguageGrammar, LanguageGrammarError> {
    let lower = token.to_ascii_lowercase();

    if let Some(grammar) = grammar_from_alias(&lower) {
        return Ok(grammar);
    }

    if is_plain_text_token(&lower) {
        return Ok(LanguageGrammar::OtherByToken(token.to_string()));
    }

    let syntax_set = load_syntax_set();
    if syntax_set.find_syntax_by_extension(token).is_some()
        || syntax_set.find_syntax_by_name(token).is_some()
        || syntax_set
            .syntaxes()
            .iter()
            .any(|s| s.name.to_ascii_lowercase() == lower)
    {
        return Ok(LanguageGrammar::OtherByToken(token.to_string()));
    }

    // Well-known extensionless tokens (`make`, `dockerfile`, `c++`) are neither
    // syntect extensions nor names; consult the shared alias table before
    // giving up. Mirrors the path in `from_filename` / `find_via_token`.
    if let Some(alias) = extensionless_alias(&lower)
        && (syntax_set.find_syntax_by_extension(alias).is_some()
            || syntax_set.find_syntax_by_name(alias).is_some())
    {
        return Ok(LanguageGrammar::OtherByToken(token.to_string()));
    }

    Err(LanguageGrammarError::UnknownGrammar(token.to_string()))
}

/// Looks up a syntax by trying multiple `("kind", "value")` pairs in order.
///
/// `kind` is `"extension"` or `"name"`. The first match wins.
fn find_first<'a>(
    syntax_set: &'a SyntaxSet,
    candidates: &[(&str, &str)],
) -> Result<&'a SyntaxReference, LanguageGrammarError> {
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

/// Resolves a raw fence token against the syntax set: plain-text tokens
/// first, then extension, then exact name, then case-insensitive name,
/// then the alias map.
fn find_via_token<'a>(
    syntax_set: &'a SyntaxSet,
    token: &str,
) -> Result<&'a SyntaxReference, LanguageGrammarError> {
    if token.is_empty() {
        return Err(LanguageGrammarError::UnknownGrammar(String::new()));
    }

    if is_plain_text_token(&token.to_ascii_lowercase()) {
        return Ok(syntax_set.find_syntax_plain_text());
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
    let alias = extensionless_alias(&lower).ok_or_else(|| {
        LanguageGrammarError::UnknownGrammar(token.to_string())
    })?;
    syntax_set
        .find_syntax_by_extension(alias)
        .or_else(|| syntax_set.find_syntax_by_name(alias))
        .ok_or_else(|| LanguageGrammarError::UnknownGrammar(token.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syntax_set() -> &'static SyntaxSet {
        load_syntax_set()
    }

    #[test]
    fn named_constructors_return_expected_variants() {
        assert!(matches!(LanguageGrammar::yaml(), LanguageGrammar::Yaml));
        assert!(matches!(LanguageGrammar::rust(), LanguageGrammar::Rust));
        assert!(matches!(LanguageGrammar::json(), LanguageGrammar::Json));
        assert!(matches!(LanguageGrammar::toml(), LanguageGrammar::Toml));
        assert!(matches!(LanguageGrammar::markdown(), LanguageGrammar::Markdown));
        assert!(matches!(LanguageGrammar::plain_text(), LanguageGrammar::PlainText));
        assert!(matches!(LanguageGrammar::text(), LanguageGrammar::PlainText));
    }

    #[test]
    fn from_token_returns_named_variants() {
        assert!(matches!(
            LanguageGrammar::from_token("rust").unwrap(),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_token("yaml").unwrap(),
            LanguageGrammar::Yaml
        ));
        assert!(matches!(
            LanguageGrammar::from_token("json").unwrap(),
            LanguageGrammar::Json
        ));
        assert!(matches!(
            LanguageGrammar::from_token("toml").unwrap(),
            LanguageGrammar::Toml
        ));
    }

    #[test]
    fn from_token_handles_native_syntect_tokens() {
        let rs = LanguageGrammar::from_token("rs").unwrap();
        let py = LanguageGrammar::from_token("py").unwrap();
        let js = LanguageGrammar::from_token("js").unwrap();
        let ts = LanguageGrammar::from_token("ts").unwrap();
        assert!(rs.resolve(syntax_set()).is_ok());
        assert!(py.resolve(syntax_set()).is_ok());
        assert!(js.resolve(syntax_set()).is_ok());
        assert!(ts.resolve(syntax_set()).is_ok());
    }

    #[test]
    fn from_token_handles_aliases() {
        assert!(matches!(
            LanguageGrammar::from_token("shell").unwrap(),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_token("zsh").unwrap(),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_token("c++").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_token("dockerfile").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_token("makefile").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_token("make").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_token("javascript").unwrap(),
            LanguageGrammar::JavaScript
        ));
        assert!(matches!(
            LanguageGrammar::from_token("typescript").unwrap(),
            LanguageGrammar::TypeScript
        ));
        assert!(matches!(
            LanguageGrammar::from_token("python3").unwrap(),
            LanguageGrammar::Python
        ));
        assert!(matches!(
            LanguageGrammar::from_token("sh").unwrap(),
            LanguageGrammar::Bash
        ));
        assert!(matches!(
            LanguageGrammar::from_token("tsx").unwrap(),
            LanguageGrammar::TypeScript
        ));
        assert!(matches!(
            LanguageGrammar::from_token("python").unwrap(),
            LanguageGrammar::Python
        ));
        assert!(matches!(
            LanguageGrammar::from_token("yml").unwrap(),
            LanguageGrammar::Yaml
        ));
    }

    #[test]
    fn json5_and_mermaid_are_recognized_languages() {
        // Neither has a syntect grammar, but both are recognized so tooling
        // (e.g. DMLS's fence diagnostic) never flags them as unknown.
        for token in ["json5", "mermaid", "mmd", "JSON5", "Mermaid"] {
            let grammar = LanguageGrammar::from_token(token)
                .unwrap_or_else(|_| panic!("{token} should resolve"));
            assert!(grammar.resolve_default().is_ok(), "{token} should resolve to a syntax");
        }

        // JSON5 highlights through the JSON grammar; Mermaid falls back to plain
        // text (it is rendered as a diagram, not highlighted).
        let json5 = LanguageGrammar::from_token("json5").unwrap();
        assert!(matches!(json5, LanguageGrammar::Json5));
        assert_eq!(json5.resolve_default().unwrap().name, "JSON");
        assert_eq!(format!("{json5}"), "json5");

        let mermaid = LanguageGrammar::from_token("mermaid").unwrap();
        assert!(matches!(mermaid, LanguageGrammar::Mermaid));
        assert_eq!(mermaid.resolve_default().unwrap().name, "Plain Text");
        assert_eq!(format!("{mermaid}"), "mermaid");
    }

    #[test]
    fn from_token_is_case_insensitive() {
        assert!(matches!(
            LanguageGrammar::from_token("RUST").unwrap(),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_token("Rust").unwrap(),
            LanguageGrammar::Rust
        ));
    }

    #[test]
    fn from_token_trims_whitespace() {
        assert!(matches!(
            LanguageGrammar::from_token("  rust  ").unwrap(),
            LanguageGrammar::Rust
        ));
    }

    #[test]
    fn from_token_ignores_metadata_after_first_token() {
        let grammar = LanguageGrammar::from_token("rust title=\"hi\"").unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));

        let grammar = LanguageGrammar::from_token("rust title=\"a.b\"").unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn from_token_quoted_token_uses_quoted_content_for_resolution() {
        // The quoted content is extracted and used as the token. If it cannot be
        // resolved by the default syntax set, the constructor returns an error.
        let err = LanguageGrammar::from_token("\"my custom grammar\" title=\"hi\"").unwrap_err();
        assert!(
            matches!(err, LanguageGrammarError::UnknownGrammar(ref s) if s == "my custom grammar"),
            "expected error to preserve the quoted token, got {err:?}"
        );
    }

    #[test]
    fn from_token_empty_returns_unknown_grammar() {
        let err = LanguageGrammar::from_token("").unwrap_err();
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(ref s) if s.is_empty()));
    }

    #[test]
    fn from_token_unknown_returns_unknown_grammar() {
        let err = LanguageGrammar::from_token("nosuchgrammar_xyz").unwrap_err();
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn try_from_fence_info_ignores_metadata() {
        let grammar = LanguageGrammar::try_from("rust title=\"hi\"").unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn try_from_dotted_metadata_is_not_filename() {
        let grammar = LanguageGrammar::try_from("rust title=\"a.b\"").unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn try_from_filename_resolves_by_extension() {
        assert!(matches!(
            LanguageGrammar::try_from("src/main.rs").unwrap(),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::try_from("config.yml").unwrap(),
            LanguageGrammar::Yaml
        ));
    }

    #[test]
    fn try_from_extensionless_filename_resolves_by_alias() {
        assert!(matches!(
            LanguageGrammar::try_from("Dockerfile").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
        assert!(matches!(
            LanguageGrammar::try_from("Makefile").unwrap(),
            LanguageGrammar::OtherByToken(_)
        ));
    }

    #[test]
    fn try_from_unknown_returns_error() {
        let err = LanguageGrammar::try_from("unknown-language-xyz").unwrap_err();
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn try_from_empty_returns_unknown_grammar() {
        let err = LanguageGrammar::try_from("").unwrap_err();
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(ref s) if s.is_empty()));
    }

    #[test]
    fn from_string_matches_str() {
        let grammar = LanguageGrammar::try_from("rust".to_string()).unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn from_str_matches_try_from() {
        let grammar: LanguageGrammar = "rust".parse().unwrap();
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn from_lossy_unknown_returns_plain_text() {
        let grammar = LanguageGrammar::from_lossy("unknown-language-xyz");
        assert!(matches!(grammar, LanguageGrammar::PlainText));
    }

    #[test]
    fn from_lossy_filename_resolves() {
        let grammar = LanguageGrammar::from_lossy("src/main.rs");
        assert!(matches!(grammar, LanguageGrammar::Rust));
    }

    #[test]
    fn from_token_or_plain_text_filename_returns_plain_text() {
        // Filename detection is exclusive to the full/lossy path.
        let grammar = LanguageGrammar::from_token_or_plain_text("src/main.rs");
        assert!(matches!(grammar, LanguageGrammar::PlainText));
    }

    #[test]
    fn from_token_or_plain_text_unknown_returns_plain_text() {
        let grammar = LanguageGrammar::from_token_or_plain_text("unknown-language-xyz");
        assert!(matches!(grammar, LanguageGrammar::PlainText));
    }

    #[test]
    fn explicit_plain_text_tokens_are_token_preserving() {
        for token in ["txt", "text", "plain", "plaintext", "plain-text"] {
            let from_token = LanguageGrammar::from_token(token).unwrap();
            assert!(
                matches!(from_token, LanguageGrammar::OtherByToken(ref s) if s == token),
                "{token} should be token-preserving"
            );
            let from_try = LanguageGrammar::try_from(token).unwrap();
            assert!(
                matches!(from_try, LanguageGrammar::OtherByToken(ref s) if s == token),
                "{token} should be token-preserving via TryFrom"
            );
            assert!(from_token.resolve_default().is_ok());
        }
    }

    #[test]
    fn plain_text_display_is_empty() {
        assert_eq!(format!("{}", LanguageGrammar::PlainText), "");
    }

    #[test]
    fn plain_text_resolves_to_plain_text_syntax() {
        let syntax = LanguageGrammar::PlainText.resolve_default().unwrap();
        assert_eq!(syntax.name, "Plain Text");
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
            LanguageGrammar::PlainText,
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

        let sh_token = LanguageGrammar::OtherByToken("sh".to_string());
        let sh_resolved = sh_token.resolve(syntax_set()).expect("sh resolves via alias");
        assert!(!sh_resolved.name.is_empty());

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
            (LanguageGrammar::PlainText, ""),
        ] {
            assert_eq!(format!("{grammar}"), expected);
        }
    }

    #[test]
    fn from_extension_resolves_named_variants_and_other() {
        assert!(matches!(
            LanguageGrammar::from_extension("rs").unwrap(),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_extension(".rs").unwrap(),
            LanguageGrammar::Rust
        ));
        assert!(matches!(
            LanguageGrammar::from_extension("yaml").unwrap(),
            LanguageGrammar::Yaml
        ));
        let unknown = LanguageGrammar::from_extension("nosuchgrammar_xyz");
        assert!(matches!(unknown, Err(LanguageGrammarError::UnknownGrammar(_))));
    }

    #[test]
    fn from_name_resolves_by_name() {
        assert!(matches!(
            LanguageGrammar::from_name("Rust").unwrap(),
            LanguageGrammar::OtherByName(_)
        ));
        assert!(matches!(
            LanguageGrammar::from_name("YAML").unwrap(),
            LanguageGrammar::OtherByName(_)
        ));
        let unknown = LanguageGrammar::from_name("Totally Made Up Language");
        assert!(matches!(unknown, Err(LanguageGrammarError::UnknownGrammar(_))));
    }

    #[test]
    fn from_filename_rejects_whitespace() {
        let err = LanguageGrammar::from_filename("rust title=\"hi\"").unwrap_err();
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn from_filename_resolves_extensionless_well_known_basenames() {
        // Spec contract: extensionless well-known source filenames resolve
        // through the same alias table used by token lookup. The stored token
        // preserves the caller's spelling; resolution still succeeds.
        for name in ["Dockerfile", "Makefile", "dockerfile", "makefile"] {
            let grammar = LanguageGrammar::from_filename(name)
                .unwrap_or_else(|_| panic!("{name} should resolve"));
            assert!(
                matches!(grammar, LanguageGrammar::OtherByToken(_)),
                "{name} should resolve to OtherByToken, got {grammar:?}"
            );
            assert!(grammar.resolve_default().is_ok(), "{name} should resolve");
        }
    }

    #[test]
    fn from_filename_extensionless_resolves_make_alias() {
        // The shared extensionless alias table maps `make` -> Makefile.
        let grammar = LanguageGrammar::from_filename("make").expect("make alias resolves");
        assert!(matches!(grammar, LanguageGrammar::OtherByToken(_)));
        assert!(grammar.resolve_default().is_ok());
    }

    #[test]
    fn make_alias_resolves_across_token_paths() {
        // Spec contract: `make` resolves to the Makefile grammar on every
        // public token path, preserving the caller's spelling.
        for grammar in [
            LanguageGrammar::from_token("make").expect("from_token(make)"),
            LanguageGrammar::try_from("make").expect("try_from(make)"),
            LanguageGrammar::from_str("make").expect("from_str(make)"),
        ] {
            assert!(
                matches!(grammar, LanguageGrammar::OtherByToken(ref s) if s == "make"),
                "make should preserve its spelling as OtherByToken, got {grammar:?}"
            );
            let syntax = grammar.resolve_default().expect("make resolves to a syntax");
            assert_eq!(syntax.name, "Makefile", "make should resolve to Makefile, got {}", syntax.name);
            assert_eq!(format!("{grammar}"), "make", "Display should echo the token");
        }

        // Lossy paths never fail: unknown becomes PlainText, but `make` resolves.
        let lossy = LanguageGrammar::from_lossy("make");
        assert!(
            matches!(lossy, LanguageGrammar::OtherByToken(ref s) if s == "make"),
            "from_lossy(make) should resolve, got {lossy:?}"
        );
        assert!(lossy.resolve_default().is_ok());

        let token_or_plain = LanguageGrammar::from_token_or_plain_text("make");
        assert!(
            matches!(token_or_plain, LanguageGrammar::OtherByToken(ref s) if s == "make"),
            "from_token_or_plain_text(make) should resolve, got {token_or_plain:?}"
        );
    }

    #[test]
    fn from_filename_extensionless_unknown_returns_error() {
        let err = LanguageGrammar::from_filename("not-a-known-basename")
            .expect_err("unknown basename should not resolve");
        assert!(matches!(err, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn from_extension_case_insensitive_for_dynamic_extension() {
        // `cs` is a non-aliased two-face extension (C#) that does not appear
        // in `grammar_from_alias`. Mixed-case and uppercase inputs must still
        // resolve, and the stored spelling must canonicalize to lowercase so
        // downstream `Display` emits a stable fence token instead of echoing
        // the caller's casing.
        for input in ["cs", "CS", "Cs", "cS"] {
            let grammar = LanguageGrammar::from_extension(input)
                .unwrap_or_else(|_| panic!("{input} should resolve"));
            assert!(
                matches!(grammar, LanguageGrammar::OtherByExtension(ref s) if s == "cs"),
                "{input} should canonicalize to OtherByExtension(\"cs\"), got {grammar:?}"
            );
            assert!(
                grammar.resolve_default().is_ok(),
                "{input} should resolve to a syntax"
            );
        }

        // Unknown extensions still canonicalize the error spelling.
        let err = LanguageGrammar::from_extension("NOPE").unwrap_err();
        assert!(
            matches!(err, LanguageGrammarError::UnknownGrammar(ref s) if s == "nope"),
            "unknown extension error should canonicalize to lowercase, got {err:?}"
        );
    }

    #[test]
    fn resolve_default_uses_two_face_syntax_set() {
        // TypeScript is only available in the two-face extended set.
        let grammar = LanguageGrammar::from_extension("ts").unwrap();
        assert!(grammar.resolve_default().is_ok());
    }

    #[test]
    fn resolve_via_extension_finds_rust() {
        let grammar = LanguageGrammar::from_token("rs").unwrap();
        let syntax = grammar.resolve(syntax_set()).expect("rs resolves");
        assert_eq!(syntax.name, "Rust");
    }

    #[test]
    fn resolve_unknown_token_errors() {
        let grammar = LanguageGrammar::from_token("unknown_language_xyz").unwrap_err();
        assert!(matches!(grammar, LanguageGrammarError::UnknownGrammar(_)));
    }

    #[test]
    fn resolve_is_case_insensitive_for_names() {
        for token in ["rust", "Rust", "RUST"] {
            let grammar = LanguageGrammar::from_token(token).unwrap();
            let syntax = grammar.resolve(syntax_set()).expect("{token} resolves");
            assert_eq!(syntax.name, "Rust", "{token} should resolve to Rust");
        }
    }

    #[test]
    fn resolve_alias_bash_family() {
        for token in ["bash", "sh", "shell", "zsh"] {
            let grammar = LanguageGrammar::from_token(token).unwrap();
            let syntax = grammar.resolve(syntax_set()).expect("{token} resolves");
            assert!(
                syntax.name.to_ascii_lowercase().contains("bash")
                    || syntax.name.to_ascii_lowercase().contains("bourne"),
                "{token} should resolve to a bash syntax, got {}",
                syntax.name
            );
        }
    }

    #[test]
    fn resolve_alias_javascript_and_typescript() {
        for token in ["js", "javascript"] {
            let grammar = LanguageGrammar::from_token(token).unwrap();
            let syntax = grammar.resolve(syntax_set()).expect("{token} resolves");
            assert!(
                syntax.name.contains("JavaScript"),
                "{token} should resolve to a JavaScript syntax, got {}",
                syntax.name
            );
        }
        for token in ["ts", "typescript"] {
            let grammar = LanguageGrammar::from_token(token).unwrap();
            let syntax = grammar.resolve(syntax_set()).expect("{token} resolves");
            assert_eq!(syntax.name, "TypeScript", "{token} should resolve to TypeScript");
        }
    }

    #[test]
    fn resolve_python_family() {
        for token in ["python", "Python", "py"] {
            let grammar = LanguageGrammar::from_token(token).unwrap();
            let syntax = grammar.resolve(syntax_set()).expect("{token} resolves");
            assert_eq!(syntax.name, "Python", "{token} should resolve to Python");
        }
    }
}

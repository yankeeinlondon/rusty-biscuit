# Language Grammar Resolution

## Summary

Darkmatter should have one authoritative grammar lookup API for syntect-backed
syntax highlighting: `LanguageGrammar`. All production code that needs to turn a
user-provided language, filename, extension, or syntect name into a syntax grammar
must route through this type instead of calling `SyntaxSet::find_syntax_by_*`
directly.

This feature makes `LanguageGrammar` ergonomic enough to use everywhere, then
migrates existing rendering, YAML highlighting, and code transclusion paths onto
it.

## Problem

`LanguageGrammar` exists, but it is not currently the single resolution authority.
Several live code paths still perform their own lookup:

- Markdown/code-block rendering uses a local `find_syntax` helper.
- YAML highlighting directly asks syntect for the `yaml` extension/name.
- Code transclusion extension inference uses a separate default `SyntaxSet` and
  only checks extensions.

This creates drift. For example, `LanguageGrammar` knows aliases such as `sh`,
`python`, `tsx`, and `yml`, while the code-block renderer's local resolver has a
smaller alias map. A caller can construct a `CodeBlock` with a `LanguageGrammar`,
but rendering converts it back to a string and re-resolves it through the older
helper.

## Goals

- Provide fallible and infallible public ways to construct a `LanguageGrammar`.
- Accept the input forms users naturally provide: fence info strings, language
  names, file extensions, and filenames.
- Treat Markdown fence info defensively by ignoring key/value metadata after the
  language token.
- Preserve explicit lookup APIs for callers that know whether they have a name,
  extension, token, or filename.
- Make `LanguageGrammar` the only production grammar lookup path.
- Keep custom `SyntaxSet` resolution possible for tests and advanced callers.
- Update comments, rustdoc, docs, and the `darkmatter` skill so the rule is
  discoverable.

## Non-Goals

- Do not redesign theme selection.
- Do not add support for loading arbitrary external `.sublime-syntax` files.
- Do not guarantee every syntect grammar has a named enum variant.
- Do not preserve redundant public API surface merely for compatibility; this API
  does not yet have real external customers.

## API Design

### Type

`LanguageGrammar` remains the public typed resolver for fenced code-block
languages.

It should continue to include common named variants such as:

```rust
pub enum LanguageGrammar {
    Rust,
    JavaScript,
    TypeScript,
    Go,
    Php,
    Python,
    Bash,
    Html,
    Css,
    Markdown,
    Yaml,
    Json,
    Toml,
    PlainText,
    OtherByExtension(String),
    OtherByName(String),
    OtherByToken(String),
}
```

The exact variant list can stay `#[non_exhaustive]`. `PlainText` should represent
syntect's plain-text grammar and be the infallible fallback.

### Fallible Construction

Implement:

```rust
impl TryFrom<&str> for LanguageGrammar;
impl TryFrom<String> for LanguageGrammar;
impl std::str::FromStr for LanguageGrammar;
```

`TryFrom<&str>` and `FromStr` should share behavior.

Important Rust constraint: do **not** also implement `From<&str>` or
`From<String>` if `TryFrom<&str>` / `TryFrom<String>` are custom. Rust's blanket
`TryFrom<U> for T where U: Into<T>` conflicts with having both a fallible and an
infallible conversion for the same source type.

### Infallible Construction

Instead of `From<&str>`, expose named infallible constructors:

```rust
impl LanguageGrammar {
    /// Parses user-provided grammar text and falls back to plain text when no
    /// Darkmatter grammar matches.
    ///
    /// This is the preferred infallible entry point for UI, CLI, and rendering
    /// paths that should never fail just because a user supplied an unknown
    /// language token.
    pub fn from_lossy(input: impl AsRef<str>) -> Self;

    /// Parses a Markdown fence-style language token and falls back to plain
    /// text when no Darkmatter grammar matches.
    ///
    /// Unlike [`from_lossy`], this name makes the fence-token behavior explicit:
    /// unquoted input is read only up to the first ASCII whitespace character,
    /// while quoted input uses the quoted token.
    pub fn from_token_or_plain_text(input: impl AsRef<str>) -> Self;
}
```

Both should use the fallible path and return `LanguageGrammar::PlainText` when no
match is found. Pick one final public name; `from_token_or_plain_text` is more
explicit, while `from_lossy` is shorter.

### Explicit Constructors

Expose public constructors for callers that know what they are passing:

```rust
impl LanguageGrammar {
    /// Parses a Markdown fence-style language token.
    ///
    /// Unquoted input is read only up to the first ASCII whitespace character so
    /// metadata such as `title="hi"` is ignored. Quoted input uses the quoted
    /// value as the token. Returns an error when the normalized token is empty
    /// or cannot be resolved by Darkmatter's default grammar set.
    pub fn from_token(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError>;

    /// Resolves a file extension to a Darkmatter grammar.
    ///
    /// The input may include or omit a leading dot. Lookup is case-insensitive
    /// for user ergonomics, but the returned grammar keeps Darkmatter's
    /// canonical spelling.
    pub fn from_extension(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError>;

    /// Resolves a syntect display name to a Darkmatter grammar.
    ///
    /// This is for callers that know they have a grammar name such as `Rust`,
    /// `YAML`, `Dockerfile`, or `Bourne Again Shell (bash)`. Prefer
    /// [`from_token`] for Markdown fence input and [`from_extension`] for file
    /// suffixes.
    pub fn from_name(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError>;

    /// Resolves a filename or path to a Darkmatter grammar.
    ///
    /// Paths with extensions resolve by their final extension. Extensionless
    /// well-known filenames such as `Makefile` and `Dockerfile` resolve through
    /// the same alias table used by token lookup.
    pub fn from_filename(input: impl AsRef<str>) -> Result<Self, LanguageGrammarError>;

    /// Returns the plain-text grammar used as the infallible fallback.
    pub fn plain_text() -> Self;

    /// Alias for [`plain_text`].
    pub fn text() -> Self;

    /// Returns the YAML grammar.
    ///
    /// This constructor is infallible because YAML is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn yaml() -> Self;

    /// Returns the Rust grammar.
    ///
    /// This constructor is infallible because Rust is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn rust() -> Self;

    /// Returns the Markdown grammar.
    ///
    /// This constructor is infallible because Markdown is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn markdown() -> Self;

    /// Returns the JSON grammar.
    ///
    /// This constructor is infallible because JSON is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn json() -> Self;

    /// Returns the TOML grammar.
    ///
    /// This constructor is infallible because TOML is guaranteed by
    /// Darkmatter's default grammar set.
    pub fn toml() -> Self;
}
```

Convenience functions should return named variants where possible. They should
not perform lookup work and should not be fallible. Darkmatter-known grammars
such as YAML, Rust, Markdown, JSON, and TOML are guaranteed by the default
two-face grammar set.

### Input Normalization

All public constructors that accept user text should trim surrounding whitespace.

For token-like input, including `TryFrom<&str>` and `from_token`:

- If input starts with a quote (`"` or `'`), read until the matching closing quote
  and use the quoted content as the token.
- If input is not quoted, read only up to the first ASCII whitespace character.
- This intentionally handles Markdown fence strings like
  `rust title="hi" highlight="1-2"` by resolving only `rust`.
- Empty normalized input is an error for fallible constructors and plain text for
  infallible constructors.

Filename detection for `TryFrom<&str>`:

- If the normalized input appears to be a filename or path, extract the final file
  extension and try extension lookup.
- A string appears to be a filename/path when it contains a path separator, has a
  basename with a dot and a non-empty suffix, or otherwise matches a common
  source filename form that Darkmatter supports.
- Extension lookup should be case-insensitive for user ergonomics.
- Filenames without extensions, such as `Makefile` or `Dockerfile`, should still
  resolve through name/token aliases.

When input is not treated as a filename:

- Try token/name/extension resolution through the shared resolver.
- Prefer explicit aliases before falling back to dynamic `OtherByToken` only when
  the default syntax set can resolve the token.

### Resolution

`LanguageGrammar::resolve(&self, syntax_set: &SyntaxSet)` remains the advanced
resolver for custom syntax sets.

Add an ergonomic default resolver if useful:

```rust
impl LanguageGrammar {
    /// Resolves this grammar against Darkmatter's default two-face syntax set.
    ///
    /// Known variants such as [`Yaml`] and [`Rust`] are expected to resolve
    /// successfully because Darkmatter treats them as guaranteed grammars.
    /// Dynamic variants can still fail if they were constructed manually with
    /// an unsupported extension, name, or token.
    pub fn resolve_default(&self) -> Result<&'static SyntaxReference, LanguageGrammarError>;

    /// Resolves this grammar against a caller-provided syntax set.
    ///
    /// This is primarily for tests and future advanced callers that supply a
    /// custom syntect grammar set. It remains fallible because custom syntax
    /// sets may omit grammars that Darkmatter's default set guarantees.
    pub fn resolve_with(
        &self,
        syntax_set: &SyntaxSet,
    ) -> Result<&SyntaxReference, LanguageGrammarError>;
}
```

The fallible constructors should validate against Darkmatter's default grammar
set, which is the two-face extended syntax set already used by code highlighting.
This ensures `TryFrom<&str>` fails when Darkmatter cannot actually highlight the
grammar.

## Resolver Rules

The private resolver currently called `find_via_token` should become the central
implementation behind all public construction paths. It must be reviewed and
made robust enough to be the single source of truth.

Required aliases:

| Input | Resolves To |
|---|---|
| `rust`, `rs` | Rust |
| `javascript`, `js` | JavaScript |
| `typescript`, `ts`, `tsx` | TypeScript |
| `python`, `python3`, `py` | Python |
| `bash`, `sh`, `shell`, `zsh` | Bash |
| `yaml`, `yml` | YAML |
| `markdown`, `md` | Markdown |
| `json` | JSON |
| `toml` | TOML |
| `html`, `htm` | HTML |
| `css` | CSS |
| `php` | PHP |
| `go`, `golang` | Go |
| `c++`, `cpp` | C++ grammar by extension |
| `dockerfile` | Dockerfile grammar by name/token |
| `makefile`, `make` | Makefile grammar by name/token |
| `txt`, `text`, `plain`, `plaintext`, `plain-text` | PlainText |

Resolution order should be documented and tested. A reasonable order is:

1. Normalize the input.
2. Apply explicit aliases to known variants.
3. Try extension lookup.
4. Try exact name lookup.
5. Try case-insensitive name lookup.
6. Return `OtherByToken` only if the default syntax set can resolve it.
7. Return `UnknownGrammar` for fallible paths, or `PlainText` for infallible
   paths.

## Migration Plan

### Code-Block Rendering

Remove the local `find_syntax` helper in
`darkmatter/lib/src/markdown/output/code_block.rs`.

The terminal and HTML code-block renderers should accept either a
`LanguageGrammar` or normalize their existing `&str` argument through
`LanguageGrammar::from_token_or_plain_text`, then call `resolve`.

`CodeBlock` should not convert a stored `LanguageGrammar` back into a string and
then rely on a second resolver. Its render path should preserve the grammar value
until the final syntect lookup.

### YAML Highlighting

`highlight_yaml_lines_with_theme` should use `LanguageGrammar::yaml().resolve(...)`
instead of direct `find_syntax_by_extension("yaml")` calls.

### Code Transclusion

`infer_language(path, fallback)` should use `LanguageGrammar::from_filename(path)`
for validation against the same syntax set used by rendering. It should not load
or own a separate `SyntaxSet::load_defaults_newlines()`.

The returned fence token can remain the lowercase extension for byte-compatible
Markdown output, but the decision that the extension is supported should come
from `LanguageGrammar`.

### Tests

Keep direct `SyntaxSet::find_syntax_by_*` calls only in tests that intentionally
assert syntect baseline behavior. Production tests should prefer
`LanguageGrammar` so they exercise the public contract.

## Documentation

Update rustdoc for `LanguageGrammar` to state:

- It is Darkmatter's authoritative grammar lookup API.
- `from_token` is for Markdown fence info strings.
- `from_extension`, `from_name`, and `from_filename` are for explicit caller
  intent.
- Infallible helpers fall back to plain text.
- Production code should not call `SyntaxSet::find_syntax_by_*` directly.

Update the `darkmatter` skill with the same rule:

> Use `LanguageGrammar` for all grammar lookup. Do not call
> `SyntaxSet::find_syntax_by_extension`, `find_syntax_by_name`, or equivalent
> syntect lookup APIs directly in production code outside the `LanguageGrammar`
> implementation.

## Acceptance Criteria

- `LanguageGrammar::try_from("rust title=\"hi\"")` resolves to Rust.
- `LanguageGrammar::try_from("\"my custom grammar\" title=\"hi\"")` uses the
  quoted token.
- `LanguageGrammar::try_from("src/main.rs")` resolves to Rust.
- `LanguageGrammar::try_from("config.yml")` resolves to YAML.
- `LanguageGrammar::try_from("Dockerfile")` resolves to Dockerfile.
- `LanguageGrammar::try_from("unknown-language-xyz")` returns
  `LanguageGrammarError::UnknownGrammar`.
- The infallible lossy constructor returns `PlainText` for unknown input.
- All production grammar lookup paths in Darkmatter route through
  `LanguageGrammar`.
- The only production direct `find_syntax_by_*` calls are inside
  `language_grammar.rs` or syntax-set loading tests.
- Relevant comments and docs no longer claim duplicated resolvers mirror each
  other.

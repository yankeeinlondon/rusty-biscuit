---
created: 2026-06-15
reviewed: true
status: finalized for planning and implementation
---

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

`PlainText` semantics (filling a gap — these were unspecified):

- `LanguageGrammar::resolve` / `resolve_default` for `PlainText` must call
  `syntax_set.find_syntax_plain_text()` and therefore is **infallible** (it never
  yields `UnknownGrammar`).
- `Display` for `PlainText` must produce the **empty string**, not `"plaintext"`.
  The HTML renderer gates the `class="language-…"` attribute on a non-empty
  language, and a Markdown fence with no language re-emits an empty info token.
  Making `PlainText` display empty keeps "unknown input fell back to plain text"
  byte-identical to today's "no language" path (where `find_syntax` returns
  `None` and the caller substitutes `find_syntax_plain_text()`).
- `PlainText` is reserved for the infallible fallback and for direct
  `LanguageGrammar::plain_text()` / `text()` construction. Explicit fence tokens
  such as `txt`, `text`, `plain`, `plaintext`, and `plain-text` must resolve
  through a token-preserving dynamic variant so Markdown round-trip and
  `language-…` HTML class emission keep the user-provided token.

### Disposition of the Existing `from_fence_token` Constructor

The current code ships an infallible
`LanguageGrammar::from_fence_token(impl AsRef<str>) -> Self` that returns
`OtherByToken(token)` (never `PlainText`) for unknown input. It is the
constructor `CodeBlock::with_fence_language`, `CodeBlock::rust/yaml/json/toml`,
and `CodeBlock::from_source_file` currently call.

Decision: **remove `from_fence_token`** and replace its call sites with
`from_token_or_plain_text` (the token-only infallible constructor below). Per the
Non-Goals, redundant public surface is not preserved for compatibility, and
`from_fence_token`'s "unknown → `OtherByToken`" behavior is superseded by
"unknown → `PlainText`". All in-tree callers (`code_block.rs`) must be updated in
the same change. This is a behavior change for unknown tokens: an unknown fence
token now stores `PlainText` rather than `OtherByToken("…")`. Because both resolve
to the plain-text grammar at render time, rendered terminal/HTML output is
unchanged; the only observable difference is the stored variant and the
`Display`/fence token for truly unknown languages. Explicit plain-text tokens are
handled by the token-preserving rule above.

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

These constructors are intentionally distinct:

- `from_lossy` is the infallible counterpart of `TryFrom<&str>` / `FromStr`. It
  runs the **full** resolution, including filename/path detection, so
  `from_lossy("src/main.rs")` resolves to `Rust`.
- `from_token_or_plain_text` is the infallible counterpart of `from_token`. It
  runs **token-only** resolution (no filename detection), so
  `from_token_or_plain_text("src/main.rs")` reads the token up to the first
  whitespace (`src/main.rs`), fails to resolve it as a language, and returns
  `PlainText`.

Both return `LanguageGrammar::PlainText` when their underlying fallible path
returns `UnknownGrammar`. Keep both names; do not collapse them. Rendering and
fence paths (which receive a fence info string, never a path) should call
`from_token_or_plain_text`; general user/CLI input that might be a filename should
call `from_lossy`.

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

- If the trimmed input contains unquoted ASCII whitespace, treat it as fence info
  and use the token path; **skip filename detection entirely.**
- Otherwise, if the (whitespace-free) normalized input appears to be a filename or
  path, extract the final file extension and try extension lookup.
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

`LanguageGrammar::resolve(&self, syntax_set: &SyntaxSet)` remains the resolver for
caller-provided (custom) syntax sets. It is already public, already used, and
already tested; **keep its name.**

Do not add a separate `resolve_with` method. It would have the same signature and
behavior as the existing `resolve(&self, &SyntaxSet)`, adding redundant public
surface without a new capability.

Add the ergonomic default resolver:

```rust
impl LanguageGrammar {
    /// Resolves this grammar against Darkmatter's default two-face syntax set.
    ///
    /// Known variants such as [`Yaml`] and [`Rust`] are expected to resolve
    /// successfully because Darkmatter treats them as guaranteed grammars.
    /// `PlainText` always resolves. Dynamic variants can still fail if they were
    /// constructed manually with an unsupported extension, name, or token.
    pub fn resolve_default(&self) -> Result<&'static SyntaxReference, LanguageGrammarError>;
}
```

`resolve_default` is `self.resolve(load_syntax_set())` against the shared static
two-face set, returning a `'static` reference.

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
| `txt`, `text`, `plain`, `plaintext`, `plain-text` | Plain Text grammar via a token-preserving dynamic variant |

Explicit plain-text tokens are intentionally not aliases for
`LanguageGrammar::PlainText`. They should resolve to syntect's Plain Text grammar
through `OtherByToken` or `OtherByExtension` so the original fence token and
`language-…` HTML class survive round-trip. `LanguageGrammar::PlainText` means
"no grammar matched; use the plain-text fallback."

Resolution order should be documented and tested. The order below governs the
**token / lossy / `TryFrom` / `FromStr`** paths only.

The explicit constructors (`from_name`, `from_extension`, `from_token`,
`from_filename`) deliberately do **not** run this full ladder because they express
known caller intent. `from_extension` does extension lookup plus the extension
alias map, `from_name` does exact then case-insensitive name lookup, and so on.
Only inputs of unknown shape walk the full order.

1. Normalize the input.
2. Apply explicit aliases to known variants.
3. Try extension lookup.
4. Try exact name lookup.
5. Try case-insensitive name lookup.
6. Return `OtherByToken` only if the default syntax set can resolve it.
7. Return `UnknownGrammar` for fallible paths, or `PlainText` for infallible
   paths.

Empty normalized input on a fallible path returns
`LanguageGrammarError::UnknownGrammar(String::new())` (no new error variant is
needed); on an infallible path it returns `PlainText`.

## Migration Plan

### Code-Block Rendering

Remove the local `find_syntax` helper in
`darkmatter/lib/src/markdown/output/code_block.rs` (the `find_syntax` unit tests
there move to `language_grammar.rs`).

`render_terminal_code_block` and `render_html_code_block` should accept
`&LanguageGrammar` instead of `language: &str`. They should call
`grammar.resolve_default()` for syntax resolution and use a separate display
label for HTML class / Markdown fence emission. This removes the grammar →
string → grammar round-trip while keeping labeling and syntax resolution
decoupled.

`CodeBlock` should not convert a stored `LanguageGrammar` back into a string and
then rely on a second resolver for **syntax resolution**. Its render path should
preserve the grammar value until the final syntect lookup
(`grammar.resolve_default()` / `resolve`).

`CodeBlock::render` currently derives `language_label` via
`self.language.as_ref().map(|g| g.to_string())` (`code_block.rs:254`) and the
Markdown target re-emits the fence as `{lang} {raw_meta}`. Eliminating the
*resolver* round-trip must not eliminate this *display* string: the HTML
`language-…` class, the Markdown fence token, and TOC change-detection all depend
on it. The migration must keep a `Display`-derived or `raw_meta`-derived label
flowing to those surfaces while routing **resolution** through the typed grammar.
Explicit plain-text tokens (`txt`, `text`, etc.) must use token-preserving
dynamic variants so their display label does not collapse to `PlainText`'s empty
display string.

### YAML Highlighting

`highlight_yaml_lines_with_theme` should use `LanguageGrammar::yaml().resolve(...)`
instead of direct `find_syntax_by_extension("yaml")` calls.

### Code Transclusion

`infer_language(path, fallback)` should use `LanguageGrammar::from_filename(path)`
for validation against the same syntax set used by rendering. It should not load
or own a separate `SyntaxSet::load_defaults_newlines()` (the `lazy_static!
SYNTAX_SET` in `compose/transclusion/code.rs` is deleted).

The returned fence token can remain the lowercase extension for byte-compatible
Markdown output, but the decision that the extension is supported should come
from `LanguageGrammar`.

Today `infer_language` validates against `SyntaxSet::load_defaults_newlines()`:
syntect's **bare default** set. Rendering and `LanguageGrammar` validate against
the **two-face extended** set, which carries grammars syntect defaults lack
(TypeScript, TOML, Dockerfile, and others). Unifying on `LanguageGrammar`
therefore widens the set of extensions `infer_language` recognizes. For a file
whose extension exists only in two-face (e.g. a `.ts` or `.toml` source
transcluded via `::code`), `infer_language` previously returned the `fallback`
token and will now return the real extension token. **This changes the composed
Markdown output** (the fence info string) for those files.

This is an intended consequence of the feature's central goal: one grammar set
everywhere. It is desirable because language tagging improves, but it is **not**
byte-compatible for widened extensions, only for extensions present in both sets.
Mitigation required by this spec:

- Update any transclusion golden/snapshot tests that assumed the narrower
  syntect-defaults behavior; add coverage for at least one two-face-only
  extension (e.g. `.ts`) asserting the real token is now emitted.
- Note the behavior change in the feature's drift updates (README / skill) so
  downstream composed documents are not mistaken for regressions.

### Tests

Keep direct `SyntaxSet::find_syntax_by_*` calls only in tests that intentionally
assert syntect baseline behavior. Production tests should prefer
`LanguageGrammar` so they exercise the public contract.

A repo scan found three additional direct `find_syntax_by_extension("yaml")`
call sites beyond those named in the Migration Plan:
`render_tree/code_renderer.rs:426`, `render_tree/entrypoints.rs:818`, and
`cli/src/commands/schema/about.rs:521`. All three are inside `#[cfg(test)]`
`one_half_yaml_color` helpers, so they are **out of scope** for the production
migration. They may stay as-is because they assert syntect's own YAML token
coloring.

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
- `LanguageGrammar::try_from("rust title=\"a.b\"")` resolves to Rust (the dotted
  metadata is not mistaken for a filename extension).
- `LanguageGrammar::from_lossy("src/main.rs")` resolves to Rust, while
  `LanguageGrammar::from_token_or_plain_text("src/main.rs")` returns `PlainText`
  (filename detection is exclusive to the full/lossy path).
- Explicit plain-text tokens such as `txt` and `text` resolve to the syntect
  Plain Text grammar through a token-preserving dynamic variant, not
  `LanguageGrammar::PlainText`.
- All production grammar lookup paths in Darkmatter route through
  `LanguageGrammar`.
- The only **production** (non-`#[cfg(test)]`) direct `find_syntax_by_*` calls are
  inside `language_grammar.rs`. Direct calls inside `#[cfg(test)]` modules
  (including the `one_half_yaml_color` helpers in `code_renderer.rs`,
  `entrypoints.rs`, and `cli/.../schema/about.rs`, and the syntax-set loading
  tests in `grammars.rs`) are permitted and explicitly out of scope.
- `from_fence_token` is removed; no in-tree caller references it.
- `compose/transclusion/code.rs` no longer owns a private
  `SyntaxSet::load_defaults_newlines()`.
- Relevant comments and docs no longer claim duplicated resolvers mirror each
  other.

## Follow-Up: Summary-and-Suggest Plan Coordination

The completed plan at
`darkmatter/features/2026-06-14-summary-and-suggest/plan.md` was written from a
review that predates this grammar specification. Before executing or re-opening
that plan, update it so its code-block serialization, rendering, and verification
steps do not conflict with this feature.

Assume the summary-and-suggest plan will not be executed until this grammar spec
is implemented. The update should explicitly account for:

- `LanguageGrammar` being the single production grammar lookup path.
- `from_fence_token` being removed in favor of `from_token_or_plain_text`,
  `from_lossy`, and the explicit fallible constructors.
- Code-block render helpers accepting `&LanguageGrammar` for syntax resolution
  rather than resolving a raw `&str` through a local helper.
- Explicit plain-text fence tokens (`txt`, `text`, etc.) preserving their display
  token while resolving to the syntect Plain Text grammar.
- Code transclusion using the two-face-backed `LanguageGrammar` path rather than
  a private `SyntaxSet::load_defaults_newlines()`.

The summary-and-suggest plan's acceptance checks should include a guard that no
completed maintenance step reintroduces direct production
`SyntaxSet::find_syntax_by_*` lookup outside `language_grammar.rs`.

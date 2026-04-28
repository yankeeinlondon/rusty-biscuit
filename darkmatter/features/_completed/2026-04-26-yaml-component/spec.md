# YAML Component

## Goal

Provide an ergonomic way to ingest YAML data — from a raw string, a markdown file's frontmatter, or a YAML file on disk — and render it as a syntax-highlighted code block in both terminal and browser/HTML, by delegating to darkmatter's existing highlighting pipeline.

The value of this feature is **ergonomic ingestion**, not new visuals. `YamlBlock` is a typed wrapper around a validated YAML string that reuses the existing `syntect` / `two-face` highlighting in `darkmatter/lib/src/markdown/highlighting/`.

## Public API

```rust
impl YamlBlock {
    /// Construct from a raw YAML string. Validates by parsing via `serde_yaml_ng`.
    pub fn new<T: Into<String>>(yaml: T) -> Result<Self, YamlBlockError>;

    /// Construct from raw markdown content. Extracts YAML frontmatter and validates.
    /// If the markdown contains no frontmatter, the YamlBlock contains an empty mapping.
    pub fn from_markdown_content<T: Into<String>>(md: T) -> Result<Self, YamlBlockError>;

    /// Construct from a markdown file on disk. Reads the file, extracts frontmatter, validates.
    pub fn from_markdown_file<P: AsRef<Path>>(path: P) -> Result<Self, YamlBlockError>;

    /// Construct from a YAML file on disk. Reads the file and validates.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, YamlBlockError>;
}
```

`YamlBlock` will implement [`Renderable`] and [`BrowserRenderable`] from the [`biscuit-terminal`](../../../../../biscuit-terminal/) package.

## Constructor Semantics

- **Argument types are split by kind.** YAML/markdown content uses `Into<String>`; file paths use `AsRef<Path>`. This matches monorepo precedent (`Markdown::try_from(Path)`, `TerminalImage::new(path)`, `compose_with_source_file`).
- **All constructors validate at construction time.** YAML is parsed via `serde_yaml_ng::from_str` and any error fails fast — bad YAML never reaches the renderer.
- **The parsed `serde_yaml_ng::Value` is not retained.** `YamlBlock` stores the raw YAML text; parsing exists only for validation. This keeps the type small and avoids leaking `serde_yaml_ng::Value` into the public API.
- **Markdown extraction is frontmatter-only.** `from_markdown_content` and `from_markdown_file` reuse the existing frontmatter parser at `darkmatter/lib/src/markdown/frontmatter.rs`. They do **not** scan the markdown body for fenced ` ```yaml ` blocks.
- **Missing or empty frontmatter ⇒ empty mapping.** A markdown document with no frontmatter produces a valid `YamlBlock` whose payload is `{}`. (This replaces the previous spec's ambiguous "if there's no markdown then YAML is an empty object" line.)
- **Malformed frontmatter ⇒ `YamlBlockError::MarkdownParse`** (wrapping `MarkdownError::FrontmatterParse`). This preserves the rich diagnostics surfaced by `Markdown::try_from_content`. See tech-design §"Error Type" for the rationale. Note: `YamlBlockError::YamlParse` is still surfaced for `new`, `from_yaml_file`, and any re-serialization validation failures inside `from_markdown_content` / `from_markdown_file`.

## Rendering

`YamlBlock` produces a styled fenced code block — nothing more.

- **Terminal (`Renderable`):** delegates to the existing `darkmatter/lib/src/markdown/highlighting/` module, producing the same ANSI output you would get from a ` ```yaml ` fenced block in a markdown document.
- **Browser (`BrowserRenderable`):** emits `<pre><code class="language-yaml">…</code></pre>` using the same CSS-variable-driven theme classes the rest of darkmatter already uses for fenced code.
- **Theme detection:** light/dark mode is handled by the existing `themes.rs::detect_color_mode()`. `YamlBlock` does not introduce its own theming.
- **No new renderer is needed.** This feature does not add tree views, collapsible nodes, or per-key styling.

## Errors

`YamlBlockError` is a `thiserror::Error` enum with the following variants:

- `Io(std::io::Error)` — file read failures from `from_markdown_file` / `from_yaml_file`.
- `YamlParse(serde_yaml_ng::Error)` — malformed YAML in `new`, `from_yaml_file`, or in extracted frontmatter.
- `MarkdownParse(...)` — only if frontmatter extraction itself fails (separate from a YAML parse failure inside the frontmatter block). The exact inner type depends on what the existing frontmatter parser surfaces; see Open Questions.

## Scope

- Programmatic, standalone component. Constructors take a string, a markdown source, or a file path and produce a single styled YAML code block.
- Reuses the existing markdown highlighting pipeline; introduces no new rendering code paths.
- Reuses the existing frontmatter parser for markdown ingestion.

## Out of Scope

- Tree / structural views, collapsible nodes, or type-aware per-key styling.
- Extracting fenced ` ```yaml ` blocks from markdown bodies (only frontmatter is extracted).
- Multi-document YAML streams (`---`-separated documents producing multiple values).
- Special-case rendering of YAML anchors, aliases, or tags — they remain raw YAML text and are highlighted as such.
- Any persistence, editing, or programmatic access to the parsed YAML value.
- Changes to markdown's existing yaml-fence rendering. A ` ```yaml ` fence inside a markdown document continues to be rendered by the existing pipeline; it is **not** routed through `YamlBlock`.

A future opt-in tree view (e.g. `.with_view(YamlView::Tree)`) could be added later without breaking changes, but is explicitly excluded from this feature.

## Open Questions

- **Frontmatter parser error type.** Does the existing `darkmatter/lib/src/markdown/frontmatter.rs` parser surface a single, named error type that can be wrapped in `YamlBlockError::MarkdownParse`, or does it only return YAML parse errors? If the latter, `MarkdownParse` may collapse into `YamlParse` and not be needed as a separate variant.
- **Theming knobs.** Does `YamlBlock` need page-level frontmatter knobs (analogous to the documented `hr:` pattern in `darkmatter/SKILL.md`) or per-instance attribute overrides for things like theme selection or language label? No design has been chosen here; the default is "no knobs — inherit whatever the existing highlighting pipeline does."

## Acceptance Criteria

1. `YamlBlock::new("foo: 1")` returns `Ok(YamlBlock)`; `YamlBlock::new("foo: : :")` returns `Err(YamlBlockError::YamlParse(_))`. A markdown document with a malformed frontmatter block (e.g. `"---\nfoo: : :\n---\n"`) returned from `YamlBlock::from_markdown_content` returns `Err(YamlBlockError::MarkdownParse(_))` (the rich error from `Markdown::try_from_content`), not `YamlParse`.
2. `YamlBlock::from_yaml_file(path)` on a missing path returns `Err(YamlBlockError::Io(_))`; on a malformed YAML file returns `Err(YamlBlockError::YamlParse(_))`.
3. `YamlBlock::from_markdown_content("# hello\n")` (no frontmatter) returns `Ok(YamlBlock)` whose rendered payload is the empty mapping `{}`.
4. `YamlBlock::from_markdown_content("---\nfoo: 1\n---\nbody\n")` returns `Ok(YamlBlock)` containing the frontmatter YAML only; the body is ignored.
5. `YamlBlock::from_markdown_file(path)` on a missing path returns `Err(YamlBlockError::Io(_))`.
6. Rendering a `YamlBlock` with content `X` (terminal) is byte-identical to rendering a markdown document containing only a ` ```yaml `-fenced block with content `X`, under the same theme.
7. Rendering a `YamlBlock` with content `X` (browser) emits `<pre><code class="language-yaml">…</code></pre>` and uses the same theme CSS classes/variables as a ` ```yaml ` fenced block in a markdown document.
8. Light-mode rendering and dark-mode rendering each have at least one passing test exercising `themes.rs::detect_color_mode()` selection.

---

Use the 'darkmatter', 'biscuit-terminal', 'syntect', and 'two-face' skills.

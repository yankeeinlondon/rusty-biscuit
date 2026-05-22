---
prompt: |-
    Before we kickoff the Darkmatter implementation of the new [tree-rendering](@renderable/docs/tree-rendering.md)
    we need to define the API boundary before building it
    
    For example:
        
    ```rust
    Markdown::to_render_document(...)
    Markdown::render_tree_html(...)
    Markdown::render_tree_terminal(...)
    ```
    
    or keep it module-level/internal:

    ```rust
    darkmatter::markdown::render_tree::to_document(...)
    darkmatter::markdown::render_tree::render_terminal(...)
    ```
    
    Prefer internal/module-level experimental functions first, not new Markdown public methods, until parity is proven.
feature: '@renderable/features/2026-05-20-darkmatter-tree'
last_updated: 2026-05-20
---

## Darkmatter Tree-Rendering API Boundary

### Principle

Keep the new pipeline **internal and module-level** until parity is proven against the legacy renderers. No new `Markdown` public methods until the parity gate passes on the full corpus.

### Internal API Surface (`darkmatter::markdown::render_tree`)

The darkmatter side of the render-tree pipeline is a thin adapter layer: it folds a `Markdown` into a `renderable::tree::Document`, then delegates rendering to the target-specific crates (`renderable` for Browser/Markdown, `biscuit-terminal` for Terminal).

#### 1. Document construction

```rust
/// Folds a `Markdown` into a canonical `renderable::tree::Document`.
///
/// Frontmatter is **not** populated in this phase: darkmatter extracts
/// frontmatter before the parser sees content, and the fold does not enable
/// metadata blocks. The returned diagnostics are non-fatal; malformed input
/// never panics.
pub(crate) fn to_render_document(md: &Markdown) -> (Document, Vec<Diagnostic>);
```

This is a convenience wrapper around the existing `fold_markdown_to_document` that:

- Derives the `SourceDescriptor` from `md.source()` (file path when known, virtual otherwise).
- Passes `md.content()` (frontmatter already stripped) as the input text.
- In the next migration step, accepts or attaches Darkmatter's already
  extracted frontmatter so `DocumentMetadata::frontmatter` is populated without
  enabling pulldown-cmark metadata blocks.

#### 2. Target renderers (experimental, `pub(crate)`)

```rust
/// Renders a `Markdown` to HTML via the render-tree pipeline.
///
/// Maps `HtmlOptions` to `BrowserRenderOptions`, folds the document, and
/// delegates to `renderable::tree::render_browser_document`.
pub(crate) fn render_tree_html(
    md: &Markdown,
    options: &HtmlOptions,
) -> PipelineRenderResult<String>;

/// Renders a `Markdown` to a terminal string via the render-tree pipeline.
///
/// Maps `TerminalOptions` to `biscuit_terminal::render_tree::TerminalRenderOptions`,
/// folds the document, and delegates to `render_terminal_document`.
pub(crate) fn render_tree_terminal(
    md: &Markdown,
    options: &TerminalOptions,
) -> PipelineRenderResult<String>;

/// Renders a `Markdown` back to a Markdown string via the render-tree pipeline.
///
/// Delegates to `renderable::tree::render_markdown_document`. This is
/// primarily useful for round-trip / normalization testing.
pub(crate) fn render_tree_markdown(
    md: &Markdown,
) -> PipelineRenderResult<String>;
```

The concrete result alias comes from `diagnostic-model.md`:

```rust
pub(crate) type PipelineRenderResult<T> = Result<PipelineResult<T>, RenderError>;
```

The public legacy wrappers can later map this into `MarkdownResult<String>`;
the experimental entry points should preserve fold and render diagnostics
separately.

#### 3. Options mapping (private helpers)

The legacy options structs (`HtmlOptions`, `TerminalOptions`) carry darkmatter-specific concepts (code themes, mermaid modes, image renderers, horizontal-rule CSS variables). The tree renderers have their own options (`BrowserRenderOptions`, `TerminalRenderOptions`). The adapter layer owns the **one-way mapping**:

```rust
// Private helper in darkmatter::markdown::render_tree
fn browser_options_from_html_options(opts: &HtmlOptions) -> BrowserRenderOptions;

// Private helper in darkmatter::markdown::render_tree
fn terminal_options_from_terminal_options(
    opts: &TerminalOptions,
) -> TerminalRenderOptions;
```

These mappings are intentionally narrow for the experimental phase:

- **Code highlighting** — *closed (review-11 finding 2).* Both `render_tree_html` and `render_tree_terminal` wire darkmatter's `TerminalCodeRenderer` via the `CodeRenderer` hook. The hook now also receives the fenced info-string `meta` (the `CodeRenderer` trait grew a `meta` parameter), so it re-parses `title="…" line-numbering=true highlight=N` through `parse_code_info` and reproduces the legacy renderer's title block, line-number table/gutter, and highlighted-line markup on both the browser and terminal surfaces. On `ColorDepth::None` the terminal hook returns `None` so the plain (no-color) fallback runs, matching the legacy no-formatting contract (review-11 finding 1).
- **Mermaid** — `BrowserRenderOptions` has no mermaid mode. The experimental
  adapter should preserve legacy raw-HTML safety first (`RawHtmlPolicy::Escape`
  by default). Mermaid parity is a documented gap until a tree-level Mermaid
  lowering or a deliberate raw-HTML policy exists.
- **HR CSS variables** — `BrowserRenderOptions::page` does not yet support custom `:root` CSS injection. The adapter ignores `hr_css_variables` in the experimental phase; this is another documented parity gap.
- **Terminal images** — `TerminalRenderOptions` carries a `Terminal` context but no image renderer. The adapter ignores `image_mode` and `base_path` until `render_terminal_document` gains image support.

### Why module-level and not `Markdown` methods

The legacy renderers (`Markdown::as_html`, `Markdown::for_terminal`, and the
module-level `for_terminal`) are public, stable, and shipped. The tree pipeline
is **parallel and experimental**. Keeping the entry points as module-level
`pub(crate)` functions:

1. Prevents accidental public dependency on an unstable pipeline.
2. Lets the parity tests call both legacy and tree paths without method-name collision.
3. Keeps the `Markdown` impl block clean until the migration is complete.

The future public API is reserved but not implemented:

```rust
// RESERVED — do not add until parity is proven.
// impl Markdown {
//     pub fn to_render_document(&self) -> (Document, Vec<Diagnostic>);
//     pub fn render_tree_html(&self, options: HtmlOptions) -> MarkdownResult<String>;
//     pub fn render_tree_terminal(&self, options: TerminalOptions) -> MarkdownResult<String>;
// }
```

### Parity gate requirement

Before any of these `pub(crate)` functions are promoted to public `Markdown` methods, the existing `render_tree_parity.rs` integration test must be expanded to cover:

- All constructs in `tests/fixtures/render_tree/*.md`.
- The full darkmatter Phase 11 harness corpus (or a representative subset).
- Options permutations (light/dark themes, line numbering, style inclusion).

The gate is **semantic parity**, not byte-identical output. The classification from the existing parity test applies: *acceptable formatting difference*, *semantic mismatch*, *missing feature*, *bug in old renderer*, *bug in new renderer*.

### Deferred features (out of scope for the internal API)

These darkmatter-specific constructs are intentionally **not** folded in the experimental phase and therefore remain parity gaps:

- `==mark==` / dim inline styles until the span-aware processor chain lands.
- Horizontal rules with attribute blocks until the span-aware processor chain
  lands.
- `compose/` transformations (transclusion, interpolation, TOC linking) — these are string-level preprocessors and do not yet have tree-rewrite equivalents.

The internal API boundary does not attempt to solve these. It exposes only the Milestone 1 fold plus the target renderers, gated by parity.

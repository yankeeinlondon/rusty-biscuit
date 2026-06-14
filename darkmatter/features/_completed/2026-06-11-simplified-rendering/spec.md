---
created: 2026-06-11
reviewed: true
status: ready for planning and implementation
---

# Simplified Rendering Components

Darkmatter's rendering pipeline has accumulated too many public surfaces and
too much duplicated rendering logic. Code blocks, YAML snippets, Markdown pages,
browser rendering, terminal rendering, theme resolution, and render-tree adapter
hooks are currently spread across several APIs that overlap in responsibility.

This feature simplifies the public rendering model around two Darkmatter-owned
components:

- `CodeBlock` renders one syntax-highlighted code block.
- `DarkmatterPage` renders one Markdown document.

`CodeBlock` is the composable atomic rendering component and implements the
terminal and browser component traits. `DarkmatterPage` remains the page
assembler: it renders terminal output with `render(&Markdown)` and browser output
with `render_to_browser(&Markdown)`. It intentionally does **not** implement
`BrowserRenderable`, because that trait renders one already-owned component and
cannot receive the Markdown document a page assembler needs.

Reader note: this review preserves the existing `DarkmatterPage` boundary instead
of adding a synthetic `BrowserRenderable` implementation. That avoids storing
extra Markdown on the page just to satisfy a trait shape and keeps
`DarkmatterPage` aligned with the current render-tree cutover contract.
Lower-level render-tree hooks such as `TerminalCodeRenderer`, highlighter
internals, and theme-resolving helpers remain implementation details.

`DarkmatterPage` is not a new type — it already exists and renders Markdown
documents to both terminal and browser targets through the render tree. This
feature keeps extending that type in place rather than introducing a separate
`Page` component.

## Audience

This spec is for maintainers of Darkmatter, biscuit-terminal, and renderable.
It assumes the reader understands the render-tree migration, but it should also
be useful as a north star for future library callers: code is rendered with
`CodeBlock`, and Markdown documents are rendered with `DarkmatterPage`.

## Motivating Defect

The concrete defect that motivates this consolidation: in a **dark** terminal,
fenced code blocks fail to separate from the page — the panel inverts against
the wrong reference and syntax contrast is poor. A **light** terminal renders
correctly. The asymmetry is the tell.

Root cause is **two independent color-mode sources**, not a foreground/background
split (the panel's foreground and background always come from one
`CodeHighlighter`, so they agree by construction):

- The **page surface** mode is resolved from the real `Terminal::color_mode`
  (full OSC + macOS `AppleInterfaceStyle` detection).
- The **code panel** mode is resolved from `options.color_mode`, which the CLI
  fills from the env-only `detect_color_mode()` (`NO_COLOR` / `COLORFGBG`,
  default dark) — and `entrypoints.rs` rebuilds the renderer's `Terminal` from
  that option, discarding the real terminal mode.

When the two detectors agree (the common default) output is consistent, which is
why light mode looks correct. When they disagree, the panel inverts against the
wrong mode and the page's subtle surface fill shows around a mis-inverted panel.
The same divergence exists on the browser path (the panel-background stylesheet
and the inline highlighter spans resolve their mode independently).

This is structurally a duplication problem: code-block theme/mode is decided in
many places (four `ThemePair::for_*` wrappers, four `CodeHighlighter::for_*`
constructors — one of which inverts the mode twice — plus the renderer's own
mode plumbing and a separate CLI detector). Decision #4 (single source) and
Decision #8 (resolve once at the component boundary) eliminate the divergence by
construction; the simplification and the fix are the same change.

## Goals & Non-Goals

**Goals**

- Introduce `CodeBlock` as the single atomic renderer for syntax-highlighted
  code on terminal and browser targets.
- Extend `DarkmatterPage` to be the single public renderer for Markdown
  documents on terminal and browser targets. It already renders both targets
  through the render tree via `render(&Markdown)` and
  `render_to_browser(&Markdown)`; this feature keeps that public boundary while
  routing nested code panels through `CodeBlock`.
- Centralize `ThemePair -> Theme` resolution so production code resolves themes
  in one place per component.
- Collapse the duplicated code-block theme/mode resolution — today spread across
  four `ThemePair::for_*` wrappers, four `CodeHighlighter::for_*` constructors
  (with a redundant double inversion), the renderer's own mode plumbing, and a
  separate CLI `detect_color_mode()` source — into a single boundary resolver
  that produces a resolved `(Theme, ColorMode)` pair and hands it to
  `CodeHighlighter::from_theme(theme, mode)`. This is both the DRY win and the
  fix for the Motivating Defect.
- Make `Terminal` the source of truth for terminal color mode.
- Keep browser page rendering on the same `DarkmatterPage` color-mode policy as
  terminal rendering: captured terminal mode is preferred, `Unknown` falls back
  to the page's configured `ColorMode`, and that configured value defaults to
  dark.
- Replace `YamlBlock` behavior with `CodeBlock::yaml(...)`, then deprecate and
  remove `YamlBlock` as a separate public component.
- Keep arbitrary Markdown fence language strings working while adding typed
  convenience paths for common languages.
- Reduce public exposure of adapter plumbing such as `TerminalCodeRenderer`.
- Preserve current rendered output while the implementation is consolidated.
- Expand `md render` only where the current global render flags are incomplete;
  existing global layout flags remain the canonical CLI surface.
- Add `md code-block <file | content>` as the CLI surface for rendering one
  `CodeBlock`.

**Non-Goals**

- Moving Markdown parsing into biscuit-terminal. `DarkmatterPage` belongs in
  Darkmatter.
- Making biscuit-terminal understand Darkmatter's Markdown, schema, or compose
  semantics.
- Treating syntect's loaded grammar set as a stable exhaustive enum.
- Rewriting the renderable tree model.
- Removing `Markdown` parsing/composition/hash/schema APIs. This feature only
  simplifies the rendering API surface.
- Fully solving browser light/dark theme negotiation. Browser rendering keeps
  the existing configured-mode fallback with a dark default; richer browser mode
  detection can be a later feature.

## Foundational Decisions

- **Decision #1** - Darkmatter owns `CodeBlock` and `DarkmatterPage`.
  biscuit-terminal remains responsible for terminal detection and terminal
  rendering primitives, but it does not parse Markdown.
- **Decision #2** - `CodeBlock` is the only component that renders an atomic code
  panel. Markdown fences, schema examples, direct snippets, and `YamlBlock`
  compatibility all flow through the same code-block implementation.
- **Decision #3** - `DarkmatterPage` is the only component that renders a full
  Markdown document. It parses Markdown body content and delegates nested fenced
  code to the same `CodeBlock` path.
- **Decision #4** - `Terminal` is the source of truth for terminal color mode.
  Terminal rendering must not re-detect or infer color mode from lower-level
  options when a `Terminal` is available. The **same** `Terminal::color_mode`
  value must feed both the page surface and the nested code panel; the render
  path must not derive a second, independent mode (for example via the env-only
  `detect_color_mode()`) for the code panel. The Motivating Defect below is a
  direct violation of this rule.
- **Decision #5** - `DarkmatterPage` does not gain a separate
  `browser_color_mode` field. Browser rendering uses the same page color-mode
  policy as terminal rendering: a real captured terminal mode wins, and
  `ColorMode::Unknown` falls back to `with_color_mode(...)` /
  `TerminalOptions::color_mode` (default dark). This keeps page surface and
  nested code-panel mode resolution on one source instead of introducing a
  second browser-only source.
- **Decision #6** - `ColorMode::Unknown` falls back to the page's configured
  `ColorMode` before page/prose and code-block variants are resolved. With
  defaults, that means page/prose resolves dark and the default inverse code
  block resolves light.
- **Decision #7** - `ThemePair` is the user/config-facing theme choice. `Theme`
  is the resolved internal implementation detail passed to highlighters.
- **Decision #8** - Production rendering resolves `ThemePair -> Theme` at the
  component boundary. Code below `CodeBlock` and `DarkmatterPage` receives a
  resolved theme and must not independently choose page/code-block variants.
- **Decision #9** - Code blocks resolve their theme through `CodeBlockMode`.
  The default is `Inverse` (opposite the page mode), while existing explicit
  `dark`, `light`, and `same` modes remain supported. Page/prose rendering
  resolves against the page mode.
- **Decision #10** - `LanguageGrammar` is a typed convenience resolver, not an
  exhaustive list of syntect languages.
- **Decision #11** - `YamlBlock` is deprecated as soon as `CodeBlock` can replace
  its behavior. It is removed after callers migrate.
- **Decision #12** - Existing output parity is the migration guardrail. "Output
  parity" is defined concretely against the existing characterization suite
  (`cutover_reference.rs`, `layout_snapshots.rs`,
  `tree_features_characterization.rs`), used as a byte-exact baseline for
  string-level renderers. This aligns with the existing page.rs byte-for-byte
  contract that default-layout `DarkmatterPage` equals
  `Markdown::as_terminal(default)`. Refactors must keep terminal and browser
  output stable against that oracle unless a visible change is explicitly called
  out and accepted. Real-terminal (L2) captures are compared semantically, not
  byte-for-byte (see Testing Requirements). **Accepted visible change:** the
  dark-terminal code-block contrast fix (Motivating Defect) intentionally changes
  current dark-mode output. Parity must not freeze the buggy dark-mode panel;
  affected baselines (for example the `pronounced` browser snapshot) are
  re-captured as part of this feature, and the dark-mode fix is validated by the
  new cross-surface contrast test rather than held to the old bytes.
- **Decision #13** - CLI rendering commands use the same `DarkmatterPage` and
  `CodeBlock` components as library callers. The CLI must not retain parallel
  rendering implementations for page layout or code-block output.

## Public API Sketch

### `CodeBlock`

`CodeBlock` renders a single code block. It is independent of Markdown parsing.

```rust
pub struct CodeBlock {
    code: String,
    language: Option<LanguageGrammar>,
    meta: CodeBlockMeta,
    raw_meta: Option<String>,
    theme: Option<ThemePair>,
    layout: Layout,
}
```

`CodeBlock` stores **both** the parsed `CodeBlockMeta` (which drives rendering
decisions such as title, highlight ranges, and line numbering) and the raw fence
remainder text (`raw_meta`). The raw text is retained because the Markdown render
target re-emits the fence info string verbatim (`{lang} {raw_meta}`) and the TOC
tracks the full info string for change detection. Re-deriving the fence text from
the parsed struct alone would risk reordering keys or normalizing quoting and
whitespace, breaking byte-exact Markdown parity. When a `CodeBlock` is
constructed directly (not from a fence), `raw_meta` is `None` and the fence is
serialized from `meta`.

Expected construction surface:

```rust
impl CodeBlock {
    pub fn new(code: impl Into<String>) -> Self;
    pub fn with_language(mut self, language: LanguageGrammar) -> Self;
    pub fn with_fence_language(mut self, language: impl Into<String>) -> Self;
    pub fn with_meta(mut self, meta: CodeBlockMeta) -> Self;
    pub fn with_theme(mut self, theme: ThemePair) -> Self;

    pub fn yaml(code: impl Into<String>) -> Self;
    pub fn rust(code: impl Into<String>) -> Self;
    pub fn json(code: impl Into<String>) -> Self;
    pub fn toml(code: impl Into<String>) -> Self;

    pub fn from_source_file(path: impl AsRef<Path>) -> Result<Self, CodeBlockError>;
}
```

`CodeBlock` implements:

```rust
impl TerminalRenderable for CodeBlock;
impl BrowserRenderable for CodeBlock;
impl TreeRenderable for CodeBlock;
```

The `TreeRenderable` projection must be a plain `NodeKind::Code` subtree with
language and raw info-string metadata attached. It must not run syntax
highlighting during projection; terminal/browser target folds remain responsible
for rendering the code panel.

The terminal implementation resolves its theme like this:

```text
Terminal + CodeBlock.theme.or(DarkmatterPage.code_theme).or(env/default)
  -> page mode from Terminal
  -> CodeBlockMode::resolve(page_mode)
  -> ThemePair::resolve(resolved_code_mode)
  -> Theme
  -> CodeHighlighter
  -> terminal code block output
```

The browser implementation resolves its theme like this:

```text
page mode from DarkmatterPage policy + CodeBlock theme override
  -> CodeBlockMode::resolve(page_mode)
  -> ThemePair::resolve(resolved_code_mode)
  -> CodeHighlighter
  -> browser code block output
```

### `LanguageGrammar`

`LanguageGrammar` gives common languages a typed, tested path while still
allowing arbitrary syntect grammar lookup.

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
    OtherByExtension(String),
    OtherByName(String),
    OtherByToken(String),
}
```

Resolution is fallible:

```rust
impl LanguageGrammar {
    pub fn from_fence_token(token: impl AsRef<str>) -> Self;

    pub fn resolve<'a>(
        &self,
        syntax_set: &'a SyntaxSet,
    ) -> Result<&'a SyntaxReference, LanguageGrammarError>;
}
```

Common variants use canonical, tested lookup paths. `OtherByExtension`,
`OtherByName`, and `OtherByToken` preserve syntect's dynamic grammar model and
return a `Result` when a caller needs to know whether the grammar exists.

Markdown fence compatibility remains string-first. A fence language such as
`rust`, `yaml`, `sh`, `shell`, `tsx`, or a project-specific grammar token must
continue to work when syntect can resolve it.

#### Guaranteed aliases (first implementation)

`from_fence_token` resolves via syntect's native extension/name lookup first,
then falls back to an explicit alias map. The first implementation must preserve
the seven aliases the current resolver already special-cases
(`code_block.rs:357`) and add the four gaps the spec's compatibility examples
imply but the resolver does not yet handle:

| Fence token | Resolves to | Status |
|---|---|---|
| `shell`, `zsh` | `bash` | Existing |
| `c++` | `cpp` | Existing |
| `dockerfile` | `Dockerfile` | Existing |
| `makefile`, `make` | `Makefile` | Existing |
| `javascript` | `js` | Existing |
| `typescript` | `ts` | Existing |
| `python3` | `py` | Existing |
| `sh` | `bash` | New (gap fill) |
| `tsx` | TypeScript grammar | New (gap fill) |
| `python` | `py` | New (gap fill) |
| `yml` | `yaml` | New (gap fill) |

Tokens that syntect already resolves natively (for example `rs`, `py`, `js`,
`ts`, `yaml`, `md`, `json`, `toml`, `go`, `css`, `html`, `php`) need no explicit
alias and must continue to resolve through native lookup. A broader ecosystem
alias table can be a later, additive change.

### `DarkmatterPage`

`DarkmatterPage` renders a Markdown document. It already exists and renders both
terminal and browser targets through the render tree. This feature keeps its
existing constructor, per-edge layout builders, `render(&Markdown)`, and
`render_to_browser(&Markdown)` methods. It does **not** add
`BrowserRenderable`, `browser_color_mode`, or page-level `with_width`.

Existing construction surface (retained):

```rust
impl DarkmatterPage {
    pub fn new(term: &Terminal) -> Self;

    pub fn with_margin_top(mut self, cells: u16) -> Self;
    pub fn with_margin_bottom(mut self, cells: u16) -> Self;
    pub fn with_margin_left(mut self, cells: u16) -> Self;
    pub fn with_margin_right(mut self, cells: u16) -> Self;
    pub fn with_max_width(mut self, width: u16) -> Self;
    pub fn with_page_background(mut self, background: PageBackground) -> Self;
    pub fn with_page_color(mut self, color: PaintColor) -> Self;
    pub fn with_page_bg_color(mut self, color: PaintColor) -> Self;
    pub fn with_page_code_theme(mut self, theme: ThemePair) -> Self;
    pub fn with_code_block_mode(mut self, mode: CodeBlockMode) -> Self;
}
```

The existing `with_page_code_theme` override flows into the same nested-fence
`CodeBlock` path the page already drives via its page-code-theme resolution.
Note that the existing string-based `with_code_theme(impl Into<String>)` builder
is a separate `TerminalOptions::code_theme` pass-through and is not the
page-level override.

`DarkmatterPage` implements terminal component rendering for compatibility:

```rust
impl TerminalRenderable for DarkmatterPage;
```

Browser rendering remains the inherent page-assembler method:

```rust
impl DarkmatterPage {
    pub fn render_to_browser(&self, md: &Markdown) -> Result<String, PageRenderError>;
}
```

The terminal implementation resolves page theme from the `Terminal` captured at
construction. Fenced code blocks inside the page are rendered through the same
`CodeBlock` renderer and use `CodeBlockMode` against the terminal page mode.

The browser implementation uses the same resolved page mode as the page frame:
captured terminal mode when known, otherwise the configured `ColorMode`
(default dark). Fenced code blocks inside the page use `CodeBlockMode` against
that mode.

#### Terminal/browser asymmetry

`DarkmatterPage::new(&Terminal)` captures terminal context at construction, and
`render_to_browser` intentionally reuses that context for page-frame layout and
mode resolution. Constructing a page therefore always requires a `Terminal`,
even when only the browser target is ultimately rendered.

The browser page-frame layout already exists: `render_to_browser` wraps the
folded HTML when page-frame settings require margin, padding, max-width,
centering, background, meta, or stylesheet output. This feature should not
rebuild that wrapper; it should only ensure nested code panels flow through the
new `CodeBlock` boundary and share the page's resolved mode.

#### Page width decision

This feature does not add a page-level `with_width` builder or `--width` flag.
`DarkmatterPage` already has `with_max_width`, while exact widths exist at the
component-policy layer through `renderable::layout::Width::Fixed` and the CLI
`fill=explicit` grammar. Adding exact page width would create a second page-frame
sizing contract that needs separate terminal/browser semantics. Defer exact
page-frame width until there is a concrete use case that cannot be expressed by
`max-width` plus component explicit widths.

## CLI Scope

The CLI should become a thin command surface over `DarkmatterPage` and
`CodeBlock`.

### `md render`

The existing `md render` command renders a Markdown document and should map to
the public `DarkmatterPage` component. It must expose the page-level style
controls that a caller expects from the tree-renderer layout system.

The flags map directly (1:1) onto `DarkmatterPage`'s existing per-edge builders.
No opaque aggregate layout type is introduced; the authoritative layout lives on
`DarkmatterPage`'s existing builders.

Existing global render options that `md render` must continue to honor:

| Option | Alias | Builder | Meaning |
|---|---|---|---|
| `--mt <n>` | `--margin-top` (additive alias) | `with_margin_top` | Top page margin in terminal cells. |
| `--mb <n>` | `--margin-bottom` (additive alias) | `with_margin_bottom` | Bottom page margin in terminal cells. |
| `--ml <n>` | `--margin-left` (additive alias) | `with_margin_left` | Left page margin in terminal cells. |
| `--mr <n>` | `--margin-right` (additive alias) | `with_margin_right` | Right page margin in terminal cells. |
| `--max-width <n>` | | `with_max_width` | Maximum page content width. |
| `--page-bg <transparent\|subtle\|pronounced>` | `--page-background` | `with_page_background` | Page background style. `Pronounced` drives code-theme contrast. |
| `--page-bg-color <color>` | | `with_page_bg_color` | Free-form page background color. New CLI coverage for an existing builder. |

The background flag is split in two: `--page-background` takes the
`PageBackground` enum (`transparent` / `subtle` / `pronounced`), while
`--page-bg-color` takes a free-form `PaintColor`. `Pronounced` drives code-theme
contrast, so it is not interchangeable with a free color.

`--width <n>` is intentionally out of scope. Exact component width is already
available through `--fill-* explicit=<length>`; exact page-frame width is a
separate design.

These flags set `DarkmatterPage` layout/style options. They should not be
implemented as post-render string padding. Existing CLI layout helpers may be
reused, but the authoritative layout should live on the `DarkmatterPage`
component.

The current top-level implicit render path should continue to behave like
`md render` for compatibility.

### `md code-block`

Add a new command:

```text
md code-block <file | content>
```

The command renders one `CodeBlock` to the requested output target.

Initial expected options:

| Option | Meaning |
|---|---|
| `<file | content>` | A file path or literal code content. |
| `--language <token>` | Fence-style language token, resolved by `LanguageGrammar::from_fence_token`. |
| `--theme <theme>` | Code-block theme override. |
| `--title <text>` | Optional title/header label. |
| `--line-numbering` | Enable line numbers for this block. |
| `--highlight <range>` | Highlight one or more line ranges using the existing code-block metadata syntax. |
| `--output <terminal|html|markdown>` | Output target, matching existing render command conventions where possible. |

The command should use `CodeBlock` directly, not construct a synthetic Markdown
document containing a fence unless a compatibility path temporarily requires it.

Input disambiguation is an implementation detail, but the command must support
both file-backed and literal content use cases. If ambiguity cannot be resolved
reliably from filesystem existence alone, add explicit `--file` and `--content`
forms before broadening the command.

## Theme Resolution Policy

Theme resolution has one production rule:

```text
Render surface + ThemePair + optional override -> Theme
```

For terminal rendering, the render surface is `&Terminal`.

```text
Page/prose:
  ThemePair::for_page(&term, override)

Code block:
  term.color_mode()
  -> CodeBlockMode::resolve(page_mode)
  -> ThemePair::resolve(resolved_code_mode)
```

For browser rendering, the render surface is the page mode resolved by
`DarkmatterPage`. The default browser fallback mode is dark, but
`DarkmatterPage::render_to_browser` uses the captured terminal mode when it is
known.

```text
Page/prose:
  ThemePair::resolve(page_mode.known_or(configured_mode_or_Dark))

Code block:
  ThemePair::resolve(CodeBlockMode::resolve(page_mode.known_or(configured_mode_or_Dark)))
```

Lower layers must not repeat this policy. In particular:

- `CodeHighlighter` must not store `ThemePair`.
- `CodeHighlighter` must not decide whether something is page or code-block
  themed.
- `ProseHighlighter` must not decide theme variants.
- `TerminalCodeRenderer` must not be a public policy surface.
- Production terminal code must not call mode-only helpers when a `Terminal` is
  available.

Mode-only helpers may exist for browser rendering, tests, or compatibility
adapters, but they should be visibly secondary to the `Terminal` path.

## Highlighter Responsibilities

### `CodeHighlighter`

`CodeHighlighter` is an implementation detail for syntax-highlighted code.

It should own:

- the loaded `SyntaxSet`
- the resolved syntect `Theme`
- the resolved `ColorMode` used for line-highlight contrast calculations

It should not own:

- `ThemePair`
- terminal detection
- page/code-block policy
- environment-variable theme selection

Preferred construction shape:

```rust
impl CodeHighlighter {
    pub(crate) fn from_theme(theme: Theme, mode: ColorMode) -> Self;
}
```

Public callers should generally not need to construct `CodeHighlighter`
directly. They should use `CodeBlock` or `DarkmatterPage`.

### `ProseHighlighter`

`ProseHighlighter` remains the prose-style adapter around
`syntect::highlighting::Highlighter`. It receives a resolved syntect theme and
maps Markdown prose scopes such as heading, emphasis, link, and inline code to
styles.

`ProseHighlighter` should not resolve `ThemePair`.

## `YamlBlock` Deprecation

`YamlBlock` is replaced by `CodeBlock::yaml(...)`.

During migration:

```rust
impl TerminalRenderable for YamlBlock {
    fn render(&self, term: &Terminal) -> String {
        CodeBlock::yaml(self.yaml()).render(term)
    }
}
```

The browser implementation delegates the same way. Constructors that validate
YAML may remain temporarily, but rendering behavior must be delegated so fixes
land in one code path.

After callers migrate, `YamlBlock` is removed as a separate public component.

## Relationship To Existing Types

### `Markdown`

`Markdown` remains the parsing and document model. `DarkmatterPage` wraps
`Markdown` for rendering and should not replace parsing, composition, schema,
hashing, TOC, or reference APIs.

### `DarkmatterPage`

`DarkmatterPage` already owns page-frame rendering concerns and is the public
page component. It already exposes terminal and browser page rendering through
inherent methods, and this feature keeps that in-place API rather than
introducing a separate `Page` type, a compatibility alias, or a
`BrowserRenderable` shim. The public caller should not need to choose between
`Markdown::as_terminal`, `Markdown::as_html`, and a render-tree entry point for
page-framed output: `DarkmatterPage` is the one page-framed entry point for both
targets.

### `TerminalCodeRenderer`

`TerminalCodeRenderer` remains an adapter between renderable's generic
`CodeRenderer` hook and Darkmatter's `CodeBlock` implementation. It should not
be part of the public rendering API.

### `TreeComponent`

`biscuit-terminal::render_tree::TreeComponent` remains useful as a generic
adapter for arbitrary `TreeRenderable` values. It is not the public Darkmatter
Markdown rendering API because it cannot parse Markdown and should not own
Darkmatter theme/code-block policy.

## Migration Plan

### Phase 1 - Extract `CodeBlock`

- Add `CodeBlock` as a public Darkmatter component.
- Move shared code-block terminal and browser rendering behind `CodeBlock`.
- Add `LanguageGrammar` with typed common variants and dynamic fallback variants.
- Make `YamlBlock` delegate rendering to `CodeBlock::yaml(...)`.
- Preserve current `YamlBlock` terminal and browser parity tests.

### Phase 2 - Centralize Theme Resolution

- Make `ThemePair` the only user/config-facing theme choice.
- Keep `Theme` internal.
- Make `CodeBlock` and `DarkmatterPage` the only production locations that
  resolve `ThemePair -> Theme`.
- Remove or demote `for_page_mode()` / `for_code_block_mode()` from production
  terminal paths.
- Make highlighters accept resolved themes only.
- Fix the Motivating Defect's dual color-mode source: route the **same**
  `Terminal::color_mode` into both the page surface and the code panel, and stop
  using the env-only `detect_color_mode()` (and the
  `term.color_mode = opts.color_mode` rebuild) in the render path.
- Collapse the four `ThemePair::for_*` wrappers and the four
  `CodeHighlighter::for_*` constructors (eliminating the double inversion) into
  the single boundary resolver + `CodeHighlighter::from_theme(theme, mode)`.
- Re-baseline snapshots invalidated by the accepted dark-mode contrast fix (for
  example the `pronounced` browser snapshot).

### Phase 3 - Normalize `DarkmatterPage` browser/page integration

- Keep `DarkmatterPage::render_to_browser(&Markdown)` as the browser page
  boundary; do not add `BrowserRenderable`.
- Preserve the existing browser page-frame layout (margins, max-width,
  centering, background, meta, and stylesheet wrapper).
- Do not add `browser_color_mode`; use the existing captured-terminal /
  configured-`ColorMode` fallback policy.
- Do not add page-level `with_width`; keep `with_max_width` as the page-frame
  sizing API for this feature.
- Route fenced Markdown code blocks through `CodeBlock` for both targets.
- Keep `Markdown::as_terminal` and `Markdown::as_html` behavior; they continue
  to delegate appropriately (both already route through the render tree).
- Wire `md render` and the implicit top-level render path through
  `DarkmatterPage`.
- Map `md render` page layout/style flags 1:1 onto `DarkmatterPage` builders:
  `--margin-top` / `--mt`, `--margin-bottom` / `--mb`, `--margin-left` /
  `--ml`, `--margin-right` / `--mr`, `--max-width`, `--page-bg` /
  `--page-background` (the `PageBackground` enum), and `--page-bg-color`
  (free-form color).

### Phase 4 - Retire Legacy Public Rendering Surfaces

- Deprecate `YamlBlock`.
- Deprecate direct public use of `TerminalCodeRenderer`, if currently public.
- Reduce public rendering docs to `CodeBlock` and `DarkmatterPage`.
- Add `md code-block <file | content>` as the direct CLI entry point for
  `CodeBlock`.
- Remove `YamlBlock` after the migration window.

## Testing Requirements

- Output parity is checked against the existing characterization suite
  (`cutover_reference.rs`, `layout_snapshots.rs`,
  `tree_features_characterization.rs`) as a byte-exact baseline for string-level
  (terminal and browser) renderers. Default-layout `DarkmatterPage` must remain
  byte-for-byte equal to `Markdown::as_terminal(default)`.
- Add targeted golden cases for three gaps the existing suite under-covers:
  - YamlBlock terminal **and** browser output versus `CodeBlock::yaml`.
  - `ColorMode::Unknown`: page/prose falls back to the configured page mode
    (default dark), and the default inverse code block resolves light
    (Decision #6).
  - `CodeBlock`-direct output equals fenced-code-in-`DarkmatterPage` output for
    the same code, language, metadata, theme, and surface.
- `CodeBlock::yaml(...)` must match existing `YamlBlock` terminal and browser
  output during the compatibility phase.
- Markdown fenced code blocks must match direct `CodeBlock` output for the same
  code, language, metadata, theme, and surface.
- Terminal tests must verify that dark terminals use the inverse code-block
  theme and light terminals use the opposite inverse.
- A **cross-surface contrast** test must render a full `DarkmatterPage` and
  assert that the code-panel background luminance is well-separated from the
  page-surface luminance, in **both** light and dark modes. This is the
  assertion that catches the Motivating Defect; the existing in-isolation
  luminosity check (`assert_lighter`) and the `code_renderer` tests do not,
  because they feed one mode into both surfaces and never cross the page↔panel
  boundary (and the `code_renderer` tests accept `with_page_surface(...)` colors
  the renderer never reads). The test must drive a case where the real
  `Terminal` mode and any option-derived mode would disagree, to prove the
  single-source rule (Decision #4) holds.
- Browser tests must verify that the default browser fallback page mode is dark,
  a known captured terminal mode wins over that fallback, and code blocks resolve
  through `CodeBlockMode`.
- CLI tests for `md render` must verify the margin, max-width,
  `--page-background`, and `--page-bg-color` options are reflected through
  tree-renderer layout/style, not post-render string manipulation.
- CLI tests for `md code-block` must verify file input, literal content input,
  language selection, theme override, line numbering, and highlighted line
  ranges.
- Theme override tests must cover explicit caller overrides and `THEME`
  environment fallback behavior.
- `ColorMode::Unknown` tests must verify page/prose falls back to the configured
  page mode and the default inverse code block resolves to the opposite mode.
- `LanguageGrammar` tests must cover common variants, aliases, dynamic
  extension/name/token lookup, and unknown grammar errors.

L2 tests should be used only through the package `just test-l2` harness. Unit
tests should cover the pure resolution and render-string contracts wherever a
real terminal is not required.

L2 / real-terminal captures use **semantic** equality only — SGR colors, OSC8
links, and structure — never byte equality. Real-terminal captures collapse and
rewrite SGR sequences (for example, WezTerm), so byte-for-byte assertions
against L2 frames are invalid. Byte-exact parity belongs to the string-level
characterization oracle above, not to captured frames.

## Open Questions

- Should the new boundary resolver return `(Theme, ColorMode)` directly, or a
  small `ResolvedTheme` value that carries both fields? `CodeHighlighter::from_theme(theme, mode)`
  currently takes both separately, so the resolver must produce both regardless;
  this is an internal-shape question, not a behavioral one, and can be settled at
  implementation.

## Resolved Questions

- **`CodeBlock` metadata storage** — `CodeBlock` stores both the parsed
  `CodeBlockMeta` and the raw fence remainder text. See the `CodeBlock` section.
- **Guaranteed `from_fence_token` aliases** — the seven existing aliases are
  preserved and four gaps (`sh`, `tsx`, `python`, `yml`) are filled. See the
  `LanguageGrammar` section.
- **Browser dark/light auto-detection** — out of scope; browser rendering uses
  the `DarkmatterPage` page mode with a dark configured fallback when the
  captured terminal mode is unknown (see Non-Goals and Decision #5). Automatic
  browser preference detection is deferred to a later feature.
- **`md code-block` input disambiguation** — handled by the rule already stated
  in the CLI Scope section: resolve from filesystem existence when reliable,
  otherwise add explicit `--file` / `--content` forms before broadening the
  command.

## Success Criteria

- A caller can render a code block with `CodeBlock` without knowing about
  `CodeHighlighter`, `TerminalCodeRenderer`, or render-tree hooks.
- A caller can render a Markdown document with `DarkmatterPage` (terminal or
  browser) without dropping to render-tree entry points or `Markdown::as_*` for
  page-framed output.
- All code-block output flows through one implementation.
- Theme resolution for terminal rendering always starts from `Terminal`.
- Code blocks visibly separate from the page in **both** light and dark
  terminals — the Motivating Defect is fixed and guarded by the cross-surface
  contrast test, with a single `Terminal::color_mode` feeding page and panel.
- Browser rendering has a configured dark fallback when no known terminal mode
  is available.
- `md render` exposes page-level tree-renderer style controls through
  `DarkmatterPage`.
- `md code-block` exposes direct `CodeBlock` rendering from the CLI.
- `YamlBlock` is a thin delegating compatibility wrapper, then removable.

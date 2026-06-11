---
created: 2026-06-11
status: draft
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

Both components implement `TerminalRenderable` and `BrowserRenderable`. They are
the intended public rendering footprint for Darkmatter. Lower-level render-tree
hooks such as `TerminalCodeRenderer`, highlighter internals, and theme-resolving
helpers remain implementation details.

`DarkmatterPage` is not a new type — it already exists and renders Markdown
documents to the terminal through the render tree. This feature extends it to
cover the browser target as well, rather than introducing a separate `Page`
component.

## Audience

This spec is for maintainers of Darkmatter, biscuit-terminal, and renderable.
It assumes the reader understands the render-tree migration, but it should also
be useful as a north star for future library callers: code is rendered with
`CodeBlock`, and Markdown documents are rendered with `DarkmatterPage`.

## Goals & Non-Goals

**Goals**

- Introduce `CodeBlock` as the single atomic renderer for syntax-highlighted
  code on terminal and browser targets.
- Extend `DarkmatterPage` to be the single public renderer for Markdown
  documents on terminal and browser targets. It already renders the terminal
  target through the render tree; this feature wires the browser target through
  the tree too and adds `BrowserRenderable`.
- Centralize `ThemePair -> Theme` resolution so production code resolves themes
  in one place per component.
- Make `Terminal` the source of truth for terminal color mode.
- Make browser rendering use an explicit page color mode, defaulting to dark.
- Replace `YamlBlock` behavior with `CodeBlock::yaml(...)`, then deprecate and
  remove `YamlBlock` as a separate public component.
- Keep arbitrary Markdown fence language strings working while adding typed
  convenience paths for common languages.
- Reduce public exposure of adapter plumbing such as `TerminalCodeRenderer`.
- Preserve current rendered output while the implementation is consolidated.
- Expand `md render` so the CLI exposes the expected page-level tree-renderer
  style controls.
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
- Fully solving browser light/dark theme negotiation. Browser rendering gets an
  explicit mode option with a dark default; richer browser mode detection can be
  a later feature.

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
  options when a `Terminal` is available.
- **Decision #5** - Browser rendering uses explicit page color mode, defaulting
  to `ColorMode::Dark`.
- **Decision #6** - `ColorMode::Unknown` is treated as dark for page/prose
  resolution. Its inverse for code blocks is light.
- **Decision #7** - `ThemePair` is the user/config-facing theme choice. `Theme`
  is the resolved internal implementation detail passed to highlighters.
- **Decision #8** - Production rendering resolves `ThemePair -> Theme` at the
  component boundary. Code below `CodeBlock` and `DarkmatterPage` receives a
  resolved theme and must not independently choose page/code-block variants.
- **Decision #9** - Code blocks resolve their theme against the inverse of the
  page mode. Page/prose rendering resolves against the page mode.
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
  byte-for-byte (see Testing Requirements).
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

The terminal implementation resolves its theme like this:

```text
Terminal + CodeBlock.theme.or(DarkmatterPage.code_theme).or(env/default)
  -> ThemePair::for_code_block(&term, override)
  -> Theme
  -> CodeHighlighter
  -> terminal code block output
```

The browser implementation resolves its theme like this:

```text
Browser color mode (default Dark) + CodeBlock theme override
  -> inverse mode
  -> Theme
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

`DarkmatterPage` renders a Markdown document. It already exists and renders the
terminal target through the render tree; this feature extends it to the browser
target. It keeps its existing constructor and per-edge layout builders, gains a
`browser_color_mode` field and builder, gains a `with_width` builder to back an
explicit content width, and gains a `BrowserRenderable` implementation.

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
}
```

New construction surface (added by this feature):

```rust
impl DarkmatterPage {
    pub fn with_width(mut self, width: u16) -> Self;
    pub fn with_browser_color_mode(mut self, mode: ColorMode) -> Self;
}
```

`browser_color_mode` defaults to `ColorMode::Dark` (Decision #5). The existing
`with_page_code_theme` override flows into the same nested-fence `CodeBlock` path
the page already drives via its page-code-theme resolution. Note that the
existing string-based `with_code_theme(impl Into<String>)` builder is a separate
`TerminalOptions::code_theme` pass-through and is not the page-level override.

`DarkmatterPage` implements:

```rust
impl TerminalRenderable for DarkmatterPage;
impl BrowserRenderable for DarkmatterPage;
```

It renders both targets through the render tree.

The terminal implementation resolves page theme from the `Terminal` captured at
construction. Fenced code blocks inside the page are rendered through the same
`CodeBlock` renderer and use the inverse of the terminal page mode.

The browser implementation uses `browser_color_mode`, defaulting to dark. Fenced
code blocks inside the page use the inverse of that mode.

#### Terminal/browser asymmetry

`DarkmatterPage::new(&Terminal)` captures terminal context at construction, but
`BrowserRenderable` has no `Terminal`. As a design consequence, the same
`DarkmatterPage` is terminal-aware for the terminal target and
`browser_color_mode`-aware for the browser target: the browser render path reads
`browser_color_mode`, never terminal state. Constructing a page therefore always
requires a `Terminal`, even when only the browser target is ultimately rendered.

The browser page-frame layout (margins, max-width / width, centering, and
background applied to HTML output) is net-new work. Today only the terminal
page-frame exists, via `DarkmatterPage`'s `LayoutContext::from_page`; there is no
browser page-frame path anywhere yet. This is genuinely new behavior, not a
rename of an existing path.

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

Required options:

| Option | Alias | Builder | Meaning |
|---|---|---|---|
| `--margin-top <n>` | `--mt` | `with_margin_top` | Top page margin in terminal cells. |
| `--margin-bottom <n>` | `--mb` | `with_margin_bottom` | Bottom page margin in terminal cells. |
| `--margin-left <n>` | `--ml` | `with_margin_left` | Left page margin in terminal cells. |
| `--margin-right <n>` | `--mr` | `with_margin_right` | Right page margin in terminal cells. |
| `--max-width <n>` | | `with_max_width` | Maximum page content width. |
| `--width <n>` | | `with_width` (new) | Explicit page content width. |
| `--page-background <transparent\|subtle\|pronounced>` | | `with_page_background` | Page background style. `Pronounced` drives code-theme contrast. |
| `--page-bg-color <color>` | | `with_page_bg_color` | Free-form page background color. |

The background flag is split in two: `--page-background` takes the
`PageBackground` enum (`transparent` / `subtle` / `pronounced`), while
`--page-bg-color` takes a free-form `PaintColor`. `Pronounced` drives code-theme
contrast, so it is not interchangeable with a free color.

`--width <n>` is net-new: there is no `with_width` builder today (only
`with_max_width`), so Phase 3 must add a `with_width` builder to
`DarkmatterPage` to back this flag.

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
  ThemePair::for_code_block(&term, override)
```

For browser rendering, the render surface is an explicit browser color mode.
The default browser color mode is dark.

```text
Page/prose:
  ThemePair::resolve(browser_mode.known_or(Dark))

Code block:
  ThemePair::resolve(browser_mode.known_or(Dark).inverted())
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
page component. This feature extends it in place to cover the browser target
(adding `BrowserRenderable` and a net-new browser page-frame layout) rather than
introducing a separate `Page` type or a compatibility alias. The public caller
should not need to choose between `Markdown::as_terminal`,
`Markdown::as_html`, and a render-tree entry point for page-framed output:
`DarkmatterPage` is the one entry point for both targets.

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

### Phase 3 - Extend `DarkmatterPage` to browser + tree

- Add `BrowserRenderable` to `DarkmatterPage`; keep its existing
  `TerminalRenderable`.
- Wire the browser page-frame layout (margins, max-width / width, centering,
  background applied to HTML output). This is net-new: only the terminal
  page-frame exists today.
- Add a `browser_color_mode` field and `with_browser_color_mode` builder,
  defaulting to `ColorMode::Dark`.
- Add a `with_width` builder to back `--width` (only `with_max_width` exists
  today).
- Route fenced Markdown code blocks through `CodeBlock` for both targets.
- Keep `Markdown::as_terminal` and `Markdown::as_html` behavior; they continue
  to delegate appropriately (both already route through the render tree).
- Wire `md render` and the implicit top-level render path through
  `DarkmatterPage`.
- Map `md render` page layout/style flags 1:1 onto `DarkmatterPage` builders:
  `--margin-top` / `--mt`, `--margin-bottom` / `--mb`, `--margin-left` /
  `--ml`, `--margin-right` / `--mr`, `--max-width`, `--width` (new
  `with_width`), `--page-background` (the `PageBackground` enum), and
  `--page-bg-color` (free-form color).

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
  - `ColorMode::Unknown`: page/prose resolves dark, code-block resolves light
    (Decision #6).
  - `CodeBlock`-direct output equals fenced-code-in-`DarkmatterPage` output for
    the same code, language, metadata, theme, and surface.
- `CodeBlock::yaml(...)` must match existing `YamlBlock` terminal and browser
  output during the compatibility phase.
- Markdown fenced code blocks must match direct `CodeBlock` output for the same
  code, language, metadata, theme, and surface.
- Terminal tests must verify that dark terminals use the inverse code-block
  theme and light terminals use the opposite inverse.
- Browser tests must verify that the default browser page mode is dark and that
  code blocks resolve against the inverse mode.
- CLI tests for `md render` must verify the margin, width, max-width,
  `--page-background`, and `--page-bg-color` options are reflected through
  tree-renderer layout/style, not post-render string manipulation.
- CLI tests for `md code-block` must verify file input, literal content input,
  language selection, theme override, line numbering, and highlighted line
  ranges.
- Theme override tests must cover explicit caller overrides and `THEME`
  environment fallback behavior.
- `ColorMode::Unknown` tests must verify page/prose resolves as dark and
  code-block resolves as light.
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

- Should `ThemePair::for_page` and `ThemePair::for_code_block` return internal
  `Theme` directly, or should a small resolved theme value also carry the
  effective mode? (`CodeHighlighter::from_theme(theme, mode)` currently takes
  both separately, so the resolver must produce both regardless; this is an
  internal-shape question, not a behavioral one, and can be settled at
  implementation.)

## Resolved Questions

- **`CodeBlock` metadata storage** — `CodeBlock` stores both the parsed
  `CodeBlockMeta` and the raw fence remainder text. See the `CodeBlock` section.
- **Guaranteed `from_fence_token` aliases** — the seven existing aliases are
  preserved and four gaps (`sh`, `tsx`, `python`, `yml`) are filled. See the
  `LanguageGrammar` section.
- **Browser dark/light auto-detection** — out of scope; browser color mode is
  explicit with a dark default (see Non-Goals and Decision #5). Automatic
  preference detection is deferred to a later feature.
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
- Browser rendering has an explicit dark default.
- `md render` exposes page-level tree-renderer style controls through
  `DarkmatterPage`.
- `md code-block` exposes direct `CodeBlock` rendering from the CLI.
- `YamlBlock` is a thin delegating compatibility wrapper, then removable.

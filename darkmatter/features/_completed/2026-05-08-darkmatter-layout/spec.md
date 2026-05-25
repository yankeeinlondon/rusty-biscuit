---
title: Darkmatter Layout (DarkmatterPage)
status: draft
created: 2026-05-08
owner: Ken Snyder
package: darkmatter
---
# Darkmatter Layout

## Summary

Darkmatter today renders Markdown directly through its own `LineWrapper`, with no
page-level layout container. Margins, padding, page background, max-width, and
component alignment/fill are not exposed — the only `Layout` usage is inside the
horizontal-rule renderer (`darkmatter/lib/src/markdown/output/terminal.rs:2604+`).

This spec introduces a new top-level rendering primitive — **`DarkmatterPage`** —
that owns these layout concerns for terminal and browser output. It implements
biscuit-terminal's `Renderable` and `BrowserRenderable` traits so it composes with
the existing component ecosystem, but the implementation lives inside the
`darkmatter` crate because rendering Darkmatter documents is a Darkmatter concern.

## Goals

- Provide a single, ergonomic entry point (`DarkmatterPage`) that owns layout state.
- Expose margins, padding, page background, max-width, line numbers, alignment, and
  per-component fill via both the library builder API and the `md` CLI.
- Keep the existing `TerminalOptions` knobs (themes, image mode, mermaid mode,
  hyperlink mode, italic/dim modes, color depth, base path) usable through the new
  page primitive — `DarkmatterPage` orchestrates them rather than replacing them.
- Match biscuit-terminal's CLI conventions for margin flags (`--margin`, `--mx`,
  `--my`, `--mt`, `--mb`, `--ml`, `--mr`).
- Render to terminal today; render to browser via `BrowserRenderable` with a
  near-1:1 mapping of `ch` units (refinements deferred).

## Non-Goals

- **Page-level alignment** of the document's main content stream is out of scope;
  alignment in this spec applies only to *page components* (images, block quotes,
  tables, code blocks, lists).
- HTML/CSS unit fidelity beyond a 1:1 `ch` mapping (deferred).
- Replacing `TerminalOptions` — `DarkmatterPage` wraps and extends, it does not
  replace.
- Reflowing document body width to match component fill (the body still uses the
  available content width; fill applies to partial-width components).

## Public API Surface

### `DarkmatterPage`

```rust
use biscuit_terminal::Terminal;
use darkmatter::markdown::Markdown;
use darkmatter::layout::{
    DarkmatterPage,
    PageBackground,
    PageComponent,
    PageAlignment,
    PageFill,
    WidthUnit,
};

let page = DarkmatterPage::new(&terminal)
    .with_margin(1)                       // shorthand: all four sides
    .with_margin_x(2)                     // left + right
    .with_margin_y(1)                     // top + bottom
    .with_margin_top(1)
    .with_margin_bottom(1)
    .with_margin_left(2)
    .with_margin_right(2)
    .with_padding(2)
    .with_padding_x(2)
    .with_padding_y(1)
    .with_padding_top(1)
    .with_padding_bottom(1)
    .with_padding_left(2)
    .with_padding_right(2)
    .with_page_background(PageBackground::Subtle)
    .with_max_width(100)
    .use_line_numbers()
    .use_alignment(PageComponent::Images, PageAlignment::Center)
    .with_fill(PageComponent::CodeBlocks, PageFill::Pad(WidthUnit::Fixed(2)))
    .render(&markdown)?; // -> Result<String, PageRenderError>
```

`DarkmatterPage` is constructed against a `&Terminal` so it can capture terminal
width, color mode, and capability information up front. The builder is consuming
(`self -> Self`) for ergonomic chaining.

`DarkmatterPage` is an owned type. The constructor `DarkmatterPage::new(&Terminal)`
captures the terminal's width, color mode, and capability flags by value at
construction; the page does not borrow the `Terminal`.

#### Return Type and Errors

`DarkmatterPage::render(&self, &Markdown)` returns `Result<String, PageRenderError>`.
`PageRenderError` is a new `thiserror`-derived enum owned by darkmatter and
defined in the `darkmatter::layout` module. It includes (at minimum) the
following variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PageRenderError {
    /// The combined horizontal margin + padding meets or exceeds the terminal
    /// width, leaving no room for content.
    #[error("margins ({required} cols) meet or exceed terminal width ({terminal_width} cols)")]
    MarginsExceedTerminalWidth { terminal_width: u16, required: u16 },

    /// A caller set `max_width = Some(0)` via the library API. (The CLI
    /// already rejects `--max-width 0` at parse time.)
    #[error("max_width must be greater than zero")]
    MaxWidthZero,

    /// A `WidthUnit::Percent` value is outside the valid `0.0..=100.0` range.
    #[error("invalid percent value: {0} (expected 0.0..=100.0)")]
    InvalidPercent(f32),

    /// An underlying markdown render failure.
    #[error("markdown render failed: {0}")]
    Render(String),
}
```

The `Renderable` and `BrowserRenderable` trait impls wrap `PageRenderError` into
each trait's associated `Error` type (typically via a `From` conversion or by
mapping into the trait's error variant).

#### Terminal Options Integration

`DarkmatterPage` exposes first-class builder methods for the most commonly
configured `TerminalOptions` knobs, so callers do not need to construct a
`TerminalOptions` value to tune them:

- `with_image_mode(ImageMode)`
- `with_mermaid_mode(MermaidMode)`
- `with_hyperlink_mode(HyperlinkMode)`
- `with_italic_mode(ItalicMode)`
- `with_dim_mode(DimMode)`
- `with_color_depth(ColorDepth)`
- `with_color_mode(ColorMode)` — note that `with_page_background(Pronounced)`
  overrides this (see `### CLI Conflict Resolution`)
- `with_code_theme(impl Into<String>)`
- `with_prose_theme(impl Into<String>)`
- `with_base_path(impl Into<PathBuf>)`

Escape hatch: `with_terminal_options(TerminalOptions) -> Self` replaces the
entire underlying `TerminalOptions` in one call. First-class builders called
*after* this method override individual fields on the replaced options.

CLI mapping (use existing `md` flag names where they exist; new flags are
marked TBD):

| Builder | CLI flag |
|---------|----------|
| `with_image_mode` | TBD |
| `with_mermaid_mode` | TBD |
| `with_hyperlink_mode` | TBD |
| `with_italic_mode` | TBD |
| `with_dim_mode` | TBD |
| `with_color_depth` | TBD |
| `with_color_mode` | TBD |
| `with_code_theme` | TBD |
| `with_prose_theme` | TBD |
| `with_base_path` | TBD |

### Configuration Options

#### Margin

Margins are transparent space outside the page's content rectangle. They are
specified in terminal characters (rows for vertical, columns for horizontal).

| Library API | CLI Flag | Effect |
|-------------|----------|--------|
| `with_margin(n)` | `--margin <n>` / `-m <n>` | All four sides |
| `with_margin_x(n)` | `--mx <n>` | Left + right |
| `with_margin_y(n)` | `--my <n>` | Top + bottom |
| `with_margin_top(n)` | `--mt <n>` | Top only |
| `with_margin_bottom(n)` | `--mb <n>` | Bottom only |
| `with_margin_left(n)` | `--ml <n>` | Left only |
| `with_margin_right(n)` | `--mr <n>` | Right only |

CLI precedence: more-specific flags win. Order from least to most specific:
`-m / --margin` → `--mx / --my` → `--mt / --mb / --ml / --mr`.

```rust
pub struct PageMargin {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}
```

Default: `PageMargin::ZERO` (preserves current behavior).

#### Page Background

```rust
pub enum PageBackground {
    /// Default. Margin and padding are visually identical (both transparent).
    Transparent,
    /// Slightly off-background fill — darker than terminal bg in light mode,
    /// lighter than terminal bg in dark mode.
    Subtle,
    /// High-contrast inverse fill — near-black in light mode, near-white in
    /// dark mode. **Inverts the renderer's effective color mode** so themes
    /// remain readable on the inverted surface.
    Pronounced,
}
```

- `Subtle` resolves at render time using the terminal's detected color mode
  (`ColorMode::Light` / `ColorMode::Dark`).
- `Pronounced` produces a deliberately strong contrast surface and **flips** the
  `color_mode` field passed down into syntax highlighting and prose theme
  selection. Practical example: terminal is dark, `page_background = Pronounced`
  ⇒ Darkmatter renders as if `ColorMode::Light`, picks a light theme, and paints
  the page surface near-white.

CLI: `--page-bg <transparent|subtle|pronounced>` (alias `--page-background`).

Specific RGB values for `Subtle` and `Pronounced` are deferred to implementation
but should be expressed as named constants (e.g. `PAGE_BG_SUBTLE_DARK`,
`PAGE_BG_PRONOUNCED_DARK`, etc.) so they can be tuned without touching
rendering code.

#### Padding

Padding sits inside the margin and outside the content. Unlike margin, padded
cells are filled with the page's background color (when not transparent), so:

- `PageBackground::Transparent` ⇒ padding is visually indistinguishable from
  margin; effective whitespace = `margin + padding`.
- `PageBackground::Subtle` / `Pronounced` ⇒ padding is filled with the page
  background; the page background also fills the area between padding edges and
  content (text retains its own bg color overrides).

```rust
pub struct PagePadding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}
```

CLI: `--padding`, `--px`, `--py`, `--pt`, `--pb`, `--pl`, `--pr` (mirrors margin).

#### Max Width

```rust
pub fn with_max_width(self, max_width: u16) -> Self;
```

CLI: `--max-width <cols>`.

Resolution rule:

```
content_width = terminal_width - margin.left - margin.right - padding.left - padding.right
effective_width = min(content_width, max_width)
```

When `effective_width < content_width`, **margins/padding are still honored** but
the page reports `effective_width` to its content. The remaining horizontal space
is absorbed as additional left margin (or distributed per page alignment, when
that is added in the future). Default behavior in this draft: surplus space goes
to the right (i.e., page hugs the left).

Default: `None` (no cap; current behavior).

#### Line Numbers

```rust
pub fn use_line_numbers(self) -> Self;        // sets to true
pub fn with_line_numbers(self, on: bool) -> Self;
```

CLI: `--line-numbers <true|false>` (defaults to `false`).

Equivalent to `TerminalOptions::include_line_numbers`. `DarkmatterPage` writes
this through to the underlying `TerminalOptions` when delegating to the document
renderer.

#### Alignment

Applies only to *page components* — partial-width elements that do not consume
the full content width.

```rust
pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    Lists,
}

pub enum PageAlignment {
    Left,
    Center,
    Right,
}
```

API:

```rust
pub fn use_alignment(self, component: PageComponent, alignment: PageAlignment) -> Self;
```

CLI:

| Flag | Effect |
|------|--------|
| `--alignment <left\|center\|right>` | Sets alignment for *all* `PageComponent` variants |
| `--align-images <…>` | Images only |
| `--align-lists <…>` | Lists only |
| `--align-block-quotes <…>` | Block quotes only |
| `--align-tables <…>` | Tables only |
| `--align-code-blocks <…>` | Code blocks only |

Per-component flags override `--alignment`. Default: `PageAlignment::Left`.

> Page-level alignment of the main document stream is **out of scope** for this
> spec.

#### Fill

Specifies how a page component consumes available width.

```rust
pub enum WidthUnit {
    // u16 matches PageMargin / PagePadding / with_max_width — negatives impossible at the type level.
    Fixed(u16),
    Percent(f32), // 0.0..=100.0
}

pub enum PageFill {
    /// Default. Component may use the full content width.
    Full,
    /// Symmetric padding on left + right (filled with page background).
    Pad(WidthUnit),
    /// One-sided padding driven by the component's alignment.
    /// - Left alignment ⇒ padding on the left only
    /// - Right alignment ⇒ padding on the right only
    /// - Center alignment ⇒ behaves like `Pad`
    Indent(WidthUnit),
    /// Cap on the component's render width; the component renders at
    /// `min(natural_width, max)`. Useful to prevent unbounded growth.
    Max(WidthUnit),
    /// Explicit render width. The component is told to render at the resolved
    /// width directly.
    /// - `Percent` values must be `<= 100.0`; resolved against content width
    /// - `Fixed` values are used as-is, capped to content width when larger
    Explicit(WidthUnit),
}
```

API:

```rust
pub fn with_fill(self, component: PageComponent, fill: PageFill) -> Self;
```

CLI:

- `--fill <variant>` to set all page component types
- `--fill-{kind} <variant>` to set a specific page component's strategy

CLI value grammar:

- The flag value follows the form `<kind>=<value>` for variants that take a
  `WidthUnit`, or the bare token `full` for `PageFill::Full`.
- `<kind>` is one of: `pad`, `indent`, `max`, `explicit`, `full` (with `full`
  taking no `=value`).
- `<value>` is either a non-negative integer (resolves to `WidthUnit::Fixed(n)`)
  or a non-negative integer followed by `%` (resolves to
  `WidthUnit::Percent(n.0)`, must be `<= 100`).

Examples:

```
--fill-code-blocks pad=2          # PageFill::Pad(WidthUnit::Fixed(2))
--fill-code-blocks max=50%        # PageFill::Max(WidthUnit::Percent(50.0))
--fill-images indent=10%          # PageFill::Indent(WidthUnit::Percent(10.0))
--fill-tables explicit=80         # PageFill::Explicit(WidthUnit::Fixed(80))
--fill full                       # PageFill::Full (applies to all components)
```

Malformed values (unknown kind, negative number, percent > 100) are rejected at
CLI parse time and return a clap parse error; the library API surfaces these via
`PageRenderError::InvalidPercent` (existing) when invoked directly.

Resolution semantics:

- `WidthUnit::Percent(p)` resolves against the page's *content width* (after
  margin and padding have been deducted, before max-width cap), then is itself
  capped by the post-max-width effective width.
- `WidthUnit::Fixed(n)` resolves to `min(n, effective_width)` (using the unsigned column count).
- `Pad` / `Indent` reduce the component's available width by the resolved amount
  (split symmetrically for `Pad`, asymmetrically for `Indent`); the reclaimed
  cells are filled with the page background.
- `Max` and `Explicit` produce a target render width; the component is
  responsible for honoring it.

Default: `PageFill::Full` for every `PageComponent`.

### Trait Implementations

```rust
impl biscuit_terminal::renderable::Renderable for DarkmatterPage { /* … */ }
impl biscuit_terminal::renderable::BrowserRenderable for DarkmatterPage { /* … */ }
```

Browser rendering uses CSS:

- `margin: <top>ch <right>ch <bottom>ch <left>ch;`
- `padding: <top>ch <right>ch <bottom>ch <left>ch;`
- `max-width: <max>ch;`
- `background-color: <subtle|pronounced resolved color>;`
- Per-component alignment via `text-align` or `margin: 0 auto;` on the
  component's wrapper element, depending on the component.
- `PageFill::Pad` / `Indent` translate to symmetric / asymmetric horizontal
  padding on the component wrapper; `Max` / `Explicit` translate to `max-width` /
  `width` on the wrapper.

A near-1:1 `ch`-unit mapping is the v1 target. A future refinement may translate
character widths into `em`/`rem` based on detected font metrics.

## CLI Specification

All flags below attach to the existing `md` render path (the default subcommand
that consumes a Markdown file and emits a terminal rendering). Flags are
additive; defaults preserve current behavior.

```
md doc.md
   [-m <n> | --margin <n>]
   [--mx <n>] [--my <n>]
   [--mt <n>] [--mb <n>] [--ml <n>] [--mr <n>]
   [--padding <n>]
   [--px <n>] [--py <n>]
   [--pt <n>] [--pb <n>] [--pl <n>] [--pr <n>]
   [--page-bg <transparent|subtle|pronounced>]
   [--max-width <cols>]
   [--line-numbers <true|false>]
   [--alignment <left|center|right>]
   [--align-images <left|center|right>]
   [--align-lists <left|center|right>]
   [--align-block-quotes <left|center|right>]
   [--align-tables <left|center|right>]
   [--align-code-blocks <left|center|right>]
```

Negative numbers are rejected at parse time. `--max-width 0` is rejected
(use omission instead).

### CLI Conflict Resolution

- Margin / padding shorthands compose with specific overrides; specific wins.
  `-m 2 --mt 0` ⇒ `top=0, right=2, bottom=2, left=2`.
- `--alignment` sets a default for every component; `--align-<component>`
  overrides only that one.
- `--page-bg pronounced` overrides any caller-supplied `color_mode` by inverting
  it for the document render.

## Implementation Considerations

### Rendering Flow Changes

Today: `for_terminal(&Markdown, TerminalOptions) -> String`. The function builds
a `LineWrapper` at `terminal_width` (or `max_width`) and writes directly.

Proposed:

1. `DarkmatterPage::new(&Terminal)` captures terminal context.
2. `.render(&Markdown)` (or `Renderable::render`) computes:
   - `available_cols = terminal.width()`
   - `content_cols = available_cols - margin_x - padding_x`
   - `effective_cols = min(content_cols, max_width.unwrap_or(content_cols))`
3. Internally calls `for_terminal` with a derived `TerminalOptions` whose
   `max_width = Some(effective_cols)` and `include_line_numbers` set from the
   page builder.
4. Wraps the resulting line-output with:
   - Top margin: `margin.top` empty rows (transparent).
   - Top padding: `padding.top` rows filled with page background.
   - Per content row: prepend `margin.left` transparent cells, then `padding.left`
     bg-filled cells; append `padding.right` bg-filled cells, then `margin.right`
     transparent cells. Row interior is bg-filled to `effective_cols` (when bg
     is not transparent) so explicit text-bg overrides remain visible.
   - Bottom padding / margin mirror the top.
5. Page-component alignment / fill are applied **inside** the document rendering
   pipeline — the renderer must learn to consult `DarkmatterPage` settings when
   emitting images, block quotes, tables, code blocks, and lists. Concretely
   this means threading a layout-context object into the existing dispatcher in
   `terminal.rs`, alongside `TerminalOptions`.

The document body itself remains left-aligned within its content rectangle in
this iteration; only the listed page components reposition / re-fill.

### Touch Points (initial estimate)

- `darkmatter/lib/src/markdown/output/terminal.rs` — add layout context
  threading; component dispatch for alignment + fill; row-decoration wrapper.
- `darkmatter/lib/src/markdown/output/code_block.rs` — honor effective width and
  fill for code blocks; preserve existing padding-row behavior.
- `darkmatter/lib/src/markdown/output/html.rs` — emit page-level wrapper with
  margin / padding / max-width / bg styles for `BrowserRenderable`.
- New module: `darkmatter/lib/src/layout/` containing
  `DarkmatterPage`, `PageMargin`, `PagePadding`, `PageBackground`, `PageComponent`,
  `PageAlignment`, `PageFill`, `WidthUnit`.
- `darkmatter/cli/src/args.rs` — new flags with conflict resolution.
- `darkmatter/cli/src/output.rs` (and/or render command module) — translate CLI
  args into a `DarkmatterPage` and call `.render(&md)?`.

### Interaction with Existing Layout Use

The existing horizontal-rule path already calls
`Layout::resolve_margin`. `DarkmatterPage` should not interfere with per-rule
frontmatter margins; rule margins remain in addition to page padding/margin
(rules render inside the page's content rectangle).

### Color Mode Inversion (Pronounced)

When `PageBackground::Pronounced`:

- Compute the inverted mode: `Light <-> Dark`.
- Set `TerminalOptions::color_mode` to the inverted value.
- Re-detect (or re-select) `code_theme` and `prose_theme` against the inverted
  mode so syntax highlighting reads on the new surface.
- Page background fill uses the contrast color appropriate for the *original*
  terminal mode (i.e. near-white in dark terminal, near-black in light
  terminal).

### Defaults Summary

| Field | Default |
|-------|---------|
| `margin` | `(0, 0, 0, 0)` |
| `padding` | `(0, 0, 0, 0)` |
| `page_background` | `Transparent` |
| `max_width` | `None` |
| `line_numbers` | `false` |
| `alignment` (per component) | `Left` |
| `fill` (per component) | `Full` |

These defaults reproduce today's render exactly when no flags are supplied.

## Acceptance Criteria

- **Zero-config equivalence:** invoking `DarkmatterPage::new(&terminal).render(&md)?`
  with no other builder calls produces output that matches
  `for_terminal(&md, TerminalOptions::default())` byte-for-byte for a
  representative fixture set.
- **Error paths:** every `PageRenderError` variant is reachable from a public
  API call. `MarginsExceedTerminalWidth` fires when
  `margin_x + padding_x >= terminal_width`. `MaxWidthZero` fires for
  `with_max_width(0)`. `InvalidPercent` fires for percent values outside
  `0.0..=100.0`.
- **CLI precedence:** `-m 2 --mt 0` resolves to
  `top=0, right=2, bottom=2, left=2`. `--alignment center --align-images left`
  resolves to images-left, all-others-center.
- **Pronounced inversion:** `--page-bg pronounced` on a dark terminal produces
  a render whose detected/effective `color_mode` is `Light` and whose page
  surface is the pronounced contrast color.
- **End-to-End example:** the worked example at the bottom of the spec
  produces a render whose dimensions match the bullet-list description
  (2 transparent rows top, 1 subtle-bg row, 100-col content, 1 subtle-bg row,
  2 transparent rows bottom; per-row column layout as described).

## Test Plan

- **Snapshot tests** for the End-to-End example and a small fixture set in
  `darkmatter/lib/tests/snapshots/`.
- **Unit tests** in `darkmatter/lib/src/layout/` covering each
  `PageRenderError` variant.
- **CLI integration tests** via `assert_cmd` + `predicates` in
  `darkmatter/cli/tests/`, covering CLI precedence rules and `--fill` grammar
  parsing.
- **Browser HTML golden tests** for `BrowserRenderable` output (margin /
  padding / max-width / bg styles emitted as expected).
- **Regression test** asserting zero-config equivalence with the pre-existing
  `for_terminal` path.

## Open Questions

1. **Surplus distribution under `max_width`:** when content is capped, where
   does the surplus go?

    DECISION: v1 always sends surplus to the right (the page hugs the left
    edge of the content rectangle). Configurable surplus distribution is
    deferred to the future page-alignment feature, which will own this knob
    alongside main-stream alignment.

2. **`Pad` vs `Indent` on transparent page background:** when bg is transparent,
   `Pad` and `Indent` still reduce component width but the "filled" cells are
   transparent. Confirm this matches expectations (effectively margin-on-
   component) — or should fill be a no-op when bg is transparent?

    DECISION: still reduces component width.

3. **Text-bg overrides inside padded regions:** confirmed that explicit text
   background colors continue to work; padding only paints cells that have no
   character-level bg already.

    DECISION: correct, all rendering is the same but when a text block paints an explicit background color instead of returning to NO background color we will return to the page's background.

4. **`Subtle` / `Pronounced` exact RGB values:** defer to implementation, but
   pick named constants up front so they can be tuned independently.

    DECISION: defer to implementation, agreed that named constants need to be set

5. **Browser unit mapping:** v1 ships `1ch` per terminal column. Worth
   reconsidering for code blocks where monospace `ch` is reliable but for prose
   where proportional fonts may be desired in the future.

    DECISION: v1 ships `1ch` per terminal column for all components,
    including prose. Differentiated unit mapping (e.g. `rem`/`em` for prose
    containers) is deferred until browser rendering has concrete demand.



## Out of Scope (Explicit)

- Page-level alignment for the main document stream.
- Reflowing the body to match component fill widths. Concretely: if
  `--fill-code-blocks max=60` is set on a 100-col page, prose paragraphs
  still wrap at 100, not 60. The body's wrap width is always
  `effective_cols`, independent of any per-component `PageFill` setting.
- HTML/CSS unit fidelity beyond `ch`.
- A general-purpose `Layout` re-export from biscuit-terminal — `DarkmatterPage`
  may *use* `biscuit_terminal::utils::layout` internally, but the public surface
  is darkmatter-owned types.

## Example: End-to-End

Terminal is dark mode, 120 cols wide. User runs:

```bash
md doc.md \
  --margin 2 \
  --padding 1 \
  --page-bg subtle \
  --max-width 100 \
  --line-numbers true \
  --align-code-blocks center
```

Render:

- `available_cols = 120`
- `content_cols = 120 - 2*2 - 1*2 = 114`
- `effective_cols = min(114, 100) = 100`
- 2 transparent rows top, 1 subtle-bg row, 100-column content area, 1 subtle-bg
  row, 2 transparent rows bottom.
- Each content row: `[2 transparent][1 subtle-bg][100 cols of content, subtle bg
  except where text overrides][1 subtle-bg][2 transparent][14 transparent
  surplus from max-width]`.
- Code blocks render centered within the 100-col content area.
- Line numbers appear inside code blocks.
- All other components render left-aligned, full-fill, as today.

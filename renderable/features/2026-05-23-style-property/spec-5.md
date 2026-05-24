---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 5-of-7
depends-on: spec-1.md (sub-spec #1), spec-2.md (sub-spec #2), spec-3.md (sub-spec #3), spec-4.md (sub-spec #4)
reviewed: true
---

# `style:` Frontmatter — Sub-Spec #5: Color & Background-Color Mutations

## Problem

Sub-spec #1 parses `color` and `bg-color` into `StyleColor {
color: renderable::color::Color, opacity: Option<u8> }`, but the active
rendering phases still treat those fields as parsed-but-inactive. This means a
schema-valid document can request `style.page.color`, `style.table.bg-color`,
or `style.ul.color` and receive no terminal or browser styling.

This sub-spec activates those already-parsed fields for every component that
exists after sub-spec #4:

- `PageComponent::Images`
- `PageComponent::BlockQuotes`
- `PageComponent::Tables`
- `PageComponent::CodeBlocks`
- `PageComponent::Ul`
- `PageComponent::Ol`
- `PageComponent::Li`

It also adds `PageComponent::Hyperlinks` because the accepted schema catalog
already marks `style.hyperlinks.color` and `style.hyperlinks.bg-color` as
sub-spec #5 fields. Hyperlink-local style (`style.hyperlinks.local-style.*`)
remains sub-spec #7.

HR color remains pending until sub-spec #6 adds `PageComponent::Hr` and moves
the HR style surface under `style.hr.*`.

## Goals

- Apply `style.page.color` and `style.page.bg-color` as inherited defaults for
  every page component that does not define its own color slot.
- Apply component-level `color` and `bg-color` for table, images, block-quote,
  hyperlinks, ul, ol, and li.
- Apply page-level inherited color/bg-color to code blocks. The v1 schema does
  not contain a `style.code-blocks.*` bucket, so code blocks have no
  component-specific frontmatter color in this sub-spec.
- Terminal: emit foreground/background SGR sequences for colors that resolve to
  RGB via `renderable::color::Color::to_rgb()`, and reset at component
  boundaries so color never leaks into later content or the user's shell.
- Browser/HTML: emit per-component CSS through the existing
  `wrap_browser_html` / `build_component_css` path, preserving opacity with
  `rgba(...)` when `StyleColor.opacity` is present.
- Preserve the existing CLI precedence model: CLI layout flags still win for
  layout fields; color has no CLI override in this sub-spec, so frontmatter is
  the only color source.
- Advance active style wiring to sub-spec `5` so wired color keys no longer
  emit `KnownButInactive { sub_spec: 5 }`.

## Non-Goals

- No new color type and no fork of the color parser; use `StyleColor` from
  `darkmatter::style::color`.
- No alpha blending in terminal. Terminal rendering ignores `opacity`.
- No syntax-highlighting foreground override. `style.page.color` must not
  replace token foreground colors selected by `code_theme`.
- No `Color::Reset` parser work. The v1 style parser does not accept the
  string `"reset"`, so explicit component-level inheritance clearing is out of
  scope for this sub-spec.
- No graph-level style propagation between composed parent and child
  documents. This sub-spec only applies the style block already present on the
  `Markdown` value being rendered.
- No HR color routing until sub-spec #6.
- No `style.code-blocks.*` schema addition. Code-block-specific knobs remain
  under `style.page.code.*` for sub-spec #7 unless a later schema revision adds
  a dedicated code-block bucket.

## Dependencies

- Sub-spec #1 (schema/parser, including `StyleColor`).
- Sub-spec #2 (page-level style application and active wiring phase support).
- Sub-spec #3 (component style wiring for table/images/block-quote).
- Sub-spec #4 (`PageComponent::{Ul, Ol, Li}` split and list styling).

## Design Decisions

1. **Store `StyleColor` directly.** Extend `DarkmatterPage` with color maps
   that store the same `StyleColor` produced by the parser:

   ```rust
   page_color: Option<StyleColor>,
   page_bg_color: Option<StyleColor>,
   component_colors: HashMap<PageComponent, StyleColor>,
   component_bg_colors: HashMap<PageComponent, StyleColor>,
   ```

   Do not add a separate `with_component_opacity` API. Splitting opacity from
   color would undo the contract established by sub-spec #1 and make it
   possible to attach opacity to the wrong slot.

2. **Page color is inheritance, not a wrapper-only CSS rule.**
   `style.page.color` and `style.page.bg-color` fill component defaults in
   `LayoutContext`. A component-level value overrides the page-level value for
   that component. If neither exists, no color rule/SGR is emitted. Code
   blocks only participate through this page-level inheritance in the current
   schema.

3. **No explicit inheritance clearing in this phase.** Because the parser does
   not currently produce `Color::Reset`, a component cannot say "ignore the
   page color and return to terminal/default CSS color." This is acceptable for
   sub-spec #5 because it keeps the implementation aligned with the accepted
   schema. A future parser extension can add a dedicated clear/reset value if
   real documents need it.

4. **Terminal lowering uses RGB-only colors.** For terminal output, call
   `StyleColor.color.to_rgb()`.
   - `Some((r, g, b))` lowers to truecolor SGR when color depth allows it, or
     to the existing terminal renderer's lower-depth fallback if such a helper
     already exists.
   - `None` means "no terminal SGR" for this slot. This covers
     `Tailwind::{Transparent, Current, Inherit}`,
     `Color::{DefaultForeground, DefaultBackground, Reset}` if they ever reach
     the value, and any other non-fixed color.
   - `ColorDepth::None` emits no color SGR.

5. **Browser lowering preserves CSS-special colors where possible.** Add a
   darkmatter-local helper for `StyleColor` to CSS:
   - RGB-capable values produce `rgb(r, g, b)` or
     `rgba(r, g, b, opacity / 100.0)`.
   - `Color::Tailwind(Tailwind::Transparent)` produces `transparent`.
   - `Color::Tailwind(Tailwind::Current)` produces `currentColor`.
   - `Color::Tailwind(Tailwind::Inherit)` produces `inherit`.
   - Unsupported non-RGB/default/reset values return `None` and emit no CSS
     declaration.

6. **Component color CSS joins the existing component CSS rule.**
   Extend `build_component_css` so alignment, fill, color, background-color,
   and list-indent declarations for the same `PageComponent` are emitted in a
   single selector rule. Split list selectors from sub-spec #4 still apply:
   `Ul -> ul`, `Ol -> ol`, `Li -> li`, deprecated `Lists -> ul, ol`.

7. **Add `PageComponent::Hyperlinks` in this phase.** The schema already
   reserves `style.hyperlinks.color` and `style.hyperlinks.bg-color` for
   sub-spec #5, so this phase must add enough hyperlink component routing to
   honor those two common color fields. Full hyperlink layout and
   `local-style` behavior remain sub-spec #7. Browser selectors map
   `Hyperlinks -> a`; terminal rendering wraps link label text with SGR while
   preserving existing OSC8 hyperlink sequences.

8. **Terminal component color is scoped at render boundaries.** Extend
   `LayoutContext` with resolved component foreground/background accessors and
   add a small renderer helper that wraps a component's rendered string with:

   ```text
   <foreground SGR><background SGR>component output<reset>
   ```

   The helper must no-op when neither slot lowers to SGR. The reset is emitted
   only when at least one SGR was opened.

9. **Code-block inherited color semantics are deliberately asymmetric.**
   Page-inherited `bg-color` may apply to the code-block panel/container.
   Page-inherited foreground color is a fallback/default only and must not
   override syntax token foreground colors. If the current renderer has no
   safe hook for non-highlighted fallback text, line numbers, or wrapper text,
   skip inherited foreground for highlighted code blocks and document that
   limitation.

10. **Background-color should not erase token highlights.**
    For code blocks, inherited `bg-color` changes the containing
    panel/background fill rather than rewriting per-token background spans.
    Token-level backgrounds emitted by the syntax highlighter remain intact.

11. **Warning lifecycle follows the active phase.** Advance
    `ACTIVE_STYLE_WIRING_SUB_SPEC` to `5` only after all color fields in this
    spec are actually wired. Do not mutate schema descriptor `sub_spec` values.

## Public API

```rust
// darkmatter::layout::DarkmatterPage — extended

impl DarkmatterPage {
    pub fn with_page_color(self, color: StyleColor) -> Self;
    pub fn with_page_bg_color(self, color: StyleColor) -> Self;
    pub fn with_component_color(self, component: PageComponent, color: StyleColor) -> Self;
    pub fn with_component_bg_color(self, component: PageComponent, color: StyleColor) -> Self;

    pub fn page_color(&self) -> Option<&StyleColor>;
    pub fn page_bg_color(&self) -> Option<&StyleColor>;
    pub fn color_for(&self, component: PageComponent) -> Option<&StyleColor>;
    pub fn bg_color_for(&self, component: PageComponent) -> Option<&StyleColor>;
}

// darkmatter::style — extended

pub fn apply_color_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
) -> Result<DarkmatterPage, StyleApplyError>;
```

```rust
// darkmatter::layout::PageComponent — modified

pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    #[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
    Lists,
    Ul,
    Ol,
    Li,
    Hyperlinks,
}
```

`color_for` and `bg_color_for` should return the effective value after
component-over-page inheritance. If a test needs to inspect only the explicitly
configured component maps, add separate crate-private helpers rather than
overloading the public effective accessors.

The CLI render pipeline becomes:

```text
DarkmatterPage::new(...)
  -> apply_cli_layout_flags(...)
  -> apply_page_style(...)
  -> apply_component_style(...)
  -> apply_list_style(...)
  -> apply_color_style(...)
  -> render / render_to_browser
```

## Implementation Notes

- Thread the new color maps into `LayoutContext::from_page` for both
  `render` and `render_to_browser`.
- Update `is_default_layout`, `LayoutContext::needs_decoration`, and
  `LayoutContext::has_component_styles` so color-only configuration does not
  get optimized away.
- Keep page background (`style.page.background`) separate from page
  background-color (`style.page.bg-color`). `background` remains the existing
  coarse fill level (`transparent|subtle|pronounced`); `bg-color` is the
  explicit color used when painting component/page backgrounds.
- If both `style.page.background` and `style.page.bg-color` are set,
  `bg-color` supplies the color while `background` continues to control whether
  row decoration/fill is active. If `bg-color` is set without `background`,
  row decoration must still activate so the color can be visible.
- For terminal color depth, reuse existing darkmatter/biscuit-terminal helpers
  if available. If no shared helper exists, implement the minimum local helper
  that emits `38;2;r;g;b` / `48;2;r;g;b` for truecolor and no-ops for
  `ColorDepth::None`; do not introduce raw escape-code construction outside a
  terminal-rendering helper.
- Preserve canonical kebab-case in errors, docs, and tests:
  `style.block-quote.bg-color`, not `style.block_quote.bg_color`.
- Update `darkmatter/docs/rendering/style.md` to move color/bg-color from
  "pending" to "live" for the buckets wired here and to document opacity as
  browser-only.
- Update sub-spec #7 during implementation planning if needed: #7 should no
  longer introduce `PageComponent::Hyperlinks`; it should extend the component
  created here with layout/local-style behavior.

## Tests

1. **Page color inherited by components** — `style.page.color: red-500` makes
   tables, block quotes, hyperlinks, lists, and safe code-block fallback text
   render with red foreground where the component has no explicit color.
2. **Component color overrides page color** — `style.page.color: blue-500`
   plus `style.table.color: red-500` renders tables red while other components
   remain blue.
3. **Component bg-color overrides page bg-color** —
   `style.page.bg-color: black` plus `style.table.bg-color: red-500/50`
   renders table backgrounds red on terminal and `rgba(239, 68, 68, 0.5)` in
   HTML.
4. **Opacity dropped on terminal** — `style.table.bg-color: red-500/50`
   produces normal RGB/background SGR with no opacity-like bytes; browser CSS
   preserves `rgba(...)`.
5. **Color depth none** — with `ColorDepth::None`, color settings emit no SGR
   while the visible text/layout remains unchanged.
6. **Terminal reset boundary** — a colored component is followed by unstyled
   content; the output contains a reset after the component and no color leaks
   into the following line.
7. **Inherited code-block bg-color panel override** —
   `style.page.bg-color: black` changes the code block panel/background
   without removing syntax token foreground styling.
8. **Inherited code-block foreground does not clobber highlighting** —
   `style.page.color: red-500` with a highlighted Rust block preserves token
   foreground SGR. If the implementation skips inherited code-block foreground
   for highlighted blocks, assert the skip and document it.
9. **Browser CSS special colors** — `style.table.bg-color: transparent`,
   `style.table.color: current`, and `style.table.color: inherit` emit valid
   CSS declarations and no terminal SGR.
10. **List selectors** — `style.ul.color`, `style.ol.bg-color`, and
    `style.li.color` emit separate `ul`, `ol`, and `li` CSS rules after
    sub-spec #4's selector split.
11. **Hyperlink color routing** — `style.hyperlinks.color: red-500` colors
    terminal link label text without breaking OSC8 hyperlink sequences and
    emits `a { color: ... }` CSS for browser output.
12. **Active wiring warnings** — color and bg-color fields for page, table,
    images, block-quote, hyperlinks, ul, ol, and li no longer emit
    `KnownButInactive { sub_spec: 5 }`; `style.hr.color`,
    `style.hr.bg-color`, and `style.hyperlinks.local-style.*` remain pending
    until their later sub-specs.

## Acceptance Criteria

- Every active `PageComponent` after this sub-spec, including the newly added
  `Hyperlinks` variant, honors effective foreground/background color from
  frontmatter in terminal and browser output.
- `style.page.color` and `style.page.bg-color` act as inherited component
  defaults.
- Component-level color and bg-color override page-level color and bg-color.
- Browser output preserves opacity; terminal output drops it.
- Terminal color output respects `ColorDepth::None` and resets after styled
  component scopes.
- Code-block background color changes the panel/container without breaking
  token highlighting.
- All previous sub-spec tests pass.
- `KnownButInactive { sub_spec: 5 }` warnings are suppressed for all fields
  wired by this sub-spec and still emitted for HR color fields until #6.
- `style.code-blocks.color` and `style.code-blocks.bg-color` remain unknown
  keys; this sub-spec must not document or implement them as valid
  frontmatter.
- `darkmatter/docs/rendering/style.md` documents live color support, opacity
  behavior, special CSS colors, and the lack of explicit `reset` parsing.

## Risks

- **SGR sequence pollution.** If a component opens a color sequence and does
  not reset it, later content or the user's shell can inherit the wrong color.
  Mitigation: use one shared scoped-color helper and test a colored component
  followed by unstyled text.
- **Code-block interaction.** Wrapping an entire highlighted code block with a
  foreground SGR can override token colors. Mitigation: treat code-block
  foreground as a fallback only; bg-color applies to the panel/container.
- **Special color mismatch.** Tailwind special values such as `transparent`,
  `current`, and `inherit` are meaningful in CSS but not terminal SGR.
  Mitigation: browser lowers them to CSS keywords; terminal no-ops for
  non-RGB values.
- **Default-layout fast path.** Color-only style can be accidentally dropped if
  `is_default_layout` and `has_component_styles` only look at layout fields.
  Mitigation: include color maps in those checks and add a color-only render
  regression test.

## Reader Note

This reviewed version intentionally changes the draft API. The earlier sketch
accepted `renderable::color::Color` and then added separate opacity builders.
That conflicts with sub-spec #1, which already chose `StyleColor` as the
frontmatter representation so opacity stays attached to the exact color slot
the user configured. Implementers should pass `StyleColor` through
`DarkmatterPage` and `LayoutContext` unchanged until target-specific lowering.

The reviewed version also settles `Color::Reset`: it is not added here because
the current v1 parser does not accept `"reset"`. Treating reset as supported in
this sub-spec would require schema/parser changes outside the intended color
rendering scope.

## Open Questions

None. Explicit reset/clear syntax can be proposed as a future parser extension
if documents need component-level opt-out from page color inheritance.

## Out-of-Spec

After sub-spec #5 lands, the common mutation surface is wired for all
post-sub-spec-#4 components except HR. Sub-spec #6 adds HR, and sub-spec #7
finishes the bespoke knobs.

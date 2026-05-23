---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 5-of-7
depends-on: spec.md, spec-2.md..spec-4.md
---

# `style:` Frontmatter — Sub-Spec #5: Color & Background-Color Mutations

## Problem

Every `CommonStyle` field carries `color` and `bg_color` (parsed as
`StyleColor { color: renderable::color::Color, opacity: Option<u8> }`).
The schema accepts them today but no rendering path honors them. This
sub-spec adds per-component color/bg-color rendering to both terminal
(SGR sequences) and browser (CSS) output.

## Goals

- Implement per-component color and background-color in
  `DarkmatterPage` for every `PageComponent` variant: `Images`,
  `BlockQuotes`, `Tables`, `CodeBlocks`, `Ul`, `Ol`, `Li`.
- Terminal: emit appropriate SGR foreground/background sequences before
  the component renders, reset after.
- Browser/HTML: emit per-component `<style>` rules (via the existing
  `wrap_browser_html` path) with `color: rgb(...)` /
  `background-color: rgb(...)` and optional opacity via `rgba(...)`.
- `style.page.color` / `style.page.bg-color` apply as page-level defaults
  (every component without its own color inherits them).
- Opacity (`/50`) is **HTML-only**, dropped on terminal targets, matching
  `docs/rendering/style.md`.
- Suppress `KnownButInactive { sub_spec: 5 }` warnings for wired keys.

## Non-Goals

- No new color types (reuse `renderable::color::Color`).
- No alpha blending in terminal (terminals don't support it).
- No syntax-highlighting overrides — `style.page.color` does not
  override `code_theme`.
- No per-component nested inheritance beyond page → component.

## Dependencies

- Sub-spec #1 (schema, including `StyleColor`).
- Sub-spec #2 (page-level wiring, so `style.page.color` has a hook).
- Sub-spec #3 (component-level wiring for the existing variants).
- Sub-spec #4 (list split, so per-list-kind color is possible).

## Decisions to Settle (Brainstorm Inputs)

1. **Where do color settings live on `DarkmatterPage`?** Add two new
   `HashMap<PageComponent, StyleColor>` fields: `colors` and
   `bg_colors`. New builders: `with_component_color(component, color)`
   and `with_component_bg_color(component, color)`. Page-level
   `with_page_color` / `with_page_bg_color` set the defaults.

2. **SGR emission strategy.** Two options:
   - Wrap component output in an SGR open + SGR reset.
   - Modify the per-component render path to thread the color through
     and let it own emission.
   Recommended: wrap-style (less intrusive; works uniformly across
   variants).

3. **`color: Color::Reset` semantics.** `style.page.color: reset` would
   explicitly clear inheritance. Confirm this is supported by the
   Tailwind/hex parser path. (Currently `Color::Reset` is not produced
   by the v1 color parser — adding it would require schema updates.)

4. **Opacity on hex colors.** `#ff000080` → `Color::Rgb(...)` with
   opacity `Some(50)`. CSS output: `rgba(255, 0, 0, 0.5)`. Terminal
   output: ignore opacity, emit `\x1b[48;2;255;0;0m`.

5. **Tailwind-name `Color::Tailwind(Tailwind::Red500)` SGR mapping.**
   Use `Tailwind::Red500.to_rgb()` (existing on the type? Confirm via
   `renderable/src/color/tailwind.rs` — the doc example shows
   `Color::to_rgb()` working).

6. **Page-level vs. component-level priority.** Component-level wins.
   Page-level fills the gap when no component sets it.

7. **Background and code blocks.** Code blocks already have their own
   theme-managed background. Setting `style.code-blocks.bg-color`
   would override the syntax background. Confirm desired behavior:
   recommended → override the theme background only for the
   code-block bounding box, not the per-token highlight.

## Public API (Sketch)

```rust
// darkmatter::layout::DarkmatterPage — extended

impl DarkmatterPage {
    pub fn with_page_color(self, c: renderable::color::Color) -> Self;
    pub fn with_page_bg_color(self, c: renderable::color::Color) -> Self;
    pub fn with_component_color(self, comp: PageComponent, c: renderable::color::Color) -> Self;
    pub fn with_component_bg_color(self, comp: PageComponent, c: renderable::color::Color) -> Self;
    // Opacity stored separately; needed only on the browser path.
    pub fn with_component_opacity(self, comp: PageComponent, opacity: u8) -> Self;
}

// darkmatter::style — extended

pub fn apply_color_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    cli_overrides: &CliLayoutOverrides,
) -> Result<(DarkmatterPage, Vec<StyleWarning>), StyleApplyError>;
```

## Tests

1. **Page color from frontmatter** — `style.page.color: red-500` →
   rendered prose carries `\x1b[38;2;239;68;68m` (Tailwind red-500 RGB).
2. **Component bg-color overrides page bg** — `style.page.bg-color: black`
   + `style.tables.bg-color: red-500/50` → table backgrounds emit
   red SGR on terminal, `rgba(239, 68, 68, 0.5)` on HTML.
3. **Opacity dropped on terminal** — `style.tables.bg-color: red-500/50`
   → terminal output has no `;0.5` (or whatever the broken syntax would
   look like); HTML output has `rgba(...)`.
4. **Reset propagation** — A page-level color is reset at the document
   boundary so the user's shell prompt isn't corrupted.
5. **Code-block bg-color override** — `style.code-blocks.bg-color: black`
   → code block panel renders against black instead of the theme color.
6. **No-color terminal (`ColorDepth::None`)** — color settings are
   silently dropped; output is plain.

## Acceptance Criteria

- Every `PageComponent` variant honors color and bg-color from
  frontmatter on both terminal and browser targets.
- Opacity preserved on browser, dropped on terminal.
- All previous sub-spec tests pass.
- `KnownButInactive { sub_spec: 5 }` warnings suppressed for wired keys.

## Risks

- **SGR sequence pollution.** If a page-level color is set but reset is
  forgotten on error paths, the user's shell will be left in the wrong
  color state. Mitigation: always emit reset on render exit, even on
  error.
- **Code-block interaction.** Overriding the bg-color of a code panel
  must not break the per-token highlight (which uses its own SGR
  sequences). Need careful nesting of color contexts.
- **`Color::Tailwind` → RGB resolution.** Tailwind enum doesn't expose
  a direct RGB method on every variant — needs the existing
  `Color::to_rgb()` path. Confirm it covers every variant.

## Open Questions

1. Schema-level handling of `Color::Reset` — extend the v1 parser to
   accept the string `"reset"` as a color value?
2. Code-block bg-color: override the panel only, or the whole code area?
3. Opacity on terminal — drop silently, or warn the user?

## Out-of-Spec

After sub-spec #5 lands, the visual styling surface is complete except
for HR (#6) and bespoke knobs (#7).

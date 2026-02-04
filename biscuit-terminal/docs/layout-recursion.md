# Layout and Recursion Notes

This document summarizes how `Layout` is used in core components and how
recursive rendering through `RenderableContent` behaves today.

## Layout in Prose

`Prose` owns a `Layout` and applies it after token parsing:

- `render()` and `fallback_render()` both call `parse_tokens()` and then
  `layout.apply_layout(...)` with the final terminal width.
- The only layout configuration exposed directly is word wrap and left/right
  margin (`with_word_wrap`, `with_left_margin`, `with_right_margin`).
- Token parsing is recursive for nested block tags; this recursion is internal
  to prose parsing and happens before layout is applied.

Implication: layout affects the post-tokenized string, so styles and OSC8 links
are preserved while margins, alignment, and wrapping are applied at the end.

## Layout in BlockQuote

`BlockQuote` uses `Layout` as a container, but it does not fully propagate
width constraints to nested components:

- `render()`/`fallback_render()` compute `available = layout.available_width(...)`.
- The inner content is rendered via `render_content(...)`, then the outer
  layout is applied with `layout.apply_layout(&content, width)`.
- `render_content()` computes a `_child_width` (terminal width minus the
  border width), but that value is not used. When the content is a component,
  it calls `component.fallback_render(term)` which renders against the full
  terminal width.

Implication: the left border reduces visible width, but nested components can
still wrap as if they have full width. Percent-based margins in nested
components resolve relative to the terminal, not the block quote's available
width.

## Layout in UnorderedList (and OrderedList)

`UnorderedList` and `OrderedList` treat `Layout` as a wrapper around the
entire list output:

- `render()`/`fallback_render()` compute `available = layout.available_width(...)`.
- `render_content(...)` uses that available width when it renders children and
  then `layout.apply_layout(&content, width)` is applied to the final string.
- Block-level children are rendered with a reduced width (`term_width - indent`)
  and then manually indented. Inline components and string items use the full
  `term_width` before the list-level layout runs.
- `hanging_indent` is stored but never used; wrapping is handled only by the
  list's layout applied to the final assembled string.

Implication: list-level wrapping can break bullet alignment (wrapped lines do
not carry the bullet prefix), and inline components can wrap as if the bullet
does not consume any width.

## RenderableContent Recursion: What Works Well

Components that own `RenderableContent` gain composability via recursion:

- `RenderableContent::Component` calls `render()`/`fallback_render()` on the
  child component, so nested components render themselves first and the parent
  only post-processes (prefixes, indentation, layout wrappers).
- Lists are a good example: nested lists render recursively, and indentation
  compounds (see tests like `test_three_level_nesting_width_compounds`).
- This pattern keeps component logic local and lets each component apply its
  own layout before the parent applies its layout.

## Missed Opportunities / Possible Improvements

1) Use layout composition for nested components.
   - `Renderable::as_child_of` and `Margin::Offset` exist to compose margins
     without prematurely resolving percentages, but lists and block quotes do
     not use them. As a result, percent-based margins in children resolve to
     terminal width rather than the parent container width.

2) Respect inner width constraints when terminal capabilities are needed.
   - `BlockQuote` computes a child width but does not pass it to the child
     renderer. A `fallback_render_with_width` (or similar) would let
     terminal-aware rendering respect container width without losing capability
     checks.

3) Wire hanging indent to word wrap in lists.
   - `UnorderedList::hanging_indent` is currently unused. It could map to
     `Layout.word_wrap = WordWrap::WrapProse(_, Some(indent))`, where `indent`
     matches the bullet width or `indent_children`.

4) Inline list items should account for bullet width.
   - Inline components currently render against full `term_width`. Passing
     `term_width - bullet_width` would keep their internal wrapping aligned
     with the bullet and prevent overflow.

5) Prefer layout-driven indentation over manual prefixing.
   - Manual indentation works, but it bypasses alignment and percent margin
     handling. Using `as_child_of` and layout margins would make nested
     components behave consistently with the rest of the layout system.

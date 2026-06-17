# Rendering

`Icon` renders through the shared `renderable` multi-target tree, so
the same `Icon` value produces inline SVG in a browser, inline SVG in
markdown, and a degradation ladder in a terminal. Two traits are
implemented:

- `biscuit_terminal::components::renderable::TerminalRenderable` for the
  terminal target.
- `renderable::tree::TreeRenderable` for the browser / markdown /
  tree-routed terminal targets.

## Terminal Degradation Ladder

`Icon::render(&term)` walks the rungs in this order and returns the
first one that produces output:

1. **Nerd Font glyph** — only when `Icon::nerd_font(true)` was set (or
   the CLI was invoked with `--nerd` / `ICON_NERD_FONT=1`) **and** the
   icon defines a Nerd Font codepoint. Nerd Font presence is not
   reliably detectable, so this is config/flag/env-gated, never
   auto-sniffed.
2. **Unicode glyph** — when the icon defines a Unicode codepoint. The
   most common case for the curated `Emoji` set.
3. **Image-protocol render** — only when the `image` cargo feature is
   enabled **and** the terminal advertises an image protocol other
   than `ImageSupport::None`. The assembled SVG is written to a
   temp file, rasterized via `biscuit-visualized` (which uses `resvg`),
   and rendered through `biscuit_terminal`'s image protocol
   negotiation (Kitty, iTerm2, Sixel, half-block fallback).
4. **Text identifier** — the icon's id (`hugeicons:apple-finder`,
   `mdi:home`, ...) rendered through `Prose`. Always available.

Glyph-first ordering keeps the common terminal path cheap and avoids
pulling `resvg` into the hot path. Image rendering is an opt-in
enhancement for glyph-less icons.

## `TreeRenderable` Payload

`Icon::render_tree()` projects the icon into a canonical `RenderNode`
tree:

```rust
RenderNode::root(vec![RenderNode::extended(
    "icon",
    vec![RenderNode::html(self.svg(), false)],
    Some(json_payload),
)])
```

The `icon` extension token carries a JSON payload that encodes the
ladder rungs so a renderer that *receives* a tree (rather than an
`Icon`) can still pick the best representation:

```json
{
  "nerd_font": "ﴓ",
  "unicode": "😀",
  "text": "mdi:home",
  "svg": "<svg ...>...</svg>",
  "nerd_font_preferred": false
}
```

The `svg` field is only present when the `image` cargo feature is
enabled. The `text` field is always present (the id is the universal
fallback). Terminal renderers consume this payload via
`biscuit_terminal::render_tree::render_terminal_node`.

## Target Folds

### Browser / HTML

`render_browser_node(&tree, &BrowserRenderOptions { raw_html: RawHtmlPolicy::Allow, ... })`
lowers the tree to inline HTML; the inner `RenderNode::html(...)`
emits the assembled `<svg>` verbatim when the policy permits raw
HTML. The `Icon` produces valid, well-formed inline SVG (`<svg
xmlns="..." viewBox="...">...</svg>`), so it round-trips through any
modern browser.

### Markdown

Two dialects:

- `MarkdownDialect::MarkdownPlus` — inline SVG is emitted (markdown
  permits HTML). Use this in darkmatter-rendered docs.
- `MarkdownDialect::Markdown` with `RenderStrictness::Strict` — raw
  HTML is rejected. The icon cannot be rendered as inline SVG; the
  caller is expected to switch to a different target or to allow
  raw HTML.

### Terminal (tree-routed)

`render_terminal_node(&tree, &TerminalRenderOptions::default())`
recognizes the `icon` extension token and walks the payload ladder,
producing the same output as `Icon::render(&term)` for the equivalent
`Terminal` capability set.

## `Style::assemble` — the SVG output

The assembled `<svg>` produced by `Icon::svg()` (or, equivalently,
`Style::assemble(body)`) has the shape:

```html
<svg xmlns="http://www.w3.org/2000/svg" width="..." height="..."
     viewBox="left top width height" style="color: ...">
  <rect ... fill="none"/>     <!-- only when view_box(true) -->
  <g transform="...">...</g>  <!-- when flip/rotate is set -->
  <path .../>                 <!-- the Iconify body -->
</svg>
```

Notes:

- `width` and `height` default to `1em`; user-supplied values are
  XML-attribute-escaped.
- `color` is emitted as an inline `style="color: …"` and is also
  escaped.
- `Flip` / `Rotate` are emitted as a `<g transform="…">` wrapping
  the body. Non-square bodies with `R90` / `R270` swap the viewBox
  width and height; non-zero view-box origins are compensated in
  the `translate` so the icon does not get pushed off-canvas.
- The `view_box` rect uses `fill="none"` and is invisible; it is
  there to make the bounding box hit-testable / selectable,
  matching Iconify's "Background" bounding box.

## Cargo Features

| Feature | Default | Effect |
|---------|---------|--------|
| `image` (library + CLI) | off | Enables the image rung of the terminal ladder. The library pulls in `biscuit-visualized` (which brings `resvg`); the `svg` field in the tree payload is only present when this feature is on. |

Build the library with image rendering:

```toml
biscuit-icon = { path = ".../biscuit-icon/lib", features = ["image"] }
```

Build the CLI with image rendering for `just test-l2`:

```bash
just test-l2
# (expands to: just _test_l2 biscuit-icon-cli --features image)
```

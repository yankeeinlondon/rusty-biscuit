# Progress — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                                  |
|------------|----------------------------------------------------------------------------------------|
| Name       | Progress                                                                               |
| Kind       | Block component rendered as one visual line                                            |
| Location   | `biscuit-terminal/lib/src/components/progress.rs`                                      |
| Terminal   | Bespoke `TerminalRenderable` still owns default `.render()`                            |
| Browser    | No direct `BrowserRenderable`; adapter path exists only for `TreeRenderable` producers |
| Markdown   | No direct `MarkdownRenderable`                                                         |
| Tree       | Compatibility `render_tree_node()` exists; canonical `TreeRenderable` is still missing |
| bt CLI     | `bt progress` exists and already renders Terminal output through `render_terminal_node` |

Progress renders a horizontal completion bar with an optional label:

```text
Loading [████████░░░░░░░░░░░░]  40%
```

The component owns:

- a clamped numeric value (`0.0..=1.0`);
- an optional label;
- a bar width in terminal cells / CSS `ch` units;
- a typed `ProgressStyle` carrying fill/empty/bracket glyphs plus
  `filled_color`, `empty_color`, and `bracket_color` slots;
- a block `Layout`.

Progress already projects to a `NodeKind::Paragraph` carrying
`ProgressHints` on `NodeAttrs`. The paragraph text is the semantic fallback:
`"{label} {percentage}%"` or `"{percentage}%"`. Renderers that understand
`ProgressHints` may draw a visual bar; renderers that do not still produce
readable text.

## Review Decisions

- Keep `NodeKind::Paragraph + ProgressHints`; do not add a dedicated
  `NodeKind::Progress`. Progress is a widget-specific presentation of a
  paragraph-level status, and `ProgressHints` already carry the required typed
  payload without expanding the structural Markdown-like node vocabulary.
- Add canonical `TreeRenderable` for `Progress`. The existing
  `TerminalRenderable::render_tree_node()` hook is a biscuit-terminal
  compatibility hook, not the canonical producer trait required by
  `TreeComponent`, `BrowserTreeComponent`, and future cross-target adapters.
- Make Terminal default rendering delegate through the tree only after a
  bespoke-vs-tree parity test is green. Keep the old implementation as an
  internal test helper until the parity ledger is stable.
- Treat portable Markdown as semantic text only. Markdown has no native
  progress bar or color model, so it should output the label and percentage and
  intentionally ignore bar glyphs, colors, and layout.
- Treat Browser and MarkdownPlus as visual targets. They should render a
  semantic HTML progress widget from `ProgressHints`, not a terminal
  character-art bar.

## Tree Projection

Factor Progress projection into one private helper so all producer surfaces
stay in lockstep:

```rust
impl Progress {
    fn to_render_node(&self) -> RenderNode {
        let percentage = (self.value * 100.0).round() as u32;
        let visible = match &self.label {
            Some(label) => format!("{label} {percentage}%"),
            None => format!("{percentage}%"),
        };

        let mut node = RenderNode::paragraph(vec![RenderNode::text(visible)]);
        node.attrs.set_progress_hints(&ProgressHints {
            value: self.value,
            bar_width: self.bar_width,
            fill_char: self.style.fill_char,
            empty_char: self.style.empty_char,
            left_bracket: self.style.left_bracket,
            right_bracket: self.style.right_bracket,
            filled_color: self.style.filled_color,
            empty_color: self.style.empty_color,
            bracket_color: self.style.bracket_color,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        node
    }
}
```

Then wire both traits to the helper:

```rust
impl renderable::tree::TreeRenderable for Progress {
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl TerminalRenderable for Progress {
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node())
    }
}
```

`ProgressHints` should remain typed and accessed through
`NodeAttrs::set_progress_hints()` / `NodeAttrs::progress_hints()`. Renderers
must not inspect raw `data` strings at call sites.

## Terminal IR Implementation

The terminal tree renderer already handles `ProgressHints` on paragraphs. It
reconstructs the bar from the hints, applies slot colors through the shared
terminal color lowering path, extracts the label from the paragraph fallback
text, and applies node `Layout` through `render_with_layout`.

Switch `TerminalRenderable::render()` and `render_optimistic()` to this tree
path:

```rust
impl Progress {
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }

    #[cfg(test)]
    fn render_bespoke_for_parity(&self, term: &Terminal) -> String {
        let bar_content = self.render_bar(term.color_depth);
        self.layout.apply_block_layout(&bar_content, term.width())
    }
}
```

`render_optimistic()` should construct `Terminal::new_optimistic(width)` and
then call `render_via_tree()` so optimistic rendering uses the same code path
as normal rendering.

### Layout Mapping

Progress attaches `Layout` to the projected paragraph when the layout is
non-default. Terminal layout applies to the whole single-line visual block:

- left/right margins add horizontal space and narrow the available width;
- top/bottom margins add blank rows;
- alignment positions the whole label/bar/percentage unit;
- `word_wrap` is ignored in practice because the progress bar is inherently
  non-wrapping;
- terminal `max_width` remains a no-op, matching `layout-and-style.md`.

### Terminal Test Strategy

Add or keep a dedicated `progress_parity.rs` gate that compares:

| Variant                                                | Required assertion                                                   |
|--------------------------------------------------------|----------------------------------------------------------------------|
| `0.0`, `0.5`, `1.0`                                    | Fill count, empty count, and formatted percentage match              |
| Label and no label                                     | Label placement and unlabeled leading bracket match                  |
| Custom width                                           | Filled + empty glyph count equals requested width                    |
| Custom fill/empty/bracket glyphs                       | Hints carry glyphs and tree output uses them                         |
| Filled, empty, and bracket colors                      | SGR is present at capable color depths and absent at `ColorDepth::None` |
| RGB color degradation                                  | Truecolor, 16-color fallback, and no-color behavior are covered      |
| Clamping below `0.0` and above `1.0`                    | Projection and output show `0%` / `100%`                             |
| Percentage alignment (`  0%`, ` 75%`, `100%`)          | Right-aligned percentage formatting is preserved                     |
| Layout left/right/top/bottom margins and center align  | Tree layout matches bespoke layout semantics                         |
| Small terminal widths                                  | Output remains deterministic and does not panic                      |
| `ProgressStyle` serde round trip                       | Glyphs and color slots survive serialization                         |

Parity should assert ANSI-stripped content equality for the full line and
separate color-specific assertions for SGR behavior. Any accepted divergence
must be listed in a local `KNOWN_DRIFT` ledger with the reason.

## Browser IR Implementation

Progress should not get a bespoke browser renderer. Browser support should
come from the canonical tree path:

1. `Progress` implements `TreeRenderable`.
2. `BrowserTreeComponent<Progress>` projects to the tree.
3. `renderable::tree::render_browser_node()` handles `ProgressHints`.

### Render-Tree Feature Request RT-PROGRESS-001

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: the tree already carries a target-agnostic `ProgressHints` payload, and
the terminal renderer already treats that payload as native widget semantics.
Adding browser handling to the tree renderer keeps the tree as the single
source of truth and avoids a second bespoke `Progress` browser implementation.

Required behavior:

- In the browser renderer, handle `ProgressHints` in the
  `NodeKind::Paragraph` branch before normal paragraph rendering.
- Emit semantic HTML with `role="progressbar"`, `aria-valuemin="0"`,
  `aria-valuemax="100"`, `aria-valuenow`, and an accessible label derived from
  the component label when present.
- Use stable classes such as `progress`, `progress-label`, `progress-track`,
  `progress-filled`, and `progress-percentage`.
- Apply node `Layout` to the outer progress element via the existing browser
  layout lowering.
- Clamp `hints.value` to `0.0..=1.0`; render filled width as
  `round(value * 100)%`; render track width as `bar_width` in `ch`.
- Lower `filled_color` and `empty_color` to CSS `background-color` values on
  the filled segment and track. Lower `bracket_color` only if the browser
  output chooses to render bracket affordances; otherwise preserve it as a
  typed hint and do not invent terminal glyph output.
- Do not render terminal fill/empty glyph repetition by default. Browser output
  is a CSS progress bar, not terminal character art.
- Preserve non-default glyph and bracket values in `data-fill-char`,
  `data-empty-char`, `data-left-bracket`, and `data-right-bracket` attributes
  so the information is not silently discarded from the HTML surface.
- HTML-escape all label and percentage text.
- Normal paragraph rendering must remain unchanged when no `ProgressHints` are
  present.

### Browser Test Strategy

| Variant                              | Required assertion                                           |
|--------------------------------------|--------------------------------------------------------------|
| `0.0`, `0.5`, `1.0`                  | `aria-valuenow` and filled width are `0`, `50`, `100`        |
| Label and no label                   | Label span appears only when label exists                    |
| Custom bar width                     | Track width uses requested `ch` value                        |
| Filled and empty colors              | CSS background colors are emitted on the correct elements    |
| Bracket color                        | Either bracket affordance color is used or no glyph is emitted |
| Custom glyphs/brackets               | Values are preserved in `data-*` attributes                  |
| Layout with margin/alignment/width   | Existing `layout_to_css` output appears on the outer element |
| Fallback paragraph                   | Plain paragraphs without hints still render as `<p>`         |
| Escaping                             | Label text is escaped in HTML and attributes                 |

## Markdown IR Implementation

Portable Markdown should continue to render the paragraph fallback text:

- `Progress::new(0.75).with_label("Loading")` -> `Loading 75%`
- `Progress::new(0.5)` -> `50%`

This output intentionally drops colors, glyphs, bracket presentation, and
layout because portable Markdown has no native representation for them.

MarkdownPlus can preserve the visual widget because inline HTML is allowed.

### Render-Tree Feature Request RT-PROGRESS-002

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: MarkdownPlus is the rich Markdown dialect in this codebase. Rendering
`ProgressHints` as inline HTML in MarkdownPlus gives components one canonical
projection while allowing richer outputs to preserve the progress widget
visually. Portable Markdown remains clean, valid, and semantic by using the
paragraph fallback text.

Required behavior:

- In the Markdown renderer, keep `MarkdownDialect::Markdown` behavior as plain
  text from the paragraph children.
- In `MarkdownDialect::MarkdownPlus`, handle `ProgressHints` on paragraphs by
  emitting the same semantic progress HTML shape used by the browser renderer,
  serialized as inline/block HTML acceptable in MarkdownPlus.
- Apply color slots as inline CSS when present.
- Do not apply `Layout` in either Markdown dialect; this matches the documented
  Markdown layout contract in `layout-and-style.md`.
- Preserve non-default glyph and bracket values in `data-*` attributes for the
  same reason as browser rendering.
- Plain paragraphs without `ProgressHints` must remain unchanged.

### Markdown Test Strategy

| Variant                                      | Required assertion                                      |
|----------------------------------------------|---------------------------------------------------------|
| Markdown, no label                           | Output is `50%` style semantic text                     |
| Markdown, with label                         | Output is `Loading 50%`                                 |
| Markdown, with colors/custom glyphs/layout   | Output is still only label + percentage                 |
| MarkdownPlus, default style                  | Output contains progress HTML and visible percentage    |
| MarkdownPlus, with label                     | Label appears in the HTML and accessible label          |
| MarkdownPlus, with colors                    | Inline CSS color slots are present                      |
| MarkdownPlus, custom glyphs/brackets         | `data-*` attributes preserve the values                 |
| MarkdownPlus, with layout                    | Layout has no effect on output                          |
| Plain paragraph without hints                | Existing Markdown and MarkdownPlus output is unchanged  |

## `bt progress` CLI

`bt progress` already exists and renders terminal output through the tree. It
should gain cross-target switches once Browser and MarkdownPlus tree support
exists.

| Flag              | Type             | Description                                                       |
|-------------------|------------------|-------------------------------------------------------------------|
| `PERCENT`         | `Option<u8>`     | Completion percentage 0-100, required unless `--example`          |
| `--example`, `-e` | `bool`           | Render example and print the example command                      |
| `--label`         | `Option<String>` | Label shown before the bar                                        |
| `--width`         | `Option<u32>`    | Width of the bar portion                                          |
| `--fill-color`    | `Option<String>` | Filled segment color                                              |
| `--empty-color`   | `Option<String>` | Empty track color                                                 |
| `--bracket-color` | `Option<String>` | Bracket color, retained for terminal and hint-preserving outputs  |
| `--html`          | `bool`           | Render an HTML fragment; conflicts with `--md` and `--md-plus`    |
| `--md`            | `bool`           | Render portable Markdown; conflicts with `--html` and `--md-plus` |
| `--md-plus`       | `bool`           | Render MarkdownPlus; conflicts with `--html` and `--md`           |

Render paths:

1. Terminal default: `Progress::render_tree()` or `render_tree_node()` ->
   `render_terminal_node()`.
2. `--html`: `BrowserTreeComponent::new(progress).render_html_fragment()`.
3. `--md`: `render_markdown_node()` with `MarkdownDialect::Markdown`.
4. `--md-plus`: `render_markdown_node()` with
   `MarkdownDialect::MarkdownPlus`.

The CLI should not use `MarkdownRenderOptions::default_plus()` unless that
helper is added; today the precise option is:

```rust
MarkdownRenderOptions {
    dialect: MarkdownDialect::MarkdownPlus,
    ..Default::default()
}
```

CLI tests should cover default terminal output, the existing `--example`
behavior, flag conflict validation, `--html`, `--md`, `--md-plus`, color flag
parsing for every target, and invalid percentages above 100.

## Acceptance Criteria Summary

- [ ] `Progress` has one private tree projection helper.
- [ ] `Progress` implements canonical `TreeRenderable`.
- [ ] `TerminalRenderable::render_tree_node()` delegates to the shared helper.
- [ ] `TerminalRenderable::render()` and `render_optimistic()` delegate to the
      tree renderer by default.
- [ ] The old bespoke rendering logic is retained only as a test-only parity
      helper until migration confidence is established.
- [ ] Browser tree rendering handles `ProgressHints` as approved in
      RT-PROGRESS-001.
- [ ] MarkdownPlus tree rendering handles `ProgressHints` as approved in
      RT-PROGRESS-002.
- [ ] Portable Markdown output remains label + percentage text.
- [ ] `Progress` can be rendered through `BrowserTreeComponent`.
- [ ] `Progress` implements `MarkdownRenderable` by delegating to the tree
      renderer for Markdown and MarkdownPlus.
- [ ] `bt progress --html`, `--md`, and `--md-plus` are added with conflicts.
- [ ] Existing `bt progress` terminal flags and `--example` behavior remain
      unchanged.
- [ ] Terminal, Browser, Markdown, MarkdownPlus, CLI, serde, layout, and
      parity tests cover the variants listed above.

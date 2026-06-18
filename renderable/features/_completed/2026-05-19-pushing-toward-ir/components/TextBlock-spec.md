# TextBlock — IR Rendering Design Specification

| Property | Value |
|----------|-------|
| Component | `TextBlock` |
| Location | `biscuit-terminal/lib/src/components/text_block.rs` |
| Kind | Block |
| Terminal | ✅ (bespoke) |
| Browser | ❌ |
| Markdown | ❌ |
| Tree | ❌ |
| IR State | `needs TreeRenderable; terminal can delegate after parity` |
| bt CLI | `none; bt block is adjacent but not TextBlock-backed` |
| `will_use_tree_renderer` | `true` |
| `will_use_tree_renderer_with_features` | `true` for Terminal and Browser, `false` for styled MarkdownPlus |

## Current State

`TextBlock` is intended to be a uniformly styled block of text with optional layout. Its public builder surface stores text, font weight, foreground/background color, italic, strikethrough, blink, underline, and `Layout`.

The current bespoke terminal renderer does **not** apply every stored field. `to_terminal()` currently applies only:

- italic, gated by `term.supports_italic`
- `font_weight`, gated by `term.is_tty` through `FontWeight::term_wrap`
- layout, through `LayoutTerminalExt::apply_layout`

The following stored fields are currently inert in `TextBlock::render()` / `render_optimistic()`:

- `fg_color`
- `bg_color`
- `strikethrough`
- `blink`
- `underline`

That makes the migration partly parity work and partly bug-fix/field-activation work. The tree projection can represent all stored fields losslessly, but re-pointing terminal rendering through the tree will intentionally make previously inert fields visible.

Fields on the struct today:

| Field | Type | Tree mapping | Current bespoke terminal behavior |
|-------|------|--------------|-----------------------------------|
| `content` | `String` | `NodeKind::Text` inside `NodeKind::Paragraph` | rendered |
| `font_weight` | `FontWeight` (`Normal` / `Bold` / `Dim`) | `Style.emphasis.bold` / `Style.emphasis.dim` | rendered |
| `fg_color` | `Option<Color>` | `Style.color` | stored but not rendered |
| `bg_color` | `Option<Color>` | `Style.background` | stored but not rendered |
| `italic` | `bool` | `Style.emphasis.italic` | rendered when supported |
| `strikethrough` | `bool` | `Style.emphasis.strikethrough` | stored but not rendered |
| `blink` | `bool` | `Style.emphasis.blink` | stored but not rendered |
| `underline` | `UnderliningRequest` | `Style.emphasis.underline` | stored but not rendered |
| `layout` | `Layout` | `NodeAttrs::layout()` on the projected node | rendered |

## Terminal IR Implementation

### Tree Projection

`TextBlock` projects into the canonical render tree as:

```text
Paragraph [Style, Layout]
  └─ Text { value: <content> }
```

- **Root node:** `NodeKind::Paragraph` with one `NodeKind::Text` child.
- **Style:** a `renderable::style::Style` constructed from the component's stored fields. Only non-default values are recorded.
- **Layout:** the existing `Layout` is recorded on the paragraph node via `node.attrs.set_layout(&self.layout)` when it differs from `Layout::default()`.

This shape matches the documented tree model: `Layout` is block-only and belongs on the paragraph node; `Style` may attach to block nodes and is already consumed by the terminal tree renderer.

### Field Mapping Detail

| TextBlock field | `Style` / `Layout` target | Notes |
|-----------------|---------------------------|-------|
| `font_weight = Bold` | `emphasis.bold = true` | |
| `font_weight = Dim` | `emphasis.dim = true` | |
| `font_weight = Normal` | default | |
| `fg_color = Some(c)` | `color = Some(TargetValue::Universal(PerMode::Universal(c)))` | Use the shared `renderable::color::Color` type. |
| `bg_color = Some(c)` | `background = Some(TargetValue::Universal(PerMode::Universal(c)))` | |
| `italic = true` | `emphasis.italic = true` | |
| `strikethrough = true` | `emphasis.strikethrough = true` | Activates a currently inert stored field. |
| `blink = true` | `emphasis.blink = true` | Activates a currently inert stored field. |
| `underline = Straight(color)` | `emphasis.underline = Some(UnderlineStyle::Straight)` | Underline color has no current `Style` slot; see Notes. |
| `underline = Double(color)` | `emphasis.underline = Some(UnderlineStyle::Double)` | |
| `underline = Curly(color)` | `emphasis.underline = Some(UnderlineStyle::Curly)` | |
| `underline = Dotted(color)` | `emphasis.underline = Some(UnderlineStyle::Dotted)` | |
| `underline = Dashed(color)` | `emphasis.underline = Some(UnderlineStyle::Dashed)` | |
| `underline = None` | default | |
| `layout` | `node.attrs.set_layout(&layout)` | When non-default. |

`UnderliningRequest` can carry an optional underline color. `renderable::style::UnderlineStyle` currently records underline shape only. Do not add underline color to the render tree for this migration; the current `TextBlock` terminal path does not render underline at all, so preserving underline color is not required for parity. If colored underlines become a public requirement later, they should be designed as a general `TextEmphasis`/`Style` feature, not as a `TextBlock`-only hint.

### TreeRenderable Implementation

```rust
impl renderable::tree::TreeRenderable for TextBlock {
    fn render_tree(&self) -> renderable::tree::RenderNode {
        let style = self.build_style();
        let mut node = RenderNode::paragraph(vec![RenderNode::text(&self.content)]);
        if !style.is_empty() {
            node.attrs.set_style(&style);
        }
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        node
    }
}
```

Add a private `build_style(&self) -> renderable::style::Style` helper. Keep it private unless another component needs the same conversion.

### TerminalRenderable Delegation

After tree structure tests and terminal parity tests are in place, re-point `TerminalRenderable` to the tree renderer:

```rust
fn render(&self, term: &Terminal) -> String {
    let node = self.render_tree();
    let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
    render_terminal_node(&node, &opts)
        .map(|r| r.output)
        .unwrap_or_else(|_| self.bespoke_render(term))
}
```

Retain the old implementation as a private `bespoke_render()` fallback during the transition. The fallback preserves the infallible `TerminalRenderable` contract if tree validation fails unexpectedly.

### Terminal Test Strategy

Split tests into two groups so the intentional behavior change is explicit.

**Legacy parity tests** compare current bespoke output with tree output only for behavior the bespoke path actually implements:

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text | Text content matches after ANSI stripping. |
| 2 | Bold | Both paths contain bold SGR when `term.is_tty`. |
| 3 | Dim | Both paths contain dim SGR when `term.is_tty`. |
| 4 | Italic | Both paths contain italic SGR when supported. |
| 5 | Bold + italic | Both visible styles are present. |
| 6 | Layout with left margin | Both outputs have equivalent left padding. |
| 7 | Layout with center alignment | Both outputs center equivalently for a fixed optimistic width. |
| 8 | Empty content | Both paths produce an empty/blank-equivalent output after layout. |
| 9 | Unicode content | Unicode content is preserved. |

**Activated stored-field tests** assert the new tree-backed behavior for fields that were previously stored but inert:

| # | Variant | Asserts |
|---|---------|---------|
| 10 | Foreground basic color | Tree terminal output contains the expected foreground SGR. |
| 11 | Foreground RGB/Tailwind color | Tree terminal output uses the shared color degradation path. |
| 12 | Background color | Tree terminal output contains a background SGR. |
| 13 | Straight underline | Tree terminal output contains underline SGR when supported. |
| 14 | Curly underline with limited support | Tree terminal output degrades to straight underline when needed. |
| 15 | Strikethrough | Tree terminal output contains strikethrough SGR. |
| 16 | Blink | Tree terminal output contains blink SGR. |
| 17 | All stored styles combined | All applicable SGR layers are present and reset safely. |

**Tree structure tests:**

| # | Variant | Asserts |
|---|---------|---------|
| 18 | Plain text projection | Root is `Paragraph`, single child is `Text`. |
| 19 | Style on node | `node.attrs.style()` returns expected emphasis/color/background. |
| 20 | Layout on node | `node.attrs.layout()` returns the component layout. |
| 21 | Default layout not recorded | Default layout does not create a layout attr. |
| 22 | Underline color ignored | `UnderliningRequest::*Some(color)` maps shape only, with no ad hoc data hint. |

## Browser IR Implementation

`TextBlock` has no existing bespoke browser implementation. Browser output should be derived from the tree projection through `BrowserTreeComponent<TextBlock>` or direct `render_browser_node()` delegation.

The existing browser tree renderer already handles `Paragraph`, `Text`, and `Layout`, but it currently does **not** lower `Style` to CSS or semantic wrappers. Therefore styled browser output requires a render-tree implementation change.

### Feature Requests for Tree Rendering

#### RT-TEXTBLOCK-001: Browser lowering for `Style` text appearance and colors

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: `TextBlock` is exactly the simple styled-paragraph case that `Style` was created to support. The terminal renderer already consumes `Style`; leaving browser output unstyled would make `TextBlock` browser support structurally present but visually incomplete. Browser `Style` lowering is already called out as designed but unwired in `layout-and-style.md`, so approving it here closes a known render-tree gap rather than adding a component-specific special case.

Required behavior:

- Lower `Style.color` to CSS `color`.
- Lower `Style.background` to CSS `background-color`.
- Lower `TextEmphasis.bold`, `italic`, and `strikethrough` to semantic wrappers (`<strong>`, `<em>`, `<s>`) or equivalent valid HTML that preserves nesting.
- Lower underline variants with `UnderlineStyle::css_declaration()`.
- Lower dim to `opacity: 0.6`.
- Lower blink to `text-decoration: blink`, accepting that browser support is limited.
- Apply style to block nodes and inline `Span` nodes without overwriting existing layout CSS in the same `style` attribute.
- Continue to ignore `border` and `fill` in the browser until the broader Browser Style lowering work defines those box-painting semantics.
- Preserve current plain paragraph output when no `Style` is present.
- Tests must cover foreground, background, bold, italic, strikethrough, each underline variant, dim, blink, style plus layout on the same node, and no-style output unchanged.

### Browser Output Examples

These examples assume RT-TEXTBLOCK-001 has landed:

| TextBlock config | Browser output shape |
|------------------|----------------------|
| Plain text | `<p>Hello world</p>` |
| Bold + red fg | `<p style="color:..."><strong>Hello</strong></p>` or equivalent |
| Bold + italic | `<p><strong><em>Hello</em></strong></p>` |
| Blue fg, gray bg | `<p style="color:...;background-color:...">Hello</p>` |
| Curly underline | element with `text-decoration: underline; text-decoration-style: wavy` |
| Strikethrough | `<s>Hello</s>` or equivalent |
| With left margin | same node also includes `margin-left:4ch` from `Layout` |

### Browser Test Strategy

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text | HTML contains `<p>` and escaped text, no style attribute from `Style`. |
| 2 | Bold | HTML preserves bold semantics. |
| 3 | Italic | HTML preserves italic semantics. |
| 4 | Strikethrough | HTML preserves strikethrough semantics. |
| 5 | Foreground color | HTML has `color:` in CSS. |
| 6 | Background color | HTML has `background-color:` in CSS. |
| 7 | Underline variants | HTML has the expected underline CSS declaration. |
| 8 | Dim | HTML has `opacity: 0.6`. |
| 9 | Blink | HTML has `text-decoration: blink`. |
| 10 | Style + layout | One valid `style` attribute contains both style and layout declarations. |
| 11 | Empty content | HTML is an empty paragraph. |
| 12 | Unicode and HTML-sensitive content | Unicode is preserved and `<`, `>`, `&` are escaped. |

## Markdown IR Implementation

The current Markdown tree renderer ignores `Style` entirely for both `Markdown` and `MarkdownPlus`, per `layout-and-style.md`. It renders `Paragraph(Text)` as plain text and does not inspect `NodeAttrs::style()`.

For this migration, `TextBlock` should still implement `MarkdownRenderable` by delegating to `render_markdown_node()`, but the output contract must be plain-text Markdown. Do not add bespoke Markdown styling logic to `TextBlock`; that would immediately fork behavior away from the canonical tree renderer.

### Feature Requests for Tree Rendering

#### RT-TEXTBLOCK-002: MarkdownPlus lowering for `Style`

**DENIED**

this feature will not be added to the render-tree tree implementation. You should try to still use the render-tree where practical and work around the complexity but if the complexity is too great then you have permission to create a bespoke IR implementation for this component.

Why: `layout-and-style.md` explicitly documents that Markdown ignores `Style` entirely and emits no diagnostic. Adding MarkdownPlus style lowering as part of the `TextBlock` migration would change a cross-cutting Markdown renderer contract for one component. TextBlock can still use the render tree for Markdown by emitting plain text. Rich MarkdownPlus styling should be handled later by the dedicated Markdown styling work, with a renderer-wide design for escaping, nesting, strictness diagnostics, and interaction with semantic inline nodes.

Required behavior for this component:

- `render_markdown()` delegates to `render_markdown_node()` with `MarkdownDialect::Markdown`.
- `render_markdown_plus()` delegates to `render_markdown_node()` with `MarkdownDialect::MarkdownPlus`.
- Both methods currently produce the same plain text for styled `TextBlock` values.
- Tests must assert that color, background, underline, dim, blink, bold, italic, and strikethrough stored in `Style` do not change Markdown output until the renderer-wide Markdown styling feature exists.

### Markdown Test Strategy

| # | Variant | Markdown asserts | MarkdownPlus asserts |
|---|---------|------------------|----------------------|
| 1 | Plain text | `Hello` | `Hello` |
| 2 | Bold stored as `Style` | `Hello` | `Hello` |
| 3 | Italic stored as `Style` | `Hello` | `Hello` |
| 4 | Strikethrough stored as `Style` | `Hello` | `Hello` |
| 5 | Red foreground | `Hello` | `Hello` |
| 6 | Gray background | `Hello` | `Hello` |
| 7 | Underline | `Hello` | `Hello` |
| 8 | Dim | `Hello` | `Hello` |
| 9 | Blink | `Hello` | `Hello` |
| 10 | Empty content | `""` | `""` |
| 11 | HTML-sensitive content | Markdown text is not interpreted as raw HTML by the renderer. | Same. |

## `bt` CLI

### Current State

| Aspect | Status |
|--------|--------|
| CLI command exists | No (`TextBlock` has no subcommand; `bt block` is a generic tree-based command) |
| Render method used | N/A |
| Has `--md` / `--html` switches | No |
| Has `--example` switch | No |

### CLI Decision

Add a dedicated `bt text-block` command rather than reusing `bt block`.

Why: `bt block` is a generic render-tree style exerciser and includes box concepts such as border/fill that `TextBlock` does not expose. `bt text-block` should instantiate the actual component and exercise its migration path. Keeping the command separate makes it useful as a component parity surface without overloading `bt block`.

### CLI Specification

Add a `TextBlock(TextBlockArgs)` variant to the CLI `Command` enum and create a `commands/text_block.rs` module.

| Flag | Type | Description |
|------|------|-------------|
| `TEXT` | positional, required, variadic | Text content; multiple values joined with spaces. |
| `--example` / `-e` | flag | Render a representative example and the command used. |
| `--bold` | flag | Bold text. |
| `--dim` | flag | Dim text. |
| `--italic` | flag | Italic text. |
| `--strikethrough` / `--strike` | flag | Strikethrough text. |
| `--underline` | flag | Straight underline. |
| `--double-underline` | flag | Double underline. |
| `--curly-underline` | flag | Curly/wavy underline. |
| `--dotted-underline` | flag | Dotted underline. |
| `--dashed-underline` | flag | Dashed underline. |
| `--blink` | flag | Blinking text. |
| `--fg` | string | Foreground color accepted by the existing CLI color parser. |
| `--bg` | string | Background color accepted by the existing CLI color parser. |
| `--html` | flag | Render an HTML fragment instead of terminal output. |
| `--md` | flag | Render portable Markdown instead of terminal output. |
| `--md-plus` | flag | Render MarkdownPlus instead of terminal output. Currently plain text for `TextBlock` style. |
| Layout flags | flattened existing layout args | Margins, alignment, and other supported layout options. |

`--html`, `--md`, and `--md-plus` are mutually exclusive. Terminal remains the default target.

Example:

```text
bt text-block "Release candidate passed" --fg green --bold --underline
```

Implementation:

1. Parse args into a `TextBlock` via the builder API.
2. Terminal target: render through `render_tree()` -> `render_terminal_node()`.
3. Browser target: render through `render_tree()` -> `render_browser_node()` once RT-TEXTBLOCK-001 is available.
4. Markdown targets: call the component's `MarkdownRenderable` methods, which delegate to the tree renderer.
5. `--example`: use preset values (`--fg green --bold --underline`) and print both rendered output and the exact command.

## Documentation Updates

Update alongside implementation:

- `biscuit-terminal/docs/components/text_block.md`: document that foreground/background, underline, strikethrough, and blink are now active through the tree renderer.
- `biscuit-terminal/docs/components/index.md`: update target support for `TextBlock`.
- `renderable/docs/components.md` if present in the implementation branch: mark Tree/Browser/Markdown support according to the final implemented state.
- Per-area `docs/dependencies.md` only if new crates are added; none should be needed for this migration.

## Acceptance Criteria Summary

1. `TextBlock` implements `TreeRenderable` with a `Paragraph(Text)` projection carrying non-default `Style` and `Layout`.
2. Terminal rendering delegates to the tree renderer after parity coverage is in place, with the old bespoke path retained as a private fallback.
3. Tests distinguish legacy parity from newly activated stored fields.
4. Browser rendering is implemented through `BrowserTreeComponent` or direct tree delegation after RT-TEXTBLOCK-001 lands.
5. Markdown and MarkdownPlus rendering delegate to the tree renderer and currently produce plain text for style-only formatting.
6. A `bt text-block` subcommand exists with style flags, target switches, `--example`, and layout flags.
7. Cross-target tests cover Terminal, Browser, Markdown, and MarkdownPlus behavior.
8. Documentation is updated for the public behavior change that previously inert stored fields now render.

## Follow-up Clarifications and Design Decisions

- Design decision: activate the stored `fg_color`, `bg_color`, `underline`, `strikethrough`, and `blink` fields when rendering through the tree. These fields are already part of `TextBlock`'s public API and map cleanly to `Style`; leaving them inert would preserve a bug rather than preserve meaningful compatibility.
- Design decision: do not preserve `UnderliningRequest`'s optional underline color in the tree projection. The current component does not render underline at all, and `Style` has no underline-color slot. A future underline-color feature should be renderer-wide.
- Design decision: keep Markdown and MarkdownPlus plain-text for this migration. Styled MarkdownPlus should wait for a renderer-wide Markdown style design rather than a component-specific workaround.
